//! File:       Opus/Conductor/dev/gameclock/src/view.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! What each player's client is told is in the world around them
//! (protocol version 14).  Jacob, 2026-10-03: "The server will be the
//! authority, always on where the object actually is in the world.  The
//! client is just a dumb renderer."  So the client draws an object where
//! this says it is, its own character included, and never decides a
//! place of its own.
//!
//! A player sees every object within `view_chunks` of the column of
//! chunks their character stands in, the same reach as the chunks
//! they're sent, so nothing is shown standing on ground the client
//! hasn't got.  Only what changed goes out (Jacob's pick): an object
//! coming into view is sent whole, once (a Hydrate, his word); one that
//! moves is sent where it is now; one that goes out of view, or out of
//! the world, is sent gone.  UDP loses a packet now and then, so once a
//! second each player is sent a roll call of everything they're believed
//! to know and where it is.  The client drops whatever isn't on it, and
//! asks about a number it doesn't know (`ask_about()`): the asked
//! object is forgotten here, and the next broadcast sends it whole again,
//! or sends it gone.
//!
//! The broadcast check does the work: it reads the world once a cycle and
//! hands what each player is to be told, as plain data, to the function
//! in our slot (`set_view_sender()`, filled by networking as it starts),
//! which makes the packets and sends them.  The same shape as the chat
//! and `/who`, for the same reason: the GameClock can't call networking
//! itself.
//!
//! Every object the client is told about has a number, handed out when it
//! comes into the world (`next_object_number()`) and never used again in
//! the run, so the packets after the Hydrate carry four bytes instead of
//! its uuid.  Today the objects are players' characters; NPCs join them
//! when they're spawned (0.0.3 on WAYPOINTS.md).  Nothing moves yet, so
//! every velocity is 0 until movement gives it a place to live.
//!
//! The cost: every player against every object, every cycle.  A guess
//! until the timing test below is run (`view_of_five_hundred`).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::mem;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use conductor_gameworld::chunk::SIDE;
use conductor_primlib::{Entity, PrimitiveShape, World};

use crate::Game;
use crate::players::Players;

/// How many cycles between two roll calls: every 4th, so once a second.
const ROLL_CALL_EVERY: u64 = 4;

/// The number the next object to come into the world gets.  0 is never
/// handed out, so a client can use it for "none".
static NEXT_OBJECT: AtomicU32 = AtomicU32::new(1);

/// The asks about objects waiting for the next broadcast: whose character
/// asked, by its row's id, and the numbers.  `None` while the GameClock is
/// stopped.
static ASKED: Mutex<Option<Vec<(i64, Vec<u32>)>>> = Mutex::new(None);

/// What sends each player their news.  Networking's, handed in with
/// `set_view_sender()`.
static SENDER: Mutex<Option<fn(&[News])>> = Mutex::new(None);

/// Where an object is and where it's going: what a player is sent when it
/// moves, and on every roll call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    /// Its number, from `next_object_number()`.
    pub object: u32,
    /// x, y and z in blocks, y up.  A character's is its feet.
    pub position: [f32; 3],
    /// Which way it faces: degrees about x, y and z, the way Unity has
    /// them (Jacob: "full rotation if possible").
    pub rotation: [f32; 3],
    /// Blocks a second along x, y and z (Jacob: WASD, so a velocity).
    /// The client moves it along this every frame until told otherwise.
    /// Always 0 until movement.
    pub velocity: [f32; 3],
}

/// Everything a client needs to draw an object, sent once when it comes
/// into view: a Hydrate (Jacob: "a simple datagram that the client can use
/// to hydrate an actor with or an inanimate game object").
#[derive(Debug, Clone, PartialEq)]
pub struct Hydrate {
    pub motion: Motion,
    /// The game's name for it.
    pub uuid: String,
    /// True for anything Living: the client makes it an Actor, with its
    /// short name over its head.
    pub living: bool,
    /// Empty for none.
    pub short_name: String,
    /// 1, 1, 1 is as the model was made.
    pub scale: [f32; 3],
    /// The model's uuid, empty for none, and then the client draws
    /// `shape` instead.  No object has one until the model draw.
    pub model: String,
    /// The shape drawn without a model: 0 cube, 1 sphere, 2 capsule, 3
    /// cylinder, 4 plane, 5 quad (`shape_byte()`).
    pub shape: u8,
    /// What the model is doing ("idle"), empty for nothing.
    pub track: String,
}

/// What one player is to be told this cycle.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct News {
    /// The player's character, by its row's id; networking knows the
    /// address by it.
    pub viewer: i64,
    /// Objects come into view (or asked about), sent whole.
    pub hydrates: Vec<Hydrate>,
    /// Objects already known that moved since the last cycle.
    pub moved: Vec<Motion>,
    /// Objects gone out of view or out of the world, by number.
    pub gone: Vec<u32>,
    /// On a roll call: its number, and every object the player is
    /// believed to know, where it is now.
    pub roll_call: Option<(u32, Vec<Motion>)>,
}

impl News {
    fn is_empty(&self) -> bool {
        self.hydrates.is_empty() && self.moved.is_empty() && self.gone.is_empty() && self.roll_call.is_none()
    }
}

/// The next object's number.  Comes straight back, from any thread.
pub(crate) fn next_object_number() -> u32 {
    // Rust note: `fetch_add` adds one and hands back what it was before,
    // in one step, so two threads asking at once get two numbers.  Four
    // billion objects in one run would wrap it past 0; skipping 0 then
    // keeps "none" meaning none.
    let number = NEXT_OBJECT.fetch_add(1, Ordering::Relaxed);
    if number == 0 { NEXT_OBJECT.fetch_add(1, Ordering::Relaxed) } else { number }
}

/// Opens the mailbox, empty, for a fresh start of the GameClock.
pub(crate) fn open() {
    *crate::lock(&ASKED) = Some(Vec::new());
}

/// Turns asks away from here on, and drops any still waiting.  For
/// `stop()`.
pub(crate) fn close() {
    crate::lock(&ASKED).take();
}

/// A player's client doesn't know these objects (they were on a roll call
/// and it has never had them whole): leaves the ask for the next
/// broadcast, which sends each one whole, or sends it gone if the player
/// can't see it.  Comes straight back.  An error means the GameClock
/// isn't running.
pub fn ask_about(viewer: i64, objects: Vec<u32>) -> Result<(), String> {
    match crate::lock(&ASKED).as_mut() {
        Some(asked) => {
            asked.push((viewer, objects));
            Ok(())
        }
        None => Err("the GameClock isn't running".to_string()),
    }
}

/// Networking's function for sending each player their news.  Networking
/// calls this as it starts.
pub fn set_view_sender(send: fn(&[News])) {
    *crate::lock(&SENDER) = Some(send);
}

/// The broadcast check's part: what every player is to be told this
/// cycle, worked out and handed to networking to send.
pub(crate) fn broadcast(game: &mut Game) {
    let asked = match crate::lock(&ASKED).as_mut() {
        Some(asked) => mem::take(asked),
        None => Vec::new(),
    };
    let news = game.view.news(&game.world, &game.players, &asked, conductor_gameworld::view_chunks());
    if news.is_empty() {
        return;
    }
    let send = *crate::lock(&SENDER);
    if let Some(send) = send {
        send(&news);
    }
}

/// What each player's client has been told, kept between cycles.
pub(crate) struct View {
    /// What each player's client is believed to have, by their
    /// character's row id: the numbers of the objects it was sent whole.
    known: HashMap<i64, BTreeSet<u32>>,
    /// Where each object was last cycle, so only what changed goes out.
    last: HashMap<u32, Motion>,
    /// Cycles counted, for the roll call.
    cycles: u64,
    /// The last roll call's number.
    rolls: u32,
}

/// One object as this cycle sees it.
struct Seen {
    /// The player whose character it is, by its row's id.
    character_id: i64,
    entity: Entity,
    uuid: String,
    motion: Motion,
    /// The column of chunks it stands in, x and z.
    column: (i32, i32),
}

impl View {
    pub(crate) fn new() -> View {
        View { known: HashMap::new(), last: HashMap::new(), cycles: 0, rolls: 0 }
    }

    /// What every player is to be told this cycle, with `asked` (who asked
    /// about which objects) taken in, for players who see `reach` chunks
    /// each way.  Players with nothing to be told aren't on it.
    pub(crate) fn news(&mut self, world: &World, players: &Players, asked: &[(i64, Vec<u32>)], reach: i32)
        -> Vec<News> {
        self.cycles += 1;
        let roll_call = self.cycles % ROLL_CALL_EVERY == 0;
        if roll_call {
            self.rolls = self.rolls.wrapping_add(1);
        }

        // Every object once, and whether it moved since the last cycle.
        let seen: Vec<Seen> = players.objects().into_iter().map(|(character_id, entity, number, uuid)| {
            let motion = motion_of(world, entity, number);
            Seen { character_id, entity, uuid: uuid.to_string(), motion, column: column_of(motion.position) }
        }).collect();
        let moved: HashSet<u32> = seen.iter()
            .filter(|object| self.last.get(&object.motion.object).is_some_and(|last| *last != object.motion))
            .map(|object| object.motion.object)
            .collect();
        self.last = seen.iter().map(|object| (object.motion.object, object.motion)).collect();
        let by_number: HashMap<u32, &Seen> = seen.iter().map(|object| (object.motion.object, object)).collect();

        let mut asked_by: HashMap<i64, Vec<u32>> = HashMap::new();
        for (viewer, numbers) in asked {
            asked_by.entry(*viewer).or_default().extend(numbers);
        }

        let mut everybody = Vec::new();
        for viewer in &seen {
            let known = self.known.entry(viewer.character_id).or_default();
            let mut news = News { viewer: viewer.character_id, ..News::default() };
            let mut gone = BTreeSet::new();
            let sees = |object: &Seen| in_view(viewer.column, object.column, reach);

            // Asked about: forgotten, so it's sent whole below if it's in
            // view, or sent gone if it isn't (or isn't anything).
            for number in asked_by.get(&viewer.character_id).into_iter().flatten() {
                known.remove(number);
                if !by_number.get(number).is_some_and(|object| sees(*object)) {
                    gone.insert(*number);
                }
            }

            let mut in_view_now = BTreeSet::new();
            for object in seen.iter().filter(|object| sees(*object)) {
                let number = object.motion.object;
                in_view_now.insert(number);
                if known.insert(number) {
                    news.hydrates.push(hydrate(world, object));
                } else if moved.contains(&number) {
                    news.moved.push(object.motion);
                }
            }

            let left: Vec<u32> = known.difference(&in_view_now).copied().collect();
            for number in left {
                known.remove(&number);
                gone.insert(number);
            }
            news.gone = gone.into_iter().collect();

            if roll_call {
                let present = known.iter().filter_map(|number| by_number.get(number)).map(|object| object.motion);
                news.roll_call = Some((self.rolls, present.collect()));
            }
            if !news.is_empty() {
                everybody.push(news);
            }
        }

        // A player whose character has left the world isn't told anything
        // any more, and starts from nothing if they come back.
        let viewers: HashSet<i64> = seen.iter().map(|object| object.character_id).collect();
        self.known.retain(|viewer, _| viewers.contains(viewer));
        everybody
    }
}

/// Where an object is, which way it faces, and where it's going.
fn motion_of(world: &World, entity: Entity, number: u32) -> Motion {
    let (position, rotation) = match world.transform(entity) {
        Some(transform) => (transform.position, transform.rotation),
        None => Default::default(),
    };
    Motion {
        object: number,
        position: [position.x, position.y, position.z],
        rotation: [rotation.x, rotation.y, rotation.z],
        velocity: [0.0; 3],
    }
}

/// An object whole, for a Hydrate.
fn hydrate(world: &World, object: &Seen) -> Hydrate {
    let entity = object.entity;
    let scale = world.transform(entity).map(|transform| transform.scale)
        .map(|scale| [scale.x, scale.y, scale.z])
        .unwrap_or([1.0; 3]);
    Hydrate {
        motion: object.motion,
        uuid: object.uuid.clone(),
        living: world.is(entity, "Living"),
        short_name: world.short_name(entity).map(|name| name.text.clone()).unwrap_or_default(),
        scale,
        model: world.model(entity).map(|model| model.path.clone()).unwrap_or_default(),
        shape: shape_byte(world.primitive_shape(entity).copied().unwrap_or_default()),
        track: world.animator(entity).map(|animator| animator.current_track.clone()).unwrap_or_default(),
    }
}

/// A shape's byte on the wire.  Written out rather than `as u8`, so the
/// numbers can't move if the list in primlib is ever put in another order.
fn shape_byte(shape: PrimitiveShape) -> u8 {
    match shape {
        PrimitiveShape::Cube => 0,
        PrimitiveShape::Sphere => 1,
        PrimitiveShape::Capsule => 2,
        PrimitiveShape::Cylinder => 3,
        PrimitiveShape::Plane => 4,
        PrimitiveShape::Quad => 5,
    }
}

/// The column of chunks a position is in, x and z, the way networking
/// works out a player's for the chunks they're sent.
fn column_of(position: [f32; 3]) -> (i32, i32) {
    ((position[0].floor() as i32).div_euclid(SIDE), (position[2].floor() as i32).div_euclid(SIDE))
}

/// Whether something in column `there` is in the view of a player in
/// column `here`: within `reach` chunks east, west, north and south, the
/// same square as the chunks.
fn in_view(here: (i32, i32), there: (i32, i32), reach: i32) -> bool {
    (there.0 - here.0).abs() <= reach && (there.1 - here.1).abs() <= reach
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::players::{Arrival, Note};
    use conductor_primlib::gameobject::new_character;
    use conductor_primlib::{Component, PlayerCharacter, Vector3};
    use std::sync::mpsc;

    /// A world with these players' characters in it, by row id, each
    /// object numbered as its row id, at 0,0,0.
    fn world_of(ids: &[i64]) -> (mpsc::Sender<Note>, Players, World) {
        let (mailbox, notes) = mpsc::channel();
        let mut players = Players::new(notes);
        let mut world = World::new();
        for &id in ids {
            assert!(mailbox.send(Note::Enter(arrival(id))).is_ok());
        }
        players.take_notes(&mut world);
        (mailbox, players, world)
    }

    fn arrival(id: i64) -> Arrival {
        let mut character = new_character(&format!("Player{id}"));
        character.set(Component::PlayerCharacter(PlayerCharacter::new(7, id)));
        Arrival { character, uuid: format!("u-{id}"), object: id as u32 }
    }

    fn move_to(world: &mut World, players: &Players, id: i64, x: f32, z: f32) {
        let entity = players.objects().into_iter().find(|object| object.0 == id).map(|object| object.1);
        let moved = entity.and_then(|entity| world.transform_mut(entity))
            .map(|transform| transform.position = Vector3::new(x, 1.0, z));
        assert!(moved.is_some());
    }

    fn news_for(news: &[News], viewer: i64) -> Option<&News> {
        news.iter().find(|news| news.viewer == viewer)
    }

    #[test]
    fn everybody_in_view_is_sent_whole_once_themselves_included() {
        let (_mailbox, players, world) = world_of(&[1, 2]);
        let mut view = View::new();

        let first = view.news(&world, &players, &[], 8);
        for viewer in [1, 2] {
            let news = news_for(&first, viewer).unwrap();
            let mut numbers: Vec<u32> = news.hydrates.iter().map(|hydrate| hydrate.motion.object).collect();
            numbers.sort();
            assert_eq!(numbers, vec![1, 2]);
        }
        let jacob = &news_for(&first, 1).unwrap().hydrates.iter().find(|h| h.motion.object == 1).unwrap();
        assert_eq!(jacob.uuid, "u-1");
        assert_eq!(jacob.short_name, "Player1");
        assert!(jacob.living);
        assert_eq!(jacob.shape, 2, "a character is a capsule");
        assert_eq!(jacob.scale, [1.0; 3]);
        assert_eq!(jacob.model, "");

        // Nothing changed, and it isn't a roll call: nobody is told anything.
        assert!(view.news(&world, &players, &[], 8).is_empty());
    }

    #[test]
    fn a_move_goes_out_to_whoever_sees_it() {
        let (_mailbox, players, mut world) = world_of(&[1, 2]);
        let mut view = View::new();
        view.news(&world, &players, &[], 8);

        move_to(&mut world, &players, 2, 5.5, -3.0);
        let news = view.news(&world, &players, &[], 8);
        for viewer in [1, 2] {
            let moved = &news_for(&news, viewer).unwrap().moved;
            assert_eq!(moved.len(), 1);
            assert_eq!(moved[0].object, 2);
            assert_eq!(moved[0].position, [5.5, 1.0, -3.0]);
        }
    }

    #[test]
    fn walking_out_of_view_is_gone_and_back_in_is_whole_again() {
        let (_mailbox, players, mut world) = world_of(&[1, 2]);
        let mut view = View::new();
        view.news(&world, &players, &[], 1);

        // Two chunks east is past a reach of 1.
        move_to(&mut world, &players, 2, 64.0, 0.0);
        let news = view.news(&world, &players, &[], 1);
        assert_eq!(news_for(&news, 1).unwrap().gone, vec![2]);
        assert_eq!(news_for(&news, 2).unwrap().gone, vec![1]);

        move_to(&mut world, &players, 2, 33.0, 0.0);
        let news = view.news(&world, &players, &[], 1);
        assert_eq!(news_for(&news, 1).unwrap().hydrates.len(), 1);
        assert_eq!(news_for(&news, 1).unwrap().hydrates[0].motion.object, 2);
    }

    #[test]
    fn leaving_the_world_is_gone_to_everybody_who_knew() {
        let (mailbox, mut players, mut world) = world_of(&[1, 2]);
        let mut view = View::new();
        view.news(&world, &players, &[], 8);

        assert!(mailbox.send(Note::Leave(2)).is_ok());
        players.take_notes(&mut world);
        let news = view.news(&world, &players, &[], 8);
        assert_eq!(news_for(&news, 1).unwrap().gone, vec![2]);
        assert!(news_for(&news, 2).is_none(), "the one who left is told nothing");
        assert!(!view.known.contains_key(&2));
    }

    #[test]
    fn every_fourth_cycle_is_a_roll_call_of_what_each_knows() {
        let (_mailbox, players, world) = world_of(&[1, 2]);
        let mut view = View::new();
        for _ in 0..3 {
            assert!(view.news(&world, &players, &[], 8).iter().all(|news| news.roll_call.is_none()));
        }
        let news = view.news(&world, &players, &[], 8);
        let (roll, present) = news_for(&news, 1).unwrap().roll_call.clone().unwrap();
        assert_eq!(roll, 1);
        assert_eq!(present.iter().map(|motion| motion.object).collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn an_object_asked_about_is_sent_whole_again_or_gone() {
        let (_mailbox, players, world) = world_of(&[1, 2]);
        let mut view = View::new();
        view.news(&world, &players, &[], 8);

        // 2 is there; 99 never was.
        let news = view.news(&world, &players, &[(1, vec![2, 99])], 8);
        let mine = news_for(&news, 1).unwrap();
        assert_eq!(mine.hydrates.iter().map(|h| h.motion.object).collect::<Vec<_>>(), vec![2]);
        assert_eq!(mine.gone, vec![99]);
        assert!(news_for(&news, 2).is_none(), "nobody else asked");
    }

    #[test]
    fn a_position_is_in_the_column_below_it() {
        assert_eq!(column_of([0.5, 1.0, 0.5]), (0, 0));
        assert_eq!(column_of([31.9, 0.0, 32.0]), (0, 1));
        assert_eq!(column_of([-0.1, 0.0, -32.0]), (-1, -1));
        assert_eq!(column_of([-32.1, 0.0, 0.0]), (-2, 0));
    }

    #[test]
    fn the_shapes_keep_their_bytes() {
        assert_eq!(shape_byte(PrimitiveShape::Cube), 0);
        assert_eq!(shape_byte(PrimitiveShape::Capsule), 2);
        assert_eq!(shape_byte(PrimitiveShape::Quad), 5);
    }

    #[test]
    fn object_numbers_are_handed_out_once_and_never_zero() {
        let first = next_object_number();
        let second = next_object_number();
        assert!(first > 0 && second > first);
    }

    /// How long one cycle's view takes for 500 players all standing in
    /// sight of each other, every one of them moved: the worst a cycle of
    /// them can be.  Timed, so it runs by hand, optimized:
    /// `cargo test --release -p conductor-gameclock -- --ignored --nocapture view_of_five_hundred`
    #[test]
    #[ignore]
    fn view_of_five_hundred() {
        let ids: Vec<i64> = (1..=500).collect();
        let (_mailbox, players, mut world) = world_of(&ids);
        let mut view = View::new();
        view.news(&world, &players, &[], 8);
        for &id in &ids {
            move_to(&mut world, &players, id, id as f32 / 10.0, 0.0);
        }

        let began = std::time::Instant::now();
        let news = view.news(&world, &players, &[], 8);
        let took = began.elapsed();
        assert_eq!(news.len(), 500);
        println!("One cycle's view of 500 players, every one moved, took {:.2} ms of the broadcast's 50.",
                 took.as_secs_f64() * 1000.0);
    }
}
