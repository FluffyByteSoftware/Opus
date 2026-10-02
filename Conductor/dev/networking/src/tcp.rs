//! File:       Opus/Conductor/dev/networking/src/tcp.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The TCP side: the front door.  A client connects, TLS comes up, the
//! client logs in, and if the password is right it's handed a ticket for
//! UDP and the connection closes.  That's the whole conversation.  Nothing
//! stays open here once a player is in the world.
//!
//! Two kinds of thread.  One acceptor, `net-tcp`, sits in accept() and
//! puts each new connection on a queue.  A fixed handful of login threads,
//! `net-login-1` and so on (`login_threads` in networking.cfg), take
//! connections off the queue one at a time and see each through.  A
//! thread per connection would cost a thread start per connection and a
//! thread-list entry for good, and it would buy nothing: Security hashes
//! one login at a time however many threads are waiting on it.  A
//! connection that finds the queue full is closed at the door, and one
//! that waited in the queue past the login deadline is closed unserved.
//!
//! No thread polls.  Each read is armed with exactly the time left to
//! the connection's deadline, so a thread sleeps in the OS's read until
//! bytes come or the time is up, and wakes for nothing else.  While a
//! login waits in Security's line, the thread waits on its ticket a
//! second at a time, and tells the client its place in between.  `stop()`
//! wakes the acceptor by connecting to it and the login threads by
//! shutting down the sockets they're reading, so a stop takes as long as
//! the slowest thread needs to notice, never a deadline.
//!
//! The login, in order: we say Hello, the client sends its version, the
//! secret word, the username and the password's key in one Login, and we
//! answer with a Ticket or a LoginResult.  (The key is what the client
//! makes from the password, and it stands in for the password from here
//! on: "password" below means the key.)  An old version is told so before
//! the password is looked at, and a wrong secret word, a name that
//! couldn't be an account, or a key that couldn't be one fails without a
//! hash.  A name that could be one has its
//! password hash read (conductor-accounts), and its password checked
//! through Security, in Security's line: a name with no account still
//! costs a hash there, so a stopwatch can't tell the two apart.  Every
//! failure gets the same answer, closes the connection, and makes the
//! address wait FAILURE_HOLD before its next connection is taken at all.
//! The right password for an account already in the world gets asked what
//! to do instead (sessions.rs); a right password otherwise gets a ticket.
//! A login that logs the other session out waits, before its ticket, for
//! that session's character to have its save in the database
//! (`conductor_gameclock::wait_until_saved()`), so it can't come in on the
//! save before (Jacob, 2026-10-01: "force a save on the connection being
//! kicked before the new one pops in").  Up to OLD_SAVE_WAIT, a second at
//! a time so a STOP SERVER is heard; past that it's Login Unavailable.
//! The ticket has the account's name and nothing else: an account is
//! never held in memory (Jacob, 2026-09-29), so the row is read when
//! something needs it.
//!
//! Every connection goes on the ledger (ledger.rs) as it's accepted, and
//! moves along it a stage at a time, so the web admin's Connections tab
//! can show where each one is.  Before any of that, the acceptor asks
//! the access lists (access.rs) about the address: one on the blacklist,
//! or off the whitelist, is closed at the door for the cost of an accept.
//! A clone of every open socket is kept under its ledger number from
//! accept until its login thread is done with it: that is what `stop()`
//! shuts to wake the threads, what `kick()` shuts when the admin kicks a
//! connection from the tab, and what `close_where()` shuts for a ban.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rustls::{ServerConfig, ServerConnection, StreamOwned};

use conductor_tools::scribe::{self, Channel};
use conductor_tools::security::{self, SecurityError, Ticket};
use conductor_tools::services::{self, State};
use conductor_tools::threads;

use crate::access::{self, Verdict};
use crate::ledger::{self, End, Stage};
use crate::protocol::{self, Choice, KickReason, LoginAnswer, LoginRequest, Packet, PacketType};
use crate::settings::Settings;
use crate::sessions::{self, Issued};
use crate::{dns, timed_out, udp, wake_address};

/// How long a player gets to answer "this account is already logged in".
/// A person is reading a prompt, so it's longer than the login deadline.
const CHOICE_DEADLINE: Duration = Duration::from_secs(30);

/// How long a write may take before we give up on the client.  A client
/// that stops reading would otherwise hold its thread once the socket's
/// buffer fills up.
const WRITE_WAIT: Duration = Duration::from_secs(5);

/// How long `stop()` waits for the login threads after waking them.
const STOP_WAIT: Duration = Duration::from_secs(2);

/// How long an address waits after a failed login before its next
/// connection is taken, counted from the failure.  It isn't a sleep: the
/// connection is closed at the door, which costs us nothing.
const FAILURE_HOLD: Duration = Duration::from_secs(2);

/// How often a login waiting in Security's line is told its place.
const PLACE_EVERY: Duration = Duration::from_secs(1);

/// How long a login that logged the other session out waits for that
/// session's character to have its save in the database (Jacob's 5
/// seconds).  It's normally a fraction of a second; longer means the
/// database is stuck, and the login gets Login Unavailable rather than
/// bring the character in on the save before.
const OLD_SAVE_WAIT: Duration = Duration::from_secs(5);

/// How long a connection gets to finish TLS, counted from when it
/// arrived, inside the login deadline.  A connection that sends nothing
/// would otherwise hold a login thread for the whole login deadline, and
/// eight of those every ten seconds would keep every real player out
/// (the 0.0.1 review's R2).  A handshake takes milliseconds; three
/// seconds is a slow link, not a client.
const HANDSHAKE_WAIT: Duration = Duration::from_secs(3);

/// How many connections one address may have open at the door at once
/// (queued or being served).  Past that its next one is closed at the
/// door.  A player needs one; a few covers a retry or two in flight.
const MOST_OPEN_PER_ADDRESS: usize = 4;

/// How many times a login goes round the "is the account in the world"
/// check before it gives up with Login Unavailable.  More than one means
/// somebody keeps coming into the world on the account from somewhere
/// else while this login waits for the last one's save; three is plenty.
const ISSUE_TRIES: usize = 3;

/// How much we ask TLS for in one read.  Bigger than any packet we take,
/// so one read can hold a whole one.
const READ_CHUNK: usize = 8192;

/// What every login thread needs, worked out once in `start()` and
/// shared.
struct Setup {
    tls: Arc<ServerConfig>,
    /// Handed to a player with their token.
    udp_port: u16,
    secret_word: String,
    client_versions: Vec<String>,
    login_deadline: Duration,
}

/// A connection the acceptor took, waiting for a login thread.
struct Arrival {
    /// Its number on the ledger, and its key in the `open` map.
    id: u64,
    socket: TcpStream,
    peer: SocketAddr,
    arrived: Instant,
}

/// Every connection's socket from accept until its login thread is done
/// with it, keyed by ledger number.  A clone: shutting it down shuts the
/// socket the thread is reading, and the read comes back at once.
type OpenSockets = Arc<Mutex<HashMap<u64, TcpStream>>>;

/// What a kick from the Connections tab got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kicked {
    /// The socket is shut.  The login thread finds out on its next read,
    /// or skips the connection if it was still queued.
    Yes,
    /// The connection had closed, but its login's player was in the world
    /// (or its ticket not yet used), and they're out now.  `lib.rs`'s
    /// `kick()` does that part.
    FromWorld,
    /// No connection with that number is open, and nothing from its login
    /// is left in the world: finished already, or never was.
    NotOpen,
    /// The TCP side isn't running.
    NotListening,
}

/// The running TCP side.  Held in TCP below while the server is started.
struct TcpSide {
    /// Set by `stop()`.  The acceptor checks it after every accept(), and
    /// the login threads after every connection and every second in line.
    stopping: Arc<AtomicBool>,
    address: SocketAddr,
    acceptor: JoinHandle<()>,
    workers: Vec<JoinHandle<()>>,
    /// Every open connection's socket, so `stop()` can shut them all and
    /// wake the threads out of their reads, and `kick()` can shut one.
    open: OpenSockets,
}

// Rust note: the same shape Security uses for its worker.  `None` means
// the TCP side isn't running.
static TCP: Mutex<Option<TcpSide>> = Mutex::new(None);

/// When each address last failed a login.  Only the ones inside
/// FAILURE_HOLD matter, and the rest are cleared out as new failures come
/// in, so it never grows for as long as the server runs.
static RECENT_FAILURES: LazyLock<Mutex<HashMap<IpAddr, Instant>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// The TLS connection once the handshake is done.  It reads and writes
/// like a socket and does the encrypting and decrypting on the way.
type TlsStream = StreamOwned<ServerConnection, TcpStream>;

/// Binds the address and starts the acceptor and the login threads.  An
/// `Err` says what went wrong, in words, and nothing is left running.
pub fn start(settings: &Settings, tls: Arc<ServerConfig>) -> Result<(), String> {
    let mut guard = TCP.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.is_some() {
        return Err("The TCP side is already running.".to_string());
    }

    let address = settings.tcp_address();
    let listener = TcpListener::bind(address)
        .map_err(|e| format!("Couldn't listen on TCP {address}: {e}."))?;

    let setup = Arc::new(Setup {
        tls,
        udp_port: settings.udp_port,
        secret_word: settings.secret_word.clone(),
        client_versions: settings.client_versions.clone(),
        login_deadline: settings.login_deadline,
    });
    let stopping = Arc::new(AtomicBool::new(false));
    let open: OpenSockets = Arc::new(Mutex::new(HashMap::new()));

    // A fresh ledger for a fresh run, and the name lookups behind it.
    ledger::start();
    dns::start();

    // Rust note: a sync_channel holds at most that many arrivals.  The
    // acceptor's try_send() says so when it's full instead of waiting,
    // and that's the "turned away at the door".
    let (queue, arrivals) = mpsc::sync_channel::<Arrival>(settings.max_waiting_logins);
    let arrivals = Arc::new(Mutex::new(arrivals));

    let mut workers = Vec::with_capacity(settings.login_threads);
    for number in 1..=settings.login_threads {
        let arrivals = Arc::clone(&arrivals);
        let setup = Arc::clone(&setup);
        let stopping = Arc::clone(&stopping);
        let open = Arc::clone(&open);
        let handle = threads::spawn(&format!("net-login-{number}"), move || work(arrivals, setup, stopping, open))
            // If one won't start, `queue` is dropped on the way out of
            // here, and the ones already started see the queue close and
            // end.
            .map_err(|e| format!("Couldn't start login thread {number}: {e}."))?;
        workers.push(handle);
    }

    let flag = Arc::clone(&stopping);
    let sockets = Arc::clone(&open);
    let acceptor = threads::spawn("net-tcp", move || accept(listener, queue, flag, sockets))
        .map_err(|e| format!("Couldn't start the TCP acceptor thread: {e}."))?;

    let note = format!("Listening on {address}.  {} login thread(s).", settings.login_threads);
    services::set(services::NETWORK_TCP, State::Running, &note);
    scribe::info(Channel::Network, &format!("TCP: {note}"));
    *guard = Some(TcpSide { stopping, address, acceptor, workers, open });
    Ok(())
}

/// Stops the acceptor and the login threads and waits for them.  Does
/// nothing if the TCP side isn't running.
pub fn stop() {
    // Take the side out of the global first, so the lock isn't held while
    // we wait on the threads.
    let side = {
        let mut guard = TCP.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.take()
    };
    let Some(side) = side else {
        return;
    };

    side.stopping.store(true, Ordering::SeqCst);

    // Wake accept() by connecting to ourselves.  The acceptor sees the
    // flag and ends, and its end of the queue goes with it, which is what
    // tells the login threads there's nothing more coming.
    let knock = wake_address(side.address);
    if let Err(e) = TcpStream::connect_timeout(&knock, Duration::from_secs(1)) {
        scribe::warn(Channel::Network, &format!("Couldn't wake the TCP acceptor on {knock}: {e}."));
    }
    let _ = side.acceptor.join();

    // Wake the login threads out of whatever read they're in.  A shutdown
    // on a clone shuts the socket itself, and the read comes back at
    // once.  The queued ones are shut here too; nobody serves them now.
    {
        let open = side.open.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for socket in open.values() {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }

    // Then wait for them, but not forever.
    let started = Instant::now();
    while side.workers.iter().any(|worker| !worker.is_finished()) && started.elapsed() < STOP_WAIT {
        thread::sleep(Duration::from_millis(10));
    }
    let mut left_running = 0;
    for worker in side.workers {
        if worker.is_finished() {
            let _ = worker.join();
        } else {
            left_running += 1;
        }
    }
    if left_running > 0 {
        scribe::warn(Channel::Network, &format!("{left_running} login thread(s) still busy after {} seconds.  \
            Stopping without them; they end when their connection does.", STOP_WAIT.as_secs()));
    }

    dns::stop();
    ledger::clear();
    services::set(services::NETWORK_TCP, State::Stopped, "Stopped.");
    scribe::info(Channel::Network, "TCP: stopped listening.");
}

/// The admin kicked connection `id` from the Connections tab.  The ledger says
/// so first, so the login thread's own "hung up" a moment later doesn't
/// overwrite it, then the socket is shut.  The client just sees the
/// connection close.
pub fn kick(id: u64) -> Kicked {
    let open = {
        let guard = TCP.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.as_ref() {
            Some(side) => Arc::clone(&side.open),
            None => return Kicked::NotListening,
        }
    };
    // Taken out of the map, so a second kick says it's gone.
    let socket = {
        let mut open = open.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        open.remove(&id)
    };
    let Some(socket) = socket else {
        return Kicked::NotOpen;
    };
    ledger::ended(id, End::Kicked);
    let peer = socket.peer_addr().map_or_else(|_| "a closed socket".to_string(), |peer| peer.to_string());
    let _ = socket.shutdown(Shutdown::Both);
    scribe::info(Channel::Network, &format!("The admin kicked {peer} at the door."));
    Kicked::Yes
}

/// The admin changed the access lists: every open connection whose
/// address `turned_away` says so for is closed where it stands, the way
/// a kick is, and the ledger says banned.  How many there were.
pub fn close_where(turned_away: impl Fn(IpAddr) -> bool) -> usize {
    let open = {
        let guard = TCP.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.as_ref() {
            Some(side) => Arc::clone(&side.open),
            None => return 0,
        }
    };
    let closing: Vec<(u64, TcpStream)> = {
        let mut open = open.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let ids: Vec<u64> = open.iter()
            .filter(|(_, socket)| socket.peer_addr().is_ok_and(|peer| turned_away(peer.ip())))
            .map(|(id, _)| *id)
            .collect();
        ids.into_iter().filter_map(|id| open.remove(&id).map(|socket| (id, socket))).collect()
    };
    for (id, socket) in &closing {
        ledger::ended(*id, End::Banned);
        let _ = socket.shutdown(Shutdown::Both);
    }
    if !closing.is_empty() {
        scribe::info(Channel::Network, &format!("The access lists changed: {} connection(s) banned at the door.",
                                                closing.len()));
    }
    closing.len()
}

/// Where the TCP side is listening, if it is.
pub fn listening_on() -> Option<SocketAddr> {
    let guard = TCP.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.as_ref().map(|side| side.address)
}

// ---------------------------------------------------------------------------
// The acceptor
// ---------------------------------------------------------------------------

/// The acceptor's thread.  Each connection goes on the ledger and then
/// the queue, unless its address is turned away by the access lists,
/// failed a login a moment ago, or finds the queue full, which the
/// ledger says instead.
fn accept(listener: TcpListener, queue: SyncSender<Arrival>, stopping: Arc<AtomicBool>, open: OpenSockets) {
    // Set once we've said the queue is full, so a flood gets one Warn and
    // not one per connection.  The same for a failing accept(): one Warn,
    // and again only after it has worked in between.
    let mut said_full = false;
    let mut said_failed = false;

    loop {
        match listener.accept() {
            Ok((socket, peer)) => {
                if stopping.load(Ordering::SeqCst) {
                    return;
                }
                said_failed = false;
                let id = ledger::arrived(peer);
                // Rust note: `continue` drops `socket` here, and a
                // dropped socket is a closed one.  That's the whole cost
                // of a turned-away connection.
                match access::verdict(peer.ip()) {
                    Verdict::Allowed => {}
                    Verdict::Blacklisted => {
                        ledger::ended(id, End::Blacklisted);
                        scribe::debug(Channel::Network, &format!("{peer} is on the blacklist.  Closed at the door."));
                        continue;
                    }
                    Verdict::NotWhitelisted => {
                        ledger::ended(id, End::NotWhitelisted);
                        scribe::debug(Channel::Network, &format!("{peer} isn't on the whitelist.  Closed at the \
                            door."));
                        continue;
                    }
                }
                if let Some(left) = hold_remaining(peer.ip()) {
                    ledger::ended(id, End::Held);
                    scribe::debug(Channel::Network, &format!("{peer} failed a login less than {} seconds ago.  \
                        Closed at the door; {} ms of the hold left.", FAILURE_HOLD.as_secs(), left.as_millis()));
                    continue;
                }
                if open_from(&open, peer.ip()) >= MOST_OPEN_PER_ADDRESS {
                    ledger::ended(id, End::TooManyFromOne);
                    scribe::debug(Channel::Network, &format!("{peer} has {MOST_OPEN_PER_ADDRESS} connections at the \
                        door already.  Closed at the door."));
                    continue;
                }
                // The clone that stop() and kick() shut.  Without one the
                // connection is still served; it just can't be kicked.
                match socket.try_clone() {
                    Ok(clone) => {
                        open.lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .insert(id, clone);
                    }
                    Err(e) => scribe::debug(Channel::Network, &format!("Couldn't clone {peer}'s socket: {e}.  It \
                        can't be kicked or woken.")),
                }
                match queue.try_send(Arrival { id, socket, peer, arrived: Instant::now() }) {
                    Ok(()) => {
                        if said_full {
                            said_full = false;
                            scribe::debug(Channel::Network, "The login queue has room again.");
                        }
                        scribe::debug(Channel::Network, &format!("Connection from {peer}."));
                    }
                    Err(TrySendError::Full(_)) => {
                        ledger::ended(id, End::TurnedAway);
                        open.lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .remove(&id);
                        if !said_full {
                            said_full = true;
                            scribe::warn(Channel::Network, "The login queue is full.  New connections are being \
                                closed at the door until it drains.  max_waiting_logins in networking.cfg is the \
                                size.");
                        }
                    }
                    Err(TrySendError::Disconnected(_)) => return,
                }
            }
            Err(e) => {
                if stopping.load(Ordering::SeqCst) {
                    return;
                }
                // Said once: a failure that keeps happening (out of file
                // handles, say) would otherwise be ten notices a second
                // on the bell, and the sleep keeps it off the log too.
                if !said_failed {
                    said_failed = true;
                    scribe::warn(Channel::Network, &format!("TCP accept failed: {e}.  Said once; the next line \
                        about it is when it works again."));
                } else {
                    scribe::debug(Channel::Network, &format!("TCP accept failed again: {e}."));
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

/// How many connections from `address` are at the door right now, queued
/// or being served.  A walk of the open list, which is at most the queue
/// plus the login threads.
fn open_from(open: &OpenSockets, address: IpAddr) -> usize {
    open.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .values()
        .filter(|socket| socket.peer_addr().is_ok_and(|peer| peer.ip() == address))
        .count()
}

// ---------------------------------------------------------------------------
// The login threads
// ---------------------------------------------------------------------------

/// One login thread: takes connections off the queue until the queue
/// closes.  A connection taken while we're stopping, or that waited past
/// the login deadline, is closed unserved.
fn work(arrivals: Arc<Mutex<Receiver<Arrival>>>, setup: Arc<Setup>, stopping: Arc<AtomicBool>, open: OpenSockets) {
    loop {
        // Rust note: the receiver is behind a lock because a channel has
        // one receiving end and there are several of us.  Whoever holds
        // the lock sleeps in recv(); the rest sleep on the lock.  Either
        // way nobody spins.
        let next = {
            let guard = arrivals.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.recv()
        };
        let Ok(arrival) = next else {
            return;
        };
        // Off the open list when this pass ends, whichever way.
        let _open = Open::new(&open, arrival.id);
        if stopping.load(Ordering::SeqCst) {
            ledger::ended(arrival.id, End::Stopped);
            continue;
        }
        if arrival.arrived.elapsed() >= setup.login_deadline {
            ledger::ended(arrival.id, End::Unserved);
            scribe::debug(Channel::Network, &format!("{} waited in the login queue past the deadline.  Closed \
                unserved.", arrival.peer));
            continue;
        }
        // Kicked while it waited: its socket is already shut.
        if ledger::is_done(arrival.id) {
            continue;
        }
        serve(arrival, &setup, &stopping);
    }
}

/// One connection, from the first byte to the last: TLS, then the login,
/// then goodbye.  The deadline runs from when the connection arrived.
/// However it ends, the ledger is told.
fn serve(arrival: Arrival, setup: &Setup, stopping: &AtomicBool) {
    let Arrival { id, socket, peer, arrived } = arrival;
    let deadline = arrived + setup.login_deadline;

    // Small messages go out straight away, instead of being held back to
    // be sent together.
    let _ = socket.set_nodelay(true);
    if socket.set_write_timeout(Some(WRITE_WAIT)).is_err() {
        ledger::ended(id, End::HungUp);
        return;
    }

    ledger::set(id, Stage::Handshake);
    let mut stream = match handshake(socket, peer, setup, deadline.min(arrived + HANDSHAKE_WAIT)) {
        Ok(stream) => stream,
        Err(end) => {
            ledger::ended(id, end);
            return;
        }
    };
    let end = talk(&mut stream, id, peer, setup, deadline, stopping);
    ledger::ended(id, end);
    goodbye(&mut stream);
}

/// Keeps a connection on the open list for as long as it's held.
// Rust note: `Drop` is code that runs when a value goes away, here at the
// end of a pass through work()'s loop, however it ends.  So there's no
// way out that leaves a closed socket on the list.
struct Open<'a> {
    id: u64,
    list: &'a Mutex<HashMap<u64, TcpStream>>,
}

impl<'a> Open<'a> {
    fn new(list: &'a Mutex<HashMap<u64, TcpStream>>, id: u64) -> Open<'a> {
        Open { id, list }
    }
}

impl Drop for Open<'_> {
    fn drop(&mut self) {
        self.list.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.id);
    }
}

/// The TLS handshake, inside the deadline.  An `Err` is how it didn't
/// happen, for the ledger; why is in the log.
fn handshake(mut socket: TcpStream, peer: SocketAddr, setup: &Setup, deadline: Instant) -> Result<TlsStream, End> {
    let started = Instant::now();
    let mut conn = match ServerConnection::new(Arc::clone(&setup.tls)) {
        Ok(conn) => conn,
        Err(e) => {
            scribe::warn(Channel::Network, &format!("Couldn't start TLS for {peer}: {e}.  Closing it."));
            return Err(End::HungUp);
        }
    };

    // Rust note: complete_io() does whatever reading and writing the
    // handshake needs next.  The read is armed with the time left, so a
    // WouldBlock only means the time is up, and the next arm_read() says
    // so.
    while conn.is_handshaking() {
        if !arm_read(&socket, deadline) {
            scribe::debug(Channel::Network, &format!("{peer} didn't finish TLS in {} seconds.  Closing it.",
                                                     HANDSHAKE_WAIT.as_secs()));
            return Err(End::TimedOut);
        }
        match conn.complete_io(&mut socket) {
            Ok(_) => {}
            Err(e) if timed_out(&e) => {}
            Err(e) => {
                scribe::debug(Channel::Network, &format!("TLS with {peer} failed: {e}."));
                return Err(End::HungUp);
            }
        }
    }

    scribe::debug(Channel::Network, &format!("TLS with {peer} is up, in {} ms.", started.elapsed().as_millis()));
    Ok(StreamOwned::new(conn, socket))
}

/// Sets the socket's read timeout to what's left before `deadline`, so the
/// read sleeps exactly that long and no longer.  False if nothing is left.
fn arm_read(socket: &TcpStream, deadline: Instant) -> bool {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        return false;
    }
    socket.set_read_timeout(Some(left)).is_ok()
}

/// Everything after TLS: Hello, the Login, the answer.  Hands back how it
/// ended, for the ledger.
fn talk(stream: &mut TlsStream, id: u64, peer: SocketAddr, setup: &Setup, deadline: Instant,
        stopping: &AtomicBool) -> End {
    if send(stream, &protocol::hello()).is_err() {
        return End::HungUp;
    }
    ledger::set(id, Stage::AwaitingLogin);

    // Bytes that have arrived and aren't a whole packet yet.
    let mut incoming: Vec<u8> = Vec::with_capacity(READ_CHUNK);

    let packet = match read_packet(stream, &mut incoming, deadline) {
        Ok(packet) => packet,
        Err(Trouble::Timeout) => {
            scribe::debug(Channel::Network, &format!("{peer} didn't log in within {} seconds.  Closing it.",
                                                     setup.login_deadline.as_secs()));
            return End::TimedOut;
        }
        Err(Trouble::HungUp) => {
            scribe::debug(Channel::Network, &format!("{peer} hung up before logging in."));
            return End::HungUp;
        }
        Err(Trouble::Bad(why)) => {
            scribe::debug(Channel::Network, &format!("{peer} sent {why}.  Closing it."));
            refuse(stream, peer);
            return End::Junk;
        }
        Err(Trouble::Io(e)) => {
            scribe::debug(Channel::Network, &format!("Lost {peer}: {e}."));
            return End::HungUp;
        }
    };
    if packet.kind != PacketType::Login as u8 {
        scribe::debug(Channel::Network, &format!("{peer} sent packet type 0x{:02X} instead of a Login.  Closing \
            it.", packet.kind));
        refuse(stream, peer);
        return End::Junk;
    }
    let login = match protocol::read_login(&packet.payload) {
        Ok(login) => login,
        Err(why) => {
            scribe::debug(Channel::Network, &format!("{peer} sent a Login with {why}.  Closing it."));
            refuse(stream, peer);
            return End::Junk;
        }
    };

    // The clock starts the moment the login is in, and every path below
    // runs out the same floor before anything goes back.
    let arrived = Instant::now();
    let outcome = log_in(stream, id, peer, setup, &login, stopping);
    security::pad_login_time(arrived);

    let account = match outcome {
        Outcome::In(account) => account,
        Outcome::Refused => {
            refuse(stream, peer);
            return End::Refused;
        }
        Outcome::Outdated => {
            let _ = send(stream, &protocol::login_result(LoginAnswer::Outdated));
            return End::Outdated;
        }
        Outcome::Unavailable => {
            let _ = send(stream, &protocol::login_result(LoginAnswer::Unavailable));
            return End::Unavailable;
        }
        Outcome::Gone => {
            return if stopping.load(Ordering::SeqCst) { End::Stopped } else { End::HungUp };
        }
    };

    // The password was right.  Is the account already in the world?  If
    // so the client is asked what to do, and a yes logs the other session
    // out.  The ticket is only issued if nobody is in the world on the
    // account at that moment: a player who came in between the look and
    // the issue (another login on the account during the wait for the
    // kicked session's save, say) sends it round again, and the client
    // isn't asked twice: it already said to log the other out.
    let mut chose_to_log_out = false;
    for _ in 0..ISSUE_TRIES {
        if let Some(elsewhere) = sessions::playing(&account) {
            if !chose_to_log_out {
                if let Err(end) = ask_about_the_other(stream, id, peer, &account, elsewhere, &mut incoming) {
                    return end;
                }
                chose_to_log_out = true;
            }
            if let Err(end) = log_the_other_out(stream, peer, &account, stopping) {
                return end;
            }
        }
        match sessions::issue(&account, id) {
            Ok(Issued::Ticket(token)) => {
                scribe::info(Channel::Security, &format!("{peer} logged in as {account} and has a ticket for UDP."));
                let _ = send(stream, &protocol::ticket(&token, setup.udp_port));
                return End::LoggedIn;
            }
            Ok(Issued::Playing(elsewhere)) => {
                scribe::debug(Channel::Security, &format!("{account} came into the world from {elsewhere} while \
                    {peer} was logging in.  Round again."));
            }
            Err(e) => {
                scribe::error(Channel::Security, &format!("NO TICKET FOR {account}: Fingerprinter couldn't make a \
                    token ({e}).  Nobody can get past the login until the OS gives random bytes again."));
                let _ = send(stream, &protocol::login_result(LoginAnswer::Unavailable));
                return End::Unavailable;
            }
        }
    }
    scribe::warn(Channel::Security, &format!("{account} kept coming into the world from somewhere else while {peer} \
        was logging in, {ISSUE_TRIES} times over.  {peer} gets Login Unavailable."));
    let _ = send(stream, &protocol::login_result(LoginAnswer::Unavailable));
    End::Unavailable
}

/// The account is in the world from `elsewhere`: tells the client so and
/// reads its SessionChoice.  `Ok` means log the other session out; an
/// `Err` is how the login ended instead (the client hung up, chose to
/// leave the other session alone, sent junk, or ran out of time).
fn ask_about_the_other(stream: &mut TlsStream, id: u64, peer: SocketAddr, account: &str, elsewhere: SocketAddr,
                       incoming: &mut Vec<u8>) -> Result<(), End> {
    scribe::info(Channel::Security, &format!("{account} is already in the world from {elsewhere}.  Asking {peer} \
        what to do."));
    if send(stream, &protocol::login_result(LoginAnswer::AlreadyLoggedIn)).is_err() {
        return Err(End::HungUp);
    }
    ledger::set(id, Stage::Asked);
    let packet = match read_packet(stream, incoming, Instant::now() + CHOICE_DEADLINE) {
        Ok(packet) => packet,
        Err(trouble) => {
            scribe::debug(Channel::Network, &format!("{peer} didn't say what to do about the other session.  \
                Closing it; the other session stands."));
            return Err(match trouble {
                Trouble::Timeout => End::TimedOut,
                Trouble::Bad(_) => End::Junk,
                Trouble::HungUp | Trouble::Io(_) => End::HungUp,
            });
        }
    };
    if packet.kind != PacketType::SessionChoice as u8 {
        scribe::debug(Channel::Network, &format!("{peer} sent packet type 0x{:02X} instead of a SessionChoice.  \
            Closing it; the other session stands.", packet.kind));
        return Err(End::Junk);
    }
    match protocol::read_session_choice(&packet.payload) {
        Ok(Choice::LogTheOtherOut) => Ok(()),
        Ok(Choice::HangUp) => {
            scribe::info(Channel::Security, &format!("{peer} left the other session on {account} alone and hung \
                up."));
            Err(End::LeftAlone)
        }
        Err(why) => {
            scribe::debug(Channel::Network, &format!("{peer} sent {why}.  Closing it; the other session stands."));
            Err(End::Junk)
        }
    }
}

/// Logs the account's other session out, for a client that chose to, and
/// waits for its character's save to land before the ticket goes out.  An
/// `Err` is how the login ended instead: the server stopping, or the save
/// taking too long.
fn log_the_other_out(stream: &mut TlsStream, peer: SocketAddr, account: &str, stopping: &AtomicBool)
                     -> Result<(), End> {
    let kicked = sessions::kick(account, peer);
    if let Some((address, _)) = kicked {
        udp::tell(address, &protocol::kicked(KickReason::LoggedInElsewhere));
    }
    scribe::info(Channel::Security, &format!("{peer} logged the other session on {account} out."));
    // Its character has been asked out of the world.  Its save lands
    // before this login's ticket goes out.
    if let Some((_, Some(character_id))) = kicked {
        if !wait_for_save(character_id, stopping) {
            if stopping.load(Ordering::SeqCst) {
                return Err(End::Stopped);
            }
            scribe::warn(Channel::Game, &format!("{account}'s character still didn't have its save in the database \
                {} s after {peer} logged the other session out.  {peer} gets Login Unavailable rather than come in \
                on the save before.", OLD_SAVE_WAIT.as_secs()));
            let _ = send(stream, &protocol::login_result(LoginAnswer::Unavailable));
            return Err(End::Unavailable);
        }
    }
    Ok(())
}

/// How a login ended.
enum Outcome {
    /// The password was right.  The account, lowercase.
    In(String),
    /// Wrong secret word, name or password.  One answer for all three.
    Refused,
    /// The client's version isn't on the list.
    Outdated,
    /// Archivist or Security isn't there to check it.
    Unavailable,
    /// The client went away, or the server is stopping.  Nothing to say.
    Gone,
}

/// Checks a Login: the version, the secret word, the name, then the
/// password through Archivist and Security.  It doesn't pad the time; the
/// caller does that, on every path.
fn log_in(stream: &mut TlsStream, id: u64, peer: SocketAddr, setup: &Setup, login: &LoginRequest,
          stopping: &AtomicBool) -> Outcome {
    if !setup.client_versions.iter().any(|version| *version == login.client_version) {
        // `{:?}` puts the version in quotes with anything odd in it
        // escaped, so one line stays one line whatever the client sent.
        scribe::info(Channel::Network, &format!("Turned away client version {:?} from {peer}.", login.client_version));
        return Outcome::Outdated;
    }
    if login.secret_word != setup.secret_word {
        // What they sent instead isn't logged.  It could be anything.
        scribe::info(Channel::Security, &format!("{peer} didn't know the secret word."));
        return Outcome::Refused;
    }

    // A name that couldn't be an account fails without a hash.  The name
    // rule is no secret (it's in the schema), so a quick answer here gives
    // nothing away.  The name itself isn't logged: it could be a password
    // typed into the wrong box.
    let account = login.username.to_ascii_lowercase();
    if !conductor_accounts::username_allowed(&account) {
        scribe::info(Channel::Security, &format!("Login from {peer} as a name that isn't allowed failed."));
        return Outcome::Refused;
    }

    // The same for a password that isn't a key.  The client sends the
    // password's key, never the password (protocol version 7), and a key
    // is 64 of 0-9 and a-f.  That's no secret either.  What was sent
    // isn't logged: it could be somebody's password.
    if !security::looks_like_key(&login.key) {
        scribe::info(Channel::Security, &format!("Login from {peer} as {account} didn't send a key, and failed."));
        return Outcome::Refused;
    }

    // The account's row, waited for on this thread.  Archivist's worker
    // does the reading; we only sleep until it's done.
    ledger::set(id, Stage::Checking);
    let stored: Option<String> = match conductor_accounts::password_hash(&account).wait() {
        Ok(stored) => stored,
        Err(e) => {
            // Archivist has already said what's wrong with it, and once.
            scribe::info(Channel::Security, &format!("Login from {peer} as {account} couldn't be checked: {e}."));
            return Outcome::Unavailable;
        }
    };

    // Into Security's line, one way or the other, and the same wait.
    let verified = match &stored {
        Some(hash) => wait_in_line(stream, id, security::verify_password(&login.key, hash), stopping),
        None => wait_in_line(stream, id, security::verify_no_account(&login.key), stopping)
            .map(|answer| answer.map(|()| false)),
    };

    match verified {
        None => Outcome::Gone,
        Some(Ok(true)) => Outcome::In(account),
        Some(Ok(false)) => {
            scribe::info(Channel::Security, &format!("Login from {peer} as {account} failed."));
            Outcome::Refused
        }
        Some(Err(SecurityError::NotRunning)) => {
            scribe::info(Channel::Security, &format!("Login from {peer} as {account} couldn't be checked: Security \
                isn't running."));
            Outcome::Unavailable
        }
        // A stored line Security can't read.  It has logged the damaged
        // row; to the player it's a wrong password.
        Some(Err(SecurityError::Failed(_))) => Outcome::Refused,
    }
}

/// Waits for Security's answer, a second at a time, telling the client
/// (and the ledger) its place in the line in between.  `None` if the
/// client went away or the server is stopping before the answer came.
fn wait_in_line<T>(stream: &mut TlsStream, id: u64, ticket: Ticket<T>, stopping: &AtomicBool)
                   -> Option<Result<T, SecurityError>> {
    loop {
        if let Some(answer) = ticket.wait_for(PLACE_EVERY) {
            return Some(answer);
        }
        if stopping.load(Ordering::SeqCst) {
            return None;
        }
        let place = ticket.place();
        ledger::set(id, Stage::InLine { ahead: place.ahead, wait: place.wait });
        if send(stream, &protocol::in_line(place.ahead, place.wait.as_millis())).is_err() {
            return None;
        }
    }
}

/// Waits up to OLD_SAVE_WAIT for a character this login logged out to
/// have its save in the database, a second at a time so a STOP SERVER is
/// heard.  True once it has; false if the time ran out or the server is
/// stopping.  The GameClock rings when a save lands, so this wakes then,
/// not at the end of a second.
fn wait_for_save(character_id: i64, stopping: &AtomicBool) -> bool {
    let until = Instant::now() + OLD_SAVE_WAIT;
    loop {
        let left = until.saturating_duration_since(Instant::now());
        if conductor_gameclock::wait_until_saved(character_id, left.min(PLACE_EVERY)) {
            return true;
        }
        if left <= PLACE_EVERY || stopping.load(Ordering::SeqCst) {
            return false;
        }
    }
}

/// Sends the one failure answer and puts the address on hold.  Every way
/// out of a failed login comes through here.
fn refuse(stream: &mut TlsStream, peer: SocketAddr) {
    record_failure(peer.ip());
    let _ = send(stream, &protocol::login_result(LoginAnswer::Failed));
}

// ---------------------------------------------------------------------------
// Reading and writing
// ---------------------------------------------------------------------------

/// Why a read didn't hand back a packet.
enum Trouble {
    /// The deadline passed.
    Timeout,
    /// The client closed the connection.
    HungUp,
    /// The client sent a frame we won't take.  The words say what.
    Bad(String),
    /// The connection broke.
    Io(io::Error),
}

/// Reads until one whole packet is in, or `deadline`.  One read can hold
/// part of a packet or more than one; whatever is left over stays in
/// `incoming` for the next call.
fn read_packet(stream: &mut TlsStream, incoming: &mut Vec<u8>, deadline: Instant) -> Result<Packet, Trouble> {
    let mut chunk = [0u8; READ_CHUNK];
    loop {
        if let Some(packet) = protocol::take_packet(incoming).map_err(Trouble::Bad)? {
            return Ok(packet);
        }
        if !arm_read(&stream.sock, deadline) {
            return Err(Trouble::Timeout);
        }
        match stream.read(&mut chunk) {
            Ok(0) => return Err(Trouble::HungUp),
            Ok(count) => incoming.extend_from_slice(&chunk[..count]),
            // The time is up, or near enough that arm_read() will say so.
            Err(e) if timed_out(&e) => continue,
            Err(e) => return Err(Trouble::Io(e)),
        }
    }
}

/// Sends one packet's bytes and pushes them out onto the wire.
fn send(stream: &mut TlsStream, bytes: &[u8]) -> io::Result<()> {
    stream.write_all(bytes)?;
    stream.flush()
}

/// Says goodbye properly (TLS's close_notify), so the client knows we
/// closed on purpose and nothing was cut off.  The socket itself closes
/// when the stream goes away, straight after.
fn goodbye(stream: &mut TlsStream) {
    stream.conn.send_close_notify();
    while stream.conn.wants_write() {
        if stream.conn.write_tls(&mut stream.sock).is_err() {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// The hold
// ---------------------------------------------------------------------------

/// Notes that an address just failed a login.
fn record_failure(address: IpAddr) {
    let mut failures = RECENT_FAILURES.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // Anything older than the hold is no use any more.  Clearing it out
    // here keeps the list from growing for as long as the server runs.
    failures.retain(|_, failed_at| failed_at.elapsed() < FAILURE_HOLD);
    failures.insert(address, Instant::now());
}

/// How much longer an address has to wait, or `None` if it doesn't.
fn hold_remaining(address: IpAddr) -> Option<Duration> {
    let failures = RECENT_FAILURES.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let failed_at = failures.get(&address)?;
    // Rust note: `checked_sub` is `None` when the hold is already over,
    // instead of going below zero.
    FAILURE_HOLD.checked_sub(failed_at.elapsed())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// The login itself needs Archivist, Security and a client, so it's tested
// by hand with the Python client.  What can run on its own does.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_address_is_held_and_then_let_go() {
        let address: IpAddr = "203.0.113.7".parse().unwrap();
        assert_eq!(hold_remaining(address), None);

        record_failure(address);
        let left = hold_remaining(address).expect("the address should be on hold");
        assert!(left <= FAILURE_HOLD);

        // An old failure clears out when a new one comes in.
        let other: IpAddr = "203.0.113.8".parse().unwrap();
        {
            let mut failures = RECENT_FAILURES.lock().unwrap();
            failures.insert(address, Instant::now() - FAILURE_HOLD * 2);
        }
        record_failure(other);
        assert_eq!(hold_remaining(address), None);
        assert!(hold_remaining(other).is_some());
    }

    #[test]
    fn arming_a_read_refuses_a_passed_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let socket = TcpStream::connect(listener.local_addr().unwrap()).unwrap();

        assert!(arm_read(&socket, Instant::now() + Duration::from_secs(5)));
        assert_eq!(socket.read_timeout().unwrap().map(|t| t <= Duration::from_secs(5)), Some(true));
        assert!(!arm_read(&socket, Instant::now() - Duration::from_millis(1)));
    }

    #[test]
    fn stop_with_nothing_started_does_nothing() {
        stop();
        assert_eq!(listening_on(), None);
        assert_eq!(kick(1), Kicked::NotListening);
    }
}
