//! File:       Opus/Conductor/dev/gameclock/src/players.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The players' characters in the world, and the mailbox that brings them
//! in and takes them out.  Only the GameClock's thread touches the world,
//! so nothing else spawns a character: networking leaves a note with
//! `enter()` or `leave()`, which come straight back, and the input check
//! empties the mailbox at the start of every cycle.
//!
//! The slow part of a character coming in (reading its row, reading its
//! save through lua-parser, laying it over the Character template) is
//! done before the note is left, on whatever thread left it.  The note
//! carries the finished blueprint, and the GameClock only spawns it.
//!
//! Leaving takes the copy out at once (Jacob, 2026-09-30: "when the
//! character's registered as quit out the game removes them"), and its
//! save goes to the database on the way.  Nothing is left standing in the
//! world for a reconnect.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use conductor_accounts::characters::SavedCharacter;
use conductor_primlib::{Blueprint, Component, Entity, Kind, Save, World};
use conductor_tools::scribe::{self, Channel};

/// A note for the GameClock.
pub enum Note {
    /// Put this character in the world.  The blueprint is the Character
    /// template with its save laid over it, `PlayerCharacter` and all.
    Enter(Blueprint),
    /// Take this character out of the world, by its row's id, and save
    /// it on the way.
    Leave(i64),
}

/// The sending end of the mailbox while the GameClock runs.  `None` while
/// it's stopped, so a note left then is turned away instead of waiting
/// for a GameClock that isn't coming.
static MAILBOX: Mutex<Option<Sender<Note>>> = Mutex::new(None);

/// A fresh mailbox, for a fresh start of the GameClock.  The receiving
/// end goes to its thread.
pub(crate) fn open_mailbox() -> Receiver<Note> {
    let (sender, receiver) = mpsc::channel();
    *lock(&MAILBOX) = Some(sender);
    receiver
}

/// Turns notes away from here on.  For `stop()`.
pub(crate) fn close_mailbox() {
    lock(&MAILBOX).take();
}

/// Asks for a player's character to be put in the world.  Comes straight
/// back; the character is in by the end of the next cycle's input check.
/// An error says why it wasn't asked: the blueprint isn't a player's
/// character, or the GameClock isn't running.
pub fn enter(character: Blueprint) -> Result<(), String> {
    if character_id_of(&character).is_none() {
        return Err("it has no PlayerCharacter with a row behind it, so it isn't a player's character".to_string());
    }
    send(Note::Enter(character))
}

/// Asks for a player's character to be taken out of the world and saved.
/// Comes straight back.  An error means the GameClock isn't running.
pub fn leave(character_id: i64) -> Result<(), String> {
    send(Note::Leave(character_id))
}

/// Leaves a note in the mailbox.
fn send(note: Note) -> Result<(), String> {
    match lock(&MAILBOX).as_ref() {
        Some(mailbox) => mailbox.send(note).map_err(|_| "the GameClock has stopped".to_string()),
        None => Err("the GameClock isn't running".to_string()),
    }
}

/// The character's row id, from its `PlayerCharacter`.  `None` without
/// one, or with the template's empty one (0: a row's id starts at 1), so
/// there's no row to save it to.
fn character_id_of(character: &Blueprint) -> Option<i64> {
    match character.get(Kind::PlayerCharacter) {
        Some(Component::PlayerCharacter(player)) if player.character_id > 0 => Some(player.character_id),
        _ => None,
    }
}

/// The lock idiom, for the static above.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The players' characters in the world, by their row's id, and the
/// GameClock's end of the mailbox.
pub struct Players {
    in_world: BTreeMap<i64, Entity>,
    notes: Receiver<Note>,
}

impl Players {
    pub fn new(notes: Receiver<Note>) -> Players {
        Players { in_world: BTreeMap::new(), notes }
    }

    /// How many players' characters are in the world.
    pub fn count(&self) -> usize {
        self.in_world.len()
    }

    /// Empties the mailbox: every character asked in is spawned, and every
    /// one asked out is despawned.  Hands back the saves of the ones who
    /// left, taken just before they went, for the caller to send to the
    /// database.  Never waits: a note left after this looked is read next
    /// cycle.
    pub fn take_notes(&mut self, world: &mut World) -> Vec<SavedCharacter> {
        let mut leaving = Vec::new();
        loop {
            match self.notes.try_recv() {
                Ok(Note::Enter(character)) => self.enter(world, &character),
                Ok(Note::Leave(character_id)) => leaving.extend(self.leave(world, character_id)),
                // Empty is the usual end.  Disconnected can't happen while
                // the GameClock runs, since the sending end is only dropped
                // by `stop()`.
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        leaving
    }

    /// Spawns a character, unless it's already in the world.
    fn enter(&mut self, world: &mut World, character: &Blueprint) {
        let Some(character_id) = character_id_of(character) else {
            return;
        };
        if let Some(&there) = self.in_world.get(&character_id) {
            scribe::warn(Channel::Game, &format!("Character {character_id} ({}) was asked into the world while \
                it's already there.  The one there stands.", name(world, there)));
            return;
        }
        let entity = world.spawn(character);
        self.in_world.insert(character_id, entity);
        scribe::debug(Channel::Game, &format!("Character {character_id} ({}) came into the world.",
                                              name(world, entity)));
    }

    /// Saves a character and despawns it.  `None` if it wasn't in the
    /// world.
    fn leave(&mut self, world: &mut World, character_id: i64) -> Option<SavedCharacter> {
        let Some(entity) = self.in_world.remove(&character_id) else {
            scribe::debug(Channel::Game, &format!("Character {character_id} was asked out of the world, and it \
                wasn't in it."));
            return None;
        };
        let saved = saved_character(world, character_id, entity);
        scribe::debug(Channel::Game, &format!("Character {character_id} ({}) left the world.", name(world, entity)));
        world.despawn(entity);
        saved
    }

    /// Every player's character in the world as it stands right now, for a
    /// world save.  Only copies: nothing is turned into text here, and
    /// the database isn't touched.
    pub fn snapshot(&self, world: &World) -> Vec<SavedCharacter> {
        self.in_world.iter()
            .filter_map(|(&character_id, &entity)| saved_character(world, character_id, entity))
            .collect()
    }
}

/// One character's save, as it stands.  `None` (and an Error) if its entity
/// has gone from under the list, which would be a bug.
fn saved_character(world: &World, character_id: i64, entity: Entity) -> Option<SavedCharacter> {
    let Some(save) = Save::of(world, entity) else {
        scribe::error(Channel::Game, &format!("Character {character_id} is on the GameClock's list of players, \
            and its entity is gone from the world.  It can't be saved."));
        return None;
    };
    let position = world.transform(entity).map(|transform| transform.position).unwrap_or_default();
    Some(SavedCharacter { character_id, position: [position.x, position.y, position.z], save })
}

/// A character's short name, for the log.
fn name(world: &World, entity: Entity) -> String {
    world.short_name(entity).map(|name| name.text.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use conductor_primlib::gameobject::new_character;
    use conductor_primlib::{PlayerCharacter, Vector3};

    /// A player's character, as it would come in from its row.
    fn character(name: &str, character_id: i64) -> Blueprint {
        let mut blueprint = new_character(name);
        blueprint.set(Component::PlayerCharacter(PlayerCharacter::new(7, character_id)));
        blueprint
    }

    fn players() -> (Sender<Note>, Players) {
        let (sender, receiver) = mpsc::channel();
        (sender, Players::new(receiver))
    }

    #[test]
    fn a_character_asked_in_is_spawned() {
        let (mailbox, mut players) = players();
        let mut world = World::new();
        assert!(mailbox.send(Note::Enter(character("Jacob", 42))).is_ok());
        assert!(players.take_notes(&mut world).is_empty());
        assert_eq!(players.count(), 1);
        assert_eq!(world.count(), 1);
    }

    #[test]
    fn a_character_asked_in_twice_is_spawned_once() {
        let (mailbox, mut players) = players();
        let mut world = World::new();
        assert!(mailbox.send(Note::Enter(character("Jacob", 42))).is_ok());
        assert!(mailbox.send(Note::Enter(character("Jacob", 42))).is_ok());
        players.take_notes(&mut world);
        assert_eq!(players.count(), 1);
        assert_eq!(world.count(), 1);
    }

    #[test]
    fn leaving_despawns_and_hands_back_the_save() {
        let (mailbox, mut players) = players();
        let mut world = World::new();
        assert!(mailbox.send(Note::Enter(character("Jacob", 42))).is_ok());
        assert!(mailbox.send(Note::Enter(character("Mckay", 43))).is_ok());
        players.take_notes(&mut world);

        assert!(mailbox.send(Note::Leave(42)).is_ok());
        let leaving = players.take_notes(&mut world);
        assert_eq!(leaving.len(), 1);
        assert_eq!(leaving[0].character_id, 42);
        assert_eq!(players.count(), 1);
        assert_eq!(world.count(), 1, "Mckay is still there");
    }

    #[test]
    fn leaving_when_not_in_the_world_saves_nothing() {
        let (mailbox, mut players) = players();
        let mut world = World::new();
        assert!(mailbox.send(Note::Leave(42)).is_ok());
        assert!(players.take_notes(&mut world).is_empty());
    }

    #[test]
    fn a_snapshot_is_every_player_as_they_stand() {
        let (mailbox, mut players) = players();
        let mut world = World::new();
        assert!(mailbox.send(Note::Enter(character("Jacob", 42))).is_ok());
        assert!(mailbox.send(Note::Enter(character("Mckay", 43))).is_ok());
        players.take_notes(&mut world);

        // Jacob walks off, after he came in.
        let jacob = world.all().into_iter()
            .find(|&entity| world.player_character(entity).is_some_and(|player| player.character_id == 42));
        let moved = jacob.and_then(|jacob| world.transform_mut(jacob))
            .map(|transform| transform.position = Vector3::new(12.5, 1.0, -7.25));
        assert!(moved.is_some());

        let snapshot = players.snapshot(&world);
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].character_id, 42);
        assert_eq!(snapshot[0].position, [12.5, 1.0, -7.25]);
        assert_eq!(snapshot[1].character_id, 43);
        assert_eq!(snapshot[1].position, [0.0, 0.0, 0.0]);
        assert!(snapshot[0].save.to_lua().contains("x = 12.5"), "{}", snapshot[0].save.to_lua());
    }

    #[test]
    fn a_blueprint_that_isnt_a_players_character_is_turned_away() {
        assert_eq!(character_id_of(&character("Jacob", 42)), Some(42));
        assert_eq!(character_id_of(&new_character("Jacob")), None, "no row behind it yet");
        let mut blueprint = new_character("Jacob");
        blueprint.remove(Kind::PlayerCharacter);
        assert_eq!(character_id_of(&blueprint), None);
        assert!(enter(blueprint).is_err());
    }

    /// How long a world save holds up the GameClock: the snapshot of
    /// 10,000 characters.  Timed, so it runs by hand, optimized:
    /// `cargo test --release -p conductor-gameclock -- --ignored --nocapture snapshot_of_ten_thousand`
    #[test]
    #[ignore]
    fn snapshot_of_ten_thousand() {
        let (mailbox, mut players) = players();
        let mut world = World::new();
        for character_id in 1..=10_000 {
            assert!(mailbox.send(Note::Enter(character("Jacob", character_id))).is_ok());
        }
        players.take_notes(&mut world);
        assert_eq!(players.count(), 10_000);

        let began = std::time::Instant::now();
        let snapshot = players.snapshot(&world);
        let took = began.elapsed();
        assert_eq!(snapshot.len(), 10_000);
        println!("The snapshot of 10,000 characters took {:.2} ms, of housekeeping's 50.",
                 took.as_secs_f64() * 1000.0);
    }
}
