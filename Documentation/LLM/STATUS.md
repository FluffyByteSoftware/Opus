<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor has four tools and a program to run them.  `conductor-tools` (lib) holds Scribe, Constellations,
Archivist and the clock; `conductor-launcher` (bin) starts them and runs the admin's text menu (L and Q).
Archivist talks to PostgreSQL 18 on localhost and makes the `accounts` table on its own.  Ensemble hasn't
been started.

All of it builds with no warnings, and all 32 tests pass on Jacob's machine (25 in the tools, 7 in the
launcher).  A run connected to `opusdb` as `opus_game`, read the server version back, ran the accounts
schema, and added the new settings to `postgres.cfg`.  `\d accounts` in psql matched the design exactly.

## Last session -- 2026-09-28

Archivist, the database.

What we did:

- Added the `postgres` crate (the blocking client) and wrote Archivist on a thread of its own.  Jobs go in
  a mailbox and come back as a `Pending`: `check()` never waits, `wait()` does.  So the game never waits on
  the database.
- `Content/cfg/postgres.cfg` holds the address, port, database, username and password, plus a query time
  limit (10 s, Postgres cancels anything longer) and a slow-job limit (250 ms, logged as a Warn).  Settings
  the file is missing get added to the end with their defaults.
- The `accounts` table, in `Content/psql/defaults/schemas/accounts.sql`, run on every connect with
  `CREATE ... IF NOT EXISTS` and baked into Conductor in case the file goes missing.
- Migrations: numbered files in `Content/psql/migrations/`, each run once, in order, in a transaction,
  tracked in `archivist_migrations`.  None written yet.
- `archivist::transaction(name, |tx| ...)` for all-or-nothing work where one step uses the last one's result.
- `archivist::status()`: running, connected, jobs waiting, jobs done, slow jobs.  Nothing shows it yet.
- Split Archivist into `archivist.rs` and `archivist/` (settings, worker, schemas, status).
- `Content/` is committed now, except `Content/Assets/` and `Content/logs/`.
- Merged into `main` as pull request #3, then cleaned up: the stray empty `Content/conductor_globals.cfg`
  removed, and every branch but `main` deleted from GitHub.

What fought back:

- A pool of workers that grew when busy went in and came out the same day.  Two workers can finish jobs out
  of order, so a SELECT could miss the UPDATE sent right before it.  Jacob asked whether async was the answer
  to keep the server from lagging on a write.  It wasn't: the server already doesn't wait, since every job
  hands back a `Pending` straight away.  So it's one worker, jobs in order, and the speed comes from keeping
  prepared statements instead.
- The first commit carried attribution lines that CLAUDE.md doesn't allow in commit messages.  Amended
  before anything was built on it.

What Jacob decided:

- The database piece is called Archivist and lives in `conductor-tools`.
- `postgres.cfg` lives in `Content/cfg/` and is committed.  The password is a placeholder, and Postgres
  only listens on localhost.
- Schemas in `Content/psql/defaults/schemas/`, no file headers (nothing in `Content/` gets one).  A schema
  file is frozen once its table exists; every change after is a migration.
- Accounts: `account_username` 8 to 32 characters of `a-z`, `0-9`, `_`; the owner's name as they capitalize
  it; one account per email; Argon2 for the password hash, later.
- Transactions as a closure (option B), so a step can use the last step's result.
- One worker.  No async.
- The admin interface outgrows a console.  Jacob wants a web application Conductor hosts to manage the
  server through, modeled on how the TLP at his work is designed: critical service status at a glance, and
  the console window becomes raw log output.  He plans to work on it before anything else.
- How we work from here: Jacob steers, the sessions clarify what he means and then write it, in small steps
  so the history reads well.  He's hands-off on files; the sessions edit everything, CLAUDE.md included,
  and merge when he says so.
- Most log messages should be Debug, with a switch in the config to turn Debug off, so a finished server's
  log isn't chatty.  The rule is in CLAUDE.md; the switch and the pass over existing lines are in TODO.

## What's waiting

- The web admin, and the launcher's console going back to being raw output for Scribe.  Jacob picked this
  for the next conversation.  Open before it starts: its name (CLAUDE.md says to ask before creating a third piece), what
  "the TLP at work" looks like (Jacob to describe it), which web server crate (needs an OK), whether it lives
  in a new lib crate the launcher starts, and how it's kept to this machine or logged into.
- The Debug switch in `conductor_globals.cfg`, and moving the routine log lines to Debug.
- `\dt` in psql to confirm `archivist_migrations` exists.  It should, but it wasn't looked at.
- Accounts (make, check, log in), which waits on Security for Argon2.
- The rest of Conductor's tools: the disk manager and Security.
- Picking Ensemble's engine.
