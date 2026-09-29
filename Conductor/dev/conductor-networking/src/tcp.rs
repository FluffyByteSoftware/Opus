//! File:       Opus/Conductor/dev/conductor-networking/src/tcp.rs
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
//! secret word, the username and the password in one Login, and we answer
//! with a Ticket or a LoginResult.  An old version is told so before the
//! password is looked at, and a wrong secret word or a name that couldn't
//! be an account fails without a hash.  A name that could be one is
//! looked up through Archivist, and its password checked through
//! Security, in Security's line: a name with no account still costs a
//! hash there, so a stopwatch can't tell the two apart.  Every failure
//! gets the same answer, closes the connection, and makes the address
//! wait FAILURE_HOLD before its next connection is taken at all.  The
//! right password for an account already in the world gets asked what to
//! do instead (sessions.rs); a right password otherwise gets a ticket.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rustls::{ServerConfig, ServerConnection, StreamOwned};

use conductor_tools::scribe::{self, Channel};
use conductor_tools::security::{self, SecurityError, Ticket};
use conductor_tools::services::{self, State};
use conductor_tools::{archivist, threads};

use crate::protocol::{self, Choice, KickReason, LoginAnswer, LoginRequest, Packet, PacketType};
use crate::settings::Settings;
use crate::{sessions, timed_out, udp, wake_address};

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

/// How much we ask TLS for in one read.  Bigger than any packet we take,
/// so one read can hold a whole one.
const READ_CHUNK: usize = 8192;

/// The one row a login needs.
const ACCOUNT_SQL: &str = "SELECT password_hash FROM accounts WHERE account_username = $1";

/// Noted on the way in.  Not waited on: a login time we couldn't save
/// isn't worth turning the player away over, and Archivist logs a failed
/// job itself.
const LOGIN_TIME_SQL: &str = "UPDATE accounts SET last_login_datetime = now() WHERE account_username = $1";

/// The account name rule, the same one the accounts table checks: 8 to 32
/// of `a-z`, `0-9` and `_`.
const NAME_MIN: usize = 8;
const NAME_MAX: usize = 32;

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
    socket: TcpStream,
    peer: SocketAddr,
    arrived: Instant,
}

/// The running TCP side.  Held in TCP below while the server is started.
struct TcpSide {
    /// Set by `stop()`.  The acceptor checks it after every accept(), and
    /// the login threads after every connection and every second in line.
    stopping: Arc<AtomicBool>,
    address: SocketAddr,
    acceptor: JoinHandle<()>,
    workers: Vec<JoinHandle<()>>,
    /// The sockets the login threads are serving right now, so `stop()`
    /// can shut them down and wake the threads out of their reads.
    serving: Arc<Mutex<HashMap<u64, TcpStream>>>,
}

// Rust note: the same shape Security uses for its worker.  `None` means
// the TCP side isn't running.
static TCP: Mutex<Option<TcpSide>> = Mutex::new(None);

/// When each address last failed a login.  Only the ones inside
/// FAILURE_HOLD matter, and the rest are cleared out as new failures come
/// in, so it never grows for as long as the server runs.
static RECENT_FAILURES: LazyLock<Mutex<HashMap<IpAddr, Instant>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Numbers the connections being served, for the `serving` map.
static NEXT_SERVING: AtomicU64 = AtomicU64::new(1);

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
    let serving = Arc::new(Mutex::new(HashMap::new()));

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
        let serving = Arc::clone(&serving);
        let handle = threads::spawn(&format!("net-login-{number}"), move || work(arrivals, setup, stopping, serving))
            // If one won't start, `queue` is dropped on the way out of
            // here, and the ones already started see the queue close and
            // end.
            .map_err(|e| format!("Couldn't start login thread {number}: {e}."))?;
        workers.push(handle);
    }

    let flag = Arc::clone(&stopping);
    let acceptor = threads::spawn("net-tcp", move || accept(listener, queue, flag))
        .map_err(|e| format!("Couldn't start the TCP acceptor thread: {e}."))?;

    let note = format!("Listening on {address}.  {} login thread(s).", settings.login_threads);
    services::set(services::NETWORK_TCP, State::Running, &note);
    scribe::info(Channel::Network, &format!("TCP: {note}"));
    *guard = Some(TcpSide { stopping, address, acceptor, workers, serving });
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
    // once.
    {
        let serving = side.serving.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for socket in serving.values() {
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

    services::set(services::NETWORK_TCP, State::Stopped, "Stopped.");
    scribe::info(Channel::Network, "TCP: stopped listening.");
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

/// The acceptor's thread.  Each connection goes on the queue, unless its
/// address failed a login a moment ago or the queue is full.
fn accept(listener: TcpListener, queue: SyncSender<Arrival>, stopping: Arc<AtomicBool>) {
    // Set once we've said the queue is full, so a flood gets one Warn and
    // not one per connection.
    let mut said_full = false;

    loop {
        match listener.accept() {
            Ok((socket, peer)) => {
                if stopping.load(Ordering::SeqCst) {
                    return;
                }
                if let Some(left) = hold_remaining(peer.ip()) {
                    scribe::debug(Channel::Network, &format!("{peer} failed a login less than {} seconds ago.  \
                        Closed at the door; {} ms of the hold left.", FAILURE_HOLD.as_secs(), left.as_millis()));
                    continue;
                }
                match queue.try_send(Arrival { socket, peer, arrived: Instant::now() }) {
                    Ok(()) => {
                        said_full = false;
                        scribe::debug(Channel::Network, &format!("Connection from {peer}."));
                    }
                    Err(TrySendError::Full(_)) => {
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
                scribe::warn(Channel::Network, &format!("TCP accept failed: {e}."));
                // A failure that keeps happening (out of file handles,
                // say) would otherwise fill the log as fast as the disk
                // allows.
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The login threads
// ---------------------------------------------------------------------------

/// One login thread: takes connections off the queue until the queue
/// closes.  A connection taken while we're stopping, or that waited past
/// the login deadline, is closed unserved.
fn work(arrivals: Arc<Mutex<Receiver<Arrival>>>, setup: Arc<Setup>, stopping: Arc<AtomicBool>,
        serving: Arc<Mutex<HashMap<u64, TcpStream>>>) {
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
        if stopping.load(Ordering::SeqCst) {
            continue;
        }
        if arrival.arrived.elapsed() >= setup.login_deadline {
            scribe::debug(Channel::Network, &format!("{} waited in the login queue past the deadline.  Closed \
                unserved.", arrival.peer));
            continue;
        }
        serve(arrival, &setup, &stopping, &serving);
    }
}

/// One connection, from the first byte to the last: TLS, then the login,
/// then goodbye.  The deadline runs from when the connection arrived.
fn serve(arrival: Arrival, setup: &Setup, stopping: &AtomicBool, serving: &Arc<Mutex<HashMap<u64, TcpStream>>>) {
    let Arrival { socket, peer, arrived } = arrival;
    let deadline = arrived + setup.login_deadline;

    // A clone on the serving list, so stop() can wake us.  Off the list
    // when this function ends, whichever way.
    let clone = match socket.try_clone() {
        Ok(clone) => clone,
        Err(e) => {
            scribe::debug(Channel::Network, &format!("Couldn't clone {peer}'s socket: {e}.  Closing it."));
            return;
        }
    };
    let _on_list = Serving::new(serving, clone);

    // Small messages go out straight away, instead of being held back to
    // be sent together.
    let _ = socket.set_nodelay(true);
    if socket.set_write_timeout(Some(WRITE_WAIT)).is_err() {
        return;
    }

    let Some(mut stream) = handshake(socket, peer, setup, deadline) else {
        return;
    };
    talk(&mut stream, peer, setup, deadline, stopping);
    goodbye(&mut stream);
}

/// Keeps a socket on the serving list for as long as it's held.
// Rust note: `Drop` is code that runs when a value goes away, here at the
// end of serve(), however it ends.  So there's no way out that leaves a
// closed socket on the list.
struct Serving<'a> {
    id: u64,
    list: &'a Mutex<HashMap<u64, TcpStream>>,
}

impl<'a> Serving<'a> {
    fn new(list: &'a Mutex<HashMap<u64, TcpStream>>, socket: TcpStream) -> Serving<'a> {
        let id = NEXT_SERVING.fetch_add(1, Ordering::Relaxed);
        list.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(id, socket);
        Serving { id, list }
    }
}

impl Drop for Serving<'_> {
    fn drop(&mut self) {
        self.list.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.id);
    }
}

/// The TLS handshake, inside the deadline.  `None` means it didn't
/// happen, and why is in the log.
fn handshake(mut socket: TcpStream, peer: SocketAddr, setup: &Setup, deadline: Instant) -> Option<TlsStream> {
    let started = Instant::now();
    let mut conn = match ServerConnection::new(Arc::clone(&setup.tls)) {
        Ok(conn) => conn,
        Err(e) => {
            scribe::warn(Channel::Network, &format!("Couldn't start TLS for {peer}: {e}.  Closing it."));
            return None;
        }
    };

    // Rust note: complete_io() does whatever reading and writing the
    // handshake needs next.  The read is armed with the time left, so a
    // WouldBlock only means the time is up, and the next arm_read() says
    // so.
    while conn.is_handshaking() {
        if !arm_read(&socket, deadline) {
            scribe::debug(Channel::Network, &format!("{peer} didn't finish TLS in {} seconds.  Closing it.",
                                                     setup.login_deadline.as_secs()));
            return None;
        }
        match conn.complete_io(&mut socket) {
            Ok(_) => {}
            Err(e) if timed_out(&e) => {}
            Err(e) => {
                scribe::debug(Channel::Network, &format!("TLS with {peer} failed: {e}."));
                return None;
            }
        }
    }

    scribe::debug(Channel::Network, &format!("TLS with {peer} is up, in {} ms.", started.elapsed().as_millis()));
    Some(StreamOwned::new(conn, socket))
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

/// Everything after TLS: Hello, the Login, the answer.
fn talk(stream: &mut TlsStream, peer: SocketAddr, setup: &Setup, deadline: Instant, stopping: &AtomicBool) {
    if send(stream, &protocol::hello()).is_err() {
        return;
    }

    // Bytes that have arrived and aren't a whole packet yet.
    let mut incoming: Vec<u8> = Vec::with_capacity(READ_CHUNK);

    let packet = match read_packet(stream, &mut incoming, deadline) {
        Ok(packet) => packet,
        Err(Trouble::Timeout) => {
            scribe::debug(Channel::Network, &format!("{peer} didn't log in within {} seconds.  Closing it.",
                                                     setup.login_deadline.as_secs()));
            return;
        }
        Err(Trouble::HungUp) => {
            scribe::debug(Channel::Network, &format!("{peer} hung up before logging in."));
            return;
        }
        Err(Trouble::Bad(why)) => {
            scribe::debug(Channel::Network, &format!("{peer} sent {why}.  Closing it."));
            refuse(stream, peer);
            return;
        }
        Err(Trouble::Io(e)) => {
            scribe::debug(Channel::Network, &format!("Lost {peer}: {e}."));
            return;
        }
    };
    if packet.kind != PacketType::Login as u8 {
        scribe::debug(Channel::Network, &format!("{peer} sent packet type 0x{:02X} instead of a Login.  Closing \
            it.", packet.kind));
        refuse(stream, peer);
        return;
    }
    let login = match protocol::read_login(&packet.payload) {
        Ok(login) => login,
        Err(why) => {
            scribe::debug(Channel::Network, &format!("{peer} sent a Login with {why}.  Closing it."));
            refuse(stream, peer);
            return;
        }
    };

    // The clock starts the moment the login is in, and every path below
    // runs out the same floor before anything goes back.
    let arrived = Instant::now();
    let outcome = log_in(stream, peer, setup, &login, stopping);
    security::pad_login_time(arrived);

    let account = match outcome {
        Outcome::In(account) => account,
        Outcome::Refused => {
            refuse(stream, peer);
            return;
        }
        Outcome::Outdated => {
            let _ = send(stream, &protocol::login_result(LoginAnswer::Outdated));
            return;
        }
        Outcome::Unavailable => {
            let _ = send(stream, &protocol::login_result(LoginAnswer::Unavailable));
            return;
        }
        Outcome::Gone => return,
    };

    // The password was right.  Is the account already in the world?
    if let Some(elsewhere) = sessions::playing(&account) {
        scribe::info(Channel::Security, &format!("{account} is already in the world from {elsewhere}.  Asking \
            {peer} what to do."));
        if send(stream, &protocol::login_result(LoginAnswer::AlreadyLoggedIn)).is_err() {
            return;
        }
        let packet = match read_packet(stream, &mut incoming, Instant::now() + CHOICE_DEADLINE) {
            Ok(packet) => packet,
            Err(_) => {
                scribe::debug(Channel::Network, &format!("{peer} didn't say what to do about the other session.  \
                    Closing it; the other session stands."));
                return;
            }
        };
        if packet.kind != PacketType::SessionChoice as u8 {
            scribe::debug(Channel::Network, &format!("{peer} sent packet type 0x{:02X} instead of a \
                SessionChoice.  Closing it; the other session stands.", packet.kind));
            return;
        }
        match protocol::read_session_choice(&packet.payload) {
            Ok(Choice::LogTheOtherOut) => {
                if let Some(address) = sessions::kick(&account) {
                    udp::tell(address, &protocol::kicked(KickReason::LoggedInElsewhere));
                }
                scribe::info(Channel::Security, &format!("{peer} logged the other session on {account} out."));
            }
            Ok(Choice::HangUp) => {
                scribe::info(Channel::Security, &format!("{peer} left the other session on {account} alone and \
                    hung up."));
                return;
            }
            Err(why) => {
                scribe::debug(Channel::Network, &format!("{peer} sent {why}.  Closing it; the other session \
                    stands."));
                return;
            }
        }
    }

    match sessions::issue(&account) {
        Ok(token) => {
            scribe::info(Channel::Security, &format!("{peer} logged in as {account} and has a ticket for UDP."));
            let _ = send(stream, &protocol::ticket(&token, setup.udp_port));
        }
        Err(e) => {
            scribe::error(Channel::Security, &format!("NO TICKET FOR {account}: Fingerprinter couldn't make a token \
                ({e}).  Nobody can get past the login until the OS gives random bytes again."));
            let _ = send(stream, &protocol::login_result(LoginAnswer::Unavailable));
        }
    }
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
fn log_in(stream: &mut TlsStream, peer: SocketAddr, setup: &Setup, login: &LoginRequest, stopping: &AtomicBool)
          -> Outcome {
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
    if !name_allowed(&account) {
        scribe::info(Channel::Security, &format!("Login from {peer} as a name that isn't allowed failed."));
        return Outcome::Refused;
    }

    // The account's row, waited for on this thread.  Archivist's worker
    // does the reading; we only sleep until it's done.
    let params: Vec<archivist::Param> = vec![Box::new(account.clone())];
    let stored: Option<String> = match archivist::query(ACCOUNT_SQL, params).wait() {
        Ok(rows) => rows.first().map(|row| row.get("password_hash")),
        Err(e) => {
            // Archivist has already said what's wrong with it, and once.
            scribe::info(Channel::Security, &format!("Login from {peer} as {account} couldn't be checked: {e}."));
            return Outcome::Unavailable;
        }
    };

    // Into Security's line, one way or the other, and the same wait.
    let verified = match &stored {
        Some(hash) => wait_in_line(stream, security::verify_password(&login.password, hash), stopping),
        None => wait_in_line(stream, security::verify_no_account(&login.password), stopping)
            .map(|answer| answer.map(|()| false)),
    };

    match verified {
        None => Outcome::Gone,
        Some(Ok(true)) => {
            // Not waited on: see LOGIN_TIME_SQL.
            let params: Vec<archivist::Param> = vec![Box::new(account.clone())];
            let _ = archivist::execute(LOGIN_TIME_SQL, params);
            Outcome::In(account)
        }
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
/// its place in the line in between.  `None` if the client went away or
/// the server is stopping before the answer came.
fn wait_in_line<T>(stream: &mut TlsStream, ticket: Ticket<T>, stopping: &AtomicBool)
                   -> Option<Result<T, SecurityError>> {
    loop {
        if let Some(answer) = ticket.wait_for(PLACE_EVERY) {
            return Some(answer);
        }
        if stopping.load(Ordering::SeqCst) {
            return None;
        }
        let place = ticket.place();
        if send(stream, &protocol::in_line(place.ahead, place.wait.as_millis())).is_err() {
            return None;
        }
    }
}

/// The account name rule: 8 to 32 of `a-z`, `0-9` and `_`.  The same rule
/// the accounts table checks, so a name that passes here can be looked
/// up and one that doesn't can't be a row.
fn name_allowed(name: &str) -> bool {
    (NAME_MIN..=NAME_MAX).contains(&name.len())
        && name.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
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
    fn the_name_rule_matches_the_table() {
        assert!(name_allowed("jacob_01"));
        assert!(name_allowed("a2345678"));
        assert!(name_allowed(&"a".repeat(32)));

        assert!(!name_allowed("jacob"));
        assert!(!name_allowed(&"a".repeat(33)));
        assert!(!name_allowed("Jacob_01"));
        assert!(!name_allowed("jacob-01"));
        assert!(!name_allowed("jacob 01"));
        assert!(!name_allowed(""));
    }

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
    }
}
