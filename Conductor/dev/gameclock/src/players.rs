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
//!
//! A character asked out is marked "saving" from the moment `leave()` is
//! called, on the caller's thread, until its save has landed in the
//! database (or failed, with the Error on the bell).  Networking asks
//! `saving()` before it reads a character's row to play it, and a login
//! that logged another session out waits on `wait_until_saved()` before
//! it hands out its ticket, so nobody comes in on the save before last.
//! Jacob, 2026-10-01: "do we have any way to force a save on the
//! connection being kicked before the new one pops in?"

use std::collections::{BTreeMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Condvar, LazyLock, Mutex};
use std::time::{Duration, Instant};

use conductor_accounts::characters::SavedCharacter;
use conductor_primlib::{Blueprint, Component, Entity, Kind, Save, World};
use conductor_tools::scribe::{self, Channel};

use crate::who::{Standing, block_of};

/// A note for the GameClock.
pub enum Note {
    /// Put this character in the world.
    Enter(Arrival),
    /// Take this character out of the world, by its row's id, and save
    /// it on the way.
    Leave(i64),
}

/// A player's character on its way into the world.
pub struct Arrival {
    /// The Character template with its save laid over it,
    /// `PlayerCharacter` and all.
    pub character: Blueprint,
    /// Its row's uuid, the game's name for it, for the clients
    /// (`view.rs`).
    pub uuid: String,
    /// The number its clients know it by from now on, handed out by
    /// `enter()`.
    pub object: u32,
}

/// The sending end of the mailbox while the GameClock runs.  `None` while
/// it's stopped, so a note left then is turned away instead of waiting
/// for a GameClock that isn't coming.
static MAILBOX: Mutex<Option<Sender<Note>>> = Mutex::new(None);

/// The characters asked out of the world whose save hasn't landed yet, by
/// their row's id, and the bell that rings when one comes off.
// Rust note: a Condvar is the OS's own "wait until somebody says so": a
// thread waits on it with the lock let go, and `notify_all()` wakes it to
// look again.  Nothing polls.  A HashSet can't be built before the
// program starts, so LazyLock builds it the first time it's touched.
static SAVING: LazyLock<Mutex<HashSet<i64>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static SAVED: Condvar = Condvar::new();

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

/// Asks for a player's character, whose row's uuid is `uuid`, to be put
/// in the world.  Comes straight back with the number every client will
/// know it by (protocol version 14), so the player can be told it before
/// it's there; the character is in by the end of the next cycle's input
/// check.  An error says why it wasn't asked: the blueprint isn't a
/// player's character, or the GameClock isn't running.
pub fn enter(character: Blueprint, uuid: &str) -> Result<u32, String> {
    if character_id_of(&character).is_none() {
        return Err("it has no PlayerCharacter with a row behind it, so it isn't a player's character".to_string());
    }
    let object = crate::view::next_object_number();
    send(Note::Enter(Arrival { character, uuid: uuid.to_string(), object }))?;
    Ok(object)
}

/// Asks for a player's character to be taken out of the world and saved.
/// Comes straight back.  The character is marked "saving" until its save
/// has landed.  An error means the GameClock isn't running, and then
/// there's no mark: its last world save on the way down had it.
pub fn leave(character_id: i64) -> Result<(), String> {
    lock(&SAVING).insert(character_id);
    let sent = send(Note::Leave(character_id));
    if sent.is_err() {
        saved(&[character_id]);
    }
    sent
}

/// Whether a character asked out of the world is still on its way to the
/// database.  Its row isn't to be read to play it until this says false.
pub fn saving(character_id: i64) -> bool {
    lock(&SAVING).contains(&character_id)
}

/// Waits until a character asked out of the world has its save in the
/// database, for up to `limit`.  True once it has (or it was never
/// marked); false if `limit` ran out first.  For a login thread, never the
/// GameClock's.
pub fn wait_until_saved(character_id: i64, limit: Duration) -> bool {
    let marks = lock(&SAVING);
    let (marks, _) = SAVED.wait_timeout_while(marks, limit, |marks| marks.contains(&character_id))
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    !marks.contains(&character_id)
}

/// Takes the "saving" mark off these characters: their save landed, or
/// failed, or there was nothing of them to save.  Wakes whoever waits.
pub(crate) fn saved(character_ids: &[i64]) {
    if character_ids.is_empty() {
        return;
    }
    let mut marks = lock(&SAVING);
    for character_id in character_ids {
        marks.remove(character_id);
    }
    SAVED.notify_all();
}

/// Takes every mark off, for a GameClock starting or stopping: nothing is
/// on its way any more, and nobody should wait on it.
pub(crate) fn forget_saving() {
    lock(&SAVING).clear();
    SAVED.notify_all();
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
    in_world: BTreeMap<i64, InWorld>,
    notes: Receiver<Note>,
}

/// A player's character in the world: its entity, when it came in (for
/// `/who`'s time online), its uuid and its number (for the clients).
struct InWorld {
    entity: Entity,
    entered: Instant,
    uuid: String,
    object: u32,
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
                Ok(Note::Enter(arrival)) => self.enter(world, arrival),
                Ok(Note::Leave(character_id)) => leaving.extend(self.leave(world, character_id)),
                // Empty is the usual end.  Disconnected only comes in the
                // gap inside `stop()` between the mailbox closing and the
                // beat stopping, and means the same: nothing more to read.
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        leaving
    }

    /// Spawns a character, unless it's already in the world.
    fn enter(&mut self, world: &mut World, arrival: Arrival) {
        let Arrival { character, uuid, object } = arrival;
        let Some(character_id) = character_id_of(&character) else {
            return;
        };
        if let Some(there) = self.in_world.get(&character_id) {
            scribe::warn(Channel::Game, &format!("Character {character_id} ({}) was asked into the world while \
                it's already there.  The one there stands.", name(world, there.entity)));
            return;
        }
        let entity = world.spawn(&character);
        self.in_world.insert(character_id, InWorld { entity, entered: Instant::now(), uuid, object });
        scribe::debug(Channel::Game, &format!("Character {character_id} ({}) came into the world.",
                                              name(world, entity)));
    }

    /// Saves a character and despawns it.  `None` if it wasn't in the
    /// world, and then its "saving" mark comes off at once, since there's
    /// nothing of it to save.
    fn leave(&mut self, world: &mut World, character_id: i64) -> Option<SavedCharacter> {
        let Some(InWorld { entity, .. }) = self.in_world.remove(&character_id) else {
            scribe::debug(Channel::Game, &format!("Character {character_id} was asked out of the world, and it \
                wasn't in it."));
            saved(&[character_id]);
            return None;
        };
        let save = saved_character(world, character_id, entity);
        scribe::debug(Channel::Game, &format!("Character {character_id} ({}) left the world.", name(world, entity)));
        world.despawn(entity);
        if save.is_none() {
            saved(&[character_id]);
        }
        save
    }

    /// Every player's character in the world, its name, the block it
    /// stands in and how long it's been in the world as of `now`, for a
    /// `/who`.  The one in longest first and the newest last (Jacob,
    /// 2026-10-03: "Oldest log in goes at the top, newest at the bottom").
    pub fn standing(&self, world: &World, now: Instant) -> Vec<Standing> {
        let mut in_order: Vec<&InWorld> = self.in_world.values().collect();
        in_order.sort_by_key(|character| character.entered);
        in_order.into_iter().map(|character| {
            let position = world.transform(character.entity).map(|transform| transform.position).unwrap_or_default();
            Standing {
                name: name(world, character.entity),
                block: block_of([position.x, position.y, position.z]),
                online: now.saturating_duration_since(character.entered),
            }
        }).collect()
    }

    /// The entity of the character whose row's id is `character_id`, if
    /// it's in the world.
    pub fn entity_of(&self, character_id: i64) -> Option<Entity> {
        self.in_world.get(&character_id).map(|character| character.entity)
    }

    /// Every player's character in the world, its row's id and its
    /// entity, in row id order.  Borrowed, not copied, for the checks that
    /// go through everybody (`movement.rs`, `ground.rs`).
    pub fn characters(&self) -> impl Iterator<Item = (i64, Entity)> + '_ {
        self.in_world.iter().map(|(&id, character)| (id, character.entity))
    }

    /// Every player's character in the world, for what the clients are
    /// told (`view.rs`): its row's id, its entity, its number and its uuid,
    /// in row id order.
    pub fn objects(&self) -> Vec<(i64, Entity, u32, &str)> {
        self.in_world.iter()
            .map(|(&id, character)| (id, character.entity, character.object, character.uuid.as_str()))
            .collect()
    }

    /// Every player's character in the world as it stands right now, for a
    /// world save.  Only copies: nothing is turned into text here, and
    /// the database isn't touched.
    pub fn snapshot(&self, world: &World) -> Vec<SavedCharacter> {
        self.in_world.iter()
            .filter_map(|(&character_id, character)| saved_character(world, character_id, character.entity))
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

    /// A note asking a character in, numbered as its row id.
    fn enter_note(character: Blueprint) -> Note {
        let id = character_id_of(&character).unwrap_or_default();
        Note::Enter(Arrival { character, uuid: format!("u-{id}"), object: id as u32 })
    }

    fn players() -> (Sender<Note>, Players) {
        let (sender, receiver) = mpsc::channel();
        (sender, Players::new(receiver))
    }

    #[test]
    fn a_character_asked_in_is_spawned() {
        let (mailbox, mut players) = players();
        let mut world = World::new();
        assert!(mailbox.send(enter_note(character("Jacob", 42))).is_ok());
        assert!(players.take_notes(&mut world).is_empty());
        assert_eq!(players.count(), 1);
        assert_eq!(world.count(), 1);
    }

    #[test]
    fn a_character_asked_in_twice_is_spawned_once() {
        let (mailbox, mut players) = players();
        let mut world = World::new();
        assert!(mailbox.send(enter_note(character("Jacob", 42))).is_ok());
        assert!(mailbox.send(enter_note(character("Jacob", 42))).is_ok());
        players.take_notes(&mut world);
        assert_eq!(players.count(), 1);
        assert_eq!(world.count(), 1);
    }

    #[test]
    fn leaving_despawns_and_hands_back_the_save() {
        let (mailbox, mut players) = players();
        let mut world = World::new();
        assert!(mailbox.send(enter_note(character("Jacob", 42))).is_ok());
        assert!(mailbox.send(enter_note(character("Mckay", 43))).is_ok());
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
        assert!(mailbox.send(enter_note(character("Jacob", 42))).is_ok());
        assert!(mailbox.send(enter_note(character("Mckay", 43))).is_ok());
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
    fn who_stands_where_the_one_in_longest_first() {
        let (mailbox, mut players) = players();
        let mut world = World::new();
        // Mckay comes in first, though Jacob's row is the older.
        assert!(mailbox.send(enter_note(character("Mckay", 43))).is_ok());
        players.take_notes(&mut world);
        std::thread::sleep(Duration::from_millis(5));
        assert!(mailbox.send(enter_note(character("Jacob", 42))).is_ok());
        players.take_notes(&mut world);

        let jacob = world.all().into_iter()
            .find(|&entity| world.player_character(entity).is_some_and(|player| player.character_id == 42));
        let moved = jacob.and_then(|jacob| world.transform_mut(jacob))
            .map(|transform| transform.position = Vector3::new(1.5, 0.0, -1.5));
        assert!(moved.is_some());

        let an_hour_on = Instant::now() + Duration::from_secs(3_600);
        let standing = players.standing(&world, an_hour_on);
        let names: Vec<&str> = standing.iter().map(|character| character.name.as_str()).collect();
        assert_eq!(names, vec!["Mckay", "Jacob"]);
        assert_eq!(standing[0].block, [0, 0, 0]);
        assert_eq!(standing[1].block, [1, 0, -2]);
        assert!(standing[0].online > standing[1].online);
        assert!(standing[1].online >= Duration::from_secs(3_600));
    }

    #[test]
    fn the_saving_mark_is_waited_on_and_comes_off() {
        // A row id of its own, so the other tests' marks don't touch it.
        let id = 9_000_001;
        assert!(!saving(id));
        assert!(wait_until_saved(id, Duration::ZERO), "never marked: nothing to wait for");

        lock(&SAVING).insert(id);
        assert!(saving(id));
        assert!(!wait_until_saved(id, Duration::from_millis(20)), "still saving when the limit runs out");

        // Another thread lands the save while this one waits.
        let lands = conductor_tools::threads::spawn("test-save-lands", move || {
            std::thread::sleep(Duration::from_millis(20));
            saved(&[id]);
        });
        assert!(lands.is_ok());
        assert!(wait_until_saved(id, Duration::from_secs(5)));
        assert!(!saving(id));
    }

    #[test]
    fn leaving_when_the_gameclock_isnt_running_leaves_no_mark() {
        let id = 9_000_002;
        // No GameClock runs in the tests, so there's no mailbox.
        assert!(leave(id).is_err());
        assert!(!saving(id));
    }

    #[test]
    fn leaving_a_character_that_isnt_in_the_world_takes_its_mark_off() {
        let id = 9_000_003;
        let (mailbox, mut players) = players();
        let mut world = World::new();
        lock(&SAVING).insert(id);
        assert!(mailbox.send(Note::Leave(id)).is_ok());
        players.take_notes(&mut world);
        assert!(!saving(id));
    }

    #[test]
    fn a_blueprint_that_isnt_a_players_character_is_turned_away() {
        assert_eq!(character_id_of(&character("Jacob", 42)), Some(42));
        assert_eq!(character_id_of(&new_character("Jacob")), None, "no row behind it yet");
        let mut blueprint = new_character("Jacob");
        blueprint.remove(Kind::PlayerCharacter);
        assert_eq!(character_id_of(&blueprint), None);
        assert!(enter(blueprint, "u-1").is_err());
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
            assert!(mailbox.send(enter_note(character("Jacob", character_id))).is_ok());
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
