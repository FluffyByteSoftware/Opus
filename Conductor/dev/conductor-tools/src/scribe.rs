//! File:       Opus/Conductor/dev/conductor-tools/src/scribe.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Scribe is the log.  Every line is appended to a file named for the UTC
//! date (`2026_09_28.scribe.log`), and at midnight UTC Scribe closes that
//! file and opens the next one.  A line looks like this (it is one line in
//! the file, wrapped here to fit):
//!
//! ```text
//! [ 12:08:45 PM - 09-28-26 Z ] - [ System / Info ]
//!     - [ Conductor is starting. ]
//!     [ Caller: conductor-launcher/src/main.rs, Line: 27 ]
//! ```
//!
//! The file is the only place a line goes.  Nothing prints to the terminal,
//! because the terminal belongs to the launcher's menu, and the menu's L
//! is how the admin reads the log.  The one exception is a line with no
//! file to go to -- before `start()`, or when the file can't be opened or
//! written.  Then it prints to the terminal, since a line nobody can see
//! anywhere is worse than one that lands in the middle of the menu.
//!
//! Scribe never panics and never returns an error from a log call.  A log
//! that takes the server down is worse than no log.
//!
//! Scribe starts before Constellations, so it has somewhere to put the
//! config file's complaints.  It starts on the default folder, and
//! `move_to()` switches it to the configured one once the config is
//! loaded.  The one thing I don't like about that: if the config points
//! somewhere else, the handful of lines logged before the move stay behind
//! in the default folder.  Not worth fixing while both are the same one.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::panic::Location;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::clock::Utc;

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

/// Scribe's working state.  There is exactly one of these, in SCRIBE below.
struct Scribe {
    /// The folder the log files go in.
    dir: PathBuf,
    /// Today's log file, whether or not we managed to open it.
    path: PathBuf,
    /// The open file.  `None` when it couldn't be opened or written, and
    /// then lines go to the terminal until the next day or `move_to()`
    /// gives it another try.
    file: Option<File>,
    /// The UTC date the file belongs to.  When today's date is different,
    /// it is time for a new file.
    day: (i64, u32, u32),
}

// Rust note: a `Mutex` is a lock around the value inside it.  The compiler
// won't let us touch the Scribe without locking first, which is what stops
// two threads writing over the top of each other's lines.  `None` means
// start() hasn't run yet.
static SCRIBE: Mutex<Option<Scribe>> = Mutex::new(None);

/// Opens today's log file in `dir`, making the folder if it has to.  main
/// calls this first thing, before Constellations, on the default folder.
/// Until it runs, log lines print to the terminal.
pub fn start(dir: &Path) {
    open_in(dir);
}

/// Switches to today's file in `dir`.  main calls this once Constellations
/// has loaded, with the folder from the config file.  If that is the folder
/// we are already writing to, nothing happens.
pub fn move_to(dir: &Path) {
    {
        let guard = SCRIBE.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(scribe) = guard.as_ref() {
            if scribe.dir == dir && scribe.file.is_some() {
                return;
            }
        }
    }
    open_in(dir);
}

/// The file Scribe is writing to right now, for the launcher's L.  `None`
/// before `start()`, or when the file couldn't be opened.
pub fn current_file() -> Option<PathBuf> {
    let guard = SCRIBE.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match guard.as_ref() {
        Some(scribe) if scribe.file.is_some() => Some(scribe.path.clone()),
        _ => None,
    }
}

/// Opens (or makes) today's file in `dir` and makes it the one Scribe
/// writes to.  If it won't open, we say so once on stderr and carry on
/// without a file.
fn open_in(dir: &Path) {
    let now = Utc::now();
    let mut scribe = Scribe {
        dir: dir.to_path_buf(),
        path: file_path(dir, &now),
        file: None,
        day: now.date(),
    };
    reopen(&mut scribe);

    let mut guard = SCRIBE.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // Rust note: there is no close call.  Replacing the old Scribe throws
    // its File away, and Rust closes a file when it is thrown away.
    *guard = Some(scribe);
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

/// Builds the line and appends it to the file, rolling to a new file first
/// if the UTC date has changed since the last write.  A line with no file
/// to go to prints to the terminal instead.
///
/// The lock is held for the whole call, so two threads can't interleave
/// their lines.
fn write(priority: Priority, channel: Channel, message: &str, caller: &Location<'_>) {
    let now = Utc::now();
    let line = format_line(&now, priority, channel, message, caller);

    let mut guard = SCRIBE.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // Rust note: `let ... else` runs the else block when the pattern doesn't
    // match -- here, when start() hasn't run yet -- and that block has to
    // leave the function.
    let Some(scribe) = guard.as_mut() else {
        println!("{line}");
        return;
    };

    // Midnight UTC has come and gone.  This is also the retry for a file we
    // lost, since a new day means a fresh try.
    if scribe.day != now.date() {
        scribe.day = now.date();
        scribe.path = file_path(&scribe.dir, &now);
        reopen(scribe);
    }

    // The write happens first and its answer is kept, so the file is no
    // longer in use when we let go of it below.
    let result = match scribe.file.as_mut() {
        Some(file) => Some(writeln!(file, "{line}")),
        None => None,
    };

    match result {
        Some(Ok(())) => {}
        Some(Err(e)) => {
            eprintln!("Scribe can't write to {}: {e}.  Log lines print here until midnight UTC.",
                      scribe.path.display());
            scribe.file = None;
            println!("{line}");
        }
        None => println!("{line}"),
    }
}

/// Opens `scribe.path` for appending, making its folder first.  On failure
/// the file stays `None` and stderr gets one line saying why.
fn reopen(scribe: &mut Scribe) {
    scribe.file = None;
    if let Err(e) = fs::create_dir_all(&scribe.dir) {
        eprintln!("Scribe can't make the log folder {}: {e}.  Log lines print here instead.",
                  scribe.dir.display());
        return;
    }
    match OpenOptions::new().append(true).create(true).open(&scribe.path) {
        Ok(file) => scribe.file = Some(file),
        Err(e) => {
            eprintln!("Scribe can't open {}: {e}.  Log lines print here instead.", scribe.path.display());
        }
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

/// The log file for a given day: `2026_09_28.scribe.log` in `dir`.  An
/// earlier run's file for the same day gets appended to.
fn file_path(dir: &Path, day: &Utc) -> PathBuf {
    dir.join(format!("{}.scribe.log", day.file_stamp()))
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

    #[test]
    fn log_files_are_named_by_the_utc_date() {
        let now = Utc::from_unix(1_790_596_800); // 2026-09-28 12:00:00 UTC
        assert_eq!(file_path(Path::new("/tmp/logs"), &now), PathBuf::from("/tmp/logs/2026_09_28.scribe.log"));
    }
}
