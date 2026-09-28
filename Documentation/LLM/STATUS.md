<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is four crates.  `conductor-tools` (lib) holds DiskMan, Scribe, Constellations, Archivist, the
notices, the clock, the thread list and the services list.  `conductor-monitor` (lib) looks at the process
and every process on the machine once a second.  `conductor-wgui` (lib) is the web admin at
`http://127.0.0.1:9996/Opus`, and the only way to shut the server down.  `conductor-launcher` (bin) starts
all of it and waits on the web admin.  Ensemble hasn't been started.

**Built and tested on Linux (Nobara 44), 2026-09-28.**  `cargo clean && cargo build` was clean with no
warnings, and `cargo test` passed 77 tests (15 monitor, 47 tools, 15 web admin).  A run connected to
Postgres, read the config and the schema through DiskMan, wrote the log through it, and shut down in about a
second.  Jacob looked at the page, the bell and the test notification.  The Windows code has never been
built.

## Last session -- 2026-09-28 (the third that day)

DiskMan, the disk manager, and the web admin's notifications.

What we did:

- **DiskMan** (`diskman.rs`, `diskman/cache.rs`, `diskman/worker.rs`): every file Conductor reads or writes
  goes through one worker thread.  `write()`, `append()`, `read()` hand back a `Pending`; `stream()` reads
  big files back 1 MB at a time.  What comes in while the disk is busy is held in memory, dirty until it's
  written, and a second write replaces the first before it goes out.  Reads stay loaded.  Clean files unload
  past 256 MB; dirty ones never do.  Whole writes go through a temp file and a rename.  Writes over 8 MB go
  out 1 MB at a time with other files in between.  A failed write gets 3 tries, then a capitals Error.
- **Everything moved onto it**: Scribe hands it each line as an append; Constellations and Archivist
  (`postgres.cfg`, schemas, migrations) read and write through it.  DiskMan starts first and stops last.
- **Shutdown waits on DiskMan.**  Over a second, the console counts down from 60; at zero it says
  `SHOULD BE CLOSED, IF STILL RUNNING PLEASE FORCE QUIT` with the files that would be lost, and keeps
  waiting.
- **`Pending` moved to `pending.rs`**, shared by Archivist and DiskMan.
- **Notices** (`notices.rs`): every Warn and Error, and anything raised with `notices::publish()`, stays
  until it's ACKed.  Memory only, since boot.
- **The web admin**: a bell in the header's corner with a badge (up to `5+`) that pulls out a tray of the
  newest five, each fading after 30 seconds; a sixth tab, Notifications History, with ACK, ACK ALL and TEST
  NOTIFICATION; DiskMan's row on the Services tab and its panel on the Storage tab.  Four new routes:
  `GET /Opus/notices`, and `POST /Opus/notices/ack?id=N`, `/ack-all` and `/test`, all needing `X-Opus: ack`.

What fought back:

- Notifications came up in the middle of DiskMan and grew into their own feature.  "Viewed" clearing them was
  dropped for a manual ACK, and a journal file was dropped for memory only.  Worth it, but it was two
  features in one session.
- A log line is itself a write, so DiskMan can't log routine work, and Scribe's own file failing has to go to
  the console.  Otherwise it loops forever.

What Jacob decided:

- DiskMan does every file read and write, appends and whole writes both, with streaming reads and the
  chunker, all this session.
- Replace, don't queue, a second write to the same file.  Dirty data held until it's on disk, clean data
  unloaded.  "A crash is the worst possible scenario... That's why we do atomic writes and best measures."
- Shutdown waits a minute, then tells the admin to force quit.  Conductor never quits on its own.
- Notices: every Warn and Error, plus ones raised on purpose; cleared only by ACK, and gone after it; kept in
  memory since boot.
- **Next: the Fingerprinter.**  Jacob's pick for the next conversation.  Stratum had one (UUIDs); Opus doesn't
  take its code, and its shape is Jacob's call.

## What's waiting

- **The Fingerprinter.**  Jacob's pick for the next conversation.
- Archivist retrying on its own every 5 seconds while disconnected, so the page's lock lifts when Postgres
  comes back.  Asked, not answered.
- DiskMan: seeing hand edits to a file it already holds.  In TODO.
- The Windows build, whenever getting to that machine is less of a hassle.  The probe, the process list,
  the threads route and DiskMan's rename are all untried there.
- The Debug switch in `conductor_globals.cfg`, and moving the routine log lines to Debug.  It matters more
  now: every Warn is a notice, so a Warn that isn't really wrong is one more thing to ACK.
- Catching Ctrl-C, now that it can lose what DiskMan holds.
- `\dt` in psql to confirm `archivist_migrations` exists.
- Accounts, which wait on Security for Argon2.  Security itself.
- Picking Ensemble's engine.
