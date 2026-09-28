//! File:       Opus/Conductor/dev/conductor-tools/src/constellations.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Constellations loads `Content/cfg/conductor_globals.cfg` once at startup
//! and holds the settings where the rest of the server can read them.  The
//! file is plain `key = value` lines with `#` comments, so it can be edited
//! by hand with the server stopped.
//!
//! Nothing in here can stop the server.  A missing file gets written with
//! the defaults -- a fresh checkout has no `Content/` folder at all, and
//! that must never be a crash.  A line we can't make sense of is a Warn in
//! the log, and that setting keeps its default.  A file we can't read at
//! all is an Error, and we run on the defaults.
//!
//! Adding a setting means touching four places, all in this file: the
//! Settings struct, default_settings(), apply_setting() and file_text().

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::scribe::{self, Channel};

/// Where the config lives, under the Content folder.
const CONFIG_FILE: &str = "cfg/conductor_globals.cfg";

/// The environment variable that points at the Content folder when the
/// walk-up search would land in the wrong place.
const CONTENT_ENV: &str = "OPUS_CONTENT";

/// Every setting the config file can hold.  The field names are the keys in
/// the file.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// The folder Scribe writes its daily log files into, as written in the
    /// file.  A relative path is taken from the Content folder, so the
    /// default of `logs` means `Content/logs`.  `log_dir()` does that.
    pub scribe_log_dir: PathBuf,
    /// The port the web admin listens on.  It only ever listens on
    /// 127.0.0.1, so the port is the only part that can be changed.
    pub wgui_port: u16,
}

/// The built-in values.  These go into a freshly written config file, and a
/// setting falls back to them when the file has a bad value for it or none
/// at all.
fn default_settings() -> Settings {
    Settings {
        scribe_log_dir: PathBuf::from("logs"),
        wgui_port: 9996,
    }
}

// Rust note: `OnceLock` is a global that can be written exactly once and
// read from anywhere after that.  It is how Rust does a "set at startup,
// read forever" global without `unsafe`.
static CONTENT_DIR: OnceLock<PathBuf> = OnceLock::new();
static SETTINGS: OnceLock<Settings> = OnceLock::new();

/// Reads the config file, or writes one with the defaults if there isn't
/// one, and keeps the settings for `settings()`.  main calls this once,
/// right after Scribe has started, so the complaints have somewhere to go.
pub fn load() {
    let path = config_path();
    let mut settings = default_settings();

    match fs::read_to_string(&path) {
        Ok(text) => {
            for problem in parse_text(&text, &mut settings) {
                scribe::warn(Channel::System, &format!("{}, {problem}", path.display()));
            }
            scribe::info(Channel::System, &format!("Constellations loaded {}", path.display()));
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => write_default_file(&path),
        Err(e) => {
            // The file is there and we can't read it (permissions, most
            // likely, or it isn't text).  We don't write over it either.
            // Somebody's settings are in there.
            scribe::error_with(Channel::System, &e, &format!("Constellations can't read {}.  \
                Running on the built-in defaults.", path.display()));
        }
    }

    if SETTINGS.set(settings).is_err() {
        scribe::warn(Channel::System, "Constellations was asked to load twice.  The first load stands.");
    }
}

/// The loaded settings.  Panics if `load()` hasn't run, because that is a
/// bug in startup order, not something to limp past.
pub fn settings() -> &'static Settings {
    SETTINGS.get().expect("constellations::load must run before the settings are read")
}

/// The folder Scribe's logs go in, as a full path.  Before `load()` it is
/// the default one, which is what lets main start Scribe first.
pub fn log_dir() -> PathBuf {
    let from_file = match SETTINGS.get() {
        Some(settings) => settings.scribe_log_dir.clone(),
        None => default_settings().scribe_log_dir,
    };
    // Rust note: `join` with a path that starts with `/` hands back that
    // path as it is, so an absolute folder in the config file just works.
    content_dir().join(from_file)
}

/// The Content folder that was found (or will be made) at startup.
pub fn content_dir() -> &'static Path {
    CONTENT_DIR.get_or_init(find_content_dir)
}

/// The full path of the config file.
pub fn config_path() -> PathBuf {
    content_dir().join(CONFIG_FILE)
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
// Reading the file
// ---------------------------------------------------------------------------

/// Goes through the file a line at a time and puts every good value into
/// `settings`.  Anything it didn't like comes back as a complaint, one per
/// line, and the setting on that line is left alone.
///
/// It doesn't log and doesn't touch the disk, which is what lets the tests
/// run it.
///
/// The rules: blank lines and lines starting with `#` are skipped.  The rest
/// are split at the first `=`, and the spaces around both halves are thrown
/// away.  Keys can be in any case.  If a key shows up twice the later one
/// wins, and it gets a complaint of its own so a forgotten line higher up
/// doesn't go unnoticed.
fn parse_text(text: &str, settings: &mut Settings) -> Vec<String> {
    let mut problems = Vec::new();
    // The line each key was last set on.
    let mut set_on: HashMap<String, usize> = HashMap::new();

    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            problems.push(format!("line {number}: \"{line}\" isn't a \"key = value\" line.  Ignored."));
            continue;
        };
        let key = key.trim().to_ascii_lowercase();

        match apply_setting(settings, &key, value.trim()) {
            Ok(()) => {
                if let Some(earlier) = set_on.get(&key) {
                    problems.push(format!("line {number}: {key} was already set on line {earlier}.  \
                        This one wins."));
                }
                set_on.insert(key, number);
            }
            Err(problem) => problems.push(format!("line {number}: {problem}  Ignored.")),
        }
    }

    problems
}

/// Puts one value into the settings, if the key is one we know and the value
/// makes sense for it.  If not, the settings are left alone and the reason
/// comes back as the error.
// Rust note: the `?` on the end means "if that failed, return its error
// right here", so a bad value never gets as far as the assignment.
fn apply_setting(settings: &mut Settings, key: &str, value: &str) -> Result<(), String> {
    match key {
        "scribe_log_dir" => settings.scribe_log_dir = parse_folder(key, value)?,
        "wgui_port" => settings.wgui_port = parse_port(key, value)?,
        _ => return Err(format!("There is no setting called {key}.")),
    }
    Ok(())
}

/// A folder.  Anything goes except nothing at all.
fn parse_folder(key: &str, value: &str) -> Result<PathBuf, String> {
    if value.is_empty() {
        return Err(format!("{key} is empty, and it needs a folder, like logs or /var/log/opus."));
    }
    Ok(PathBuf::from(value))
}

/// A port.  1 to 65535, since 0 would mean "any port the OS likes" and
/// nobody would know where to point the browser.
fn parse_port(key: &str, value: &str) -> Result<u16, String> {
    match value.parse::<u16>() {
        Ok(port) if port > 0 => Ok(port),
        _ => Err(format!("{key} is \"{value}\", and it needs a port from 1 to 65535, like 9996.")),
    }
}

// ---------------------------------------------------------------------------
// Writing the file
// ---------------------------------------------------------------------------

/// The whole text of a config file holding these settings, comments and
/// all.  Whatever this writes, parse_text() has to read back without a
/// complaint, and there is a test that holds us to it.
fn file_text(settings: &Settings) -> String {
    format!("\
# Conductor's settings.  One \"key = value\" a line, and \"#\" starts a comment.
# Paths are relative to the Content folder unless they start with \"/\".
# Conductor wrote this file with its defaults because there wasn't one.
# Stop the server before editing it; it is only read at startup.  A line
# Conductor can't make sense of is logged and skipped, and that setting
# keeps its default.

# The folder Scribe writes its logs into.  One file a day, named by the
# UTC date, rolling over at midnight UTC.
scribe_log_dir = {}

# The port the web admin listens on.  It only listens on 127.0.0.1, so it
# can't be reached from another machine.  Open http://127.0.0.1:<port>/Opus
# in a browser on this one.
wgui_port = {}
", settings.scribe_log_dir.display(), settings.wgui_port)
}

/// There was no config file, so we write one with the defaults.  If that
/// fails too we say so and move on.  The server runs on the same defaults
/// either way, it just doesn't have a file to show for it.
fn write_default_file(path: &Path) {
    // If the folder can't be made, the write below fails and says why, so
    // this one's own error can go.
    if let Some(folder) = path.parent() {
        let _ = fs::create_dir_all(folder);
    }

    match fs::write(path, file_text(&default_settings())) {
        Ok(()) => scribe::info(Channel::System, &format!("No config file, so Constellations wrote one with the \
            defaults: {}", path.display())),
        Err(e) => scribe::error_with(Channel::System, &e, &format!("No config file, and Constellations \
            can't write one at {}.  Running on the built-in defaults.", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_we_write_we_can_read() {
        // Different from the default, or a line that silently failed to
        // parse would still "match".
        let written = Settings {
            scribe_log_dir: PathBuf::from("/tmp/somewhere else/logs"),
            wgui_port: 12345,
        };

        let mut read_back = default_settings();
        let problems = parse_text(&file_text(&written), &mut read_back);

        assert!(problems.is_empty(), "complaints: {problems:?}");
        assert_eq!(read_back, written);
    }

    #[test]
    fn the_default_file_reads_clean() {
        let mut settings = default_settings();
        assert!(parse_text(&file_text(&default_settings()), &mut settings).is_empty());
        assert_eq!(settings, default_settings());
    }

    #[test]
    fn a_bad_value_keeps_the_default() {
        let mut settings = default_settings();
        let problems = parse_text("scribe_log_dir =\n", &mut settings);

        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("line 1:"));
        assert_eq!(settings, default_settings());
    }

    #[test]
    fn a_port_has_to_be_a_real_one() {
        for bad in ["0", "65536", "-1", "port", ""] {
            let mut settings = default_settings();
            let problems = parse_text(&format!("wgui_port = {bad}\n"), &mut settings);
            assert_eq!(problems.len(), 1, "{bad} should be a complaint");
            assert_eq!(settings, default_settings());
        }

        let mut settings = default_settings();
        assert!(parse_text("wgui_port = 8080\n", &mut settings).is_empty());
        assert_eq!(settings.wgui_port, 8080);
    }

    #[test]
    fn an_unknown_key_is_a_complaint() {
        let mut settings = default_settings();
        let problems = parse_text("# a comment\n\ntypo = 1\n", &mut settings);

        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("line 3:"));
        assert!(problems[0].contains("typo"));
        assert_eq!(settings, default_settings());
    }

    #[test]
    fn the_later_one_wins() {
        let mut settings = default_settings();
        let problems = parse_text("scribe_log_dir = first\nscribe_log_dir = second\n", &mut settings);

        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("line 2:"));
        assert_eq!(settings.scribe_log_dir, PathBuf::from("second"));
    }

    #[test]
    fn a_bad_line_does_not_spoil_the_good_ones() {
        // Also: keys in any case, spaces around both halves, a Windows line
        // ending, and an `=` inside a value.
        let mut settings = default_settings();
        let problems = parse_text("this line is broken\n  SCRIBE_LOG_DIR =  /tmp/a=b  \r\n", &mut settings);

        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("line 1:"));
        assert_eq!(settings.scribe_log_dir, PathBuf::from("/tmp/a=b"));
    }
}
