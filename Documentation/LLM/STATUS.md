<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is four crates.  `conductor-tools` (lib) holds DiskMan, Scribe, Constellations, Fingerprinter,
Security, Archivist, the notices, the clock, the thread list, the services list and the server's switch
(`server.rs`).  `conductor-monitor` (lib) looks at the process and every process on the machine once a
second.  `conductor-wgui` (lib) is the web admin at `http://127.0.0.1:9996/Opus`, and the only way to start
and stop the server and to shut Conductor down.  `conductor-launcher` (bin) boots the program and waits on
the Control Panel.  Ensemble hasn't been started.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The web admin has a login: `admin` / `admin` does everything,
`user` / `user` looks, both passwords in `wgui.cfg`.  The server (Fingerprinter, Security, Archivist, the monitor, and
whatever comes later) only starts when START SERVER is pressed on the web admin's Control Panel, and STOP
SERVER takes it back down with Conductor still running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests (the session
pushes `unstable` onto it when a round is ready), `main` is the stable release, moved only when Jacob
says.  `main` sits on the Constellations rebuild; `unstable` and `testing` carry this session on top.

**Last built and tested on Linux (Nobara 44), 2026-09-29**, from `testing` at `4a34aa3`, before the
login and the Settings tab; those are written and not yet built.  `cargo build` clean
(the first test build failed on four lines in DiskMan's new tests, `status()` called on the test's own
DiskMan instead of through its lock; fixed).  `cargo test` passed 119 tests (15 monitor, 88 tools with the
benchmark ignored, 16 web admin), and a run did START SERVER, STOP SERVER and SHUT DOWN from the Control
Panel: `conductor_globals.cfg` loaded at boot and `postgres.cfg` at START SERVER, both through
Constellations, Archivist connected, shutdown clean.  The `.wait4server` swap ran only in the tests so
far; nothing on the page writes one yet.  The Windows code has never been built.

## Last session -- 2026-09-29 (the fourth that day)

Two pieces of the web admin, in this order at Jacob's say: **a login**, then **the config editor's web
admin half**.  Written and checked in a headless browser against a stand-in for the routes with made-up
numbers; **not built or run**, since that's Jacob's.

What we did:

- **`wgui.cfg`**, a new hard config file in Constellations' table (`WGUI` in `files.rs`), the web admin's
  own: `user_password` (`user`) and `admin_password` (`admin`), both `Text` so neither can be empty.  The
  launcher loads it at boot right after globals.  The committed `Content/cfg/wgui.cfg` is in the same
  layout as the other two.  `constellations::file_named(name)` finds a file by name for the routes, and
  `file_values(file)` reads one as it sits on disk, for a file not loaded yet this run.
- **`conductor-wgui/src/login.rs`**, new: the two accounts, `Role` (`User` looks, `Admin` acts), `log_in()`
  checking the name and password against `wgui.cfg` and handing back a random token from Fingerprinter's
  `new_token()` (works with the server stopped), the live tokens in a list in memory, `role_of(request)`
  from the cookie, `log_out()`, the `Set-Cookie` lines (`HttpOnly; SameSite=Strict; Path=/Opus`, 30 days
  on the browser's side).  The passwords are plain, not hashed: Security only runs with the server.
- **`http.rs`** reads a body now, exactly `Content-Length` bytes, capped at 16 KB, with a loopback test.
- **`lib.rs`**: `POST /Opus/login` and `/Opus/logout` (`X-Opus: login`); everything but `/`, the page and
  the login needs the cookie and answers 401 without it; every route that changes something goes through
  `only_admin()` and answers 403 to `user`.  `GET /Opus/settings`, `POST /Opus/wwwhook/settings/save?file=`
  and `/discard?file=` (`X-Opus: settings`, admin only).  A save hands the body straight to
  `constellations::save_waiting()`; a bad line is a 400 with the complaints as JSON, a disk failure a 500
  the same way, and nothing is written either time.
- **`json.rs`**: `login` in the status (`name`, `can_change`), `settings()` (every file: reboot, comment,
  loaded, waiting; every setting: key, kind with its range, comment, default, running, waiting), and
  `problems()`.  The shapes are at the top of the file.
- **`page.html`**: a login card over the whole page on every 401 (the status loop stops until a login
  starts it again); who's logged in and LOG OUT at the bottom of the sidebar; every changing button greyed
  for `user`.  The **Settings** tab, eighth after Log, always clickable and outside the database lock: one
  card per file, a field per setting, SAVE and DISCARD, a WAITING tag and "running on" lines after a
  save, a complaint under the field it names after a failed one.
- Tests: 4 in `login.rs`, 2 loopback ones in `http.rs`, the route tests reworked to log in first plus
  new ones for the login, `user` being turned away, and the settings routes; 1 for the settings JSON; 1
  for `file_named()`.  Not run yet.
- Docs: the web admin and tools design docs, TODO, PROJECT_OPUS, README, and CLAUDE.md (the login rule,
  eight tabs, `wgui.cfg`).

What Jacob decided:

- Login first, then the editor.  The Settings tab goes after Log.
- The accounts live in `wgui.cfg`, the web admin's own file, not split into files of their own.
- `user` is read only; `admin` can edit config files (and everything else).
- A card over the full page until you log in; a login every time Conductor is started; one that survives
  reloading the page.

`wgui_port` stayed in `conductor_globals.cfg`; only the passwords went in `wgui.cfg`.  Say if it should
move.

## What's waiting

- **Building and running this session's work**: `cargo build`, `cargo test`, then a run: log in as
  `user` and see the buttons greyed, as `admin` and start the server, then the Settings tab: save a bad
  port and see the complaint under the field, save a good one and see the WAITING tag, SHUT DOWN and look
  at `Content/cfg/conductor_globals.cfg` for the new value (the first real `.wait4server` swap).  For
  `postgres.cfg`, STOP SERVER is the swap.  The log line for a leftover at load is "found ... and swapped
  it in"; the ordinary swap is quiet.
- On GitHub, by hand: delete `claude/gracious-ramanujan-j5ojyq` and `testing_/charming-euler-jlyos7`.
  Jacob said he'd do it.
- Networking: the welcome TCP connection, the login flow on Security's line (with the queue place told to
  the client), PROTOCOL.md filled in.  The first piece of the server proper, started and stopped from the
  Control Panel with the rest.  Its name is Jacob's to give.  Its settings would be a soft file of its own.
- Accounts: making, checking and logging in.  Security and Fingerprinter are ready for it; the login itself
  waits on networking.
- Archivist retrying on its own every 5 seconds while disconnected.  Asked, not answered.  A STOP SERVER
  and a START SERVER is the way round it today.
- DiskMan: seeing hand edits to a file it already holds.  In TODO.
- The Windows build, whenever getting to that machine is less of a hassle.
- The Debug switch in `conductor_globals.cfg`, and moving the routine log lines to Debug.  Constellations'
  own "loaded" and "added the missing settings" lines are Debug already.
- Catching Ctrl-C.
- `\dt` in psql to confirm `archivist_migrations` exists, and that `0001_uuid_on_every_table.sql` ran.
- Picking Ensemble's engine.
