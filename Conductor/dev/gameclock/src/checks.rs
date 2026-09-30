//! File:       Opus/Conductor/dev/gameclock/src/checks.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The five checks, one to each 50 ms of a cycle, in the order they run.
//! Each one gets the world and does whatever it needs to on its own group
//! of objects.  They're all empty for now: nothing in the world moves yet,
//! and the protocol has no input packet.  Each says what goes in it.
//!
//! Jacob's order, 2026-09-30: what the players asked for comes in first,
//! then the AI decides, then everything moves, then the positions go out,
//! so a player is sent this cycle's positions and not last cycle's.
//! Housekeeping goes last.  He wasn't sure of the order ("I have no idea
//! what order they should go in"), so it can move.

use conductor_primlib::World;

/// One check: its name, for the log, and what it does.
pub struct Check {
    pub name: &'static str,
    // Rust note: `fn(&mut World)` is the type of a plain function that
    // takes the world, so `run` holds one of the functions below and
    // `(check.run)(&mut world)` calls it.
    pub run: fn(&mut World),
}

/// Every check, in the order they run.  Five, at 50 ms each, is what makes
/// the cycle 250 ms, so adding one makes every cycle longer.
pub const ALL: [Check; 5] = [
    Check { name: "input", run: input },
    Check { name: "AI", run: ai },
    Check { name: "movement", run: movement },
    Check { name: "broadcast", run: broadcast },
    Check { name: "housekeeping", run: housekeeping },
];

/// What the players asked for since the last cycle.  Networking gets it
/// over UDP, but only this thread touches the world, so the idea is a
/// mailbox networking drops the asks in and this empties.  The server
/// decides what each ask actually does.
fn input(_world: &mut World) {}

/// The AI's brains: each NPC with a brain decides what it wants to do.
/// No component for a brain yet.
fn ai(_world: &mut World) {}

/// Everything that decided to move, moves.  What input and the AI asked
/// for, checked against the world, lands in each `Transform`.
fn movement(_world: &mut World) {}

/// The positions go out: each player is sent what moved, and only what
/// that player may see.
fn broadcast(_world: &mut World) {}

/// The rest.  The spawn system topping up the goblins goes here when it
/// comes.
fn housekeeping(_world: &mut World) {}
