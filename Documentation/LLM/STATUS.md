<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is four crates.  `conductor-tools` (lib) holds Scribe, Constellations, Archivist, the clock, the
thread list and now the services list.  `conductor-monitor` (lib) looks at the process, and now every process
on the machine, once a second.  `conductor-wgui` (lib) is the web admin at `http://127.0.0.1:9996/Opus`, and
the only way to shut the server down.  `conductor-launcher` (bin) starts all of it and waits on the web
admin.  Ensemble hasn't been started.

**Nothing from this session has been built or run.**  Jacob builds and tests; the page was only checked by
rendering it in a browser with made-up numbers.  The previous session's CPU chart and memory bar were never
run either, and no `cargo build` / `cargo test` output has been pasted back for either session.  The
Windows code has never been built, and this session added more of it.

## Last session -- 2026-09-28 (the second that day)

The web admin's layout: tabs, a database lock, services, and every process on the machine.

What we did:

- **Tabs down the left sidebar**, under the OP logo: System, Conductor, Services, Storage, Log.  Conductor
  opens first; the browser remembers the last one picked (`localStorage`, nothing on the server).
  - **System**: the whole machine's CPU and memory, and every process Conductor can see, busiest first, with
    Conductor's row in green with a CONDUCTOR tag and a filter box.  Click a process to see its threads with
    their core % (Conductor's own threads carry our names).
  - **Conductor**: CPU chart, memory bar, disk, machine, and Conductor's threads (in use / asked for).
  - **Services**: Scribe, Constellations, Archivist, the monitor and the web admin, each with its state, what
    it last said, since when, and "last seen" for the monitor.  A flashing red dot for any that isn't healthy,
    on the row and on the sidebar tab.  The disk manager is a grey "not built yet" row.
  - **Storage**: Archivist's numbers, and a "not built yet" panel for the disk manager.
  - **Log**: Scribe's terminal, the full height of the window.
- **The database lock.**  A DB pill in the header on every tab (ONLINE / CONNECTING / OFFLINE).  Until
  Archivist is connected, everything under the header is blurred and locked, the tabs too, with a flashing
  red "DATABASE OFFLINE -- the game can't run right now" card.  SHUT DOWN is the only thing that works.  The
  page keeps asking underneath and unlocks itself when Archivist connects.  The first 10 seconds after
  Conductor starts count as "connecting", not offline.
- **The status line** reads NOMINAL, or everything that's wrong: `DATABASE NOT CONNECTED, 1 SERVICE DOWN`.
- **`services.rs`** in conductor-tools: every expected service is listed from the start as "expected", and
  each reports starting / running / trouble (with why) / stopped.  One whose thread has ended shows as
  stopped whatever it last said, and the monitor checks in every second so a stuck one goes red after 5.
  The status JSON carries `services` straight from the list, not through the monitor, so it's still right
  if the monitor dies.
- **Every process on the machine**: `probe/linux.rs` reads `/proc/<pid>/stat` (kernel threads left out);
  `probe/windows.rs` uses the toolhelp process list plus `OpenProcess`, and processes Windows won't open
  come back with no numbers.  One new read-only route, `GET /Opus/threads?pid=N`, gives one process's
  threads; the page asks once a second, only while that process is picked on the System tab.
- The main thread is on the thread list as "main" (`threads::name_this_thread`).
- TODO: running with no console window, and showing and changing the settings live from the page.

What fought back:

- The layout took four passes of talking before any code.  Jacob first asked for IDE-style docking, then
  looked at the page again and dropped it for tabs: "docking won't fix this, I was over engineering".  A
  saved-settings file (`Content/web/wgui_settings.json.cfg`) was planned and dropped with it.
- "Scribe lives in tools" -- asking where "Scribe" should go meant the log panel on the page, and read as
  moving the crate.  Say "the log panel" for the page's piece.
- "Tabs" meant the left sidebar where OP is, not a row across the top.  Fixed after the first build.
- Archivist only reconnects when a job comes in, and nothing sends jobs yet.  So once the database drops,
  the page's lock stays up until Conductor restarts, even after Postgres is back.  Found, not fixed: it
  changes how Archivist behaves, and Jacob hasn't said yes to it.

What Jacob decided:

- Tabs, not docking.  Five of them, down the left side, Conductor first.
- While the database is offline, the admin sees that and nothing else: blurred, locked, SHUT DOWN only.
  "The whole point is to draw attention to the user that the DB is offline and the game can't run."
- The log gets a tab of its own.
- No settings file for the page.
- **Next: the disk manager.**  Jacob's pick for the next conversation.

## What's waiting

- **The disk manager.**  Jacob's pick for the next conversation.  What's known so far is in TODO.md: the
  one place whole files get written, through a temp file and a rename, so a crash can't leave half a file.
  Constellations' config writes and the settings-from-the-page idea both wait on it.  Once it exists it
  reports to the services list (the name is already on the page as "not built yet") and fills the Storage
  tab's empty panel.  Stratum had a DiskMan; Opus doesn't take its code, and its shape is Jacob's call.
- **Archivist retrying on its own**, every 5 seconds while it's disconnected, so the page's lock lifts when
  Postgres comes back.  Asked, not answered.
- Build and test everything from this session and the last, and paste back `cargo build` and `cargo test`.
  This session added tests in `services.rs`, `probe/linux.rs`, `snapshot.rs`, `json.rs` and the wgui's
  `lib.rs`.
- Run the page: each tab, the DB lock (stop Postgres or break the password in `postgres.cfg`), the Services
  tab going red, and the System tab with Conductor in green.
- The Windows build, whenever getting to that machine is less of a hassle.  The process list and the
  threads route are new Windows code on top of the untried probe.
- The Debug switch in `conductor_globals.cfg`, and moving the routine log lines to Debug.
- `\dt` in psql to confirm `archivist_migrations` exists.
- Accounts, which wait on Security for Argon2.  Security itself.
- Picking Ensemble's engine.
