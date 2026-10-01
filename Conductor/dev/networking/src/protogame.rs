//! File:       Opus/Conductor/dev/networking/src/protogame.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Protogame: what goes on between a player logging in and their
//! character being in the world.  Jacob's word, 2026-09-30: "the character
//! selection and character construction are proto game then become game
//! objects after load."  That's character select: listing an account's
//! characters, making one, deleting one, putting one back at 0, 0, 0, and
//! the last step, playing one (protocol version 6).
//!
//! Playing one is the spawn's slow part, done here so the GameClock never
//! waits on it: the row and the save are read, the save is laid over the
//! Character template, and the finished blueprint goes in the GameClock's
//! mailbox (`conductor_gameclock::enter()`).  Only then is the character
//! written on the player in the book, so a player who left while it was
//! being brought in leaves nothing standing: their character is taken
//! straight back out.  A character that left the world under a second
//! ago isn't read at all (`sessions::just_left()`): its player is sent
//! back to the login to try again, Jacob's fix for a quick second login
//! reading the row before the last session's save is in it.
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

use conductor_accounts::characters::{self, CharacterCreated, CharacterSave};
use conductor_primlib::gameobject;
use conductor_primlib::{Blueprint, Component, Kind, Save, Transform, Vector3};
use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};
use conductor_tools::threads;

use crate::protocol::{self, CreateAnswer, DeleteAnswer, EnteredCharacter, KickReason, ListedCharacter};
use crate::sessions::{self, InWorld};
use crate::udp;

/// How long Protogame waits on the database for one ask.  Past that the
/// player is told it's unavailable (the job may still happen: a list
/// asked for again shows it), so a stuck database can't hold up every
/// player's asks behind it for good.
const DATABASE_WAIT: Duration = Duration::from_secs(10);

/// How long the thread sleeps for an ask before it checks in with the
/// services list.
const CHECK_IN_EVERY: Duration = Duration::from_secs(1);

/// The word a player types to delete a character, whatever the capitals
/// (Jacob: "the client pre-reqs to ask them to type in delete").
const DELETE_WORD: &str = "DELETE";

/// What the player hears when the database can't be asked.  Short labels,
/// no period: nothing the player can act on (the protocol's rule).
const LIST_UNAVAILABLE: &str = "Character List Unavailable";
const DELETE_UNAVAILABLE: &str = "Character Deletion Unavailable";
const RESET_UNAVAILABLE: &str = "Reset Home Unavailable";
const PLAY_UNAVAILABLE: &str = "World Unavailable";

/// What the player hears for a character that isn't on their account,
/// and for one whose save won't load or that's been marked unplayable.
const NO_SUCH_CHARACTER: &str = "There's no such character on this account.";
const CANT_BE_LOADED: &str = "That character can't be loaded.  The admin has been told.";
const UNPLAYABLE: &str = "That character can't be played until the admin has looked at it.";

/// What a player asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Work {
    /// Their characters: a CharacterListRequest.
    List,
    /// A new character with this name: a CreateCharacter.
    Create { name: String },
    /// Delete this character, with the word they typed: a DeleteCharacter.
    Delete { uuid: String, typed: String },
    /// Put this character back at 0, 0, 0: a CharacterRequestResetHome.
    ResetHome { uuid: String },
    /// Bring this character into the world: a UserPressPlay.
    Play { uuid: String },
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

/// Starts the thread.  An `Err` says what went wrong, in words.
pub fn start() -> Result<(), String> {
    let mut mailbox = MAILBOX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if mailbox.is_some() {
        return Err("Protogame is already running.".to_string());
    }
    let (sender, receiver) = mpsc::channel();
    let worker = threads::spawn("protogame", move || work(receiver))
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
        Work::Play { .. } => protocol::command_refused(ask, PLAY_UNAVAILABLE),
    }
}

// ---------------------------------------------------------------------------
// The thread
// ---------------------------------------------------------------------------

fn work(mailbox: Receiver<Job>) {
    loop {
        match mailbox.recv_timeout(CHECK_IN_EVERY) {
            Ok(job) => answer(job),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        services::seen(services::PROTOGAME);
    }
}

/// Works one ask out, keeps the answer in the book, and sends it, if the
/// player who asked is still there.  Playing a character has its own way
/// through (`play()`), since it changes more than the answer.
fn answer(job: Job) {
    let answer = match &job.work {
        Work::List => list(&job.account, job.ask),
        Work::Create { name } => create(&job.account, job.ask, name),
        Work::Delete { uuid, typed } => delete(&job.account, job.ask, uuid, typed),
        Work::ResetHome { uuid } => reset_home(&job.account, job.ask, uuid),
        Work::Play { uuid } => return play(job.from, &job.account, job.ask, uuid),
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
/// name as its short name saved as Lua, and the row made in the account's
/// first empty slot.
fn create(account: &str, ask: u32, name: &str) -> Vec<u8> {
    if characters::check_character_name(name).is_err() {
        return protocol::create_result(ask, CreateAnswer::NameNotAllowed);
    }
    let save_lua = Save::of_blueprint(&gameobject::new_character(name)).to_lua();
    let pending = match characters::create(account, name, save_lua) {
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

/// Puts one of the account's characters back at 0, 0, 0.  The save holds
/// the position too, so the save is read back, made into the character,
/// moved, and saved again whole, with the position columns beside it.  A
/// save that won't load makes the character unplayable, the same as it
/// would at the spawn.
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

    // Only the position changes: the way it faces and its size stay.
    let home = Vector3::new(0.0, 0.0, 0.0);
    let mut moved = match blueprint.get(Kind::Transform) {
        Some(Component::Transform(transform)) => *transform,
        _ => Transform::default(),
    };
    moved.position = home;
    blueprint.set(Component::Transform(moved));
    let save_lua = Save::of_blueprint(&blueprint).to_lua();
    match characters::save(character.id(), [home.x, home.y, home.z], save_lua).wait_for(DATABASE_WAIT) {
        Some(Ok(0)) => protocol::command_refused(ask, NO_SUCH_CHARACTER),
        Some(Ok(_)) => {
            scribe::info(Channel::Game, &format!("{account} reset {} home to 0, 0, 0.", character.name()));
            protocol::command_accepted(ask)
        }
        Some(Err(e)) => {
            scribe::warn(Channel::Game, &format!("{} couldn't be saved at 0, 0, 0: {e}.", character.name()));
            protocol::command_refused(ask, RESET_UNAVAILABLE)
        }
        None => protocol::command_refused(ask, RESET_UNAVAILABLE),
    }
}

/// Plays one of the account's characters: brings it into the world and
/// tells the player where it stands, or tells them why not.  A character
/// in its lockout sends the player back to the login instead, with
/// nothing read.
fn play(from: SocketAddr, account: &str, ask: u32, uuid: &str) {
    // Before the row is read: the point is not to read it until the last
    // session's save is in it.
    if sessions::just_left(uuid) {
        if sessions::turn_away(from, account) {
            udp::tell(from, &protocol::kicked(KickReason::CharacterLeaving));
            scribe::info(Channel::Security, &format!("{account} at {from} picked a character that left the \
                world under a second ago.  Sent back to the login to try again."));
        }
        return;
    }

    let (character, answer) = match bring_in(account, ask, uuid) {
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
        scribe::info(Channel::Security, &format!("{account} is in the world as {name}, from {from}."));
    } else {
        // They left while it was being brought in.  Nobody is there to
        // play it, so it comes straight back out.
        sessions::leave_world(id, &character_uuid);
        scribe::debug(Channel::Game, &format!("{account} left before {name} was in the world.  It's taken back \
            out."));
    }
}

/// The slow part of playing a character: its row and save read, made
/// into the character, and handed to the GameClock.  The character and the
/// CharacterEnteredWorld to send, or the CommandRefused saying why not.
fn bring_in(account: &str, ask: u32, uuid: &str) -> Result<(InWorld, Vec<u8>), Vec<u8>> {
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
    let Some(blueprint) = from_save(&loaded) else {
        return Err(protocol::command_refused(ask, CANT_BE_LOADED));
    };

    // Where it stands is where its save left it.
    let position = match blueprint.get(Kind::Transform) {
        Some(Component::Transform(transform)) => transform.position,
        _ => Vector3::default(),
    };
    if let Err(why) = conductor_gameclock::enter(blueprint) {
        scribe::warn(Channel::Game, &format!("{} couldn't be put in the world: {why}.", character.name()));
        return Err(protocol::command_refused(ask, PLAY_UNAVAILABLE));
    }

    let entered = EnteredCharacter { uuid: character.uuid().to_string(), name: character.name().to_string(),
                                     position: [position.x, position.y, position.z] };
    let answer = protocol::entered_world(ask, &entered);
    Ok((InWorld { id: character.id(), uuid: entered.uuid, name: entered.name }, answer))
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
