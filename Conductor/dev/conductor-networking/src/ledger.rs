//! File:       Opus/Conductor/dev/conductor-networking/src/ledger.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The door's ledger: every connection that reached the TCP listener in
//! the last five minutes, and where each one is.  The acceptor writes a
//! connection in as it arrives, the login thread moves it along a stage
//! at a time (TLS, waiting for its Login, in Security's line with its
//! place, asked about another session), and whichever way it ends, that's
//! written in too.  The web admin's TCP tab is drawn from `snapshot()`.
//!
//! A connection is known here by its address and nothing else.  No
//! account name goes on the ledger, even once the login is done: this
//! tab is about the door, and who came through it is in the log.
//! Jacob's rule, 2026-09-29.
//!
//! The ledger is touched a handful of times per login (once per stage,
//! and once a second while a login waits in Security's line), never per
//! byte.  One lock, held for a few instructions.  Entries are numbered as
//! they arrive and kept in that order, so "how many are ahead of me in
//! the queue" is a count of the queued entries with smaller numbers, and
//! the sweep drops from the front.  A finished connection stays for
//! REMEMBER_FOR after it arrived, then goes; one still in progress stays
//! whatever the clock says.  A flood is capped at MOST_KEPT, the oldest
//! finished ones making room.
//!
//! The work is done by functions on a `Ledger` handed to them, so the
//! tests run on ledgers of their own and never touch the real one.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use conductor_tools::clock::Utc;

use crate::dns;

/// How long a connection stays on the ledger after it arrived, once it's
/// finished.  Five minutes, Jacob's number.
pub const REMEMBER_FOR: Duration = Duration::from_secs(5 * 60);

/// The most connections the ledger holds at once.  Past this, the oldest
/// finished ones go early.  One in progress is never dropped.
const MOST_KEPT: usize = 1000;

/// Where a connection is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// In the acceptor's queue, waiting for a login thread.
    Queued,
    /// TLS is coming up.
    Handshake,
    /// Hello is sent; waiting for the client's Login.
    AwaitingLogin,
    /// The Login is in and the account row is being read.
    Checking,
    /// In Security's line: how many jobs are ahead, and about how long.
    InLine { ahead: u64, wait: Duration },
    /// The password was right and the account is already in the world;
    /// the client is being asked what to do about that.
    Asked,
    /// Finished, one way or another.
    Done(End),
}

/// How a connection ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// Logged in and handed a ticket for UDP.
    LoggedIn,
    /// Wrong secret word, name or password.
    Refused,
    /// The client's version isn't on the list.
    Outdated,
    /// Archivist or Security wasn't there to check the login.
    Unavailable,
    /// The client closed the connection, or it broke, before the end.
    HungUp,
    /// Asked about the other session on the account, and chose to hang
    /// up and leave it alone.
    LeftAlone,
    /// The client sent something that isn't the protocol.
    Junk,
    /// A deadline passed.
    TimedOut,
    /// Closed at the door: the address failed a login a moment ago.
    Held,
    /// Closed at the door: the queue was full.
    TurnedAway,
    /// Waited in the queue past the login deadline; closed unserved.
    Unserved,
    /// The server was stopping.
    Stopped,
    /// The admin kicked it from the TCP tab.
    Kicked,
}

impl Stage {
    /// One word for the page to switch on.
    pub fn word(&self) -> &'static str {
        match self {
            Stage::Queued => "queued",
            Stage::Handshake => "handshake",
            Stage::AwaitingLogin => "login",
            Stage::Checking => "checking",
            Stage::InLine { .. } => "in_line",
            Stage::Asked => "asked",
            Stage::Done(_) => "done",
        }
    }

    /// The stage in words, for the page.  Never an account name.
    pub fn describe(&self) -> String {
        match self {
            Stage::Queued => "Waiting for a login thread".to_string(),
            Stage::Handshake => "TLS coming up".to_string(),
            Stage::AwaitingLogin => "Waiting for its Login".to_string(),
            Stage::Checking => "Checking the login".to_string(),
            Stage::InLine { ahead, wait } => {
                format!("In Security's line: {ahead} ahead, about {} ms", wait.as_millis())
            }
            Stage::Asked => "Asked what to do about the account's other session".to_string(),
            Stage::Done(end) => end.describe().to_string(),
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self, Stage::Done(_))
    }
}

impl End {
    /// The ending in words, for the page.
    pub fn describe(&self) -> &'static str {
        match self {
            End::LoggedIn => "Logged in and handed a ticket for UDP",
            End::Refused => "Refused: wrong secret word, name or password",
            End::Outdated => "Turned away: the client version isn't on the list",
            End::Unavailable => "Couldn't be checked: Archivist or Security wasn't there",
            End::HungUp => "Hung up, or the connection broke",
            End::LeftAlone => "Left the account's other session alone and hung up",
            End::Junk => "Sent something that isn't the protocol",
            End::TimedOut => "Ran out of time",
            End::Held => "Closed at the door: on hold after a failed login",
            End::TurnedAway => "Closed at the door: the login queue was full",
            End::Unserved => "Waited in the queue past the login deadline; closed unserved",
            End::Stopped => "The server was stopping",
            End::Kicked => "Kicked by the admin",
        }
    }
}

/// One connection as the web admin sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// Its number on the ledger, which a kick names.
    pub id: u64,
    pub address: SocketAddr,
    /// The address's name from a reverse DNS lookup, once the lookup is
    /// back and if it has one.
    pub host: Option<String>,
    /// When it arrived, UTC.
    pub arrived: Utc,
    /// How long ago that was.
    pub ago: Duration,
    pub stage: Stage,
    /// For a queued connection, how many arrived before it and are still
    /// waiting too.  0 for anything else.
    pub queued_ahead: usize,
}

/// One line of the ledger.
struct Entry {
    address: SocketAddr,
    /// For the sweep and the "ago".
    arrived_at: Instant,
    /// For the page.
    arrived: Utc,
    stage: Stage,
}

/// The whole ledger.  Entries are numbered as they arrive, and a BTreeMap
/// keeps them in that order.
struct Ledger {
    entries: BTreeMap<u64, Entry>,
    next: u64,
}

impl Ledger {
    fn new() -> Ledger {
        Ledger { entries: BTreeMap::new(), next: 1 }
    }
}

static LEDGER: LazyLock<Mutex<Ledger>> = LazyLock::new(|| Mutex::new(Ledger::new()));

fn ledger() -> std::sync::MutexGuard<'static, Ledger> {
    LEDGER.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Wipes the ledger.  Called on START SERVER and STOP SERVER, so a run
/// starts with an empty tab.
pub fn clear() {
    *ledger() = Ledger::new();
}

/// A connection just reached the listener.  Its number, for the stage
/// changes to come.  Also asks for the address's name, which comes back
/// on its own time.
pub fn arrived(address: SocketAddr) -> u64 {
    let now = Instant::now();
    let id = arrive_in(&mut ledger(), address, now, Utc::now());
    dns::ask(address.ip());
    id
}

/// Connection `id` moved on to `stage`.
pub fn set(id: u64, stage: Stage) {
    set_in(&mut ledger(), id, stage);
}

/// Connection `id` is finished.  The first ending written wins: a
/// connection the admin kicked stays "kicked" when its login thread
/// finds the socket closed a moment later and says "hung up".
pub fn ended(id: u64, end: End) {
    set(id, Stage::Done(end));
}

/// Whether connection `id` is finished.  True for one the ledger has
/// forgotten too: there's nothing left to do for it either way.
pub fn is_done(id: u64) -> bool {
    ledger().entries.get(&id).is_none_or(|entry| entry.stage.is_done())
}

/// Every connection on the ledger, newest first, with the names DNS has
/// found so far.
pub fn snapshot() -> Vec<Connection> {
    let now = Instant::now();
    let mut connections = snapshot_in(&mut ledger(), now);
    for connection in &mut connections {
        connection.host = dns::name_of(connection.address.ip());
    }
    connections
}

// ---------------------------------------------------------------------------
// The work, on whatever ledger is handed in
// ---------------------------------------------------------------------------

fn arrive_in(ledger: &mut Ledger, address: SocketAddr, now: Instant, stamp: Utc) -> u64 {
    sweep_in(ledger, now);
    let id = ledger.next;
    ledger.next += 1;
    ledger.entries.insert(id, Entry { address, arrived_at: now, arrived: stamp, stage: Stage::Queued });
    id
}

fn set_in(ledger: &mut Ledger, id: u64, stage: Stage) {
    // An entry the sweep or a clear() already took is nothing to update,
    // and one that's finished stays finished the way it first did.
    if let Some(entry) = ledger.entries.get_mut(&id) {
        if !entry.stage.is_done() {
            entry.stage = stage;
        }
    }
}

/// Drops finished entries older than REMEMBER_FOR, then the oldest
/// finished ones past MOST_KEPT.
fn sweep_in(ledger: &mut Ledger, now: Instant) {
    ledger.entries.retain(|_, entry| {
        !(entry.stage.is_done() && now.duration_since(entry.arrived_at) >= REMEMBER_FOR)
    });
    if ledger.entries.len() > MOST_KEPT {
        let over = ledger.entries.len() - MOST_KEPT;
        let oldest_done: Vec<u64> = ledger.entries.iter()
            .filter(|(_, entry)| entry.stage.is_done())
            .map(|(id, _)| *id)
            .take(over)
            .collect();
        for id in oldest_done {
            ledger.entries.remove(&id);
        }
    }
}

fn snapshot_in(ledger: &mut Ledger, now: Instant) -> Vec<Connection> {
    sweep_in(ledger, now);
    let mut queued_so_far = 0;
    let mut connections: Vec<Connection> = ledger.entries.iter()
        .map(|(id, entry)| {
            let queued_ahead = if entry.stage == Stage::Queued {
                queued_so_far += 1;
                queued_so_far - 1
            } else {
                0
            };
            Connection {
                id: *id,
                address: entry.address,
                host: None,
                arrived: entry.arrived,
                ago: now.saturating_duration_since(entry.arrived_at),
                stage: entry.stage,
                queued_ahead,
            }
        })
        .collect();
    connections.reverse();
    connections
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn address(text: &str) -> SocketAddr {
        text.parse().unwrap()
    }

    fn stamp() -> Utc {
        Utc::from_unix(1_790_000_000)
    }

    #[test]
    fn a_connection_moves_through_its_stages_and_is_listed_newest_first() {
        let mut ledger = Ledger::new();
        let start = Instant::now();
        let first = arrive_in(&mut ledger, address("10.0.0.5:50000"), start, stamp());
        let second = arrive_in(&mut ledger, address("10.0.0.6:50001"), start + Duration::from_secs(1), stamp());

        set_in(&mut ledger, first, Stage::Handshake);
        set_in(&mut ledger, first, Stage::InLine { ahead: 2, wait: Duration::from_millis(300) });

        let listed = snapshot_in(&mut ledger, start + Duration::from_secs(5));
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, second);
        assert_eq!(listed[0].address, address("10.0.0.6:50001"));
        assert_eq!(listed[0].stage, Stage::Queued);
        assert_eq!(listed[0].ago, Duration::from_secs(4));
        assert_eq!(listed[1].address, address("10.0.0.5:50000"));
        assert_eq!(listed[1].stage, Stage::InLine { ahead: 2, wait: Duration::from_millis(300) });
        assert_eq!(listed[1].stage.describe(), "In Security's line: 2 ahead, about 300 ms");
        assert!(listed.iter().all(|connection| connection.host.is_none()));

        set_in(&mut ledger, second, Stage::Done(End::Kicked));
        // A number the ledger doesn't know is nothing to update, and the
        // first ending wins over a later one.
        set_in(&mut ledger, 99, Stage::Asked);
        set_in(&mut ledger, second, Stage::Done(End::HungUp));
        let listed = snapshot_in(&mut ledger, start + Duration::from_secs(6));
        assert_eq!(listed[0].stage, Stage::Done(End::Kicked));
        assert!(listed[0].stage.is_done());
        assert_eq!(listed[0].stage.word(), "done");
    }

    #[test]
    fn queued_connections_know_how_many_are_ahead() {
        let mut ledger = Ledger::new();
        let now = Instant::now();
        let first = arrive_in(&mut ledger, address("10.0.0.1:1"), now, stamp());
        arrive_in(&mut ledger, address("10.0.0.2:2"), now, stamp());
        arrive_in(&mut ledger, address("10.0.0.3:3"), now, stamp());

        // Newest first, so the last to arrive has two ahead of it.
        let listed = snapshot_in(&mut ledger, now);
        assert_eq!(listed.iter().map(|c| c.queued_ahead).collect::<Vec<_>>(), vec![2, 1, 0]);

        // The first is taken by a login thread: the other two move up.
        set_in(&mut ledger, first, Stage::Handshake);
        let listed = snapshot_in(&mut ledger, now);
        assert_eq!(listed.iter().map(|c| c.queued_ahead).collect::<Vec<_>>(), vec![1, 0, 0]);
    }

    #[test]
    fn the_sweep_forgets_finished_connections_after_five_minutes() {
        let mut ledger = Ledger::new();
        let start = Instant::now();
        let done = arrive_in(&mut ledger, address("10.0.0.1:1"), start, stamp());
        let stuck = arrive_in(&mut ledger, address("10.0.0.2:2"), start, stamp());
        set_in(&mut ledger, done, Stage::Done(End::LoggedIn));
        set_in(&mut ledger, stuck, Stage::AwaitingLogin);

        // Four minutes on: both still there.
        assert_eq!(snapshot_in(&mut ledger, start + Duration::from_secs(240)).len(), 2);

        // Five minutes on: the finished one is gone, the one in progress
        // stays whatever the clock says.
        let listed = snapshot_in(&mut ledger, start + REMEMBER_FOR);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].address, address("10.0.0.2:2"));
        assert_eq!(snapshot_in(&mut ledger, start + REMEMBER_FOR * 10).len(), 1);
    }

    #[test]
    fn a_flood_drops_the_oldest_finished_ones_first() {
        let mut ledger = Ledger::new();
        let now = Instant::now();
        let live = arrive_in(&mut ledger, address("10.0.0.1:1"), now, stamp());
        for n in 0..MOST_KEPT as u64 {
            let id = arrive_in(&mut ledger, address("10.0.0.2:2"), now, stamp());
            set_in(&mut ledger, id, Stage::Done(End::TurnedAway));
            // The cap holds on the way in, one over at most.
            assert!(ledger.entries.len() <= MOST_KEPT + 1, "at {n}");
        }
        sweep_in(&mut ledger, now);
        assert_eq!(ledger.entries.len(), MOST_KEPT);
        // The one in progress was the oldest of all, and it's still there.
        assert!(ledger.entries.contains_key(&live));
    }

    #[test]
    fn every_ending_has_words() {
        let ends = [End::LoggedIn, End::Refused, End::Outdated, End::Unavailable, End::HungUp, End::LeftAlone,
                    End::Junk, End::TimedOut, End::Held, End::TurnedAway, End::Unserved, End::Stopped,
                    End::Kicked];
        for end in ends {
            assert!(!end.describe().is_empty());
            assert_eq!(Stage::Done(end).describe(), end.describe());
        }
    }
}
