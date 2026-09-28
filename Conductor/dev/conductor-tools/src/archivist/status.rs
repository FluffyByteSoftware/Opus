//! File:       Opus/Conductor/dev/conductor-tools/src/archivist/status.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The running totals: how many jobs, how many were slow, the last few
//! slow ones.  The worker adds to them, and `archivist::status()` hands a
//! copy to whoever asks, which for now means whatever shows the admin how
//! the database is doing.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::clock::Utc;
use crate::scribe::{self, Channel};

/// How many slow jobs `status()` remembers.
const RECENT_SLOW: usize = 5;

/// How much of a job's SQL goes in the log and in `SlowJob`.
const LABEL_CHARS: usize = 80;

/// What a job was, for the read and write counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JobKind {
    /// `query()`, which hands back rows.
    Read,
    /// `execute()`, which changes rows and says how many.
    Write,
    /// `batch()` and `transaction()`, which can do either or both.
    Other,
}

/// One job that took longer than `slow_job_ms`.
#[derive(Debug, Clone)]
pub struct SlowJob {
    /// When it finished.
    pub when: Utc,
    /// The start of its SQL, or the name a transaction was given.  Never
    /// the values, so a password can't end up in here.
    pub label: String,
    /// How long it ran.
    pub ran_for: Duration,
    /// How long it sat in the mailbox before the worker picked it up.
    pub waited: Duration,
}

/// A copy of how Archivist is doing, right now.
#[derive(Debug, Clone)]
pub struct Status {
    /// Whether the worker thread is running.
    pub running: bool,
    /// Whether it has a connection to Postgres.
    pub connected: bool,
    /// How many jobs are in the mailbox waiting for the worker.
    pub waiting: usize,
    /// Jobs finished since Conductor started.
    pub jobs_done: u64,
    /// How many of those were `query()` jobs.
    pub reads: u64,
    /// How many were `execute()` jobs.
    pub writes: u64,
    /// How many were `batch()` or `transaction()` jobs.
    pub other: u64,
    /// How many of those were slow.
    pub slow_jobs: u64,
    /// The longest any job has run.
    pub slowest: Duration,
    /// The last few slow jobs, oldest first.
    pub recent_slow: Vec<SlowJob>,
}

/// The totals that need a lock, because they change together.
struct Totals {
    jobs_done: u64,
    reads: u64,
    writes: u64,
    other: u64,
    slow_jobs: u64,
    slowest: Duration,
    recent_slow: Vec<SlowJob>,
}

static TOTALS: Mutex<Totals> = Mutex::new(Totals {
    jobs_done: 0,
    reads: 0,
    writes: 0,
    other: 0,
    slow_jobs: 0,
    slowest: Duration::ZERO,
    recent_slow: Vec::new(),
});

// Rust note: an atomic is a value that threads can read and change without
// a lock.  An AtomicBool is a bool that way.
static CONNECTED: AtomicBool = AtomicBool::new(false);

/// The worker calls this when it connects and when it lets go.
pub(super) fn set_connected(connected: bool) {
    CONNECTED.store(connected, Ordering::SeqCst);
}

/// The worker calls this after every job.  A slow one goes in the log too.
pub(super) fn record(label: &str, kind: JobKind, waited: Duration, ran_for: Duration, slow_limit: Duration) {
    let slow = ran_for >= slow_limit;
    let label = short_label(label);

    if slow {
        scribe::warn(Channel::Database, &format!("Slow database job: {} ms, after {} ms in the mailbox.  {label}",
                                                 ran_for.as_millis(),
                                                 waited.as_millis()));
    }

    let mut guard = TOTALS.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.jobs_done += 1;
    match kind {
        JobKind::Read => guard.reads += 1,
        JobKind::Write => guard.writes += 1,
        JobKind::Other => guard.other += 1,
    }
    if ran_for > guard.slowest {
        guard.slowest = ran_for;
    }
    if slow {
        guard.slow_jobs += 1;
        guard.recent_slow.push(SlowJob { when: Utc::now(), label, ran_for, waited });
        if guard.recent_slow.len() > RECENT_SLOW {
            guard.recent_slow.remove(0);
        }
    }
}

/// Everything but `running` and `waiting`, which the worker side knows and
/// fills in.
pub(super) fn snapshot(running: bool, waiting: usize) -> Status {
    let guard = TOTALS.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Status {
        running,
        connected: CONNECTED.load(Ordering::SeqCst),
        waiting,
        jobs_done: guard.jobs_done,
        reads: guard.reads,
        writes: guard.writes,
        other: guard.other,
        slow_jobs: guard.slow_jobs,
        slowest: guard.slowest,
        recent_slow: guard.recent_slow.clone(),
    }
}

/// The SQL squashed onto one line and cut to `LABEL_CHARS`, with `...` on
/// the end if anything was cut.
fn short_label(label: &str) -> String {
    let one_line = label.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= LABEL_CHARS {
        return one_line;
    }
    let cut: String = one_line.chars().take(LABEL_CHARS).collect();
    format!("{cut}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_one_short_line() {
        assert_eq!(short_label("SELECT *\n    FROM accounts\n"), "SELECT * FROM accounts");

        let long = "x".repeat(200);
        let cut = short_label(&long);
        assert_eq!(cut.chars().count(), LABEL_CHARS + 3);
        assert!(cut.ends_with("..."));
    }
}
