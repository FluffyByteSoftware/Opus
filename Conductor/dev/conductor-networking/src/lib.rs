//! File:       Opus/Conductor/dev/conductor-networking/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Networking: the part of Conductor that talks to players.  Two sides.
//! TCP, inside TLS, is the front door: a client logs in there, and if the
//! password is right it's handed a ticket (a one-time token and the UDP
//! port) and the connection closes.  UDP is the game: the client shows up
//! with the ticket, and from then on everything goes over UDP.  When the
//! UDP session ends, for any reason, the player is gone, and the client
//! starts over at the login screen.  Jacob's design, 2026-09-29.
//!
//! A server piece: the launcher calls `start()` on START SERVER and
//! `stop()` on STOP SERVER, and it has to come up again after that.
//! Nothing in here touches the game (there isn't one yet).  When there
//! is, the UDP side hands it messages through a queue and never waits on
//! it.
//!
//! Written with the CPU in mind and the RAM less so, Jacob's ask.  Logins
//! run on a fixed handful of threads rather than one per connection, so a
//! flood of connections starts no threads; a player is found by their
//! address in a map, never by a search; every fixed answer is built once
//! and sent as it is; and every thread sleeps in the OS's own wait calls,
//! never in a polling loop.  Security hashes one login at a time whatever
//! happens here, and that is the ceiling on what a login can cost.

use std::net::SocketAddr;
use std::time::Duration;

use conductor_tools::constellations;
use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};

mod dns;
mod ledger;
pub mod protocol;
mod sessions;
mod settings;
mod tcp;
mod tls;
mod udp;

pub use ledger::{Connection, End, Stage};
pub use tcp::Kicked;

/// How networking is doing, for whoever asks (the web admin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// Where the TCP side is listening.  `None` while it isn't.
    pub tcp: Option<SocketAddr>,
    /// Where the UDP side is listening.  `None` while it isn't.
    pub udp: Option<SocketAddr>,
    /// Players connected over UDP right now.
    pub players: usize,
    /// Tickets handed out and not yet used on UDP.
    pub tickets: usize,
    /// How long a finished connection stays in `connections`
    /// (`connections_remember_seconds` in networking.cfg).
    pub remember: Duration,
    /// Every connection that reached the TCP listener in the last
    /// `remember`, newest first, and where each one is.  Empty while the
    /// TCP side isn't running.
    pub connections: Vec<Connection>,
}

/// Brings both sides up: `networking.cfg` is read again, the TLS files
/// are loaded, TCP listens, then UDP.  If the TLS files are missing or
/// TCP can't listen, nothing listens and the Services tab says why; if
/// UDP can't listen, TCP comes back down, so it's both or neither.  None
/// of it stops the rest of the server.  The launcher calls this after
/// Archivist, since a login reads the accounts table.
pub fn start() {
    services::set(services::NETWORK_TCP, State::Starting, "Reading networking.cfg and the TLS files.");
    services::set(services::NETWORK_UDP, State::Starting, "Waiting on the TCP side.");

    constellations::load(&constellations::NETWORKING);
    let settings = settings::load();

    let tls = match tls::server_config(&settings) {
        Ok(tls) => tls,
        Err(why) => {
            services::set(services::NETWORK_TCP, State::Trouble, &why);
            services::set(services::NETWORK_UDP, State::Stopped, "Not started: the TCP side couldn't.");
            scribe::error(Channel::Network, &format!("NOBODY CAN LOG IN.  {why}"));
            return;
        }
    };

    if let Err(why) = tcp::start(&settings, tls) {
        services::set(services::NETWORK_TCP, State::Trouble, &why);
        services::set(services::NETWORK_UDP, State::Stopped, "Not started: the TCP side couldn't.");
        scribe::error(Channel::Network, &format!("NOBODY CAN LOG IN.  {why}"));
        return;
    }

    if let Err(why) = udp::start(&settings) {
        tcp::stop();
        services::set(services::NETWORK_TCP, State::Stopped, "Stopped again: the UDP side couldn't start.");
        services::set(services::NETWORK_UDP, State::Trouble, &why);
        scribe::error(Channel::Network, &format!("NOBODY CAN LOG IN.  {why}  The TCP side was stopped \
            again, since a login with nowhere to go is no use."));
        return;
    }

    scribe::info(Channel::Network, &format!("Networking is running.  Logins on TCP {}, the game on UDP {}.",
                                            settings.tcp_address(), settings.udp_address()));
}

/// Takes both sides down and waits for their threads.  TCP first, so no
/// new ticket is handed out while UDP is telling every player the server
/// is stopping.  Safe to call when nothing was started.
pub fn stop() {
    tcp::stop();
    udp::stop();
}

/// A copy of how networking is doing.
pub fn status() -> Status {
    let (players, tickets) = sessions::counts();
    Status { tcp: tcp::listening_on(), udp: udp::listening_on(), players, tickets, remember: ledger::remember_for(),
             connections: ledger::snapshot() }
}

/// The admin kicked TCP connection `id` (its number in `status()`'s
/// list) from the web admin's TCP tab.  The connection is closed where
/// it stands; the client sees the connection drop and nothing else.
pub fn kick(id: u64) -> Kicked {
    tcp::kick(id)
}

// ---------------------------------------------------------------------------
// Shared by the two sides
// ---------------------------------------------------------------------------

/// A read or a receive that ran out of time.  Linux calls it WouldBlock
/// and some other systems TimedOut; either one only means nothing came.
pub(crate) fn timed_out(error: &std::io::Error) -> bool {
    matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut)
}

/// Where `stop()` knocks to wake a thread that's sitting in accept() or
/// recv_from().  Usually the listening address itself.  But "every
/// address" (0.0.0.0, or ::) isn't one anybody can connect to, so that
/// becomes localhost on the same port.
pub(crate) fn wake_address(address: SocketAddr) -> SocketAddr {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    if !address.ip().is_unspecified() {
        return address;
    }
    let localhost = match address.ip() {
        IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
    };
    SocketAddr::new(localhost, address.port())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_knock_on_every_address_goes_to_localhost() {
        let everything: SocketAddr = "0.0.0.0:9997".parse().unwrap();
        assert_eq!(wake_address(everything), "127.0.0.1:9997".parse().unwrap());
        let everything6: SocketAddr = "[::]:9997".parse().unwrap();
        assert_eq!(wake_address(everything6), "[::1]:9997".parse().unwrap());
        let one: SocketAddr = "10.0.0.5:9997".parse().unwrap();
        assert_eq!(wake_address(one), one);
    }
}
