//! File:       Opus/Conductor/dev/gameworld/src/chunk.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! A chunk: a cube of blocks, 32 a side, so 32 m a side and 32,768 blocks.
//! Chunks sit side by side east-west (x) and north-south (z), and stack in
//! eleven rows up and down (y), from -32 to +319: row 0 is -32 to -1, row 1
//! is 0 to 31, and so on up to row 10, 288 to 319.  Chunk 0,0 has its
//! south-west corner at block 0,0.
//!
//! A chunk somebody has changed is kept as a file of its own:
//! `Content/world/Regions/<Region>/<region>_<x>_<z>_<row>.chunk`, x and z
//! padded to three digits and the row to two, so a folder lists them in
//! order (`alpha_-015_003_01.chunk`).  The file is:
//!
//! ```text
//! 8 bytes   OPUSCHNK
//! u16       version, 2
//! i16       x, the chunk's place east-west
//! i16       z, north-south
//! u8        row, 0 (bottom) to 10 (top)
//! u16 x 32768  the blocks, bottom layer first; in a layer, the south row
//!              first; in a row, west to east.  (y * 32 + z) * 32 + x.
//! ```
//!
//! About 64 KB.  Every number is little-endian.

use std::fmt;

use crate::block::Block;
use crate::bytes::Reader;

/// A chunk's side, in blocks.
pub const SIDE: i32 = 32;

/// How many blocks are in a chunk.
pub const BLOCKS: usize = (SIDE * SIDE * SIDE) as usize;

/// The lowest block in the world, the bottom of row 0.  It and the layer
/// over it are the floor nobody can dig through, all BEDROCK (Jacob,
/// 2026-10-01: "-31 is bedrock can dig to -30 and stand on top of -31").
/// The floor starts at -32 so the rows line up on 32s: block 0 is the
/// bottom of row 1.
pub const BOTTOM_Y: i32 = -32;

/// The highest BEDROCK block.  Everything from `BOTTOM_Y` to here is
/// floor.
pub const FLOOR_Y: i32 = -31;

/// How many rows of chunks the world has, stacked up and down: -32 to
/// +319, 352 blocks.  +319 is the highest anything goes, Minecraft's top
/// (Jacob, 2026-10-01: "go Minecraft height and depth values for now").
pub const ROWS: u8 = 11;

const TAG: &[u8; 8] = b"OPUSCHNK";
/// Version 2 (2026-10-01): blocks went to 1 m and the rows to eleven from
/// -32, so a row number in a version 1 file means another place.
const VERSION: u16 = 2;

/// Where a chunk is: its place east-west and north-south, counted in
/// chunks from 0,0, and its row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
    pub row: u8,
}

impl ChunkPos {
    /// The chunk block x,y,z is in.  `None` for a y above or below both
    /// rows.
    pub fn of_block(x: i32, y: i32, z: i32) -> Option<ChunkPos> {
        // Rust note: `div_euclid` rounds down, so block -1 is in chunk -1,
        // not chunk 0 the way plain `/` would have it.
        let row = (y - BOTTOM_Y).div_euclid(SIDE);
        if row < 0 || row >= ROWS as i32 {
            return None;
        }
        Some(ChunkPos { x: x.div_euclid(SIDE), z: z.div_euclid(SIDE), row: row as u8 })
    }

    /// The x of the chunk's westmost blocks.
    pub fn west_x(&self) -> i32 {
        self.x * SIDE
    }

    /// The z of the chunk's southmost blocks.
    pub fn south_z(&self) -> i32 {
        self.z * SIDE
    }

    /// The y of the chunk's bottom layer.
    pub fn bottom_y(&self) -> i32 {
        BOTTOM_Y + self.row as i32 * SIDE
    }

    /// The chunk's file name, for the region it's in:
    /// `alpha_-015_003_01.chunk`.  The region's name goes in lowercase.
    pub fn file_name(&self, region: &str) -> String {
        format!("{}_{}_{}_{:02}.chunk", region.to_lowercase(), padded(self.x), padded(self.z), self.row)
    }
}

/// A number padded to three digits, with its minus sign in front of them:
/// 3 is `003`, -15 is `-015`.
fn padded(number: i32) -> String {
    if number < 0 {
        format!("-{:03}", -number)
    } else {
        format!("{number:03}")
    }
}

/// A chunk's blocks.
#[derive(Clone)]
pub struct Chunk {
    pos: ChunkPos,
    blocks: Vec<Block>,
}

impl Chunk {
    /// A chunk of one kind of block all through.
    pub fn filled(pos: ChunkPos, block: Block) -> Chunk {
        Chunk { pos, blocks: vec![block; BLOCKS] }
    }

    /// A chunk from its blocks, in the file's order.  The caller makes
    /// sure there are `BLOCKS` of them.
    pub(crate) fn from_blocks(pos: ChunkPos, blocks: Vec<Block>) -> Chunk {
        Chunk { pos, blocks }
    }

    pub fn pos(&self) -> ChunkPos {
        self.pos
    }

    /// Every block, in the file's order: bottom layer first, the south row
    /// first in a layer, west to east in a row.
    pub(crate) fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// The block at x,y,z inside the chunk, each 0 to 31.
    pub fn block(&self, x: i32, y: i32, z: i32) -> Block {
        self.blocks[index(x, y, z)]
    }

    /// Changes the block at x,y,z inside the chunk.
    pub(crate) fn set(&mut self, x: i32, y: i32, z: i32, block: Block) {
        self.blocks[index(x, y, z)] = block;
    }

    /// The chunk as its file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + 2 + 2 + 2 + 1 + BLOCKS * 2);
        bytes.extend_from_slice(TAG);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        // The positions fit in an i16: at world_size 32, the most, the
        // world is 1024 chunks across, -512 to 511.
        bytes.extend_from_slice(&(self.pos.x as i16).to_le_bytes());
        bytes.extend_from_slice(&(self.pos.z as i16).to_le_bytes());
        bytes.push(self.pos.row);
        for block in &self.blocks {
            bytes.extend_from_slice(&block.0.to_le_bytes());
        }
        bytes
    }

    /// A chunk read back from its file.  The file has to say it's the
    /// chunk it was read for: one copied in under another chunk's name
    /// would put a hill in the wrong place.
    pub fn from_bytes(bytes: &[u8], expected: ChunkPos) -> Result<Chunk, String> {
        let mut reader = Reader::new(bytes);
        reader.tag_and_version(TAG, VERSION)?;
        let pos = ChunkPos {
            x: reader.i16("its x")? as i32,
            z: reader.i16("its z")? as i32,
            row: reader.u8("its row")?,
        };
        if pos != expected {
            return Err(format!("it says it's chunk {},{} row {}, and it's in the file for chunk {},{} row {}",
                               pos.x, pos.z, pos.row, expected.x, expected.z, expected.row));
        }
        let mut blocks = Vec::with_capacity(BLOCKS);
        for _ in 0..BLOCKS {
            blocks.push(Block(reader.u16("its blocks")?));
        }
        reader.finish()?;
        Ok(Chunk { pos, blocks })
    }
}

// Printing 32,768 blocks helps nobody, so a chunk prints as where it is.
impl fmt::Debug for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Chunk {},{} row {}", self.pos.x, self.pos.z, self.pos.row)
    }
}

/// Where block x,y,z (each 0 to 31) sits in the chunk's list.
fn index(x: i32, y: i32, z: i32) -> usize {
    ((y * SIDE + z) * SIDE + x) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_below_zero_are_in_the_chunks_below_zero() {
        assert_eq!(ChunkPos::of_block(0, 0, 0), Some(ChunkPos { x: 0, z: 0, row: 1 }));
        assert_eq!(ChunkPos::of_block(-1, 0, -1), Some(ChunkPos { x: -1, z: -1, row: 1 }));
        assert_eq!(ChunkPos::of_block(31, 0, 32), Some(ChunkPos { x: 0, z: 1, row: 1 }));
        assert_eq!(ChunkPos::of_block(-8192, 0, 8191), Some(ChunkPos { x: -256, z: 255, row: 1 }));
    }

    #[test]
    fn the_rows_run_from_minus_32_to_319_in_32s() {
        assert_eq!(ChunkPos::of_block(0, -32, 0).map(|pos| pos.row), Some(0));
        assert_eq!(ChunkPos::of_block(0, -1, 0).map(|pos| pos.row), Some(0));
        assert_eq!(ChunkPos::of_block(0, 0, 0).map(|pos| pos.row), Some(1));
        assert_eq!(ChunkPos::of_block(0, 31, 0).map(|pos| pos.row), Some(1));
        assert_eq!(ChunkPos::of_block(0, 288, 0).map(|pos| pos.row), Some(10));
        assert_eq!(ChunkPos::of_block(0, 319, 0).map(|pos| pos.row), Some(10));
        assert_eq!(ChunkPos::of_block(0, -33, 0), None);
        assert_eq!(ChunkPos::of_block(0, 320, 0), None);
        assert_eq!(ChunkPos { x: 0, z: 0, row: 0 }.bottom_y(), -32);
        assert_eq!(ChunkPos { x: 0, z: 0, row: 1 }.bottom_y(), 0);
        assert_eq!(ChunkPos { x: 0, z: 0, row: 10 }.bottom_y(), 288);
    }

    #[test]
    fn file_names_are_padded_to_three_digits_and_the_row_to_two() {
        assert_eq!(ChunkPos { x: -15, z: 3, row: 0 }.file_name("Alpha"), "alpha_-015_003_00.chunk");
        assert_eq!(ChunkPos { x: 255, z: -256, row: 10 }.file_name("Omega"), "omega_255_-256_10.chunk");
    }

    #[test]
    fn a_chunk_comes_back_from_its_file_as_it_went_in() {
        let pos = ChunkPos { x: -3, z: 7, row: 1 };
        let mut chunk = Chunk::filled(pos, Block::AIR);
        chunk.set(0, 0, 0, Block::STONE);
        chunk.set(31, 31, 31, Block::GOLD);
        chunk.set(5, 6, 7, Block(999));

        let bytes = chunk.to_bytes();
        assert_eq!(bytes.len(), 15 + BLOCKS * 2);
        let back = Chunk::from_bytes(&bytes, pos).unwrap();
        assert_eq!(back.block(0, 0, 0), Block::STONE);
        assert_eq!(back.block(31, 31, 31), Block::GOLD);
        assert_eq!(back.block(5, 6, 7), Block(999));
        assert_eq!(back.block(1, 0, 0), Block::AIR);
    }

    #[test]
    fn a_file_for_another_chunk_or_a_short_one_is_turned_away() {
        let pos = ChunkPos { x: 1, z: 2, row: 0 };
        let bytes = Chunk::filled(pos, Block::DIRT).to_bytes();
        assert!(Chunk::from_bytes(&bytes, ChunkPos { x: 2, z: 1, row: 0 }).is_err());
        assert!(Chunk::from_bytes(&bytes[..bytes.len() - 1], pos).is_err());
        assert!(Chunk::from_bytes(b"NOTACHNK", pos).is_err());
    }
}
