//! File:       Opus/Conductor/dev/conductor-monitor/src/probe.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! One raw look at the process, straight from the OS: CPU time used so
//! far, memory, bytes read and written, and every thread with its own CPU
//! time.  Plus three things about the whole machine: how busy each core has
//! been, how much RAM it has and how much of that is free, and every
//! process running on it.  Totals only.  Turning two looks a second apart into a percent or a speed is
//! `snapshot.rs`'s job.
//!
//! Every OS keeps these numbers somewhere different, so there is a file
//! for each: `probe/linux.rs` reads `/proc`, `probe/windows.rs` asks
//! kernel32, and `probe/other.rs` is for anything else (macOS, for now),
//! where `read()` says it can't.  Only the one for the OS being built for
//! is compiled, and all three hand back the same `Reading`.
//!
//! `threads_of(pid)` is the one thing asked for on its own: the threads of
//! one other process, for when the admin clicks it on the page.  Every
//! process's threads every second would be thousands of them, for a list
//! nobody is looking at.

use std::time::Duration;

// Rust note: `#[cfg(...)]` keeps a line in or out of the build depending on
// what it's being built for.  So on Linux, `probe::read()` is
// `linux::read()`, and the other two files aren't compiled at all.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{os_name, read, threads_of};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{os_name, read, threads_of};

#[cfg(not(any(target_os = "linux", windows)))]
mod other;
#[cfg(not(any(target_os = "linux", windows)))]
pub use other::{os_name, read, threads_of};

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
    /// Every core on the machine, in order.  Empty when the OS won't say.
    pub cores: Vec<CoreTimes>,
    /// The machine's RAM.  `None` when the OS won't say.
    pub machine_memory: Option<MachineMemory>,
    /// Every process on the machine we're allowed to see, Conductor
    /// included.  Empty when the OS won't list them.
    pub processes: Vec<ProcessReading>,
}

/// One process on the machine, as the OS sees it.  A process run by
/// another user (or the OS itself) can be listed without letting us read
/// its numbers, and those are `None`.
#[derive(Debug, Clone)]
pub struct ProcessReading {
    pub pid: u32,
    /// The program's name.  Linux cuts it to 15 characters.
    pub name: String,
    /// CPU time used by all its threads since it started.
    pub cpu_time: Option<Duration>,
    /// Memory actually in RAM.
    pub memory_bytes: Option<u64>,
    /// How many threads it has.
    pub threads: Option<u32>,
}

/// One core's running totals: time spent busy, and time in all.  The unit
/// is different on each OS (ticks on Linux, 100 ns steps on Windows), and
/// it doesn't matter, since all we ever do is divide one by the other.
#[derive(Debug, Clone, Copy)]
pub struct CoreTimes {
    pub busy: u64,
    pub total: u64,
}

/// The whole machine's RAM.
#[derive(Debug, Clone, Copy)]
pub struct MachineMemory {
    pub total_bytes: u64,
    /// What could be handed to a program right now without swapping.  Free
    /// memory plus the cache the OS would give up.
    pub available_bytes: u64,
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
