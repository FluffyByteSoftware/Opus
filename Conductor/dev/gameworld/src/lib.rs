//! File:       Opus/Conductor/dev/gameworld/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! GameWorld: the ground the game is played on.  The world is the total
//! sum of everything (Jacob, 2026-09-30), cut into regions (a region is a
//! zone is a biome), regions into chunks, chunks into blocks.  A block is
//! 1 m a side, Minecraft's size, and a player 2 blocks tall.  The world is
//! `world_size` in `game.cfg` times 1024 blocks a side (16 to start, so
//! -8192 to 8191 each way) with 0,0,0 in the middle, and -32 to +319 up
//! and down, seamless, and starts as two regions: Alpha to
//! the west, flat, and Omega to the east, in rolling hills.
//! `design/world.md` has the whole of it.
//!
//! This is a server piece with a thread of its own, `gameworld`.  On START
//! SERVER it reads `Content/world/region.map`, or makes the world if there
//! isn't one.  Then it waits for the GameClock to ask for chunks, and for
//! each one reads its file if it has one, or builds it from its region's
//! ground if it doesn't, and sends it back.  That's the slow part, so it's
//! here and not on the GameClock's thread.  The chunks themselves live in
//! the GameClock's `Terrain` (`terrain.rs`).
//!
//! What's where on disk, all under `Content/world/`, none of it in git:
//!
//! - `region.map`: which region every chunk is in (`regionmap.rs`).
//! - `Regions/<Region>/`: a region's folder.  Omega's has `omega.heights`,
//!   its hills (`heights.rs`), and a chunk somebody changed gets its own
//!   file in its region's folder (`chunk.rs`), which always wins over the
//!   ground it was built from.
//! - `simple_overworld.map`: the world's rough shape, for the client to
//!   draw the distance with (`overworld.rs`).  Written before the first
//!   chunk goes out, so the door doesn't open without it.  Its bytes are
//!   kept in memory until STOP SERVER (`overworld_map()`), for networking
//!   to send each player at PLAY.
//!
//! The thread also squeezes every chunk it hands over (`squeeze.rs`) and
//! keeps it, squeezed, for networking to send players (protocol version
//! 12, `squeezed()`).  A chunk a player asks for that nobody has asked
//! for yet is read or built here for that alone.  The squeezed chunks are
//! kept until STOP SERVER, and every player is sent the same bytes: each
//! one is squeezed once a run, never once a player.
//!
//! Saving changed chunks, on STOP SERVER and every so often, isn't built:
//! it comes with the first thing that changes one (`design/world.md`).
//! When it does, the GameClock's copy of a changed chunk comes back here
//! and is squeezed again, so what players are sent keeps up with it.

pub mod block;
mod build;
mod bytes;
pub mod chunk;
pub mod heights;
mod make;
pub mod noise;
pub mod overworld;
pub mod regionmap;
pub mod spawn;
pub mod squeeze;
pub mod terrain;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use conductor_tools::constellations::{self, GAME};
use conductor_tools::diskman;
use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};
use conductor_tools::threads;

// Rust note: these let the rest of Conductor write
// `conductor_gameworld::Terrain` instead of reaching into the files.
pub use block::Block;
pub use chunk::{Chunk, ChunkPos};
pub use spawn::SPAWN_POINTS;
pub use terrain::{Loaded, Terrain};

use regionmap::CHUNKS_PER_SIZE;

/// How long the thread waits for a job before checking in anyway.  The
/// Services tab calls a service stuck after 5 seconds of quiet.
const CHECK_IN_EVERY: Duration = Duration::from_secs(1);

/// What GameWorld can be asked to do.
enum Job {
    /// Read or build the chunk at `pos`, and send it back on `reply`.
    Load { pos: ChunkPos, reply: Sender<Loaded> },
    /// Read or build the chunk at `pos` and squeeze it, for a player who
    /// asked for it before anything else did.
    Squeeze { pos: ChunkPos },
    /// The y of the highest block in column x, z that isn't AIR, for a
    /// spawn point (`spawn.rs`), sent back on `reply`.
    Top { x: i32, z: i32, reply: Sender<Result<i32, String>> },
}

/// The chunks squeezed for players, and the ones on their way.
struct Squeezed {
    /// Ready to send, by place.
    ready: HashMap<ChunkPos, Arc<Vec<u8>>>,
    /// In GameWorld's mailbox, so a second ask doesn't put it there twice.
    coming: HashSet<ChunkPos>,
    /// Couldn't be had this run (a bad file, the world unread), so nobody
    /// asks GameWorld again.
    missing: HashSet<ChunkPos>,
}

/// What `squeezed()` has for a chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sendable {
    /// The chunk, squeezed (`squeeze.rs`).
    Ready(Arc<Vec<u8>>),
    /// On its way: ask again in a moment.
    Coming,
    /// Not this run: outside the world, a file that doesn't read right,
    /// or no world at all.  GameWorld has said why in the log.
    Missing,
}

// The mailbox the jobs go in.  Dropping the Sender is the signal to stop,
// the same way the GameClock does it.
static MAILBOX: Mutex<Option<Sender<Job>>> = Mutex::new(None);
static GAMEWORLD: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Set by `stop()`, so a world half made gives up instead of making the
/// stop wait for the rest of it.
static STOPPING: AtomicBool = AtomicBool::new(false);

/// The simple overworld map's bytes, as they are on the disk, once the
/// thread has made sure of it, until STOP SERVER.  `None` before then.
/// Networking sends it to every player at PLAY (protocol version 11).
static OVERWORLD_MAP: Mutex<Option<Arc<Vec<u8>>>> = Mutex::new(None);

/// The squeezed chunks, from START SERVER to STOP SERVER.  `None` while
/// GameWorld is stopped.
// Rust note: a HashMap can't be made before the program starts, so the
// lot starts as `None` and `start()` puts an empty one in.
static SQUEEZED: Mutex<Option<Squeezed>> = Mutex::new(None);

/// Reads `game.cfg` and starts GameWorld's thread.  It comes straight back;
/// the world is read (or made) on the thread.  The launcher calls this on
/// every START SERVER, before the GameClock, so the GameClock has somebody
/// to ask for chunks.
pub fn start() {
    if lock(&GAMEWORLD).as_ref().is_some_and(|handle| !handle.is_finished()) {
        scribe::warn(Channel::Game, "GameWorld was asked to start while it's already running.  The running one \
            stands.");
        return;
    }

    constellations::load(&GAME);
    STOPPING.store(false, Ordering::SeqCst);
    // Last run's map may be another world's.
    lock(&OVERWORLD_MAP).take();
    *lock(&SQUEEZED) = Some(Squeezed { ready: HashMap::new(), coming: HashSet::new(), missing: HashSet::new() });
    services::set(services::GAMEWORLD, State::Starting, "Reading the world.");

    let (mailbox, jobs) = mpsc::channel();
    match threads::spawn("gameworld", move || run(jobs)) {
        Ok(handle) => {
            *lock(&MAILBOX) = Some(mailbox);
            *lock(&GAMEWORLD) = Some(handle);
        }
        Err(e) => {
            scribe::error_with(Channel::Game, &e, "GameWorld couldn't start its thread.  There's no ground this \
                run.");
            services::set(services::GAMEWORLD, State::Stopped, &format!("Couldn't start its thread: {e}"));
        }
    }
}

/// Stops GameWorld and waits for its thread to end.  The jobs already in
/// its mailbox are finished first.  A world being made is given up on, and
/// made from the start on the next START SERVER.
pub fn stop() {
    STOPPING.store(true, Ordering::SeqCst);
    lock(&MAILBOX).take();

    let handle = lock(&GAMEWORLD).take();
    if let Some(handle) = handle {
        if handle.join().is_err() {
            scribe::error(Channel::Game, "GameWorld's thread had already died.");
        }
    }
    lock(&OVERWORLD_MAP).take();
    lock(&SQUEEZED).take();
}

/// The simple overworld map's bytes, the whole file, once GameWorld has
/// made sure of it on START SERVER.  `None` until then, and after STOP
/// SERVER.  It's always there by the time the door opens: the GameClock is
/// never ready without GameWorld's chunks, and GameWorld hands out none
/// before the map is in.
// Rust note: an `Arc` is shared, not copied: every caller gets a pointer
// to the same 16 MB, and it's let go when the last one is done with it.
pub fn overworld_map() -> Option<Arc<Vec<u8>>> {
    lock(&OVERWORLD_MAP).clone()
}

/// The chunk at `pos`, squeezed for sending to a player, if it's ready.
/// Comes straight back: one that isn't ready yet is put in GameWorld's
/// mailbox (once, however often it's asked for) and is `Coming`, to be
/// asked for again in a moment.  Networking calls this from its UDP
/// thread, once it has checked the player may see that chunk.
pub fn squeezed(pos: ChunkPos) -> Sendable {
    if !in_world(pos) {
        return Sendable::Missing;
    }
    let mut squeezed = lock(&SQUEEZED);
    let Some(squeezed) = squeezed.as_mut() else {
        return Sendable::Missing;
    };
    if let Some(bytes) = squeezed.ready.get(&pos) {
        return Sendable::Ready(bytes.clone());
    }
    if squeezed.missing.contains(&pos) {
        return Sendable::Missing;
    }
    if squeezed.coming.contains(&pos) {
        return Sendable::Coming;
    }
    let asked = match lock(&MAILBOX).as_ref() {
        Some(mailbox) => mailbox.send(Job::Squeeze { pos }).is_ok(),
        None => false,
    };
    if !asked {
        return Sendable::Missing;
    }
    squeezed.coming.insert(pos);
    Sendable::Coming
}

/// Whether `pos` is inside the world: `world_size` from `game.cfg`, the
/// same shape `RegionMap::first()` makes, and a row from 0 to 10.  A
/// player near the edge of the world has chunks in their view that
/// aren't in it.
pub fn in_world(pos: ChunkPos) -> bool {
    let across = world_size() * CHUNKS_PER_SIZE;
    let west = -across / 2;
    (west..west + across).contains(&pos.x) && (west..west + across).contains(&pos.z) && pos.row < chunk::ROWS
}

/// Keeps a chunk squeezed for players, or notes it can't be had.  On
/// GameWorld's thread, as each chunk is read or built.
fn keep_squeezed(pos: ChunkPos, chunk: &Result<Chunk, String>) {
    // The squeezing is done before the lock is taken, so a player's ask
    // never waits on it.
    let bytes = chunk.as_ref().ok().map(|chunk| Arc::new(squeeze::squeeze(chunk)));
    let mut squeezed = lock(&SQUEEZED);
    let Some(squeezed) = squeezed.as_mut() else {
        return;
    };
    squeezed.coming.remove(&pos);
    match bytes {
        Some(bytes) => {
            squeezed.ready.insert(pos, bytes);
        }
        None => {
            squeezed.missing.insert(pos);
        }
    }
}

/// Whether the chunk at `pos` is already squeezed, so a player's ask that
/// came in behind the GameClock's doesn't read it twice.
fn already_squeezed(pos: ChunkPos) -> bool {
    lock(&SQUEEZED).as_ref().is_some_and(|squeezed| squeezed.ready.contains_key(&pos))
}

/// How many chunks are squeezed for players, for the Services tab.
fn how_many_squeezed() -> usize {
    lock(&SQUEEZED).as_ref().map_or(0, |squeezed| squeezed.ready.len())
}

/// How many chunks each way around a player the server loads and sends:
/// `view_chunks` in `game.cfg`.  4 to start, "but it might need to be 8"
/// (Jacob, 2026-09-30).
pub fn view_chunks() -> i32 {
    constellations::number(&GAME, "view_chunks") as i32
}

/// How big the world is, in steps of 1024 blocks a side: `world_size` in
/// `game.cfg`, 2 to 32 (Jacob, 2026-10-01).  16 to start.
pub fn world_size() -> i32 {
    constellations::number(&GAME, "world_size") as i32
}

/// Asks GameWorld for the chunk at `pos`, to be sent back on `reply`.
/// Comes straight back.  False if GameWorld isn't running.  `Terrain` is
/// what calls this.
fn ask(pos: ChunkPos, reply: &Sender<Loaded>) -> bool {
    match lock(&MAILBOX).as_ref() {
        Some(mailbox) => mailbox.send(Job::Load { pos, reply: reply.clone() }).is_ok(),
        None => false,
    }
}

/// Asks GameWorld for the top of column x, z, to be sent back on `reply`.
/// Comes straight back.  False if GameWorld isn't running.
fn ask_top(x: i32, z: i32, reply: Sender<Result<i32, String>>) -> bool {
    match lock(&MAILBOX).as_ref() {
        Some(mailbox) => mailbox.send(Job::Top { x, z, reply }).is_ok(),
        None => false,
    }
}

/// True once `stop()` has been called, for the long jobs to look at.
fn stopping() -> bool {
    STOPPING.load(Ordering::SeqCst)
}

/// The lock idiom, for the statics above.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------
// Where things are
// ---------------------------------------------------------------------------

/// `Content/world/`.
fn world_dir() -> PathBuf {
    constellations::content_dir().join("world")
}

/// `Content/world/region.map`.
fn region_map_path() -> PathBuf {
    world_dir().join("region.map")
}

/// `Content/world/simple_overworld.map`.
fn overworld_path() -> PathBuf {
    world_dir().join("simple_overworld.map")
}

/// `Content/world/Regions/<Region>/`.
fn region_dir(region: &str) -> PathBuf {
    world_dir().join("Regions").join(region)
}

/// `Content/world/Regions/<Region>/<region>.heights`.
fn heights_path(region: &str) -> PathBuf {
    region_dir(region).join(format!("{}.heights", region.to_lowercase()))
}

// ---------------------------------------------------------------------------
// The thread
// ---------------------------------------------------------------------------

/// What the thread knows about the world once it's read: the map, and the
/// heights file of each region that has one, by the region's number.
struct Shape {
    map: regionmap::RegionMap,
    heights: Vec<Option<heights::Heights>>,
}

/// GameWorld's thread: read the world (or make it), then answer jobs until
/// `stop()` drops the mailbox.
fn run(jobs: Receiver<Job>) {
    // If the world can't be read, the thread stays up anyway and turns
    // every ask away, so the Services tab keeps saying why.  The GameClock
    // is never ready without its chunks, so the door stays shut.
    let shape = match read_world().and_then(with_overworld) {
        Ok(shape) => {
            services::set(services::GAMEWORLD, State::Running, "The world is read.  No chunks asked for yet.");
            scribe::info(Channel::Game, "GameWorld is up.");
            Some(shape)
        }
        // Stopped part way through making the world is no fault: the
        // next START SERVER makes it from the start.
        Err(why) if stopping() => {
            scribe::info(Channel::Game, &format!("GameWorld stopped before it was done: {why}."));
            None
        }
        Err(why) => {
            scribe::error(Channel::Game, &format!("GameWorld can't read the world: {why}.  There's no ground this \
                run."));
            services::set(services::GAMEWORLD, State::Trouble, &format!("Can't read the world: {why}."));
            None
        }
    };

    let mut handed = 0u64;
    let mut from_files = 0u64;
    loop {
        match jobs.recv_timeout(CHECK_IN_EVERY) {
            Ok(Job::Load { pos, reply }) => {
                let chunk = match &shape {
                    Some(shape) => load(shape, pos, &mut from_files),
                    None => Err("GameWorld couldn't read the world".to_string()),
                };
                if chunk.is_ok() {
                    handed += 1;
                }
                keep_squeezed(pos, &chunk);
                // Nobody left to take it is fine: the GameClock stopped
                // first.
                let _ = reply.send(Loaded { pos, chunk });
                if shape.is_some() {
                    say_how_many(handed, from_files);
                }
            }
            // One the GameClock's ask got to first is squeezed already.
            Ok(Job::Squeeze { pos }) if !already_squeezed(pos) => {
                let chunk = match &shape {
                    Some(shape) => load(shape, pos, &mut from_files),
                    None => Err("GameWorld couldn't read the world".to_string()),
                };
                keep_squeezed(pos, &chunk);
                if shape.is_some() {
                    say_how_many(handed, from_files);
                }
            }
            Ok(Job::Squeeze { .. }) => {}
            Ok(Job::Top { x, z, reply }) => {
                let top = match &shape {
                    Some(shape) => spawn::top_of(x, z, |pos| load(shape, pos, &mut from_files)),
                    None => Err("GameWorld couldn't read the world".to_string()),
                };
                // Nobody left to take it is fine: the asker gave up waiting.
                let _ = reply.send(top);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        services::seen(services::GAMEWORLD);
    }

    scribe::info(Channel::Game, &format!("GameWorld has stopped, after handing over {handed} chunks."));
    services::set(services::GAMEWORLD, State::Stopped, "Shut down.");
}

/// What the Services tab says while GameWorld is answering asks.
fn say_how_many(handed: u64, from_files: u64) {
    services::set(services::GAMEWORLD, State::Running, &format!("{handed} chunks handed over since START SERVER, \
        {from_files} read from their own files; {} squeezed for players.", how_many_squeezed()));
}

/// Makes sure the simple overworld map is there before the first chunk
/// goes out, keeps its bytes for `overworld_map()`, and hands the world
/// back.  Without it there's nothing to send a player at PLAY, so it's the
/// same as no world: every ask is turned away and the door stays shut
/// (Jacob, 2026-10-03: "keep the door shut").  The Error tells the admin
/// which file to delete.  Telling a player whose client can't get the map
/// to start over is Ensemble's half.
fn with_overworld(shape: Shape) -> Result<Shape, String> {
    let bytes = overworld::ensure(&shape.map, &shape.heights).map_err(|why| {
        // A stop part way is no fault: the next START SERVER makes it.
        if stopping() {
            return why;
        }
        format!("the simple overworld map isn't ready ({why}).  The door stays shut this run.  Delete {} if it's \
            there, then STOP SERVER and START SERVER", overworld_path().display())
    })?;
    *lock(&OVERWORLD_MAP) = Some(bytes);
    Ok(shape)
}

/// Reads `region.map` and the heights files, or makes the world if there's
/// no map.  A world that isn't `world_size` is deleted and made again at
/// the new size (Jacob, 2026-10-01: "Delete the world on disk and
/// recreate").  A heights file that has gone missing is made again from
/// the map's seed, the same as it was.
fn read_world() -> Result<Shape, String> {
    let size = world_size();
    let map_path = region_map_path();
    let map = match diskman::read(&map_path).wait() {
        // A world made by an older Conductor is turned away here too (its
        // version is wrong), so the way out goes with it.
        Ok(bytes) => {
            let map = regionmap::RegionMap::from_bytes(&bytes)
                .map_err(|why| format!("{} isn't right: {why}.  To make a new world, stop the server and delete {}",
                                       map_path.display(), world_dir().display()))?;
            if map.is_size(size) {
                map
            } else {
                scribe::info(Channel::Game, &format!("The world on disk is {} blocks a side, and world_size {size} \
                    in game.cfg asks for {}.  GameWorld is deleting it and making a new one.", map.blocks_across(),
                    size * 1024));
                services::set(services::GAMEWORLD, State::Starting, "Deleting the old world: world_size changed.");
                delete_world()?;
                make::world(size)?
            }
        }
        Err(e) if e.is_not_found() => make::world(size)?,
        Err(e) => return Err(format!("{} can't be read: {e}", map_path.display())),
    };

    let mut every_heights = Vec::with_capacity(map.regions.len());
    for (number, region) in map.regions.iter().enumerate() {
        if region.ground != regionmap::Ground::Heights {
            every_heights.push(None);
            continue;
        }
        let path = heights_path(&region.name);
        let contents = match diskman::read(&path).wait() {
            Ok(contents) => contents,
            Err(e) if e.is_not_found() => {
                scribe::warn(Channel::Game, &format!("{} is missing.  GameWorld is making it again from the seed in \
                    region.map, so {}'s hills come back as they were.", path.display(), region.name));
                make::heights_for(&map, number)?;
                diskman::read(&path).wait().map_err(|e| format!("{} can't be read: {e}", path.display()))?
            }
            Err(e) => return Err(format!("{} can't be read: {e}", path.display())),
        };
        let read = heights::Heights::from_contents(contents)
            .map_err(|why| format!("{} isn't right: {why}", path.display()))?;
        if read.seed() != map.seed {
            return Err(format!("{} was made from another world's seed than region.map's", path.display()));
        }
        every_heights.push(Some(read));
    }
    scribe::debug(Channel::Game, &format!("GameWorld read {} and its {} regions.", map_path.display(),
                                          map.regions.len()));
    Ok(Shape { map, heights: every_heights })
}

/// Deletes everything under `Content/world/`, for a world whose size no
/// longer matches `world_size`.  Every file goes through DiskMan, so it
/// drops what it held; the emptied folders go after.  `region.map` goes
/// last of all: if this stops part way, the next START SERVER still finds
/// the old map, sees the size is wrong, and finishes the job, where a new
/// world made over the leftovers would take the old chunk files as its
/// own.
fn delete_world() -> Result<(), String> {
    let map_path = region_map_path();
    let mut files = Vec::new();
    let mut folders = Vec::new();
    find_world_files(&world_dir(), &mut files, &mut folders)
        .map_err(|e| format!("{} can't be looked through: {e}", world_dir().display()))?;

    for file in files.iter().filter(|file| **file != map_path) {
        diskman::remove(file).wait().map_err(|e| format!("{} couldn't be deleted: {e}", file.display()))?;
    }
    // The deepest folders were found last, so they're emptied first.
    for folder in folders.iter().rev() {
        fs::remove_dir(folder).map_err(|e| format!("the folder {} couldn't be deleted: {e}", folder.display()))?;
    }
    diskman::remove(&map_path).wait().map_err(|e| format!("{} couldn't be deleted: {e}", map_path.display()))?;
    scribe::debug(Channel::Game, &format!("GameWorld deleted the old world: {} files and {} folders.", files.len(),
                                          folders.len()));
    Ok(())
}

/// Every file and folder inside `folder`, and the folders inside those.  A
/// folder goes in `folders` before anything inside it.  A link isn't
/// followed, only deleted.
fn find_world_files(folder: &Path, files: &mut Vec<PathBuf>, folders: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            folders.push(path.clone());
            find_world_files(&path, files, folders)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

/// The chunk at `pos`: its own file if it has one, or built from its
/// region's ground if it doesn't.  Waits on DiskMan, which is fine on this
/// thread.
fn load(shape: &Shape, pos: ChunkPos, from_files: &mut u64) -> Result<Chunk, String> {
    let Some((number, region)) = shape.map.region_at(pos) else {
        let why = format!("chunk {},{} row {} is outside the world", pos.x, pos.z, pos.row);
        scribe::warn(Channel::Game, &format!("GameWorld was asked for {why}."));
        return Err(why);
    };
    let path = region_dir(&region.name).join(pos.file_name(&region.name));

    match diskman::read(&path).wait() {
        Ok(bytes) => {
            // A bad file is never built over: it may be the only copy of
            // somebody's digging.  The chunk stays out of the game until
            // somebody looks at it.
            let chunk = Chunk::from_bytes(&bytes, pos).map_err(|why| {
                let why = format!("{} isn't right: {why}.  That chunk stays out of the game.", path.display());
                scribe::warn(Channel::Game, &why);
                why
            })?;
            *from_files += 1;
            Ok(chunk)
        }
        Err(e) if e.is_not_found() => build::untouched(pos, region, shape.heights[number].as_ref())
            .inspect_err(|why| scribe::warn(Channel::Game, &format!("GameWorld couldn't build chunk {},{} row {}: \
                {why}.", pos.x, pos.z, pos.row))),
        Err(e) => {
            let why = format!("{} can't be read: {e}", path.display());
            scribe::warn(Channel::Game, &why);
            Err(why)
        }
    }
}
