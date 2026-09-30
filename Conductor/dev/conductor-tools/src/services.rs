//! File:       Opus/Conductor/dev/conductor-tools/src/services.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The services Conductor expects to have, and how each one says it's
//! doing.  Nothing can look into a service from outside and tell whether
//! it's alive, so each one reports on itself here: starting, running,
//! trouble (with what went wrong), or stopped.  The web admin shows the
//! list, and anything that isn't healthy flashes red.
//!
//! Every expected service is on the list from the start as "expected", so
//! one that never started shows up as missing instead of just not being
//! there.  Two more checks catch a service that can't report on itself:
//!
//! - A service with a thread of its own is stopped once that thread has
//!   ended, whatever it last said.  A thread that panics says nothing on
//!   the way out.
//! - A service that checks in (`seen()`) and then goes quiet for longer
//!   than `QUIET_LIMIT` isn't healthy.  That catches one that's stuck.
//!
//! Nothing in here writes to Scribe.  Scribe reports here, so a call the
//! other way could have each waiting on the other's lock.

use std::fmt;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::clock::Utc;
use crate::threads;

// The names, so the places that report don't each spell them their own way.
pub const DISKMAN: &str = "DiskMan";
pub const SCRIBE: &str = "Scribe";
pub const CONSTELLATIONS: &str = "Constellations";
pub const FINGERPRINTER: &str = "Fingerprinter";
pub const SECURITY: &str = "Security";
pub const ARCHIVIST: &str = "Archivist";
pub const NETWORK_TCP: &str = "Network (TCP)";
pub const NETWORK_UDP: &str = "Network (UDP)";
pub const ACCOUNT_DESK: &str = "Account desk";
pub const MONITOR: &str = "Monitor";
pub const LUA: &str = "Lua";
pub const WEB_ADMIN: &str = "Web admin";

/// Every service Conductor expects, in the order the page lists them, and
/// the thread each runs on, if it has one.  The thread names are the ones
/// given to `threads::spawn()`.
const EXPECTED: [(&str, Option<&str>); 12] = [
    (DISKMAN, Some("diskman")),
    (SCRIBE, None),
    (CONSTELLATIONS, None),
    (FINGERPRINTER, None),
    (SECURITY, Some("security")),
    (ARCHIVIST, Some("archivist")),
    (NETWORK_TCP, Some("net-tcp")),
    (NETWORK_UDP, Some("net-udp")),
    (ACCOUNT_DESK, Some("account-desk")),
    (MONITOR, Some("monitor")),
    (LUA, Some("lua")),
    (WEB_ADMIN, Some("wgui")),
];

/// How long a service that checks in can go quiet before it counts as
/// stuck.  The monitor, DiskMan, Security and the UDP side check in at
/// least once a second, so this is five missed.
pub const QUIET_LIMIT: Duration = Duration::from_secs(5);

/// Where a service is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// On the list, but it hasn't said anything yet.
    Expected,
    Starting,
    Running,
    /// Up, but something is wrong.  The note says what.
    Trouble,
    Stopped,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            State::Expected => "expected",
            State::Starting => "starting",
            State::Running => "running",
            State::Trouble => "trouble",
            State::Stopped => "stopped",
        };
        write!(f, "{word}")
    }
}

/// One service, as the page sees it.
#[derive(Debug, Clone)]
pub struct Service {
    pub name: &'static str,
    pub state: State,
    /// What it last said about itself: where it's writing, what it's
    /// connected to, or what went wrong.
    pub note: String,
    /// When its state last changed.  `None` while it's still "expected".
    pub since: Option<Utc>,
    /// How long ago it last checked in.  `None` for a service that doesn't.
    pub seen_ago: Option<Duration>,
}

impl Service {
    /// Running, and not gone quiet.
    pub fn healthy(&self) -> bool {
        self.state == State::Running && self.seen_ago.is_none_or(|ago| ago <= QUIET_LIMIT)
    }
}

/// What's kept for each service between reports.
struct Record {
    name: &'static str,
    thread: Option<&'static str>,
    state: State,
    note: String,
    since: Option<Utc>,
    last_seen: Option<Instant>,
}

// Filled from EXPECTED the first time anything touches it.
static SERVICES: Mutex<Vec<Record>> = Mutex::new(Vec::new());

/// Runs `work` on the list, filling it in first if it's empty.
fn with_list<T>(work: impl FnOnce(&mut Vec<Record>) -> T) -> T {
    let mut guard = SERVICES.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.is_empty() {
        for (name, thread) in EXPECTED {
            guard.push(Record { name, thread, state: State::Expected, note: String::new(), since: None,
                                last_seen: None });
        }
    }
    work(&mut guard)
}

/// A service saying where it's at.  A name that isn't expected is ignored,
/// since the page would have nowhere to put it.
pub fn set(name: &str, state: State, note: &str) {
    with_list(|list| {
        if let Some(record) = list.iter_mut().find(|record| record.name == name) {
            if record.state != state {
                record.since = Some(Utc::now());
            }
            record.state = state;
            record.note = note.to_string();
        }
    });
}

/// A service checking in: "still here".  Once a service has called this,
/// going quiet for longer than `QUIET_LIMIT` makes it unhealthy.
pub fn seen(name: &str) {
    with_list(|list| {
        if let Some(record) = list.iter_mut().find(|record| record.name == name) {
            record.last_seen = Some(Instant::now());
        }
    });
}

/// Every expected service, in order, with the thread check applied.
pub fn list() -> Vec<Service> {
    // Taken before the services lock, so the two locks are never held at
    // once.
    let running: Vec<String> = threads::list().into_iter()
        .filter(|record| record.running)
        .map(|record| record.name)
        .collect();
    let now = Instant::now();

    with_list(|list| list.iter().map(|record| view(record, &running, now)).collect())
}

/// One record as the page should see it.  A service whose thread is gone
/// is stopped, whatever it last said.
fn view(record: &Record, running_threads: &[String], now: Instant) -> Service {
    let mut state = record.state;
    let mut note = record.note.clone();
    let thread_gone = record.thread.is_some_and(|thread| !running_threads.iter().any(|name| name == thread));
    if thread_gone && matches!(state, State::Starting | State::Running | State::Trouble) {
        state = State::Stopped;
        note = "Its thread has ended.".to_string();
    }

    Service {
        name: record.name,
        state,
        note,
        since: record.since,
        seen_ago: record.last_seen.map(|seen| now.duration_since(seen)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(state: State, thread: Option<&'static str>, last_seen: Option<Instant>) -> Record {
        Record { name: "test", thread, state, note: "fine".to_string(), since: None, last_seen }
    }

    #[test]
    fn every_expected_service_is_there_from_the_start() {
        let names: Vec<&str> = list().iter().map(|service| service.name).collect();
        assert_eq!(names, vec![DISKMAN, SCRIBE, CONSTELLATIONS, FINGERPRINTER, SECURITY, ARCHIVIST, NETWORK_TCP,
                               NETWORK_UDP, ACCOUNT_DESK, MONITOR, LUA, WEB_ADMIN]);
    }

    #[test]
    fn only_running_is_healthy() {
        let now = Instant::now();
        for (state, healthy) in [(State::Expected, false), (State::Starting, false), (State::Running, true),
                                 (State::Trouble, false), (State::Stopped, false)] {
            assert_eq!(view(&record(state, None, None), &[], now).healthy(), healthy, "{state}");
        }
    }

    #[test]
    fn a_service_whose_thread_ended_is_stopped() {
        let now = Instant::now();
        let gone = view(&record(State::Running, Some("worker"), None), &[], now);
        assert_eq!(gone.state, State::Stopped);
        assert!(!gone.healthy());

        let there = view(&record(State::Running, Some("worker"), None), &["worker".to_string()], now);
        assert_eq!(there.state, State::Running);
        assert_eq!(there.note, "fine");
    }

    #[test]
    fn a_service_that_goes_quiet_is_not_healthy() {
        let then = Instant::now();
        let just_now = view(&record(State::Running, None, Some(then)), &[], then + Duration::from_secs(1));
        assert!(just_now.healthy());

        let quiet = view(&record(State::Running, None, Some(then)), &[], then + QUIET_LIMIT + Duration::from_secs(1));
        assert!(!quiet.healthy());
    }
}
