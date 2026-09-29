//! File:       Opus/Conductor/dev/conductor-tools/src/security/other.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Anywhere that isn't Linux or Windows (macOS, for now): no huge page
//! hint yet.  The arena runs on ordinary pages.

use std::io;

/// Always says no, and why.
pub fn advise_huge_pages(_start: *mut u8, _bytes: usize) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "no huge page hint on this OS yet"))
}
