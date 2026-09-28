<!--
File:       Opus/Documentation/LLM/TODO.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- TODO

## Deferred

Things that wait on a piece that doesn't exist yet.

- The web admin: a web application Conductor hosts, to manage the server through.  Jacob's pick, over a
  desktop GUI.  Modeled on how Jacob's work designs its TLP: critical service status at a glance (Scribe,
  Constellations, Archivist and whatever comes after), and the launcher's console window becomes raw log
  output.  It shows `archivist::status()`.  Needs a name (CLAUDE.md: ask before creating a third piece), and
  almost certainly a crate for the web server, which needs an OK.  The launcher's menu items below would move
  there.
- Scribe: a debug switch in `conductor_globals.cfg` (on or off) that drops Debug lines when off.  Then go
  through every existing log line and move the routine ones to Debug, per the rule in CLAUDE.md.  Archivist's
  connect, schema and settings lines are the obvious first ones.
- Accounts: making, checking and logging in.  Waits on Security for the Argon2 hashing.
- The rest of Conductor's tools, each its own session: the disk manager (temp-file-and-rename writes, the
  one place whole files get replaced), and Security (TLS for the welcome TCP connection, password hashing).
  Stratum had a DiskMan, a Fingerprinter (UUIDs) and a Security worker; whether Opus wants the same shapes
  is Jacob's call when each comes up.
- Constellations writes `conductor_globals.cfg` straight to disk.  Once the disk manager exists it goes
  through that instead, so a crash mid-write can't leave half a config file.
- Launcher: S) Start / stop the server.  Waits on networking and a game loop.
- Launcher: account management (make, delete, list, finger, change password).  Waits on accounts.
- Launcher: config management (show, change, reload).  Reload needs Constellations to stop being load-once.
- Launcher: switch Ctrl-C off so Q is the only way out, once there is something to save on shutdown.  Today
  there isn't, so Ctrl-C is harmless.
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
