//! File:       Opus/Conductor/dev/gameclock/src/checks.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The five checks, one to each 50 ms of a cycle, in the order they run.
//! Each one gets the game (the world, the terrain, the players and the
//! saves on their way) and does whatever it needs to on its own group of
//! objects.  Input brings players' characters in and out and judges
//! where their clients say they walked them, movement stops a walker whose
//! client has gone quiet, broadcast sends out the chat, answers `/who`
//! and tells each player what they see of the world (and any pull-back),
//! and housekeeping takes in the chunks GameWorld has sent, keeps the
//! ground following the players, and saves the world.  Each says what
//! goes in it.
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

use std::time::Instant;

use crate::Game;

/// One check: its name, for the log, and what it does.
pub struct Check {
    pub name: &'static str,
    // Rust note: `fn(&mut Game)` is the type of a plain function that
    // takes the game, so `run` holds one of the functions below and
    // `(check.run)(&mut game)` calls it.
    pub run: fn(&mut Game),
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

/// What came in since the last cycle: players' characters coming into
/// the world and leaving it, from the mailbox (`players.rs`), one leaving
/// saved on its way out; then where each player's client says it walked
/// its character, judged in the order it came, each one taken or pulled
/// back (`movement.rs`).  Once a cycle, not every 50 ms: what's taken
/// only goes out to anybody in the broadcast, and the player walking sees
/// their own move at once on their own screen.
fn input(game: &mut Game) {
    let leaving = game.players.take_notes(&mut game.world);
    game.writes.send_leaving(leaving);
    crate::movement::take_moves(game);
}

/// The AI's brains: each NPC with a brain decides what it wants to do.
/// No component for a brain yet.
fn ai(_game: &mut Game) {}

/// Everything that decided to move, moves.  A player's move lands in its
/// character's `Transform` as input takes it, so today this only stops a
/// character whose client has gone quiet mid-walk (`movement.rs`).  NPCs
/// walking (0.0.3) will be moved here.
fn movement(game: &mut Game) {
    crate::movement::stop_the_quiet(game);
}

/// What the players are told goes out.  The chat said since the last
/// cycle, to everybody in the world (`chat.rs`); the answer to each
/// `/who` asked, to the one who asked (`who.rs`); and what each player
/// sees of the world (`view.rs`): what came into their view whole, what
/// moved, what's gone, and once a second a roll call, only what that
/// player may see; and each pull-back to the player it's for.
fn broadcast(game: &mut Game) {
    crate::chat::broadcast();
    crate::who::answer(game);
    crate::view::broadcast(game);
}

/// The rest.  The chunks GameWorld has finished with come into the
/// terrain here, the ground is asked for around players who walked into
/// another column and let go of where nobody is (`ground.rs`, once the
/// ground around the spawn point is in), the saves on their way to the
/// database are looked at, and the world is saved when it's due
/// (`saving.rs`).  This is the one
/// check that runs before the ground is in, so the spawn system topping
/// up the goblins, when it comes here, has to wait for `crate::ready()`
/// itself: no NPC acts before there's ground under it.  The world save
/// waits on its own: it isn't due until the ground is in.
fn housekeeping(game: &mut Game) {
    game.terrain.take_arrivals();
    if crate::ready() {
        crate::ground::follow(game);
    }
    game.writes.check();
    if game.world_save.due(Instant::now()) {
        crate::save_world(game);
    }
}
