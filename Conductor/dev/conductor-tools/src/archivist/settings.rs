//! File:       Opus/Conductor/dev/conductor-tools/src/archivist/settings.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! `Content/cfg/postgres.cfg` as Archivist reads it.  The file itself --
//! its settings, their checks, the comments, the missing ones added back
//! -- is Constellations' (`constellations/files.rs`, under `POSTGRES`).
//! This is only the typed view of it: the values in the shape the
//! postgres crate wants, and the shout when there's no password.

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use postgres::Config;

use crate::constellations::{self, POSTGRES};
use crate::scribe::{self, Channel};

/// How long a connect gets before we call it a failure.  Postgres is on
/// the same machine, so if it hasn't answered in 5 seconds it isn't going
/// to.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Everything `postgres.cfg` holds, typed.  The field names are the keys
/// in the file.
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
    /// What Constellations holds for `postgres.cfg` right now.
    fn from_constellations() -> DbSettings {
        DbSettings {
            address: constellations::value(&POSTGRES, "address"),
            port: constellations::port(&POSTGRES, "port"),
            database: constellations::value(&POSTGRES, "database"),
            username: constellations::value(&POSTGRES, "username"),
            password: constellations::value(&POSTGRES, "password"),
            query_time_limit_seconds: constellations::number(&POSTGRES, "query_time_limit_seconds"),
            slow_job_ms: constellations::number(&POSTGRES, "slow_job_ms"),
        }
    }

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
    constellations::path_of(&POSTGRES)
}

/// Has Constellations read `postgres.cfg` again (a missing file is
/// written, a missing setting added, complaints logged), and hands back
/// what it holds.  Every START SERVER comes through here, so a change to
/// the file only needs the server stopped and started.
pub(super) fn load() -> DbSettings {
    constellations::load(&POSTGRES);
    let settings = DbSettings::from_constellations();

    if settings.password.is_empty() {
        scribe::error(Channel::Database, &format!("NO POSTGRES PASSWORD, FILL IT IN BY HAND: {}.  \
            THERE IS NO DATABASE UNTIL IT IS, AND THE SERVER NEEDS A STOP AND A START AFTER.",
            config_path().display()));
    }

    settings
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
    fn the_defaults_come_through_when_nothing_is_loaded() {
        // The tests never load the file, so this is the table's defaults.
        let settings = DbSettings::from_constellations();
        assert_eq!(settings.address, "localhost");
        assert_eq!(settings.port, 5432);
        assert_eq!(settings.database, "opusdb");
        assert_eq!(settings.username, "opus_game");
        assert_eq!(settings.password, "");
        assert_eq!(settings.query_time_limit_seconds, 10);
        assert_eq!(settings.slow_job_ms, 250);
    }

    #[test]
    fn debug_never_shows_the_password() {
        let shown = format!("{:?}", filled_in());
        assert!(!shown.contains("p@ss"));
        assert!(shown.contains("(hidden)"));
    }

    #[test]
    fn where_to_never_shows_the_password() {
        assert!(!filled_in().where_to().contains("p@ss"));
    }
}
