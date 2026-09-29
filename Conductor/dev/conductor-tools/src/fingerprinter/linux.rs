//! File:       Opus/Conductor/dev/conductor-tools/src/fingerprinter/linux.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Random bytes on Linux: `getrandom()`, the kernel's own random source,
//! the same one behind `/dev/urandom`, without opening a file.  It's in the
//! C library every Linux program already has linked in.

use std::ffi::{c_uint, c_void};
use std::io;

// Rust note: an `extern` block lists functions that live outside Rust.
// Calling one is `unsafe`, since Rust can't check what the other side does
// with the pointer we hand it.
unsafe extern "C" {
    fn getrandom(buffer: *mut c_void, length: usize, flags: c_uint) -> isize;
}

/// Fills `bytes` with random bytes.  Linux can hand back fewer than asked
/// for (at most 32 MB a call, or fewer if a signal lands mid-call), so it
/// keeps asking until the whole buffer is full.
pub fn fill(bytes: &mut [u8]) -> io::Result<()> {
    let mut done = 0;
    while done < bytes.len() {
        let rest = &mut bytes[done..];
        // Safe because the pointer and the length both come from `rest`, a
        // slice we hold for the length of the call, so the kernel only
        // writes inside it.  Flags 0: the normal source, which only waits
        // in the first moments after the machine boots.
        let got = unsafe { getrandom(rest.as_mut_ptr().cast(), rest.len(), 0) };
        if got < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e);
        }
        done += got as usize;
    }
    Ok(())
}
