//! File:       Opus/Conductor/dev/src/tools/scribe.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Scribe is the log.  Every line goes to stdout and to a file named for
//! the UTC date (`2026_09_28.scribe.log`), and at midnight UTC Scribe
//! closes that file and opens the next one.  A line looks like:
//!
//! ```text
//! [ 12:08:45 PM - 09-28-26 Z ] - [ System / Info ] - [ Conductor is starting. ] [ Caller: src/main.rs, Line: 27 ]
//! ```
//!
//! Scribe never panics and never returns an error from a log call.  If the
//! file can't be written the line still goes to stdout, with a note on
//! stderr about what went wrong, because a log that takes the server down
//! is worse than no log.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::panic::Location;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::clock::Utc;

/// Which part of the server a line came from.  Add to this as the server
/// grows; the names are what shows up in the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    System,
    Network,
    Security,
    Database,
    Game,
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Rust note: `{:?}` is the debug form of an enum, which for a plain
        // variant is just its name.  Good enough here and it can't drift
        // from the variant list.
        write!(f, "{:?}", self)
    }
}

/// How much a line matters.  Everything is written for now; filtering by
/// priority is on the TODO list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// The open log file and the day it belongs to.
struct Scribe {
    dir: PathBuf,
    file: File,
    day: (i64, u32, u32),
}

static SCRIBE: Mutex<Option<Scribe>> = Mutex::new(None);

/// Opens today's log file in `dir`, creating the folder if it has to.  Call
/// once at startup, after Constellations has loaded.  Until this runs, log
/// calls still work but only reach stdout.
pub fn start(dir: &Path) -> std::io::Result<()> {
    let now = Utc::now();
    let file = open_for(dir, &now)?;
    let mut guard = SCRIBE.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = Some(Scribe {
        dir: dir.to_path_buf(),
        file,
        day: now.date(),
    });
    Ok(())
}

// Rust note: `#[track_caller]` makes `Location::caller()` inside the
// function report the file and line of whoever *called* it, not the line
// here.  That is how the log gets the caller without every call site
// having to pass `file!()` and `line!()`.

/// Chatter that only matters when something is being chased down.
#[track_caller]
pub fn debug(channel: Channel, message: &str) {
    write(Priority::Debug, channel, message, Location::caller());
}

/// Normal running: what started, what connected, what got saved.
#[track_caller]
pub fn info(channel: Channel, message: &str) {
    write(Priority::Info, channel, message, Location::caller());
}

/// Something is off but the server can carry on.
#[track_caller]
pub fn warn(channel: Channel, message: &str) {
    write(Priority::Warn, channel, message, Location::caller());
}

/// Something failed.  If an admin has to fix it by hand, open the message
/// in capitals so it can't be missed in a scroll of lines.
#[track_caller]
pub fn error(channel: Channel, message: &str) {
    write(Priority::Error, channel, message, Location::caller());
}

// The `_with` forms take an error value as well and print it in front of
// the message, the way `ex.Message` would come first in C#:
// `[ No such file or directory (os error 2) - Could not open the save. ]`.
//
// Rust note: Rust has no exceptions.  An error is an ordinary value that
// a function hands back, and nearly all of them can be printed, which is
// all Scribe needs from one.  So these take anything printable.

/// `debug`, with an error value in front of the message.
#[track_caller]
pub fn debug_with(channel: Channel, err: &dyn fmt::Display, message: &str) {
    write(Priority::Debug, channel, &format!("{err} - {message}"), Location::caller());
}

/// `info`, with an error value in front of the message.
#[track_caller]
pub fn info_with(channel: Channel, err: &dyn fmt::Display, message: &str) {
    write(Priority::Info, channel, &format!("{err} - {message}"), Location::caller());
}

/// `warn`, with an error value in front of the message.
#[track_caller]
pub fn warn_with(channel: Channel, err: &dyn fmt::Display, message: &str) {
    write(Priority::Warn, channel, &format!("{err} - {message}"), Location::caller());
}

/// `error`, with an error value in front of the message.
#[track_caller]
pub fn error_with(channel: Channel, err: &dyn fmt::Display, message: &str) {
    write(Priority::Error, channel, &format!("{err} - {message}"), Location::caller());
}

/// Builds the line, prints it, and appends it to the file, rolling to a new
/// file first if the UTC date has changed since the last write.
fn write(priority: Priority, channel: Channel, message: &str, caller: &Location<'_>) {
    let now = Utc::now();
    let line = format_line(&now, priority, channel, message, caller);
    println!("{line}");

    let mut guard = SCRIBE.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(scribe) = guard.as_mut() else {
        return;
    };

    if scribe.day != now.date() {
        match open_for(&scribe.dir, &now) {
            Ok(file) => {
                scribe.file = file;
                scribe.day = now.date();
            }
            Err(e) => {
                eprintln!("Scribe could not open the log file for {}: {e}", now.file_stamp());
                return;
            }
        }
    }

    if let Err(e) = writeln!(scribe.file, "{line}") {
        eprintln!("Scribe could not write to the log file: {e}");
    }
}

fn format_line(now: &Utc, priority: Priority, channel: Channel, message: &str, caller: &Location<'_>) -> String {
    format!(
        "[ {} ] - [ {} / {} ] - [ {} ] [ Caller: {}, Line: {} ]",
        now.line_stamp(),
        channel,
        priority,
        message,
        caller.file(),
        caller.line()
    )
}

/// Opens (or creates) the log file for the given day, appending if it is
/// already there from an earlier run.
fn open_for(dir: &Path, day: &Utc) -> std::io::Result<File> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.scribe.log", day.file_stamp()));
    OpenOptions::new().append(true).create(true).open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_has_every_bracket_in_order() {
        let now = Utc::from_unix(1_790_596_800); // 2026-09-28 12:00:00 UTC
        let line = format_line(&now, Priority::Warn, Channel::Network, "Packet was late.", Location::caller());
        let expected_start = "[ 12:00:00 PM - 09-28-26 Z ] - [ Network / Warn ] - [ Packet was late. ] [ Caller: ";
        assert!(line.starts_with(expected_start));
        assert!(line.ends_with(" ]"));
        assert!(line.contains("scribe.rs, Line: "));
    }
}
