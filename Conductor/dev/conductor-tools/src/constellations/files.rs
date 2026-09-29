//! File:       Opus/Conductor/dev/conductor-tools/src/constellations/files.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Every config file Conductor has, and every setting in each, written
//! down once.  This table is the whole truth about the files: what the
//! reader checks, what a fresh file is written with, what gets appended
//! to a file that is missing a setting, and what the web admin's editor
//! will show.  Adding a setting is one entry here and one line wherever
//! it's read.
//!
//! The files all live in `Content/cfg/`.  Each one is soft or hard as a
//! whole, never a mix: a piece that needs both kinds gets two files.
//! **Soft** means the server pieces read it when the server starts, so
//! STOP SERVER and START SERVER (or RESTART SERVER) on the Control Panel
//! is enough.  **Hard** means the program reads it at boot, so Conductor
//! has to be shut down and run again.  Jacob's rule, 2026-09-29.

use crate::scribe::Channel;

/// Which reboot a file's settings wait on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reboot {
    /// STOP SERVER then START SERVER on the Control Panel.  The pieces
    /// that read the file are server pieces, and they read it on every
    /// start.
    Soft,
    /// Conductor shut down and run again.  The program reads the file at
    /// boot and never again.
    Hard,
}

impl Reboot {
    /// What the reboot is, for the log and the page.
    pub fn describe(self) -> &'static str {
        match self {
            Reboot::Soft => "a soft reboot (STOP SERVER and START SERVER on the Control Panel)",
            Reboot::Hard => "a hard reboot (Conductor shut down and run again)",
        }
    }
}

/// What a setting's value has to be.  The check is in `text.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Anything but nothing: a name, an address.
    Text,
    /// Anything at all, empty included, and never echoed in a complaint
    /// or a log line.  The Postgres password.  (It can still be shown on
    /// the web admin's page: Postgres only listens on this machine.)
    Secret,
    /// A folder.  A relative one is taken from the Content folder.
    Folder,
    /// 1 to 65535.
    Port,
    /// A whole number from `low` to `high`, both allowed.
    Number { low: u64, high: u64 },
}

/// One setting: the key in the file, what it has to be, what it is when
/// the file doesn't say, and the comment that goes above it.
#[derive(Debug)]
pub struct Setting {
    /// The key, as written in the file (lowercase; the reader takes any
    /// case).
    pub key: &'static str,
    pub kind: Kind,
    /// As it would be written in the file.
    pub default: &'static str,
    /// The comment above the line, already wrapped at 78 with `# ` in
    /// front of each line in mind.
    pub about: &'static str,
}

/// One config file.
#[derive(Debug)]
pub struct ConfigFile {
    /// The file's name, under `Content/cfg/`.
    pub name: &'static str,
    pub reboot: Reboot,
    /// Which channel its complaints go out on.
    pub channel: Channel,
    /// The comment at the top of a fresh file, wrapped the same way as a
    /// setting's `about`.  The reboot rule is added under it by the
    /// writer, so it isn't repeated here.
    pub about: &'static str,
    pub settings: &'static [Setting],
}

impl ConfigFile {
    /// The setting called `key`, if there is one.
    pub fn setting(&self, key: &str) -> Option<&'static Setting> {
        self.settings.iter().find(|setting| setting.key == key)
    }
}

/// `Content/cfg/conductor_globals.cfg`: the program's own settings.
/// Constellations, Scribe and the web admin read it at boot, so it's hard.
pub static GLOBALS: ConfigFile = ConfigFile {
    name: "conductor_globals.cfg",
    reboot: Reboot::Hard,
    channel: Channel::System,
    about: "Conductor's own settings: Scribe's log folder and the web admin's port.\n\
            One \"key = value\" a line, and \"#\" starts a comment.  Paths are\n\
            relative to the Content folder unless they start with \"/\".",
    settings: &[
        Setting {
            key: "scribe_log_dir",
            kind: Kind::Folder,
            default: "logs",
            about: "The folder Scribe writes its logs into.  One file a day, named by the\n\
                    UTC date, rolling over at midnight UTC.",
        },
        Setting {
            key: "wgui_port",
            kind: Kind::Port,
            default: "9996",
            about: "The port the web admin listens on.  It only listens on 127.0.0.1, so it\n\
                    can't be reached from another machine.  Open http://127.0.0.1:<port>/Opus\n\
                    in a browser on this one.",
        },
    ],
};

/// `Content/cfg/wgui.cfg`: the web admin's own settings, which today are
/// its two accounts.  The web admin reads it at boot, so it's hard.
///
/// The accounts are fixed: `user` can look at every tab and change
/// nothing, `admin` can do everything.  Only their passwords are
/// settings.  They're kept as they are, not hashed: Security only runs
/// while the server does, and the login has to work before START SERVER.
/// Fine while the page is only reachable from this machine, the same
/// call as the Postgres password.  Jacob's design, 2026-09-29.
pub static WGUI: ConfigFile = ConfigFile {
    name: "wgui.cfg",
    reboot: Reboot::Hard,
    channel: Channel::System,
    about: "The web admin's settings: the passwords of its two accounts.  One\n\
            \"key = value\" a line, and \"#\" starts a comment.  The accounts are\n\
            \"user\", who can look at everything and change nothing, and \"admin\", who\n\
            can do everything.  The passwords are kept as they are, which is fine\n\
            as long as the web admin only listens on this machine, which it does.",
    settings: &[
        Setting {
            key: "user_password",
            kind: Kind::Text,
            default: "user",
            about: "The password for \"user\", the account that can look and not touch.\n\
                    It can't be empty.",
        },
        Setting {
            key: "admin_password",
            kind: Kind::Text,
            default: "admin",
            about: "The password for \"admin\", the account that can start and stop the\n\
                    server, ACK notices, change settings and shut Conductor down.  It\n\
                    can't be empty.",
        },
    ],
};

/// `Content/cfg/postgres.cfg`: where Archivist finds Postgres and how it
/// runs.  Archivist reads it on every START SERVER, so it's soft.
pub static POSTGRES: ConfigFile = ConfigFile {
    name: "postgres.cfg",
    reboot: Reboot::Soft,
    channel: Channel::Database,
    about: "Where Archivist finds Postgres, and how it runs.  One \"key = value\" a\n\
            line, and \"#\" starts a comment.  It holds the database password, which\n\
            is fine as long as Postgres only listens on this machine.",
    settings: &[
        Setting {
            key: "address",
            kind: Kind::Text,
            default: "localhost",
            about: "The machine Postgres is on.",
        },
        Setting {
            key: "port",
            kind: Kind::Port,
            default: "5432",
            about: "The port Postgres listens on.",
        },
        Setting {
            key: "database",
            kind: Kind::Text,
            default: "opusdb",
            about: "The database Conductor uses.",
        },
        Setting {
            key: "username",
            kind: Kind::Text,
            default: "opus_game",
            about: "The role Conductor logs in as.  It owns the game's tables.",
        },
        Setting {
            key: "password",
            kind: Kind::Secret,
            default: "",
            about: "The role's password, as it is, no quotes.",
        },
        Setting {
            key: "query_time_limit_seconds",
            kind: Kind::Number { low: 0, high: 3600 },
            default: "10",
            about: "Postgres cancels any query that runs longer than this many seconds, and\n\
                    the job comes back as an error.  0 means no limit.",
        },
        Setting {
            key: "slow_job_ms",
            kind: Kind::Number { low: 1, high: 600_000 },
            default: "250",
            about: "A job that takes at least this many milliseconds is logged as slow.",
        },
    ],
};

/// `Content/cfg/networking.cfg`: where Conductor listens for players, the
/// TLS files, and how long a player gets at each step.  Networking reads
/// it on every START SERVER, so it's soft.
pub static NETWORKING: ConfigFile = ConfigFile {
    name: "networking.cfg",
    reboot: Reboot::Soft,
    channel: Channel::Network,
    about: "Where Conductor listens for players, and how it treats them.  One\n\
            \"key = value\" a line, and \"#\" starts a comment.  Paths are relative\n\
            to the Content folder unless they start with \"/\".",
    settings: &[
        Setting {
            key: "bind_address",
            kind: Kind::Text,
            default: "0.0.0.0",
            about: "The address to listen on, for TCP and UDP both.  0.0.0.0 is every\n\
                    address this machine has; 127.0.0.1 is this machine only.",
        },
        Setting {
            key: "tcp_port",
            kind: Kind::Port,
            default: "9997",
            about: "The port players log in on, inside TLS.  The connection closes once\n\
                    the player has their ticket for UDP.",
        },
        Setting {
            key: "udp_port",
            kind: Kind::Port,
            default: "9998",
            about: "The port the game runs on.  A player is told it with their ticket.",
        },
        Setting {
            key: "certificate_file",
            kind: Kind::Text,
            default: "certs/conductor.crt",
            about: "The TLS certificate, a PEM file.  Clients trust this one file and no\n\
                    authority.  Make it with openssl; the log says how when it's missing.",
        },
        Setting {
            key: "private_key_file",
            kind: Kind::Text,
            default: "certs/conductor.key",
            about: "The certificate's private key, a PEM file.  It never leaves this\n\
                    machine and never goes in git.",
        },
        Setting {
            key: "secret_word",
            kind: Kind::Text,
            default: "potato",
            about: "What a client has to say with its login.  Not a secret from anybody\n\
                    with a copy of the client; it turns port scanners away before they\n\
                    cost a hash.",
        },
        Setting {
            key: "client_versions",
            kind: Kind::Text,
            default: "0.0.1",
            about: "The client versions let in, separated by commas.  Any other is told\n\
                    \"Outdated Client Failure\" before its password is looked at.",
        },
        Setting {
            key: "login_deadline_seconds",
            kind: Kind::Number { low: 1, high: 600 },
            default: "10",
            about: "How long a connection gets to finish TLS and send its login before\n\
                    it's closed.  Time spent waiting in Security's line doesn't count.",
        },
        Setting {
            key: "login_threads",
            kind: Kind::Number { low: 1, high: 256 },
            default: "8",
            about: "How many logins can be in progress at once.  Security hashes one at\n\
                    a time whatever this says; the rest wait in its line.",
        },
        Setting {
            key: "max_waiting_logins",
            kind: Kind::Number { low: 1, high: 10_000 },
            default: "64",
            about: "How many connections can wait for a login thread before new ones\n\
                    are turned away at the door.",
        },
        Setting {
            key: "token_deadline_seconds",
            kind: Kind::Number { low: 1, high: 600 },
            default: "30",
            about: "How long a ticket stays good if the client never shows up on UDP\n\
                    with it.",
        },
        Setting {
            key: "udp_timeout_seconds",
            kind: Kind::Number { low: 1, high: 3600 },
            default: "40",
            about: "How long a player can go without sending anything over UDP before\n\
                    they're dropped.  Clients send a keep-alive every second.",
        },
        Setting {
            key: "connections_remember_seconds",
            kind: Kind::Number { low: 10, high: 86_400 },
            default: "300",
            about: "How long a finished connection stays on the web admin's TCP tab,\n\
                    counted from when it arrived.  One still in progress stays whatever\n\
                    this says.",
        },
    ],
};

/// Every config file, in the order the page lists them.  A new file goes
/// here and nowhere else.
pub static FILES: [&ConfigFile; 4] = [&GLOBALS, &WGUI, &POSTGRES, &NETWORKING];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_passes_its_own_check() {
        for file in FILES {
            for setting in file.settings {
                assert!(super::super::text::check(setting, setting.default).is_ok(),
                        "{}: the default for {} doesn't pass its own check", file.name, setting.key);
            }
        }
    }

    #[test]
    fn no_key_is_used_twice_and_all_are_lowercase() {
        for file in FILES {
            for (index, setting) in file.settings.iter().enumerate() {
                assert_eq!(setting.key, setting.key.to_ascii_lowercase(), "{}", file.name);
                assert!(!file.settings[..index].iter().any(|other| other.key == setting.key),
                        "{}: {} is in the table twice", file.name, setting.key);
            }
        }
    }

    #[test]
    fn every_file_name_is_used_once() {
        for (index, file) in FILES.iter().enumerate() {
            assert!(!FILES[..index].iter().any(|other| other.name == file.name), "{} twice", file.name);
        }
    }
}
