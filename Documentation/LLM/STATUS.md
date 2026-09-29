<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is four crates.  `conductor-tools` (lib) holds DiskMan, Scribe, Constellations, Fingerprinter,
Security, Archivist, the notices, the clock, the thread list, the services list and, new this session, the
server's switch (`server.rs`).  `conductor-monitor` (lib) looks at the process and every process on the
machine once a second.  `conductor-wgui` (lib) is the web admin at `http://127.0.0.1:9996/Opus`, and the
only way to start and stop the server and to shut Conductor down.  `conductor-launcher` (bin) boots the
program and waits on the Control Panel.  Ensemble hasn't been started.

**Conductor and the server are two things now.**  The program (DiskMan, Scribe, Constellations, the web
admin) is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the
monitor, and whatever comes later) only starts when START SERVER is pressed on the web admin's Control
Panel, and STOP SERVER takes it back down with Conductor still running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests (the session
pushes `unstable` onto it when a round is ready), `main` is the stable release, moved by Jacob alone.  This
session was cut from `main` by mistake, before Security and the branch change reached it, and merged
`testing` back in at the end; the session branch `claude/gracious-ramanujan-j5ojyq` is history now.

**Last built and tested on Linux (Nobara 44), 2026-09-29**, with Security in it, before this session.  This
session's work hasn't been built by Jacob yet.  The Windows code has never been built.

**The tools are done**, as far as anything is done.  What comes next is the server proper, starting with
networking, which goes in `start_server()` and `stop_server()` like the rest.

## Last session -- 2026-09-29 (the second that day)

The Control Panel: the page Conductor greets you with, and the server's switch.  This is the "Manage
System" screen the Security session's hand-off said was next; Jacob called it the control panel when it
opened, so that's its name.

What we did:

- **`server.rs`** in `conductor-tools`: the server's state (stopped, starting, running, stopping, with a
  note and since when) and a mailbox.  `ask(Start | Stop | Restart)` is the web admin's side: it checks the
  ask fits the state, flips it to starting or stopping on the spot (so a second click is turned away), and
  posts the command.  `next_command()` is the launcher's side.  `set()` is the launcher saying where it
  got to.
- **The launcher** now boots DiskMan, Scribe, Constellations and the web admin, then sits on the mailbox.
  `start_server()` is Fingerprinter, Security, Archivist, the monitor; `stop_server()` is the same in
  reverse.  Those two functions are the list of what the server is.  SHUT DOWN ends the web admin's thread
  as before; main sees that (`conductor_wgui::has_ended()` replaces `wait()`), stops the server if it's
  running, and waits on DiskMan.
- **Archivist and the monitor can stop and start again.**  Archivist's settings used to sit in a write-once
  `OnceLock`; now they ride with the worker onto its thread, and `postgres.cfg` is read again on every start.
  A start while already running is a Warn and does nothing.  The monitor's stop drops its last look, so the
  page has no numbers while the server is stopped.  Fingerprinter got a `stop()` that only tells the
  Services tab.  Security could already do it, so it went in as it was: its arena comes and goes with the
  server.
- **Three routes**: `POST /Opus/wwwhook/start`, `/stop`, `/restart`, needing `X-Opus: server`.  Each answers
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

- The session was cut from `main`, which was eleven commits behind `testing` (the whole Security session
  and the new branch rules).  Jacob saw the old page on his machine and said so.  Merged `testing` in:
  eight files conflicted, all of them the two sessions adding lines next to each other, and Security went
  into `start_server()` and `stop_server()` where the old main had started it at boot.
- The database lock used to lock the whole sidebar with one class and `inert`.  It's per tab now
  (`lockTabs()`), since the Control Panel and the Log have to stay clickable under it while the others don't.
- The header already had SHUT DOWN, and the Control Panel got one too.  Jacob picked the Control Panel's,
  and the header's went.

What Jacob decided:

- A control panel page is what the web admin greets you with.  Only the log is reachable in the tabs until
  the server is running.  No notifications while nothing is started.  START / RESTART / STOP SERVER and
  SHUTDOWN.  When the server is running, the page is the pages as they were.
- The Log stays open under the database lock too.  The header's SHUT DOWN goes; the Control Panel is
  where STOP and SHUT DOWN live.
- The three server routes were built without asking first (they're the buttons Jacob asked for); he was
  looking them over when the session ended.

## The session before -- 2026-09-29, Security

Security (`security.rs`): Argon2id on one worker thread with a 64 MiB arena kept for the server's life,
one pass, one login hashed at a time with everybody else in line and told their place (`Ticket`).  30 ms a
login in release.  Jacob called the tools done at the end of it, and changed the branches to `unstable`,
`testing` and `main` (the rules are in CLAUDE.md).  The details are in the tools design doc.

## What's waiting

- **Jacob to build and run this session** from `testing`.  `cargo build`, `cargo test`, then the page:
  press START SERVER and watch the Services list (Security's arena and Archivist's connect included), STOP,
  START again (Archivist reconnects, the monitor's uptime starts over), then SHUT DOWN.
- Jacob's word on the three server routes.
- Whether Security belongs to the server (comes and goes with START / STOP, as built) or to the program
  (up from boot, its arena always allotted).  Built as the server; Jacob's call.
- On GitHub, by hand: delete `claude/gracious-ramanujan-j5ojyq` once this is on `unstable` and `testing`.
- Networking: the welcome TCP connection, the login flow on Security's line (with the queue place told to
  the client), PROTOCOL.md filled in.  The first piece of the server proper, started and stopped from the
  Control Panel with the rest.  Its name is Jacob's to give.
- Accounts: making, checking and logging in.  Security and Fingerprinter are ready for it; the login itself
  waits on networking.
- Archivist retrying on its own every 5 seconds while disconnected.  Asked, not answered.  A STOP SERVER
  and a START SERVER is the way round it today.
- DiskMan: seeing hand edits to a file it already holds.  In TODO.
- The Windows build, whenever getting to that machine is less of a hassle.
- The Debug switch in `conductor_globals.cfg`, and moving the routine log lines to Debug.  Every server
  start and stop adds a few Info lines now.
- Catching Ctrl-C.
- `\dt` in psql to confirm `archivist_migrations` exists, and that `0001_uuid_on_every_table.sql` ran.
- Picking Ensemble's engine.
