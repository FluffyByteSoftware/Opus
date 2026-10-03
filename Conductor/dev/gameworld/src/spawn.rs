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
//! A saved character comes back where it was, unless that's inside the
//! ground now (Jacob, 2026-10-03, session 9: "if the space they were in is
//! now occupied with impassable voxel (IE: not air) it should move them on
//! top of it").  `footing()` looks at the two blocks a character fills and,
//! if either isn't AIR, says where it stands instead: on top of its
//! column's highest block, in the middle, or nowhere, if that's too high to
//! stand on inside the world.  GameWorld works out every spawn point's
//! height once as it starts, and keeps the last it knew of each
//! (`last_known()`), for a PLAY that can't wait on it.
//!
//! The one limit (TODO.md): GameWorld knows a chunk as it was last saved,
//! so once there's digging, a column changed since the last world save
//! gives the height it had then.

use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use crate::block::Block;
use crate::chunk::{BOTTOM_Y, Chunk, ChunkPos, ROWS, SIDE};

/// The spawn points, as columns: x and z in blocks.  Only 0,0 for now
/// ("for right now is only 0,0,0"); the generated ones go here.
pub const SPAWN_POINTS: [(i32, i32); 1] = [(0, 0)];

/// The highest block in the world.  A character is 2 blocks tall, so the
/// highest it can stand is with its feet on 318.
const CEILING: i32 = BOTTOM_Y + ROWS as i32 * SIDE - 1;

/// The last place GameWorld worked out for each spawn point, by its column.
static LAST_KNOWN: LazyLock<Mutex<HashMap<(i32, i32), [f32; 3]>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

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
    let place = on_top(x, z, top);
    remember(x, z, place);
    Ok(place)
}

/// The last place worked out for the spawn point x, z, without asking
/// GameWorld: for when GameWorld can't answer (Jacob's "b").  `None` if
/// it never has, which only happens when the world couldn't be read.
pub fn last_known(x: i32, z: i32) -> Option<[f32; 3]> {
    lock(&LAST_KNOWN).get(&(x, z)).copied()
}

/// Notes the place worked out for a spawn point.
pub(crate) fn remember(x: i32, z: i32, place: [f32; 3]) {
    lock(&LAST_KNOWN).insert((x, z), place);
}

/// Standing on top of block `top` in column x, z, in the middle of it.
pub(crate) fn on_top(x: i32, z: i32, top: i32) -> [f32; 3] {
    [x as f32 + 0.5, (top + 1) as f32, z as f32 + 0.5]
}

/// Where a saved character can stand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Footing {
    /// Where it was is clear: both blocks it fills are AIR.
    Clear,
    /// Where it was is in the ground; it stands here instead, on top of its
    /// column, in the middle of the block.
    OnTop([f32; 3]),
    /// Where it was is in the ground, and the top of its column is too high
    /// to stand on inside the world.
    TooHigh,
}

/// Whether a character saved at `position` (its feet) can stand there, and
/// where it stands if not.  Asks GameWorld's thread and waits for it up to
/// `wait`, like `place_at()`.
pub fn footing(position: [f32; 3], wait: Duration) -> Result<Footing, String> {
    // Rust note: `as i32` on a float too big for an i32 gives the biggest
    // i32 rather than garbage, so a wild position can't break this.
    let block = position.map(|axis| axis.floor() as i32);
    let (reply, answer) = mpsc::channel();
    if !crate::ask_footing(block, reply) {
        return Err("GameWorld isn't running".to_string());
    }
    answer.recv_timeout(wait).map_err(|_| format!("GameWorld didn't answer in {} s", wait.as_secs()))?
}

/// The footing of a character whose feet are in `block`: the blocks at its
/// feet and its head, and the top of its column if either isn't AIR.  On
/// GameWorld's thread; `chunk` hands over the chunks, read or built.
pub(crate) fn footing_of(block: [i32; 3], mut chunk: impl FnMut(ChunkPos) -> Result<Chunk, String>)
    -> Result<Footing, String> {
    let [x, y, z] = block;
    let feet = block_at(x, y, z, &mut chunk)?;
    let head = block_at(x, y + 1, z, &mut chunk)?;
    if feet == Block::AIR && head == Block::AIR {
        return Ok(Footing::Clear);
    }
    let top = top_of(x, z, &mut chunk)?;
    if top + 2 > CEILING {
        return Ok(Footing::TooHigh);
    }
    Ok(Footing::OnTop(on_top(x, z, top)))
}

/// The block at x, y, z.  Above the world is open air; below it is as
/// solid as its floor.
fn block_at(x: i32, y: i32, z: i32, chunk: &mut impl FnMut(ChunkPos) -> Result<Chunk, String>)
    -> Result<Block, String> {
    match ChunkPos::of_block(x, y, z) {
        Some(pos) => Ok(chunk(pos)?.block(x.rem_euclid(SIDE), y - pos.bottom_y(), z.rem_euclid(SIDE))),
        None if y < BOTTOM_Y => Ok(Block::BEDROCK),
        None => Ok(Block::AIR),
    }
}

/// The lock idiom, for the static above.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
    fn the_top_at_0_0_is_its_ground() {
        assert_eq!(top_of(0, 0, |pos| build::untouched(pos, &alpha(), None)), Ok(0));
        let bytes = crate::heights::make(3, 0, 0, 32, 32, |_| true).unwrap();
        let heights = Heights::from_contents(Arc::new(bytes)).unwrap();
        let ground = heights.at(0, 0).unwrap();
        assert_eq!(top_of(0, 0, |pos| build::untouched(pos, &omega(), Some(&heights))), Ok(ground));
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
        assert!(footing_of([0, 0, 0], |_| Err("a bad file".to_string())).is_err());
    }

    #[test]
    fn standing_on_the_ground_is_clear() {
        assert_eq!(footing_of([0, 1, 0], |pos| build::untouched(pos, &alpha(), None)), Ok(Footing::Clear));
        assert_eq!(footing_of([-40, 1, 77], |pos| build::untouched(pos, &alpha(), None)), Ok(Footing::Clear));
    }

    #[test]
    fn asdf_inside_the_ground_is_stood_on_top_in_the_middle() {
        // Asdf, saved at 0, 0, 0 before spawn points: its feet in the
        // ground (the GOLD, then).
        assert_eq!(footing_of([0, 0, 0], |pos| build::untouched(pos, &alpha(), None)),
                   Ok(Footing::OnTop([0.5, 1.0, 0.5])));
        // Deep in Alpha's stone, at -2.5, -10, 3.25.
        assert_eq!(footing_of([-3, -10, 3], |pos| build::untouched(pos, &alpha(), None)),
                   Ok(Footing::OnTop([-2.5, 1.0, 3.5])));
    }

    #[test]
    fn a_head_in_the_ground_is_moved_too() {
        // Feet in the air at y 99, head in a block of WOOD at 100.
        let footing = footing_of([5, 99, 6], |pos| {
            let mut chunk = build::untouched(pos, &alpha(), None)?;
            if pos.row == 4 {
                chunk.set(5, 100 - pos.bottom_y(), 6, Block::WOOD);
            }
            Ok(chunk)
        });
        assert_eq!(footing, Ok(Footing::OnTop([5.5, 101.0, 6.5])));
    }

    #[test]
    fn a_column_too_high_to_stand_on_is_too_high() {
        assert_eq!(CEILING, 319);
        // Something built at 318: standing on it, the head would be at 320.
        let footing = footing_of([5, 0, 6], |pos| {
            let mut chunk = build::untouched(pos, &alpha(), None)?;
            if pos.row == 10 {
                chunk.set(5, 318 - pos.bottom_y(), 6, Block::WOOD);
            }
            Ok(chunk)
        });
        assert_eq!(footing, Ok(Footing::TooHigh));
    }

    #[test]
    fn above_the_world_is_air_and_below_it_is_floor() {
        assert_eq!(footing_of([0, 400, 0], |pos| build::untouched(pos, &alpha(), None)), Ok(Footing::Clear));
        assert_eq!(footing_of([0, -100, 0], |pos| build::untouched(pos, &alpha(), None)),
                   Ok(Footing::OnTop([0.5, 1.0, 0.5])));
    }

    #[test]
    fn a_spawn_points_place_is_remembered() {
        // A column of its own, so no other test's spawn point touches it.
        assert_eq!(last_known(9_001, 9_001), None);
        remember(9_001, 9_001, on_top(9_001, 9_001, 0));
        assert_eq!(last_known(9_001, 9_001), Some([9_001.5, 1.0, 9_001.5]));
    }
}
