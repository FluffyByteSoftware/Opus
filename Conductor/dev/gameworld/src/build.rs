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
//! says, -5 to 5.  Every voxel is at its kind's plain density, full or
//! empty, until the world is made from a density of its own (smooth
//! voxels, `design/smooth-voxels.md`).
//!
//! There was a GOLD block at 0,0,0, to mark the middle of the world; it
//! was dropped on 2026-10-03, with the kind.

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
            let ground = ground_at(region, heights, pos.west_x() + x, pos.south_z() + z)?;
            for y in 0..SIDE {
                let world_y = bottom + y;
                chunk.set(x, y, z, layer(world_y, ground));
            }
        }
    }
    Ok(chunk)
}

/// How high the dirt is in column x,z of `region`: 0 everywhere in a flat
/// region, wherever its heights file says in one with hills.
pub fn ground_at(region: &Region, heights: Option<&Heights>, x: i32, z: i32) -> Result<i32, String> {
    match region.ground {
        Ground::Flat => Ok(0),
        Ground::Heights => heights
            .and_then(|heights| heights.at(x, z))
            .ok_or_else(|| format!("{}'s heights don't cover column {x},{z}", region.name)),
    }
}

/// The highest block in column x,z that isn't AIR, as nobody has changed
/// it: its height and its kind, the dirt.  The simple overworld map is
/// made from this, so it and the chunks never disagree.
pub fn top(region: &Region, heights: Option<&Heights>, x: i32, z: i32) -> Result<(i32, Block), String> {
    Ok((ground_at(region, heights, x, z)?, Block::DIRT))
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
    fn every_voxel_is_at_its_kinds_plain_density() {
        let bytes = heights::make(3, 0, 0, 32, 32, |_| true).unwrap();
        let heights = heights::Heights::from_contents(Arc::new(bytes)).unwrap();
        for chunk in column(0, 0, &omega(), Some(&heights)) {
            for y in 0..SIDE {
                for z in 0..SIDE {
                    for x in 0..SIDE {
                        assert_eq!(chunk.density(x, y, z), chunk.block(x, y, z).plain_density());
                    }
                }
            }
        }
    }

    #[test]
    fn the_middle_of_the_world_is_omegas_ground_like_anywhere_else() {
        let bytes = heights::make(3, 0, 0, 32, 32, |_| true).unwrap();
        let heights = heights::Heights::from_contents(Arc::new(bytes)).unwrap();
        let rows = column(0, 0, &omega(), Some(&heights));
        let ground = heights.at(0, 0).unwrap();
        assert_eq!(at(&rows, 0, ground, 0), Block::DIRT);
        assert_eq!(at(&rows, 0, ground + 1, 0), Block::AIR);
        assert_eq!(top(&omega(), Some(&heights), 0, 0).unwrap(), (ground, Block::DIRT));
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
    fn the_top_of_a_column_is_its_highest_block_that_isnt_air() {
        let bytes = heights::make(11, -64, -64, 192, 192, |_| true).unwrap();
        let heights = heights::Heights::from_contents(Arc::new(bytes)).unwrap();
        // Columns in chunk 2,2 and chunk -1,0, and 0,0 itself, checked
        // against the chunks the same rules build.
        for (x, z) in [(64, 64), (77, 85), (-1, 0), (-32, 31), (0, 0)] {
            let (region, from) = if x < 0 { (alpha(), None) } else { (omega(), Some(&heights)) };
            let (height, block) = top(&region, from, x, z).unwrap();
            let rows = column(x.div_euclid(SIDE), z.div_euclid(SIDE), &region, from);
            let (inside_x, inside_z) = (x.rem_euclid(SIDE), z.rem_euclid(SIDE));
            assert_eq!(at(&rows, inside_x, height, inside_z), block, "{x},{z}");
            for y in height + 1..=BOTTOM_Y + ROWS as i32 * SIDE - 1 {
                assert_eq!(at(&rows, inside_x, y, inside_z), Block::AIR, "{x},{y},{z}");
            }
        }
    }

    #[test]
    fn omega_without_its_heights_is_an_error_not_a_flat_chunk() {
        assert!(untouched(ChunkPos { x: 2, z: 2, row: 0 }, &omega(), None).is_err());
    }
}
