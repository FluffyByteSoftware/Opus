//! File:       Opus/Conductor/dev/conductor-tools/src/fingerprinter/other.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Any OS but Linux and Windows.  Conductor builds here, but there's no
//! random source hooked up, so it says so instead of making UUIDs.

use std::io;

/// Always turns the caller away.  Fingerprinter shows as in trouble.
pub fn fill(_bytes: &mut [u8]) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "no random source hooked up on this OS yet"))
}
