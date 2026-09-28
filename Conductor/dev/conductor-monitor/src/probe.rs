//! File:       Opus/Conductor/dev/conductor-monitor/src/probe.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! One raw look at the process, straight from the OS: CPU time used so
//! far, memory, bytes read and written, and every thread with its own CPU
//! time.  Totals only.  Turning two looks a second apart into a percent or
//! a speed is `snapshot.rs`'s job.
//!
//! Every OS keeps these numbers somewhere different, so there is a file
//! for each: `probe/linux.rs` reads `/proc`, `probe/windows.rs` asks
//! kernel32, and `probe/other.rs` is for anything else (macOS, for now),
//! where `read()` says it can't.  Only the one for the OS being built for
//! is compiled, and all three hand back the same `Reading`.

use std::time::Duration;

// Rust note: `#[cfg(...)]` keeps a line in or out of the build depending on
// what it's being built for.  So on Linux, `probe::read()` is
// `linux::read()`, and the other two files aren't compiled at all.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{os_name, read};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{os_name, read};

#[cfg(not(any(target_os = "linux", windows)))]
mod other;
#[cfg(not(any(target_os = "linux", windows)))]
pub use other::{os_name, read};

/// One look at the process.  Everything is a running total since the
/// process started.
#[derive(Debug, Clone)]
pub struct Reading {
    /// CPU time used, by every thread together.
    pub cpu_time: Duration,
    /// Memory in use: what's actually in RAM (Linux calls it the resident
    /// set, Windows the working set).
    pub memory_bytes: u64,
    /// Bytes read and written.  `None` when the OS won't say.
    pub disk: Option<DiskTotals>,
    /// Every thread the OS says the process has right now.
    pub threads: Vec<ThreadReading>,
}

/// Bytes read and written since the process started.
#[derive(Debug, Clone, Copy)]
pub struct DiskTotals {
    pub read: u64,
    pub written: u64,
}

/// One thread, as the OS sees it.
#[derive(Debug, Clone)]
pub struct ThreadReading {
    /// The OS's number for it.  `threads::ThreadRecord::os_id` is the same
    /// number, which is how the two get matched up.
    pub os_id: u64,
    /// The name the OS has for it, where it keeps one.  Linux cuts names
    /// to 15 characters.
    pub name: Option<String>,
    /// CPU time this thread has used.
    pub cpu_time: Duration,
}
