//! File:       Opus/Conductor/dev/tools/src/notices.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Notices: the things the admin has to see and acknowledge.  Every Warn
//! and Error Scribe logs becomes one, and any code can raise one of its own
//! with `publish()`.  The web admin's bell shows the newest, and the
//! Notifications History tab shows all of them.
//!
//! A notice stays open until somebody presses ACK on it (or ACK ALL), and
//! then it's gone for good.  Looking at it doesn't count.  So a problem
//! that happened at three in the morning is still sitting there when the
//! admin gets up.
//!
//! They're kept in memory only, since Conductor started.  A restart wipes
//! them, open or not.  That was a choice: the log file still has every
//! line, and saving notices would mean somewhere to save them that can't
//! itself be the thing that's broken.
//!
//! Nothing in here writes to Scribe.  Scribe calls in here, so a call the
//! other way could have each waiting on the other's lock.

use std::fmt;
use std::sync::Mutex;

use crate::clock::Utc;

/// How bad a notice is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Raised on purpose with `publish()`: something the admin should
    /// know, that isn't a Warn or an Error in the log.
    Notice,
    /// From a Warn in the log.
    Warn,
    /// From an Error in the log.
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// One open notice.
#[derive(Debug, Clone)]
pub struct Notice {
    /// Counts up from 1 since Conductor started.  The page ACKs by this.
    pub id: u64,
    pub when: Utc,
    pub level: Level,
    /// Where it came from: the log channel (`System`, `Database`, ...), or
    /// whatever `publish()` was told.
    pub source: String,
    pub text: String,
}

struct Notices {
    next_id: u64,
    /// Oldest first.
    open: Vec<Notice>,
}

static NOTICES: Mutex<Notices> = Mutex::new(Notices { next_id: 1, open: Vec::new() });

fn with_notices<T>(work: impl FnOnce(&mut Notices) -> T) -> T {
    let mut guard = NOTICES.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    work(&mut guard)
}

/// Raises a notice, and hands back its id.  It stays open until it's
/// ACKed on the page.
///
/// ```text
/// notices::publish(Level::Notice, "Game", "The world save took 40 seconds.");
/// ```
pub fn publish(level: Level, source: &str, text: &str) -> u64 {
    with_notices(|notices| {
        let id = notices.next_id;
        notices.next_id += 1;
        notices.open.push(Notice { id, when: Utc::now(), level, source: source.to_string(),
                                   text: text.to_string() });
        id
    })
}

/// How many are open, and the newest `count` of them, newest first.
pub fn newest(count: usize) -> (usize, Vec<Notice>) {
    with_notices(|notices| {
        let newest = notices.open.iter().rev().take(count).cloned().collect();
        (notices.open.len(), newest)
    })
}

/// Every open notice, newest first.
pub fn all() -> Vec<Notice> {
    with_notices(|notices| notices.open.iter().rev().cloned().collect())
}

/// Clears one notice.  False if there's no open notice with that id (it
/// was already ACKed, most likely from another browser).
pub fn ack(id: u64) -> bool {
    with_notices(|notices| {
        let before = notices.open.len();
        notices.open.retain(|notice| notice.id != id);
        notices.open.len() < before
    })
}

/// Clears every open notice, and says how many that was.
pub fn ack_all() -> usize {
    with_notices(|notices| {
        let cleared = notices.open.len();
        notices.open.clear();
        cleared
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // The list is shared by every test in the crate (Scribe's tests log
    // Warns too), so these only look at their own notices.

    #[test]
    fn a_notice_stays_until_it_is_acked() {
        let id = publish(Level::Notice, "Test", "stays until acked");
        assert!(all().iter().any(|notice| notice.id == id));
        assert!(ack(id));
        assert!(!all().iter().any(|notice| notice.id == id));
        assert!(!ack(id));
    }

    #[test]
    fn newest_comes_first() {
        let first = publish(Level::Warn, "Test", "first");
        let second = publish(Level::Error, "Test", "second");
        let ids: Vec<u64> = all().iter().map(|notice| notice.id).collect();
        let at = |id| ids.iter().position(|&each| each == id);
        assert!(at(second) < at(first));
        ack(first);
        ack(second);
    }
}
