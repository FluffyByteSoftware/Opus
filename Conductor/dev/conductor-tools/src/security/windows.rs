//! File:       Opus/Conductor/dev/conductor-tools/src/security/windows.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Windows: no hint like Linux's.  Large pages there need a privilege the
//! process doesn't have by default (`SeLockMemoryPrivilege`) and a
//! different allocation call (`VirtualAlloc` with `MEM_LARGE_PAGES`), so
//! there's nothing to do to memory we already have.  The arena runs on
//! ordinary pages, and the benchmark's "huge" column reads the same as
//! "arena".  Never built yet, like every Windows file.

use std::io;

/// Always says no, and why.
pub fn advise_huge_pages(_start: *mut u8, _bytes: usize) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported,
                       "Windows has no huge page hint for memory already allotted"))
}
