<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is four crates.  `conductor-tools` (lib) holds DiskMan, Scribe, Constellations, Fingerprinter,
Archivist, the notices, the clock, the thread list, the services list and, new this session, the server's
switch (`server.rs`).  `conductor-monitor` (lib) looks at the process and every process on the machine once
a second.  `conductor-wgui` (lib) is the web admin at `http://127.0.0.1:9996/Opus`, and the only way to
start and stop the server and to shut Conductor down.  `conductor-launcher` (bin) brings up the program and
waits on the Control Panel.  Ensemble hasn't been started.

**Conductor and the server are two things now.**  The program (DiskMan, Scribe, Constellations, the web
admin) is up from the moment the launcher runs.  The server (Fingerprinter, Archivist, the monitor, and
whatever comes later) only starts when START SERVER is pressed on the web admin's Control Panel, and STOP
SERVER takes it back down with Conductor still running.

**Last built and tested on Linux (Nobara 44), 2026-09-28**, before the Fingerprinter and before this
session.  Fingerprinter (the session after that, commits `384cf12` and `b618512`) and this session's work
haven't been built by Jacob yet.  The Windows code has never been built.

## Last session -- 2026-09-29

The Control Panel: the page Conductor greets you with, and the server's switch.

What we did:

- **`server.rs`** in `conductor-tools`: the server's state (stopped, starting, running, stopping, with a
  note and since when) and a mailbox.  `ask(Start | Stop | Restart)` is the web admin's side: it checks the
  ask fits the state, flips it to starting or stopping on the spot (so a second click is turned away), and
  posts the command.  `next_command()` is the launcher's side.  `set()` is the launcher saying where it
  got to.
- **The launcher** now boots DiskMan, Scribe, Constellations and the web admin, then sits on the mailbox.
  `start_server()` is Fingerprinter, Archivist, the monitor; `stop_server()` is the same in reverse.  Those
  two functions are the list of what the server is.  SHUT DOWN ends the web admin's thread as before; main
  sees that (`conductor_wgui::has_ended()` replaces `wait()`), stops the server if it's running, and waits
  on DiskMan.
- **Archivist and the monitor can stop and start again.**  Archivist's settings used to sit in a write-once
  `OnceLock`; now they ride with the worker onto its thread, and `postgres.cfg` is read again on every start.
  A start while already running is a Warn and does nothing.  The monitor's stop drops its last look, so the
  page has no numbers while the server is stopped.  Fingerprinter got a `stop()` that only tells the
  Services tab.
- **Three routes**: `POST /Opus/server/start`, `/stop`, `/restart`, needing `X-Opus: server`.  Each answers
  right away (200, or 409 with "Not now.  The server is running.") and the launcher does the work.  The
  status JSON starts with `"server": { "state", "note", "since" }`.
- **The Control Panel tab**, first in the sidebar: the state big, the note, since when, START SERVER,
  RESTART SERVER, STOP SERVER and SHUT DOWN, and a short services list beside it.  While the server isn't
  running only the Control Panel and the Log can be clicked, the bell is hidden, and the header pill reads
  SERVER STOPPED.  When it turns running the page moves to the remembered tab (Conductor by default) and
  everything works as before, database lock included.  The Control Panel and the Log sit outside that
  lock, so the server can be stopped while the database is offline and the log read to see why.
- **SHUT DOWN is on the Control Panel only.**  The header's button went.
- **Only rendered in a headless browser with made-up numbers**, not run against Conductor.  Every
  transition (stopped, starting, running with the database connecting, online, stopping, stopped again)
  showed the right tabs, pill, bell and buttons, with no script errors.  Nothing in Rust has been compiled:
  Jacob builds.

What fought back:

- The database lock used to lock the whole sidebar with one class and `inert`.  It's per tab now
  (`lockTabs()`), since the Control Panel has to stay clickable under it while the others don't.
- The header already had SHUT DOWN, and the Control Panel got one too.  Jacob picked the Control Panel's,
  and the header's went.

What Jacob decided (from the ask that opened the session):

- A control panel page is what the web admin greets you with.  Only the log is reachable in the tabs until
  the server is running.  No notifications while nothing is started.  START / RESTART / STOP SERVER and
  SHUTDOWN.  When the server is running, the page is the pages as they were.
- The Log stays open under the database lock too.  The header's SHUT DOWN goes; the Control Panel is
  where STOP and SHUT DOWN live.

## What's waiting

- **Jacob to build and run this session and the Fingerprinter session.**  `cargo build`, `cargo test`,
  then the page: press START SERVER and watch the Services list, then STOP, then START again (Archivist
  reconnects, the monitor's uptime starts over), then SHUT DOWN.
- Archivist retrying on its own every 5 seconds while disconnected.  Asked, not answered.
- DiskMan: seeing hand edits to a file it already holds.  In TODO.
- The Windows build, whenever getting to that machine is less of a hassle.
- The Debug switch in `conductor_globals.cfg`, and moving the routine log lines to Debug.  Every server
  start and stop adds a few Info lines now.
- Catching Ctrl-C.
- `\dt` in psql to confirm `archivist_migrations` exists, and the uuid migration ran.
- Accounts, which wait on Security for Argon2.  Security itself.
- Picking Ensemble's engine.
