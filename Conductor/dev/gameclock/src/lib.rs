//! File:       Opus/Conductor/dev/gameclock/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The GameClock: the game loop.  It owns the world (primlib's `World`)
//! and the terrain (GameWorld's chunks), and steps them forward on a fixed
//! beat.  A full cycle is 250 ms, cut into
//! five checks of 50 ms each, and each check does its own job on its own
//! group of objects: the players' input, the AI's brains, movement, the
//! positions going out, and housekeeping.  `checks.rs` has them, in order.
//!
//! The rate is fixed here, not a setting.  Jacob, 2026-09-30: "from all
//! the testing I did before anything faster is gonna be a problem.  Slower
//! is fine but faster becomes bad."  On that earlier go, with Argon2 kept
//! to one thread on the side, a cycle never ran far over 250 ms.  Security
//! still hashes on its own thread, so nothing here ever waits on a login.
//!
//! When each check starts is measured from the start of its cycle, not
//! from the end of the check before it.  So a check that finishes early
//! doesn't pull the rest forward, and a sleep that wakes a hair late
//! doesn't pile up into drift.  A check that runs long makes the next one
//! late, and the next one runs as soon as it can; nothing is skipped.  A
//! cycle whose last check finishes past its 250 ms is "late": the next
//! cycle starts straight away, on a fresh schedule from that moment,
//! instead of rushing through checks to make up the time.
//!
//! The world and the terrain are only ever touched on this thread, so they
//! need no lock.  The world is made fresh on every START SERVER.  Players'
//! characters come into it and leave it through a mailbox (`players.rs`:
//! `enter()` and `leave()`), and the world is saved every
//! `world_save_seconds` and as the thread ends on STOP SERVER
//! (`saving.rs`).  The chat comes in through a mailbox of its own
//! (`chat.rs`) and goes out to everybody in the world from the broadcast
//! check, once a cycle, and so does the answer to a `/who list`
//! (`who.rs`).  Primlib's other copies aren't saved yet
//! (design/primlib.md).  The terrain starts empty, and the GameClock asks
//! GameWorld for the chunks around 0,0,0, where every player starts for
//! now.  They come in over the first cycles, in housekeeping.

mod chat;
mod checks;
mod players;
mod saving;
mod who;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use conductor_gameworld::{SPAWN_POINTS, Terrain};
use conductor_primlib::World;
use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};
use conductor_tools::threads;

use players::Players;
use saving::{WorldSave, Writes};

// Rust note: `pub use` hands these on, so networking can write
// `conductor_gameclock::enter(...)`.
pub use chat::{chat, set_chat_sender};
pub use players::{enter, leave, saving, wait_until_saved};
pub use who::{Standing, WhoAsked, set_who_sender, who_list};

/// One check's share of a cycle, in milliseconds.
const CHECK_MS: u64 = 50;

/// One check's share of a cycle.
const CHECK_EVERY: Duration = Duration::from_millis(CHECK_MS);

/// A full cycle: every check once.  Five checks of 50 ms is 250 ms.
const CYCLE: Duration = Duration::from_millis(CHECK_MS * checks::ALL.len() as u64);

/// How far over its 250 ms a cycle has to run before it's a Warn on the
/// bell instead of a Debug line.  A cycle a few milliseconds late is
/// nothing anybody would notice in the game; a whole second is.
const BADLY_LATE: Duration = Duration::from_secs(1);

/// The least time between two of those Warns, so a server that's
/// struggling doesn't bury the bell in them.
const WARN_EVERY_AT_MOST: Duration = Duration::from_secs(60);

// Nothing is ever sent on this.  Dropping the Sender is the signal to
// stop, the same way the monitor does it.
static STOP: Mutex<Option<Sender<()>>> = Mutex::new(None);
static GAMECLOCK: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// True once every chunk around 0,0,0 is in the terrain, so a player would
/// have ground to stand on.  The launcher waits on it to open the door.
static READY: AtomicBool = AtomicBool::new(false);

/// Everything the checks work on, owned by the GameClock's thread: the
/// world, the terrain, the players' characters in the world, when the
/// next world save is due, and the saves on their way to the database.
pub(crate) struct Game {
    pub(crate) world: World,
    pub(crate) terrain: Terrain,
    pub(crate) players: Players,
    pub(crate) world_save: WorldSave,
    pub(crate) writes: Writes,
}

/// Starts the GameClock's thread, with a fresh world.  It comes straight
/// back.  The launcher calls this every time the server starts, and
/// `stop()` every time it stops.
pub fn start() {
    if lock(&GAMECLOCK).as_ref().is_some_and(|handle| !handle.is_finished()) {
        scribe::warn(Channel::Game, "The GameClock was asked to start while it's already running.  \
            The running one stands.");
        return;
    }

    READY.store(false, Ordering::SeqCst);
    players::forget_saving();
    chat::open();
    who::open();
    services::set(services::GAMECLOCK, State::Starting, "Making a fresh world.");
    let (stop, stopped) = mpsc::channel();
    let notes = players::open_mailbox();
    match threads::spawn("gameclock", move || run(stopped, notes)) {
        Ok(handle) => {
            *lock(&STOP) = Some(stop);
            *lock(&GAMECLOCK) = Some(handle);
            scribe::info(Channel::Game, "The GameClock is up: five checks of 50 ms, a 250 ms cycle.");
        }
        Err(e) => {
            players::close_mailbox();
            chat::close();
            who::close();
            scribe::error_with(Channel::Game, &e, "The GameClock couldn't start its thread.  \
                Nothing in the world moves this run.");
            services::set(services::GAMECLOCK, State::Stopped, &format!("Couldn't start its thread: {e}"));
        }
    }
}

/// Stops the GameClock and waits for its thread to end.  It stops at the
/// next wait between two checks, so it's never cut off halfway through
/// one, saves the world one last time, and waits for its saves to land.
/// The world goes with it.  Archivist has to still be running, so the
/// launcher stops the GameClock before it.
pub fn stop() {
    READY.store(false, Ordering::SeqCst);
    players::close_mailbox();
    chat::close();
    who::close();
    lock(&STOP).take();

    let handle = lock(&GAMECLOCK).take();
    if let Some(handle) = handle {
        if handle.join().is_err() {
            scribe::error(Channel::Game, "The GameClock's thread had already died.");
        }
    }
}

/// Whether the ground around 0,0,0, where every player starts, is all in
/// memory.  Until it is, nobody should be let in.  False while the
/// GameClock is stopped, and it stays false for a run where a chunk there
/// couldn't be had.
pub fn ready() -> bool {
    READY.load(Ordering::SeqCst)
}

/// The lock idiom, for the statics above.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The GameClock's thread: a cycle of five checks, each at its time, and
/// again, until `stop()` drops the Sender.
fn run(stopped: Receiver<()>, notes: Receiver<players::Note>) {
    let mut game = Game {
        world: World::new(),
        terrain: Terrain::new(),
        players: Players::new(notes),
        world_save: WorldSave::new(saving::world_save_every()),
        writes: Writes::new(),
    };
    game.terrain.ask_around(SPAWN_POINTS[0].0, SPAWN_POINTS[0].1, conductor_gameworld::view_chunks());

    // Tallies since START SERVER, for the Services tab.
    let mut cycles: u64 = 0;
    let mut late: u64 = 0;
    let mut busiest = Duration::ZERO;
    let mut last_warn: Option<Instant> = None;

    services::set(services::GAMECLOCK, State::Running, "Beating.  No cycle finished yet.");
    let mut cycle_start = Instant::now();

    // Rust note: `'beating:` names the outer loop, so the `break` inside
    // the inner one can leave both at once.
    'beating: loop {
        // The time spent inside the checks, and the slowest one.
        let mut busy = Duration::ZERO;
        let mut slowest = (checks::ALL[0].name, Duration::ZERO);

        let ready = READY.load(Ordering::SeqCst);
        for (number, check) in checks::ALL.iter().enumerate() {
            if !wait_until(&stopped, due(cycle_start, number)) {
                break 'beating;
            }
            // Until the ground is in, a check that doesn't run before then
            // still gets its 50 ms, and does nothing with them.
            if !ready && !check.before_ready {
                continue;
            }
            let began = Instant::now();
            (check.run)(&mut game);
            let took = began.elapsed();
            busy += took;
            if took > slowest.1 {
                slowest = (check.name, took);
            }
        }

        let finished = Instant::now();
        cycles += 1;
        busiest = busiest.max(busy);

        if let Some(over) = overrun(cycle_start, finished) {
            late += 1;
            let what = format!("Cycle {cycles} ran {} ms over its 250 ms.  The slowest check was {}, at {} ms.",
                               ms(over), slowest.0, ms(slowest.1));
            if over >= BADLY_LATE && may_warn(last_warn, finished) {
                last_warn = Some(finished);
                scribe::warn(Channel::Game, &format!("The GameClock is falling behind.  {what}  {late} of \
                    {cycles} cycles have run late since START SERVER."));
            } else {
                scribe::debug(Channel::Game, &what);
            }
        }

        if !READY.load(Ordering::SeqCst) && game.terrain.asked() > 0 && game.terrain.held() == game.terrain.asked() {
            READY.store(true, Ordering::SeqCst);
            game.world_save.begin(finished);
            scribe::info(Channel::Game, &format!("The ground around 0,0,0 is in, {} chunks, after {cycles} \
                cycles.  The door can open.", game.terrain.held()));
        }

        services::set(services::GAMECLOCK, State::Running, &format!("Beating.  {cycles} cycles, {late} late.  \
            The busiest spent {} ms of its 250 in the checks.  {}  {} players in the world.  {}", ms(busiest),
            chunks(&game.terrain), game.players.count(), game.world_save.summary()));
        services::seen(services::GAMECLOCK);

        cycle_start = next_start(cycle_start, finished);
    }

    READY.store(false, Ordering::SeqCst);
    let saved = save_world(&mut game);
    game.writes.finish();
    // Whatever was marked saving has landed, failed, or been given up on
    // above.  Nobody is to wait on it past here.
    players::forget_saving();
    scribe::info(Channel::Game, &format!("The GameClock has stopped after {cycles} cycles, {late} of them late, \
        and saved the world on the way out ({saved} characters)."));
    services::set(services::GAMECLOCK, State::Stopped, "Shut down.");
}

/// A world save: every player's character copied as it stands, in this
/// cycle, and handed to Archivist to turn into text and write, while the
/// GameClock beats on.  How many characters it saved.
pub(crate) fn save_world(game: &mut Game) -> usize {
    let began = Instant::now();
    let snapshot = game.players.snapshot(&game.world);
    let took = began.elapsed();
    let count = snapshot.len();
    game.writes.send("save the world", snapshot);
    game.world_save.record(count, took);
    if count > 0 {
        scribe::debug(Channel::Game, &format!("Saved the world: {count} characters, the snapshot took {} ms.  \
            Archivist writes them now.", ms(took)));
    }
    count
}

/// Waits until `when`, or until `stop()` is called.  False means stop.
/// A time already gone doesn't wait at all, but still looks for the stop,
/// so a GameClock that's always behind can still be stopped.
fn wait_until(stopped: &Receiver<()>, when: Instant) -> bool {
    let now = Instant::now();
    if when > now {
        // Rust note: `recv_timeout` waits up to that long for a message.
        // None ever comes, so it either times out (time for the check) or
        // hears the Sender is gone (stop).
        match stopped.recv_timeout(when - now) {
            Err(RecvTimeoutError::Timeout) => true,
            Ok(()) | Err(RecvTimeoutError::Disconnected) => false,
        }
    } else {
        match stopped.try_recv() {
            Err(TryRecvError::Empty) => true,
            Ok(()) | Err(TryRecvError::Disconnected) => false,
        }
    }
}

/// When check `number` (0 to 4) of a cycle is due: 50 ms apart, counted
/// from the cycle's start.
fn due(cycle_start: Instant, number: usize) -> Instant {
    cycle_start + CHECK_EVERY * number as u32
}

/// How far past its 250 ms a cycle finished.  `None` when it finished in
/// time, or right on it.
fn overrun(cycle_start: Instant, finished: Instant) -> Option<Duration> {
    finished.checked_duration_since(cycle_start + CYCLE)
        .filter(|over| !over.is_zero())
}

/// When the next cycle starts: 250 ms after this one did, or straight away
/// if this one ran late.  A late cycle doesn't make the next ones hurry.
fn next_start(cycle_start: Instant, finished: Instant) -> Instant {
    let on_time = cycle_start + CYCLE;
    if finished > on_time { finished } else { on_time }
}

/// Whether a Warn may go out now, a minute or more after the last one.
fn may_warn(last_warn: Option<Instant>, now: Instant) -> bool {
    last_warn.is_none_or(|last| now.duration_since(last) >= WARN_EVERY_AT_MOST)
}

/// How the terrain is doing, for the Services tab.
fn chunks(terrain: &Terrain) -> String {
    let mut words = format!("{} of the {} chunks around 0,0,0 are in.", terrain.held(), terrain.asked());
    if !ready() {
        words.push_str("  Until they all are, only housekeeping runs.");
    }
    if terrain.failed() > 0 {
        words.push_str(&format!("  {} couldn't be had (the log says why).", terrain.failed()));
    }
    words
}

/// A duration in milliseconds, one place after the point.
fn ms(duration: Duration) -> String {
    format!("{:.1}", duration.as_secs_f64() * 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn millis(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    #[test]
    fn five_checks_of_50_ms_make_a_250_ms_cycle() {
        assert_eq!(checks::ALL.len(), 5);
        assert_eq!(CHECK_EVERY, millis(50));
        assert_eq!(CYCLE, millis(250));
    }

    #[test]
    fn the_checks_run_in_jacobs_order() {
        let names: Vec<&str> = checks::ALL.iter().map(|check| check.name).collect();
        assert_eq!(names, vec!["input", "AI", "movement", "broadcast", "housekeeping"]);
    }

    #[test]
    fn only_housekeeping_runs_before_the_ground_is_in() {
        let early: Vec<&str> = checks::ALL.iter().filter(|check| check.before_ready).map(|check| check.name).collect();
        assert_eq!(early, vec!["housekeeping"]);
    }

    #[test]
    fn each_check_is_due_50_ms_after_the_one_before() {
        let start = Instant::now();
        assert_eq!(due(start, 0), start);
        assert_eq!(due(start, 1), start + millis(50));
        assert_eq!(due(start, 4), start + millis(200));
    }

    #[test]
    fn a_cycle_in_time_is_not_late_and_the_next_starts_250_ms_on() {
        let start = Instant::now();
        let finished = start + millis(201);
        assert_eq!(overrun(start, finished), None);
        assert_eq!(next_start(start, finished), start + millis(250));
    }

    #[test]
    fn a_cycle_finishing_right_on_250_ms_is_not_late() {
        let start = Instant::now();
        let finished = start + millis(250);
        assert_eq!(overrun(start, finished), None);
        assert_eq!(next_start(start, finished), start + millis(250));
    }

    #[test]
    fn a_late_cycle_says_by_how_much_and_the_next_starts_at_once() {
        let start = Instant::now();
        let finished = start + millis(300);
        assert_eq!(overrun(start, finished), Some(millis(50)));
        assert_eq!(next_start(start, finished), finished);
    }

    #[test]
    fn a_warn_goes_out_at_most_once_a_minute() {
        let now = Instant::now();
        assert!(may_warn(None, now));
        assert!(!may_warn(Some(now), now + Duration::from_secs(59)));
        assert!(may_warn(Some(now), now + Duration::from_secs(60)));
    }

    #[test]
    fn a_stopped_gameclock_stops_even_when_its_check_is_overdue() {
        let (stop, stopped) = mpsc::channel::<()>();
        let past = Instant::now();
        assert!(wait_until(&stopped, past));
        drop(stop);
        assert!(!wait_until(&stopped, past));
        assert!(!wait_until(&stopped, Instant::now() + millis(50)));
    }
}
