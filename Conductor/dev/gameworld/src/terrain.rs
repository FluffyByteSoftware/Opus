//! File:       Opus/Conductor/dev/gameworld/src/terrain.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The terrain: the chunks the game has in memory, held by the GameClock's
//! thread.  The GameClock owns it the way it owns primlib's `World`, and
//! nothing else touches it, so it needs no lock (Jacob, 2026-09-30:
//! "GameClock Thread seems right").  Digging a block will be a change in
//! memory here, with no waiting.
//!
//! The slow part (reading a chunk's file, or building an untouched one)
//! happens on GameWorld's own thread.  The terrain asks for a chunk and
//! carries on; the chunk turns up in its mailbox later, and
//! `take_arrivals()` puts whatever has come into the terrain without ever
//! waiting.
//!
//! Since movement (2026-10-03) the GameClock asks for the ground around
//! each player as they walk, and lets go of chunks nobody is near
//! (`forget_all_but()`).  A chunk asked for and not in yet is remembered,
//! so walking back and forth doesn't ask for it twice.  Nothing changes a
//! chunk yet, so one let go of is only read or built again; once digging
//! comes, a changed chunk has to be saved before it's let go.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};

use crate::block::Block;
use crate::chunk::{BOTTOM_Y, Chunk, ChunkPos, ROWS, SIDE};

/// A chunk GameWorld has finished with: the chunk, or why there isn't one.
pub struct Loaded {
    pub pos: ChunkPos,
    pub chunk: Result<Chunk, String>,
}

/// The chunks in memory, and the ones asked for that haven't come yet.
pub struct Terrain {
    chunks: HashMap<ChunkPos, Chunk>,
    /// Asked for and not in yet.
    coming: HashSet<ChunkPos>,
    /// Asked for and couldn't be had (GameWorld said why in the log), so
    /// they aren't asked for again this run.
    refused: HashSet<ChunkPos>,
    /// What GameWorld sends the chunks back on, handed out with each ask.
    reply: Sender<Loaded>,
    arrivals: Receiver<Loaded>,
    asked: usize,
    failed: usize,
}

impl Terrain {
    /// No chunks at all.
    pub fn new() -> Terrain {
        let (reply, arrivals) = mpsc::channel();
        Terrain { chunks: HashMap::new(), coming: HashSet::new(), refused: HashSet::new(), reply, arrivals, asked: 0,
                  failed: 0 }
    }

    /// Asks GameWorld for every chunk within `reach` chunks east, west,
    /// north and south of block x,z, every row, that isn't here already
    /// or on its way.  Comes straight back.  The GameClock asks around the
    /// spawn point at START SERVER, and around each player as they walk.
    pub fn ask_around(&mut self, x: i32, z: i32, reach: i32) {
        let Some(middle) = ChunkPos::of_block(x, 0, z) else {
            return;
        };
        for chunk_z in middle.z - reach..=middle.z + reach {
            for chunk_x in middle.x - reach..=middle.x + reach {
                for row in 0..ROWS {
                    let pos = ChunkPos { x: chunk_x, z: chunk_z, row };
                    if self.chunks.contains_key(&pos) || self.coming.contains(&pos) || self.refused.contains(&pos)
                        || !crate::in_world(pos) {
                        continue;
                    }
                    self.asked += 1;
                    if crate::ask(pos, &self.reply) {
                        self.coming.insert(pos);
                    } else {
                        self.failed += 1;
                    }
                }
            }
        }
    }

    /// Puts every chunk that has come in since last time into the terrain.
    /// Never waits.  A chunk that couldn't be had is counted; GameWorld
    /// has already said why in the log.
    pub fn take_arrivals(&mut self) {
        // Rust note: `try_iter` hands over whatever is in the channel now
        // and stops when it's empty, without waiting for more.
        for loaded in self.arrivals.try_iter() {
            self.coming.remove(&loaded.pos);
            match loaded.chunk {
                Ok(chunk) => {
                    self.chunks.insert(loaded.pos, chunk);
                }
                Err(_) => {
                    self.failed += 1;
                    self.refused.insert(loaded.pos);
                }
            }
        }
    }

    /// Lets go of every chunk in memory that `keep` says no to.  How many
    /// went.  One still on its way comes in anyway, and is let go of the
    /// next time round if nobody wants it then.
    pub fn forget_all_but(&mut self, keep: impl Fn(ChunkPos) -> bool) -> usize {
        let before = self.chunks.len();
        self.chunks.retain(|pos, _| keep(*pos));
        before - self.chunks.len()
    }

    /// The chunk at `pos`, if it's in memory.
    pub fn chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos)
    }

    /// The block at x, y, z, counted in blocks from 0,0,0, if its chunk is
    /// in memory.  Below the world is as solid as its floor, and above it
    /// is open air, the same as GameWorld has it.
    pub fn block_at(&self, x: i32, y: i32, z: i32) -> Option<Block> {
        match ChunkPos::of_block(x, y, z) {
            Some(pos) => self.chunks.get(&pos)
                .map(|chunk| chunk.block(x.rem_euclid(SIDE), y - pos.bottom_y(), z.rem_euclid(SIDE))),
            None if y < BOTTOM_Y => Some(Block::BEDROCK),
            None => Some(Block::AIR),
        }
    }

    /// How many chunks are on their way from GameWorld.
    pub fn coming(&self) -> usize {
        self.coming.len()
    }

    /// How many chunks are in memory.
    pub fn held(&self) -> usize {
        self.chunks.len()
    }

    /// How many chunks have been asked for since START SERVER, counting a
    /// chunk let go of and asked for again each time.
    pub fn asked(&self) -> usize {
        self.asked
    }

    /// How many of those couldn't be had.
    pub fn failed(&self) -> usize {
        self.failed
    }
}

impl Default for Terrain {
    fn default() -> Terrain {
        Terrain::new()
    }
}
