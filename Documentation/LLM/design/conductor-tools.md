<!--
File:       Opus/Documentation/LLM/design/conductor-tools.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-tools

A lib crate.  The pieces the rest of Conductor leans on but that know nothing about the game: the log, the
config, the database, and the clock.  One dependency, `postgres` (the blocking Postgres client), for
Archivist.  It pulls in tokio behind the scenes, but nothing of ours is async.

## Skeleton

```
conductor-tools/
├── Cargo.toml
└── src/
    ├── lib.rs             pub mod archivist; pub mod clock; pub mod constellations; pub mod scribe;
    ├── archivist.rs       the front door: start(), stop(), status(), config_path()
    │                        execute(sql, params) -> Pending<u64>, query(sql, params) -> Pending<Vec<Row>>
    │                        batch(sql) -> Pending<()>, transaction(name, |tx| ...) -> Pending<T>
    │                        Pending<T> { check() never waits, wait() does }, enum ArchivistError, type Param
    ├── archivist/
    │   ├── settings.rs    postgres.cfg: struct DbSettings, load(), adds missing settings to the file
    │   ├── worker.rs      the one worker thread, its mailbox, struct Link (the connection, prepared statements)
    │   ├── schemas.rs     get_in_shape(): default schemas, then migrations
    │   └── status.rs      struct Status, struct SlowJob, the running totals
    ├── clock.rs           Utc { year, month, day, hour, minute, second }
    │                        Utc::now(), Utc::from_unix(seconds)
    │                        date(), file_stamp() -> "2026_09_28", line_stamp() -> "02:16:43 PM - 09-28-26 Z"
    ├── scribe.rs          enum Channel { System, Network, Security, Database, Game }
    │                      enum Priority { Debug, Info, Warn, Error }
    │                        start(dir), move_to(dir), current_file()
    │                        debug / info / warn / error (channel, message)
    │                        debug_with / info_with / warn_with / error_with (channel, err, message)
    └── constellations.rs  struct Settings { scribe_log_dir }
                             load(), settings(), log_dir(), content_dir(), config_path()
```

## Scribe

What we decided:

- One file per UTC day, `Content/logs/2026_09_28.scribe.log`, appended to, rolling at midnight UTC.  Year
  first so the folder sorts by date.
- The line is Jacob's layout:
  `[ 02:16:43 PM - 09-28-26 Z ] - [ System / Info ] - [ message ] [ Caller: file, Line: n ]`
- The caller comes from `#[track_caller]`, so nobody passes a file and line by hand.
- An error value goes in front of the message (`error_with()`), the way `ex.Message` did in C#.  Rust has
  no exceptions, so it takes anything that prints.
- **The file is the only place a line goes.**  The terminal belongs to the launcher's menu, and L is how the
  log gets read.  The exception is a line with no file to go to (before `start()`, or a file that can't be
  opened or written).  That one prints to the terminal, because a line nobody can see anywhere is worse than
  one in the middle of the menu.
- A lost file gets one note on stderr and is tried again at the next midnight or `move_to()`.
- Scribe never panics and never hands an error back from a log call.  A log that takes the server down is
  worse than no log.
- Scribe starts before Constellations, on the default folder, so the config's complaints have somewhere to
  go.  `move_to()` follows the config once it's loaded.

What's open:

- The few lines before `move_to()` stay in the default folder if the config points elsewhere.
- No size limit, no minimum priority.  Both are in TODO as ideas.

## Constellations

What we decided:

- `Content/cfg/conductor_globals.cfg`, plain `key = value` lines, `#` for comments, keys in any case.
  TOML would have cost a crate to read a handful of lines.
- `Content/` is found through `OPUS_CONTENT`, then by walking up from the working directory, then
  `./Content`.  No drive path is ever hardcoded, so a fresh checkout anywhere runs.
- A checked struct: every key has a type and a check.  A bad line is a Warn with its line number, and that
  setting keeps its default.  An unknown key is a complaint too, so a typo doesn't go silent.  If a key
  shows up twice the later one wins, with a complaint.
- **Nothing in here stops the server.**  A missing file gets written with the defaults.  A file that can't
  be read is an Error and we run on the defaults, without writing over it.
- Loaded once, at startup.  A relative path in the file is taken from `Content/`.
- A new setting touches four places, all in `constellations.rs`: the struct, `default_settings()`,
  `apply_setting()` and `file_text()`.

Settings today:

| Key              | Default | What it is                                   |
|------------------|---------|----------------------------------------------|
| `scribe_log_dir` | `logs`  | The folder Scribe writes into, under Content |

What's open:

- No reload.  The launcher's config menu will need one, which means Constellations stops being load-once.
- The file is written straight to disk.  It goes through the disk manager once that exists.

## Archivist

The database.  Named by Jacob.

What we decided:

- **One worker thread, one connection, jobs in order.**  The rest of the server never waits on it:
  `execute()`, `query()`, `batch()` and `transaction()` put a job in the mailbox and hand back a `Pending`
  straight away.  `check()` never waits (the game loop uses that one), `wait()` does (startup and admin use).
- We tried a pool of workers that grew when the mailbox got busy.  It came out the same day: two workers can
  finish jobs out of order, so a SELECT could miss the UPDATE sent just before it.  Nobody waits on the
  worker anyway, so one is enough.  Async (tokio) was talked about and turned down for the same reason -- the
  game already doesn't wait, and async would only add machinery.
- For speed, the worker keeps every statement it has prepared (up to 500), so a query sent over and over is
  planned by Postgres once.  The list is emptied on reconnect and after any `batch()`.
- Values always go in as `$1`, `$2` params, never pasted into the SQL.  `batch()` is for our own SQL only.
- `transaction(name, |tx| ...)` runs a closure on the worker inside a transaction, so one step can use the
  last one's result (insert an account, get its id, use it).  All of it commits or none of it does.  The rule:
  never wait on anything else inside it.
- `Content/cfg/postgres.cfg`: `address`, `port`, `database`, `username`, `password`,
  `query_time_limit_seconds` (10, handed to Postgres as `statement_timeout`; 0 is no limit) and
  `slow_job_ms` (250).  A missing file is written with an empty password and a capitals Error.  A setting
  the file doesn't have is added to the end with its default.  The file is committed on purpose: the
  password is a placeholder and Postgres only listens on localhost.
- The password never reaches the log: `DbSettings` has a hand-written Debug, and a broken line isn't echoed.
- On every connect, before any job: the schemas in `Content/psql/defaults/schemas/` (only `CREATE ... IF NOT
  EXISTS`, baked in with `include_str!` and written back out if missing), then the migrations in
  `Content/psql/migrations/`.
- **A schema file is frozen once its table exists.**  Every change after that is a migration named
  `0001_what_it_does.sql`, run once, in number order, in a transaction with its row in
  `archivist_migrations`.  A failure stops the rest.  A badly named file or two with one number stops them
  all.  Migrations skip the query time limit.
- A job that runs at least `slow_job_ms` is a Warn with its time, its wait in the mailbox, and the first 80
  characters of its SQL (never the values).  `status()` keeps running, connected, waiting, jobs done, slow
  jobs, the slowest, and the last 5 slow ones.  Nothing shows it yet.
- If Postgres is down, jobs come back `NotConnected`, the first failure is logged once, and it tries again
  on a later job, no more than once every 5 seconds.  Nothing in Archivist stops the server.

Tables today:

| Table                  | Made by                | What it is                                              |
|------------------------|------------------------|---------------------------------------------------------|
| `accounts`             | `schemas/accounts.sql` | One row per account.  Columns below.                    |
| `archivist_migrations` | Archivist itself       | Which migrations have run, and when.                    |

`accounts`: `id` (from Postgres), `account_username` (8 to 32 of `a-z`, `0-9`, `_`, unique),
`owner_first_name` and `owner_last_name` (as the owner capitalizes them), `owner_email` (loosely checked,
one account per address ignoring case), `password_hash` (the Argon2 string, once Security exists),
`created_at`, `last_login_datetime` (empty until the first login).  Postgres checks the name and email
itself, so even a bug in Conductor can't store a bad one.  `last_played_character` comes as a migration once
there are characters.

What's open:

- Nothing shows `status()` yet.  The web admin will.
- Most of its log lines are Info today and should be Debug (see TODO).
- A password with a space at either end loses it, since every value is trimmed.

## The clock

The standard library stops at seconds since 1970, so the calendar is worked out by hand in `clock.rs`.  The
tests pin it against dates we know: the epoch, a leap day in 2000, and the last second before midnight on
2026-09-28 rolling into the 29th.
