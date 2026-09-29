//! File:       Opus/Conductor/dev/conductor-tools/src/fingerprinter/windows.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Random bytes on Windows: `BCryptGenRandom()` from `bcrypt.dll`, asking
//! for the system's own generator.  Never built yet.

use std::ffi::c_void;
use std::io;
use std::ptr;

/// Tells `BCryptGenRandom` to use the system's generator, so we don't have
/// to open one of our own first.
const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;

// Rust note: an `extern` block lists functions that live outside Rust.
// Calling one is `unsafe`, since Rust can't check what the other side does
// with the pointer we hand it.
#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptGenRandom(algorithm: *mut c_void, buffer: *mut u8, length: u32, flags: u32) -> i32;
}

/// Fills `bytes` with random bytes.  Windows takes a 32-bit length, so a
/// buffer past 4 GB goes in pieces.  Ours are 16 or 32 bytes.
pub fn fill(bytes: &mut [u8]) -> io::Result<()> {
    for piece in bytes.chunks_mut(u32::MAX as usize) {
        // Safe because the pointer and the length both come from `piece`, a
        // slice we hold for the length of the call, so Windows only writes
        // inside it.  No algorithm handle is fine with the flag above.
        let status = unsafe {
            BCryptGenRandom(ptr::null_mut(), piece.as_mut_ptr(), piece.len() as u32, BCRYPT_USE_SYSTEM_PREFERRED_RNG)
        };
        // 0 is success.  Anything else is an NTSTATUS error code.
        if status != 0 {
            return Err(io::Error::other(format!("BCryptGenRandom failed with 0x{status:08x}")));
        }
    }
    Ok(())
}
