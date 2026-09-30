//! File:       Opus/Conductor/dev/gameclock/src/checks.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The five checks, one to each 50 ms of a cycle, in the order they run.
//! Each one gets the world and the terrain, and does whatever it needs to
//! on its own group of objects.  Only housekeeping does anything yet (it
//! takes in the chunks GameWorld has sent): nothing in the world moves,
//! and the protocol has no input packet.  Each says what goes in it.
//!
//! Until the ground around 0,0,0 is in, only housekeeping runs, so it can
//! take the chunks in.  The other four wait their turn and do nothing:
//! "I don't want NPCs acting while the server world isn't ready" (Jacob,
//! 2026-09-30).  The GameClock still beats through it.
//!
//! Jacob's order, 2026-09-30: what the players asked for comes in first,
//! then the AI decides, then everything moves, then the positions go out,
//! so a player is sent this cycle's positions and not last cycle's.
//! Housekeeping goes last.  He wasn't sure of the order ("I have no idea
//! what order they should go in"), so it can move.

use conductor_gameworld::Terrain;
use conductor_primlib::World;

/// One check: its name, for the log, and what it does.
pub struct Check {
    pub name: &'static str,
    // Rust note: `fn(&mut World, &mut Terrain)` is the type of a plain
    // function that takes the world and the terrain, so `run` holds one of
    // the functions below and `(check.run)(&mut world, &mut terrain)`
    // calls it.
    pub run: fn(&mut World, &mut Terrain),
    /// Whether it runs before the ground around 0,0,0 is in.
    pub before_ready: bool,
}

/// Every check, in the order they run.  Five, at 50 ms each, is what makes
/// the cycle 250 ms, so adding one makes every cycle longer.
pub const ALL: [Check; 5] = [
    Check { name: "input", run: input, before_ready: false },
    Check { name: "AI", run: ai, before_ready: false },
    Check { name: "movement", run: movement, before_ready: false },
    Check { name: "broadcast", run: broadcast, before_ready: false },
    Check { name: "housekeeping", run: housekeeping, before_ready: true },
];

/// What the players asked for since the last cycle.  Networking gets it
/// over UDP, but only this thread touches the world, so the idea is a
/// mailbox networking drops the asks in and this empties.  The server
/// decides what each ask actually does.
fn input(_world: &mut World, _terrain: &mut Terrain) {}

/// The AI's brains: each NPC with a brain decides what it wants to do.
/// No component for a brain yet.
fn ai(_world: &mut World, _terrain: &mut Terrain) {}

/// Everything that decided to move, moves.  What input and the AI asked
/// for, checked against the world, lands in each `Transform`.
fn movement(_world: &mut World, _terrain: &mut Terrain) {}

/// The positions go out: each player is sent what moved, and only what
/// that player may see.
fn broadcast(_world: &mut World, _terrain: &mut Terrain) {}

/// The rest.  The chunks GameWorld has finished with come into the
/// terrain here.  This is the one check that runs before the ground is
/// in, so the spawn system topping up the goblins, when it comes here,
/// has to wait for `crate::ready()` itself: no NPC acts before there's
/// ground under it.
fn housekeeping(_world: &mut World, terrain: &mut Terrain) {
    terrain.take_arrivals();
}
