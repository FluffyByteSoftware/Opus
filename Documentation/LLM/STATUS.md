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
says.  He said so at the end of this session, so `main`, `testing` and `unstable` all sit on the same
commit: the login, the Settings tab and everything before them.

**Built and tested on Linux (Nobara 44), 2026-09-29**, from `testing` at `c4cfd59`: the login and the
Settings tab.  `cargo build` clean bar one warning (an unread error field, fixed after), `cargo test`
passed 132 (15 monitor, 89 tools with the benchmark ignored, 28 web admin), and a run did the whole loop:
admin logged in, START SERVER, a save of `wgui_port` on the Settings tab, STOP SERVER, SHUT DOWN, and the
next run came up on the new port.  The first real `.wait4server` swap.  The Windows code has never been
built.  `cargo build` clean
(the first test build failed on four lines in DiskMan's new tests, `status()` called on the test's own
DiskMan instead of through its lock; fixed).  `cargo test` passed 119 tests (15 monitor, 88 tools with the
benchmark ignored, 16 web admin), and a run did START SERVER, STOP SERVER and SHUT DOWN from the Control
Panel: `conductor_globals.cfg` loaded at boot and `postgres.cfg` at START SERVER, both through
Constellations, Archivist connected, shutdown clean.  The `.wait4server` swap ran only in the tests so
far; nothing on the page writes one yet.  The Windows code has never been built.

## Last session -- 2026-09-29 (the fourth that day)

Two pieces of the web admin, in this order at Jacob's say: **a login**, then **the config editor's web
admin half**.  Checked in a headless browser against a stand-in for the routes with made-up numbers,
then built, tested and run by Jacob (see above).

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
  save, a complaint under the field it names after a failed one.  Past four files a picker at the top
  shows one card at a time (Jacob's ask after seeing the cards; the cards were more than he'd pictured).
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
- **No idle timeout, ever.**  The login is about roles (who may change the server), not security; the
  page only listens on this machine.  Said at the wrap-up.
- **`wgui_port` moves into `wgui.cfg`.**  Said at the wrap-up, after the port had gone in with the
  passwords staying separate.  Not done this session (no code at hand-off); it's first in "what's
  waiting".

## What's waiting

- **Networking, Jacob's pick for the next conversation**: the welcome TCP connection, the login flow on
  Security's line, PROTOCOL.md.  Its name is his to give, and its settings would be a soft file of its
  own, which the Settings tab will show with no page work.  The items below are unordered.
- **Move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`** (Jacob, 2026-09-29).  One entry
  moves in `files.rs`; `Settings` / `settings()` in `constellations.rs` and the launcher's
  `conductor_wgui::start(...)` call read it from `WGUI` instead; both committed `Content/cfg/` files
  change; the boot line "Settings from ..." and the docs (CLAUDE.md's file rule, the tools design
  doc's table, README, PROJECT_OPUS) follow.  Both files are hard, so nothing about reboots changes.  A
  file that lacks the setting gets it appended with the default on the next load, so an old
  `wgui.cfg` on Jacob's machine is fine; the stale line in `conductor_globals.cfg` would read as "no
  setting called wgui_port" and be Warned about once, so the committed file drops it.
- The `user` account hasn't been tried in a real run yet, only in the tests and the headless check.
- On GitHub, by hand: delete `claude/gracious-ramanujan-j5ojyq` and `testing_/charming-euler-jlyos7`.
  Jacob said he'd do it.
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
