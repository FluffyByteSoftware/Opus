<!--
File:       Opus/Documentation/LLM/TODO.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- TODO

The big features, the ones that are a run of sessions each, are in LONGTERM_TODO.md instead.

## Deferred

Things that wait on a piece that doesn't exist yet.

- Archivist: try to reconnect on its own every few seconds while it's disconnected.  Today it only tries
  when a job comes in, and nothing sends jobs yet, so the web admin's database lock never lifts by itself.
  Asked on 2026-09-28, not answered yet.
- Monitor on macOS: `proc_pidinfo` / `proc_pid_rusage` from libproc for memory, CPU, disk and per-thread
  times.  Waits on a Mac to test it on.  Today macOS builds and runs and the page says "not measured".
- Web admin: a login.  Anything on this machine can reach it today.  Security can hash the password now;
  it matters most once the page has buttons that change things.
- Web admin: HTTPS, once Security brings in TLS for the game.
- Scribe: a debug switch in `conductor_globals.cfg` (on or off) that drops Debug lines when off.  Then go
  through every existing log line and move the routine ones to Debug, per the rule in CLAUDE.md.  Archivist's
  connect, schema and settings lines are the obvious first ones.
- Accounts: making, checking and logging in.  Security's `hash_password()` and `verify_password()` are
  ready for it; Fingerprinter's `new_uuid()` names the row.  Waits on networking for the login itself.
- Accounts and networking: **one login at a time, hard limit.**  Jacob's rule from 2026-09-29.  Security's
  worker already hashes one at a time with the rest in line; the login flow has to lean on that line, never
  work around it (no hashing anywhere else, no second worker).  The client is told its place and about how
  long: `Ticket::place()` has the numbers; the message to the client is networking's.
- Accounts: a login token on reconnect (`fingerprinter::new_token()`), so a player who drops and comes back
  doesn't pay for a hash.  The biggest CPU saving Security can't make on its own.
- Security: TLS for the welcome TCP connection.  Waits on networking, and on a crate we'd have to pick.
- Web admin: the Control Panel (built 2026-09-29; the "Manage System" screen) starts and stops the server,
  which today is Fingerprinter, Security, Archivist and the monitor.  Networking and the game loop go in
  `start_server()` and `stop_server()` in the launcher when they exist, and come up and down with the
  rest.  (Was the launcher's S.)
- Web admin: account management (make, delete, list, finger, change password).  Waits on accounts.
- **Web admin: the config editor**, the rest of it.  The tools side was built on 2026-09-29 (Constellations
  rebuilt: one table of every file and setting, one reader, `save_waiting()` / `waiting()` /
  `discard_waiting()`, and DiskMan's swap list that puts a `.wait4server` file in place when its reboot
  comes).  What's left is the web admin's half, agreed with Jacob the same day:
  - `GET /Opus/settings`: every file, every setting (key, kind, comment, default, running value, the
    waiting value if there is one, and the file's reboot).  The password goes out as it is: Postgres only
    listens on this machine (Jacob's call).
  - `POST /Opus/wwwhook/settings/save?file=<name>` with the `X-Opus` header, the body being `key = value`
    lines in the file's own format, so the same reader checks it and no JSON reader is needed.  A bad line
    means nothing is written and the complaints come back for the page to show beside the fields.
    `http.rs` has to learn to read a body (`Content-Length`).
  - `POST /Opus/wwwhook/settings/discard?file=<name>`, the same way.
  - A **Settings** tab, reachable while the server is stopped and under the database lock like the Control
    Panel and the Log.  One card per file: the reboot it needs in plain words, a field per setting with its
    comment, SAVE and DISCARD, and "waiting on a soft / hard reboot" beside anything saved and not yet
    applied.  No hot swapping: nothing changes until the reboot.
  - The JSON shape at the top of `json.rs`, and the design docs.
  - Once there is a login, only an admin can change them.
  - Maybe: a `cfg_dir` setting in `conductor_globals.cfg` saying where the *other* config files live, if
    Jacob wants the config folder movable (it can't point at its own folder).  Asked on 2026-09-29, not
    settled.
- Launcher: catch Ctrl-C and shut down cleanly (or ignore it).  Since DiskMan, there is something to save
  on shutdown: Ctrl-C loses whatever it hasn't written yet.  Catching it on both Linux and Windows without a crate means a
  signal handler on one and a console handler on the other.
- Ensemble has no way to find `Content/` yet.  Decide how once the engine is picked.
- Where the purchased art lives, and whether it goes in the repo through LFS.  `Content/Assets/` is ignored
  for now, so it stays out of git.  Jacob's call when the client needs it.

## Ideas

Things we thought of along the way.  None of them are promised.

- Scribe: a size limit that starts a second file for the day (`2026_09_28.1.scribe.log`) on top of the
  midnight rollover.  Stratum had one.  Not picked for now.
- Scribe: the handful of lines logged before `move_to()` stay in the default folder if the config points
  somewhere else.  Holding them in memory until the folder is known would fix it.  Not worth it while both
  folders are the same one.
- Scribe: a lost log file only gets retried at midnight UTC or on `move_to()`.  A retry every few minutes
  would get it back sooner after, say, a full disk is cleaned up.
- Scribe: the caller shows the path Rust compiled with (`conductor-launcher/src/main.rs`).  Trim to the file
  name if that gets noisy.
- Constellations: the Storage tab could show `swaps_waiting` from DiskMan's status (it's in the struct,
  not in the JSON yet), once the Settings tab exists to explain it.
- Archivist: a password that starts or ends with a space loses the space, because every value is trimmed.
  Quotes around the value would fix it, if it ever matters.
- Archivist: more than one worker, if one ever can't keep up.  Tried and taken out on 2026-09-28: with two,
  jobs can finish out of order, so a SELECT could miss the UPDATE sent just before it.  If it comes back, it
  needs a way to keep one player's jobs in order (all of a player's jobs to the same worker, say).
- Monitor: a SQL read / write split by rows (rows returned, rows changed), not just by jobs.
- Monitor: Postgres's own view of things (`pg_stat_database`: cache hits, rows read), as an Archivist job
  once a few seconds, so it never waits on the database.
- Monitor or the Storage tab: "last read / last write" by file.  DiskMan sees every file now, so this is
  ready whenever it's wanted.
- DiskMan: notice hand edits.  A file it already holds is served from memory even if somebody edited it on
  disk since.  Checking the modified time before trusting the copy would fix it.  Only configs today:
  `postgres.cfg` is read on every START SERVER, so a hand edit between two starts while Conductor runs
  isn't seen.  A `.wait4server` file written by hand is, since Constellations reads it fresh.
- DiskMan: the `.wait4server` swap leans on `fs::rename` replacing a file, the same as its writes; on
  Windows that's the same untested spot.
- DiskMan: the Windows build.  It leans on `fs::rename` replacing a file there, and skips flushing the folder.
- Notices: no cap.  They're kept since boot until ACKed, so a flood of Warns left alone for days keeps
  growing in memory.  A cap (the oldest dropped) if it ever matters.
- Web admin: keep the CPU and memory history on the server, so a page opened late still sees the last few
  minutes.
- Web admin: a Debug on / off switch for the Log tab, once Scribe has its Debug switch.
- Web admin: saved page layouts, per user, once the web admin has users.  Jacob's long-term idea from the
  docking talk on 2026-09-28.  Today the only thing remembered is the last tab, in the browser.
- Web admin: pin Conductor to the top of the System tab's process list, if busiest-first buries it.
- Web admin: a setting in `conductor_globals.cfg` that starts the server on its own when Conductor boots,
  for a machine nobody sits at.  Today it always waits on START SERVER.
- Web admin: a Control Panel line saying what a RESTART is for (a changed `postgres.cfg` is read again).
- Security: raise the memory (128 MiB, say) once the benchmark shows what a one-pass 64 MiB hash costs on
  Jacob's machine.  The arena grows with it, and stays allotted.
- Security: the `parallel` (rayon) feature would split one hash's lanes across cores, cutting its wall time
  at the same CPU cost.  A crate, and its threads bypass `threads::spawn()`, so it's not taken.
- Security: say on the Services tab whether the huge pages actually landed, not just that they were asked
  for.  Linux says in `/proc/self/smaps` (`AnonHugePages`), which would have to be read past DiskMan the
  way the monitor reads `/proc`.
- Security: wipe passwords from memory once they're hashed (the crate's `zeroize` feature and a wipe of the
  job's `String`).  Off for now; a hobby server, and the theory matters more than the polish.
- Running Conductor with no console window.  The page's Log tab shows everything the console does, but
  closing the console kills Conductor today (Linux sends the terminal's hang-up signal, Windows ends the
  process), so it goes down without a clean shutdown.  Ways to fix it: start it detached (`setsid` or
  `nohup` on Linux), run it as a systemd service / Windows service, or build a Windows version with no
  console at all.  Jacob's pick when it matters.
