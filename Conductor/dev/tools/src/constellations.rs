//! File:       Opus/Conductor/dev/tools/src/constellations.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Constellations, the settings.  Every config file Conductor has lives in
//! `Content/cfg/`, is written down once in `constellations/files.rs` (its
//! settings, their kinds, defaults and comments), and is read and written
//! by the one reader in `constellations/text.rs`.  This file is the store:
//! it loads a file when its piece starts, holds the values where the rest
//! of the server can read them, and handles a change from the web admin.
//!
//! A file is **soft** or **hard** as a whole.  A soft file
//! (`postgres.cfg`) is read every time the server starts, so STOP SERVER
//! and START SERVER on the Control Panel is enough to pick up a change.
//! A hard file (`conductor_globals.cfg`, `wgui.cfg`) is read once at boot,
//! so Conductor has to be shut down and run again.
//!
//! A change from the web admin never touches the live file.  It's written
//! to `name.cfg.wait4server` beside it, and DiskMan is told to rename that
//! over the live file at the right moment: when the server stops for a
//! soft file, when Conductor shuts down for a hard one.  The next start
//! or boot then reads the new file the ordinary way.  If Conductor didn't
//! get to the swap (it crashed, or the change was saved while the server
//! was already stopped), `load()` finds the waiting file and swaps it in
//! before reading.  Jacob's design, 2026-09-29.
//!
//! Nothing in here can stop the server.  A missing file gets written with
//! the defaults -- a fresh checkout has no `Content/` folder at all, and
//! that must never be a crash.  A line we can't make sense of is a Warn
//! in the log, and that setting keeps its default.  A file we can't read
//! at all is an Error, and we run on the defaults.
//!
//! Adding a setting is one entry in `files.rs` and a line wherever it's
//! read.  Adding a file is one entry in `files.rs` and a `load()` where
//! its piece starts.

mod files;
mod text;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::diskman::{self, DiskError, SwapAt};
use crate::scribe::{self, Channel};
use crate::services::{self, State};

pub use files::{ConfigFile, FILES, GLOBALS, Kind, NETWORKING, POSTGRES, Reboot, Setting, WGUI};
pub use text::Values;

/// Where every config file lives, under the Content folder.
const CFG_DIR: &str = "cfg";

/// What goes on the end of a file's name for the copy waiting to replace
/// it.  Jacob's name.
const WAITING_SUFFIX: &str = ".wait4server";

/// The environment variable that points at the Content folder when the
/// walk-up search would land in the wrong place.
const CONTENT_ENV: &str = "OPUS_CONTENT";

/// `conductor_globals.cfg` as the launcher reads it: the typed view of the
/// file's values.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// The folder Scribe writes its daily log files into, as written in
    /// the file.  A relative path is taken from the Content folder, so the
    /// default of `logs` means `Content/logs`.  `log_dir()` does that.
    pub scribe_log_dir: PathBuf,
    /// The port the web admin listens on.  It only ever listens on
    /// 127.0.0.1, so the port is the only part that can be changed.
    pub wgui_port: u16,
}

// Rust note: `OnceLock` is a global that can be written exactly once and
// read from anywhere after that.  The Content folder is found once and
// never moves, so it fits.  The values don't: they change on every load,
// so they sit behind a plain lock.
static CONTENT_DIR: OnceLock<PathBuf> = OnceLock::new();

/// The values of every file that has been loaded, by the file's name.  A
/// file that isn't in here hasn't been loaded, and reads as its defaults.
static LOADED: Mutex<BTreeMap<&'static str, Values>> = Mutex::new(BTreeMap::new());

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Reads `file`, or writes one with the defaults if there isn't one, and
/// keeps the values for `value()` and friends.  The launcher calls this
/// for `GLOBALS` at boot, right after Scribe has started so the
/// complaints have somewhere to go, and each server piece calls it for
/// its own file when it starts.  Calling it again reads the file again.
/// The file comes through DiskMan, and this waits on it, which is fine at
/// a start.
pub fn load(file: &'static ConfigFile) {
    let path = path_of(file);
    apply_leftover(file, &path);

    let mut values = text::defaults(file);
    match diskman::read(&path).wait().and_then(|bytes| diskman::as_text(&bytes)) {
        Ok(contents) => {
            let parsed = text::parse(file, &contents);
            for problem in &parsed.problems {
                scribe::warn(file.channel, &format!("{}, {problem}", path.display()));
            }
            values.extend(parsed.values);
            add_missing(file, &path, &parsed.seen);
            scribe::debug(file.channel, &format!("Constellations loaded {}", path.display()));
            // A line it couldn't use is a Warn in the log, not trouble:
            // that one setting runs on its default and the rest are fine.
            let note = match parsed.problems.len() {
                0 => format!("Loaded {}", file.name),
                count => format!("Loaded {}, with {count} line(s) it couldn't use (see the log)", file.name),
            };
            services::set(services::CONSTELLATIONS, State::Running, &note);
        }
        Err(e) if e.is_not_found() => write_default_file(file, &path),
        Err(e) => {
            // The file is there and we can't read it (permissions, most
            // likely, or it isn't text).  We don't write over it either.
            // Somebody's settings are in there.
            scribe::error_with(file.channel, &e, &format!("Constellations can't read {}.  \
                Running on the built-in defaults.", path.display()));
            services::set(services::CONSTELLATIONS, State::Trouble, &format!("Can't read {}: {e}.  \
                Running on the built-in defaults.", file.name));
        }
    }

    let mut loaded = LOADED.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    loaded.insert(file.name, values);
}

/// A `.wait4server` file that is still there when its file is about to be
/// read is one Conductor didn't get to on the way down, or one saved while
/// the server was already stopped.  Either way the admin wanted it, so it
/// goes in now, before the read.
fn apply_leftover(file: &ConfigFile, path: &Path) {
    let waiting = waiting_path(file);
    match diskman::read(&waiting).wait() {
        Ok(_) => match diskman::swap(path, &waiting, SwapAt::Now).wait() {
            Ok(()) => scribe::info(file.channel, &format!("Constellations found {} and swapped it in over {}.",
                                                          waiting.display(),
                                                          path.display())),
            Err(e) => scribe::warn_with(file.channel, &e, &format!("Constellations found {} but couldn't swap \
                it in over {}.  Running on the old file.", waiting.display(), path.display())),
        },
        Err(e) if e.is_not_found() => {}
        Err(e) => scribe::warn_with(file.channel, &e, &format!("Constellations couldn't look for {}.",
                                                              waiting.display())),
    }
}

/// Adds any setting the file doesn't have to the end of it, with its
/// default.  Nothing already in the file gets touched.
fn add_missing(file: &ConfigFile, path: &Path, seen: &std::collections::HashSet<String>) {
    let Some(added) = text::missing_text(file, seen) else {
        return;
    };
    match diskman::append(path, added.as_bytes()).wait() {
        Ok(()) => scribe::debug(file.channel, &format!("Constellations added the settings missing from {}, \
            with their defaults.", path.display())),
        Err(e) => scribe::warn_with(file.channel, &e, &format!("Constellations couldn't add the missing \
            settings to {}.  They run on their defaults.", path.display())),
    }
}

/// There was no file, so we write one with the defaults.  If that fails
/// too we say so and move on.  The server runs on the same defaults either
/// way, it just doesn't have a file to show for it.  DiskMan makes the
/// folder if it has to.
fn write_default_file(file: &ConfigFile, path: &Path) {
    let contents = text::file_text(file, &text::defaults(file));
    match diskman::write(path, contents.into_bytes()).wait() {
        Ok(()) => {
            scribe::info(file.channel, &format!("No {}, so Constellations wrote one with the defaults: {}",
                                                file.name,
                                                path.display()));
            services::set(services::CONSTELLATIONS, State::Running, &format!("Wrote {} with the defaults",
                                                                             file.name));
        }
        Err(e) => {
            scribe::error_with(file.channel, &e, &format!("No {}, and Constellations can't write one at {}.  \
                Running on the built-in defaults.", file.name, path.display()));
            services::set(services::CONSTELLATIONS, State::Trouble, &format!("Can't write {}: {e}.  \
                Running on the built-in defaults.", file.name));
        }
    }
}

// ---------------------------------------------------------------------------
// Reading the values
// ---------------------------------------------------------------------------

/// Every value in `file`: the loaded ones, or the defaults if it hasn't
/// been loaded.  Every key in the file's table is in here.
pub fn values(file: &ConfigFile) -> Values {
    let loaded = LOADED.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    loaded.get(file.name).cloned().unwrap_or_else(|| text::defaults(file))
}

/// The values in `file` as it sits on disk right now, over the defaults,
/// or `None` when it can't be read (there isn't one yet, say).  For the
/// Settings tab, which shows a file that hasn't been loaded this run
/// (`postgres.cfg` before the first START SERVER) as the file says, since
/// that is what the next start reads.  Nothing is kept and nothing is
/// complained about here; `load()` is where the file's lines get their
/// Warns.
pub fn file_values(file: &ConfigFile) -> Option<Values> {
    let contents = diskman::read(&path_of(file)).wait().and_then(|bytes| diskman::as_text(&bytes)).ok()?;
    let mut values = text::defaults(file);
    values.extend(text::parse(file, &contents).values);
    Some(values)
}

/// True once `load()` has run for `file`, whether or not the file was there.
pub fn is_loaded(file: &ConfigFile) -> bool {
    let loaded = LOADED.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    loaded.contains_key(file.name)
}

/// One value, as written in the file.  A key that isn't in the file's
/// table is a bug in the caller, and comes back empty rather than
/// stopping anything.
pub fn value(file: &ConfigFile, key: &str) -> String {
    values(file).remove(key).unwrap_or_default()
}

/// A `Number` setting.  The value was checked when the file was read, so
/// this can't fail for a key in the table; a key that isn't falls back to
/// 0.
pub fn number(file: &ConfigFile, key: &str) -> u64 {
    value(file, key).parse().unwrap_or(0)
}

/// A `Port` setting, the same way.
pub fn port(file: &ConfigFile, key: &str) -> u16 {
    value(file, key).parse().unwrap_or(0)
}

/// A `Folder` setting as a full path: a relative one is taken from the
/// Content folder.
pub fn folder(file: &ConfigFile, key: &str) -> PathBuf {
    // Rust note: `join` with a path that starts with `/` hands back that
    // path as it is, so an absolute folder in the config file just works.
    content_dir().join(value(file, key))
}

/// `conductor_globals.cfg` as a struct.  Before `load(&GLOBALS)` it's the
/// defaults.
pub fn settings() -> Settings {
    Settings {
        scribe_log_dir: PathBuf::from(value(&GLOBALS, "scribe_log_dir")),
        wgui_port: port(&GLOBALS, "wgui_port"),
    }
}

/// The folder Scribe's logs go in, as a full path.  Before `load()` it is
/// the default one, which is what lets main start Scribe first.
pub fn log_dir() -> PathBuf {
    folder(&GLOBALS, "scribe_log_dir")
}

// ---------------------------------------------------------------------------
// Where things are
// ---------------------------------------------------------------------------

/// The Content folder that was found (or will be made) at startup.
pub fn content_dir() -> &'static Path {
    CONTENT_DIR.get_or_init(find_content_dir)
}

/// The file called `name` (`postgres.cfg`, say), for a caller that has
/// the name as text: the web admin, whose routes name the file in the
/// query.  `None` for a name that isn't in the table.
pub fn file_named(name: &str) -> Option<&'static ConfigFile> {
    FILES.iter().copied().find(|file| file.name == name)
}

/// The full path of `file`: `Content/cfg/<name>`.
pub fn path_of(file: &ConfigFile) -> PathBuf {
    content_dir().join(CFG_DIR).join(file.name)
}

/// The full path of the copy waiting to replace `file`:
/// `Content/cfg/<name>.wait4server`.
pub fn waiting_path(file: &ConfigFile) -> PathBuf {
    content_dir().join(CFG_DIR).join(format!("{}{WAITING_SUFFIX}", file.name))
}

/// The full path of `conductor_globals.cfg`, for the boot lines.
pub fn config_path() -> PathBuf {
    path_of(&GLOBALS)
}

/// Works out where `Content/` is.  `OPUS_CONTENT` wins if it is set.
/// Otherwise we walk up from the working directory looking for a folder
/// that already has a `Content/` in it -- from `Conductor/dev` that is two
/// levels up.  If nothing turns up we use `./Content`, and it gets made the
/// first time something writes there, so a bare binary in an empty folder
/// still runs.
fn find_content_dir() -> PathBuf {
    if let Some(from_env) = std::env::var_os(CONTENT_ENV) {
        return PathBuf::from(from_env);
    }
    if let Ok(cwd) = std::env::current_dir() {
        for dir in cwd.ancestors() {
            let candidate = dir.join("Content");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    PathBuf::from("Content")
}

// ---------------------------------------------------------------------------
// Changing a file
// ---------------------------------------------------------------------------

/// A change from the web admin.  `contents` is `key = value` lines, the
/// same as the file itself, holding the settings to change; any setting it
/// leaves out keeps the value it has now.  Every line is checked first,
/// and if any is wrong nothing is written and the complaints come back,
/// one per line, for the page to show.
///
/// A good one is written whole, comments and all, to the file's
/// `.wait4server` beside it, and DiskMan is asked to rename it over the
/// live file when the file's reboot comes: the server stopping for a soft
/// file, Conductor shutting down for a hard one.  The live file and the
/// running values don't change until then.
pub fn save_waiting(file: &'static ConfigFile, contents: &str) -> Result<(), Vec<String>> {
    let parsed = text::parse(file, contents);
    if !parsed.problems.is_empty() {
        return Err(parsed.problems);
    }
    let mut values = values(file);
    values.extend(parsed.values);

    let waiting = waiting_path(file);
    if let Err(e) = diskman::write(&waiting, text::file_text(file, &values).into_bytes()).wait() {
        scribe::error_with(file.channel, &e, &format!("Constellations couldn't write {}.  The change is lost.",
                                                     waiting.display()));
        return Err(vec![format!("Couldn't write {}: {e}", waiting.display())]);
    }

    let when = match file.reboot {
        Reboot::Soft => SwapAt::ServerStop,
        Reboot::Hard => SwapAt::Shutdown,
    };
    // The answer isn't waited on: the swap happens later, and DiskMan
    // says so in the log if it can't.
    diskman::swap(&path_of(file), &waiting, when);
    scribe::info(file.channel, &format!("Constellations saved {}.  It replaces {} at {}.",
                                        waiting.display(),
                                        file.name,
                                        file.reboot.describe()));
    Ok(())
}

/// The values waiting in `file`'s `.wait4server`, or `None` when there
/// isn't one.  For the page, so it can show what's saved next to what's
/// running.  A hand-written one with lines it can't use is read like any
/// file: those lines are Warns and the rest stand.
pub fn waiting(file: &ConfigFile) -> Option<Values> {
    let waiting = waiting_path(file);
    match diskman::read(&waiting).wait().and_then(|bytes| diskman::as_text(&bytes)) {
        Ok(contents) => {
            let parsed = text::parse(file, &contents);
            for problem in &parsed.problems {
                scribe::warn(file.channel, &format!("{}, {problem}", waiting.display()));
            }
            let mut values = values(file);
            values.extend(parsed.values);
            Some(values)
        }
        Err(e) if e.is_not_found() => None,
        Err(e) => {
            scribe::warn_with(file.channel, &e, &format!("Constellations can't read {}.", waiting.display()));
            None
        }
    }
}

/// Throws away the change waiting for `file`, if there is one.  The live
/// file stands as it is.
pub fn discard_waiting(file: &ConfigFile) -> Result<(), DiskError> {
    let waiting = waiting_path(file);
    diskman::forget_swap(&path_of(file));
    let result = diskman::remove(&waiting).wait();
    match &result {
        Ok(()) => scribe::info(file.channel, &format!("Constellations discarded {}.", waiting.display())),
        Err(e) => scribe::warn_with(file.channel, e, &format!("Constellations couldn't discard {}.",
                                                              waiting.display())),
    }
    result
}

/// The server pieces have stopped.  The launcher calls this from
/// `stop_server()`, once they're down, and it waits while DiskMan renames
/// every soft file's `.wait4server` over the live file, so the next
/// START SERVER reads the new ones.  The hard files' swaps wait for
/// DiskMan's own stop, at Conductor's shutdown.
pub fn server_stopped() {
    if let Err(e) = diskman::run_swaps(SwapAt::ServerStop).wait() {
        scribe::warn_with(Channel::System, &e, "Constellations couldn't swap in the config files waiting on \
            the server stopping.  Whatever is still waiting is picked up when the server starts, if the disk \
            allows it then.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unloaded_file_reads_as_its_defaults() {
        // Nothing in the tests loads a file, so every read is a default.
        assert_eq!(value(&POSTGRES, "port"), "5432");
        assert_eq!(number(&POSTGRES, "slow_job_ms"), 250);
        assert_eq!(port(&POSTGRES, "port"), 5432);
        assert_eq!(value(&POSTGRES, "password"), "");
        assert_eq!(value(&POSTGRES, "no such key"), "");
        assert!(!is_loaded(&POSTGRES));
        // DiskMan isn't running in the tests, so there is no file to read.
        assert!(file_values(&POSTGRES).is_none());
    }

    #[test]
    fn the_typed_view_matches_the_table() {
        let settings = settings();
        assert_eq!(settings.wgui_port, 9996);
        assert_eq!(settings.scribe_log_dir, PathBuf::from("logs"));
        assert!(log_dir().ends_with("logs"));
    }

    #[test]
    fn a_file_is_found_by_its_name() {
        assert!(std::ptr::eq(file_named("wgui.cfg").expect("it's in the table"), &WGUI));
        assert!(file_named("nope.cfg").is_none());
        assert!(file_named("").is_none());
    }

    #[test]
    fn the_waiting_file_sits_beside_the_live_one() {
        let live = path_of(&GLOBALS);
        let waiting = waiting_path(&GLOBALS);
        assert_eq!(live.parent(), waiting.parent());
        assert_eq!(waiting.file_name().and_then(|name| name.to_str()), Some("conductor_globals.cfg.wait4server"));
        assert_eq!(config_path(), live);
    }
}
