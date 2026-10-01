//! File:       Opus/Conductor/dev/gameworld/src/build.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Building a chunk nobody has changed.  Such a chunk has no file: it's
//! made from its region's ground every time it's needed, the same way
//! every time.
//!
//! Every column is the same three layers (Jacob, 2026-09-30): one block of
//! dirt at the ground's height, stone under it down to -30, and air over
//! it, on a floor of BEDROCK at -31 and -32 (2026-10-01).  In Alpha the
//! ground is at 0 everywhere; in Omega it's wherever the heights file
//! says, -5 to 5.
//!
//! The block at 0,0,0 is GOLD, whatever is around it.  It's on Omega's
//! side of the line, and Omega's ground there can be up to 5 blocks
//! higher or lower than 0, so the GOLD can end up inside a hill or with
//! air under it.

use crate::block::Block;
use crate::chunk::{Chunk, ChunkPos, FLOOR_Y, SIDE};
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
    if y <= FLOOR_Y {
        Block::BEDROCK
    } else if y < ground {
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
    use crate::chunk::{BOTTOM_Y, ROWS};
    use crate::heights;
    use std::sync::Arc;

    fn alpha() -> Region {
        Region { name: "Alpha".to_string(), ground: Ground::Flat }
    }

    fn omega() -> Region {
        Region { name: "Omega".to_string(), ground: Ground::Heights }
    }

    /// The block at world height `y` in column x,z (each 0 to 31) of the
    /// chunks of one column, row 0 first.
    fn at(rows: &[Chunk], x: i32, y: i32, z: i32) -> Block {
        let from_bottom = y - BOTTOM_Y;
        rows[(from_bottom / SIDE) as usize].block(x, from_bottom % SIDE, z)
    }

    /// Every row of the chunk column at x,z.
    fn column(x: i32, z: i32, region: &Region, heights: Option<&heights::Heights>) -> Vec<Chunk> {
        (0..ROWS).map(|row| untouched(ChunkPos { x, z, row }, region, heights).unwrap()).collect()
    }

    #[test]
    fn alpha_is_bedrock_stone_to_minus_1_dirt_at_0_and_air_above() {
        let rows = column(-3, 5, &alpha(), None);
        for (x, z) in [(0, 0), (17, 30), (31, 31)] {
            assert_eq!(at(&rows, x, -32, z), Block::BEDROCK, "-32");
            assert_eq!(at(&rows, x, -31, z), Block::BEDROCK, "-31");
            assert_eq!(at(&rows, x, -30, z), Block::STONE, "-30");
            assert_eq!(at(&rows, x, -1, z), Block::STONE, "-1");
            assert_eq!(at(&rows, x, 0, z), Block::DIRT, "0");
            assert_eq!(at(&rows, x, 1, z), Block::AIR, "1");
            assert_eq!(at(&rows, x, 319, z), Block::AIR, "319");
        }
    }

    #[test]
    fn the_rows_above_the_ground_start_as_air() {
        for row in 2..ROWS {
            let chunk = untouched(ChunkPos { x: -3, z: 5, row }, &alpha(), None).unwrap();
            for y in 0..SIDE {
                assert_eq!(chunk.block(4, y, 9), Block::AIR);
            }
        }
    }

    #[test]
    fn the_block_at_0_0_0_is_gold_and_only_that_one() {
        let bytes = heights::make(3, 0, 0, 32, 32, |_| true).unwrap();
        let heights = heights::Heights::from_contents(Arc::new(bytes)).unwrap();
        let rows = column(0, 0, &omega(), Some(&heights));
        assert_eq!(at(&rows, 0, 0, 0), Block::GOLD);
        let golds = rows.iter()
            .flat_map(|chunk| (0..SIDE).flat_map(move |y| (0..SIDE).flat_map(move |z| (0..SIDE)
                .map(move |x| chunk.block(x, y, z)))))
            .filter(|&block| block == Block::GOLD)
            .count();
        assert_eq!(golds, 1);

        let beside = column(-1, 0, &alpha(), None);
        assert_eq!(at(&beside, 31, 0, 0), Block::DIRT);
    }

    #[test]
    fn omega_puts_its_dirt_at_the_height_in_its_file() {
        let bytes = heights::make(11, 64, 64, 32, 32, |_| true).unwrap();
        let heights = heights::Heights::from_contents(Arc::new(bytes)).unwrap();
        let rows = column(2, 2, &omega(), Some(&heights));
        for (x, z) in [(0, 0), (13, 21), (31, 31)] {
            let ground = heights.at(64 + x, 64 + z).unwrap();
            assert_eq!(at(&rows, x, ground, z), Block::DIRT);
            assert_eq!(at(&rows, x, ground - 1, z), Block::STONE);
            assert_eq!(at(&rows, x, ground + 1, z), Block::AIR);
            assert_eq!(at(&rows, x, -30, z), Block::STONE);
            assert_eq!(at(&rows, x, -31, z), Block::BEDROCK);
        }
    }

    #[test]
    fn omega_without_its_heights_is_an_error_not_a_flat_chunk() {
        assert!(untouched(ChunkPos { x: 2, z: 2, row: 0 }, &omega(), None).is_err());
    }
}
