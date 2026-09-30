<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is nine crates, each in a folder without the `conductor-` in front (`Conductor/dev/tools/`)
while the crate keeps it (`conductor-tools`, `conductor_tools::` in code; the prefix is what tells our
crates apart from everybody else's, Jacob's reason).  `conductor-tools` (lib) holds DiskMan, Scribe,
Constellations, Fingerprinter, Security, Archivist, the notices, the clock, the thread list, the services
list and the server's switch (`server.rs`).  `conductor-accounts` (lib) is the one way in to the accounts
table, and the account desk.  `conductor-monitor` (lib) looks at the process and every process on the
machine once a second.  `conductor-networking` (lib) is the front door: a login over TLS on TCP that hands
a player a ticket for UDP, the UDP side the game will run on, a ledger of every connection at the door,
and a whitelist and a blacklist.  `conductor-lua-parser` (lib, folder `lua-parser`) runs the Lua scripts
in `Content/scripts/`, locked down.  `conductor-primlib` (lib, folder `primlib`) is the game library:
entities, components, templates and blueprints, in memory.  **`conductor-gameclock`** (lib, folder
`gameclock`) is the GameClock, the game loop: it owns a `World` and runs five checks of 50 ms to a 250 ms
cycle.  `conductor-wgui` (lib) is the web admin at `http://127.0.0.1:9996/Opus`, and the only way to start
and stop the server and to shut Conductor down.  `conductor-launcher` (bin) boots the program and waits on
the web admin's Server tab.  Ensemble is Unity 6000.6, on Jacob's machine, not in the repo.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The web admin has a login: `admin` / `admin` does everything,
`user` / `user` looks, both passwords in `wgui.cfg`.  The server (Fingerprinter, Security, Archivist, the
account desk, Lua, the GameClock, networking, the monitor) only starts when START SERVER is pressed on the
web admin's Server tab (CONTROL PANEL > Server), and STOP SERVER takes it back down with Conductor still
running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests, `main` is the
stable release, moved only when Jacob says.  At this close all three are on this hand-off: Jacob said to
merge to main, so `main` has the GameClock.

**Built and tested on Linux (Nobara 44, Rust 1.98.1)**: everything, the GameClock included.
TEST_CHECKLIST.html is down to one Parked check, the Windows build.  The Windows code has never been
built, Lua's C build included.

## Jacob's map (2026-09-30)

His words, at the close of the primlib session: "next session we build tick, then after that we build our
world (voxel information and zone management after that)".  Written down at his ask, so it isn't
forgotten.  His to change.

1. ~~**The world tick.**~~  **Done, 2026-09-30: the GameClock** (`design/gameclock.md`).  Its five checks
   are empty slots; filling them comes as the world gets things that move.
2. **The world: voxel information.**  What the world is made of.  Nothing is designed yet.
   LONGTERM_TODO.md's "The world: voxels and zones" entry has what's open: what a voxel holds, the chunk
   size, the world's size, flat or generated, where it's kept and when it's saved (the database, files
   through DiskMan, or both), what each client is sent (only the chunks near them, since the server
   decides what each client sees), and whether the world belongs to primlib beside its `World` of
   entities or is a piece of its own.  Anything bigger than a small fix gets a plan first, per CLAUDE.md,
   and a new crate gets its name from Jacob.
3. **Zone management.**  The world split into zones: what a zone is (a fixed square of chunks, or drawn
   by hand), what it's for (who hears what, what gets ticked, spawn areas, loading and unloading what
   nobody is near), and how the GameClock handles one.  Same entry.

Waiting beside the map: saving the copies on STOP SERVER and loading them back on START SERVER, each with
its UUID and internal name (`design/primlib.md`), and the spawn system (TODO.md), which goes in the
GameClock's housekeeping check when it comes.

## Last session -- 2026-09-30, the GameClock

Planned, written, built and tested the same day.  `design/gameclock.md` has the whole of it.

- **Jacob's design**, from an earlier go at this (a MUD, where it was "the heartbeat"): a full cycle is
  **250 ms**, cut into **five checks of 50 ms**, each touching its own group of objects.  With Argon2 on
  its one thread on the side, it never ran far over 250 ms.  He remembered it stashed in
  `conductor-tools`; it wasn't in Opus's history (Stratum or Mantle, most likely), so it was written fresh.
- **His answers**: the checks run input, AI, movement, broadcast, housekeeping ("I have no idea what order
  they should go in", so it can move); a late check makes the next one late and nothing is skipped; **the
  rate is fixed in code**, never a setting ("anything faster is gonna be a problem.  Slower is fine but
  faster becomes bad").
- **What's built**: `conductor-gameclock` (lib), a server piece on its own thread, `gameclock`.  It makes
  a fresh `World` on every START SERVER and owns it outright, no lock.  Each check is due 50 ms × n after
  its cycle started, so nothing drifts; the wait is `recv_timeout` on the stop channel.  A cycle whose
  last check finishes past 250 ms is late, and the next starts at once on a fresh schedule.  A late cycle
  is a Debug line; one a second or more over is a Warn, at most one a minute.  The Services tab's GameClock
  line counts cycles, late ones and the busiest.  The launcher starts it after Lua and before networking,
  and stops it after networking.  The five checks are empty.
- **The name went round once.**  Jacob said "`conductor_heartbeat` is the lib name in code, and the crate
  name is going to be gameclock", and it was built that way (a `[lib] name`).  Then: "it should be in code
  `conductor_gameclock::start()`", and the service is "the GameClock".  "Heartbeat" was his MUD's lingo
  and "doesn't seem professional" for Opus.  So no crate breaks the naming rule.
- **A Tick evaluator tab** under GAME MANAGEMENT is in TODO.md, Jacob's ask, for later.
- **What fought back**: nothing.  It compiled clean the first time; the rename compiled clean too.
- **Tested by Jacob, all passed**: a clean build with no warnings, every test (the GameClock's 8
  included), GameClock green on the Services tab at about 240 cycles a minute, its thread near nothing on
  the CPU, a test client login leaving the late count at 0, and STOP SERVER and START SERVER counting
  over from 0.

## What's waiting

- **Jacob's map above**: the world's voxels, then zones.
- **What goes in each of the GameClock's checks**: an input mailbox and an input packet (a protocol bump),
  a brain for the AI, movement into `Transform`, the broadcast (only what each player may see).
  `design/gameclock.md`.
- **A Tick evaluator tab** under GAME MANAGEMENT.  In TODO.md.
- **Saving the copies and their UUIDs and internal names**, and loading them back.  `design/primlib.md`.
- **The spawn system**: keeps the goblin_as topped up, in the housekeeping check.  In TODO.md.
- **primlib in Lua** (part 2): templates and blueprints as scripts, `setup()` and `awake()`.
  `design/primlib.md`.
- **The protogame database side**: `player_characters` and the three slots.  In TODO.md.
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
- **Soundcheck** (`Opus.Soundcheck`), the patcher, and a certificate for every client (mutual TLS, one per
  client).  LONGTERM_TODO.md.
- **Ensemble**, its own session.  The Unity project is `Ensemble/dev/Opus.Ensemble/`, on Jacob's machine
  and untracked.  Before any of it is committed, a look together: what a Unity project commits, where the
  purchased art goes, and LFS for anything big.
