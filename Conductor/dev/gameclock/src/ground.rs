//! File:       Opus/Conductor/dev/gameclock/src/ground.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The ground the GameClock holds follows the players (2026-10-03,
//! movement; Jacob's "b": all of it now).  At START SERVER the terrain is
//! the chunks around the spawn point, `view_chunks` of them each way,
//! which `ready()` waits on.  From then on, whenever a player's character
//! comes into the world or walks into another column of chunks, the
//! GameClock asks GameWorld for the ground around it, and every so often
//! lets go of what nobody is near.
//!
//! The server only needs the ground under and around each character to
//! check their moves (`movement.rs`), so it holds `FOLLOW_CHUNKS` each
//! way of a player, not their whole view: what a player's client sees
//! comes from GameWorld's squeezed copies, which networking sends, not
//! from here.  A chunk is 64 KB in memory (32,768 blocks of two bytes),
//! so the whole view at 8 would be about 200 MB for a player standing
//! alone; this is 275 chunks, about 18 MB.  When NPCs
//! walk (0.0.3 on WAYPOINTS.md) they'll want ground of their own, and
//! this is where it goes.  The spawn point's square is never let go of:
//! every new character comes in there.

use std::collections::HashMap;

use conductor_gameworld::{ChunkPos, SPAWN_POINTS};
use conductor_gameworld::chunk::SIDE;
use conductor_tools::scribe::{self, Channel};

use crate::Game;

/// How many chunks each way of a player's column the server holds.
const FOLLOW_CHUNKS: i32 = 2;

/// How far past that a chunk is kept before it's let go of, so walking
/// back and forth over a chunk's edge doesn't load and drop the same
/// ground over and over.
const KEEP_CHUNKS: i32 = FOLLOW_CHUNKS + 1;

/// How many cycles between two looks for ground nobody is near: 16, four
/// seconds.  Looking goes through every chunk held, so it isn't every
/// cycle.
const FORGET_EVERY: u64 = 16;

/// Which column each player's character was last in, by its row's id.
pub(crate) struct Ground {
    columns: HashMap<i64, (i32, i32)>,
    /// Whether anybody moved column, came or went since the last look.
    changed: bool,
    cycles: u64,
}

impl Ground {
    pub(crate) fn new() -> Ground {
        Ground { columns: HashMap::new(), changed: false, cycles: 0 }
    }
}

/// Housekeeping's part, once the ground around the spawn point is in:
/// asks for the ground around every character that came into the world
/// or walked into another column, and every `FORGET_EVERY` cycles lets go
/// of the chunks nobody is near.
pub(crate) fn follow(game: &mut Game) {
    let ground = &mut game.ground;
    ground.cycles += 1;

    let mut now = HashMap::new();
    for (id, entity) in game.players.characters() {
        let Some(transform) = game.world.transform(entity) else {
            continue;
        };
        let (x, z) = (transform.position.x.floor() as i32, transform.position.z.floor() as i32);
        let column = (x.div_euclid(SIDE), z.div_euclid(SIDE));
        if ground.columns.get(&id) != Some(&column) {
            game.terrain.ask_around(x, z, FOLLOW_CHUNKS);
            ground.changed = true;
        }
        now.insert(id, column);
    }
    if now.len() != ground.columns.len() {
        ground.changed = true;
    }
    ground.columns = now;

    if !ground.changed || ground.cycles % FORGET_EVERY != 0 {
        return;
    }
    ground.changed = false;
    let wanted: Vec<(i32, i32)> = ground.columns.values().copied().collect();
    let spawn = spawn_column();
    let reach = conductor_gameworld::view_chunks();
    let forgot = game.terrain.forget_all_but(|pos| near(pos, spawn, reach)
        || wanted.iter().any(|&column| near(pos, column, KEEP_CHUNKS)));
    if forgot > 0 {
        scribe::debug(Channel::Game, &format!("Let go of {forgot} chunks nobody is near.  {} held.",
                                              game.terrain.held()));
    }
}

/// The column of chunks the spawn point is in.
fn spawn_column() -> (i32, i32) {
    let (x, z) = SPAWN_POINTS[0];
    (x.div_euclid(SIDE), z.div_euclid(SIDE))
}

/// Whether the chunk at `pos` is within `reach` chunks of `column`, east,
/// west, north or south.
fn near(pos: ChunkPos, column: (i32, i32), reach: i32) -> bool {
    (pos.x - column.0).abs() <= reach && (pos.z - column.1).abs() <= reach
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_is_a_square_of_columns_every_row() {
        let pos = |x, z, row| ChunkPos { x, z, row };
        assert!(near(pos(2, -2, 0), (0, 0), 2));
        assert!(near(pos(2, -2, 10), (0, 0), 2));
        assert!(!near(pos(3, 0, 0), (0, 0), 2));
        assert!(!near(pos(0, -3, 0), (0, 0), 2));
    }

    #[test]
    fn the_server_holds_less_than_a_view_around_each_player() {
        assert!(KEEP_CHUNKS > FOLLOW_CHUNKS);
        assert_eq!(spawn_column(), (0, 0));
    }
}
