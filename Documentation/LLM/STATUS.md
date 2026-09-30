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
stable release, moved only when Jacob says.  At this close all three are on the same commit, this
hand-off: Jacob said to release once every check had passed, so `main` moved up from the
accounts-in-memory hand-off (`0a82d90`) to take in the account manager and the sections.

**Built and tested on Linux (Nobara 44)**: everything up to and including this session.  Jacob built
`testing` with the account manager and the sections on it (the build's `Cargo.lock` is committed) and went
through every check for both; all passed, and TEST_CHECKLIST.md is down to its Parked list.  The Windows
code has never been built.

## Last session -- 2026-09-29 to 30, the web admin's sections

Jacob's pick: the sidebar rethink, "a better menu for navigating as we are going to be adding more and
more stuff to the menu".

- **Four mockups first**, clickable, on a design canvas (claude.ai, kept off the repo on Jacob's say:
  "just on the canvas"): folding groups, an icon rail with a panel, sections across the top, and today's
  list with find and pins.  He picked sections across the top and named them himself: **CONTROL PANEL |
  CONFIGURATION | LOGS | ACCOUNT MANAGEMENT | GAME MANAGEMENT**.  Nothing remembered about the menu, no
  pins ("no need").
- **Built into `page.html`**, nothing else in Conductor changed.  The header is two rows (the name, pill,
  uptime and bell, then the sections); the side menu lists the open section's tabs, with a line saying
  why when they're all locked; the top of the tab says its section and name.  The Control Panel tab is
  **Server** now.  The notices are the bell and LOGS (his words): the tray has HISTORY, and opens even
  with nothing in it.  The Services dot flashes on CONTROL PANEL too.  The tabs, their locks, the
  routes and the remembered tab are as they were.  The table of what's under which section is in
  CLAUDE.md and `design/conductor-wgui.md`.
- **Blocked names**, asked for mid-session, went to TODO.md with Jacob's answers: a list of curse words
  in `Content/cfg/blocked_names.txt` beside the two access lists, checked against account usernames, a
  word of 4 letters or more blocking any name with the whole word in it ("Shitfox", "Bastardfox"), a
  shorter one blocking nothing, read on START SERVER.  A Blocked Names tab under CONFIGURATION.  Not
  built.

## Jacob's pick for next: the protocore

His words at the close: "write and build the first parts of the game protocore - the character and the
world, and the voxels so that you can select a character from login and be put into the world itself."
It's the protogame library in TODO.md, renamed.  By CLAUDE.md's rule of one small step a session, that's
more than one session's worth, so the first thing to settle is where the first step stops.  The
questions it opens with, none answered yet (the TODO.md entry has them too):

- **The crate**: its name (`conductor-protocore`?), a lib like every server piece, in `start_server()`
  and `stop_server()`, named in `services.rs`, its threads through `threads::spawn()`.
- **The character**: a `characters` table (its own schema file, `id` and `uuid`, pointing at its account
  by `id`), what's in it at first (a name, where it stands), how many an account can have, and who makes
  one: the player from the client, or the admin on the web admin the way accounts are made.  Held in
  memory while it's in the world, or never held like an account.
- **Selecting one**: at the TLS login (the character list comes back with the ticket) or after the UDP
  connect.  Either way a packet change, so `PROTOCOL_VERSION` goes to 5, with PROTOCOL.md, `protocol.rs`
  and `test_client.py` together.
- **The world and its voxels**: the size of a chunk and of the world, flat or generated, where it's kept
  (files in `Content/` through DiskMan, or the database), and what the client is sent to be "put into the
  world" (its position, the chunks around it; a chunk has to fit UDP packets).
- **The tick**: whether this is the game loop's start, and its tick rate (CLAUDE.md has a FILL IN for it).
- **What shows it working**: the test client listing characters, picking one and printing where it stands
  and what it was sent; the character beside the account on the Connections tab (TODO.md has that).
  Ensemble is still on Jacob's machine, not in the repo.

## What's waiting

- **The protocore**, above.  Jacob's pick.
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
