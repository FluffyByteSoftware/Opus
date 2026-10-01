//! File:       Opus/Conductor/dev/gameclock/src/saving.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Saving the world.  Every `world_save_seconds` (`game.cfg`, 150 to
//! start) housekeeping copies every player's character as it stands, in
//! one cycle, so what's saved is the world at one moment (Jacob,
//! 2026-09-30: "a global save to happen where the world state is pushed
//! in a tick cycle").  The first comes that long after the ground is in,
//! "after the world is loaded and ready and the gameclock starts
//! processing game ticks".
//!
//! The GameClock only copies.  The copies go to Archivist, which turns
//! each into Lua text and writes its row, one after another, all in one
//! transaction (`characters::save_all()`), while the GameClock beats on.
//! Jacob: "I'm just worried about blocking or lagging the regular tick."
//!
//! A character leaving is saved the same way, on its own.  On STOP SERVER
//! the GameClock saves the world one last time as its thread ends.
//!
//! A write that fails comes back only in its `Pending`, so the saves on
//! their way are kept here and looked at each housekeeping, never waited
//! on, and a failure is an Error on the bell.

use std::time::{Duration, Instant};

use conductor_accounts::characters::{self, SavedCharacter};
use conductor_tools::archivist::Pending;
use conductor_tools::constellations::{self, GAME};
use conductor_tools::scribe::{self, Channel};

/// How long the GameClock waits for its last saves on the way out, once
/// its thread is done beating, so the log can say whether they landed.
const LAST_SAVE_WAIT: Duration = Duration::from_secs(10);

/// `world_save_seconds` in `game.cfg`.
pub fn world_save_every() -> Duration {
    Duration::from_secs(constellations::number(&GAME, "world_save_seconds"))
}

/// When the next world save is due, and how the last one went.
pub struct WorldSave {
    every: Duration,
    /// `None` until the ground is in.
    next: Option<Instant>,
    done: u64,
    /// The last one's characters, and how long its snapshot took.
    last: Option<(usize, Duration)>,
}

impl WorldSave {
    pub fn new(every: Duration) -> WorldSave {
        WorldSave { every, next: None, done: 0, last: None }
    }

    /// The ground is in: the first world save is due `every` from `now`.
    pub fn begin(&mut self, now: Instant) {
        self.next = Some(now + self.every);
    }

    /// Whether a world save is due at `now`.  When it is, the one after is
    /// counted from `now`, so a late cycle doesn't bring two close
    /// together.
    pub fn due(&mut self, now: Instant) -> bool {
        match self.next {
            Some(next) if now >= next => {
                self.next = Some(now + self.every);
                true
            }
            _ => false,
        }
    }

    /// A world save was taken: `characters` of them, the snapshot taking
    /// `took`.
    pub fn record(&mut self, characters: usize, took: Duration) {
        self.done += 1;
        self.last = Some((characters, took));
    }

    /// For the Services tab.
    pub fn summary(&self) -> String {
        match self.last {
            None => format!("No world save yet; one every {} s, the first once the ground is in.",
                            self.every.as_secs()),
            Some((characters, took)) => format!("{} world save(s), one every {} s.  The last was {characters} \
                characters, and its snapshot took {} ms.", self.done, self.every.as_secs(), ms(took)),
        }
    }
}

/// The saves on their way to the database.
pub struct Writes {
    on_the_way: Vec<Write>,
}

/// One save on its way: what it was, how many characters, and its answer.
struct Write {
    what: &'static str,
    characters: usize,
    pending: Pending<u64>,
}

impl Writes {
    pub fn new() -> Writes {
        Writes { on_the_way: Vec::new() }
    }

    /// Hands the characters to Archivist to be written in one transaction.
    /// Comes straight back.
    pub fn send(&mut self, what: &'static str, saved: Vec<SavedCharacter>) {
        if saved.is_empty() {
            return;
        }
        let count = saved.len();
        let pending = characters::save_all(what, saved);
        self.on_the_way.push(Write { what, characters: count, pending });
    }

    /// Looks at each save on its way, without waiting, and says so if one
    /// failed.  For housekeeping.
    pub fn check(&mut self) {
        // Rust note: `retain` keeps the ones the closure says true to:
        // here, the ones with no answer yet.
        self.on_the_way.retain(|write| match write.pending.check() {
            Some(answer) => {
                report(write, answer);
                false
            }
            None => true,
        });
    }

    /// Waits up to `LAST_SAVE_WAIT` for the saves still on their way.  Only
    /// on STOP SERVER, once the GameClock has stopped beating.
    pub fn finish(&mut self) {
        let until = Instant::now() + LAST_SAVE_WAIT;
        for write in self.on_the_way.drain(..) {
            let left = until.saturating_duration_since(Instant::now());
            match write.pending.wait_for(left) {
                Some(answer) => report(&write, answer),
                None => scribe::error(Channel::Game, &format!("The GameClock stopped waiting on a save ({}, {} \
                    characters) after {} s.  Archivist may still write it; if it doesn't, they're back where they \
                    were at the save before.", write.what, write.characters, LAST_SAVE_WAIT.as_secs())),
            }
        }
    }
}

/// What a save's answer says, in the log.  Routine is Debug; a failure is
/// an Error, since characters are back where they were at the save before.
fn report(write: &Write, answer: Result<u64, conductor_tools::archivist::ArchivistError>) {
    match answer {
        Ok(rows) if rows as usize == write.characters => {
            scribe::debug(Channel::Game, &format!("Archivist wrote {} ({rows} characters).", write.what));
        }
        Ok(rows) => scribe::warn(Channel::Game, &format!("Archivist wrote {} and only {rows} of its {} characters \
            had a row to write to.  The rest were deleted while they were in the world.", write.what,
            write.characters)),
        Err(e) => scribe::error_with(Channel::Game, &e, &format!("The GameClock's save ({}, {} characters) wasn't \
            written.  Nothing of it landed; they're back where they were at the save before.", write.what,
            write.characters)),
    }
}

/// A duration in milliseconds, two places after the point.
fn ms(duration: Duration) -> String {
    format!("{:.2}", duration.as_secs_f64() * 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_world_save_comes_before_the_ground_is_in() {
        let mut world_save = WorldSave::new(Duration::from_secs(150));
        let now = Instant::now();
        assert!(!world_save.due(now));
        assert!(!world_save.due(now + Duration::from_secs(10_000)));
    }

    #[test]
    fn the_first_world_save_comes_its_seconds_after_the_ground_is_in() {
        let mut world_save = WorldSave::new(Duration::from_secs(150));
        let ready = Instant::now();
        world_save.begin(ready);
        assert!(!world_save.due(ready + Duration::from_secs(149)));
        assert!(world_save.due(ready + Duration::from_secs(150)));
    }

    #[test]
    fn the_next_world_save_counts_from_the_last() {
        let mut world_save = WorldSave::new(Duration::from_secs(150));
        let ready = Instant::now();
        world_save.begin(ready);
        let first = ready + Duration::from_secs(151);
        assert!(world_save.due(first));
        assert!(!world_save.due(first), "not twice in one go");
        assert!(!world_save.due(first + Duration::from_secs(149)));
        assert!(world_save.due(first + Duration::from_secs(150)));
    }

    #[test]
    fn nothing_to_save_sends_nothing() {
        let mut writes = Writes::new();
        writes.send("save the world", Vec::new());
        assert!(writes.on_the_way.is_empty());
    }
}
