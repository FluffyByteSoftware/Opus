//! File:       Opus/Conductor/dev/gameworld/src/spawn.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Spawn points: the columns a character is sent to when it's made, and
//! when its player presses RESET HOME (Jacob, 2026-10-03).  A spawn point is
//! a column, never a height: "we shouldn't give them a fixed position to
//! spawn at (I mean no Y=X) instead we need ot make a designated spawn
//! point ... and then put the player on top of the highest voxel."  So the
//! height is worked out from the ground as it is, on GameWorld's thread,
//! from the column's own chunk files if it has any and its region's ground
//! if not.  That's any column in the world, not only the ones the GameClock
//! has loaded, since the world will be full of spawn points one day ("we
//! want to be ready for that").  The character stands in the middle of the
//! block, its feet on top of it.
//!
//! The one limit (TODO.md): GameWorld knows a chunk as it was last saved,
//! so once there's digging, a column changed since the last world save
//! gives the height it had then.

use std::sync::mpsc;
use std::time::Duration;

use crate::block::Block;
use crate::chunk::{Chunk, ChunkPos, ROWS, SIDE};

/// The spawn points, as columns: x and z in blocks.  Only the GOLD's for
/// now ("for right now is only 0,0,0"); the generated ones go here.
pub const SPAWN_POINTS: [(i32, i32); 1] = [(0, 0)];

/// Where a character stands at the spawn point x, z: in the middle of the
/// column, its feet on the highest block that isn't AIR.  Asks GameWorld's
/// thread and waits for it up to `wait`, so it's for a caller on a thread
/// of its own (Protogame), never the GameClock.
pub fn place_at(x: i32, z: i32, wait: Duration) -> Result<[f32; 3], String> {
    let (reply, answer) = mpsc::channel();
    if !crate::ask_top(x, z, reply) {
        return Err("GameWorld isn't running".to_string());
    }
    let top = answer.recv_timeout(wait)
        .map_err(|_| format!("GameWorld didn't answer in {} s", wait.as_secs()))??;
    Ok([x as f32 + 0.5, (top + 1) as f32, z as f32 + 0.5])
}

/// The y of the highest block in column x, z that isn't AIR, looking down
/// from the top row.  `chunk` hands over each of the column's chunks, read
/// or built.  An error if one can't be had, or if the column is all AIR
/// (it never is: the floor is BEDROCK).
pub(crate) fn top_of(x: i32, z: i32, mut chunk: impl FnMut(ChunkPos) -> Result<Chunk, String>)
    -> Result<i32, String> {
    let inside_x = x.rem_euclid(SIDE);
    let inside_z = z.rem_euclid(SIDE);
    for row in (0..ROWS).rev() {
        let pos = ChunkPos { x: x.div_euclid(SIDE), z: z.div_euclid(SIDE), row };
        let chunk = chunk(pos)?;
        for y in (0..SIDE).rev() {
            if chunk.block(inside_x, y, inside_z) != Block::AIR {
                return Ok(pos.bottom_y() + y);
            }
        }
    }
    Err(format!("column {x},{z} is all air"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build;
    use crate::heights::Heights;
    use crate::regionmap::{Ground, Region};
    use std::sync::Arc;

    fn alpha() -> Region {
        Region { name: "Alpha".to_string(), ground: Ground::Flat }
    }

    fn omega() -> Region {
        Region { name: "Omega".to_string(), ground: Ground::Heights }
    }

    #[test]
    fn alphas_top_is_its_dirt_at_0() {
        assert_eq!(top_of(-40, 77, |pos| build::untouched(pos, &alpha(), None)), Ok(0));
    }

    #[test]
    fn the_gold_is_the_top_at_0_0() {
        assert_eq!(top_of(0, 0, |pos| build::untouched(pos, &alpha(), None)), Ok(0));
    }

    #[test]
    fn omegas_top_is_its_hill() {
        let bytes = crate::heights::make(11, 64, 64, 32, 32, |_| true).unwrap();
        let heights = Heights::from_contents(Arc::new(bytes)).unwrap();
        let ground = heights.at(70, 90).unwrap();
        assert_eq!(top_of(70, 90, |pos| build::untouched(pos, &omega(), Some(&heights))), Ok(ground));
    }

    #[test]
    fn something_built_above_the_ground_is_stood_on() {
        // A block of WOOD at y 100 over Alpha's dirt: row 4 is 96 to 127.
        let top = top_of(5, 6, |pos| {
            let mut chunk = build::untouched(pos, &alpha(), None)?;
            if pos.row == 4 {
                chunk.set(5, 100 - pos.bottom_y(), 6, Block::WOOD);
            }
            Ok(chunk)
        });
        assert_eq!(top, Ok(100));
    }

    #[test]
    fn a_chunk_that_cant_be_had_is_an_error() {
        assert!(top_of(0, 0, |_| Err("a bad file".to_string())).is_err());
    }
}
