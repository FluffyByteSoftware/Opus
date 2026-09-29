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
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the monitor, and
whatever comes later) only starts when START SERVER is pressed on the web admin's Control Panel, and STOP
SERVER takes it back down with Conductor still running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests (the session
pushes `unstable` onto it when a round is ready), `main` is the stable release, moved only when Jacob
says.  `main` and `testing` sit on the Control Panel commit; `unstable` is ahead of them by this session.

**Built on Linux (Nobara 44), 2026-09-29**, from `testing`: `cargo build` was clean at `e14bcc7`; the
test build failed on four lines in DiskMan's new tests (`status()` called on the test's own DiskMan
instead of through its lock), fixed at `4a34aa3`.  **`cargo test` and a run haven't been reported back
yet**, so the first thing next time is to see how they went.  The last full test and run was the Control
Panel session (108 tests, START / RESTART / STOP / SHUT DOWN from the page).  The Windows code has never
been built.

## Last session -- 2026-09-29 (the third that day)

The config overhaul, tools side: Constellations rebuilt as the one owner of every config file, and DiskMan
taught to swap a file in at the right moment.  This is the ground the web admin's config editor stands on;
the routes and the Settings tab are the next piece and are laid out in TODO.md.

What we did:

- **`constellations/files.rs`**: the table.  Every config file (`GLOBALS` is `conductor_globals.cfg`,
  `POSTGRES` is `postgres.cfg`) with its reboot, its channel, its comment and every setting: key, kind
  (text, secret, folder, port, number with a range), default, comment.  `FILES` lists them.  Adding a
  setting is one entry here.
- **`constellations/text.rs`**: the one reader and writer, driven by the table.  `parse()` hands back the
  good values, the complaints (never echoing a secret, never echoing a line that isn't `key = value`) and
  the keys seen; `file_text()` writes a whole file with the reboot rule in its header; `missing_text()`
  is what gets appended to a file that lacks a setting.  The tests from both old readers moved here and
  run over every file in the table.
- **`constellations.rs`**: the store.  A lock, not a `OnceLock`.  `load(file)` reads a file (writes it
  with the defaults if it's missing, appends missing settings, Warns for bad lines), and first swaps in a
  leftover `.wait4server` if one is there.  `value()`, `number()`, `port()`, `folder()`, `values()`; an
  unloaded file reads as its defaults.  `save_waiting(file, text)` checks every line and writes
  `name.cfg.wait4server` beside the live file, then asks DiskMan to swap it in at the file's reboot;
  `waiting()` reads it back, `discard_waiting()` removes it.  `server_stopped()` is the launcher's call
  once the server pieces are down.
- **DiskMan** holds the swap list (Jacob's design): `swap(original, replacement, when)` with `when` now,
  at the server's stop, or at shutdown; `run_swaps(ServerStop)` for the launcher, with a `Pending` that
  answers once every due swap is done; the shutdown ones run as DiskMan's last act, and anything still in
  the list runs then too.  A swap only runs when both files are quiet, and drops what was held for them.
  `forget_swap()` and `remove()` for a discard.  Three tests for the worker, two for the cache.
- **Archivist's `settings.rs`** is now only the typed view: `DbSettings::from_constellations()` after
  `constellations::load(&POSTGRES)`.  Its reader, writer and tests went to Constellations.
- **The launcher** loads `GLOBALS` at boot and calls `constellations::server_stopped()` at the end of
  `stop_server()`.  A boot line says which file needs which reboot.
- **The committed `Content/cfg/` files** were rewritten in the new header layout (the reboot rule is in
  each one's comment).  `password = newpass` kept.  `Content/cfg/*.wait4server` is ignored by git.
- CLAUDE.md: the `unstable` rule at line 1, questions at the bottom of the reply under a loud header, and
  the config file rules.

What Jacob decided:

- Every session pushes to `unstable`, never a session branch.  Said again, at line 1 of CLAUDE.md now.
- A change from the page goes to `name.cfg.wait4server`, and DiskMan holds the list and swaps the file in
  when the thing that reads it goes down.  The live file always says what's running.
- Anything about Constellations or Scribe in `conductor_globals.cfg` is a hard reboot.  `scribe_log_dir`
  was going to hot swap; it's hard now.  Files stay separate, as many as it takes.
- The password can show on the page: Postgres is local and not reachable outside the machine.
- A Settings tab, and save / discard routes under `/Opus/wwwhook/settings/`, are agreed to.

- No `cfg_dir`, ever: the config folder is `Content/cfg/`, fixed relative to Opus.

## The session before -- 2026-09-29, the Control Panel

The server's switch (`server.rs`), the launcher's `start_server()` / `stop_server()`, the three
`/Opus/wwwhook/` routes and the Control Panel tab.  Built, tested and run from `testing`.  The details
are in the web admin and launcher design docs.

## What's waiting

- **The test result and a run of this session's code**: `cargo test` after the fix at `4a34aa3`, then a
  run that starts and stops the server with a hand-written `Content/cfg/postgres.cfg.wait4server` beside
  the live file, to see it swapped in at the next START SERVER (the log says "found ... and swapped it
  in"), and a `conductor_globals.cfg.wait4server` picked up at the next boot.  If anything fails, that's
  the next conversation's first job.
- **The config editor's web admin half**: the settings route, the save and discard routes, the Settings
  tab.  Laid out in TODO.md.
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
