//! File:       Opus/Conductor/dev/conductor-monitor/src/probe/other.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Any OS that isn't Linux or Windows, which for now means macOS.  Conductor
//! builds and runs there, but the monitor can't measure anything yet, and
//! the web admin says so.  macOS keeps these numbers behind its own calls
//! (`proc_pidinfo` and friends), and that's a job for when there's a Mac
//! to test it on.

use super::{Reading, ThreadReading};

/// Always `None`: nothing measured here yet.
pub fn read() -> Option<Reading> {
    None
}

/// Always `None`: no other process can be looked at here yet.
pub fn threads_of(_pid: u32) -> Option<Vec<ThreadReading>> {
    None
}

/// "macos", "freebsd" and so on, as Rust names them, with macOS spelled
/// the way Apple spells it.
pub fn os_name() -> String {
    match std::env::consts::OS {
        "macos" => "macOS".to_string(),
        other => other.to_string(),
    }
}
