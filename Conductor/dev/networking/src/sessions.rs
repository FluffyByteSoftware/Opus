//! File:       Opus/Conductor/dev/networking/src/sessions.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The book of who is where: tickets handed out and not yet used, and
//! players in the world over UDP.  The TCP side writes tickets in it, the
//! UDP side turns them into players and crosses players out.
//!
//! A ticket is a token from Fingerprinter, good once and for a short
//! while.  A Connect with it from some address makes that address a
//! player, and the ticket is gone; the same address asking again is a
//! client whose answer got lost, and gets the same answer.  Any other
//! address with that token is refused.  A player is known by their
//! address from then on, and stays until they say Goodbye, go quiet for
//! the UDP timeout, get logged out from somewhere else, or the server
//! stops.  Whichever way, they're gone completely, and the client starts
//! over at the login screen.  Nothing is kept for a reconnect, by Jacob's
//! rule for this iteration.
//!
//! An account is in the book at most once as a player.  A login for an
//! account that's already playing is told so, and the new client says
//! whether to log the old one out (`kick()`).  An account with an unused
//! ticket and another login gets a new ticket, and the old one dies.
//!
//! Three maps, so nothing is ever found by a search: tickets by token,
//! players by address, and each account's address or ticket by name.
//! That's a little more memory per player for a lookup that costs the
//! same with 5 players or 5000.  One lock around the lot, held for a few
//! instructions at a time.  In memory only: a restart forgets every one
//! of them, on purpose.
//!
//! Every ticket and player carries the number of the door's ledger row it
//! came from.  Whenever one leaves (or a ticket dies unused), the row is
//! marked LINKDEAD with how, so the Connections tab stops showing it green
//! as if it were live.  The work notes the rows in `Book::gone`, and the
//! public functions hand them to the ledger once the book's lock is let
//! go, so the two locks are never held at once.
//!
//! Each ticket and each player has its account's name, and only the
//! name.  An account is never held in memory: whatever needs one reads
//! it from the database when it needs it (conductor-accounts), so the web
//! admin can change an account while its player is in the world and
//! nothing writes an old copy over it.  Jacob's rule, 2026-09-29.  The one
//! write networking makes is the login time, stamped straight in the row
//! the moment a ticket is used and the player is in the world over UDP
//! (`connect()`), not at the TLS login before it: that's when playing
//! starts, and it's what playtime will be counted from.
//!
//! A player at character select asks Protogame for things (their
//! characters, a new one), each ask with a number.  The book keeps the ask
//! being worked on and the last one answered, with its answer, so a
//! client that didn't hear back and asks again gets the same answer, not
//! the ask done twice; and a player has one ask at a time, so nobody can
//! queue up a pile of database jobs.  The answer goes with the player
//! when they leave.
//!
//! A player picks a character with a UserPressPlay (protocol version 6),
//! and Protogame brings it into the world and writes it on the player
//! here (`entered()`).  From then on the player is in the world, and
//! character select's asks are refused: there's no way back to it but
//! logging out (Jacob: "you log out back to log in screen every time").
//! Every way a player leaves the book goes through `remove_player_in()`,
//! so a character in the world is always taken out with its player: the
//! book notes it, and `with_book()` asks the GameClock to save it and take
//! it out once the lock is let go.
//!
//! A character that just left can't be picked again for a second
//! (`LEAVING_LOCKOUT`), so a quick second login can't read its row before
//! its leaving save lands.  Jacob's fix, 2026-10-01: "a lockout on a
//! character being instantiated for like 1 second".
//!
//! The work is done by functions on a `Book` handed to them, so the tests
//! run on books of their own and never touch the real one.

use std::collections::HashMap;
use std::io;
use std::mem;
use std::net::SocketAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use conductor_tools::clock::Utc;
use conductor_tools::fingerprinter;
use conductor_tools::scribe::{self, Channel};

use crate::ledger::{self, Gone};

/// How long a character that left the world can't be picked again, so its
/// leaving save lands before anybody reads its row.  The GameClock takes
/// the note within a cycle (250 ms) and Archivist writes it after; a
/// second covers both with room to spare.  Fixed in code: a setting
/// would only be a way to get it wrong.
pub const LEAVING_LOCKOUT: Duration = Duration::from_secs(1);

/// A ticket handed out and not yet used.
struct Ticket {
    /// The account's name, lowercase.
    account: String,
    issued: Instant,
    /// The door's ledger row of the login that got it.
    door: u64,
}

/// A player in the world.
struct Player {
    /// The account's name, lowercase.
    account: String,
    /// The token they came in with, so a repeat Connect can be told from
    /// somebody else at the same address.
    token: String,
    last_heard: Instant,
    /// When their Connect was accepted, both ways: for "how long" and
    /// for the page.
    connected_at: Instant,
    connected: Utc,
    /// The door's ledger row of the login they came in on.
    door: u64,
    /// The ask Protogame is working on for them, if any.
    asking: Option<u32>,
    /// The last ask answered, and the answer, to send again if the same
    /// ask comes in again.
    answered: Option<(u32, Vec<u8>)>,
    /// Their character in the world, once they've picked one.  `None`
    /// at character select.
    character: Option<InWorld>,
}

/// A player's character in the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InWorld {
    /// Its row's id in `player_characters`, which is how the GameClock
    /// knows it.
    pub id: i64,
    pub uuid: String,
    pub name: String,
}

/// Where an account is right now.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Whereabouts {
    /// Has a ticket, not yet on UDP.
    Ticket(String),
    /// In the world, from this address.
    Playing(SocketAddr),
}

/// The whole book.
struct Book {
    tickets: HashMap<String, Ticket>,
    players: HashMap<SocketAddr, Player>,
    accounts: HashMap<String, Whereabouts>,
    /// Ledger rows to mark LINKDEAD, and why, from the last change to the
    /// book.  Emptied by `with_book()` each time.
    gone: Vec<(u64, Gone)>,
    /// Characters to take out of the world, by their row's id, from the
    /// last change to the book.  Emptied by `with_book()` each time.
    leaving: Vec<i64>,
    /// Characters that left the world, by uuid, and when: the lockout.
    /// The sweep forgets them once it's over.
    left: HashMap<String, Instant>,
}

impl Book {
    fn new() -> Book {
        Book { tickets: HashMap::new(), players: HashMap::new(), accounts: HashMap::new(), gone: Vec::new(),
               leaving: Vec::new(), left: HashMap::new() }
    }
}

// Rust note: a HashMap can't be built before the program starts the way
// a Mutex around a `None` can.  LazyLock builds it the first time anybody
// touches it.
static BOOK: LazyLock<Mutex<Book>> = LazyLock::new(|| Mutex::new(Book::new()));

/// What a Connect got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Connected {
    /// The ticket was good.  The account is now a player at this address.
    Accepted(String),
    /// This address is already this player, asking again because our
    /// answer got lost.
    Again(String),
    /// No such ticket, or it's somebody else's now.
    Refused,
}

/// One player as the web admin's Connections tab sees them.  The account
/// is on it, unlike the door's ledger: this is the world, and who is in
/// it is the point.  Jacob's spec for the UDP list, 2026-09-29.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerView {
    pub address: SocketAddr,
    pub account: String,
    /// The character they're playing.  `None` while they're at character
    /// select.
    pub character: Option<String>,
    /// When their Connect was accepted, UTC.
    pub connected: Utc,
    /// How long they've been in.
    pub playing_for: Duration,
    /// How long since their last packet.
    pub quiet_for: Duration,
}

/// What the book says to an ask from `from` (`begin_ask()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// Nobody plays from there.  No answer.
    Stranger,
    /// Their last ask is still being worked on.  This one is dropped; the
    /// client asks again if it still wants it.
    Busy,
    /// The ask answered last time, asked again: the answer it got, to
    /// send again as it is.
    Again(Vec<u8>),
    /// A new ask, now theirs to be worked on.  The player's account.
    New(String),
    /// A new ask from a player whose character is in the world, where
    /// character select is behind them.  Theirs to be answered with a
    /// refusal (`finish_ask()`), like any other.  The player's account.
    InWorld(String),
}

/// What the admin's kick of a login's row found in the book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminKick {
    /// The login's player, taken out of the world: their address, so the
    /// UDP side can tell them, and their account, for the log.
    Player(SocketAddr, String),
    /// The login's ticket, never used, now dead.  The account, for the log.
    Ticket(String),
    /// Nothing from that login is left in the book.
    Nobody,
}

/// What deleting an account found in the book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminated {
    /// Its player, taken out of the world: their address, so the UDP side
    /// can tell them.
    Player(SocketAddr),
    /// Its ticket, never used, now dead.
    Ticket,
    /// The account wasn't in the book.
    Nobody,
}

/// What a sweep crossed out.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Swept {
    /// Players who went quiet: their address and account.
    pub quiet: Vec<(SocketAddr, String)>,
    /// Tickets that ran out unused.
    pub tickets_expired: usize,
}

fn book() -> std::sync::MutexGuard<'static, Book> {
    BOOK.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Does `work` on the book, then marks the ledger rows it left LINKDEAD,
/// and asks the GameClock to take out the characters it left, with the
/// book's lock let go first.
// Rust note: `impl FnOnce(&mut Book) -> T` takes any bit of code that
// works on the book once and hands back a T, so every public function
// below can share the same lock-then-mark steps.
fn with_book<T>(work: impl FnOnce(&mut Book) -> T) -> T {
    let (answer, gone, leaving) = {
        let mut book = book();
        // Rust note: `&mut *book` is the book itself, out of the lock's
        // guard, which is the type `work` wants.
        let answer = work(&mut *book);
        (answer, mem::take(&mut book.gone), mem::take(&mut book.leaving))
    };
    for (door, how) in gone {
        ledger::linkdead(door, how);
    }
    for character_id in leaving {
        take_out_of_the_world(character_id);
    }
    answer
}

/// Asks the GameClock to save a character and take it out of the world.
/// It comes straight back.  The GameClock stops after networking does,
/// so it's there to ask; if it somehow isn't, its last world save on the
/// way down has the character anyway.
fn take_out_of_the_world(character_id: i64) {
    if let Err(why) = conductor_gameclock::leave(character_id) {
        scribe::debug(Channel::Game, &format!("Character {character_id} couldn't be asked out of the world: \
            {why}."));
    }
}

/// The address an account is playing from, if it's in the world.
pub fn playing(account: &str) -> Option<SocketAddr> {
    match book().accounts.get(account) {
        Some(Whereabouts::Playing(address)) => Some(*address),
        _ => None,
    }
}

/// A new ticket for `account`, which just logged in on ledger row `door`.
/// Any older unused ticket for the account dies, and its row goes
/// LINKDEAD.  Fails only if Fingerprinter can't make a token, which means
/// the OS won't give random bytes.
pub fn issue(account: &str, door: u64) -> io::Result<String> {
    let token = fingerprinter::new_token()?;
    with_book(|book| issue_in(book, account, &token, door, Instant::now()));
    Ok(token)
}

/// A Connect from `from` with `token`.  A ticket used for the first time
/// stamps its account's login time in the database, once the book's lock
/// is let go; nobody waits on the write, and Archivist logs one that
/// fails.
pub fn connect(token: &str, from: SocketAddr) -> Connected {
    let connected = with_book(|book| connect_in(book, token, from, Instant::now()));
    if let Connected::Accepted(account) = &connected {
        let _ = conductor_accounts::stamp_login(account);
    }
    connected
}

/// A player at `from` sent something.  True if there is one; false for a
/// stranger, who gets no answer to anything.
pub fn heard(from: SocketAddr) -> bool {
    match book().players.get_mut(&from) {
        Some(player) => {
            player.last_heard = Instant::now();
            true
        }
        None => false,
    }
}

/// An ask numbered `ask` from `from`.  It counts as hearing from them.
pub fn begin_ask(from: SocketAddr, ask: u32) -> Ask {
    begin_ask_in(&mut book(), from, ask, Instant::now())
}

/// The answer to the ask `begin_ask()` said was new, kept for a repeat.
/// True if it should go out: the same account is still at `from`, with
/// that ask.  False if they've left, or somebody else is there now.
pub fn finish_ask(from: SocketAddr, account: &str, ask: u32, answer: &[u8]) -> bool {
    finish_ask_in(&mut book(), from, account, ask, answer)
}

/// A player's character came into the world: Protogame put it there,
/// for the ask `ask`, and `answer` is the CharacterEnteredWorld to keep
/// for a repeat.  True if it's written on the player and the answer should
/// go out.  False if the player at `from` has left (or somebody else is
/// there now), and then the caller takes the character back out
/// (`leave_world()`), since there's nobody left to play it.
pub fn entered(from: SocketAddr, account: &str, ask: u32, character: InWorld, answer: &[u8]) -> bool {
    entered_in(&mut book(), from, account, ask, character, answer)
}

/// Takes a character out of the world that no player in the book holds:
/// the one whose player left while it was being brought in.  The lockout
/// starts for it all the same.
pub fn leave_world(character_id: i64, uuid: &str) {
    with_book(|book| {
        book.leaving.push(character_id);
        book.left.insert(uuid.to_string(), Instant::now());
    });
}

/// True while the character `uuid` is in its lockout: it left the world
/// less than LEAVING_LOCKOUT ago.
pub fn just_left(uuid: &str) -> bool {
    just_left_in(&book(), uuid, Instant::now())
}

/// Sends the player at `from` back to the login: they picked a character
/// still in its lockout.  True if they were there to send back; the
/// caller tells them with a Kicked.
pub fn turn_away(from: SocketAddr, account: &str) -> bool {
    with_book(|book| {
        if book.players.get(&from).is_some_and(|player| player.account == account) {
            remove_player_in(book, from, Gone::CharacterLeaving).is_some()
        } else {
            false
        }
    })
}

/// The player at `from` said Goodbye.  Their account, if there was one.
pub fn leave(from: SocketAddr) -> Option<String> {
    with_book(|book| remove_player_in(book, from, Gone::SaidGoodbye))
}

/// An account's player is being logged out by a second login from `by`.
/// Their address, so the UDP side can tell them, and any unused ticket the
/// account had dies too.
pub fn kick(account: &str, by: SocketAddr) -> Option<SocketAddr> {
    with_book(|book| kick_in(book, account, by))
}

/// The admin deleted `account` from the web admin's Accounts tab: its
/// player out of the world, or its unused ticket.  Which, with the
/// player's address so the UDP side can tell them.
pub fn terminate(account: &str) -> Terminated {
    with_book(|book| terminate_in(book, account))
}

/// The admin kicked the login on ledger row `door` from the Connections
/// tab: its player out of the world, or its ticket if it hasn't come over
/// UDP yet.  Either way the row goes LINKDEAD.
pub fn kick_login(door: u64) -> AdminKick {
    with_book(|book| kick_login_in(book, door))
}

/// Crosses out every player quiet for `udp_timeout` and every ticket
/// older than `token_deadline`.
pub fn sweep(udp_timeout: Duration, token_deadline: Duration) -> Swept {
    with_book(|book| sweep_in(book, Instant::now(), udp_timeout, token_deadline))
}

/// Everyone out: the server is stopping.  The players' addresses and
/// accounts, so each can be told.  No row is marked: STOP SERVER wipes the
/// ledger anyway.  An account is never held, so there's nothing of it to
/// save; their characters are asked out of the world, and saved on the
/// way.
pub fn clear() -> Vec<(SocketAddr, String)> {
    let old = mem::replace(&mut *book(), Book::new());
    let mut gone = Vec::with_capacity(old.players.len());
    for (address, player) in old.players {
        if let Some(character) = player.character {
            take_out_of_the_world(character.id);
        }
        gone.push((address, player.account));
    }
    gone
}

/// How many players and how many unused tickets, for the status.
pub fn counts() -> (usize, usize) {
    let book = book();
    (book.players.len(), book.tickets.len())
}

/// A copy of every player in the world, newest first, for the web
/// admin's Connections tab.
pub fn players() -> Vec<PlayerView> {
    players_in(&book(), Instant::now())
}

/// Crosses out every player whose address `matches` says so for: a ban
/// from the web admin.  Nothing is sent from here; the caller tells each
/// one with a Kicked.  Their addresses and accounts, for that and the log.
pub fn drop_where(matches: impl Fn(SocketAddr) -> bool) -> Vec<(SocketAddr, String)> {
    with_book(|book| drop_where_in(book, matches))
}

// ---------------------------------------------------------------------------
// The work, on whatever book is handed in
// ---------------------------------------------------------------------------

fn issue_in(book: &mut Book, account: &str, token: &str, door: u64, now: Instant) {
    let name = account.to_string();
    // An older ticket for the same account is no good any more.  A player
    // in the world stays: the caller asked `playing()` and dealt with that
    // (a kick, or the new login hung up) before issuing.
    if let Some(Whereabouts::Ticket(old)) = book.accounts.get(&name).cloned() {
        remove_ticket_in(book, &old, Gone::TicketTaken);
    }
    book.tickets.insert(token.to_string(), Ticket { account: name.clone(), issued: now, door });
    book.accounts.insert(name, Whereabouts::Ticket(token.to_string()));
}

fn connect_in(book: &mut Book, token: &str, from: SocketAddr, now: Instant) -> Connected {
    if let Some(player) = book.players.get_mut(&from) {
        if player.token == token {
            player.last_heard = now;
            return Connected::Again(player.account.clone());
        }
        // A known address with a different token.  Not a repeat, and not
        // a ticket anybody can use from here.
        return Connected::Refused;
    }

    let Some(ticket) = book.tickets.remove(token) else {
        return Connected::Refused;
    };
    let door = ticket.door;
    let name = ticket.account;

    // The book says the account holds this ticket.  If it somehow says
    // the account is playing from elsewhere too, that entry is stale,
    // and the fresh ticket wins.
    if let Some(Whereabouts::Playing(elsewhere)) = book.accounts.get(&name).cloned() {
        remove_player_in(book, elsewhere, Gone::Replaced { by: from });
    }
    book.players.insert(from, Player { account: name.clone(), token: token.to_string(), last_heard: now,
                                       connected_at: now, connected: Utc::now(), door, asking: None,
                                       answered: None, character: None });
    book.accounts.insert(name.clone(), Whereabouts::Playing(from));
    Connected::Accepted(name)
}

fn begin_ask_in(book: &mut Book, from: SocketAddr, ask: u32, now: Instant) -> Ask {
    let Some(player) = book.players.get_mut(&from) else {
        return Ask::Stranger;
    };
    player.last_heard = now;
    if let Some((answered, answer)) = &player.answered {
        if *answered == ask {
            return Ask::Again(answer.clone());
        }
    }
    if player.asking.is_some() {
        return Ask::Busy;
    }
    player.asking = Some(ask);
    if player.character.is_some() {
        return Ask::InWorld(player.account.clone());
    }
    Ask::New(player.account.clone())
}

fn finish_ask_in(book: &mut Book, from: SocketAddr, account: &str, ask: u32, answer: &[u8]) -> bool {
    let Some(player) = book.players.get_mut(&from) else {
        return false;
    };
    if player.account != account || player.asking != Some(ask) {
        return false;
    }
    player.asking = None;
    player.answered = Some((ask, answer.to_vec()));
    true
}

fn entered_in(book: &mut Book, from: SocketAddr, account: &str, ask: u32, character: InWorld, answer: &[u8])
    -> bool {
    let Some(player) = book.players.get_mut(&from) else {
        return false;
    };
    if player.account != account || player.asking != Some(ask) {
        return false;
    }
    player.asking = None;
    player.answered = Some((ask, answer.to_vec()));
    player.character = Some(character);
    true
}

fn just_left_in(book: &Book, uuid: &str, now: Instant) -> bool {
    // A uuid comes out of the database in lowercase, and a client could
    // send it back in capitals and still find the row.
    match book.left.get(&uuid.to_ascii_lowercase()) {
        Some(&when) => now.saturating_duration_since(when) < LEAVING_LOCKOUT,
        None => false,
    }
}

fn players_in(book: &Book, now: Instant) -> Vec<PlayerView> {
    let mut players: Vec<PlayerView> = book.players.iter().map(|(address, player)| PlayerView {
        address: *address,
        account: player.account.clone(),
        character: player.character.as_ref().map(|character| character.name.clone()),
        connected: player.connected,
        playing_for: now.saturating_duration_since(player.connected_at),
        quiet_for: now.saturating_duration_since(player.last_heard),
    }).collect();
    // Newest first: the shortest time in the world at the top.
    players.sort_by_key(|player| player.playing_for);
    players
}

fn drop_where_in(book: &mut Book, matches: impl Fn(SocketAddr) -> bool) -> Vec<(SocketAddr, String)> {
    let banned: Vec<SocketAddr> = book.players.keys().copied().filter(|address| matches(*address)).collect();
    banned.into_iter()
        .filter_map(|address| remove_player_in(book, address, Gone::Banned).map(|account| (address, account)))
        .collect()
}

/// Takes the player at `from` out of the world and notes their row as
/// LINKDEAD for `how`.  Their character, if they had one in the world,
/// is noted to be taken out too, and its lockout starts.  Their account's
/// name.
fn remove_player_in(book: &mut Book, from: SocketAddr, how: Gone) -> Option<String> {
    let player = book.players.remove(&from)?;
    if let Some(character) = player.character {
        book.leaving.push(character.id);
        book.left.insert(character.uuid, Instant::now());
    }
    let name = player.account;
    if book.accounts.get(&name) == Some(&Whereabouts::Playing(from)) {
        book.accounts.remove(&name);
    }
    book.gone.push((player.door, how));
    Some(name)
}

/// Takes the ticket `token` out, the same way: its row LINKDEAD for `how`.
/// Its account's name.
fn remove_ticket_in(book: &mut Book, token: &str, how: Gone) -> Option<String> {
    let ticket = book.tickets.remove(token)?;
    let name = ticket.account;
    if book.accounts.get(&name) == Some(&Whereabouts::Ticket(token.to_string())) {
        book.accounts.remove(&name);
    }
    book.gone.push((ticket.door, how));
    Some(name)
}

fn kick_in(book: &mut Book, account: &str, by: SocketAddr) -> Option<SocketAddr> {
    match book.accounts.get(account).cloned() {
        Some(Whereabouts::Playing(address)) => {
            remove_player_in(book, address, Gone::Replaced { by });
            Some(address)
        }
        Some(Whereabouts::Ticket(token)) => {
            remove_ticket_in(book, &token, Gone::TicketTaken);
            None
        }
        None => None,
    }
}

fn terminate_in(book: &mut Book, account: &str) -> Terminated {
    match book.accounts.get(account).cloned() {
        Some(Whereabouts::Playing(address)) => {
            remove_player_in(book, address, Gone::Terminated);
            Terminated::Player(address)
        }
        Some(Whereabouts::Ticket(token)) => {
            remove_ticket_in(book, &token, Gone::Terminated);
            Terminated::Ticket
        }
        None => Terminated::Nobody,
    }
}

fn kick_login_in(book: &mut Book, door: u64) -> AdminKick {
    // Found by a walk rather than a map: it's an admin's click, not a
    // packet, so it doesn't have to be quick.
    let player = book.players.iter().find(|(_, player)| player.door == door).map(|(address, _)| *address);
    if let Some(address) = player {
        if let Some(account) = remove_player_in(book, address, Gone::KickedByAdmin) {
            return AdminKick::Player(address, account);
        }
    }
    let ticket = book.tickets.iter().find(|(_, ticket)| ticket.door == door).map(|(token, _)| token.clone());
    if let Some(token) = ticket {
        if let Some(account) = remove_ticket_in(book, &token, Gone::KickedByAdmin) {
            return AdminKick::Ticket(account);
        }
    }
    AdminKick::Nobody
}

fn sweep_in(book: &mut Book, now: Instant, udp_timeout: Duration, token_deadline: Duration) -> Swept {
    let mut swept = Swept::default();

    // Found first and taken out after, since nothing can come out of a
    // map while it's being walked.
    let quiet: Vec<SocketAddr> = book.players.iter()
        .filter(|(_, player)| now.duration_since(player.last_heard) >= udp_timeout)
        .map(|(address, _)| *address)
        .collect();
    for address in quiet {
        if let Some(account) = remove_player_in(book, address, Gone::WentQuiet) {
            swept.quiet.push((address, account));
        }
    }

    let expired: Vec<String> = book.tickets.iter()
        .filter(|(_, ticket)| now.duration_since(ticket.issued) >= token_deadline)
        .map(|(token, _)| token.clone())
        .collect();
    for token in expired {
        if remove_ticket_in(book, &token, Gone::TicketRanOut).is_some() {
            swept.tickets_expired += 1;
        }
    }

    // Lockouts that are over are nothing to keep.
    book.left.retain(|_, when| now.saturating_duration_since(*when) < LEAVING_LOCKOUT);

    swept
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


    #[test]
    fn a_ticket_connects_once_from_one_address() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        let elsewhere = address("10.0.0.6:50000");
        issue_in(&mut book, "jacob", "abc", 1, now);

        assert_eq!(connect_in(&mut book, "abc", home, now), Connected::Accepted("jacob".to_string()));
        // The answer got lost and the client asked again.
        assert_eq!(connect_in(&mut book, "abc", home, now), Connected::Again("jacob".to_string()));
        // Somebody else with a copy of the token.
        assert_eq!(connect_in(&mut book, "abc", elsewhere, now), Connected::Refused);
        // The same address with some other token.
        assert_eq!(connect_in(&mut book, "abd", home, now), Connected::Refused);

        assert_eq!(book.accounts.get("jacob"), Some(&Whereabouts::Playing(home)));
        assert!(book.tickets.is_empty());
    }

    #[test]
    fn an_ask_is_worked_once_and_its_answer_kept() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        let stranger = address("10.0.0.9:50000");
        issue_in(&mut book, "jacob", "abc", 1, now);
        connect_in(&mut book, "abc", home, now);

        assert_eq!(begin_ask_in(&mut book, stranger, 1, now), Ask::Stranger);
        assert_eq!(begin_ask_in(&mut book, home, 1, now), Ask::New("jacob".to_string()));
        // Still being worked on: the same ask again, or another, waits.
        assert_eq!(begin_ask_in(&mut book, home, 1, now), Ask::Busy);
        assert_eq!(begin_ask_in(&mut book, home, 2, now), Ask::Busy);

        // Only the ask being worked on, for the account that asked, is
        // answered.
        assert!(!finish_ask_in(&mut book, home, "jacob", 2, b"two"));
        assert!(!finish_ask_in(&mut book, home, "someone", 1, b"one"));
        assert!(!finish_ask_in(&mut book, stranger, "jacob", 1, b"one"));
        assert!(finish_ask_in(&mut book, home, "jacob", 1, b"one"));

        // Asked again, it gets the kept answer; a new number is new.
        assert_eq!(begin_ask_in(&mut book, home, 1, now), Ask::Again(b"one".to_vec()));
        assert_eq!(begin_ask_in(&mut book, home, 2, now), Ask::New("jacob".to_string()));
        assert!(finish_ask_in(&mut book, home, "jacob", 2, b"two"));
        assert_eq!(begin_ask_in(&mut book, home, 1, now), Ask::New("jacob".to_string()));
    }

    /// Jacob's character, as Protogame would write it on the player.
    fn jacob() -> InWorld {
        InWorld { id: 42, uuid: "0199aaaa-0000-7000-8000-000000000001".to_string(), name: "Jacob".to_string() }
    }

    #[test]
    fn a_character_entering_is_written_only_on_the_player_who_asked() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        issue_in(&mut book, "jacob", "abc", 1, now);
        connect_in(&mut book, "abc", home, now);

        // Not the ask being worked on, not the account, nobody there.
        assert!(!entered_in(&mut book, home, "jacob", 3, jacob(), b"in"));
        assert_eq!(begin_ask_in(&mut book, home, 3, now), Ask::New("jacob".to_string()));
        assert!(!entered_in(&mut book, home, "brother", 3, jacob(), b"in"));
        assert!(!entered_in(&mut book, address("10.0.0.9:50000"), "jacob", 3, jacob(), b"in"));
        assert!(entered_in(&mut book, home, "jacob", 3, jacob(), b"in"));
        assert_eq!(players_in(&book, now)[0].character, Some("Jacob".to_string()));

        // The answer is kept like any other, for a lost one.
        assert_eq!(begin_ask_in(&mut book, home, 3, now), Ask::Again(b"in".to_vec()));
    }

    #[test]
    fn character_select_is_behind_a_player_in_the_world() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        issue_in(&mut book, "jacob", "abc", 1, now);
        connect_in(&mut book, "abc", home, now);
        begin_ask_in(&mut book, home, 1, now);
        assert!(entered_in(&mut book, home, "jacob", 1, jacob(), b"in"));

        // A new ask is theirs to be refused, and the refusal is kept.
        assert_eq!(begin_ask_in(&mut book, home, 2, now), Ask::InWorld("jacob".to_string()));
        assert_eq!(begin_ask_in(&mut book, home, 3, now), Ask::Busy);
        assert!(finish_ask_in(&mut book, home, "jacob", 2, b"no"));
        assert_eq!(begin_ask_in(&mut book, home, 2, now), Ask::Again(b"no".to_vec()));
    }

    #[test]
    fn a_player_leaving_takes_their_character_out_and_starts_its_lockout() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        let uuid = jacob().uuid;
        issue_in(&mut book, "jacob", "abc", 1, now);
        connect_in(&mut book, "abc", home, now);
        begin_ask_in(&mut book, home, 1, now);
        assert!(entered_in(&mut book, home, "jacob", 1, jacob(), b"in"));
        assert!(!just_left_in(&book, &uuid, now), "in the world, not left");

        assert_eq!(remove_player_in(&mut book, home, Gone::SaidGoodbye), Some("jacob".to_string()));
        assert_eq!(book.leaving, vec![42]);
        let left = Instant::now();
        assert!(just_left_in(&book, &uuid, left));
        assert!(just_left_in(&book, &uuid.to_ascii_uppercase(), left), "whatever the capitals");
        assert!(!just_left_in(&book, &uuid, left + LEAVING_LOCKOUT), "a second on, it can be picked");
        assert!(!just_left_in(&book, "0199aaaa-0000-7000-8000-000000000002", left), "only that character");

        // The sweep forgets a lockout that's over, and keeps one that isn't.
        sweep_in(&mut book, left, Duration::from_secs(40), Duration::from_secs(30));
        assert_eq!(book.left.len(), 1);
        sweep_in(&mut book, left + LEAVING_LOCKOUT, Duration::from_secs(40), Duration::from_secs(30));
        assert!(book.left.is_empty());
    }

    #[test]
    fn a_player_at_character_select_leaves_nothing_in_the_world() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        issue_in(&mut book, "jacob", "abc", 1, now);
        connect_in(&mut book, "abc", home, now);
        remove_player_in(&mut book, home, Gone::WentQuiet);
        assert!(book.leaving.is_empty());
        assert!(book.left.is_empty());
    }

    #[test]
    fn a_token_nobody_was_handed_is_refused() {
        let mut book = Book::new();
        let now = Instant::now();
        assert_eq!(connect_in(&mut book, "abc", address("10.0.0.5:50000"), now), Connected::Refused);
        assert_eq!(connect_in(&mut book, "", address("10.0.0.5:50000"), now), Connected::Refused);
    }

    #[test]
    fn a_second_ticket_kills_the_first() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        issue_in(&mut book, "jacob", "first", 2, now);
        issue_in(&mut book, "jacob", "second", 3, now);

        assert_eq!(book.tickets.len(), 1);
        // The first login's row: its ticket was never used.
        assert_eq!(book.gone, vec![(2, Gone::TicketTaken)]);
        assert_eq!(connect_in(&mut book, "first", home, now), Connected::Refused);
        assert_eq!(connect_in(&mut book, "second", home, now), Connected::Accepted("jacob".to_string()));
    }

    #[test]
    fn playing_is_only_true_in_the_world() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        assert_eq!(book.accounts.get("jacob"), None);

        issue_in(&mut book, "jacob", "abc", 4, now);
        assert_eq!(book.accounts.get("jacob"), Some(&Whereabouts::Ticket("abc".to_string())));

        connect_in(&mut book, "abc", home, now);
        assert_eq!(book.accounts.get("jacob"), Some(&Whereabouts::Playing(home)));
    }

    #[test]
    fn goodbye_takes_the_player_out() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        issue_in(&mut book, "jacob", "abc", 5, now);
        connect_in(&mut book, "abc", home, now);

        assert_eq!(remove_player_in(&mut book, home, Gone::SaidGoodbye), Some("jacob".to_string()));
        assert!(book.players.is_empty());
        assert!(book.accounts.is_empty());
        // Their login's row is to be marked.
        assert_eq!(book.gone, vec![(5, Gone::SaidGoodbye)]);
        // A stranger saying goodbye is nobody.
        assert_eq!(remove_player_in(&mut book, home, Gone::SaidGoodbye), None);
    }

    #[test]
    fn a_kick_takes_out_the_player_or_the_ticket() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");

        let second_login = address("10.0.0.5:50001");

        issue_in(&mut book, "jacob", "abc", 6, now);
        assert_eq!(kick_in(&mut book, "jacob", second_login), None);
        assert!(book.tickets.is_empty() && book.accounts.is_empty());

        issue_in(&mut book, "jacob", "abd", 7, now);
        connect_in(&mut book, "abd", home, now);
        assert_eq!(kick_in(&mut book, "jacob", second_login), Some(home));
        assert!(book.players.is_empty() && book.accounts.is_empty());

        assert_eq!(kick_in(&mut book, "nobody", second_login), None);
        // The unused ticket's row, then the player's, named for the login
        // that logged them out.
        assert_eq!(book.gone, vec![(6, Gone::TicketTaken), (7, Gone::Replaced { by: second_login })]);
    }

    #[test]
    fn the_admins_kick_takes_out_a_logins_player_or_its_ticket() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        issue_in(&mut book, "jacob", "abc", 20, now);
        issue_in(&mut book, "brother", "def", 21, now);
        connect_in(&mut book, "abc", home, now);

        // Row 20's player is in the world; row 21 only has its ticket.
        assert_eq!(kick_login_in(&mut book, 20), AdminKick::Player(home, "jacob".to_string()));
        assert_eq!(kick_login_in(&mut book, 21), AdminKick::Ticket("brother".to_string()));
        assert_eq!(kick_login_in(&mut book, 20), AdminKick::Nobody);
        assert!(book.players.is_empty() && book.tickets.is_empty() && book.accounts.is_empty());
        assert_eq!(book.gone, vec![(20, Gone::KickedByAdmin), (21, Gone::KickedByAdmin)]);
    }

    #[test]
    fn the_sweep_drops_the_quiet_and_the_stale() {
        let mut book = Book::new();
        let start = Instant::now();
        let home = address("10.0.0.5:50000");
        let away = address("10.0.0.6:50000");
        let timeout = Duration::from_secs(40);
        let deadline = Duration::from_secs(30);

        issue_in(&mut book, "jacob", "abc", 8, start);
        issue_in(&mut book, "brother", "def", 9, start);
        issue_in(&mut book, "friend", "ghi", 10, start);
        connect_in(&mut book, "abc", home, start);
        connect_in(&mut book, "def", away, start);

        // Ten seconds on: nothing to sweep.
        let swept = sweep_in(&mut book, start + Duration::from_secs(10), timeout, deadline);
        assert_eq!(swept, Swept::default());

        // The brother keeps talking; jacob doesn't.
        book.players.get_mut(&away).unwrap().last_heard = start + Duration::from_secs(35);

        // Forty seconds on: jacob is quiet, the friend's ticket is stale.
        let swept = sweep_in(&mut book, start + timeout, timeout, deadline);
        assert_eq!(swept.quiet, vec![(home, "jacob".to_string())]);
        assert_eq!(swept.tickets_expired, 1);
        assert_eq!(book.gone, vec![(8, Gone::WentQuiet), (10, Gone::TicketRanOut)]);
        assert_eq!(book.players.len(), 1);
        assert!(book.tickets.is_empty());
        assert_eq!(book.accounts.len(), 1);
        assert_eq!(book.accounts.get("brother"), Some(&Whereabouts::Playing(away)));
    }

    #[test]
    fn the_players_come_out_newest_first_with_their_times() {
        let mut book = Book::new();
        let start = Instant::now();
        let home = address("10.0.0.5:50000");
        let away = address("10.0.0.6:50000");
        issue_in(&mut book, "jacob", "abc", 11, start);
        issue_in(&mut book, "brother", "def", 12, start);
        connect_in(&mut book, "abc", home, start);
        connect_in(&mut book, "def", away, start + Duration::from_secs(5));
        book.players.get_mut(&home).unwrap().last_heard = start + Duration::from_secs(8);

        let players = players_in(&book, start + Duration::from_secs(10));
        assert_eq!(players.len(), 2);
        assert_eq!(players[0].account, "brother");
        assert_eq!(players[0].address, away);
        assert_eq!(players[0].playing_for, Duration::from_secs(5));
        assert_eq!(players[0].quiet_for, Duration::from_secs(5));
        assert_eq!(players[1].account, "jacob");
        assert_eq!(players[1].playing_for, Duration::from_secs(10));
        assert_eq!(players[1].quiet_for, Duration::from_secs(2));

        assert!(players_in(&Book::new(), start).is_empty());
    }

    #[test]
    fn deleting_an_account_takes_out_its_player_or_its_ticket() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        issue_in(&mut book, "jacob", "abc", 30, now);
        issue_in(&mut book, "brother", "def", 31, now);
        connect_in(&mut book, "abc", home, now);

        assert_eq!(terminate_in(&mut book, "jacob"), Terminated::Player(home));
        assert_eq!(terminate_in(&mut book, "brother"), Terminated::Ticket);
        assert_eq!(terminate_in(&mut book, "jacob"), Terminated::Nobody);
        assert!(book.players.is_empty() && book.tickets.is_empty() && book.accounts.is_empty());
        assert_eq!(book.gone, vec![(30, Gone::Terminated), (31, Gone::Terminated)]);
    }

    #[test]
    fn a_ban_drops_the_players_it_names_and_nobody_else() {
        let mut book = Book::new();
        let now = Instant::now();
        let home = address("10.0.0.5:50000");
        let away = address("192.168.1.9:50000");
        issue_in(&mut book, "jacob", "abc", 13, now);
        issue_in(&mut book, "brother", "def", 14, now);
        connect_in(&mut book, "abc", home, now);
        connect_in(&mut book, "def", away, now);

        let dropped = drop_where_in(&mut book, |address| address.ip().to_string().starts_with("10."));
        assert_eq!(dropped, vec![(home, "jacob".to_string())]);
        assert_eq!(book.gone, vec![(13, Gone::Banned)]);
        assert_eq!(book.players.len(), 1);
        assert!(book.accounts.get("jacob").is_none());
        assert_eq!(book.accounts.get("brother"), Some(&Whereabouts::Playing(away)));

        assert!(drop_where_in(&mut book, |_| false).is_empty());
        assert_eq!(book.players.len(), 1);
    }
}
