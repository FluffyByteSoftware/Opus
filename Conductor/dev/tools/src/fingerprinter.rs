//! File:       Opus/Conductor/dev/tools/src/fingerprinter.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Fingerprinter, the UUID maker.  Anything on the server that needs a name
//! nothing else will ever have (an account, a character, a login token)
//! gets one from here.
//!
//! A UUID here is "version 7": the time it was made, then a counter, then
//! random bytes, written in the usual dashed form:
//!
//! ```text
//! 01890a5d-ac96-774b-bcce-b302099a8057
//! -------- ----  ---  ---- ------------
//!   time, in ms   |       random
//!               counter
//! ```
//!
//! The `7` before the counter says the version, and the first character
//! of the random part is always 8, 9, a or b, which says the layout.
//!
//! So sorting UUIDs sorts them by when they were made, and a pile of
//! similar rows reads in the order things happened.  Two made in the same
//! millisecond still sort in the order they were asked for: the counter
//! goes up by one each time.  Postgres 18 makes the same kind with
//! `uuidv7()`, which is the safety net on every table's `uuid` column.
//!
//! Anyone holding one can read when it was made (`uuid_time()`).  That's
//! fine for a name.  A login token is all random, since it must not be
//! guessable.
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
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::clock::Utc;
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

/// The time and counter of the last UUID made.  The next one always sorts
/// after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    /// Milliseconds since 1970-01-01 00:00:00 UTC.  Only 48 bits fit in
    /// a UUID, which lasts until the year 10889.
    millis: u64,
    /// 12 bits: 0 to 4095.
    counter: u16,
}

/// The largest counter that fits in its 12 bits.
const COUNTER_MAX: u16 = 0x0fff;

/// A new millisecond starts the counter somewhere from 0 to 2047, so
/// there's always room for at least 2048 more before it runs out.
const COUNTER_START_MAX: u16 = 0x07ff;

// Rust note: a `static` is one value for the whole program.  The Mutex
// makes threads take turns, so two asking at once still get two different
// stamps, one after the other.
static LAST: Mutex<Option<Stamp>> = Mutex::new(None);

/// Asks the OS for 16 bytes once, so a machine that can't give them is
/// caught when the server starts, not the first time a player makes an
/// account.  The launcher calls this first thing on a START SERVER,
/// before Archivist.
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

/// The server is stopping.  There's no thread to end and nothing to let
/// go of, and `new_uuid()` still works after this.  It only tells the
/// Services tab, so Fingerprinter doesn't show as running while the rest
/// of the server is stopped.
pub fn stop() {
    services::set(services::FINGERPRINTER, State::Stopped, "The server is stopped.");
}

/// A new UUID, in the usual form, that sorts after every one made before
/// it.  The only way it fails is if the OS won't hand over random bytes,
/// which means something is badly wrong with the machine.
pub fn new_uuid() -> io::Result<String> {
    let mut bytes = [0u8; 16];
    fill(&mut bytes)?;

    // Two of the random bytes pick where the counter starts, should this
    // be a new millisecond.  They're written over below.
    let fresh_counter = u16::from_be_bytes([bytes[6], bytes[7]]) & COUNTER_START_MAX;

    let stamp = {
        let mut last = LAST.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let stamp = next_stamp(*last, now_millis(), fresh_counter);
        *last = Some(stamp);
        stamp
    };

    // Bytes 0 to 5: the time, biggest part first, so it sorts.
    let time = stamp.millis.to_be_bytes();
    bytes[..6].copy_from_slice(&time[2..]);
    // The top four bits of byte 6 say the version (7), and the counter
    // fills the rest of 6 and all of 7.  The top two bits of byte 8 say
    // which UUID layout this is (the standard one).  The rest is random.
    bytes[6] = 0x70 | (stamp.counter >> 8) as u8;
    bytes[7] = stamp.counter as u8;
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

/// Fills `buffer` with random bytes from the OS, for anything that needs
/// raw random bytes and isn't a UUID or a token.  Security's salts, today.
/// Fails only if the OS won't hand them over.
pub fn random_bytes(buffer: &mut [u8]) -> io::Result<()> {
    fill(buffer)
}

/// True for text in the shape of one of our UUIDs: 36 characters, lowercase
/// hex, dashes in the right places, version 7.  It says nothing about
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
    bytes[14] == b'7' && b"89ab".contains(&bytes[19])
}

/// When a UUID was made, to the second.  Works for Postgres's `uuidv7()`
/// too.  `None` for text that isn't one of ours.
pub fn uuid_time(text: &str) -> Option<Utc> {
    let millis = uuid_millis(text)?;
    Some(Utc::from_unix((millis / 1000) as i64))
}

/// The milliseconds since 1970 a UUID was made at.
fn uuid_millis(text: &str) -> Option<u64> {
    if !looks_like_uuid(text) {
        return None;
    }
    // The first 12 hex characters, skipping the dash, are the time.
    let hex = format!("{}{}", &text[0..8], &text[9..13]);
    u64::from_str_radix(&hex, 16).ok()
}

/// Milliseconds since 1970 by the machine's clock.  A clock set before
/// 1970 reads as 0, and `next_stamp()` keeps the order anyway.
fn now_millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|since| since.as_millis() as u64).unwrap_or(0)
}

/// The stamp for the next UUID, given the last one.
///
/// - A new millisecond: that time, and the counter starts fresh.
/// - The same millisecond, or the clock stepped back: the last time, and
///   the counter goes up one.  The clock is never trusted to go forward.
/// - The counter is full: the last time plus one millisecond, borrowed
///   from the future.  The clock catches up a moment later.
fn next_stamp(last: Option<Stamp>, now_millis: u64, fresh_counter: u16) -> Stamp {
    match last {
        Some(last) if now_millis <= last.millis => {
            if last.counter < COUNTER_MAX {
                Stamp { millis: last.millis, counter: last.counter + 1 }
            } else {
                Stamp { millis: last.millis + 1, counter: fresh_counter }
            }
        }
        _ => Stamp { millis: now_millis, counter: fresh_counter },
    }
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
        assert_eq!(first.as_bytes()[14], b'7');
        assert!("89ab".contains(first.as_bytes()[19] as char));
        assert!(first.chars().all(|c| c == '-' || c.is_ascii_hexdigit()));
        assert!(!first.chars().any(|c| c.is_ascii_uppercase()));

        assert_ne!(first, second);
    }

    #[test]
    fn ten_thousand_in_a_row_sort_in_the_order_they_were_made() {
        // Lowercase hex of a fixed width sorts as text the same way the
        // bytes do, and the same way Postgres sorts a UUID column.
        let uuids: Vec<String> = (0..10_000).map(|_| new_uuid().unwrap()).collect();
        for pair in uuids.windows(2) {
            assert!(pair[0] < pair[1], "{} came before {}", pair[0], pair[1]);
        }
    }

    #[test]
    fn the_first_stamp_takes_the_clock() {
        assert_eq!(next_stamp(None, 100, 5), Stamp { millis: 100, counter: 5 });
    }

    #[test]
    fn a_new_millisecond_starts_the_counter_fresh() {
        let last = Stamp { millis: 100, counter: 900 };
        assert_eq!(next_stamp(Some(last), 101, 5), Stamp { millis: 101, counter: 5 });
    }

    #[test]
    fn the_same_millisecond_counts_up() {
        let last = Stamp { millis: 100, counter: 3 };
        assert_eq!(next_stamp(Some(last), 100, 5), Stamp { millis: 100, counter: 4 });
    }

    #[test]
    fn a_clock_that_steps_back_is_ignored() {
        let last = Stamp { millis: 100, counter: 3 };
        assert_eq!(next_stamp(Some(last), 90, 5), Stamp { millis: 100, counter: 4 });
    }

    #[test]
    fn a_full_counter_borrows_the_next_millisecond() {
        let last = Stamp { millis: 100, counter: COUNTER_MAX };
        assert_eq!(next_stamp(Some(last), 100, 5), Stamp { millis: 101, counter: 5 });
    }

    #[test]
    fn uuid_time_reads_the_time_back() {
        // 0x01890a5dac96 ms is 2023-06-30 03:34:18.518 UTC.
        let when = uuid_time("01890a5d-ac96-774b-bcce-b302099a8057").unwrap();
        assert_eq!(when, Utc::from_unix(1_688_096_058));
        assert_eq!(uuid_time("not a uuid"), None);

        // One made now reads back as now.  Other tests run alongside and
        // can push the stamp a few milliseconds ahead, so allow a second.
        let before = now_millis();
        let made = uuid_millis(&new_uuid().unwrap()).unwrap();
        let after = now_millis();
        assert!(made >= before && made <= after + 1000, "{before} <= {made} <= {after}");
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
        assert!(looks_like_uuid("01890a5d-ac96-774b-bcce-b302099a8057"));
    }

    #[test]
    fn things_that_are_not_our_uuids_fail_it() {
        assert!(!looks_like_uuid(""));
        assert!(!looks_like_uuid("01890a5d-ac96-774b-9c0e-2b7f5a6d8e1"));
        assert!(!looks_like_uuid("01890a5d-ac96-774b-9c0e-2b7f5a6d8e100"));
        // Uppercase, which we never write.
        assert!(!looks_like_uuid("01890A5D-AC96-774B-9C0E-2B7F5A6D8E10"));
        // A dash in the wrong place.
        assert!(!looks_like_uuid("01890a5da-c96-774b-9c0e-2b7f5a6d8e10"));
        // Not version 7.  Version 4, all random, was the old kind.
        assert!(!looks_like_uuid("01890a5d-ac96-474b-9c0e-2b7f5a6d8e10"));
        // The wrong layout bits.
        assert!(!looks_like_uuid("01890a5d-ac96-774b-1c0e-2b7f5a6d8e10"));
        // Not hex.
        assert!(!looks_like_uuid("01890a5d-ac96-774b-9c0e-2b7f5a6d8g10"));
    }
}
