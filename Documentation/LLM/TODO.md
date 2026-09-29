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
- Web admin: hash the two passwords in `wgui.cfg` through Security instead of keeping them as they are.
  Security is a server piece and only runs between START SERVER and STOP SERVER, and a login has to work
  before START SERVER, so this waits on Security being up from boot, or a hash on the caller's thread.
  Jacob's call when it matters; plain text is fine while the page only listens on this machine.
- Web admin: HTTPS.  rustls is in the build now (networking, 2026-09-29), so this waits only on wanting
  it, and on the browser warning a self-signed certificate gets.
- Scribe: a debug switch in `conductor_globals.cfg` (on or off) that drops Debug lines when off.  Then go
  through every existing log line and move the routine ones to Debug, per the rule in CLAUDE.md.  Archivist's
  connect, schema and settings lines are the obvious first ones.
- Accounts: making one.  Logging in is built (networking, 2026-09-29) and reads the `accounts` table;
  nothing writes a row yet.  Security's `hash_password()` and Fingerprinter's `new_uuid()` are ready.  The
  first throwaway account was inserted by hand (STATUS.md has the how).
- **Client management**, Jacob's words for the lot of it, 2026-09-29: not this iteration.  The point of
  this one was handing a client from TCP to UDP and logging them off.  Waiting in here:
  - A player limit: "The server is full." (Stratum had 50, with a few more TCP connections so a full
    server could still say so).  Today `max_waiting_logins` caps the door and nothing caps the world.
  - A login token on reconnect (`fingerprinter::new_token()`), so a player who drops and comes back doesn't
    pay for a hash.  The biggest CPU saving Security can't make on its own.  Against today's rule on
    purpose (a dropped UDP session is gone, start over), so it's a design change when it comes, not a fix.
  - A session id in every UDP packet, so a home router changing the port mid-session doesn't end it.
  - Anything an admin does to a player from the web admin: see who's on, kick, message.
- **Web admin: the character on the Connections tab's UDP list**, beside the account, once there are
  characters.  The list itself (address, account, connected when, playing for, quiet for) was built with
  the access lists, 2026-09-29.  Kicking a player from it is client management, above.
- **Networking: a Kicked reason for a ban.**  A player dropped by a blacklisting is told nothing today,
  like one who went quiet, because a new `KickReason` is a protocol change (`PROTOCOL_VERSION` bumps).
  When Ensemble can show "you were banned", add the reason and bump the version.
- **Networking: the whitelist-to-blacklist edge.**  Taking an address off the whitelist while the whitelist
  is on kicks nobody: they're turned away on their next login.  A ban is the blacklist's job.  If that
  ever bites, `close_matching()` and `drop_where()` are already there to call.
- **Networking: `access_list` switchable from the page at once.**  Today the switch is in `networking.cfg`
  and takes on the next START SERVER, while the lists themselves take at once.  Jacob's call if the
  reboot is a bother.
- Networking: reverse DNS on macOS.  `dns/other.rs` hands back no name; macOS has `getnameinfo` with its
  own `sockaddr` layout (a length byte first).  Waits on a Mac, like the monitor.
- Networking: the protocol version in the Hello is `1` and the client versions are a list in
  `networking.cfg`.  Whether Ensemble reports a version string or a number is Ensemble's call.
- Web admin: the Control Panel (built 2026-09-29; the "Manage System" screen) starts and stops the server,
  which today is Fingerprinter, Security, Archivist and the monitor.  Networking and the game loop go in
  `start_server()` and `stop_server()` in the launcher when they exist, and come up and down with the
  rest.  (Was the launcher's S.)
- Web admin: game account management (make, delete, list, finger, change password).  Waits on accounts.
  Not to be confused with the web admin's own two accounts, which are in `wgui.cfg` and built.
- **Web admin: move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`.**  Jacob's call at the
  2026-09-29 wrap-up, once the web admin had a file of its own.  The steps are in STATUS.md.
- Launcher: catch Ctrl-C and shut down cleanly (or ignore it).  Since DiskMan, there is something to save
  on shutdown: Ctrl-C loses whatever it hasn't written yet.  Catching it on both Linux and Windows without a crate means a
  signal handler on one and a console handler on the other.
- Ensemble has no way to find `Content/` yet.  Decide how once the engine is picked.
- Where the purchased art lives, and whether it goes in the repo through LFS.  `Content/Assets/` is ignored
  for now, so it stays out of git.  Jacob's call when the client needs it.

- **Documentation: clean-up and management.**  Jacob's pick for the session after the access lists, said
  mid-session on 2026-09-29.  What that covers is his to say when it opens.

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
- Constellations: a stray `Content/` inside `Conductor/dev` (made by hand by mistake, 2026-09-29) shadows
  the real one, since the walk up takes the nearest.  Skipping a `Content` whose parent holds a
  `Cargo.toml` would rule that one out.  Not done; the folder was removed instead.
- Constellations: the Storage tab could show `swaps_waiting` from DiskMan's status (it's in the struct,
  not in the JSON yet).  The Settings tab is there now to explain it.
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
  disk since.  Checking the modified time before trusting the copy would fix it.  The configs, and since
  2026-09-29 the two access lists: `postgres.cfg` and `whitelist.cfg` are read on every START SERVER, so
  a hand edit between two starts while Conductor runs isn't seen (Jacob's "you could edit them on disk"
  holds with Conductor down, not between a STOP and a START).  A `.wait4server` file written by hand is,
  since Constellations reads it fresh.
- DiskMan: the `.wait4server` swap leans on `fs::rename` replacing a file, the same as its writes; on
  Windows that's the same untested spot.
- DiskMan: the Windows build.  It leans on `fs::rename` replacing a file there, and skips flushing the folder.
- Notices: no cap.  They're kept since boot until ACKed, so a flood of Warns left alone for days keeps
  growing in memory.  A cap (the oldest dropped) if it ever matters.
- Web admin: keep the CPU and memory history on the server, so a page opened late still sees the last few
  minutes.
- Web admin: a Debug on / off switch for the Log tab, once Scribe has its Debug switch.
- Web admin: saved page layouts, per account.  Jacob's long-term idea from the docking talk on
  2026-09-28.  The web admin has two accounts now (`user` and `admin`); today the only thing remembered is
  the last tab, in the browser.
- Web admin: more accounts than `user` and `admin`, with names of their own.  Two fixed ones were
  Jacob's ask for now.
- Web admin: the Settings tab could offer the default beside a field, and a "back to default" click.
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
- Networking: make the TLS pair itself on first start, the way Stratum did with the `rcgen` crate, instead
  of the openssl command by hand.  One more crate; Jacob picked the command for now (2026-09-29).
- Networking: the private key sits in DiskMan's cache for as long as Conductor runs, like every file read.
  Reading it past DiskMan (the way the monitor reads `/proc`) would keep it out, if it ever matters.
- Networking: the login threads' `serving` list and the failure hold are two small maps under two locks;
  fine at this size.  If the door ever sees thousands of connections a second, look here first.
- Networking: the access lists are a Vec walked on every accept.  Fine for tens of entries; a blacklist
  of thousands would want a map for the single addresses and the Vec for the ranges only.
- Networking: a ban from the page rewrites the whole list file, so a hand-written comment in it is lost.
  Keeping the comments (reading the file, replacing only the entry lines) if anybody minds.
- Running Conductor with no console window.  The page's Log tab shows everything the console does, but
  closing the console kills Conductor today (Linux sends the terminal's hang-up signal, Windows ends the
  process), so it goes down without a clean shutdown.  Ways to fix it: start it detached (`setsid` or
  `nohup` on Linux), run it as a systemd service / Windows service, or build a Windows version with no
  console at all.  Jacob's pick when it matters.
