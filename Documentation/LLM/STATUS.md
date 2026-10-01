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

**Built and tested on Linux**: all of Conductor, this session's `world_size` included.  **In Unity**: Copy
Anims From FBX Pack, its CLASHES view included, and the HUD's Phase 1, every check passed.
TEST_CHECKLIST.html is empty: the Windows check that sat Parked there is GitHub issue #10 now (Jacob opened
it, "Windows x86/x64 Untested").  **On Windows**: Conductor builds and runs, START SERVER included, without
a database; the world, the characters, character select, the world save and the spawn haven't been tried
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
3. **Logging in to the world**, and the character spawned there, at its last saved spot.  **Done**
   (2026-10-01): the game library's half, then networking's.

His order after that (2026-10-01): "building the network infrastructure up then the chapter after that will
be testing wtih a py script then we're on to building the client", and "then we're ready to start testing
it with a real client".  Chat, the other half of 0.0.1, isn't designed yet.

On the world (2026-10-01): "we will test a mountain out after we get the client up".

At this session's hand-off (2026-10-01): **"wrap up and prepare to go back to the client"**.

## Last session -- 2026-10-01, the world's size in game.cfg

Jacob: "I want to double the world size to see how it affects RAM".  Twice as wide each way, "no impact on
height", replacing the old world.  Before it was built he turned it round: "let's make this a variable we
can change in game.cfg".  His answers, all in `design/world.md` under "The shape":

- **`world_size` in `game.cfg`**, "world_size = 1 = 1024 blocks", **2 to 32, 16 to start** (16,384 blocks a
  side, chunks -256 to 255).  Half either side of 0, so every whole number is whole chunks.
- **A change deletes the world and makes a new one** ("Delete the world on disk and recreate") on the next
  START SERVER: GameWorld sees `region.map` isn't that size, says so at Info, deletes `Content/world/`
  through DiskMan with `region.map` last, and makes a new world from a new seed.  This runs into "a chunk's
  own file always wins" once digging is saved; Jacob's yes to keeping the seed and the dug chunks then
  (TODO.md).
- `region.map` stays version 2: the size was always in its header.  REGION_MAP.md's worked example is a
  `world_size` 16 world now.

**Measured**: Jacob tried 2, 4, 8, 16 and 32.  At 32 the whole of Conductor sat around 800 MB, Omega's
heights 537 MB of it: "I have so much room to work with.  I think I'm really just CPU limited."  Every check
passed on the first build.

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

Jacob: **"prepare to go back to the client"**.  Ensemble has the HUD's Phase 1 and no networking.  What's
there to pick from is under "What's waiting": the network client, the HUD's Phases 2 and 3, Ensemble's
project files in git.  Ask which first.

## What's waiting

- **Ensemble's client code**: the network client, and the HUD's Phases 2 and 3.  **Ensemble's project
  files in git** (Packages/, the .csproj files, LFS for scenes): TODO.md.
- **Chat**, the rest of the 0.0.1 goal.  Not designed (TODO.md).
- **What the client is sent after CharacterEnteredWorld**: the world around it (chunks, `region.map`),
  other players, movement.  `design/world.md`, `design/gameclock.md`.
- **Editing characters and NPCs** from GAME MANAGEMENT: whether an edit goes to the row or the copy in the
  world, what can be edited, the routes.  TODO.md.
- **The world's part two**: saving changed chunks, and then **a `world_size` change that keeps the
  digging**.  **A character saved outside a smaller world**.  **Loading around players who move**.  **An all-air
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
- The server on Windows with a database.  GitHub issue #10, "Windows x86/x64 Untested" (Jacob opened it).
- **Soundcheck**, the patcher, and a certificate for every client.  LONGTERM_TODO.md.
- **A GDD**: Jacob is writing one with another chat.  What it settles comes in through him and goes into
  these docs.
