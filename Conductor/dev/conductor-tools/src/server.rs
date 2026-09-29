//! File:       Opus/Conductor/dev/conductor-tools/src/server.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The server's switch: stopped, starting, running or stopping, and the
//! mailbox the web admin drops START, STOP and RESTART into for the
//! launcher to act on.
//!
//! Conductor the program and "the server" are two different things.  The
//! program is DiskMan, Scribe, Constellations and the web admin, and it's
//! up from the moment the launcher runs.  The server is everything else
//! (Fingerprinter, Archivist, the monitor, and later the network and the
//! game), and it only runs once somebody presses START SERVER on the web
//! admin's Control Panel.  The launcher is what starts and stops the
//! pieces, since it's the one that knows what they are.  This file only
//! keeps the state and passes the asks along.
//!
//! `ask()` is the web admin's side.  It checks the ask makes sense (you
//! can't start what's already running), flips the state to starting or
//! stopping on the spot so a second click is turned away, and posts the
//! command.  `next_command()` is the launcher's side: it waits for one.
//! `set()` is the launcher saying where it got to.
//!
//! Nothing in here writes to Scribe.  The web admin and the launcher say
//! what happened in their own words.

use std::fmt;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use crate::clock::Utc;

/// Where the server is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Stopped,
    Starting,
    Running,
    Stopping,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            State::Stopped => "stopped",
            State::Starting => "starting",
            State::Running => "running",
            State::Stopping => "stopping",
        };
        write!(f, "{word}")
    }
}

/// What the web admin can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Start,
    Stop,
    /// Stop, then start again.
    Restart,
}

/// A copy of the switch, for the page.
#[derive(Debug, Clone)]
pub struct Status {
    pub state: State,
    /// What last happened to it: who asked, or what the launcher is doing.
    pub note: String,
    /// When the state last changed.  `None` until the launcher has said
    /// anything.
    pub since: Option<Utc>,
}

struct Switch {
    state: State,
    note: String,
    since: Option<Utc>,
}

static SWITCH: Mutex<Switch> = Mutex::new(Switch { state: State::Stopped, note: String::new(), since: None });

/// The commands on their way to the launcher.  One end for whoever asks,
/// the other for the launcher, made the first time either is needed.
struct Mailbox {
    sender: Sender<Command>,
    receiver: Mutex<Receiver<Command>>,
}

static MAILBOX: OnceLock<Mailbox> = OnceLock::new();

fn mailbox() -> &'static Mailbox {
    MAILBOX.get_or_init(|| {
        let (sender, receiver) = mpsc::channel();
        Mailbox { sender, receiver: Mutex::new(receiver) }
    })
}

/// The web admin asking for a start, a stop or a restart.  If it makes
/// sense from where the server is, the state flips to starting or
/// stopping right here, and the launcher gets the command.  If it
/// doesn't, `Err` says what state the server is in, and nothing changes.
/// Two clicks in a row can't both get through: the first one flips the
/// state, and the second sees it.
pub fn ask(command: Command) -> Result<(), State> {
    let (wanted, next, note) = match command {
        Command::Start => (State::Stopped, State::Starting, "Asked to start from the web admin."),
        Command::Stop => (State::Running, State::Stopping, "Asked to stop from the web admin."),
        Command::Restart => (State::Running, State::Stopping, "Asked to restart from the web admin.  \
            Stopping first."),
    };

    let mut guard = SWITCH.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.state != wanted {
        return Err(guard.state);
    }
    guard.state = next;
    guard.note = note.to_string();
    guard.since = Some(Utc::now());

    // The launcher's end of the mailbox lives for the whole program, so
    // this can't fail.  If it somehow did, the state would say starting
    // with nobody starting it, and the launcher's next set() is what
    // would put that right.
    let _ = mailbox().sender.send(command);
    Ok(())
}

/// The launcher's side: waits up to `wait` for the next command.  `None`
/// if none came in that time.
pub fn next_command(wait: Duration) -> Option<Command> {
    let receiver = mailbox().receiver.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    receiver.recv_timeout(wait).ok()
}

/// The launcher saying where it got to.
pub fn set(state: State, note: &str) {
    let mut guard = SWITCH.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.state != state || guard.since.is_none() {
        guard.since = Some(Utc::now());
    }
    guard.state = state;
    guard.note = note.to_string();
}

/// A copy of where the server is at.
pub fn status() -> Status {
    let guard = SWITCH.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Status { state: guard.state, note: guard.note.clone(), since: guard.since }
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test walks the whole switch, in order, since the switch is one
    // for the whole program and two tests at once would trip each other.
    #[test]
    fn the_switch_only_takes_asks_that_make_sense() {
        set(State::Stopped, "test");
        assert_eq!(ask(Command::Stop), Err(State::Stopped));
        assert_eq!(ask(Command::Restart), Err(State::Stopped));
        assert_eq!(next_command(Duration::from_millis(10)), None);

        assert_eq!(ask(Command::Start), Ok(()));
        assert_eq!(status().state, State::Starting);
        assert!(status().note.contains("start"));
        // The second click is turned away, and the launcher hears one ask.
        assert_eq!(ask(Command::Start), Err(State::Starting));
        assert_eq!(next_command(Duration::from_millis(10)), Some(Command::Start));
        assert_eq!(next_command(Duration::from_millis(10)), None);

        set(State::Running, "up");
        assert_eq!(ask(Command::Start), Err(State::Running));
        assert_eq!(ask(Command::Restart), Ok(()));
        assert_eq!(status().state, State::Stopping);
        assert_eq!(ask(Command::Stop), Err(State::Stopping));
        assert_eq!(next_command(Duration::from_millis(10)), Some(Command::Restart));

        set(State::Stopped, "down");
        assert_eq!(status().state, State::Stopped);
        assert_eq!(status().note, "down");
        assert!(status().since.is_some());
    }

    #[test]
    fn states_read_as_words() {
        assert_eq!(State::Stopped.to_string(), "stopped");
        assert_eq!(State::Stopping.to_string(), "stopping");
    }
}
