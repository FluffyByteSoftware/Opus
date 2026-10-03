//! File:       Opus/Conductor/dev/gameclock/src/movement.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Players walking their characters (protocol version 15), EverQuest's
//! way.  Jacob, 2026-10-03: "the client like has local authority and the
//! server kinda just periodically validates the client movement and pulls
//! it backward if it doesn't match to the last known good spots."  So a
//! player's client moves its own character the moment a key goes down,
//! and tells us where it went with a PlayerMoved: whenever which way it's
//! walking or facing changes, and every half second while it doesn't.
//! Networking leaves each one here with `moved()`, stamped with when it
//! came in, and the input check takes them all, in the order they came,
//! once a cycle.  A move that passes becomes the character's place (its
//! `Transform`, the last good spot), and the broadcast passes it on to
//! everybody who can see it; one that doesn't pulls the character back to
//! the last good spot, and the broadcast tells its player so with a
//! MoveCorrection.
//!
//! His earlier server "kept rubber banding": the client and the server
//! fought about where you were.  What's here is built against the four
//! usual reasons for that:
//!
//! - **Packets come in bunches.**  Two moves sent 100 ms apart can land
//!   5 ms apart, and judged by when they landed the second looks too
//!   fast.  So how far a character may have walked counts from its last
//!   good spot, with a block of slack let go without a word, and the
//!   tolerance (`movement_tolerance_blocks` in `game.cfg`, 16) before a
//!   pull-back.
//! - **A pull-back crosses moves already on their way.**  Every pull-back
//!   has a number, and every move carries the number of the last one its
//!   client had.  Until a move says it's had this one, moves are dropped
//!   (and the pull-back sent again), so the moves made before it can't
//!   pull the character back a second time.
//! - **Two sets of physics disagreeing.**  The client's Unity slides a
//!   capsule along a wall; the server doesn't try to do the same and
//!   argue.  It checks only what can't be argued: not faster than walking
//!   allows, not turned faster than `player.cfg` allows, not inside a
//!   block, not on ground the server hasn't got, not hanging in the air
//!   longer than a fall takes, and not walking into a character standing
//!   still (Jacob's "a", 2026-10-03: a still character blocks on the
//!   server too, so standing in a doorway shuts it; a moving one only on
//!   the screens).
//! - **The client hearing its own place from the broadcast.**  A player
//!   isn't sent their own character's moves (`view.rs`), and their client
//!   never takes its own place from the roll call; only a MoveCorrection
//!   moves it.
//!
//! Jacob's Warns (2026-10-03): "if your rotation is more than 90 deg off
//! that should flag a warning to the sys admin along with the being 1-16
//! blocks past a point expected to be at."  A Warn rings the bell, so it's
//! at most one a minute for each character, with a count of the ones held
//! back.
//!
//! The rest of the checks pull a character back with a Debug line: a
//! client a step out of line with the server's ground is nothing an admin
//! has to see.  None of this is timed yet: every move is checked against
//! everybody standing still, so a cycle's checks grow with players times
//! moves, a guess of well under a millisecond for a few hundred players.

use std::collections::HashMap;
use std::mem;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use conductor_gameworld::{Block, ChunkPos, Terrain};
use conductor_primlib::{Collider, Entity, Vector3, World};
use conductor_tools::constellations::{self, GAME, PLAYER};
use conductor_tools::scribe::{self, Channel};

use crate::Game;
use crate::players::Players;

/// How fast a character walks, in blocks a second (Jacob, 2026-10-03:
/// "we'll start with 4 blocks per second").  Fixed in code for now; each
/// client is told it as its character comes into the world.
pub const WALK_BLOCKS_PER_SECOND: f32 = 4.0;

/// The most time a move is given credit for since the character's last
/// good spot.  A client checks in every half second while it walks, so
/// two seconds is four check-ins lost in a row; a character that stood
/// still for an hour doesn't get an hour's walk to spend in one jump.
const MOST_TIME_COUNTED: Duration = Duration::from_secs(2);

/// How far past what walking allows is let go without a word.  Packets
/// that land in a bunch and a client's frames not lining up with ours
/// come to less than this.
const LET_GO_BLOCKS: f32 = 1.0;

/// How high a character steps up on its own: a block, so walking up a
/// hill isn't walking any further.
const STEP_UP: f32 = 1.0;

/// How far past what turning allows a character may face before it's
/// pulled back, with a Warn (Jacob: "more than 90 deg off").
const TURN_WARN_DEGREES: f32 = 90.0;

/// How long a walking character may go without a word from its client
/// before the server stops it, so everybody else doesn't watch it walk on
/// for ever.  Four check-ins missed.
const QUIET_FOR: Duration = Duration::from_secs(2);

/// How long a character may stand on nothing before it has to be falling.
/// A fall of 2 seconds is far more than a block, so a character that's
/// spent that long in the air and come down less than `FALLEN_AT_LEAST`
/// is hanging there, and goes back to the last spot it stood on.
const IN_THE_AIR_FOR: Duration = Duration::from_secs(2);
const FALLEN_AT_LEAST: f32 = 1.0;

/// How far one character's collider may sink into a still one's before
/// it counts as walking into it.  Two capsules touching on one screen can
/// be a hair inside each other on ours.
const OVERLAP_LET_GO: f32 = 0.25;

/// How fast a client may say its character is falling, in blocks a
/// second.  Only what everybody else's screens carry it along by; where
/// it lands is checked like any other move.
const FASTEST_FALL: f32 = 60.0;

/// The least time between two Warns about one character.
const WARN_EVERY_AT_MOST: Duration = Duration::from_secs(60);

/// What a player's client said about its own character: a PlayerMoved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moved {
    /// The character, by its row's id.
    pub character_id: i64,
    /// The move's own number, one higher each time.  An older one coming
    /// in late is dropped.
    pub number: u32,
    /// The number of the last MoveCorrection its client had, 0 for none.
    pub pull_backs_had: u32,
    /// Its feet, x, y and z in blocks.
    pub position: [f32; 3],
    /// Degrees about x, y and z, the way Unity has them.
    pub rotation: [f32; 3],
    /// Blocks a second along x, y and z.
    pub velocity: [f32; 3],
    /// When networking heard it.
    pub heard: Instant,
}

/// A character pulled back to its last good spot, for its player's
/// client: a MoveCorrection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PullBack {
    /// The pull-back's number for this character, from 1.  The client's
    /// moves say the last one they had.
    pub number: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
}

/// The moves waiting for the next input check.  `None` while the
/// GameClock is stopped.
static MOVES: Mutex<Option<Vec<Moved>>> = Mutex::new(None);

/// Opens the mailbox, empty, for a fresh start of the GameClock.
pub(crate) fn open() {
    *crate::lock(&MOVES) = Some(Vec::new());
}

/// Turns moves away from here on, and drops any still waiting.  For
/// `stop()`.
pub(crate) fn close() {
    crate::lock(&MOVES).take();
}

/// A player's client says where its character went.  Left for the next
/// input check, which judges it.  Comes straight back.  An error means the
/// GameClock isn't running.
pub fn moved(moved: Moved) -> Result<(), String> {
    match crate::lock(&MOVES).as_mut() {
        Some(moves) => {
            moves.push(moved);
            Ok(())
        }
        None => Err("the GameClock isn't running".to_string()),
    }
}

/// How fast a character turns, in degrees a second: `turn_degrees_per_second`
/// in `player.cfg`.  Each client is told it as its character comes in.
pub fn turn_degrees_per_second() -> f32 {
    constellations::number(&PLAYER, "turn_degrees_per_second") as f32
}

/// How far past what walking allows a move may land before it's pulled
/// back: `movement_tolerance_blocks` in `game.cfg`.
fn tolerance_blocks() -> f32 {
    constellations::number(&GAME, "movement_tolerance_blocks") as f32
}

/// The input check's part: every move that came in since the last cycle,
/// judged in the order it came.
pub(crate) fn take_moves(game: &mut Game) {
    let moves = match crate::lock(&MOVES).as_mut() {
        Some(moves) => mem::take(moves),
        None => Vec::new(),
    };
    game.movement.forget_the_gone(&game.players);
    let rules = Rules { walk: WALK_BLOCKS_PER_SECOND, turn: turn_degrees_per_second(),
                        tolerance: tolerance_blocks() };
    for moved in moves {
        game.movement.judge(&mut game.world, &game.players, &game.terrain, &rules, moved);
    }
}

/// The movement check's part: a character walking with nothing heard from
/// its client for `QUIET_FOR` is stopped where it was last good.
pub(crate) fn stop_the_quiet(game: &mut Game) {
    game.movement.stop_the_quiet(&mut game.world, &game.players, Instant::now());
}

/// The numbers a move is held to.  Read once a cycle, so a test can set
/// its own.
struct Rules {
    walk: f32,
    turn: f32,
    tolerance: f32,
}

/// What the GameClock keeps about each player's character's walking.
pub(crate) struct Movement {
    tracks: HashMap<i64, Track>,
    /// Pull-backs for the next broadcast to send, by character.
    to_send: HashMap<i64, PullBack>,
}

/// One character's walking.  Its last good spot is its `Transform`; this
/// is the rest.
struct Track {
    /// The last move taken, by its number.
    taken: u32,
    /// When the last move came in, taken or not.
    heard: Instant,
    /// When its place was last good: a move taken, or a pull-back.
    good_at: Instant,
    /// The last pull-back's number, 0 for none yet.
    pull_backs: u32,
    /// Whether moves are being dropped until one says it's had the last
    /// pull-back.
    waiting: bool,
    /// The last good spot it stood on something at, and which way it
    /// faced, to go back to if it hangs in the air.
    last_footing: ([f32; 3], [f32; 3]),
    /// When it left the ground, and how high it was then.
    in_the_air: Option<(Instant, f32)>,
    /// When the last Warn about it went out, and how many were held back
    /// since.
    warned: Option<Instant>,
    held_back: u32,
}

/// What became of a move.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Verdict {
    /// It stands.  How far past walking it was, if more than
    /// `LET_GO_BLOCKS`, for a Warn.
    Taken { over: Option<f32> },
    /// Back to the last good spot, a Warn with why.
    PulledBack(Why),
    /// Back to the last spot it stood on, a Debug line.
    BackToFooting,
}

/// Why a move was pulled back.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Why {
    /// Further than walking allows by more than the tolerance.
    TooFar(f32),
    /// Turned further than `player.cfg` allows by more than 90 degrees.
    TurnedTooFar(f32),
    /// Its numbers aren't numbers (a NaN or an infinity).
    NotANumber,
    /// The ground there isn't in the server's memory.
    NoGround,
    /// Inside a block.
    InsideABlock,
    /// Into a character standing still.
    IntoSomebody,
}

impl Why {
    /// Whether it rings the bell.  Only Jacob's two do.
    fn warns(self) -> bool {
        matches!(self, Why::TooFar(_) | Why::TurnedTooFar(_))
    }

    fn words(self) -> String {
        match self {
            Why::TooFar(over) => format!("{over:.1} blocks further than it could have walked"),
            Why::TurnedTooFar(over) => format!("turned {over:.0} degrees further than it could have"),
            Why::NotANumber => "a place that isn't a number".to_string(),
            Why::NoGround => "onto ground the server hasn't loaded".to_string(),
            Why::InsideABlock => "inside a block".to_string(),
            Why::IntoSomebody => "into a character standing still".to_string(),
        }
    }
}

impl Movement {
    pub(crate) fn new() -> Movement {
        Movement { tracks: HashMap::new(), to_send: HashMap::new() }
    }

    /// The pull-backs for this cycle's broadcast to send, taken.
    pub(crate) fn take_pull_backs(&mut self) -> HashMap<i64, PullBack> {
        mem::take(&mut self.to_send)
    }

    /// Forgets the characters that have left the world.
    fn forget_the_gone(&mut self, players: &Players) {
        self.tracks.retain(|id, _| players.entity_of(*id).is_some());
        self.to_send.retain(|id, _| players.entity_of(*id).is_some());
    }

    /// One move, judged, and the character's place changed to suit: the
    /// move's spot, or back to the last good one.
    fn judge(&mut self, world: &mut World, players: &Players, terrain: &Terrain, rules: &Rules, moved: Moved) {
        let id = moved.character_id;
        let Some(entity) = players.entity_of(id) else {
            return;
        };
        let Some(place) = world.transform(entity).copied() else {
            return;
        };
        let track = self.tracks.entry(id).or_insert_with(|| Track {
            taken: 0,
            heard: moved.heard,
            good_at: moved.heard,
            pull_backs: 0,
            waiting: false,
            last_footing: (array(place.position), array(place.rotation)),
            in_the_air: None,
            warned: None,
            held_back: 0,
        });
        track.heard = moved.heard;

        // UDP can hand them over out of order: an older move is already
        // overtaken by the one taken.
        if moved.number <= track.taken {
            return;
        }
        if track.waiting {
            if moved.pull_backs_had < track.pull_backs {
                // Made before its client heard of the pull-back.  Dropped,
                // and the pull-back sent again in case it was lost.
                self.to_send.insert(id, PullBack { number: track.pull_backs, position: array(place.position),
                                                   rotation: array(place.rotation) });
                return;
            }
            track.waiting = false;
        }
        track.taken = moved.number;

        let collider = world.collider(entity).copied().unwrap_or(Collider::CHARACTER);
        let around = Around { world: &*world, players, terrain, rules };
        let verdict = verdict(&around, track, id, collider, &place, &moved);
        let name = world.short_name(entity).map(|name| name.text.clone()).unwrap_or_default();
        match verdict {
            Verdict::Taken { over } => {
                if let Some(over) = over {
                    warn(track, moved.heard, &format!("{name} (character {id}) moved {over:.1} blocks further than \
                        it could have walked.  The move stands; past {} blocks it would have been pulled back.",
                        rules.tolerance));
                }
                if let Some(transform) = world.transform_mut(entity) {
                    transform.position = vector(moved.position);
                    transform.rotation = vector(moved.rotation);
                    transform.velocity = clamped(vector(moved.velocity), rules.walk);
                }
                track.good_at = moved.heard;
                let standing = stands_on_something(terrain, vector(moved.position), collider.reach());
                if standing {
                    track.last_footing = (moved.position, moved.rotation);
                    track.in_the_air = None;
                } else if track.in_the_air.is_none() {
                    track.in_the_air = Some((moved.heard, moved.position[1]));
                }
            }
            Verdict::PulledBack(why) => {
                let words = format!("{name} (character {id}) was pulled back: its client put it {}.", why.words());
                if why.warns() {
                    warn(track, moved.heard, &words);
                } else {
                    scribe::debug(Channel::Game, &words);
                }
                let spot = (array(place.position), array(place.rotation));
                self.pull_back(world, entity, id, spot, moved.heard);
            }
            Verdict::BackToFooting => {
                scribe::debug(Channel::Game, &format!("{name} (character {id}) was pulled back to where it last \
                    stood: it hung in the air for {} seconds.", IN_THE_AIR_FOR.as_secs()));
                let spot = track.last_footing;
                self.pull_back(world, entity, id, spot, moved.heard);
            }
        }
    }

    /// Puts a character back on `spot`, standing still, and leaves its
    /// player a MoveCorrection for the broadcast.  Its moves are dropped
    /// until one says it's had this.
    fn pull_back(&mut self, world: &mut World, entity: Entity, id: i64, spot: ([f32; 3], [f32; 3]), heard: Instant) {
        let Some(track) = self.tracks.get_mut(&id) else {
            return;
        };
        track.pull_backs += 1;
        track.waiting = true;
        track.good_at = heard;
        track.in_the_air = None;
        if let Some(transform) = world.transform_mut(entity) {
            transform.position = vector(spot.0);
            transform.rotation = vector(spot.1);
            transform.velocity = Vector3::default();
        }
        self.to_send.insert(id, PullBack { number: track.pull_backs, position: spot.0, rotation: spot.1 });
    }

    /// Stops every character still walking whose client has gone quiet.
    fn stop_the_quiet(&mut self, world: &mut World, players: &Players, now: Instant) {
        for (id, entity) in players.characters() {
            let Some(track) = self.tracks.get(&id) else {
                continue;
            };
            if now.saturating_duration_since(track.heard) < QUIET_FOR {
                continue;
            }
            let Some(transform) = world.transform_mut(entity) else {
                continue;
            };
            if !transform.is_still() {
                transform.velocity = Vector3::default();
                scribe::debug(Channel::Game, &format!("Character {id} was walking and its client went quiet for \
                    {} seconds.  Stopped where it was last good.", QUIET_FOR.as_secs()));
            }
        }
    }
}

/// What a move is judged against: the world, the players, the ground and
/// the rules.
struct Around<'a> {
    world: &'a World,
    players: &'a Players,
    terrain: &'a Terrain,
    rules: &'a Rules,
}

/// What becomes of `moved`, for character `id`, taking up `collider`, at
/// `place` with `track`.
fn verdict(around: &Around, track: &Track, id: i64, collider: Collider, place: &conductor_primlib::Transform,
           moved: &Moved) -> Verdict {
    let Around { world, players, terrain, rules } = *around;
    let all = moved.position.iter().chain(&moved.rotation).chain(&moved.velocity);
    if all.into_iter().any(|number| !number.is_finite()) {
        return Verdict::PulledBack(Why::NotANumber);
    }
    let from = place.position;
    let to = vector(moved.position);
    let seconds = moved.heard.saturating_duration_since(track.good_at).min(MOST_TIME_COUNTED).as_secs_f32();

    // A step up of a block is free (the client steps up on its own); any
    // more up counts as walking.  Down is falling, and a fall is as fast
    // as it is.  Climbing with nothing underfoot is caught by the time in
    // the air below.
    let walked = flat_distance(from, to) + (to.y - from.y - STEP_UP).max(0.0);
    let over = walked - rules.walk * seconds;
    if over > rules.tolerance {
        return Verdict::PulledBack(Why::TooFar(over));
    }
    let turned = turn_between(place.rotation.y, moved.rotation[1]);
    let turned_over = turned - rules.turn * seconds;
    if turned_over > TURN_WARN_DEGREES {
        return Verdict::PulledBack(Why::TurnedTooFar(turned_over));
    }

    let height = collider.height();
    if !ground_is_in(terrain, to, height) {
        return Verdict::PulledBack(Why::NoGround);
    }
    if inside_a_block(terrain, to, height) {
        return Verdict::PulledBack(Why::InsideABlock);
    }
    if walks_into_somebody_still(world, players, id, collider, from, to) {
        return Verdict::PulledBack(Why::IntoSomebody);
    }
    if let Some((since, from_height)) = track.in_the_air {
        let hanging = moved.heard.saturating_duration_since(since) >= IN_THE_AIR_FOR
            && from_height - to.y < FALLEN_AT_LEAST
            && !stands_on_something(terrain, to, collider.reach());
        if hanging {
            return Verdict::BackToFooting;
        }
    }
    Verdict::Taken { over: (over >= LET_GO_BLOCKS).then_some(over) }
}

/// A Warn about a character, unless one went out about it in the last
/// minute; then it's counted, and the next one says how many.
fn warn(track: &mut Track, now: Instant, words: &str) {
    let may = track.warned.is_none_or(|last| now.saturating_duration_since(last) >= WARN_EVERY_AT_MOST);
    if !may {
        track.held_back += 1;
        scribe::debug(Channel::Game, words);
        return;
    }
    let held_back = mem::take(&mut track.held_back);
    track.warned = Some(now);
    if held_back > 0 {
        scribe::warn(Channel::Game, &format!("{words}  ({held_back} more like it in the minute before.)"));
    } else {
        scribe::warn(Channel::Game, words);
    }
}

/// How far apart two places are, flat on the ground.
fn flat_distance(a: Vector3, b: Vector3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// How far apart two headings are, in degrees, 0 to 180, whichever way
/// round is shorter.
fn turn_between(from: f32, to: f32) -> f32 {
    let difference = (to - from).rem_euclid(360.0);
    if difference > 180.0 { 360.0 - difference } else { difference }
}

/// A velocity no faster along the ground than `walk`, and no faster up or
/// down than walking or `FASTEST_FALL`.  What a client says is only
/// what everybody else's screens carry its character along by.
fn clamped(velocity: Vector3, walk: f32) -> Vector3 {
    let flat = (velocity.x.powi(2) + velocity.z.powi(2)).sqrt();
    let scale = if flat > walk { walk / flat } else { 1.0 };
    Vector3::new(velocity.x * scale, velocity.y.clamp(-FASTEST_FALL, walk), velocity.z * scale)
}

/// The block x, y or z a position is in.
fn block_of(number: f32) -> i32 {
    number.floor() as i32
}

/// Whether the chunks a character `height` tall at `at` stands in are in
/// the server's memory.
fn ground_is_in(terrain: &Terrain, at: Vector3, height: f32) -> bool {
    [at.y, at.y + height].iter().all(|&y| {
        match ChunkPos::of_block(block_of(at.x), block_of(y), block_of(at.z)) {
            Some(pos) => terrain.chunk(pos).is_some(),
            // Above the top or below the bottom: no chunk, nothing to load.
            None => true,
        }
    })
}

/// Whether the middle of a character `height` tall at `at` is inside a
/// block: just over its feet, its middle and just under its head.  A
/// capsule's middle never is, walking as it should: its round side keeps
/// it half a block off a wall, and a step up puts its feet on top.
fn inside_a_block(terrain: &Terrain, at: Vector3, height: f32) -> bool {
    [0.25, height / 2.0, height - 0.25].iter().any(|&up| {
        terrain.block_at(block_of(at.x), block_of(at.y + up), block_of(at.z))
            .is_some_and(|block| block != Block::AIR)
    })
}

/// Whether a character `reach` round at `at` is standing on something:
/// a block just under its feet anywhere under its round bottom.  Ground
/// the server hasn't got counts as something, so a character is never
/// pulled back for what we don't know.
fn stands_on_something(terrain: &Terrain, at: Vector3, reach: f32) -> bool {
    let side = reach * 0.9;
    let corner = reach * 0.64;
    let under = [(0.0, 0.0), (side, 0.0), (-side, 0.0), (0.0, side), (0.0, -side),
                 (corner, corner), (corner, -corner), (-corner, corner), (-corner, -corner)];
    let y = block_of(at.y - 0.05);
    under.iter().any(|&(east, north)| {
        match terrain.block_at(block_of(at.x + east), y, block_of(at.z + north)) {
            Some(block) => block != Block::AIR,
            None => true,
        }
    })
}

/// Whether moving from `from` to `to` takes a character with `collider`
/// into one standing still: its collider sinks into theirs by more than
/// `OVERLAP_LET_GO`, and it's getting closer.  Moving away is always
/// allowed, so two characters put in the same spot (both at the spawn
/// point) can walk apart.
fn walks_into_somebody_still(world: &World, players: &Players, id: i64, collider: Collider, from: Vector3, to: Vector3)
    -> bool {
    players.characters().filter(|(other, _)| *other != id).any(|(_, entity)| {
        let Some(them) = world.transform(entity) else {
            return false;
        };
        if !them.is_still() {
            return false;
        }
        let theirs = world.collider(entity).copied().unwrap_or(Collider::CHARACTER);
        let side_by_side = to.y < them.position.y + theirs.height() && them.position.y < to.y + collider.height();
        let apart = flat_distance(to, them.position);
        side_by_side && apart < collider.reach() + theirs.reach() - OVERLAP_LET_GO
            && apart < flat_distance(from, them.position)
    })
}

fn vector(numbers: [f32; 3]) -> Vector3 {
    Vector3::new(numbers[0], numbers[1], numbers[2])
}

fn array(vector: Vector3) -> [f32; 3] {
    [vector.x, vector.y, vector.z]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::players::{Arrival, Note};
    use conductor_primlib::gameobject::new_character;
    use conductor_primlib::{Component, PlayerCharacter, Transform};
    use std::sync::mpsc;

    const RULES: Rules = Rules { walk: 4.0, turn: 450.0, tolerance: 16.0 };

    /// A world with these players' characters standing at these spots.
    fn world_of(spots: &[(i64, [f32; 3])]) -> (mpsc::Sender<Note>, Players, World) {
        let (mailbox, notes) = mpsc::channel();
        let mut players = Players::new(notes);
        let mut world = World::new();
        for &(id, spot) in spots {
            let mut character = new_character(&format!("Player{id}"));
            character.set(Component::PlayerCharacter(PlayerCharacter::new(7, id)));
            character.set(Component::Transform(Transform::at(vector(spot))));
            let arrival = Arrival { character, uuid: format!("u-{id}"), object: id as u32 };
            assert!(mailbox.send(Note::Enter(arrival)).is_ok());
        }
        players.take_notes(&mut world);
        (mailbox, players, world)
    }

    /// A move, heard `after` the start.
    fn moved(id: i64, number: u32, had: u32, position: [f32; 3], start: Instant, after_ms: u64) -> Moved {
        Moved { character_id: id, number, pull_backs_had: had, position, rotation: [0.0; 3],
                velocity: [4.0, 0.0, 0.0], heard: start + Duration::from_millis(after_ms) }
    }

    fn place(world: &World, players: &Players, id: i64) -> [f32; 3] {
        let entity = players.entity_of(id);
        entity.and_then(|entity| world.transform(entity)).map(|transform| array(transform.position))
            .unwrap_or([f32::NAN; 3])
    }

    /// Above the top of the world, where every block is open air and
    /// there are no chunks to load: the tests that aren't about the
    /// ground walk up here, on a terrain with nothing in it.
    const SKY: f32 = 400.0;

    #[test]
    fn a_walk_at_walking_speed_is_taken() {
        let (_mailbox, players, mut world) = world_of(&[(1, [0.5, SKY, 0.5])]);
        let terrain = Terrain::new();
        let mut movement = Movement::new();
        let start = Instant::now();
        // The first move sets the clock; the next is half a second on.
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 1, 0, [0.5, SKY, 0.5], start, 0));
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 2, 0, [2.5, SKY, 0.5], start, 500));
        assert_eq!(place(&world, &players, 1), [2.5, SKY, 0.5]);
        assert!(movement.take_pull_backs().is_empty());
    }

    #[test]
    fn two_moves_in_a_bunch_are_let_go() {
        let (_mailbox, players, mut world) = world_of(&[(1, [0.5, SKY, 0.5])]);
        let terrain = Terrain::new();
        let mut movement = Movement::new();
        let start = Instant::now();
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 1, 0, [0.5, SKY, 0.5], start, 0));
        // Sent 100 ms apart, landed 5 ms apart: 0.4 blocks in 5 ms.
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 2, 0, [0.9, SKY, 0.5], start, 100));
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 3, 0, [1.3, SKY, 0.5], start, 105));
        assert_eq!(place(&world, &players, 1), [1.3, SKY, 0.5]);
        assert!(movement.take_pull_backs().is_empty());
    }

    #[test]
    fn past_the_tolerance_is_pulled_back_and_later_moves_wait_for_it() {
        let (_mailbox, players, mut world) = world_of(&[(1, [0.5, SKY, 0.5])]);
        let terrain = Terrain::new();
        let mut movement = Movement::new();
        let start = Instant::now();
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 1, 0, [0.5, SKY, 0.5], start, 0));
        // 30 blocks in a quarter of a second.
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 2, 0, [30.5, SKY, 0.5], start, 250));
        assert_eq!(place(&world, &players, 1), [0.5, SKY, 0.5]);
        let pull_backs = movement.take_pull_backs();
        assert_eq!(pull_backs.get(&1).map(|pull_back| pull_back.number), Some(1));

        // Made before its client heard: dropped, and the pull-back again.
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 3, 0, [31.0, SKY, 0.5], start, 300));
        assert_eq!(place(&world, &players, 1), [0.5, SKY, 0.5]);
        assert_eq!(movement.take_pull_backs().get(&1).map(|pull_back| pull_back.number), Some(1));

        // Once it's had it, it walks on from where it was put.
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 4, 1, [1.0, SKY, 0.5], start, 400));
        assert_eq!(place(&world, &players, 1), [1.0, SKY, 0.5]);
        assert!(movement.take_pull_backs().is_empty());
    }

    #[test]
    fn a_few_blocks_too_far_stands() {
        let (_mailbox, players, mut world) = world_of(&[(1, [0.5, SKY, 0.5])]);
        let terrain = Terrain::new();
        let mut movement = Movement::new();
        let start = Instant::now();
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 1, 0, [0.5, SKY, 0.5], start, 0));
        // 5 blocks in a quarter of a second: 4 too far, under 16.
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 2, 0, [5.5, SKY, 0.5], start, 250));
        assert_eq!(place(&world, &players, 1), [5.5, SKY, 0.5]);
        assert!(movement.take_pull_backs().is_empty());
    }

    #[test]
    fn an_old_move_coming_in_late_is_dropped() {
        let (_mailbox, players, mut world) = world_of(&[(1, [0.5, SKY, 0.5])]);
        let terrain = Terrain::new();
        let mut movement = Movement::new();
        let start = Instant::now();
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 5, 0, [0.5, SKY, 0.5], start, 0));
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 4, 0, [0.0, SKY, 0.0], start, 100));
        assert_eq!(place(&world, &players, 1), [0.5, SKY, 0.5]);
    }

    #[test]
    fn turning_more_than_90_degrees_too_fast_is_pulled_back() {
        let (_mailbox, players, mut world) = world_of(&[(1, [0.5, SKY, 0.5])]);
        let terrain = Terrain::new();
        let mut movement = Movement::new();
        let start = Instant::now();
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 1, 0, [0.5, SKY, 0.5], start, 0));
        // A half turn in 100 ms, at 450 a second: 135 degrees too far.
        let mut spun = moved(1, 2, 0, [0.5, SKY, 0.5], start, 100);
        spun.rotation = [0.0, 180.0, 0.0];
        movement.judge(&mut world, &players, &terrain, &RULES, spun);
        assert_eq!(movement.take_pull_backs().len(), 1);

        // 60 degrees in 100 ms is 15 over: it stands.
        let mut turned = moved(1, 3, 1, [0.5, SKY, 0.5], start, 200);
        turned.rotation = [0.0, 60.0, 0.0];
        movement.judge(&mut world, &players, &terrain, &RULES, turned);
        assert!(movement.take_pull_backs().is_empty());
    }

    #[test]
    fn headings_go_the_short_way_round() {
        assert_eq!(turn_between(350.0, 10.0), 20.0);
        assert_eq!(turn_between(10.0, 350.0), 20.0);
        assert_eq!(turn_between(0.0, 180.0), 180.0);
        assert_eq!(turn_between(-90.0, 90.0), 180.0);
        assert_eq!(turn_between(720.0, 45.0), 45.0);
    }

    #[test]
    fn a_still_character_blocks_and_a_moving_one_doesnt() {
        let (_mailbox, players, mut world) = world_of(&[(1, [0.5, SKY, 0.5]), (2, [2.0, SKY, 0.5])]);
        let terrain = Terrain::new();
        let mut movement = Movement::new();
        let start = Instant::now();
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 1, 0, [0.5, SKY, 0.5], start, 0));
        // Into 2, standing still at 2.0: half a block apart is a whole
        // block's overlap.
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 2, 0, [1.5, SKY, 0.5], start, 250));
        assert_eq!(place(&world, &players, 1), [0.5, SKY, 0.5]);
        assert_eq!(movement.take_pull_backs().len(), 1);

        // 2 walking: 1 passes through it.
        if let Some(transform) = players.entity_of(2).and_then(|entity| world.transform_mut(entity)) {
            transform.velocity = Vector3::new(0.0, 0.0, 4.0);
        }
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 3, 1, [1.5, SKY, 0.5], start, 500));
        assert_eq!(place(&world, &players, 1), [1.5, SKY, 0.5]);
    }

    #[test]
    fn two_in_one_spot_can_walk_apart() {
        let (_mailbox, players, mut world) = world_of(&[(1, [0.5, SKY, 0.5]), (2, [0.5, SKY, 0.5])]);
        let terrain = Terrain::new();
        let mut movement = Movement::new();
        let start = Instant::now();
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 1, 0, [0.5, SKY, 0.5], start, 0));
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 2, 0, [0.9, SKY, 0.5], start, 100));
        assert_eq!(place(&world, &players, 1), [0.9, SKY, 0.5]);
    }

    #[test]
    fn a_character_with_no_ground_loaded_is_pulled_back() {
        let (_mailbox, players, mut world) = world_of(&[(1, [0.5, 1.0, 0.5])]);
        let terrain = Terrain::new();
        let mut movement = Movement::new();
        let start = Instant::now();
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 1, 0, [0.9, 1.0, 0.5], start, 0));
        assert_eq!(place(&world, &players, 1), [0.5, 1.0, 0.5]);
    }

    #[test]
    fn a_number_that_isnt_one_is_pulled_back() {
        let (_mailbox, players, mut world) = world_of(&[(1, [0.5, SKY, 0.5])]);
        let terrain = Terrain::new();
        let mut movement = Movement::new();
        let start = Instant::now();
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 1, 0, [f32::NAN, SKY, 0.5], start, 0));
        assert_eq!(place(&world, &players, 1), [0.5, SKY, 0.5]);
    }

    #[test]
    fn a_quiet_walker_is_stopped() {
        let (_mailbox, players, mut world) = world_of(&[(1, [0.5, SKY, 0.5])]);
        let terrain = Terrain::new();
        let mut movement = Movement::new();
        let start = Instant::now();
        movement.judge(&mut world, &players, &terrain, &RULES, moved(1, 1, 0, [0.5, SKY, 0.5], start, 0));
        let walking = |world: &World| players.entity_of(1).and_then(|entity| world.transform(entity))
            .is_some_and(|transform| !transform.is_still());
        assert!(walking(&world));
        movement.stop_the_quiet(&mut world, &players, start + Duration::from_millis(1000));
        assert!(walking(&world), "a second is nothing");
        movement.stop_the_quiet(&mut world, &players, start + QUIET_FOR);
        assert!(!walking(&world));
    }

    #[test]
    fn a_velocity_is_held_to_walking() {
        let fast = clamped(Vector3::new(30.0, 50.0, 40.0), 4.0);
        assert!((fast.x - 2.4).abs() < 0.001 && (fast.z - 3.2).abs() < 0.001);
        assert_eq!(fast.y, 4.0);
        assert_eq!(clamped(Vector3::new(1.0, -100.0, 0.0), 4.0), Vector3::new(1.0, -FASTEST_FALL, 0.0));
    }
}
