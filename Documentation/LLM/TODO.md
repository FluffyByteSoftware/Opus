<!--
File:       Opus/Documentation/LLM/TODO.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- TODO

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
  work around it (no hashing anywhere else, no second worker).  Whether a waiting client is told its place
  in the queue is open.
- Accounts: a login token on reconnect (`fingerprinter::new_token()`), so a player who drops and comes back
  doesn't pay for a hash.  The biggest CPU saving Security can't make on its own.
- Security: TLS for the welcome TCP connection.  Waits on networking, and on a crate we'd have to pick.
- Web admin: start / stop the game.  Waits on networking and a game loop.  (Was the launcher's S.)
- Web admin: account management (make, delete, list, finger, change password).  Waits on accounts.
- Web admin: the settings, shown and changed live (Jacob asked on 2026-09-28).  Showing them is small: a
  read-only route and a Settings tab.  Changing them live needs:
  - Constellations to stop being load-once.  Today the settings sit in a `OnceLock`, which can't change
    after it's set; it would become a lock around settings that can be swapped.
  - A route that changes things (`POST`, with the `X-Opus` header like Shut Down), which is Jacob's call
    per CLAUDE.md.  Every value is checked before anything is written, and a bad one is turned away.
  - Each setting saying what happens when it changes.  `scribe_log_dir` can switch on the spot
    (`scribe::move_to()`); `wgui_port` means restarting the web admin on the new port, so the page has to
    follow it there, or it waits for the next start.  Every setting added later says which kind it is.
  - The file written back safely, through `diskman::write()`, which is ready for it.  It also needs
    DiskMan to see hand edits (see Ideas), or a changed file on disk won't match what's loaded.
  - Maybe `postgres.cfg` too, but it holds the password, and showing that on a page is Jacob's call.
  - Once there is a login, only an admin can change them.
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
- Constellations: log a warning at startup for a key that is missing from the file and fell back to its
  default, the way unknown keys are warned about today.
- Archivist and Constellations each have their own `key = value` reader.  They're nearly the same code, and
  could share one if a third config file shows up.
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
  disk since.  Checking the modified time before trusting the copy would fix it.  Only configs today, and
  they're read once at startup.
- DiskMan: the Windows build.  It leans on `fs::rename` replacing a file there, and skips flushing the folder.
- Notices: no cap.  They're kept since boot until ACKed, so a flood of Warns left alone for days keeps
  growing in memory.  A cap (the oldest dropped) if it ever matters.
- Web admin: keep the CPU and memory history on the server, so a page opened late still sees the last few
  minutes.
- Web admin: a Debug on / off switch for the Log tab, once Scribe has its Debug switch.
- Web admin: saved page layouts, per user, once the web admin has users.  Jacob's long-term idea from the
  docking talk on 2026-09-28.  Today the only thing remembered is the last tab, in the browser.
- Web admin: pin Conductor to the top of the System tab's process list, if busiest-first buries it.
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
