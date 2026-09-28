//! File:       Opus/Conductor/dev/conductor-tools/src/archivist/settings.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! `Content/cfg/postgres.cfg`: where Postgres is, how to log in, and the
//! knobs for how Archivist runs.  Same rules as `conductor_globals.cfg`.
//!
//! A missing file gets written with everything but the password.  A file
//! that is missing some settings (because Archivist learned new ones since
//! it was written) gets them added to the end with their defaults, so
//! every knob there is shows up in the file to be changed.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use postgres::Config;

use crate::constellations;
use crate::diskman;
use crate::scribe::{self, Channel};

/// Where Archivist's settings live, under the Content folder.
const CONFIG_FILE: &str = "cfg/postgres.cfg";

/// How long a connect gets before we call it a failure.  Postgres is on
/// the same machine, so if it hasn't answered in 5 seconds it isn't going
/// to.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Everything `postgres.cfg` can hold.  The field names are the keys in the
/// file.
// Rust note: no `#[derive(Debug)]` here.  A derived Debug prints every
// field, and the password would end up in the log the first time somebody
// printed the settings.  There is a hand-written one further down.
#[derive(Clone, PartialEq)]
pub(super) struct DbSettings {
    pub(super) address: String,
    pub(super) port: u16,
    pub(super) database: String,
    pub(super) username: String,
    pub(super) password: String,
    /// Postgres cancels any query that runs longer than this.  0 means no
    /// limit.
    pub(super) query_time_limit_seconds: u64,
    /// A job that takes at least this long is a Warn in the log and counts
    /// as slow in `status()`.
    pub(super) slow_job_ms: u64,
}

/// The built-in values.  Everything but the password, which only the admin
/// knows.
pub(super) fn default_settings() -> DbSettings {
    DbSettings {
        address: "localhost".to_string(),
        port: 5432,
        database: "opusdb".to_string(),
        username: "opus_game".to_string(),
        password: String::new(),
        query_time_limit_seconds: 10,
        slow_job_ms: 250,
    }
}

impl fmt::Debug for DbSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let password = if self.password.is_empty() { "(none)" } else { "(hidden)" };
        write!(f,
               "{}@{}:{}/{} password {password}",
               self.username,
               self.address,
               self.port,
               self.database)
    }
}

impl DbSettings {
    /// The settings in the shape the postgres crate wants.
    pub(super) fn to_config(&self) -> Config {
        let mut config = Config::new();
        config.host(&self.address)
              .port(self.port)
              .dbname(&self.database)
              .user(&self.username)
              .password(&self.password)
              .application_name("Conductor")
              .connect_timeout(CONNECT_TIMEOUT);

        // Handed to Postgres when the connection opens, so it holds for
        // every query on it.  Postgres counts in milliseconds.
        if self.query_time_limit_seconds > 0 {
            config.options(&format!("-c statement_timeout={}", self.query_time_limit_seconds * 1000));
        }
        config
    }

    /// Where to connect, for the log.  Never the password.
    pub(super) fn where_to(&self) -> String {
        format!("{} at {}:{} as {}", self.database, self.address, self.port, self.username)
    }
}

/// The full path of `postgres.cfg`.
pub fn config_path() -> PathBuf {
    constellations::content_dir().join(CONFIG_FILE)
}

/// Reads `postgres.cfg`, writes one if it's missing, and adds any setting
/// the file doesn't have yet.  Complaints go to the log, and anything we
/// couldn't read keeps its default.  The file comes through DiskMan, and
/// this waits on it, which is fine at startup.
pub(super) fn load(path: &Path) -> DbSettings {
    let mut settings = default_settings();

    match diskman::read(path).wait().and_then(|bytes| diskman::as_text(&bytes)) {
        Ok(text) => {
            let (problems, seen) = parse_text(&text, &mut settings);
            for problem in problems {
                scribe::warn(Channel::Database, &format!("{}, {problem}", path.display()));
            }
            add_missing(path, &seen);
        }
        Err(e) if e.is_not_found() => write_default_file(path),
        Err(e) => {
            scribe::error_with(Channel::Database, &e, &format!("Archivist can't read {}.", path.display()));
        }
    }

    if settings.password.is_empty() {
        scribe::error(Channel::Database, &format!("NO POSTGRES PASSWORD, FILL IT IN BY HAND: {}.  \
            THERE IS NO DATABASE UNTIL IT IS, AND CONDUCTOR NEEDS A RESTART AFTER.",
            path.display()));
    }

    settings
}

// ---------------------------------------------------------------------------
// Reading the file
// ---------------------------------------------------------------------------

/// Goes through the file a line at a time, the same rules as
/// `conductor_globals.cfg`: `#` comments, `key = value`, keys in any case,
/// the later of two wins.  Hands back every complaint, one per line, and
/// the keys the file had, good value or not.
fn parse_text(text: &str, settings: &mut DbSettings) -> (Vec<String>, HashSet<String>) {
    let mut problems = Vec::new();
    let mut set_on: HashMap<String, usize> = HashMap::new();
    let mut seen = HashSet::new();

    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // The line itself isn't in this complaint, because it might be the
        // password line with the `=` left out.
        let Some((key, value)) = line.split_once('=') else {
            problems.push(format!("line {number} isn't a \"key = value\" line.  Ignored."));
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        seen.insert(key.clone());

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

    (problems, seen)
}

/// Puts one value into the settings, if the key is one we know and the
/// value makes sense for it.
fn apply_setting(settings: &mut DbSettings, key: &str, value: &str) -> Result<(), String> {
    match key {
        "address" => settings.address = parse_word(key, value)?,
        "port" => settings.port = parse_number(key, value, 1, u16::MAX as u64)? as u16,
        "database" => settings.database = parse_word(key, value)?,
        "username" => settings.username = parse_word(key, value)?,
        // An empty password is allowed here.  It's what a fresh file has,
        // and load() shouts about it.
        "password" => settings.password = value.to_string(),
        "query_time_limit_seconds" => settings.query_time_limit_seconds = parse_number(key, value, 0, 3600)?,
        "slow_job_ms" => settings.slow_job_ms = parse_number(key, value, 1, 600_000)?,
        _ => return Err(format!("There is no setting called {key}.")),
    }
    Ok(())
}

/// A name or an address.  Anything but nothing.
fn parse_word(key: &str, value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err(format!("{key} is empty."));
    }
    Ok(value.to_string())
}

/// A whole number from `low` to `high`.
fn parse_number(key: &str, value: &str, low: u64, high: u64) -> Result<u64, String> {
    match value.parse::<u64>() {
        Ok(number) if (low..=high).contains(&number) => Ok(number),
        _ => Err(format!("{key} has to be a number from {low} to {high}, and \"{value}\" isn't.")),
    }
}

// ---------------------------------------------------------------------------
// Writing the file
// ---------------------------------------------------------------------------

/// The top of a fresh file.
const FILE_HEADER: &str = "\
# Where Archivist finds Postgres, and how it runs.  One
# \"key = value\" a line, and \"#\" starts a comment.  Conductor wrote this
# file because there wasn't one.  Stop the server before editing it; it is
# only read at startup.
";

/// Every setting as a little block of text: what it is, then the line
/// itself.  The file is these one after another, and a setting that goes
/// missing from the file gets its block added to the end.
fn setting_blocks(settings: &DbSettings) -> Vec<(&'static str, String)> {
    vec![
        ("address", format!("\
# The machine Postgres is on.
address = {}
", settings.address)),
        ("port", format!("\
# The port Postgres listens on.
port = {}
", settings.port)),
        ("database", format!("\
# The database Conductor uses.
database = {}
", settings.database)),
        ("username", format!("\
# The role Conductor logs in as.  It owns the game's tables.
username = {}
", settings.username)),
        ("password", format!("\
# The role's password, as it is, no quotes.
password = {}
", settings.password)),
        ("query_time_limit_seconds", format!("\
# Postgres cancels any query that runs longer than this many seconds, and
# the job comes back as an error.  0 means no limit.
query_time_limit_seconds = {}
", settings.query_time_limit_seconds)),
        ("slow_job_ms", format!("\
# A job that takes at least this many milliseconds is logged as slow.
slow_job_ms = {}
", settings.slow_job_ms)),
    ]
}

/// The whole text of a `postgres.cfg` holding these settings.  Whatever
/// this writes, parse_text() has to read back without a complaint.
fn file_text(settings: &DbSettings) -> String {
    let mut text = FILE_HEADER.to_string();
    for (_, block) in setting_blocks(settings) {
        text.push('\n');
        text.push_str(&block);
    }
    text
}

/// The text to add to the end of a file that has only the keys in `seen`,
/// or `None` if it has them all.
fn missing_text(seen: &HashSet<String>) -> Option<String> {
    let missing: Vec<(&str, String)> = setting_blocks(&default_settings())
        .into_iter()
        .filter(|(key, _)| !seen.contains(*key))
        .collect();
    if missing.is_empty() {
        return None;
    }

    let mut text = "\n# Conductor added the settings below with their defaults, because it knows\n\
                    # them and they weren't in this file.\n".to_string();
    for (_, block) in missing {
        text.push('\n');
        text.push_str(&block);
    }
    Some(text)
}

/// Adds any setting the file doesn't have to the end of it.  Nothing that
/// is already in the file gets touched.
fn add_missing(path: &Path, seen: &HashSet<String>) {
    let Some(text) = missing_text(seen) else {
        return;
    };

    match diskman::append(path, text.as_bytes()).wait() {
        Ok(()) => scribe::info(Channel::Database, &format!("Archivist added the settings missing from {}, \
            with their defaults.", path.display())),
        Err(e) => scribe::warn_with(Channel::Database, &e, &format!("Archivist couldn't add the missing \
            settings to {}.  They run on their defaults.", path.display())),
    }
}

/// There was no `postgres.cfg`, so we write one with the defaults and an
/// empty password for the admin to fill in.  DiskMan makes the folder if
/// it has to.
fn write_default_file(path: &Path) {
    match diskman::write(path, file_text(&default_settings()).into_bytes()).wait() {
        Ok(()) => scribe::info(Channel::Database, &format!("No postgres.cfg, so Archivist wrote one: {}",
                                                          path.display())),
        Err(e) => scribe::error_with(Channel::Database, &e, &format!("No postgres.cfg, and Archivist \
            can't write one at {}.", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled_in() -> DbSettings {
        DbSettings {
            address: "db.example".to_string(),
            port: 6543,
            database: "otherdb".to_string(),
            username: "someone".to_string(),
            password: "p@ss = word".to_string(),
            query_time_limit_seconds: 0,
            slow_job_ms: 1000,
        }
    }

    #[test]
    fn what_we_write_we_can_read() {
        let mut read_back = default_settings();
        let (problems, seen) = parse_text(&file_text(&filled_in()), &mut read_back);

        assert!(problems.is_empty(), "complaints: {problems:?}");
        assert_eq!(read_back, filled_in());
        assert!(missing_text(&seen).is_none());
    }

    #[test]
    fn the_default_file_reads_clean() {
        let mut settings = default_settings();
        let (problems, _) = parse_text(&file_text(&default_settings()), &mut settings);
        assert!(problems.is_empty());
        assert_eq!(settings, default_settings());
    }

    #[test]
    fn missing_settings_get_added_and_read_back() {
        // An older file, from before the time limit and slow jobs existed.
        let old = "address = localhost\nport = 5432\ndatabase = opusdb\nusername = opus_game\npassword = x\n";
        let mut settings = default_settings();
        let (_, seen) = parse_text(old, &mut settings);

        let added = missing_text(&seen).expect("the newer settings are missing");
        assert!(added.contains("query_time_limit_seconds = 10"));
        assert!(!added.contains("address"));

        let mut read_back = default_settings();
        let (problems, seen) = parse_text(&format!("{old}{added}"), &mut read_back);
        assert!(problems.is_empty(), "complaints: {problems:?}");
        assert!(missing_text(&seen).is_none());
        assert_eq!(read_back.password, "x");
    }

    #[test]
    fn a_bad_number_keeps_the_default() {
        for line in ["port = 0", "port = 65536", "port = five", "slow_job_ms = 0", "query_time_limit_seconds = -1",
                     "query_time_limit_seconds = 3601"] {
            let mut settings = default_settings();
            let (problems, _) = parse_text(line, &mut settings);

            assert_eq!(problems.len(), 1, "{line}");
            assert_eq!(settings, default_settings(), "{line}");
        }
    }

    #[test]
    fn an_unknown_key_is_a_complaint() {
        let mut settings = default_settings();
        let (problems, _) = parse_text("# a comment\n\npasword = typo\n", &mut settings);

        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("line 3:"));
        assert_eq!(settings, default_settings());
    }

    #[test]
    fn a_broken_line_does_not_show_itself() {
        // A password line with the `=` forgotten must not end up in the log.
        let mut settings = default_settings();
        let (problems, _) = parse_text("password hunter2\n", &mut settings);

        assert_eq!(problems.len(), 1);
        assert!(!problems[0].contains("hunter2"));
    }

    #[test]
    fn debug_never_shows_the_password() {
        let shown = format!("{:?}", filled_in());
        assert!(!shown.contains("p@ss"));
        assert!(shown.contains("(hidden)"));
    }
}
