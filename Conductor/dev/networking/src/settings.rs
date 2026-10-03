//! File:       Opus/Conductor/dev/networking/src/settings.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! `networking.cfg` as networking reads it: the typed view of the file's
//! values.  The file itself is Constellations' (`NETWORKING` in its
//! table), which checks every line and shows the file on the Settings
//! tab.  This only turns the checked text into addresses, paths, lists
//! and durations, once per START SERVER.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use conductor_tools::constellations::{self, NETWORKING};
use conductor_tools::scribe::{self, Channel};

use crate::access::Mode;

/// Everything networking is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The address both sides listen on.
    pub bind_address: IpAddr,
    pub tcp_port: u16,
    pub udp_port: u16,
    /// The TLS certificate and its key, as full paths.
    pub certificate_file: PathBuf,
    pub private_key_file: PathBuf,
    /// What a client says with its login to prove it's a client.
    pub secret_word: String,
    /// The client versions let in.
    pub client_versions: Vec<String>,
    /// How long a connection gets to finish TLS and send its login.
    pub login_deadline: Duration,
    /// How many login threads to run.
    pub login_threads: usize,
    /// How many connections may wait for one.
    pub max_waiting_logins: usize,
    /// How long an unused ticket stays good.
    pub token_deadline: Duration,
    /// How long a player may go quiet over UDP.
    pub udp_timeout: Duration,
    /// How long an account waits after being sent the map before it may
    /// be sent it again.  Zero is no wait.
    pub map_cooldown: Duration,
    /// Which access list the door looks at, if either.
    pub access_list: Mode,
    /// The two lists, as full paths.
    pub whitelist_file: PathBuf,
    pub blacklist_file: PathBuf,
}

impl Settings {
    pub fn tcp_address(&self) -> SocketAddr {
        SocketAddr::new(self.bind_address, self.tcp_port)
    }

    pub fn udp_address(&self) -> SocketAddr {
        SocketAddr::new(self.bind_address, self.udp_port)
    }
}

/// The file's values as Constellations holds them right now.  Every value
/// but the address and the access switch was checked when the file was
/// read; those two are plain text to Constellations, so they're checked
/// here.  An address that isn't one is an Error and "every address"
/// instead; a switch that isn't `off`, `whitelist` or `blacklist` is an
/// Error and `off`.  Nothing here stops the server.
pub fn load() -> Settings {
    let address_text = constellations::value(&NETWORKING, "bind_address");
    let bind_address = match address_text.parse::<IpAddr>() {
        Ok(address) => address,
        Err(_) => {
            scribe::error(Channel::Network, &format!("networking.cfg's bind_address, {address_text:?}, isn't an \
                IP address.  Listening on every address instead.  Fix it and STOP SERVER, START SERVER."));
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        }
    };
    let access_text = constellations::value(&NETWORKING, "access_list");
    let access_list = match Mode::parse(&access_text) {
        Some(mode) => mode,
        None => {
            scribe::error(Channel::Network, &format!("networking.cfg's access_list, {access_text:?}, isn't off, \
                whitelist or blacklist.  Checking nobody at the door instead.  Fix it and STOP SERVER, START \
                SERVER."));
            Mode::Off
        }
    };

    Settings {
        bind_address,
        tcp_port: constellations::port(&NETWORKING, "tcp_port"),
        udp_port: constellations::port(&NETWORKING, "udp_port"),
        certificate_file: constellations::content_dir().join(constellations::value(&NETWORKING, "certificate_file")),
        private_key_file: constellations::content_dir().join(constellations::value(&NETWORKING, "private_key_file")),
        secret_word: constellations::value(&NETWORKING, "secret_word"),
        client_versions: versions_in(&constellations::value(&NETWORKING, "client_versions")),
        login_deadline: seconds("login_deadline_seconds"),
        login_threads: constellations::number(&NETWORKING, "login_threads") as usize,
        max_waiting_logins: constellations::number(&NETWORKING, "max_waiting_logins") as usize,
        token_deadline: seconds("token_deadline_seconds"),
        udp_timeout: seconds("udp_timeout_seconds"),
        map_cooldown: seconds("map_cooldown_seconds"),
        access_list,
        whitelist_file: constellations::content_dir().join(constellations::value(&NETWORKING, "whitelist_file")),
        blacklist_file: constellations::content_dir().join(constellations::value(&NETWORKING, "blacklist_file")),
    }
}

fn seconds(key: &str) -> Duration {
    Duration::from_secs(constellations::number(&NETWORKING, key))
}

/// The versions in a comma-separated list, with the spaces around each
/// one taken off and empty ones skipped.
fn versions_in(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_come_apart_at_the_commas() {
        assert_eq!(versions_in("0.0.1"), vec!["0.0.1"]);
        assert_eq!(versions_in(" 0.0.1 , 0.0.2,,0.1.0 "), vec!["0.0.1", "0.0.2", "0.1.0"]);
        assert!(versions_in("").is_empty());
        assert!(versions_in(" , ").is_empty());
    }

    #[test]
    fn the_defaults_read_as_settings() {
        // Nothing loaded, so Constellations hands back the table's
        // defaults, and every one of them has to turn into its type.
        let settings = load();
        assert_eq!(settings.bind_address, IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        assert_eq!(settings.tcp_port, 9997);
        assert_eq!(settings.udp_port, 9998);
        assert!(settings.certificate_file.ends_with("certs/conductor.crt"));
        assert!(settings.private_key_file.ends_with("certs/conductor.key"));
        assert_eq!(settings.client_versions, vec!["0.0.1"]);
        assert_eq!(settings.login_deadline, Duration::from_secs(10));
        assert_eq!(settings.login_threads, 8);
        assert_eq!(settings.max_waiting_logins, 64);
        assert_eq!(settings.token_deadline, Duration::from_secs(30));
        assert_eq!(settings.udp_timeout, Duration::from_secs(40));
        assert_eq!(settings.map_cooldown, Duration::from_secs(5));
        assert_eq!(settings.access_list, Mode::Off);
        assert!(settings.whitelist_file.ends_with("cfg/whitelist.cfg"));
        assert!(settings.blacklist_file.ends_with("cfg/blacklist.cfg"));
        assert_eq!(settings.tcp_address().port(), 9997);
        assert_eq!(settings.udp_address().port(), 9998);
    }
}
