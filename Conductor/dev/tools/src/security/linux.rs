//! File:       Opus/Conductor/dev/tools/src/security/linux.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Linux: asks the kernel to back the arena with huge pages, through
//! `madvise()` from the C library every program already has.  It's a
//! hint, not an order.  The kernel goes along with it when transparent
//! huge pages are set to `madvise` or `always` (the file is
//! `/sys/kernel/mm/transparent_hugepage/enabled`), and ignores it on
//! `never`.  Either way the arena works; the hint only changes how fast.

use std::ffi::{c_int, c_long, c_void};
use std::io;

/// `MADV_HUGEPAGE` from the kernel's headers.
const MADV_HUGEPAGE: c_int = 14;

/// `_SC_PAGESIZE` from the C library's headers, for `sysconf()`.
const SC_PAGESIZE: c_int = 30;

// Rust note: an `extern` block lists functions that live outside Rust,
// here in the C library.  Calling one is `unsafe`, since Rust can't check
// what the other side does with a pointer, so each call below says why
// it's fine.
unsafe extern "C" {
    fn madvise(addr: *mut c_void, length: usize, advice: c_int) -> c_int;
    fn sysconf(name: c_int) -> c_long;
}

/// Marks the `bytes` starting at `start` for huge pages.  `madvise()`
/// wants a span of whole pages, and the allocator doesn't promise the
/// arena starts on one, so the span is trimmed inward to whole pages: the
/// start rounded up, the end rounded down.  The slivers left at each end
/// stay on ordinary pages, which costs nothing worth measuring.
pub fn advise_huge_pages(start: *mut u8, bytes: usize) -> io::Result<()> {
    // SAFETY: sysconf() reads a constant out of the C library and touches
    // nothing of ours.
    let page = unsafe { sysconf(SC_PAGESIZE) };
    let Ok(page) = usize::try_from(page) else {
        return Err(io::Error::last_os_error());
    };
    if page == 0 {
        return Err(io::Error::other("the C library says the page size is 0"));
    }

    let first = (start as usize).div_ceil(page) * page;
    let last = (start as usize + bytes) / page * page;
    if last <= first {
        return Err(io::Error::other("the arena is smaller than a page"));
    }

    // SAFETY: the span lies inside the memory the caller owns (it was
    // trimmed inward at both ends, never outward), and MADV_HUGEPAGE only
    // changes how the kernel backs those pages, never what's in them.
    let answer = unsafe { madvise(first as *mut c_void, last - first, MADV_HUGEPAGE) };
    if answer == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
