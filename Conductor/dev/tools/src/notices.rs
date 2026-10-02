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
    /// How many have been dropped off the old end, since Conductor started.
    dropped: u64,
}

static NOTICES: Mutex<Notices> = Mutex::new(Notices { next_id: 1, open: Vec::new(), dropped: 0 });

/// The most notices kept open.  Past this the oldest go, and one notice
/// says how many have gone that way, so a Warn that keeps coming (a
/// script tripping its limit all weekend) can't eat the memory or make
/// the History tab copy thousands of lines a second.
pub const MOST_OPEN: usize = 1_000;

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
    with_notices(|notices| publish_in(notices, level, source, text))
}

fn publish_in(notices: &mut Notices, level: Level, source: &str, text: &str) -> u64 {
    let id = notices.next_id;
    notices.next_id += 1;
    notices.open.push(Notice { id, when: Utc::now(), level, source: source.to_string(), text: text.to_string() });
    if notices.open.len() > MOST_OPEN {
        let over = notices.open.len() - MOST_OPEN;
        notices.open.drain(..over);
        notices.dropped += over as u64;
        let dropped = notices.dropped;
        // Written over the oldest one left, so there's always exactly one
        // line saying so, and it's at the top of the list.
        let first = &mut notices.open[0];
        first.level = Level::Notice;
        first.source = "Notices".to_string();
        first.text = format!("{dropped} older notice(s) were dropped: only the newest {MOST_OPEN} are kept.  The \
            log has them all.");
    }
    id
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
    fn past_the_cap_the_oldest_go_and_the_oldest_left_says_so() {
        // A list of the test's own, since the shared one would take a
        // thousand lines to fill and trip the other tests.
        let mut notices = Notices { next_id: 1, open: Vec::new(), dropped: 0 };
        for n in 0..MOST_OPEN {
            publish_in(&mut notices, Level::Warn, "Test", &format!("warn {n}"));
        }
        assert_eq!(notices.open.len(), MOST_OPEN);
        assert_eq!(notices.open[0].text, "warn 0");

        publish_in(&mut notices, Level::Warn, "Test", "one too many");
        assert_eq!(notices.open.len(), MOST_OPEN);
        assert_eq!(notices.dropped, 1);
        assert_eq!(notices.open[0].level, Level::Notice);
        assert!(notices.open[0].text.starts_with("1 older notice(s) were dropped"), "{}", notices.open[0].text);
        assert_eq!(notices.open[1].text, "warn 2");
        assert_eq!(notices.open[MOST_OPEN - 1].text, "one too many");

        publish_in(&mut notices, Level::Warn, "Test", "and another");
        assert_eq!(notices.dropped, 2);
        assert!(notices.open[0].text.starts_with("2 older notice(s) were dropped"), "{}", notices.open[0].text);
        assert_eq!(notices.open[1].text, "warn 3");
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
