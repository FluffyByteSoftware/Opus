//! File:       Opus/Conductor/dev/networking/src/lib.rs
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
//! A server piece: START SERVER calls `wait_for_world()`, and the
//! launcher calls `start()` once the ground around 0,0,0 is in, and
//! `stop()` on STOP SERVER.  It has to come up again after that.  Nothing
//! in here touches the world itself: a character goes in and out through
//! the GameClock's mailbox (`conductor_gameclock::enter()` and `leave()`),
//! which never waits.  What a player types is `conductor-player-commands`'
//! business: it leans on this crate (the book, the packets, sending), so
//! this crate never names it; the launcher hands its dispatcher to
//! `typed.rs`'s slot, and its senders to the GameClock, on every START
//! SERVER.
//!
//! Written with the CPU in mind and the RAM less so, Jacob's ask.  Logins
//! run on a fixed handful of threads rather than one per connection, so a
//! flood of connections starts no threads; a player is found by their
//! address in a map, never by a search; every fixed answer is built once
//! and sent as it is; and every thread sleeps in the OS's own wait calls,
//! never in a polling loop.  Security hashes one login at a time whatever
//! happens here, and that is the ceiling on what a login can cost.

use std::net::SocketAddr;

use conductor_tools::constellations;
use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};

mod access;
mod chunks;
mod dns;
mod ledger;
mod overworld;
pub mod protocol;
mod protogame;
mod sessions;
mod settings;
mod tcp;
mod tls;
pub mod typed;
mod udp;
mod view;

pub use access::{Entry, List, Mode as AccessMode, Snapshot as AccessLists};
pub use ledger::{Connection, End, Gone, Stage};
pub use sessions::PlayerView as Player;
pub use tcp::Kicked;

// What conductor-player-commands needs of the book and the UDP side: the
// anti-flood, who's in the world, an ask's answer, and sending.  The
// modules themselves stay ours.
pub use sessions::{finish_ask, in_world, may_command};
pub use udp::{tell_all, tell_answer};

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
    /// Every connection that reached the TCP listener since START SERVER,
    /// newest first, and where each one is (the oldest finished ones go
    /// past ten thousand).  Empty while the TCP side isn't running.
    pub connections: Vec<Connection>,
    /// Every player connected over UDP, newest first, whether at
    /// character select or with their character in the world.
    pub in_world: Vec<Player>,
    /// Which access list the door is checking, if either.
    pub access: AccessMode,
    /// How many entries each list has.
    pub whitelisted: usize,
    pub blacklisted: usize,
}

/// What a change to a list did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed {
    /// The entry as it's kept, host bits cleared.
    pub entry: String,
    /// False if there was nothing to do: listed already, or not on the
    /// list to take off.
    pub changed: bool,
    /// Whether the door is checking that list right now
    /// (`access_list` in networking.cfg).  A change to a list that isn't
    /// switched on is kept, and does nothing until it is.
    pub enforced: bool,
    /// When the change shut somebody out (a blacklisting, or a whitelist
    /// entry taken away, with that list on): the open TCP connections
    /// closed and the players dropped from the world, then and there.
    pub tcp_closed: usize,
    pub players_dropped: usize,
}

/// Brings both sides up: `networking.cfg` is read again, the TLS files
/// are loaded, TCP listens, Protogame starts, then UDP listens.  If the TLS files are missing or
/// TCP can't listen, nothing listens and the Services tab says why; if
/// UDP can't listen, TCP comes back down, so it's both or neither.  None
/// of it stops the rest of the server.  The launcher calls this last of
/// all, once the GameClock has the ground around 0,0,0 in, so Archivist
/// (a login reads the accounts table) is up long before.
pub fn start() {
    services::set(services::NETWORK_TCP, State::Starting, "Reading networking.cfg and the TLS files.");
    services::set(services::NETWORK_UDP, State::Starting, "Waiting on the TCP side.");
    services::set(services::PROTOGAME, State::Starting, "Waiting on the TCP side.");

    constellations::load(&constellations::NETWORKING);
    let settings = settings::load();

    let tls = match tls::server_config(&settings) {
        Ok(tls) => tls,
        Err(why) => {
            services::set(services::NETWORK_TCP, State::Trouble, &why);
            services::set(services::NETWORK_UDP, State::Stopped, "Not started: the TCP side couldn't.");
            services::set(services::PROTOGAME, State::Stopped, "Not started: the TCP side couldn't.");
            scribe::error(Channel::Network, &format!("NOBODY CAN LOG IN.  {why}"));
            return;
        }
    };

    // The map every player is sent at PLAY (protocol version 11), with its
    // hash worked out and its pieces built.  Without it nobody gets into
    // the world, so the door stays shut.
    if let Err(why) = overworld::load() {
        services::set(services::NETWORK_TCP, State::Trouble, &why);
        services::set(services::NETWORK_UDP, State::Stopped, "Not started: there's no map to send players.");
        services::set(services::PROTOGAME, State::Stopped, "Not started: there's no map to send players.");
        scribe::error(Channel::Network, &format!("NOBODY CAN LOG IN.  {why}"));
        return;
    }

    // The lists are built again from disk on every start, before the
    // door opens, so the first connection is checked too.
    access::start(settings.access_list, &settings.whitelist_file, &settings.blacklist_file);

    if let Err(why) = tcp::start(&settings, tls) {
        access::stop();
        services::set(services::NETWORK_TCP, State::Trouble, &why);
        services::set(services::NETWORK_UDP, State::Stopped, "Not started: the TCP side couldn't.");
        services::set(services::PROTOGAME, State::Stopped, "Not started: the TCP side couldn't.");
        scribe::error(Channel::Network, &format!("NOBODY CAN LOG IN.  {why}"));
        return;
    }

    // What each player sees of the world goes out from the GameClock
    // through us; it has nobody to send to until UDP is up.
    view::wire();

    // Protogame before UDP, so a player's first ask has somewhere to go.
    if let Err(why) = protogame::start(settings.map_cooldown) {
        tcp::stop();
        access::stop();
        services::set(services::NETWORK_TCP, State::Stopped, "Stopped again: Protogame couldn't start.");
        services::set(services::NETWORK_UDP, State::Stopped, "Not started: Protogame couldn't.");
        services::set(services::PROTOGAME, State::Trouble, &why);
        scribe::error(Channel::Network, &format!("NOBODY CAN LOG IN.  {why}  The TCP side was stopped again, \
            since a player couldn't get past character select."));
        return;
    }

    if let Err(why) = udp::start(&settings) {
        protogame::stop();
        tcp::stop();
        access::stop();
        services::set(services::NETWORK_TCP, State::Stopped, "Stopped again: the UDP side couldn't start.");
        services::set(services::NETWORK_UDP, State::Trouble, &why);
        scribe::error(Channel::Network, &format!("NOBODY CAN LOG IN.  {why}  The TCP side was stopped \
            again, since a login with nowhere to go is no use."));
        return;
    }

    scribe::info(Channel::Network, &format!("Networking is running.  Logins on TCP {}, the game on UDP {}.",
                                            settings.tcp_address(), settings.udp_address()));
}

/// The server has started with the door shut, waiting on the world.  This
/// only says so on the Services tab.  The launcher calls `start()` once
/// the GameClock has the ground around 0,0,0 in: nobody gets in before
/// there's a voxel to step on (Jacob, 2026-09-30).
pub fn wait_for_world() {
    let note = "Waiting on the world: the door opens once the chunks around 0,0,0 are in.";
    services::set(services::NETWORK_TCP, State::Starting, note);
    services::set(services::NETWORK_UDP, State::Starting, note);
    services::set(services::PROTOGAME, State::Starting, note);
}

/// Takes both sides down and waits for their threads.  TCP first, so no
/// new ticket is handed out while UDP is telling every player the server
/// is stopping.  Safe to call when nothing was started.  A door that never
/// opened (the world wasn't ready yet) says so on the Services tab, or it
/// would go on saying it's waiting.
pub fn stop() {
    let never_opened = tcp::listening_on().is_none() && udp::listening_on().is_none();
    tcp::stop();
    udp::stop();
    // After UDP, so no new ask comes in; it answers the ones it has first,
    // to nobody, since the players are gone.
    protogame::stop();
    access::stop();
    overworld::unload();
    if never_opened {
        services::set(services::NETWORK_TCP, State::Stopped, "Stopped.  The door never opened this run.");
        services::set(services::NETWORK_UDP, State::Stopped, "Stopped.  The door never opened this run.");
        services::set(services::PROTOGAME, State::Stopped, "Stopped.  The door never opened this run.");
    }
}

/// A copy of how networking is doing.
pub fn status() -> Status {
    let (players, tickets) = sessions::counts();
    let (whitelisted, blacklisted) = access::counts();
    Status { tcp: tcp::listening_on(), udp: udp::listening_on(), players, tickets, connections: ledger::snapshot(),
             in_world: sessions::players(), access: access::mode(), whitelisted, blacklisted }
}

/// The admin kicked row `id` (its number in `status()`'s list) from the
/// web admin's Connections tab.  A connection still open at the door is
/// closed where it stands, and the client sees it drop and nothing else.
/// One that logged in closed long ago, so the kick goes to what it
/// became: its player is told Kicked, kicked by the admin, and taken out
/// of the world, or its unused ticket dies.  Jacob's ask, 2026-09-29: the
/// three dots on a row kick, whatever the row is.
pub fn kick(id: u64) -> Kicked {
    match tcp::kick(id) {
        Kicked::NotOpen => {}
        other => return other,
    }
    match sessions::kick_login(id) {
        sessions::AdminKick::Player(address, account) => {
            udp::tell(address, &protocol::kicked(protocol::KickReason::KickedByAdmin));
            scribe::info(Channel::Security, &format!("The admin kicked {account} at {address} out of the world."));
            Kicked::FromWorld
        }
        sessions::AdminKick::Ticket(account) => {
            scribe::info(Channel::Security, &format!("The admin kicked {account} before their ticket was used."));
            Kicked::FromWorld
        }
        sessions::AdminKick::Nobody => Kicked::NotOpen,
    }
}

/// The admin deleted `account` on the web admin's Accounts tab.  Its
/// player, if it has one in the world, is told Kicked, account
/// terminated, and taken out; an unused ticket for it dies.  True if
/// there was either.  Jacob's ask, 2026-09-29.
pub fn terminate(account: &str) -> bool {
    match sessions::terminate(account) {
        sessions::Terminated::Player(address) => {
            udp::tell(address, &protocol::kicked(protocol::KickReason::AccountTerminated));
            scribe::info(Channel::Security, &format!("{account} at {address} was taken out of the world: the \
                account was deleted."));
            true
        }
        sessions::Terminated::Ticket => {
            scribe::info(Channel::Security, &format!("{account}'s unused ticket died: the account was deleted."));
            true
        }
        sessions::Terminated::Nobody => false,
    }
}

/// Both access lists as they stand, for the web admin's Whitelist and
/// Blacklist tabs.  `None` while the server isn't running: the lists
/// only load with it, and can only be changed while it is.
pub fn access_lists() -> Option<AccessLists> {
    access::snapshot()
}

/// The admin put `entry` on `list`, from the web admin.  It takes at
/// once and the file is written behind it.  A blacklisting while the
/// blacklist is on is a ban: everybody the door would now turn away is
/// dropped, then and there.  An `Err` while the server isn't running.
pub fn list_address(list: List, entry: Entry) -> Result<Changed, String> {
    let changed = access::add(list, entry)?;
    let enforced = is_enforced(list);
    let (tcp_closed, players_dropped) = if list == List::Blacklist && enforced { enforce() } else { (0, 0) };
    Ok(Changed { entry: entry.to_string(), changed, enforced, tcp_closed, players_dropped })
}

/// The admin took `entry` off `list`.  Taking an address off the
/// whitelist while the whitelist is on is a ban too, Jacob's rule: anybody
/// online from an address no other entry covers is dropped.  Taking one
/// off the blacklist lets it back in on its next connection and drops
/// nobody.
pub fn unlist_address(list: List, entry: Entry) -> Result<Changed, String> {
    let changed = access::remove(list, entry)?;
    let enforced = is_enforced(list);
    let (tcp_closed, players_dropped) = if list == List::Whitelist && enforced { enforce() } else { (0, 0) };
    Ok(Changed { entry: entry.to_string(), changed, enforced, tcp_closed, players_dropped })
}

/// Whether the door is checking `list` right now.
fn is_enforced(list: List) -> bool {
    match list {
        List::Whitelist => access::mode() == AccessMode::Whitelist,
        List::Blacklist => access::mode() == AccessMode::Blacklist,
    }
}

/// Drops everybody the door would turn away now: every open TCP
/// connection is closed where it stands (the ledger says banned) and
/// every player is told Kicked, reason banned, and forgotten.  Asking the
/// verdict again for each one, rather than matching the entry that
/// changed, is what makes a whitelist removal right when another entry
/// still covers the address.  How many of each.
fn enforce() -> (usize, usize) {
    let tcp_closed = tcp::close_where(|ip| access::verdict(ip) != access::Verdict::Allowed);
    let dropped = sessions::drop_where(|address| access::verdict(address.ip()) != access::Verdict::Allowed);
    let banned = protocol::kicked(protocol::KickReason::Banned);
    for (address, account) in &dropped {
        udp::tell(*address, &banned);
        scribe::info(Channel::Security, &format!("Banned: {account} at {address} was dropped from the world."));
    }
    (tcp_closed, dropped.len())
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
