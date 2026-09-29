<!--
File:       Opus/Documentation/LLM/design/conductor-tools.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-tools

A lib crate.  The pieces the rest of Conductor leans on but that know nothing about the game: the disk, the
log, the config, the UUIDs, the password hashing, the database, the clock, the list of threads we started,
the list of services we expect, and the notices the admin has to acknowledge.  Two dependencies: `postgres`
(the blocking Postgres client) for Archivist, which pulls in tokio behind the scenes though nothing of ours
is async, and `argon2` for Security, with its default features off.

## Skeleton

```
conductor-tools/
├── Cargo.toml
└── src/
    ├── lib.rs             pub mod archivist; clock; constellations; diskman; fingerprinter; notices; pending;
    │                        scribe; security; services; threads;
    ├── archivist.rs       the front door: start(), stop(), status(), config_path()
    │                        execute(sql, params) -> Pending<u64>, query(sql, params) -> Pending<Vec<Row>>
    │                        batch(sql) -> Pending<()>, transaction(name, |tx| ...) -> Pending<T>
    │                        type Pending<T> (from pending.rs), enum ArchivistError, type Param
    ├── archivist/
    │   ├── settings.rs    postgres.cfg: struct DbSettings, load(), adds missing settings to the file
    │   ├── worker.rs      the one worker thread, its mailbox, struct Link (the connection, prepared statements)
    │   ├── schemas.rs     get_in_shape(): default schemas, then migrations
    │   └── status.rs      struct Status, struct SlowJob, enum JobKind (read, write, other), the totals
    ├── clock.rs           Utc { year, month, day, hour, minute, second }
    │                        Utc::now(), Utc::from_unix(seconds)
    │                        date(), file_stamp() -> "2026_09_28", line_stamp() -> "02:16:43 PM - 09-28-26 Z"
    ├── scribe.rs          enum Channel { System, Network, Security, Database, Game }
    │                      enum Priority { Debug, Info, Warn, Error }
    │                        start(dir), move_to(dir), current_file()
    │                        recent_lines(after) -> Vec<RecentLine { number, priority, text }>
    │                        debug / info / warn / error (channel, message)
    │                        debug_with / info_with / warn_with / error_with (channel, err, message)
    ├── diskman.rs         the front door: start(), stop(), finished(), status(), waiting_files()
    │                        write(path, bytes) -> Pending<()>, append(path, bytes) -> Pending<()>
    │                        read(path) -> Pending<Contents>, stream(path) -> Stream, as_text(bytes)
    │                        type Contents = Arc<Vec<u8>>, enum DiskError, enum Piece, struct Status
    ├── diskman/
    │   ├── cache.rs       struct State, struct Entry (whole file, dirty, tail), the rules; Limits
    │   └── worker.rs      the one worker thread: reads, writes, the big write, streams; the disk calls
    ├── fingerprinter.rs   start(), new_uuid() -> io::Result<String>, new_token(), random_bytes(buffer)
    │                        looks_like_uuid(text), uuid_time(text) -> Option<Utc>
    ├── fingerprinter/     fill(bytes) per OS: linux.rs (getrandom), windows.rs (BCryptGenRandom), other.rs
    ├── security.rs        start(), stop()
    │                        hash_password(password) -> Pending<String>
    │                        verify_password(password, stored) -> Pending<bool>
    │                        verify_no_account(password) -> Pending<()>
    │                        check_password_rules(password) -> Result<(), String>, pad_login_time(started)
    │                        type Pending<T> (from pending.rs), enum SecurityError { NotRunning, Failed }
    ├── security/          advise_huge_pages(start, bytes) per OS: linux.rs (madvise), windows.rs, other.rs
    ├── notices.rs         enum Level { Notice, Warn, Error }, struct Notice { id, when, level, source, text }
    │                        publish(level, source, text) -> id, newest(n), all(), ack(id), ack_all()
    ├── pending.rs         Pending<T, E> { check() never waits, wait() does }, trait NotRunning
    ├── constellations.rs  struct Settings { scribe_log_dir, wgui_port }
    │                        load(), settings(), log_dir(), content_dir(), config_path()
    ├── services.rs        enum State { Expected, Starting, Running, Trouble, Stopped }
    │                        set(name, state, note), seen(name), list() -> Vec<Service>
    │                        Service { name, state, note, since, seen_ago }, healthy(); the names as consts
    └── threads.rs         spawn(name, work) -> io::Result<JoinHandle<T>>, list() -> Vec<ThreadRecord>
                             name_this_thread(name), for main
                             ThreadRecord { name, started_by, started_at, os_id, running }
```

## DiskMan

The disk manager.  Built 2026-09-28.  Named by Jacob.  Every file Conductor reads or writes goes through it.

Jacob's idea, nearly in his words: DiskMan is a layer between the server and the disk.  It buffers what's
written and uses a single background thread to keep streaming it to disk.  The disk can only go so fast, so
while it's writing, whatever comes in is edited in memory and marked dirty, and DiskMan knows to flush it.
Anything that isn't dirty can just be unloaded.

What we decided:

- **One worker thread**, `diskman`, and nobody else waits on the disk.  `write()`, `append()` and `read()`
  hand back a `Pending` straight away, the same one Archivist uses (`pending.rs`).
- **An entry per file**, in `cache.rs`.  It holds the whole file when we know it (after a read or a
  `write()`), marked dirty until it's on disk, and a tail of appends for a file we don't hold whole (Scribe's
  log).  A second `write()` before the first goes out just replaces it in memory: only the newest reaches
  the disk, and everyone who was waiting hears back when it lands.
- **Reads are cached.**  A file that's been read stays loaded, and the next read comes from memory.  A read
  of a file with appends still waiting gets the disk plus the appends.
- **Clean files unload past 256 MB**, the one used longest ago first.  Dirty files don't count and are never
  unloaded, whatever their size: a few GB of world terrain is held until it's on disk.
- **Whole writes never leave half a file**: temp file (`name.diskman-tmp`, same folder), flushed, renamed over
  the old one, and on Linux the folder flushed too.  Appends are flushed but not all-at-once; a crash can cut
  off the last line.
- **The chunker**: a whole write over 8 MB goes into its temp file 1 MB at a time, and other files get their
  turn in between, so terrain never holds up a config or a log line.  One big write at a time.  A newer
  `write()` of the same file drops the big one part way and starts over with the newer.
- **`stream()`** reads a file back 1 MB at a time, 4 chunks ahead of the reader at most, without caching it.
- **A failed write is tried 3 times, 5 seconds apart, then given up** with a capitals Error naming the file.
  Jacob's call: a folder that can't be written would otherwise hang shutdown forever.  The `Pending` hears the
  first failure; DiskMan keeps trying behind it.
- **Nothing routine is logged.**  Every log line is itself a write, so a "wrote a file" line would be another
  write, forever.  Only failures and recoveries get a line.  Problems with Scribe's own file go to the
  console and the Services tab only.
- **The lock is never held while touching the disk or logging.**  That keeps callers free while it writes,
  and it's what stops a deadlock with Scribe, who hands DiskMan every line while holding Scribe's lock.
- **First to start, last to stop.**  Shutdown waits on it after Archivist.  If it takes more than a second,
  the console counts down from 60; at zero it says `SHOULD BE CLOSED, IF STILL RUNNING PLEASE FORCE QUIT` and
  lists what would be lost, again every 30 seconds, while it keeps waiting.  Jacob's wording.
- Folders aren't DiskMan's: making the empty migrations folder and listing a folder stay with `std::fs`.
- Paths go in full (from `content_dir()`); the same file under two spellings would be two files to it.

What's open:

- **Hand edits while running aren't seen** if DiskMan already holds the file.  Today only configs, read once
  at startup.  Checking the file's modified time before trusting the copy would fix it.
- The Windows side (renaming over a file, no folder flush) hasn't been built there.
- Nothing uses the big write or `stream()` for real yet: there's no terrain.  The tests cover them with tiny
  sizes.

## Fingerprinter

The UUID maker, in `fingerprinter.rs`.  Built 2026-09-28.  The design is in the file's header; the short
version:

- **Version 7 UUIDs**: the time in milliseconds, a 12-bit counter, then random bytes.  So they sort in the
  order they were made, two in the same millisecond included.  Postgres 18's `uuidv7()` is the safety net on
  every table's `uuid` column.
- **Random bytes come straight from the OS**, never through DiskMan (which would cache them): `getrandom()`
  on Linux, `BCryptGenRandom()` on Windows (never built), and a refusal anywhere else.  `start()` asks for 16
  bytes once so a machine that can't is caught at startup.
- `new_token()` is 32 random bytes as hex, for logins.  `random_bytes()` is the raw source for anything else
  that needs some; Security's salts, today.

## Security

The password hasher, in `security.rs`.  Written 2026-09-29, modelled on Stratum's, with the emphasis this
time on spending less CPU per hash and more RAM where that buys the same protection.  Not built yet.

What we decided:

- **Argon2id, through the `argon2` crate** (0.6, default features off).  Nobody writes their own password
  hash.  What's stored is the PHC string, `$argon2id$v=19$m=65536,t=1,p=1$<salt>$<hash>`, which carries its
  own settings, so an old account still checks after the settings change.
- **The settings: 64 MiB, one pass, one lane, a 32-byte hash, a 16-byte salt.**  Argon2's CPU time is
  close to memory times passes, and what an attacker's graphics card is short of is memory, so the trade is
  passes down to one and memory as high as we can spare.  To make a hash harder later, raise the memory.
  Stratum measured 64 MiB at two passes at about 85 ms; one pass should be near half that, and the benchmark
  says for real.
- **One worker thread, `security`, one hash at a time**, jobs in order, `Pending` back to the caller the
  same as Archivist and DiskMan.  Stratum's tick-sim showed 50 logins hashing on 50 threads took 1.2 s each
  and blew the tick; one at a time, each took 72 ms and all 50 were done in 3.6 s.
- **One arena, allotted once and kept**: 64 MiB of `Block`s the worker owns for Conductor's whole run.
  The crate would otherwise ask the OS for a fresh 64 MiB on every hash (16,384 page faults' worth of CPU
  that isn't ours).  Every hash runs in the arena through `hash_password_into_with_memory()`; a stored line
  made with more memory than the arena holds gets a one-off allocation instead of no answer.
- **The huge page hint on Linux**: `madvise(MADV_HUGEPAGE)` on the arena before its pages are touched, so
  Argon2's random jumps through 64 MiB land in 32 pages instead of 16,384 and the TLB covers all of it.  A
  hint, in the OS file `security/linux.rs`; Windows and macOS have nothing yet and say so.  The Services tab
  note says whether the kernel took the hint.
- **A name with no account still costs a hash** (`verify_no_account()`), in the same line, and
  `pad_login_time()` makes every login attempt take at least 150 ms, so a stopwatch can't tell a real name
  from a made-up one.  Both carried over from Stratum.
- **The password rules are Jacob's**: 8 to 128 characters, printable ASCII, at least one digit, one capital
  and one symbol.  `check_password_rules()` says which one in words for the player.
- **The salt comes from Fingerprinter**, so the crate's own random source (and the crates behind it) stays
  out of the build.
- A wrong password is `Ok(false)`.  A stored line that can't be read is `Err` and an Error on the Security
  channel, without the line in it.  Nothing here ever logs a password.
- The benchmark, `cargo test -p conductor-tools argon2_cost -- --ignored --nocapture`, times five memory
  settings at one and two passes three ways: fresh memory each hash, the kept arena, and the arena with the
  huge page hint.  The gaps between the columns are the page faults and the TLB, measured.

What's open:

- **Nothing is measured yet.**  The numbers above are Stratum's and the theory's; the benchmark on Jacob's
  machine sets the real ones, and the memory setting may go up after it.
- Rayon lanes would cut a single hash's wall time across cores at the same CPU cost, but it's a crate and
  its threads bypass `threads::spawn()`.  Not taken.
- Not hashing at all on a reconnect (a token from Fingerprinter instead) is the biggest CPU saving there is,
  and it's accounts' job once there are accounts.
- Passwords sit in ordinary `String`s while they're in line and aren't wiped after.  The crate's `zeroize`
  feature is off.  Fine for a hobby server; written down so it's a choice, not an oversight.

## Notices

What the admin has to see and acknowledge, in `notices.rs`.  Built 2026-09-28, the same session as DiskMan.

What we decided:

- **Every Warn and Error Scribe logs becomes one**, and code can raise one on purpose with
  `notices::publish(Level::Notice, source, text)`.
- **A notice stays until it's ACKed**, one at a time or ACK ALL, and then it's gone completely.  Looking at
  it doesn't count.
- **Memory only, since Conductor started.**  Jacob picked this over a journal file or a Postgres table: a
  restart wipes them.  (A table couldn't hold "the database dropped" anyway.)  No cap.
- Nothing in here writes to Scribe; Scribe calls in here before taking its own lock.
- The bell, its tray and the Notifications History tab are in `conductor-wgui.md`.

## Scribe

What we decided:

- One file per UTC day, `Content/logs/2026_09_28.scribe.log`, appended to, rolling at midnight UTC.  Year
  first so the folder sorts by date.
- The line is Jacob's layout:
  `[ 02:16:43 PM - 09-28-26 Z ] - [ System / Info ] - [ message ] [ Caller: file, Line: n ]`
- The caller comes from `#[track_caller]`, so nobody passes a file and line by hand.
- An error value goes in front of the message (`error_with()`), the way `ex.Message` did in C#.  Rust has
  no exceptions, so it takes anything that prints.
- **Scribe doesn't touch the disk.**  Since 2026-09-28 each line is handed to DiskMan as an append, still
  under Scribe's lock so the order holds, and DiskMan decides when it goes out.  DiskMan starts first.
- **A Warn or an Error also becomes a notice** for the web admin's bell.
- **Every line goes three places: the file, the terminal, and the last 200 in memory.**  The terminal is the
  launcher's console, which is only Scribe's output since the menu went away.  The 200 in memory are what
  the web admin shows, numbered from 1 so the page can ask for the ones after the last it saw.  (Until
  2026-09-28 the file was the only place, because the terminal belonged to the L/Q menu.)
- The terminal write ignores a failure instead of using `println!`, which panics when stdout is gone.
- A file that won't write is DiskMan's to report: one note on stderr, the Services tab, and its usual retries.
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
- **Nothing in here stops the server.**  A missing file gets written with the defaults, through DiskMan.  A file that can't
  be read is an Error and we run on the defaults, without writing over it.
- Loaded once, at startup.  A relative path in the file is taken from `Content/`.
- A new setting touches four places, all in `constellations.rs`: the struct, `default_settings()`,
  `apply_setting()` and `file_text()`.

Settings today:

| Key              | Default | What it is                                   |
|------------------|---------|----------------------------------------------|
| `scribe_log_dir` | `logs`  | The folder Scribe writes into, under Content |
| `wgui_port`      | `9996`  | The web admin's port, on 127.0.0.1 only      |

What's open:

- No reload.  The launcher's config menu will need one, which means Constellations stops being load-once.

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
  password is a placeholder and Postgres only listens on localhost.  It, the schemas and the migrations are
  read and written through DiskMan.
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
  jobs, the slowest, and the last 5 slow ones.  It also counts `query()` jobs as reads, `execute()` jobs as
  writes, and `batch()` and `transaction()` as other.  The web admin shows all of it.
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

- Most of its log lines are Info today and should be Debug (see TODO).
- A password with a space at either end loses it, since every value is trimmed.

## Threads

The list of threads our code started, in `threads.rs`.

What we decided:

- **Every thread goes through `threads::spawn(name, work)`**, never `std::thread::spawn`.  It records the
  name, the file and line that called it (`#[track_caller]`, the same trick Scribe uses), and when.
- The thread writes down the OS's own number for itself as its first act: `/proc/thread-self` on Linux,
  `GetCurrentThreadId` on Windows, nothing elsewhere.  conductor-monitor matches that number to what the OS
  reports, which is how a thread in the "in use" view gets our name and an "ours" mark.
- A thread is marked finished when its closure ends, a panic included (a guard that's dropped either way).
  Finished threads stay on the list.  There are a handful of them, not thousands.
- Threads today: `main`, `diskman`, `security`, `archivist`, `monitor`, `wgui`.  The postgres crate starts
  some of its own, and those show up as "not ours".
- main can't be started by `spawn()`, so it puts itself on the list with `name_this_thread("main")` as the
  first line of `main()`.  It stays "running" for good, since main ending ends Conductor.

## Services

The list of services Conductor expects, in `services.rs`, and how each says it's doing.  The web admin's
Services tab shows it.  Built 2026-09-28, Zabbix style.

What we decided:

- Nothing can look into a service from outside and tell whether it's alive, so **each one reports on
  itself**: `services::set(name, state, note)`, with a note that says what it's doing or what went wrong.
- **Every expected service is on the list from the start**, as "expected", so one that never started shows
  as missing.  The list is `EXPECTED` in `services.rs`: DiskMan, Scribe, Constellations, Fingerprinter,
  Security, Archivist, Monitor, Web admin, each with the name of its thread if it has one.  Adding a service
  means adding it there.
- A service with a thread is **stopped once that thread has ended**, whatever it last said.  A thread that
  panics says nothing on the way out.  This is worked out when the list is read, from `threads::list()`.
- A service can **check in** with `seen(name)`.  One that has checked in and then goes quiet for more than
  `QUIET_LIMIT` (5 seconds) isn't healthy.  The monitor, DiskMan and Security do; the others have no loop to
  check in from.
- Healthy means running and not gone quiet.  The page shows starting as yellow, not down.
- **Nothing in `services.rs` writes to Scribe.**  Scribe reports to the list, so a call the other way could
  leave each waiting on the other's lock.
- Who reports what is the table in `conductor-wgui.md`.

## The clock

The standard library stops at seconds since 1970, so the calendar is worked out by hand in `clock.rs`.  The
tests pin it against dates we know: the epoch, a leap day in 2000, and the last second before midnight on
2026-09-28 rolling into the 29th.
