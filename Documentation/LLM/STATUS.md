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
accounts table, and the account desk.  `conductor-monitor` (lib) looks at the process and the machine once
a second.  `conductor-networking` (lib) is the front door: a login over TLS on TCP that hands a player a
ticket for UDP, the UDP side, a ledger of every connection, and the access lists.  `conductor-lua-parser`
(lib) runs the Lua scripts, locked down.  `conductor-primlib` (lib) is the game library, an ECS in memory.
**`conductor-gameworld`** (lib, folder `gameworld`) is GameWorld, the ground, new this session.
`conductor-gameclock` (lib) is the GameClock, the game loop: five checks of 50 ms to a 250 ms cycle, and it
owns primlib's `World` and GameWorld's `Terrain`.  `conductor-wgui` (lib) is the web admin at
`http://127.0.0.1:9996/Opus`.  `conductor-launcher` (bin) boots the program and waits on the web admin's
Server tab.  Ensemble is Unity 6000.6; its first project settings are on `main` (Jacob's commit), the rest
on his machine.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**: nobody
gets in before there's a voxel to step on.

**The branches**: `unstable` and `testing` are on this hand-off.  `main` is still at Jacob's Ensemble
project settings commit, behind both; it moves when Jacob says.

**Built and tested on Linux**: everything.  The only check left on TEST_CHECKLIST.html is the Parked
Windows one.  **On Windows**: built and runs, START SERVER included, without a database; the
world hasn't been made there yet.

## Jacob's map (2026-09-30)

**The 0.0.1 goal**: "get a player spawned in the world and able to chat."

His words, after the docs were tidied: "We are going to work on marrying the network code to the game by
finishing out character as a template for hydrating from an account.  Then we will build the character
selection (start of UDP connection), then the log in to the world, and spawn character in world."  And the
flow: "account logs in (done) -> character selection -> selected character spawns in world at its last save
loc (0,0,0 for now)".  His to change.

1. **The character as a template**, hydrated from an account.
2. **Character selection**, at the start of the UDP connection.
3. **Logging in to the world**, and the character spawned there, at its last saved spot (0,0,0 for now).

The map before this one (the tick, the world's voxels, zones) is done: the GameClock, `conductor-gameworld`,
and zones settled as regions.

## Last session -- 2026-09-30, the docs tidied

No code.  Jacob asked for the docs to be re-evaluated, the fat and old session notes trimmed, and the layout
brought up to date.  The README is a shorter front page now: where every piece stands and the 0.0.1 goal.
TODO.md, LONGTERM_TODO.md, CLAUDE.md and every design file keep each decision and Jacob's words, and drop
the story of how it was reached; the skeletons were checked against the code (GameWorld in the launcher's
order, networking opened from the command loop, `game.cfg`, ten crates).  `design/ecs-discussion.md` is gone.
The contracts (PROTOCOL.md, REGION_MAP.md) were checked byte for byte and agree with the code.  The stale
comments found in the code on the way are listed in TODO.md, for whichever session next touches each file.

## The session before -- 2026-09-30, the world

Designed with Jacob from nothing, then part one built, tested and passed the same day.  `design/world.md`
has every answer in his words; `REGION_MAP.md` is new.

- **The design**: the world is the total sum of everything, **seamless** (one `World`, one GameClock; zones
  on their own clocks were weighed and passed over), for 25 to 50 players.  **Blocky**, blocks **50 cm** a
  side ("A player is to be 4 blocks tall at 2 meters"), **chunks 32 a side** (16 m, cubes stacked in two
  rows), **8 km a side**, -8192 to 8191 blocks each way, 0,0,0 in the middle.  Dig to -15, build to +30,
  BEDROCK at -16.  A block holds its kind only, two bytes: AIR, DIRT, STONE, WOOD, GOLD (the block at
  0,0,0), BEDROCK.  **A region is a zone is a biome**, flagged per chunk.  The first world is **Alpha** to
  the west, flat (dirt at 0, stone to -15), and **Omega** to the east, smooth rolling hills of +/- 5, with a
  sharp divide between them ("in a real build ... we'll have a blending technique").
- **The files**, all in `Content/world/`, gitignored: `region.map`, binary, for the server and the client
  (`REGION_MAP.md`, Jacob's ask: "a thorough document that explains how to read our new binary map
  file"); `Regions/Omega/omega.heights`, Omega's hills saved once (134 MB, instead of 8 GB of chunks); and
  `Regions/<Region>/<region>_<x>_<z>_<row>.chunk` for a changed chunk, which always wins.  An untouched
  chunk isn't saved at all.
- **What's built**: GameWorld, a server piece on its own thread, makes the world on the first START SERVER
  (19 seconds on Jacob's machine), reads it on every one after, makes a lost heights file again from the
  seed, and hands the GameClock the chunks it asks for.  The GameClock holds them in a `Terrain` and asks
  for the ones within `view_chunks` (a new soft file, `game.cfg`) of 0,0,0, where every player starts.
- **Two fixes from Jacob's testing**: the door was open while the world was being made ("big bug xD"), so
  networking now starts from the launcher's command loop once `conductor_gameclock::ready()`; and "I don't
  want NPCs acting while the server world isn't ready", so until then only housekeeping runs.
- **What fought back**: nothing in the build.  Git did: Jacob's `Cargo.lock` commit was turned away
  because the session had pushed meanwhile; a `git pull --no-rebase` and a push to both branches fixed it,
  and CLAUDE.md now gives that line.
- **Tested by Jacob, all 12 passed**: the build, making the world, the files and their sizes, a restart
  reading it back, `view_chunks` at 8 (578 chunks), a lost heights file coming back with the same checksum,
  a stop part way through making, the door waiting and a stop while it waits, the thread, and only
  housekeeping before the ground is in.

## What's waiting

- **The 0.0.1 goal**: a player spawned in the world and able to chat.  Chat isn't designed (TODO.md).
- **The character, Part A, is built and tested** (no warnings, every test passing): `Template::take_in()`,
  the Living and Character templates (`primlib/src/gameobject.rs`), `PlayerCharacter`, every GameObject
  remembering its templates, `saved()` / `load()` per component, a save as Lua text (`primlib/src/save.rs`)
  and `read_save()` in lua-parser (which now depends on primlib).  Every check passed, START SERVER
  included.  The `Cargo.lock` change is Jacob's to commit.  `design/primlib.md` has the design.
- **The character, Part B**: the `player_characters` table, the slots and the functions in
  `conductor-accounts`.  TODO.md, under Protogame.
- **The world's part two**: saving changed chunks on STOP SERVER and every `save_minutes`, with the first
  thing that changes a block.  `design/world.md`.
- **Sending chunks to a client**, and how Ensemble gets `region.map`.  A protocol change.
- **Loading around players who move**, not just 0,0,0.
- **What goes in each of the GameClock's checks**: an input mailbox and an input packet (a protocol bump),
  a brain for the AI, movement into `Transform`, the broadcast.  `design/gameclock.md`.
- **What a region does**: what grows and spawns there.  And blending biomes where they meet (TODO.md's
  Ideas).
- **A Tick evaluator tab** under GAME MANAGEMENT.  In TODO.md.
- **Saving primlib's copies and their UUIDs and internal names**, and loading them back.  `design/primlib.md`.
- **The spawn system**, in the housekeeping check, waiting on `ready()` itself.  In TODO.md.
- **primlib in Lua** (part 2).  `design/primlib.md`.
- **The protogame database side**: `player_characters` and the three slots.  In TODO.md.
- **The blocked names list**, **playtime metrics**, **moving `wgui_port` into `wgui.cfg`**, **the stale
  words in the code**: in TODO.md.
- **Where the test client lives** and what it's called.
- Archivist retrying on its own while disconnected; the Debug switch in `conductor_globals.cfg`; catching
  Ctrl-C.
- The server on Windows with a database.  Parked in TEST_CHECKLIST.html.
- **Soundcheck**, the patcher, and a certificate for every client.  LONGTERM_TODO.md.
- **Ensemble**, its own session.  Its first project settings went onto `main` from Jacob's machine this
  morning, including `Assembly-CSharp*.csproj` and `Opus.Ensemble.slnx`, which Unity rewrites on every open
  and usually stay out of git; and the nested `Ensemble/dev/Opus.Ensemble/.gitignore` has no header.  A look
  together before more of it goes in: what a Unity project commits, where the purchased art goes, and LFS.
