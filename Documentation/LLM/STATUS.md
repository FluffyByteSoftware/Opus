<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is eight crates, each in a folder without the `conductor-` in front (`Conductor/dev/tools/`)
while the crate keeps it (`conductor-tools`, `conductor_tools::` in code).  `conductor-tools` (lib) holds
DiskMan, Scribe, Constellations, Fingerprinter, Security, Archivist, the notices, the clock, the thread
list, the services list and the server's switch (`server.rs`).  `conductor-accounts` (lib) is the one way
in to the accounts table, and the account desk.  `conductor-monitor` (lib) looks at the process and every
process on the machine once a second.  `conductor-networking` (lib) is the front door: a login over TLS on
TCP that hands a player a ticket for UDP, the UDP side the game will run on, a ledger of every connection
at the door, and a whitelist and a blacklist.  `conductor-lua-parser` (lib, folder `lua-parser`) runs the
Lua scripts in `Content/scripts/`, locked down.  **`conductor-primlib`** (lib, folder `primlib`) is the
game library: entities, components, templates and blueprints, in memory, with nothing running it yet.
`conductor-wgui` (lib) is the web admin at `http://127.0.0.1:9996/Opus`, and the only way to start and
stop the server and to shut Conductor down.  `conductor-launcher` (bin) boots the program and waits on the
web admin's Server tab.  Ensemble is Unity 6000.6, on Jacob's machine, not in the repo.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The web admin has a login: `admin` / `admin` does everything,
`user` / `user` looks, both passwords in `wgui.cfg`.  The server (Fingerprinter, Security, Archivist, the
account desk, Lua, networking, the monitor, and whatever comes later) only starts when START SERVER is
pressed on the web admin's Server tab (CONTROL PANEL > Server), and STOP SERVER takes it back down with
Conductor still running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests, `main` is the
stable release, moved only when Jacob says.  At this close all three are on this hand-off: Jacob said to
release, so `main` has the Lua work and this session's.

**Built and tested on Linux (Nobara 44, Rust 1.98.1)**: everything, primlib included.  The first login from
outside went through (Jacob's work laptop over the internet, after the close).  TEST_CHECKLIST.html is
down to one Parked check, the Windows build.  The Windows code has never been built, Lua's C build included.

## Jacob's map (2026-09-30, at the close)

His words: "next session we build tick, then after that we build our world (voxel information and zone
management after that)".  Written down at his ask, so it isn't forgotten.  His to change.

1. **The world tick.**  The game loop that owns primlib's `World` and steps it forward.  TODO.md's entry
   ("The world tick") has what's open: the rate, what runs each tick and in what order, a tick that runs
   long, a fresh world on every START SERVER or not.  The plan with files comes first, per CLAUDE.md.
2. **The world: voxel information.**  What the world is made of.  Nothing is designed yet: chunk size,
   world size, flat or generated, where it's kept.  LONGTERM_TODO.md's "The world" entry.
3. **Zone management.**  The world split into zones.  Same entry.

Waiting beside the map, on the tick: saving the copies on STOP SERVER and loading them back on START
SERVER, each with its UUID and internal name (`design/primlib.md`), and the spawn system (TODO.md).

## Last session -- 2026-09-30, primlib and the folder rename

Written, built and tested the same day.  `design/primlib.md` has the whole of the game library.

- **The crate folders lost their `conductor-`** (Jacob: "rip the bandaid").  Folders, `path` lines,
  `File:` headers and docs only; the crate names didn't change.  CLAUDE.md has the rule for new crates.
- **`conductor-primlib`**, a lib crate, no dependencies (Jacob named it: "prim for primitive").  An ECS
  written by hand:
  - an **entity** is a slot and a generation, so an old handle to a despawned goblin can't reach the
    goblin that took its slot;
  - a **`Store<T>`** per kind of component, a slot per entity, in a **`World`** that spawns, despawns,
    adds, removes and reads;
  - ten **components**: `Transform` (position, rotation in degrees, scale; Y up; no parent), `Model` (the
    path the client loads it from), `PrimitiveShape` (the client's fallback, cube by default),
    `Animator` (a skeleton: `current_track`, `is_looping_currently`), `ShortName`, `LongName`, `Titles`,
    and `Health`, `Endurance` and `Mana` (each a `Pool` that starts full);
  - **templates** (`NPC`, a cheat sheet of components and defaults) and **blueprints** (`goblin_a`,
    which starts from a template and can change, add and drop), and `world.spawn(&goblin_a)` for a copy.
- **Jacob's answers along the way**, all in `design/primlib.md`: a template is a starting set, not a
  contract; a spawned copy lives in memory, is written to the database as its own row on STOP SERVER,
  and comes back on START SERVER "as if they never left"; every copy has a UUID and an internal name
  made from its short name (`goblin_1`, `goblin_2`; "goblin_1" gives `goblin_1_1`), built together with
  saving; the Lua part (`setup()` and `awake()`) comes after the Rust side.
- **What ran into a rule**: Jacob's first thought was a spawned goblin managed through its database row.
  That ran into the game loop never waiting on the database, and he turned it round into saving on STOP
  SERVER.
- **What fought back**: nothing in the code.  The whole thing was written unbuilt and compiled clean the
  first time.  One muddle: Jacob's checklist tab was the page from mid-Lua-session, never reloaded, so
  its message listed checks long gone.  A reload fixed it.
- **Tested by Jacob, all passed**: the rename's `ls`, a clean build with no warnings, every test (primlib's
  23 included), and a run with START and STOP SERVER showing the new folder names in the Caller.

## What's waiting

- **Jacob's map above**: the tick, then the world's voxels, then zones.
- **Saving the copies and their UUIDs and internal names**, and loading them back.  `design/primlib.md`.
- **The spawn system**: keeps the goblin_as topped up.  In TODO.md.
- **primlib in Lua** (part 2): templates and blueprints as scripts, `setup()` and `awake()`.
  `design/primlib.md`.
- **The protogame database side**: `player_characters` and the three slots.  In TODO.md; it waited on
  the ECS, which is in now.
- **The blocked names list**, in TODO.md with his answers.
- **Playtime metrics**: a table of play sessions.  In TODO.md.
- **Move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`** (Jacob, 2026-09-29).  In TODO.md.
- **The stale words in the code**, in TODO.md.
- **Where the test client lives** and what it's called.  It's `Conductor/dev/networking/test_client.py`
  for now.
- Archivist retrying on its own every 5 seconds while disconnected.  Asked, not answered.
- The Debug switch in `conductor_globals.cfg`.
- Catching Ctrl-C.
- The Windows build, whenever getting to that machine is less of a hassle.  Lua's C build is part of it.
- **Soundcheck** (`Opus.Soundcheck`), the patcher, and a certificate for every client (mutual TLS,
  one per client), after the first login from outside showed the test client going on unchecked
  without `--cert`.  LONGTERM_TODO.md.  Not a server bug: the server was never asking for one.
- **Ensemble**, its own session.  The Unity project is `Ensemble/dev/Opus.Ensemble/`, on Jacob's machine
  and untracked.  Before any of it is committed, a look together: what a Unity project commits, where the
  purchased art goes, and LFS for anything big.
