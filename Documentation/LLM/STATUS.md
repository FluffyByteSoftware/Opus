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
TCP that hands a player a ticket for UDP, the UDP side, character select and the spawn (Protogame), a
ledger of every connection, and the access lists.
`conductor-lua-parser` (lib) runs the Lua scripts, locked down, and reads saved GameObjects back.
`conductor-primlib` (lib) is the game library, an ECS in memory, with the Living and Character templates.
`conductor-gameworld` (lib) is GameWorld, the ground.  `conductor-gameclock` (lib) is the GameClock, the game
loop: five checks of 50 ms to a 250 ms cycle; it owns primlib's `World` and GameWorld's `Terrain`, takes
players' characters in and out through a mailbox, and saves the world.  `conductor-wgui` (lib) is the web
admin at `http://127.0.0.1:9996/Opus`.  `conductor-launcher` (bin) boots the program and waits on the web
admin's Server tab.

Ensemble is Unity 6000.6: its project settings and our four folders under `Assets/` (`Editor/`, `Code/`,
`Scripts/`, `Data/`) are committed, the rest is on Jacob's machine, the purchased art in
`Assets/Purchased/`.  It has one editor tool, Tools > Opus > Copy Anims From FBX Pack, and the HUD's Phase
1: widgets placed on the screen from a layout file (`design/ensemble-hud.md`).  No networking yet.  **The
game's name is Forgotten Legends** (Unity's Product Name); the project, its folders and code stay Opus.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` is still at `8bf9f70`, released at the hand-off of 2026-09-30.  `unstable` and
`testing` are level with each other and carry everything since, this session included.  `main` moves when
Jacob says.

**Built and tested on Linux**: all of Conductor, this session's 1 m blocks included.  **In Unity**: Copy
Anims From FBX Pack, its CLASHES view included, and the HUD's Phase 1, every check passed.
TEST_CHECKLIST.html has nothing left but the Parked Windows check.  **On Windows**: Conductor builds and
runs, START SERVER included, without a database; the world, the characters, character select, the world
save and the spawn haven't been tried there.

## Jacob's map (2026-09-30, and on)

**The 0.0.1 goal**: "get a player spawned in the world and able to chat."

His words: "We are going to work on marrying the network code to the game by finishing out character as a
template for hydrating from an account.  Then we will build the character selection (start of UDP
connection), then the log in to the world, and spawn character in world."  And the flow: "account logs in
(done) -> character selection -> selected character spawns in world at its last save loc (0,0,0 for
now)".  His to change.

1. **The character as a template**, hydrated from an account.  **Done.**
2. **Character selection**, at the start of the UDP connection.  **Done**: list, make, delete, reset home.
3. **Logging in to the world**, and the character spawned there, at its last saved spot.  **Done**
   (2026-10-01): the game library's half, then networking's.

His order after that (2026-10-01): "building the network infrastructure up then the chapter after that will
be testing wtih a py script then we're on to building the client", and "then we're ready to start testing
it with a real client".  Chat, the other half of 0.0.1, isn't designed yet.

On the world (2026-10-01, at this hand-off): **"we're gonna make the world twice as big in the next
session"**.  And earlier the same day: "we will test a mountain out after we get the client up".

## Last session -- 2026-10-01, blocks 1 m a side

Jacob: "we need to revise conductor voxels so that they are more in line with the size of a Minecraft
voxel".  The code counted in blocks only (nothing multiplied by 0.5), so most of it was settling what each
number means in metres.  His answers, all in `design/world.md`:

- **A block is 1 m, a cube**: "it will be simpler to start there for now and then we may make different
  non cubed voxels".  **A player is 2 blocks tall**, 2 m.  **Chunks stay 32 a side**, now 32 m.
- **The world stays 8 km by 8 km** ("I thought we bout 8k x 8k?"): blocks -4096 to 4095, chunks -128 to
  127, 256 by 256.
- **Minecraft's height, his own depth**: "we're gonna squeeze more memory and go Minecraft height and
  depth values for now", and "-31 is bedrock can dig to -30 and stand on top of -31".  So +319 is the top,
  -30 the deepest dig, and BEDROCK at -31 and -32 (the floor starts at -32 so the rows line up on 32s;
  his yes).  **Eleven rows of chunks**: row 0 is -32 to -1, row 1 is 0 to 31.  891 chunks around a player
  at `view_chunks` 4 (128 m), about 57 MB; most of them air, kept whole.
- **Omega's hills stay ±5** until a mountain is tried, after the client is up.
- `region.map` and the chunk file went to **version 2**: the same layout, other numbers, so an old world
  is turned away (an Error, the door shut, and the message says to delete `Content/world/`).  A chunk
  file's row is padded to two digits (`alpha_-015_003_01.chunk`).  REGION_MAP.md has the new worked
  example and a version history line.

Every check passed on the first build.  **What fought back**: not the code.  Jacob's machine locked up hard
the first time, and it was RustRover's code analysis running alongside the server; from a terminal it ran
clean.  CLAUDE.md now says Conductor is run from a terminal.

## The session before -- 2026-10-01, the HUD's Phase 1

Not handed off at the time, so here.  Jacob's brief (`HUD_LAYOUT_SYSTEM.md`) answered in
`design/ensemble-hud.md`, the two file formats' contract in `HUD_FORMATS.md`.  Phase 1 is the HUD built
from a layout file: `Assets/Code/Hud/` (the widgets, the registry, the loader, the checker, the builder),
`Assets/Scripts/Hud/HudRoot.cs`, the default layout in `Assets/Data/Layouts/`, the stylesheet in
`Assets/Data/Styles/`.  The player's own layout lives in `FluffyByte Studios/Opus.Ensemble/Unity/` beside
Unity's per-player folder, not under the game's name.  Every screen is made with the tool, and only the
player's HUD is the player's to change.  Sizes are reference pixels, scaled to the real screen.  All its
checks passed.  Phases 2 and 3 are in TODO.md.

## Where the next session starts

Jacob: **"make the world twice as big"**.  Ask before planning what "twice as big" means: twice as wide
each way (16 km a side, four times the ground: blocks -8192 to 8191, 512 by 512 chunks, Omega's heights
file back to about 134 MB and the world about four times as slow to make), or twice the ground (about
11.6 km a side, which isn't a whole number of 32-block chunks, so it'd be rounded to one).  What's known:
REGION_MAP.md says a bigger world is the same `region.map` version with bigger numbers (its size is in its
header), and chunk positions fit in an i16 up to ±32,767.  But Conductor reads the world it finds: making
it bigger means making a new one (delete `Content/world/`), unless growing the one that's there is what
Jacob wants, which is new code (more `region.map`, more heights, nothing moved).  The constants are
`FIRST_WEST` and friends in `gameworld/src/regionmap.rs`.

## What's waiting

- **Ensemble's client code**: the network client, and the HUD's Phases 2 and 3.  **Ensemble's project
  files in git** (Packages/, the .csproj files, LFS for scenes): TODO.md.
- **Chat**, the rest of the 0.0.1 goal.  Not designed (TODO.md).
- **What the client is sent after CharacterEnteredWorld**: the world around it (chunks, `region.map`),
  other players, movement.  `design/world.md`, `design/gameclock.md`.
- **Editing characters and NPCs** from GAME MANAGEMENT: whether an edit goes to the row or the copy in the
  world, what can be edited, the routes.  TODO.md.
- **The world's part two**: saving changed chunks.  **Loading around players who move**.  **An all-air
  chunk that costs nothing**, now that most of a player's 891 are air.  **A mountain**, once the client is
  up.  `design/world.md`, LONGTERM_TODO.md.
- **What goes in each of the GameClock's checks**: an input packet, a brain, movement, the broadcast.
  `design/gameclock.md`.
- **Saving primlib's copies** with their UUIDs and internal names (and the snapshot's cost when they join
  the world save: 32.88 ms for 10,000 characters), **the spawn system** for NPCs, **primlib in Lua** (part
  2).  `design/primlib.md`, TODO.md.
- **What a region does**, and blending biomes.  **A Tick evaluator tab**.  In TODO.md.
- **The blocked names list** (whether it checks character names too is open), **playtime metrics**,
  **moving `wgui_port` into `wgui.cfg`**, **the stale words in the code**, **where the test client lives**:
  in TODO.md.
- Archivist retrying on its own while disconnected; the Debug switch in `conductor_globals.cfg`; catching
  Ctrl-C.
- The server on Windows with a database.  Parked in TEST_CHECKLIST.html.
- **Soundcheck**, the patcher, and a certificate for every client.  LONGTERM_TODO.md.
- **A GDD**: Jacob is writing one with another chat.  What it settles comes in through him and goes into
  these docs.
