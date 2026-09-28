//! File:       Opus/Conductor/dev/conductor-monitor/src/snapshot.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! A snapshot is what the monitor hands out: one look at Conductor, ready
//! for a person to read.  The OS only gives running totals ("this thread
//! has used 2.8 seconds of CPU so far"), so a percent or a speed needs two
//! readings and the time between them.  That math lives here, away from
//! the thread and the OS calls, so the tests can feed it made-up readings.

use std::time::{Duration, Instant};

use conductor_tools::archivist::{self, Status};
use conductor_tools::clock::Utc;
use conductor_tools::threads::{self, ThreadRecord};

use crate::probe::Reading;

/// One look at Conductor.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub taken_at: Utc,
    /// How long the monitor has been running, which is a moment less than
    /// Conductor has.
    pub uptime: Duration,
    /// Something like "Nobara Linux 42 (KDE Plasma), kernel 6.14.5".
    pub os: String,
    /// The OS's number for the Conductor process.
    pub process_id: u32,
    /// How many CPU cores the machine has, as far as Rust can tell.
    pub cores: usize,
    /// False on an OS the monitor can't measure yet (macOS, for now).  Then
    /// the numbers below are all empty, but the database and the threads
    /// we asked for still show.
    pub measured: bool,
    /// Conductor's share of the whole machine's CPU, 0 to 100, over the
    /// last second.  `None` on the very first look, with nothing to
    /// compare it to.
    pub cpu_percent: Option<f64>,
    /// Memory actually in RAM.
    pub memory_bytes: Option<u64>,
    pub disk: Option<Disk>,
    /// Every thread the OS says Conductor has, whoever started it.
    pub threads_in_use: Vec<ThreadInUse>,
    /// The threads our own code asked for, from `threads::spawn()`.
    pub threads_asked_for: Vec<ThreadRecord>,
    pub database: Status,
}

/// Disk totals since Conductor started, and the speed over the last second.
#[derive(Debug, Clone, Copy)]
pub struct Disk {
    pub read_bytes: u64,
    pub written_bytes: u64,
    pub read_per_second: Option<f64>,
    pub written_per_second: Option<f64>,
}

/// One thread the OS says Conductor has.
#[derive(Debug, Clone)]
pub struct ThreadInUse {
    pub os_id: u64,
    /// Our name for it if we started it, the OS's name if it has one, and
    /// "(no name)" if neither.
    pub name: String,
    /// True if `threads::spawn()` started it.
    pub ours: bool,
    /// How much of one core it used over the last second, 0 to 100.  One
    /// thread can only ever run on one core at a time, which is why it's
    /// "of one core" and not "of the machine".
    pub core_percent: Option<f64>,
    /// CPU time it has used since it started.
    pub cpu_time: Duration,
}

/// The last reading, and when it was taken.
pub(crate) struct Previous {
    pub(crate) at: Instant,
    pub(crate) reading: Reading,
}

/// What stays the same from one look to the next.
pub(crate) struct Fixed {
    pub(crate) os: String,
    pub(crate) cores: usize,
    pub(crate) started: Instant,
}

/// Puts a snapshot together from this reading, the last one, the
/// database's status and the list of our threads.
pub(crate) fn build(fixed: &Fixed, previous: Option<&Previous>, now: Instant, reading: Option<&Reading>) -> Snapshot {
    let ours = threads::list();
    // The time since the last reading.  Only when both readings are there.
    let since = match (previous, reading) {
        (Some(previous), Some(_)) => Some(now.duration_since(previous.at)),
        _ => None,
    };

    let cpu_percent = match (previous, reading, since) {
        (Some(previous), Some(reading), Some(since)) => {
            let used = reading.cpu_time.saturating_sub(previous.reading.cpu_time);
            Some(percent(used, since) / fixed.cores.max(1) as f64)
        }
        _ => None,
    };

    let disk = reading.and_then(|reading| reading.disk).map(|now_disk| {
        let before = previous.and_then(|previous| previous.reading.disk);
        let speed = |now_total: u64, before_total: Option<u64>| -> Option<f64> {
            Some(per_second(now_total, before_total?, since?))
        };
        Disk {
            read_bytes: now_disk.read,
            written_bytes: now_disk.written,
            read_per_second: speed(now_disk.read, before.map(|before| before.read)),
            written_per_second: speed(now_disk.written, before.map(|before| before.written)),
        }
    });

    let threads_in_use = match reading {
        Some(reading) => threads_in_use(reading, previous.map(|previous| &previous.reading), since, &ours),
        None => Vec::new(),
    };

    Snapshot {
        taken_at: Utc::now(),
        uptime: now.duration_since(fixed.started),
        os: fixed.os.clone(),
        process_id: std::process::id(),
        cores: fixed.cores,
        measured: reading.is_some(),
        cpu_percent,
        memory_bytes: reading.map(|reading| reading.memory_bytes),
        disk,
        threads_in_use,
        threads_asked_for: ours,
        database: archivist::status(),
    }
}

/// Every thread in the reading, matched up to our list by the OS's
/// number, with its share of a core since the last reading.
fn threads_in_use(reading: &Reading,
                  previous: Option<&Reading>,
                  since: Option<Duration>,
                  ours: &[ThreadRecord]) -> Vec<ThreadInUse> {
    reading.threads.iter().map(|thread| {
        // Only a running thread counts.  The OS hands a finished thread's
        // number out again, and an old entry on our list shouldn't claim
        // somebody else's thread.
        let record = ours.iter().find(|record| record.running && record.os_id == Some(thread.os_id));
        let before = previous
            .and_then(|previous| previous.threads.iter().find(|old| old.os_id == thread.os_id));
        let core_percent = match (before, since) {
            (Some(before), Some(since)) => Some(percent(thread.cpu_time.saturating_sub(before.cpu_time), since)),
            _ => None,
        };

        let name = match (record, &thread.name) {
            (Some(record), _) => record.name.clone(),
            (None, Some(name)) => name.clone(),
            (None, None) => "(no name)".to_string(),
        };

        ThreadInUse { os_id: thread.os_id, name, ours: record.is_some(), core_percent, cpu_time: thread.cpu_time }
    }).collect()
}

/// `used` as a percent of `over`.  The CPU time used in a second, over that
/// second, is how much of one core it kept busy.
fn percent(used: Duration, over: Duration) -> f64 {
    if over.is_zero() {
        return 0.0;
    }
    used.as_secs_f64() / over.as_secs_f64() * 100.0
}

/// How fast a running total grew, per second.
fn per_second(now_total: u64, before_total: u64, over: Duration) -> f64 {
    if over.is_zero() {
        return 0.0;
    }
    now_total.saturating_sub(before_total) as f64 / over.as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::{DiskTotals, ThreadReading};

    fn reading(cpu_ms: u64, read: u64, threads: &[(u64, u64)]) -> Reading {
        Reading {
            cpu_time: Duration::from_millis(cpu_ms),
            memory_bytes: 1024,
            disk: Some(DiskTotals { read, written: 0 }),
            threads: threads.iter()
                .map(|&(os_id, cpu_ms)| ThreadReading {
                    os_id,
                    name: Some(format!("os-{os_id}")),
                    cpu_time: Duration::from_millis(cpu_ms),
                })
                .collect(),
        }
    }

    #[test]
    fn half_a_second_of_cpu_in_a_second_is_fifty_percent() {
        assert_eq!(percent(Duration::from_millis(500), Duration::from_secs(1)), 50.0);
        assert_eq!(percent(Duration::from_millis(500), Duration::ZERO), 0.0);
        assert_eq!(per_second(3_000, 1_000, Duration::from_secs(2)), 1_000.0);
    }

    #[test]
    fn the_first_look_has_no_percents_and_the_second_does() {
        let fixed = Fixed { os: "Test OS".to_string(), cores: 4, started: Instant::now() };
        let at = Instant::now();
        let first = reading(1_000, 0, &[(10, 400), (11, 600)]);

        let snapshot = build(&fixed, None, at, Some(&first));
        assert!(snapshot.measured);
        assert_eq!(snapshot.cpu_percent, None);
        assert_eq!(snapshot.threads_in_use.len(), 2);
        assert_eq!(snapshot.threads_in_use[0].core_percent, None);

        // One second later: 2 seconds of CPU across 4 cores is 50% of the
        // machine, and thread 10 kept one core fully busy.
        let previous = Previous { at, reading: first };
        let second = reading(3_000, 4_096, &[(10, 1_400), (11, 600), (12, 0)]);
        let snapshot = build(&fixed, Some(&previous), at + Duration::from_secs(1), Some(&second));

        assert_eq!(snapshot.cpu_percent, Some(50.0));
        assert_eq!(snapshot.disk.and_then(|disk| disk.read_per_second), Some(4_096.0));
        assert_eq!(snapshot.threads_in_use[0].core_percent, Some(100.0));
        assert_eq!(snapshot.threads_in_use[1].core_percent, Some(0.0));
        // A thread that wasn't there last time has nothing to compare to.
        assert_eq!(snapshot.threads_in_use[2].core_percent, None);
        assert_eq!(snapshot.threads_in_use[2].name, "os-12");
    }

    #[test]
    fn nothing_measured_still_makes_a_snapshot() {
        let fixed = Fixed { os: "Somewhere".to_string(), cores: 1, started: Instant::now() };
        let snapshot = build(&fixed, None, Instant::now(), None);
        assert!(!snapshot.measured);
        assert_eq!(snapshot.memory_bytes, None);
        assert!(snapshot.disk.is_none());
        assert!(snapshot.threads_in_use.is_empty());
    }
}
