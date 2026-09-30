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
`main` has two commits of Jacob's own from his machine (`bind_address = 10.0.0.84` in `networking.cfg`,
his machine's address and correct, and a merge); `unstable` took them in, so `main` is behind by this
session's doc commits only, and its next release is a plain catch-up to `testing`.

**Built and tested on Linux (Nobara 44)**: everything.  This session changed no code, so there's nothing
to build and nothing new on TEST_CHECKLIST.md, which is down to its Parked list.  The Windows code has
never been built.

## Last session -- 2026-09-30, a soft review and the protogame design talk

- **The soft review.**  Two chats had got mixed up (the other was deleted, and had pushed nothing).
  The repo was sound: code and docs agreed, the protocol version was 4 everywhere.  Four doc slips
  were fixed: CLAUDE.md said questions go at the top in one place (they go at the bottom), said
  "nothing hot swaps" after naming the access lists as the exception, and it and TODO.md still had
  kicking from the web admin as not built; the folder layout now names the two list files.
- **The protogame design talk**, no code.  Every answer went into TODO.md's protogame entry as it came
  and was pushed, so the entry is the whole of it.  Settled: the name is **protogame** again; Actor,
  Character and Agent live in a separate **game library**; a read-only **`CharacterSnapshot`** (name,
  where it is, from the last save) lives in `conductor-accounts` beside `Account`, since protogame in
  its place would make a circle of crates; a **`player_characters`** table, each row with its
  `account_id`; **three slots** on `accounts` (`character_slot_1` to `_3`, each a character's `id`, by
  migration); **`conductor-accounts` writes the SQL for both tables** (CLAUDE.md says so now);
  **Postgres does the wiping** (`ON DELETE CASCADE` from the account, `ON DELETE SET NULL` on a slot);
  the **first step is the database side** (the table, the slots, the functions).
- **Then Jacob stepped back: the game library is going to be an ECS**, and a character is a few
  components, not one type.  The database step waits on it, for what's in the row.  He's taking the
  ECS to a separate chat with a brief written at this close
  (`Documentation/LLM/design/ecs-discussion.md`), and bringing what comes out of it back here.

## Jacob's pick for next: the scripting language

Later the same day the separate ECS chat didn't work out, and the talk came back here.  Jacob wrote a
sample NPC in the scripting language he has in mind (LPC-like; in LONGTERM_TODO.md), and then paused
the ECS: **the scripting language comes first**, our own interpreter or an existing language, still
open.  The ECS notes so far are in TODO.md's protogame entry.  What follows below was written before
that, and holds for when the ECS opens again.


The next session here starts with what Jacob brings back from the ECS chat.  The brief asks that chat
to end with a summary in a fixed shape (decisions, open questions, a sketch); fold the decisions into
TODO.md's protogame entry and the brief, ask about anything that runs into a rule in CLAUDE.md, and
only then plan the first step (the database side, now with its columns).  The questions the brief
carries: written by hand or a crate (`bevy_ecs`, `hecs`: a dependency, heavy on generics and macros);
which components a character is and what makes an Actor; how a character's components are saved (a
column each on `player_characters`, a table per component, or one column holding them all); the tick
and the world's size; where the world and the voxels live.

## What's waiting

- **The scripting language**, above.  Jacob's pick.
- **Protogame** and the ECS, paused behind the language.
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
