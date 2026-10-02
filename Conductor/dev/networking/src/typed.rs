//! File:       Opus/Conductor/dev/networking/src/typed.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! A line a player typed in the world: who typed it, what came of it,
//! and the hand-off to whatever runs it.  The commands themselves live
//! in `conductor-player-commands`, which leans on this crate (the book,
//! the packets, sending), so this crate can't name it back.  Instead it
//! holds a slot, `set_runner()`, that the launcher fills through
//! `conductor_player_commands::wire()` on every START SERVER, the same
//! way the GameClock is handed its chat sender.  A plain function, so
//! filling it again does no harm.
//!
//! The UDP thread calls `run()` for every line.  With nothing in the
//! slot (the launcher didn't wire it), a line is refused with "Commands
//! Unavailable", and the first one is a Warn, since it means Conductor is
//! put together wrong, not that the player did anything.

use std::net::SocketAddr;
use std::sync::Mutex;

use conductor_tools::scribe::{self, Channel};

use crate::protocol;

/// Who typed the line: where from, their account and character, and the
/// ask number it came in as.
pub struct Asker<'a> {
    pub from: SocketAddr,
    pub account: &'a str,
    pub character: &'a str,
    pub ask: u32,
}

/// What became of a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The answer, to keep in the book and send.
    Answer(Vec<u8>),
    /// The GameClock answers it, a cycle from now (`/who list`).  The ask
    /// stays open in the book until then.
    Later,
}

/// What runs a line: `conductor_player_commands`' dispatcher.
pub type Runner = fn(&Asker<'_>, &str) -> Outcome;

const UNAVAILABLE: &str = "Commands Unavailable";

static RUNNER: Mutex<Option<Runner>> = Mutex::new(None);

/// Puts the function that runs a line in the slot.  `wire()` in
/// `conductor-player-commands` calls this; nothing else should.
pub fn set_runner(run: Runner) {
    *lock() = Some(run);
}

/// Runs a line typed by a player whose character is in the world (the
/// UDP side turns away the rest).
pub(crate) fn run(asker: &Asker<'_>, line: &str) -> Outcome {
    // Rust note: the lock is let go before the runner is called, so a
    // command can't hold it up and nothing in a command can deadlock on it.
    let run = *lock();
    match run {
        Some(run) => run(asker, line),
        None => {
            warn_once();
            Outcome::Answer(protocol::command_refused(asker.ask, UNAVAILABLE))
        }
    }
}

fn warn_once() {
    static WARNED: Mutex<bool> = Mutex::new(false);
    let mut warned = WARNED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if !*warned {
        *warned = true;
        scribe::warn(Channel::Network, "A player typed a line and nothing runs commands: the launcher never \
            called conductor_player_commands::wire().  Every line is refused until it does.");
    }
}

fn lock() -> std::sync::MutexGuard<'static, Option<Runner>> {
    RUNNER.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn asker() -> Asker<'static> {
        Asker { from: "10.0.0.5:50000".parse().unwrap(), account: "jacob", character: "Jacob", ask: 4 }
    }

    // One test, since the slot is one static shared by every test in the
    // crate: the order matters, and both halves are checked here.
    #[test]
    fn an_empty_slot_refuses_and_a_filled_one_runs() {
        *lock() = None;
        assert_eq!(run(&asker(), "/chat hi"), Outcome::Answer(protocol::command_refused(4, UNAVAILABLE)));

        fn echo(asker: &Asker<'_>, line: &str) -> Outcome {
            Outcome::Answer(protocol::command_refused(asker.ask, line))
        }
        set_runner(echo);
        assert_eq!(run(&asker(), "/chat hi"), Outcome::Answer(protocol::command_refused(4, "/chat hi")));
        *lock() = None;
    }
}
