//! File:       Opus/Conductor/dev/conductor-tools/src/fingerprinter.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Fingerprinter, the UUID maker.  Anything on the server that needs a name
//! nothing else will ever have (an account, a character, a login token)
//! gets one from here.
//!
//! A UUID here is 16 random bytes from the OS, with two of them bent to
//! mark it as a "version 4" (random) UUID, written in the usual dashed
//! form:
//!
//! ```text
//! 3f2a91c0-e4b7-4d1a-9c0e-2b7f5a6d8e10
//! ```
//!
//! The random bytes come straight from the OS, never from a file.  Reading
//! `/dev/urandom` would have to go through DiskMan, and DiskMan keeps what
//! it reads, so every UUID would come out the same.  Each OS gets a file of
//! its own under `fingerprinter/`, with one function, `fill()`:
//!
//! - Linux: `getrandom()`, from the C library every program already has.
//! - Windows: `BCryptGenRandom()`, from `bcrypt.dll`.  Never built yet.
//! - Anything else: says it can't, and Fingerprinter shows as in trouble.
//!
//! No crate.  Nothing in here logs but `start()`.  A caller that can't get
//! a UUID knows best what that means for it.

use std::io;

use crate::scribe::{self, Channel};
use crate::services::{self, State};

// Rust note: `#[cfg(...)]` keeps a line in or out of the build depending on
// what it's being built for.  On Linux, `fill()` is `linux::fill()`, and
// the other two files aren't compiled at all.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::fill;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::fill;

#[cfg(not(any(target_os = "linux", windows)))]
mod other;
#[cfg(not(any(target_os = "linux", windows)))]
use other::fill;

/// Asks the OS for 16 bytes once, so a machine that can't give them is
/// caught at startup, not the first time a player makes an account.  The
/// launcher calls this after Constellations and before Archivist.
pub fn start() {
    services::set(services::FINGERPRINTER, State::Starting, "Asking the OS for random bytes.");

    let mut test = [0u8; 16];
    match fill(&mut test) {
        Ok(()) => {
            services::set(services::FINGERPRINTER, State::Running, "The OS's random source works.");
            scribe::info(Channel::System, "Fingerprinter is running.");
        }
        Err(e) => {
            services::set(services::FINGERPRINTER, State::Trouble, &format!("The OS won't give random bytes: {e}"));
            scribe::error_with(Channel::System, &e, "FINGERPRINTER CAN'T GET RANDOM BYTES FROM THE OS.  \
                Nothing can be given a UUID or a token.");
        }
    }
}

/// A new random UUID, in the usual form.  The only way it fails is if the
/// OS won't hand over random bytes, which means something is badly wrong
/// with the machine.
pub fn new_uuid() -> io::Result<String> {
    let mut bytes = [0u8; 16];
    fill(&mut bytes)?;

    // The top four bits of byte 6 say the version (4).  The top two bits of
    // byte 8 say which UUID layout this is (the standard one).
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let mut text = String::with_capacity(36);
    for (position, byte) in bytes.iter().enumerate() {
        if position == 4 || position == 6 || position == 8 || position == 10 {
            text.push('-');
        }
        text.push_str(&format!("{byte:02x}"));
    }
    Ok(text)
}

/// A new login token: 32 random bytes, written as 64 lowercase hex
/// characters.  Nothing about it means anything.  It only has to be
/// impossible to guess.
pub fn new_token() -> io::Result<String> {
    let mut bytes = [0u8; 32];
    fill(&mut bytes)?;

    let mut text = String::with_capacity(64);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    Ok(text)
}

/// True for text in the shape of one of our UUIDs: 36 characters, lowercase
/// hex, dashes in the right places, version 4.  It says nothing about
/// whether anything has that UUID.  For checking a file or a message that
/// claims to hold one.
///
/// Postgres hands a UUID back in this same form, so one read out of the
/// database passes too.
pub fn looks_like_uuid(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (position, byte) in bytes.iter().enumerate() {
        let wanted_dash = position == 8 || position == 13 || position == 18 || position == 23;
        let is_dash = *byte == b'-';
        if wanted_dash != is_dash {
            return false;
        }
        if !is_dash && !(byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)) {
            return false;
        }
    }
    bytes[14] == b'4' && b"89ab".contains(&bytes[19])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuids_look_right() {
        let first = new_uuid().unwrap();
        let second = new_uuid().unwrap();

        assert_eq!(first.len(), 36);
        for position in [8, 13, 18, 23] {
            assert_eq!(first.as_bytes()[position], b'-');
        }
        assert_eq!(first.as_bytes()[14], b'4');
        assert!("89ab".contains(first.as_bytes()[19] as char));
        assert!(first.chars().all(|c| c == '-' || c.is_ascii_hexdigit()));
        assert!(!first.chars().any(|c| c.is_ascii_uppercase()));

        assert_ne!(first, second);
    }

    #[test]
    fn tokens_look_right() {
        let first = new_token().unwrap();
        let second = new_token().unwrap();

        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
        assert_ne!(first, second);
    }

    #[test]
    fn a_thousand_uuids_are_all_different() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1000 {
            assert!(seen.insert(new_uuid().unwrap()));
        }
    }

    #[test]
    fn fill_fills_more_than_one_call_can() {
        // Linux hands back at most 32 MB a call, and fill() has to keep
        // asking until the buffer is full.  Anything past that is enough
        // to see it loop.  A buffer left at zero would be all zeros.
        let mut bytes = vec![0u8; 33 * 1024 * 1024];
        fill(&mut bytes).unwrap();
        assert!(bytes[bytes.len() - 64..].iter().any(|byte| *byte != 0));
    }

    #[test]
    fn our_own_uuids_pass_the_shape_check() {
        for _ in 0..20 {
            assert!(looks_like_uuid(&new_uuid().unwrap()));
        }
        assert!(looks_like_uuid("3f2a91c0-e4b7-4d1a-9c0e-2b7f5a6d8e10"));
    }

    #[test]
    fn things_that_are_not_our_uuids_fail_it() {
        assert!(!looks_like_uuid(""));
        assert!(!looks_like_uuid("3f2a91c0-e4b7-4d1a-9c0e-2b7f5a6d8e1"));
        assert!(!looks_like_uuid("3f2a91c0-e4b7-4d1a-9c0e-2b7f5a6d8e100"));
        // Uppercase, which we never write.
        assert!(!looks_like_uuid("3F2A91C0-E4B7-4D1A-9C0E-2B7F5A6D8E10"));
        // A dash in the wrong place.
        assert!(!looks_like_uuid("3f2a91c0e-4b7-4d1a-9c0e-2b7f5a6d8e10"));
        // Not version 4.
        assert!(!looks_like_uuid("3f2a91c0-e4b7-1d1a-9c0e-2b7f5a6d8e10"));
        // The wrong layout bits.
        assert!(!looks_like_uuid("3f2a91c0-e4b7-4d1a-1c0e-2b7f5a6d8e10"));
        // Not hex.
        assert!(!looks_like_uuid("3f2a91c0-e4b7-4d1a-9c0e-2b7f5a6d8g10"));
    }
}
