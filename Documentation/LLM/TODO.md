<!--
File:       Opus/Documentation/LLM/TODO.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- TODO

## Deferred

- PostgreSQL for Conductor.  Needs a crate (`tokio-postgres` or `sqlx`), which needs an OK first.
- The rest of Conductor's tools, each its own session: the disk manager (temp-file-and-rename writes, the
  one place whole files get replaced), and Security (TLS for the welcome TCP connection, password hashing).
  Stratum had a DiskMan, a Fingerprinter (UUIDs) and a Security worker; whether Opus wants the same shapes
  is Jacob's call when each comes up.
- Ensemble has no way to find `Content/` yet.  Decide how once the engine is picked.

## Ideas

- Scribe: a minimum priority in `conductor_globals.cfg` so Debug lines can be turned off outside development.
- Scribe: the caller shows the path Rust compiled with (`src/tools/scribe.rs`).  Trim to the file name if that
  gets noisy.
- Constellations: log a warning at startup for a config key that is missing and fell back to its default, the
  way unknown keys are warned about today.
