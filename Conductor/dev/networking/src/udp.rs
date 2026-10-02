//! File:       Opus/Conductor/dev/networking/src/udp.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The UDP side, where the game goes.  A player gets here with the token
//! from their Ticket, and from then on everything they say and hear goes
//! over UDP.  The TCP connection that logged them in is already closed.
//!
//! UDP has no connections, so there are no connection threads.  One
//! thread, `net-udp`, reads every packet from every player off one
//! socket.  It knows who a packet is from by the address and port it
//! came from, which it learns from the player's first packet, the
//! Connect.  The token only goes over once.
//!
//! Anything it can't use gets no answer at all: junk, a packet too big,
//! and anything but a Connect from an address it doesn't know.  A server
//! that answers strangers can be used to flood somebody else.
//!
//! A player stays in by sending a KeepAlive once a second, and hears one
//! back each time, so the client knows we're still here too.  One who
//! goes `udp_timeout_seconds` without sending anything is dropped, told
//! nothing (they wouldn't hear it), and forgotten: the client sees its
//! own silence and goes back to the login screen.  Goodbye, being logged
//! out from somewhere else, and STOP SERVER end a session the same way,
//! the last two with a Kicked first.  Nothing is kept for a reconnect.
//!
//! The thread sleeps in recv_from() with a one-second limit.  Once a
//! second, packets or none, it checks in with the services list and
//! sweeps the book for players gone quiet and tickets gone stale.  The
//! answers that never change are built once (`Replies`) and sent as they
//! are, and a packet is looked at in the buffer it arrived in, so a
//! keep-alive costs a map lookup and a send and nothing else.
//!
//! A player starts at character select (protocol version 5): their asks
//! (the list of their characters, a new one, a delete, a reset home, and
//! playing one, version 6) are database jobs, so this thread hands each
//! to Protogame through its mailbox and never waits on it; Protogame
//! sends the answer.  An ask the book has already answered is answered
//! again from here, straight from the book.  Once their character is in
//! the world, character select is behind them, and an ask from there is
//! refused from here; they keep alive until they leave one way or
//! another, and their character leaves with them.
//!
//! A player in the world types lines (protocol version 8): a
//! PlayerCommand, answered from here, since nothing in it waits on the
//! database.  `typed.rs` hands the line to `conductor-player-commands`,
//! which finds the command and holds back a flood.  `/chat` makes a line
//! for everybody, which goes out from the GameClock's next broadcast
//! check through `tell_all()`; `/who` is answered on the spot, and `/who
//! list` from the GameClock's broadcast check through `tell_answer()`.
//! An answer too big for one packet goes out in Spans (protocol version
//! 9).

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};
use conductor_tools::threads;

use crate::access::{self, Verdict};
use crate::typed::{self, Asker, Outcome};
use crate::protocol::{self, ConnectAnswer, KickReason, PacketType};
use crate::protogame::{self, Work};
use crate::sessions::{self, Ask, Connected};
use crate::settings::Settings;
use crate::{timed_out, wake_address};

/// How long the thread waits on a receive before it sweeps and checks in.
const SWEEP_EVERY: Duration = Duration::from_secs(1);

/// What a player in the world hears for an ask from character select.
const CHARACTER_SELECT_IS_BEHIND: &str = "Your character is in the world.  Log out to get back to character \
    select.";

/// What a player at character select hears for a command.
const NOT_IN_THE_WORLD: &str = "Commands work once your character is in the world.";

/// How much we read in one go.  Bigger than MAX_UDP_BYTES on purpose: a
/// packet that doesn't fit the buffer gets cut to fit without a word, and
/// then it would look like one we take.  This way a too-big one comes in
/// too big, and is dropped.
const READ_BUFFER: usize = 2048;

/// The running UDP side.  Held in UDP below while the server is started.
struct UdpSide {
    /// Set by `stop()`.  The thread checks it after every receive.
    stopping: Arc<AtomicBool>,
    /// The socket, shared with the thread.  Sending on it is safe from
    /// any thread, which is how the TCP side kicks a player.
    socket: Arc<UdpSocket>,
    address: SocketAddr,
    listener: JoinHandle<()>,
}

// Rust note: the same shape as TCP in tcp.rs.  `None` means the UDP side
// isn't running.
static UDP: Mutex<Option<UdpSide>> = Mutex::new(None);

/// The answers that never change, built once.  The kick for "logged in
/// elsewhere" isn't here: the TCP side builds that one and sends it
/// through `tell()`.
struct Replies {
    keep_alive: Vec<u8>,
    accepted: Vec<u8>,
    refused: Vec<u8>,
    kicked_stopping: Vec<u8>,
}

static REPLIES: LazyLock<Replies> = LazyLock::new(|| Replies {
    keep_alive: protocol::keep_alive(),
    accepted: protocol::connect_result(ConnectAnswer::Accepted),
    refused: protocol::connect_result(ConnectAnswer::Refused),
    kicked_stopping: protocol::kicked(KickReason::ServerStopping),
});

/// Binds the address and starts the thread.  An `Err` says what went
/// wrong, in words, and nothing is running.
pub fn start(settings: &Settings) -> Result<(), String> {
    let mut guard = UDP.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.is_some() {
        return Err("The UDP side is already running.".to_string());
    }

    let address = settings.udp_address();
    let socket = UdpSocket::bind(address)
        .map_err(|e| format!("Couldn't listen on UDP {address}: {e}."))?;
    socket.set_read_timeout(Some(SWEEP_EVERY))
        .map_err(|e| format!("Couldn't set up the UDP socket on {address}: {e}."))?;
    let socket = Arc::new(socket);

    let stopping = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stopping);
    let shared = Arc::clone(&socket);
    let (udp_timeout, token_deadline) = (settings.udp_timeout, settings.token_deadline);
    let listener = threads::spawn("net-udp", move || listen(shared, flag, udp_timeout, token_deadline))
        .map_err(|e| format!("Couldn't start the UDP thread: {e}."))?;

    let note = format!("Listening on {address}.");
    services::set(services::NETWORK_UDP, State::Running, &note);
    scribe::info(Channel::Network, &format!("UDP: {note}"));
    *guard = Some(UdpSide { stopping, socket, address, listener });
    Ok(())
}

/// Tells every player the server is stopping, forgets them all (their
/// characters are asked out of the world, and saved), stops the thread
/// and waits for it.  Does nothing if the UDP side isn't running.
pub fn stop() {
    // Take the side out of the global first, so the lock isn't held while
    // we wait on the thread.
    let side = {
        let mut guard = UDP.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.take()
    };
    let Some(side) = side else {
        return;
    };

    let gone = sessions::clear();
    for (address, account) in &gone {
        send(&side.socket, *address, &REPLIES.kicked_stopping);
        scribe::debug(Channel::Network, &format!("{account} at {address} was told the server is stopping."));
    }

    // Wake recv_from() with an empty packet to ourselves.  The thread sees
    // the flag and ends.  If the knock doesn't land, the read wakes on
    // its own within SWEEP_EVERY anyway.
    side.stopping.store(true, Ordering::SeqCst);
    let _ = side.socket.send_to(&[], wake_address(side.address));
    let _ = side.listener.join();

    services::set(services::NETWORK_UDP, State::Stopped, "Stopped.");
    scribe::info(Channel::Network, &format!("UDP: stopped listening.  {} player(s) were told the server is \
        stopping.", gone.len()));
}

/// Sends `bytes` to `to` from the listening socket, from any thread.  For
/// the TCP side, kicking a player who was logged out from somewhere else.
/// Nothing happens if the UDP side isn't running.
pub fn tell(to: SocketAddr, bytes: &[u8]) {
    let guard = UDP.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(side) = guard.as_ref() {
        send(&side.socket, to, bytes);
    }
}

/// Sends every packet in `packets` to every address in `addresses`, from
/// the listening socket, from any thread.  For the chat, which goes out
/// from the GameClock's thread.  Nothing happens if the UDP side isn't
/// running.
pub fn tell_all(addresses: &[SocketAddr], packets: &[Vec<u8>]) {
    let guard = UDP.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(side) = guard.as_ref() {
        for address in addresses {
            for packet in packets {
                send(&side.socket, *address, packet);
            }
        }
    }
}

/// Sends the answer to the ask numbered `ask` to `to`, in Spans if it's
/// too big for one packet, from any thread.  For the GameClock's answer to
/// a `/who list`.  Nothing happens if the UDP side isn't running.
pub fn tell_answer(to: SocketAddr, ask: u32, answer: &[u8]) {
    let guard = UDP.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(side) = guard.as_ref() {
        send_answer(&side.socket, to, ask, answer);
    }
}

/// Where the UDP side is listening, if it is.
pub fn listening_on() -> Option<SocketAddr> {
    let guard = UDP.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.as_ref().map(|side| side.address)
}

// ---------------------------------------------------------------------------
// The thread
// ---------------------------------------------------------------------------

/// Reads a packet, deals with it, and once a second sweeps and checks in.
fn listen(socket: Arc<UdpSocket>, stopping: Arc<AtomicBool>, udp_timeout: Duration, token_deadline: Duration) {
    let mut buffer = [0u8; READ_BUFFER];
    let mut last_sweep = Instant::now();
    // Set once we've said the receive is failing, so a failure that keeps
    // happening is one Warn on the bell and not ten a second.
    let mut said_failed = false;

    loop {
        if stopping.load(Ordering::SeqCst) {
            return;
        }
        match socket.recv_from(&mut buffer) {
            Ok((size, from)) => {
                said_failed = false;
                heard(&socket, &buffer[..size], from);
            }
            Err(e) if timed_out(&e) => {}
            // Windows reports a packet we sent earlier bouncing off a
            // closed port as a failed receive.  It's about the other end,
            // not us, and means nothing here.
            Err(e) if e.kind() == io::ErrorKind::ConnectionReset => {}
            Err(e) => {
                if !said_failed {
                    said_failed = true;
                    scribe::warn(Channel::Network, &format!("UDP receive failed: {e}.  Said once; the next line \
                        about it is when it works again."));
                } else {
                    scribe::debug(Channel::Network, &format!("UDP receive failed again: {e}."));
                }
                // A failure that keeps happening would otherwise fill the
                // log as fast as the disk allows.
                thread::sleep(Duration::from_millis(100));
            }
        }

        if last_sweep.elapsed() >= SWEEP_EVERY {
            sweep(udp_timeout, token_deadline);
            services::seen(services::NETWORK_UDP);
            last_sweep = Instant::now();
        }
    }
}

/// One packet, from one address.
fn heard(socket: &UdpSocket, bytes: &[u8], from: SocketAddr) {
    let Some((kind, payload)) = protocol::take_datagram(bytes) else {
        return;
    };

    match PacketType::from_byte(kind) {
        Some(PacketType::KeepAlive) => {
            // Hearing it was the point.  A player hears one back; a
            // stranger hears nothing.
            if sessions::heard(from) {
                send(socket, from, &REPLIES.keep_alive);
            }
        }
        Some(PacketType::Goodbye) => {
            if let Some(account) = sessions::leave(from) {
                scribe::info(Channel::Security, &format!("{account} logged out from {from}."));
            }
        }
        Some(PacketType::Connect) => {
            // An address the door wouldn't let in, with a ticket in hand
            // from before it was listed, gets what a stranger gets:
            // nothing.  A keep-alive from a player already in the world
            // isn't checked; a ban drops them the moment it's made.
            if access::verdict(from.ip()) != Verdict::Allowed {
                scribe::debug(Channel::Network, &format!("Ignored a UDP connect from {from}: the access list \
                    turns that address away."));
                return;
            }
            let Ok(token) = protocol::read_connect(payload) else {
                return;
            };
            match sessions::connect(&token, from) {
                Connected::Accepted(account) => {
                    scribe::info(Channel::Security, &format!("{account} is at character select, over UDP from \
                        {from}."));
                    send(socket, from, &REPLIES.accepted);
                }
                // Our answer got lost, and the client is asking again.
                Connected::Again(_) => send(socket, from, &REPLIES.accepted),
                Connected::Refused => {
                    // Never the token.  Whether it was a guess or a copy,
                    // the log doesn't need it.  Debug, not Info: the
                    // address on a UDP packet can be anybody's, so a
                    // flood of these would be a flood of lines on disk.
                    scribe::debug(Channel::Security, &format!("Refused a UDP connect from {from}."));
                    send(socket, from, &REPLIES.refused);
                }
            }
        }
        // Character select.  One that can't be read counts as hearing
        // from a player, and gets no answer.
        Some(PacketType::CharacterListRequest) => match protocol::read_list_request(payload) {
            Ok(ask) => ask_protogame(socket, from, ask, Work::List),
            Err(_) => {
                sessions::heard(from);
            }
        },
        Some(PacketType::CreateCharacter) => match protocol::read_create(payload) {
            Ok((ask, name)) => ask_protogame(socket, from, ask, Work::Create { name }),
            Err(_) => {
                sessions::heard(from);
            }
        },
        Some(PacketType::DeleteCharacter) => match protocol::read_delete(payload) {
            Ok((ask, uuid, typed)) => ask_protogame(socket, from, ask, Work::Delete { uuid, typed }),
            Err(_) => {
                sessions::heard(from);
            }
        },
        Some(PacketType::CharacterRequestResetHome) => match protocol::read_reset_home(payload) {
            Ok((ask, uuid)) => ask_protogame(socket, from, ask, Work::ResetHome { uuid }),
            Err(_) => {
                sessions::heard(from);
            }
        },
        Some(PacketType::UserPressPlay) => match protocol::read_user_press_play(payload) {
            Ok((ask, uuid)) => ask_protogame(socket, from, ask, Work::Play { uuid }),
            Err(_) => {
                sessions::heard(from);
            }
        },
        // A line the player typed: a command, `/chat` so far.
        Some(PacketType::PlayerCommand) => match protocol::read_player_command(payload) {
            Ok((ask, line)) => player_command(socket, from, ask, &line),
            Err(_) => {
                sessions::heard(from);
            }
        },
        // Anything else from a player counts as hearing from them: there
        // are no other game packets yet.  From a stranger, silence.
        _ => {
            sessions::heard(from);
        }
    }
}

/// An ask from character select.  A stranger, or a player whose last ask
/// is still being worked on, hears nothing; one asked again gets the
/// answer it missed; one from a player in the world is refused here; a
/// new one goes to Protogame, which answers it.
fn ask_protogame(socket: &UdpSocket, from: SocketAddr, ask: u32, work: Work) {
    match sessions::begin_ask(from, ask) {
        Ask::Stranger | Ask::Busy => {}
        Ask::Again(answer) => send_answer(socket, from, ask, &answer),
        Ask::InWorld(account, _) => {
            let answer = protocol::command_refused(ask, CHARACTER_SELECT_IS_BEHIND);
            if sessions::finish_ask(from, &account, ask, &answer) {
                send(socket, from, &answer);
            }
        }
        Ask::New(account) => {
            // Protogame isn't running (which shouldn't happen while this
            // thread is): the answer is that it's unavailable, kept like
            // any other.
            if let Err(answer) = protogame::hand_in(from, account.clone(), ask, work) {
                if sessions::finish_ask(from, &account, ask, &answer) {
                    send(socket, from, &answer);
                }
            }
        }
    }
}

/// A line a player typed.  The other way round from character select:
/// only a player in the world may, and a player at character select is
/// refused.  A stranger, or a player whose last ask is still being worked
/// on, hears nothing; one asked again gets the answer it missed, so a
/// chat whose answer got lost isn't said twice.
fn player_command(socket: &UdpSocket, from: SocketAddr, ask: u32, line: &str) {
    let (account, answer) = match sessions::begin_ask(from, ask) {
        Ask::Stranger | Ask::Busy => return,
        Ask::Again(answer) => {
            send_answer(socket, from, ask, &answer);
            return;
        }
        Ask::New(account) => {
            let answer = protocol::command_refused(ask, NOT_IN_THE_WORLD);
            (account, answer)
        }
        Ask::InWorld(account, character) => {
            let asker = Asker { from, account: &account, character: &character, ask };
            match typed::run(&asker, line) {
                Outcome::Answer(answer) => (account, answer),
                // The GameClock answers it, and finishes the ask then.
                Outcome::Later => return,
            }
        }
    };
    if sessions::finish_ask(from, &account, ask, &answer) {
        send_answer(socket, from, ask, &answer);
    }
}

/// Drops everybody quiet for too long, and every ticket gone stale.
fn sweep(udp_timeout: Duration, token_deadline: Duration) {
    let swept = sessions::sweep(udp_timeout, token_deadline);
    for (address, account) in &swept.quiet {
        scribe::info(Channel::Security, &format!("{account} at {address} went quiet for {} seconds over UDP.  \
            Dropped; they start over at the login.", udp_timeout.as_secs()));
    }
    if swept.tickets_expired > 0 {
        scribe::debug(Channel::Network, &format!("{} ticket(s) ran out unused.", swept.tickets_expired));
    }
}

/// An answer, whole or in Spans (`protocol::spans()`).
fn send_answer(socket: &UdpSocket, to: SocketAddr, ask: u32, answer: &[u8]) {
    let packets = protocol::spans(ask, answer);
    if packets.is_empty() {
        scribe::warn(Channel::Network, &format!("An answer of {} bytes to {to} is too big to send, even in \
            pieces.", answer.len()));
    }
    for packet in &packets {
        send(socket, to, packet);
    }
}

fn send(socket: &UdpSocket, to: SocketAddr, bytes: &[u8]) {
    if let Err(e) = socket.send_to(bytes, to) {
        scribe::warn(Channel::Network, &format!("Couldn't send to {to} over UDP: {e}."));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_replies_are_what_the_protocol_says() {
        assert_eq!(REPLIES.keep_alive, vec![PacketType::KeepAlive as u8]);
        assert_eq!(REPLIES.accepted[0], PacketType::ConnectResult as u8);
        assert_eq!(REPLIES.accepted[1], ConnectAnswer::Accepted as u8);
        assert_eq!(REPLIES.refused[1], ConnectAnswer::Refused as u8);
        assert_eq!(REPLIES.kicked_stopping, vec![PacketType::Kicked as u8, 2, 0, 0, 0]);
    }

    #[test]
    fn a_stranger_hears_nothing_but_a_connect_answer() {
        // Two sockets on this machine: ours, and a stranger's.  Nothing
        // is started, so heard() is called by hand.
        let ours = UdpSocket::bind("127.0.0.1:0").unwrap();
        let stranger = UdpSocket::bind("127.0.0.1:0").unwrap();
        stranger.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        let from = stranger.local_addr().unwrap();
        let mut buffer = [0u8; 64];

        // A keep-alive, a goodbye, junk and an empty packet: silence.
        heard(&ours, &protocol::keep_alive(), from);
        heard(&ours, &[PacketType::Goodbye as u8], from);
        heard(&ours, &[0x77, 1, 2, 3], from);
        heard(&ours, &[], from);
        assert!(stranger.recv_from(&mut buffer).is_err());

        // A Connect with a token nobody was handed: refused, out loud.
        let mut connect = vec![PacketType::Connect as u8];
        connect.extend_from_slice(&64u32.to_le_bytes());
        connect.extend_from_slice("f".repeat(64).as_bytes());
        heard(&ours, &connect, from);
        let (size, _) = stranger.recv_from(&mut buffer).unwrap();
        assert_eq!(&buffer[..size], &REPLIES.refused[..]);

        // A Connect too short to be real: silence again.
        heard(&ours, &connect[..20], from);
        assert!(stranger.recv_from(&mut buffer).is_err());

        // Character select's asks from a stranger: silence too.
        heard(&ours, &[PacketType::CharacterListRequest as u8, 1, 0, 0, 0], from);
        let mut create = vec![PacketType::CreateCharacter as u8, 2, 0, 0, 0, 5, 0, 0, 0];
        create.extend_from_slice(b"Jacob");
        heard(&ours, &create, from);
        assert!(stranger.recv_from(&mut buffer).is_err());

        // A chat from a stranger: silence too.
        let mut said = vec![PacketType::PlayerCommand as u8, 3, 0, 0, 0, 9, 0, 0, 0];
        said.extend_from_slice(b"/chat Yo!");
        heard(&ours, &said, from);
        assert!(stranger.recv_from(&mut buffer).is_err());
    }

    #[test]
    fn stop_with_nothing_started_does_nothing() {
        stop();
        assert_eq!(listening_on(), None);
    }
}
