//! File:       Opus/Conductor/dev/networking/src/protogame.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Protogame: what goes on between a player logging in and their
//! character being in the world.  Jacob's word, 2026-09-30: "the character
//! selection and character construction are proto game then become game
//! objects after load."  That's character select: listing an account's
//! characters, making one, deleting one, putting one back at its spawn
//! point, and the last step, playing one (protocol version 6).  A new
//! character and RESET HOME stand the character on top of the spawn
//! point's highest block, worked out by GameWorld (`conductor_gameworld::
//! spawn`, Jacob, 2026-10-03), so this thread waits on it as it waits on
//! the database.
//!
//! Playing one is the spawn's slow part, done here so the GameClock never
//! waits on it: the row and the save are read, and the save is laid over
//! the Character template.  Since protocol version 11 the finished
//! character isn't put in the world at once: it's held on the player in
//! the book (`sessions::parked()`), and the answer to PLAY is the offer of
//! the simple overworld map, which the client fetches straight from the
//! UDP thread.  Its PlayerReady, with the map's hash, comes back here, and
//! only then does the character go in the GameClock's mailbox
//! (`conductor_gameclock::enter()`) and get written on the player as in
//! the world (Jacob: "it doesn't show them or spawn them in the physical
//! world until they're ready").  So a player who left while it was being
//! brought in leaves nothing standing: their character is taken straight
//! back out, or, still loading, was never in.  Before any of it the
//! character is locked for a second (`sessions::lock_for_loading()`),
//! and one that's locked already (loading, just out of the world, or its
//! save still on its way) isn't read until the lock clears: the player is
//! told to wait (PleaseWait)
//! and the lock is waited out, up to `LOCK_WAIT`; past that they're sent
//! back to the login to try again.  Jacob's lock, 2026-10-01, so no
//! character is brought in twice at once or on the save before its last;
//! the wait is his too (2026-10-02), after a second login's PLAY inside
//! the first one's second got the Kicked.
//!
//! A character saved inside the ground (the ground changed under it, or it
//! was saved before spawn points) is moved before the offer: on top of its
//! column, or to the spawn point if that's too high, and told so in its
//! chat once it's in (Jacob, 2026-10-03, session 9).  If GameWorld can't
//! say in time, PLAY is refused and the character is moved to the spawn
//! point's last known place and saved there, for the next PLAY to check.
//!
//! A PLAY inside the account's map cooldown (`map_cooldown_seconds`,
//! counted from the last time it was sent the map's offer) is refused
//! before any of that, saying how long is left: Jacob's DDOS protection,
//! 2026-10-03, since every PLAY is the whole map again.
//!
//! Every one of those is a database job, and the UDP thread never waits on
//! the database.  So the UDP thread hands each ask in here, through a
//! mailbox, and goes back to its packets.  Protogame's one thread,
//! `protogame`, takes the asks one at a time, waits on conductor-accounts
//! for each, and sends the answer back over the UDP socket itself.  The
//! answer is kept in the book first (`sessions::finish_ask()`), so a
//! client that didn't hear it and asks again gets it again.  A player has
//! one ask at a time, so the mailbox never holds more than one per player.
//!
//! The account is the one the player logged in as, from the book, never
//! anything the client says: a player only ever sees, makes and deletes
//! characters on their own account.
//!
//! Part of networking, and it comes up and goes down with it: started
//! before the UDP side, so the first ask has somewhere to go, and stopped
//! after it, finishing whatever asks were already handed in.

use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

use conductor_accounts::characters::{self, CharacterCreated, CharacterSave, CharacterSnapshot};
use conductor_gameworld::spawn::{self, Footing, SPAWN_POINTS};
use conductor_primlib::gameobject;
use conductor_primlib::{Blueprint, Component, Kind, Save, Transform, Vector3};
use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};
use conductor_tools::threads;

use crate::overworld;
use crate::protocol::{self, CreateAnswer, DeleteAnswer, EnteredCharacter, KickReason, ListedCharacter};
use crate::sessions::{self, InWorld, Loading};
use crate::udp;

/// How long Protogame waits on the database for one ask.  Past that the
/// player is told it's unavailable (the job may still happen: a list
/// asked for again shows it), so a stuck database can't hold up every
/// player's asks behind it for good.
const DATABASE_WAIT: Duration = Duration::from_secs(10);

/// How long the thread sleeps for an ask before it checks in with the
/// services list.
const CHECK_IN_EVERY: Duration = Duration::from_secs(1);

/// How long a UserPressPlay waits for its character's lock to clear
/// before the player is sent back to the login.  The lock is a second,
/// plus the save if one's still on its way; past this the save is stuck,
/// and the login's own wait for it (`tcp.rs`) is the same 5 seconds.
/// Protogame is one thread, so another player's ask waits behind this,
/// which at these numbers is nothing.
const LOCK_WAIT: Duration = Duration::from_secs(5);

/// What the player sees on character select while the lock is waited
/// out (PleaseWait, protocol version 10).
const LOCK_WAIT_WORDS: &str = "Your character is still being saved from its last session. One moment.";

/// The word a player types to delete a character, whatever the capitals
/// (Jacob: "the client pre-reqs to ask them to type in delete").
const DELETE_WORD: &str = "DELETE";

/// What the player hears when the database can't be asked.  Short labels,
/// no period: nothing the player can act on (the protocol's rule).
const LIST_UNAVAILABLE: &str = "Character List Unavailable";
const DELETE_UNAVAILABLE: &str = "Character Deletion Unavailable";
const RESET_UNAVAILABLE: &str = "Reset Home Unavailable";
const PLAY_UNAVAILABLE: &str = "World Unavailable";

/// How long a new character or RESET HOME waits on GameWorld for the
/// spawn point's height.  It's a column of chunks read or built, so a
/// fraction of this.
const SPAWN_WAIT: Duration = Duration::from_secs(10);

/// What the player hears for a PlayerReady whose hash isn't the map's
/// (version 11).  Their client checks its copy before it sends one, so
/// this is a client that's broken or changed.
const MAP_DOES_NOT_MATCH: &str = "Your copy of the world's map doesn't match the server's.  Log in and try \
    again.";

/// What the player hears for a PLAY inside the account's map cooldown,
/// with the seconds left.  Jacob's words.
fn cooling_down_words(left: Duration) -> String {
    // Rounded up, so the last moment says 1, never 0.
    let seconds = left.as_secs() + u64::from(left.subsec_nanos() > 0);
    format!("You are temporarily cooling down from download for DDOS protection. You have {seconds} seconds \
        remaining.")
}

/// What the player hears for a character that isn't on their account,
/// and for one whose save won't load or that's been marked unplayable.
const NO_SUCH_CHARACTER: &str = "There's no such character on this account.";
const CANT_BE_LOADED: &str = "That character can't be loaded.  The admin has been told.";
const UNPLAYABLE: &str = "That character can't be played until the admin has looked at it.";

/// What a player whose character was moved out of the ground at PLAY is
/// told in their chat once it's in.  The first is Jacob's words.
const MOVED_ON_TOP: &str = "You were inside the ground, and have been moved on top of it.";
const MOVED_TO_SPAWN: &str = "You were inside the ground, and have been moved to the spawn point.";

/// What a player hears when GameWorld couldn't say whether their character
/// can stand where it was saved, and it's been moved to the spawn point.
const COULDNT_CHECK: &str = "The server couldn't check where your character stands, so it's been moved to the \
    spawn point.  Press PLAY again.";

/// What a player asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Work {
    /// Their characters: a CharacterListRequest.
    List,
    /// A new character with this name: a CreateCharacter.
    Create { name: String },
    /// Delete this character, with the word they typed: a DeleteCharacter.
    Delete { uuid: String, typed: String },
    /// Put this character back at its spawn point: a
    /// CharacterRequestResetHome.
    ResetHome { uuid: String },
    /// Load this character and offer the map: a UserPressPlay.
    Play { uuid: String },
    /// The client has the map, with this hash: put the loaded character
    /// in the world.  A PlayerReady (version 11).
    Ready { hash: String },
}

/// One ask in the mailbox.
struct Job {
    from: SocketAddr,
    account: String,
    ask: u32,
    work: Work,
}

// Rust note: `None` means Protogame isn't running.  The Sender is how
// asks get in; dropping it is how the thread is told to finish.
static MAILBOX: Mutex<Option<Sender<Job>>> = Mutex::new(None);
static WORKER: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Starts the thread, with `map_cooldown` (`map_cooldown_seconds` in
/// `networking.cfg`) as how long an account waits between being sent the
/// map.  An `Err` says what went wrong, in words.
pub fn start(map_cooldown: Duration) -> Result<(), String> {
    let mut mailbox = MAILBOX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if mailbox.is_some() {
        return Err("Protogame is already running.".to_string());
    }
    let (sender, receiver) = mpsc::channel();
    let worker = threads::spawn("protogame", move || work(receiver, map_cooldown))
        .map_err(|e| format!("Couldn't start Protogame's thread: {e}."))?;
    *mailbox = Some(sender);
    *WORKER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(worker);
    services::set(services::PROTOGAME, State::Running, "Waiting for asks from character select.");
    Ok(())
}

/// Stops taking asks, answers the ones already handed in, and waits for
/// the thread to end.  Safe to call when it isn't running.
pub fn stop() {
    // Dropping the Sender closes the mailbox: the thread answers what's in
    // it, then finds it closed and ends.
    let sender = MAILBOX.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
    let was_running = sender.is_some();
    drop(sender);
    let worker = WORKER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
    if let Some(worker) = worker {
        let _ = worker.join();
    }
    if was_running {
        services::set(services::PROTOGAME, State::Stopped, "Stopped.");
    }
}

/// Hands an ask in, for the UDP thread.  An `Err` is the answer to send
/// right away instead, because Protogame isn't running to work it out.
pub fn hand_in(from: SocketAddr, account: String, ask: u32, work: Work) -> Result<(), Vec<u8>> {
    let unavailable = unavailable(ask, &work);
    let mailbox = MAILBOX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(sender) = mailbox.as_ref() else {
        return Err(unavailable);
    };
    sender.send(Job { from, account, ask, work }).map_err(|_| unavailable)
}

/// The answer to an ask that can't be worked on at all right now.
fn unavailable(ask: u32, work: &Work) -> Vec<u8> {
    match work {
        Work::List => protocol::command_refused(ask, LIST_UNAVAILABLE),
        Work::Create { .. } => protocol::create_result(ask, CreateAnswer::Unavailable),
        Work::Delete { .. } => protocol::delete_result(ask, DeleteAnswer::Denied, DELETE_UNAVAILABLE),
        Work::ResetHome { .. } => protocol::command_refused(ask, RESET_UNAVAILABLE),
        Work::Play { .. } | Work::Ready { .. } => protocol::command_refused(ask, PLAY_UNAVAILABLE),
    }
}

// ---------------------------------------------------------------------------
// The thread
// ---------------------------------------------------------------------------

fn work(mailbox: Receiver<Job>, map_cooldown: Duration) {
    loop {
        match mailbox.recv_timeout(CHECK_IN_EVERY) {
            Ok(job) => answer(job, map_cooldown),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        services::seen(services::PROTOGAME);
    }
}

/// Works one ask out, keeps the answer in the book, and sends it, if the
/// player who asked is still there.  Playing a character has its own way
/// through (`play()` and `ready()`), since it changes more than the answer.
fn answer(job: Job, map_cooldown: Duration) {
    let answer = match &job.work {
        Work::List => list(&job.account, job.ask),
        Work::Create { name } => create(&job.account, job.ask, name),
        Work::Delete { uuid, typed } => delete(&job.account, job.ask, uuid, typed),
        Work::ResetHome { uuid } => reset_home(&job.account, job.ask, uuid),
        Work::Play { uuid } => return play(job.from, &job.account, job.ask, uuid, map_cooldown),
        Work::Ready { hash } => return ready(job.from, &job.account, job.ask, hash),
    };
    if sessions::finish_ask(job.from, &job.account, job.ask, &answer) {
        udp::tell(job.from, &answer);
    }
}

/// The account's characters, with whether each can be played.
fn list(account: &str, ask: u32) -> Vec<u8> {
    match characters::list(account).wait_for(DATABASE_WAIT) {
        Some(Ok(list)) => {
            let listed: Vec<ListedCharacter> = list.iter().map(|character| ListedCharacter {
                uuid: character.uuid().to_string(),
                name: character.name().to_string(),
                slot: character.slot(),
                playable: !character.unplayable(),
            }).collect();
            protocol::character_list(ask, &listed)
        }
        Some(Err(e)) => {
            scribe::warn(Channel::Game, &format!("{account}'s characters couldn't be listed: {e}."));
            protocol::command_refused(ask, LIST_UNAVAILABLE)
        }
        None => protocol::command_refused(ask, LIST_UNAVAILABLE),
    }
}

/// A new character: the name checked, the Character template with the
/// name as its short name, standing at the spawn point, saved as Lua, and
/// the row made in the account's first empty slot.
fn create(account: &str, ask: u32, name: &str) -> Vec<u8> {
    if characters::check_character_name(name).is_err() {
        return protocol::create_result(ask, CreateAnswer::NameNotAllowed);
    }
    let place = match spawn_place() {
        Ok(place) => place,
        Err(why) => {
            scribe::warn(Channel::Game, &format!("{account}'s new character {name} wasn't made: the spawn point \
                can't be had ({why})."));
            return protocol::create_result(ask, CreateAnswer::Unavailable);
        }
    };
    let mut blueprint = gameobject::new_character(name);
    stand_at(&mut blueprint, place);
    let save_lua = Save::of_blueprint(&blueprint).to_lua();
    let pending = match characters::create(account, name, save_lua, place) {
        Ok(pending) => pending,
        Err(e) => {
            scribe::error(Channel::Game, &format!("Fingerprinter couldn't make a UUID for {account}'s new \
                character: {e}."));
            return protocol::create_result(ask, CreateAnswer::Unavailable);
        }
    };
    let answer = match pending.wait_for(DATABASE_WAIT) {
        Some(Ok(CharacterCreated::Made(character))) => {
            scribe::info(Channel::Game, &format!("{account} made the character {} in slot {}.", character.name(),
                                                 character.slot()));
            CreateAnswer::Made
        }
        Some(Ok(CharacterCreated::NameTaken)) => CreateAnswer::NameTaken,
        Some(Ok(CharacterCreated::SlotsFull)) => CreateAnswer::SlotsFull,
        // The account went while they were at character select: the
        // admin deleted it, and they're about to be kicked.
        Some(Ok(CharacterCreated::NoSuchAccount)) => CreateAnswer::Unavailable,
        Some(Err(e)) => {
            scribe::warn(Channel::Game, &format!("{account}'s new character {name} couldn't be made: {e}."));
            CreateAnswer::Unavailable
        }
        None => CreateAnswer::Unavailable,
    };
    protocol::create_result(ask, answer)
}

/// Deletes one of the account's characters, if the player typed DELETE.
fn delete(account: &str, ask: u32, uuid: &str, typed: &str) -> Vec<u8> {
    if !typed.eq_ignore_ascii_case(DELETE_WORD) {
        return protocol::delete_result(ask, DeleteAnswer::Denied, "Type DELETE to delete a character.");
    }
    match characters::delete(account, uuid).wait_for(DATABASE_WAIT) {
        Some(Ok(0)) => protocol::delete_result(ask, DeleteAnswer::Denied, "There's no such character on this \
            account."),
        Some(Ok(_)) => {
            scribe::info(Channel::Game, &format!("{account} deleted their character {uuid}."));
            protocol::delete_result(ask, DeleteAnswer::Approved, "The character has been deleted.")
        }
        Some(Err(e)) => {
            scribe::warn(Channel::Game, &format!("{account}'s character {uuid} couldn't be deleted: {e}."));
            protocol::delete_result(ask, DeleteAnswer::Denied, DELETE_UNAVAILABLE)
        }
        None => protocol::delete_result(ask, DeleteAnswer::Denied, DELETE_UNAVAILABLE),
    }
}

/// Puts one of the account's characters back at its spawn point, on top of
/// the highest block there.  The save holds the position too, so the save
/// is read back, made into the character, moved, and saved again whole,
/// with the position columns beside it.  A save that won't load makes the
/// character unplayable, the same as it would at the spawn.
fn reset_home(account: &str, ask: u32, uuid: &str) -> Vec<u8> {
    let loaded = match characters::load(account, uuid).wait_for(DATABASE_WAIT) {
        Some(Ok(Some(loaded))) => loaded,
        Some(Ok(None)) => return protocol::command_refused(ask, NO_SUCH_CHARACTER),
        Some(Err(e)) => {
            scribe::warn(Channel::Game, &format!("{account}'s character {uuid} couldn't be loaded to reset it \
                home: {e}."));
            return protocol::command_refused(ask, RESET_UNAVAILABLE);
        }
        None => return protocol::command_refused(ask, RESET_UNAVAILABLE),
    };
    let character = &loaded.snapshot;
    if character.unplayable() {
        return protocol::command_refused(ask, UNPLAYABLE);
    }
    let Some(mut blueprint) = from_save(&loaded) else {
        return protocol::command_refused(ask, CANT_BE_LOADED);
    };

    let home = match spawn_place() {
        Ok(place) => place,
        Err(why) => {
            scribe::warn(Channel::Game, &format!("{} wasn't reset home: the spawn point can't be had ({why}).",
                                                 character.name()));
            return protocol::command_refused(ask, RESET_UNAVAILABLE);
        }
    };
    stand_at(&mut blueprint, home);
    let save_lua = Save::of_blueprint(&blueprint).to_lua();
    let [x, y, z] = home;
    match characters::save(character.id(), home, save_lua).wait_for(DATABASE_WAIT) {
        Some(Ok(0)) => protocol::command_refused(ask, NO_SUCH_CHARACTER),
        Some(Ok(_)) => {
            scribe::info(Channel::Game, &format!("{account} reset {} home to {x}, {y}, {z}.", character.name()));
            protocol::command_accepted(ask)
        }
        Some(Err(e)) => {
            scribe::warn(Channel::Game, &format!("{} couldn't be saved at {x}, {y}, {z}: {e}.",
                                                 character.name()));
            protocol::command_refused(ask, RESET_UNAVAILABLE)
        }
        None => protocol::command_refused(ask, RESET_UNAVAILABLE),
    }
}

/// Where a character goes when it's made or reset home: on top of the
/// spawn point's highest block, in the middle of it.  There's one spawn
/// point today; which of them a character gets is for when there are more.
fn spawn_place() -> Result<[f32; 3], String> {
    let (x, z) = SPAWN_POINTS[0];
    spawn::place_at(x, z, SPAWN_WAIT)
}

/// Moves a character to `place`.  Only the position changes: the way it
/// faces and its size stay.
fn stand_at(blueprint: &mut Blueprint, place: [f32; 3]) {
    let mut moved = match blueprint.get(Kind::Transform) {
        Some(Component::Transform(transform)) => *transform,
        _ => Transform::default(),
    };
    moved.position = Vector3::new(place[0], place[1], place[2]);
    blueprint.set(Component::Transform(moved));
}

/// Plays one of the account's characters: loads it, holds it on the
/// player, and offers them the map to fetch before it comes in, or tells
/// them why not.  An account inside its map cooldown is refused first,
/// with nothing read or locked.  A character still locked after
/// `LOCK_WAIT` sends the player back to the login instead, with nothing
/// read.
fn play(from: SocketAddr, account: &str, ask: u32, uuid: &str, map_cooldown: Duration) {
    if let Some(left) = sessions::cooling_down(account, map_cooldown) {
        let refused = protocol::command_refused(ask, &cooling_down_words(left));
        if sessions::finish_ask(from, account, ask, &refused) {
            udp::tell(from, &refused);
        }
        scribe::debug(Channel::Game, &format!("{account} at {from} pressed PLAY inside its map cooldown, {} s \
            left.  Refused.", left.as_secs()));
        return;
    }

    // Before the row is read: the point is not to read it while another
    // copy is being brought in, or before the last session's save is in it.
    // A locked one is waited out, with the player told so, since the usual
    // way to hit it is a second login that logged the first out and pressed
    // PLAY inside its second (Jacob, 2026-10-02).
    if !sessions::lock_for_loading(uuid) {
        udp::tell(from, &protocol::please_wait(ask, LOCK_WAIT_WORDS));
        scribe::debug(Channel::Game, &format!("{account} at {from} picked a character that's locked for a moment \
            (loading, or just out of the world).  Waiting it out, up to {} s.", LOCK_WAIT.as_secs()));
        if !sessions::wait_for_loading_lock(uuid, LOCK_WAIT) {
            if sessions::turn_away(from, account) {
                udp::tell(from, &protocol::kicked(KickReason::CharacterLocked));
                scribe::warn(Channel::Game, &format!("{account} at {from} picked a character still locked after \
                    {} s: its save from its last session hasn't reached the database.  Sent back to the login \
                    to try again.", LOCK_WAIT.as_secs()));
            }
            return;
        }
    }

    let loaded = load(account, ask, uuid).and_then(|loading| match overworld::current() {
        Some(map) => {
            // `view_chunks` is 1 to 16 (game.cfg), so it fits in the byte.
            let offer = map.offer(ask, loading.position, conductor_gameworld::view_chunks() as u8);
            Ok((loading, offer))
        }
        // The door's open only with the map ready, so this is the server
        // stopping under them.
        None => Err(protocol::command_refused(ask, PLAY_UNAVAILABLE)),
    });
    let (loading, offer) = match loaded {
        Ok(loaded) => loaded,
        Err(refused) => {
            if sessions::finish_ask(from, account, ask, &refused) {
                udp::tell(from, &refused);
            }
            return;
        }
    };
    let name = loading.character.name.clone();
    if sessions::parked(from, account, ask, loading, &offer) {
        udp::tell(from, &offer);
        scribe::debug(Channel::Game, &format!("{account} at {from} is fetching the world's map before {name} comes \
            in."));
    } else {
        // They left while it was being loaded.  It was never in the
        // world, so there's nothing to take out.
        scribe::debug(Channel::Game, &format!("{account} left before {name} was loaded."));
    }
}

/// The player's client has the map: puts their loaded character in the
/// world and tells them where it stands, or tells them why not.
fn ready(from: SocketAddr, account: &str, ask: u32, hash: &str) {
    let (character, answer, told) = match put_in(from, account, ask, hash) {
        Ok(entered) => entered,
        Err(refused) => {
            if sessions::finish_ask(from, account, ask, &refused) {
                udp::tell(from, &refused);
            }
            return;
        }
    };
    let name = character.name.clone();
    let (id, character_uuid) = (character.id, character.uuid.clone());
    if sessions::entered(from, account, ask, character, &answer) {
        udp::tell(from, &answer);
        // Moved out of the ground: told in their chat, after they're in.
        if let Some(line) = told {
            udp::tell_all(&[from], &protocol::chat_deliveries(&[line.to_string()]));
        }
        scribe::info(Channel::Security, &format!("{account} is in the world as {name}, from {from}."));
    } else {
        // They left while it was being brought in.  Nobody is there to
        // play it, so it comes straight back out.
        sessions::leave_world(id, &character_uuid);
        scribe::debug(Channel::Game, &format!("{account} left before {name} was in the world.  It's taken back \
            out."));
    }
}

/// The slow part of playing a character: its row and save read, and made
/// into the character, to be held on the player while they fetch the map.
/// Or the CommandRefused saying why not.
fn load(account: &str, ask: u32, uuid: &str) -> Result<Loading, Vec<u8>> {
    let loaded = match characters::load(account, uuid).wait_for(DATABASE_WAIT) {
        Some(Ok(Some(loaded))) => loaded,
        Some(Ok(None)) => return Err(protocol::command_refused(ask, NO_SUCH_CHARACTER)),
        Some(Err(e)) => {
            scribe::warn(Channel::Game, &format!("{account}'s character {uuid} couldn't be loaded to play it: \
                {e}."));
            return Err(protocol::command_refused(ask, PLAY_UNAVAILABLE));
        }
        None => return Err(protocol::command_refused(ask, PLAY_UNAVAILABLE)),
    };
    let character = &loaded.snapshot;
    if character.unplayable() {
        return Err(protocol::command_refused(ask, UNPLAYABLE));
    }
    let Some(mut blueprint) = from_save(&loaded) else {
        return Err(protocol::command_refused(ask, CANT_BE_LOADED));
    };

    // Where it stands is where its save left it, unless that's in the
    // ground now.
    let saved_at = match blueprint.get(Kind::Transform) {
        Some(Component::Transform(transform)) => transform.position,
        _ => Vector3::default(),
    };
    let saved_at = [saved_at.x, saved_at.y, saved_at.z];
    let (position, told) = match out_of_the_ground(character.name(), saved_at) {
        Ok(Some((place, told))) => {
            stand_at(&mut blueprint, place);
            (place, Some(told))
        }
        Ok(None) => (saved_at, None),
        Err(why) => return Err(sent_to_spawn(ask, character, &mut blueprint, &why)),
    };
    let in_world = InWorld { id: character.id(), uuid: character.uuid().to_string(),
                             name: character.name().to_string() };
    Ok(Loading { character: in_world, blueprint, position, told })
}

/// Where a character saved at `saved_at` stands if that's inside the ground
/// now, and what its player is told: on top of its column, or at the spawn
/// point if that's too high.  `None` when where it was is clear.  An error
/// if GameWorld couldn't say.
fn out_of_the_ground(name: &str, saved_at: [f32; 3]) -> Result<Option<([f32; 3], &'static str)>, String> {
    let [x, y, z] = saved_at;
    match spawn::footing(saved_at, SPAWN_WAIT)? {
        Footing::Clear => Ok(None),
        Footing::OnTop(place) => {
            let [to_x, to_y, to_z] = place;
            scribe::debug(Channel::Game, &format!("{name} was saved inside the ground at {x}, {y}, {z}.  Stood on \
                top of it, at {to_x}, {to_y}, {to_z}."));
            Ok(Some((place, MOVED_ON_TOP)))
        }
        Footing::TooHigh => {
            let place = spawn_place()?;
            let [to_x, to_y, to_z] = place;
            scribe::debug(Channel::Game, &format!("{name} was saved inside the ground at {x}, {y}, {z}, under a \
                column too high to stand on.  Sent to the spawn point, {to_x}, {to_y}, {to_z}."));
            Ok(Some((place, MOVED_TO_SPAWN)))
        }
    }
}

/// GameWorld couldn't say where a character can stand (Jacob's "b"): it's
/// moved to the spawn point's last known place and saved there, and PLAY
/// is refused, for the next PLAY to check again.  The refusal to send.
fn sent_to_spawn(ask: u32, character: &CharacterSnapshot, blueprint: &mut Blueprint, why: &str) -> Vec<u8> {
    let (x, z) = SPAWN_POINTS[0];
    let Some(home) = spawn::last_known(x, z) else {
        scribe::warn(Channel::Game, &format!("{} couldn't be checked for ground at PLAY ({why}), and the spawn \
            point's place isn't known.  PLAY refused, nothing changed.", character.name()));
        return protocol::command_refused(ask, PLAY_UNAVAILABLE);
    };
    stand_at(blueprint, home);
    let save_lua = Save::of_blueprint(blueprint).to_lua();
    let [to_x, to_y, to_z] = home;
    match characters::save(character.id(), home, save_lua).wait_for(DATABASE_WAIT) {
        Some(Ok(rows)) if rows > 0 => {
            scribe::warn(Channel::Game, &format!("{} couldn't be checked for ground at PLAY ({why}).  Moved to \
                the spawn point's last known place, {to_x}, {to_y}, {to_z}, and PLAY refused.", character.name()));
            protocol::command_refused(ask, COULDNT_CHECK)
        }
        Some(Ok(_)) => protocol::command_refused(ask, NO_SUCH_CHARACTER),
        Some(Err(e)) => {
            scribe::warn(Channel::Game, &format!("{} couldn't be checked for ground at PLAY ({why}), and couldn't \
                be saved at the spawn point either: {e}.", character.name()));
            protocol::command_refused(ask, PLAY_UNAVAILABLE)
        }
        None => protocol::command_refused(ask, PLAY_UNAVAILABLE),
    }
}

/// The quick part, on PlayerReady: the hash checked against the map's,
/// and the character held on the player handed to the GameClock.  The
/// character and the CharacterEnteredWorld to send, or the CommandRefused
/// saying why not.  A character that can't go in is let go, and the player
/// is back at character select, free to press PLAY again.
fn put_in(from: SocketAddr, account: &str, ask: u32, hash: &str)
    -> Result<(InWorld, Vec<u8>, Option<&'static str>), Vec<u8>> {
    let Some(map) = overworld::current() else {
        return Err(protocol::command_refused(ask, PLAY_UNAVAILABLE));
    };
    if !hash.eq_ignore_ascii_case(map.hash()) {
        scribe::debug(Channel::Game, &format!("{account} at {from} says it has the map, and its hash isn't the \
            map's.  Not let in."));
        return Err(protocol::command_refused(ask, MAP_DOES_NOT_MATCH));
    }
    let Some(loading) = sessions::take_loading(from, account) else {
        return Err(protocol::command_refused(ask, PLAY_UNAVAILABLE));
    };
    let object = match conductor_gameclock::enter(loading.blueprint, &loading.character.uuid) {
        Ok(object) => object,
        Err(why) => {
            scribe::warn(Channel::Game, &format!("{} couldn't be put in the world: {why}.", loading.character.name));
            return Err(protocol::command_refused(ask, PLAY_UNAVAILABLE));
        }
    };
    let entered = EnteredCharacter { uuid: loading.character.uuid.clone(), name: loading.character.name.clone(),
                                     position: loading.position, object };
    Ok((loading.character, protocol::entered_world(ask, &entered), loading.told))
}

/// A loaded character's save, read back through lua-parser and laid over
/// the Character template.  A save that won't load marks the character
/// unplayable for the run, with the Error on the bell (the why is there),
/// and comes back `None`.
fn from_save(loaded: &CharacterSave) -> Option<Blueprint> {
    let character = &loaded.snapshot;
    let row = format!("player_characters row {}", character.id());
    let blueprint = conductor_lua_parser::read_save(&row, &loaded.save_lua)
        .and_then(|save| gameobject::character_from_save(character.account_id(), character.id(), &save));
    match blueprint {
        Ok(blueprint) => Some(blueprint),
        Err(why) => {
            characters::mark_unplayable(character.id(), character.name(), &why);
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::PacketType;

    #[test]
    fn an_ask_with_protogame_stopped_gets_unavailable_at_once() {
        stop();
        let from: SocketAddr = "10.0.0.5:50000".parse().unwrap();
        let answer = hand_in(from, "jacob_01".to_string(), 3, Work::List).unwrap_err();
        assert_eq!(answer[0], PacketType::CommandRefused as u8);
        let answer = hand_in(from, "jacob_01".to_string(), 4, Work::Create { name: "Jacob".to_string() })
            .unwrap_err();
        let expected = [PacketType::CharacterCreateResult as u8, 4, 0, 0, 0, CreateAnswer::Unavailable as u8];
        assert_eq!(&answer[..6], &expected);
        let answer = hand_in(from, "jacob_01".to_string(), 5, Work::Play { uuid: "u-1".to_string() }).unwrap_err();
        assert_eq!(&answer[..5], &[PacketType::CommandRefused as u8, 5, 0, 0, 0]);
    }

    #[test]
    fn the_cooldown_says_its_seconds_rounded_up() {
        let words = cooling_down_words(Duration::from_millis(213_100));
        assert_eq!(words, "You are temporarily cooling down from download for DDOS protection. You have 214 \
            seconds remaining.");
        assert!(cooling_down_words(Duration::from_secs(214)).contains(" 214 seconds"));
        assert!(cooling_down_words(Duration::from_millis(1)).contains(" 1 seconds"));
    }

    #[test]
    fn only_the_word_delete_deletes() {
        // The word is checked before the database is asked, so no
        // Archivist is needed to see it turned away.
        let denied = delete("jacob_01", 2, "u-1", "yes");
        let expected = [PacketType::CharacterDeleteResult as u8, 2, 0, 0, 0, DeleteAnswer::Denied as u8];
        assert_eq!(&denied[..6], &expected);
    }

    #[test]
    fn a_name_against_the_rule_is_turned_away_before_the_database() {
        let answer = create("jacob_01", 1, "McKay");
        assert_eq!(answer[5], CreateAnswer::NameNotAllowed as u8);
    }
}
