<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is seven crates.  `conductor-tools` (lib) holds DiskMan, Scribe, Constellations, Fingerprinter,
Security, Archivist, the notices, the clock, the thread list, the services list and the server's switch
(`server.rs`).  `conductor-accounts` (lib) is the one way in to the accounts table, and the account desk.
`conductor-monitor` (lib) looks at the process and every process on the machine once a second.
`conductor-networking` (lib) is the front door: a login over TLS on TCP that hands a player a ticket for
UDP, the UDP side the game will run on, a ledger of every connection at the door, and a whitelist and a
blacklist of addresses checked at the door.  **`lua-parser`** (lib, `conductor-lua-parser` in code, the
first folder without `conductor-`) runs the Lua scripts in `Content/scripts/`, locked down.
`conductor-wgui` (lib) is the web admin at `http://127.0.0.1:9996/Opus`, and the only way to start and stop
the server and to shut Conductor down.  `conductor-launcher` (bin) boots the program and waits on the web
admin's Server tab.  Ensemble is Unity 6000.6, on Jacob's machine, not in the repo.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The web admin has a login: `admin` / `admin` does everything,
`user` / `user` looks, both passwords in `wgui.cfg`.  The server (Fingerprinter, Security, Archivist, the
account desk, Lua, networking, the monitor, and whatever comes later) only starts when START SERVER is
pressed on the web admin's Server tab (CONTROL PANEL > Server), and STOP SERVER takes it back down with
Conductor still running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests, `main` is the
stable release, moved only when Jacob says.  At this close `unstable` and `testing` are on this hand-off,
Jacob's `Cargo.lock` commit (mlua) included.  `main` is still on the 2026-09-30 hand-off before this one:
this session's Lua work hasn't been released.

**Built and tested on Linux (Nobara 44, Rust 1.98.1)**: everything, Lua included.  TEST_CHECKLIST.html
is down to its two Parked checks.  The Windows code has never been built, and now that includes Lua's C
build, which needs Visual Studio's compiler there.

## Last session -- 2026-09-30, Lua's first step, and the checklist as a page

Written, built and tested the same day.  `design/lua-parser.md` has the whole of it.

- **`lua-parser`**, a lib crate: `mlua` 0.12 with Lua 5.4 built from source (Jacob said yes to the
  dependency the session before).  The first build compiled Lua with `gcc` without trouble.
- On START SERVER its `lua` thread runs every `.lua` under `Content/scripts/` (Jacob's pick for the root
  of the scripts, folders inside it and all) once, each in a fresh locked-down Lua, then checks in once a
  second.  `Content/scripts/hello.lua` says hello on the Log tab.  RESTART SERVER runs them again.
- **What a script gets**: `string`, `table`, `math`, `utf8`, `coroutine`, and the log
  (`log.debug/info/warn/error`, `print` as debug).  Never `io`, `os`, `package`, `debug`, `dofile`,
  `loadfile`, `load`, `warn` or `string.dump`.
- **Jacob's rules for it**: anything wrong from Lua is a Warn, never an Error (so `log.error` writes a
  Warn), and a script can't crush what's underneath.  So a 1 s time limit, 64 MB, and 50 log lines a run
  ("we'll go with your suggestion for now").  A broken script turns the Lua row red on the Services tab,
  and he wants it that way: "the log points you to where it's broken."
- New in the tools: the `Script` channel in Scribe, the "Lua" service, Lua in `start_server()` (before
  networking) and `stop_server()` (after it).
- **What fought back**: one slip of mine.  Lua went into `EXPECTED` in `services.rs` but not into the
  test that lists every expected service, so `cargo test` failed until it did.  CLAUDE.md now says a new
  service goes in both.
- **Tested by Jacob, all passed**: the crate's 8 tests, a build with no warnings, the hello line, the
  Services row, four throwaway scripts (a syntax error, a log flood, a runaway loop, a sandbox check) all
  doing what they should, and hello alone again once they were taken out.
- **TEST_CHECKLIST.md became TEST_CHECKLIST.html** (his ask): a page opened from the disk, a box per
  check, ticks kept in his browser only, a COPY button on every command, and a message panel that
  writes "these passed" as he ticks, for him to paste back.  No failed box: a failure gets told in the
  chat ("if it fails I'm gonna bitch!").  CLAUDE.md says how a session adds a check.
- **Like Unity, maybe**: asked where scripts live, Jacob wondered about attaching a script to an object
  that fires off behaviour, or having it come prepackaged.  He's thinking it over; LONGTERM_TODO.md has
  it, and it's the game library's question as much as Lua's.

## The session before -- 2026-09-30, the protogame design talk

It matters for the game library.  TODO.md's protogame entry is the whole of it: **protogame** is the
name; Actor, Character and Agent live in a separate **game library**, which is **component driven** (an
ECS: an object packed with components that make it into something else, a character with some baked
properties); a read-only **`CharacterSnapshot`** lives in `conductor-accounts`; a
**`player_characters`** table and **three slots** on `accounts`, both written by `conductor-accounts`;
Postgres does the wiping.  Jacob's sample NPC (LONGTERM_TODO.md) has three layers: the template (`NPC`,
which components), the blueprint (`goblin_a`, their starting values), and the copies in the world
(`spawn goblin_a x 100`).  **The server decides what each client sees.**

## Jacob's pick for next

At the close, once the last checks passed and the testing session he'd planned wasn't needed:
"constructing the first primitive components and the ECS for game objects."  Everything settled and
open for it is in TODO.md's protogame entry and LONGTERM_TODO.md's scripting entry (Jacob's NPC sample,
the three layers, the Unity question).  The plan with files comes first, per CLAUDE.md.

## What's waiting

- **The game library** and its ECS, unpaused now that Lua is in.  TODO.md's protogame entry and
  LONGTERM_TODO.md's scripting entry have everything settled and open, the Unity question included.
- **The blocked names list**, in TODO.md with his answers.
- **Drop `conductor-` from the other crate folders**, folders only.  In TODO.md.  `lua-parser` was made
  without it.
- **Playtime metrics**: a table of play sessions.  In TODO.md.
- **Move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`** (Jacob, 2026-09-29).  In TODO.md.
- **The stale words in the code**, in TODO.md.
- **Where the test client lives** and what it's called.  It's `Conductor/dev/conductor-networking/
  test_client.py` for now.
- Archivist retrying on its own every 5 seconds while disconnected.  Asked, not answered.
- The Debug switch in `conductor_globals.cfg`.
- Catching Ctrl-C.
- The Windows build, whenever getting to that machine is less of a hassle.  Lua's C build is part of it
  now.
- **Ensemble**, its own session.  The Unity project is `Ensemble/dev/Opus.Ensemble/`, on Jacob's machine
  and untracked.  Before any of it is committed, a look together: what a Unity project commits, where the
  purchased art goes, and LFS for anything big.
