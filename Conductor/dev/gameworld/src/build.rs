//! File:       Opus/Conductor/dev/gameworld/src/build.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Building a chunk nobody has changed.  Such a chunk has no file: it's
//! made from its region's ground every time it's needed, the same way
//! every time.
//!
//! Every column is the same three layers (Jacob, 2026-09-30): one block of
//! dirt at the ground's height, stone under it down to the floor at -16,
//! and air over it.  In Alpha the ground is at 0 everywhere; in Omega it's
//! wherever the heights file says, -5 to 5.  The block at 0,0,0 is GOLD.

use crate::block::Block;
use crate::chunk::{BOTTOM_Y, Chunk, ChunkPos, SIDE};
use crate::heights::Heights;
use crate::regionmap::{Ground, Region};

/// The chunk at `pos`, as its region's ground makes it.  `heights` is the
/// region's heights file, for a region that has one.
pub fn untouched(pos: ChunkPos, region: &Region, heights: Option<&Heights>) -> Result<Chunk, String> {
    let mut chunk = Chunk::filled(pos, Block::AIR);
    let bottom = pos.bottom_y();

    for z in 0..SIDE {
        for x in 0..SIDE {
            let world_x = pos.west_x() + x;
            let world_z = pos.south_z() + z;
            let ground = match region.ground {
                Ground::Flat => 0,
                Ground::Heights => heights
                    .and_then(|heights| heights.at(world_x, world_z))
                    .ok_or_else(|| format!("{}'s heights don't cover column {world_x},{world_z}", region.name))?,
            };
            for y in 0..SIDE {
                let world_y = bottom + y;
                chunk.set(x, y, z, layer(world_y, ground));
            }
        }
    }

    // The GOLD block marks the middle of the world.
    if let Some(origin) = ChunkPos::of_block(0, 0, 0) {
        if origin == pos {
            chunk.set(-pos.west_x(), -bottom, -pos.south_z(), Block::GOLD);
        }
    }
    Ok(chunk)
}

/// What's at height `y` in a column whose dirt is at `ground`.
fn layer(y: i32, ground: i32) -> Block {
    if y == BOTTOM_Y || y < ground {
        Block::STONE
    } else if y == ground {
        Block::DIRT
    } else {
        Block::AIR
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heights;
    use std::sync::Arc;

    fn alpha() -> Region {
        Region { name: "Alpha".to_string(), ground: Ground::Flat }
    }

    fn omega() -> Region {
        Region { name: "Omega".to_string(), ground: Ground::Heights }
    }

    #[test]
    fn alpha_is_stone_to_minus_1_dirt_at_0_and_air_above() {
        let pos = ChunkPos { x: -3, z: 5, row: 0 };
        let chunk = untouched(pos, &alpha(), None).unwrap();
        // In a chunk of the lower row, y 0 is the world's -16, so the
        // world's 0 is y 16.
        for (x, z) in [(0, 0), (17, 30), (31, 31)] {
            assert_eq!(chunk.block(x, 0, z), Block::STONE, "the floor");
            assert_eq!(chunk.block(x, 15, z), Block::STONE, "-1");
            assert_eq!(chunk.block(x, 16, z), Block::DIRT, "0");
            assert_eq!(chunk.block(x, 17, z), Block::AIR, "1");
            assert_eq!(chunk.block(x, 31, z), Block::AIR, "15");
        }
    }

    #[test]
    fn the_upper_row_starts_as_air() {
        let chunk = untouched(ChunkPos { x: -3, z: 5, row: 1 }, &alpha(), None).unwrap();
        for y in 0..SIDE {
            assert_eq!(chunk.block(4, y, 9), Block::AIR);
        }
    }

    #[test]
    fn the_block_at_0_0_0_is_gold_and_only_that_one() {
        let bytes = heights::make(3, 0, 0, 32, 32, |_| true).unwrap();
        let heights = heights::Heights::from_contents(Arc::new(bytes)).unwrap();
        let chunk = untouched(ChunkPos { x: 0, z: 0, row: 0 }, &omega(), Some(&heights)).unwrap();
        assert_eq!(chunk.block(0, 16, 0), Block::GOLD);
        assert_eq!(chunk.block(1, 16, 0), Block::DIRT);
        assert_eq!(chunk.block(0, 15, 0), Block::STONE);

        let beside = untouched(ChunkPos { x: -1, z: 0, row: 0 }, &alpha(), None).unwrap();
        assert_eq!(beside.block(31, 16, 0), Block::DIRT);
    }

    #[test]
    fn omega_puts_its_dirt_at_the_height_in_its_file() {
        let bytes = heights::make(11, 64, 64, 32, 32, |_| true).unwrap();
        let heights = heights::Heights::from_contents(Arc::new(bytes)).unwrap();
        let chunk = untouched(ChunkPos { x: 2, z: 2, row: 0 }, &omega(), Some(&heights)).unwrap();
        for (x, z) in [(0, 0), (13, 21), (31, 31)] {
            let ground = heights.at(64 + x, 64 + z).unwrap();
            let y = ground - BOTTOM_Y;
            assert_eq!(chunk.block(x, y, z), Block::DIRT);
            assert_eq!(chunk.block(x, y - 1, z), Block::STONE);
            assert_eq!(chunk.block(x, y + 1, z), Block::AIR);
            assert_eq!(chunk.block(x, 0, z), Block::STONE);
        }
    }

    #[test]
    fn omega_without_its_heights_is_an_error_not_a_flat_chunk() {
        assert!(untouched(ChunkPos { x: 2, z: 2, row: 0 }, &omega(), None).is_err());
    }
}
