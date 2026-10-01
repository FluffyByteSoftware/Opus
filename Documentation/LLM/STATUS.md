<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is ten crates, each in a folder without the `conductor-` in front (`Conductor/dev/tools/`) while
the crate keeps it (`conductor-tools`, `conductor_tools::` in code).  `conductor-tools` (lib) holds DiskMan,
Scribe, Constellations, Fingerprinter, Security, Archivist, the notices, the clock, the thread list, the
services list and the server's switch (`server.rs`).  `conductor-accounts` (lib) is the one way in to the
accounts table and `player_characters`, and the account desk.  `conductor-monitor` (lib) looks at the
process and the machine once a second.  `conductor-networking` (lib) is the front door: a login over TLS on
TCP that hands a player a ticket for UDP, the UDP side, character select (Protogame), a ledger of every
connection, and the access lists.
`conductor-lua-parser` (lib) runs the Lua scripts, locked down, and reads saved GameObjects back.
`conductor-primlib` (lib) is the game library, an ECS in memory, with the Living and Character templates.
`conductor-gameworld` (lib) is GameWorld, the ground.  `conductor-gameclock` (lib) is the GameClock, the game
loop: five checks of 50 ms to a 250 ms cycle; it owns primlib's `World` and GameWorld's `Terrain`, takes
players' characters in and out through a mailbox, and saves the world.  `conductor-wgui` (lib) is the web
admin at `http://127.0.0.1:9996/Opus`.  `conductor-launcher` (bin) boots the program and waits on the web
admin's Server tab.  Ensemble is Unity 6000.6; its first project settings are on `main` (Jacob's commit),
the rest on his machine.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` is still at `8bf9f70`, released at the hand-off of 2026-09-30.  `unstable` and
`testing` are level with each other and carry this session on top, Jacob's `Cargo.lock` commit included.
`main` moves when Jacob says.

**Built and tested on Linux**: everything, this session's code included.  The only check left on
TEST_CHECKLIST.html is the Parked Windows one.  **On Windows**: built and runs, START SERVER included,
without a database; the world, the characters, character select and the world save haven't been tried
there.

## Jacob's map (2026-09-30, and on)

**The 0.0.1 goal**: "get a player spawned in the world and able to chat."

His words: "We are going to work on marrying the network code to the game by finishing out character as a
template for hydrating from an account.  Then we will build the character selection (start of UDP
connection), then the log in to the world, and spawn character in world."  And the flow: "account logs in
(done) -> character selection -> selected character spawns in world at its last save loc (0,0,0 for
now)".  His to change.

1. **The character as a template**, hydrated from an account.  **Done.**
2. **Character selection**, at the start of the UDP connection.  **Done**: list, make, delete, reset home.
3. **Logging in to the world**, and the character spawned there, at its last saved spot (0,0,0 for now).
   **The game library's half is done** (2026-10-01): the GameClock's mailbox, saving, the world save.

His order for step 3 (2026-10-01): "building the network infrastructure up then the chapter after that will
be testing wtih a py script then we're on to building the client".  And at this hand-off: "Next session we
complete the loop and then we're ready to start testing it with a real client".

## This session -- 2026-10-01, networking's half of the spawn (built and tested)

Jacob: "CLAUDE its TIME to BUILD".  The loop: login, character select, UserPressPlay, the character in the
world, and out again with its player, saved.  Protocol version 6.  Jacob's answers are in TODO.md under
"Picking a character to play"; `design/conductor-networking.md`, "The spawn", has how it's built; the
checks passed, every one.  It built with no warnings and every test passed first time (22 in the
GameClock, 75 in networking, 39 in the web admin).  A player logs in, picks a character, and it's in the
world at its last save; it leaves with its player, saved.

## Last session -- 2026-10-01, the game library ready for the spawn

Jacob's ask: "clean up and prepare the game lib for session zero!", and of the two readings, "preparing the
game library for spawn".  The game library's half only, no packets: "Correct the next session will be
building the network infrastructure up".  Planned, talked through, built and checked.
`design/gameclock.md`, "Players and the world save", has all of it; `design/primlib.md`, "The character
and saving", has his words.

- **`conductor_gameclock::enter(blueprint)` and `leave(character_id)`** (`gameclock/src/players.rs`) leave
  a note in the GameClock's mailbox and come straight back.  The input check empties it: Enter spawns (a
  character already in the world is a Warn, and the one there stands), Leave saves and despawns at once
  ("when the character's registered as quit out the game removes them").  `enter()` turns away a blueprint
  whose `PlayerCharacter` has no row (id 0).  The slow part, reading the row and the save, is the caller's,
  on its own thread.  **Nothing calls them yet.**
- **The world save** (`gameclock/src/saving.rs`).  Jacob's answers, in the order they came: every 50th
  cycle (the wrong number), then "every 15 minutes of real time", then "a global save to happen where the
  world state is pushed in a tick cycle", then, worried "about blocking or lagging the regular tick" and
  asking "can it be streamed?", shown that the GameClock only copies: "let's make it every 2.5 minutes over
  all", a setting ("setting!  ha!"), 30 seconds to 30 minutes, the first "after the world is loaded and
  ready".  So `world_save_seconds` in `game.cfg` (150), counted from `ready()`: housekeeping copies every
  player's character in one cycle, and `characters::save_all()` turns each into Lua text and writes it on
  Archivist's thread, all in one transaction.  STOP SERVER saves the world one last time and waits up to
  10 seconds for the saves to land, after the loop has stopped.
- **A failed save is heard.**  Archivist doesn't log a job that fails; the failure is only in its
  `Pending`.  `characters::save()`'s comment said otherwise, and is fixed.  The GameClock keeps every save on
  its way and looks at each in housekeeping without waiting: an Error on the bell if it failed.
- **`conductor-accounts` depends on primlib now**, and the GameClock on accounts.
- **The snapshot's cost, measured**: 32.88 ms for 10,000 characters, about 3.3 microseconds each.  The
  guess beforehand was "a few milliseconds"; it was ten times that.  Fine for players; it matters when NPC
  copies join the world save (TODO.md, beside saving primlib's copies).

**What fought back**: nothing.  It built with no warnings and every test passed first time.

**Tested by Jacob, all passed**: every test in every crate (19 in the GameClock, 13 in accounts, 91 in
tools), the timing test, the setting on the Settings tab (29 and 1801 refused, 30 taken at a restart), the
first world save on the Services tab, and the last one logged on every STOP SERVER with nothing on the bell.

## The session before -- 2026-09-30, character select

It matters for the next one, which builds on it.  Character select is Protogame's (`protogame.rs` in
networking, its own thread): CharacterListRequest / CharacterListDelivery (with a playable byte per
character), CreateCharacter, DeleteCharacter (with the typed word), CharacterRequestResetHome, and the
general CommandAccepted / CommandRefused, PROTOCOL.md version 5.  Every ask carries a u32 ask number and the
book keeps the last answer, so a lost answer is sent again, never the ask done twice.  The character
functions (`accounts/src/characters.rs`) take the first empty slot, refuse a fourth, and only the player
deletes.  A save that won't load marks the character unplayable for the run.  The test client does all of
it: `--create`, `--delete`, `--delete-word`, `--reset-home`.

## Where the next session starts

Jacob's words: "Next session we complete the loop and then we're ready to start testing it with a real
client".  Before that he'd said "building the network infrastructure up then the chapter after that will be
testing wtih a py script".  What "the loop" covers is his to say; ask before planning.  What's on the table
(TODO.md, under Protogame, "Picking a character to play"): the packet that picks a character at character
select; Protogame loading the row and the save into a blueprint off the GameClock's thread (and turning an
unplayable character away) and calling `conductor_gameclock::enter()`; calling `leave()` whenever the UDP
session ends, for any reason; what the client is sent once it's in; the Connections tab and the log saying
"in the world" only for a player who is; a reset home of a character already in the world.  That's a
protocol bump (PROTOCOL.md, `protocol.rs` and `test_client.py` together).

## What's waiting

- **Spawning in the world**, networking's half of step 3: above.
- **Chat**, the rest of the 0.0.1 goal.  Not designed (TODO.md).
- **Editing characters and NPCs** from GAME MANAGEMENT: whether an edit goes to the row or the copy in the
  world, what can be edited, the routes.  TODO.md.
- **The world's part two**: saving changed chunks.  **Sending chunks to a client**, and how Ensemble gets
  `region.map`.  **Loading around players who move**.  `design/world.md`.
- **What goes in each of the GameClock's checks**: an input packet, a brain, movement, the broadcast.
  `design/gameclock.md`.
- **Saving primlib's copies** with their UUIDs and internal names (and the snapshot's cost when they join
  the world save), **the spawn system**, **primlib in Lua** (part 2).  `design/primlib.md`, TODO.md.
- **What a region does**, and blending biomes.  **A Tick evaluator tab**.  In TODO.md.
- **The blocked names list** (whether it checks character names too is open), **playtime metrics**,
  **moving `wgui_port` into `wgui.cfg`**, **the stale words in the code** (the launcher's boot line leaving
  out `game.cfg` is new), **where the test client lives**: in TODO.md.
- Archivist retrying on its own while disconnected; the Debug switch in `conductor_globals.cfg`; catching
  Ctrl-C.
- The server on Windows with a database.  Parked in TEST_CHECKLIST.html.
- **Soundcheck**, the patcher, and a certificate for every client.  LONGTERM_TODO.md.
- **Ensemble**, its own session.  Its first project settings are on `main`, including
  `Assembly-CSharp*.csproj` and `Opus.Ensemble.slnx`, which Unity rewrites on every open and usually stay
  out of git; and the nested `Ensemble/dev/Opus.Ensemble/.gitignore` has no header.  A look together before
  more of it goes in: what a Unity project commits, where the purchased art goes, and LFS.
- **A GDD**: Jacob is writing one with another chat.  What it settles comes in through him and goes into
  these docs.
