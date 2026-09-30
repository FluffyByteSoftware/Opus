//! File:       Opus/Conductor/dev/gameclock/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The heartbeat: the game loop.  It owns the world (primlib's `World`)
//! and steps it forward on a fixed beat.  A full cycle is 250 ms, cut into
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
//! The world is only ever touched on this thread, so it needs no lock.
//! It's made fresh on every START SERVER.  Saving it on STOP SERVER and
//! loading it back comes later (design/primlib.md).

mod checks;

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use conductor_primlib::World;
use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};
use conductor_tools::threads;

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
static HEARTBEAT: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Starts the heartbeat's thread, with a fresh world.  It comes straight
/// back.  The launcher calls this every time the server starts, and
/// `stop()` every time it stops.
pub fn start() {
    if lock(&HEARTBEAT).as_ref().is_some_and(|handle| !handle.is_finished()) {
        scribe::warn(Channel::Game, "The heartbeat was asked to start while it's already running.  \
            The running one stands.");
        return;
    }

    services::set(services::HEARTBEAT, State::Starting, "Making a fresh world.");
    let (stop, stopped) = mpsc::channel();
    match threads::spawn("heartbeat", move || run(stopped)) {
        Ok(handle) => {
            *lock(&STOP) = Some(stop);
            *lock(&HEARTBEAT) = Some(handle);
            scribe::info(Channel::Game, "The heartbeat is up: five checks of 50 ms, a 250 ms cycle.");
        }
        Err(e) => {
            scribe::error_with(Channel::Game, &e, "The heartbeat couldn't start its thread.  \
                Nothing in the world moves this run.");
            services::set(services::HEARTBEAT, State::Stopped, &format!("Couldn't start its thread: {e}"));
        }
    }
}

/// Stops the heartbeat and waits for its thread to end.  It stops at the
/// next wait between two checks, so it's never cut off halfway through
/// one.  The world goes with it.
pub fn stop() {
    lock(&STOP).take();

    let handle = lock(&HEARTBEAT).take();
    if let Some(handle) = handle {
        if handle.join().is_err() {
            scribe::error(Channel::Game, "The heartbeat's thread had already died.");
        }
    }
}

/// The lock idiom, for the statics above.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The heartbeat's thread: a cycle of five checks, each at its time, and
/// again, until `stop()` drops the Sender.
fn run(stopped: Receiver<()>) {
    let mut world = World::new();

    // Tallies since START SERVER, for the Services tab.
    let mut cycles: u64 = 0;
    let mut late: u64 = 0;
    let mut busiest = Duration::ZERO;
    let mut last_warn: Option<Instant> = None;

    services::set(services::HEARTBEAT, State::Running, "Beating.  No cycle finished yet.");
    let mut cycle_start = Instant::now();

    // Rust note: `'beating:` names the outer loop, so the `break` inside
    // the inner one can leave both at once.
    'beating: loop {
        // The time spent inside the checks, and the slowest one.
        let mut busy = Duration::ZERO;
        let mut slowest = (checks::ALL[0].name, Duration::ZERO);

        for (number, check) in checks::ALL.iter().enumerate() {
            if !wait_until(&stopped, due(cycle_start, number)) {
                break 'beating;
            }
            let began = Instant::now();
            (check.run)(&mut world);
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
                scribe::warn(Channel::Game, &format!("The heartbeat is falling behind.  {what}  {late} of \
                    {cycles} cycles have run late since START SERVER."));
            } else {
                scribe::debug(Channel::Game, &what);
            }
        }

        services::set(services::HEARTBEAT, State::Running, &format!("Beating.  {cycles} cycles, {late} late.  \
            The busiest spent {} ms of its 250 in the checks.", ms(busiest)));
        services::seen(services::HEARTBEAT);

        cycle_start = next_start(cycle_start, finished);
    }

    scribe::info(Channel::Game, &format!("The heartbeat has stopped after {cycles} cycles, {late} of them late."));
    services::set(services::HEARTBEAT, State::Stopped, "Shut down.");
}

/// Waits until `when`, or until `stop()` is called.  False means stop.
/// A time already gone doesn't wait at all, but still looks for the stop,
/// so a heartbeat that's always behind can still be stopped.
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
    fn a_stopped_heartbeat_stops_even_when_its_check_is_overdue() {
        let (stop, stopped) = mpsc::channel::<()>();
        let past = Instant::now();
        assert!(wait_until(&stopped, past));
        drop(stop);
        assert!(!wait_until(&stopped, past));
        assert!(!wait_until(&stopped, Instant::now() + millis(50)));
    }
}
