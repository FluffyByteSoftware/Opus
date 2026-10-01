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

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};

use crate::chunk::{Chunk, ChunkPos, ROWS};

/// A chunk GameWorld has finished with: the chunk, or why there isn't one.
pub struct Loaded {
    pub pos: ChunkPos,
    pub chunk: Result<Chunk, String>,
}

/// The chunks in memory, and the ones asked for that haven't come yet.
pub struct Terrain {
    chunks: HashMap<ChunkPos, Chunk>,
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
        Terrain { chunks: HashMap::new(), reply, arrivals, asked: 0, failed: 0 }
    }

    /// Asks GameWorld for every chunk within `reach` chunks east, west,
    /// north and south of block x,z, every row, that isn't here already.
    /// Comes straight back.  Every player starts at 0,0,0 for now, so the
    /// GameClock asks around there.
    pub fn ask_around(&mut self, x: i32, z: i32, reach: i32) {
        let Some(middle) = ChunkPos::of_block(x, 0, z) else {
            return;
        };
        for chunk_z in middle.z - reach..=middle.z + reach {
            for chunk_x in middle.x - reach..=middle.x + reach {
                for row in 0..ROWS {
                    let pos = ChunkPos { x: chunk_x, z: chunk_z, row };
                    if self.chunks.contains_key(&pos) {
                        continue;
                    }
                    self.asked += 1;
                    if !crate::ask(pos, &self.reply) {
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
            match loaded.chunk {
                Ok(chunk) => {
                    self.chunks.insert(loaded.pos, chunk);
                }
                Err(_) => self.failed += 1,
            }
        }
    }

    /// The chunk at `pos`, if it's in memory.
    pub fn chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos)
    }

    /// How many chunks are in memory.
    pub fn held(&self) -> usize {
        self.chunks.len()
    }

    /// How many chunks have been asked for since START SERVER.
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
