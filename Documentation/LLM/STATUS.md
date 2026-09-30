<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is six crates.  `conductor-tools` (lib) holds DiskMan, Scribe, Constellations, Fingerprinter,
Security, Archivist, the notices, the clock, the thread list, the services list and the server's switch
(`server.rs`).  `conductor-accounts` (lib) is the one way in to the accounts table, and the account desk.
`conductor-monitor` (lib) looks at the process and every process on the machine once a second.
`conductor-networking` (lib) is the front door: a login over TLS on TCP that hands a player a ticket for
UDP, the UDP side the game will run on, a ledger of every connection at the door, and a whitelist and a
blacklist of addresses checked at the door.  `conductor-wgui` (lib) is the web admin at
`http://127.0.0.1:9996/Opus`, and the only way to start and stop the server and to shut Conductor down.
`conductor-launcher` (bin) boots the program and waits on the web admin's Server tab.  Ensemble is Unity 6000.6,
on Jacob's machine, not in the repo.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The web admin has a login: `admin` / `admin` does everything,
`user` / `user` looks, both passwords in `wgui.cfg`.  The server (Fingerprinter, Security, Archivist, the
account desk, networking, the monitor, and whatever comes later) only starts when START SERVER is pressed
on the web admin's Server tab (CONTROL PANEL > Server), and STOP SERVER takes it back down with Conductor still running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests, `main` is the
stable release, moved only when Jacob says.  At this close `unstable` and `testing` are on this hand-off.
Jacob released `main` mid-session (it took in his own `bind_address = 10.0.0.84`, his machine's address
and correct); it's behind `testing` by doc commits only, so its next release is a plain catch-up.

**Built and tested on Linux (Nobara 44)**: everything.  This session changed no code, so there's nothing
to build and nothing new on TEST_CHECKLIST.md, which is down to its Parked list.  The Windows code has
never been built.

## Last session -- 2026-09-30, a soft review, the protogame design talk, and Lua

No code.  Everything settled went into the docs and was pushed as it came (CLAUDE.md's rule now).

- **The soft review.**  Two chats had got mixed up (the other was deleted, and had pushed nothing).
  The repo was sound: code and docs agreed, the protocol version was 4 everywhere.  Four doc slips
  were fixed: CLAUDE.md said questions go at the top in one place (they go at the bottom), said
  "nothing hot swaps" after naming the access lists as the exception, and it and TODO.md still had
  kicking from the web admin as not built; the folder layout now names the two list files.
- **The protogame design talk.**  TODO.md's protogame entry is the whole of it.  Settled: the name is
  **protogame** again; Actor, Character and Agent live in a separate **game library**; a read-only
  **`CharacterSnapshot`** (name, where it is, from the last save) lives in `conductor-accounts` beside
  `Account`, since protogame in its place would make a circle of crates; a **`player_characters`**
  table, each row with its `account_id`; **three slots** on `accounts` (`character_slot_1` to `_3`,
  each a character's `id`, by migration); **`conductor-accounts` writes the SQL for both tables**
  (CLAUDE.md says so); **Postgres does the wiping** (`ON DELETE CASCADE` from the account, `ON DELETE
  SET NULL` on a slot); the first step there is the database side.
- **The ECS.**  Jacob stepped back: the game library is going to be component driven, an object packed
  with components that make it into something else, with a character having some baked properties.  A
  brief for a separate chat (`design/ecs-discussion.md`) didn't work out, and the talk came back here.
  Jacob wrote a sample NPC in an LPC-like script (LONGTERM_TODO.md): a template is a named set of
  components, `goblin_a` is a blueprint of their starting values, and `spawn goblin_a x 100` makes a
  hundred copies.  An object's behaviour is a behaviour script added to it; GOAP for the thinking,
  maybe.  Who sees what isn't a setting on the components: **the server decides what each client is
  sent** (a CLAUDE.md rule now).  Then he **paused the ECS behind the scripting language**.
- **The scripting language is Lua 5.4**, embedded through the **`mlua`** crate with Lua built in (Jacob
  said yes to the dependency).  A CLAUDE.md rule: a script never gets `io`, `os` or anything else that
  reaches the disk, the network or the database.

## Jacob's pick for next: Lua's first step

Agreed in outline at the close (Jacob: "yes"); the plan with files still comes first, per CLAUDE.md, and
waits for his OK:

- A new **lib** crate that embeds Lua 5.4 through `mlua` (vendored).  Jacob named it **`lua-parser`**.
  Ask first: is the crate `conductor-lua-parser` like the others (its folder losing `conductor-` with
  the rest when that TODO comes), or plain `lua-parser`?  And "parser" is a fair name for now, though
  `mlua` does the parsing and the crate mostly runs scripts; his call.
- It loads the `.lua` files from a folder under `Content/` through DiskMan (the folder's name isn't
  picked; `Content/scripts/` was offered) and runs each in a locked-down Lua.
- It reports to `services.rs`, its threads go through `threads::spawn()`, and it's in `start_server()`
  and `stop_server()`.
- A script with an error logs its file and line, and the server keeps running.
- The only thing a script can call is a log function, so a `hello.lua` shows on the Log tab.
- No ECS, no templates.  It proves Lua runs safely inside Conductor.
- `mlua` builds Lua with the C compiler; the first build changes `Cargo.lock`, so Jacob gets the
  commit commands for it.  Windows needs Visual Studio's compiler, untested like the rest of Windows.

## What's waiting

- **Lua's first step**, above.  Jacob's pick.
- **Protogame** and the ECS, paused behind the language.  TODO.md's protogame entry has all of it.
- **The blocked names list**, in TODO.md with his answers.
- **Drop `conductor-` from the crate folders**, folders only.  In TODO.md.
- **Playtime metrics**: a table of play sessions.  In TODO.md.
- **Move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`** (Jacob, 2026-09-29).  In TODO.md.
- **The stale words in the code**, in TODO.md (two more from this session: "the Network Admin tabs" in
  `json.rs` and the web admin's `Cargo.toml`).
- **Where the test client lives** and what it's called.  It's `Conductor/dev/conductor-networking/
  test_client.py` for now.
- Archivist retrying on its own every 5 seconds while disconnected.  Asked, not answered.
- The Debug switch in `conductor_globals.cfg`.
- Catching Ctrl-C.
- The Windows build, whenever getting to that machine is less of a hassle.
- **Ensemble**, its own session.  The Unity project is `Ensemble/dev/Opus.Ensemble/`, on Jacob's machine
  and untracked.  Before any of it is committed, a look together: what a Unity project commits, where the
  purchased art goes, and LFS for anything big.
