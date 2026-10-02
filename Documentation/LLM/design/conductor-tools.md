<!--
File:       Opus/Documentation/LLM/design/conductor-tools.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-tools

A lib crate.  The pieces the rest of Conductor leans on but that know nothing about the game: the disk, the
log, the config, the UUIDs, the password hashing, the database, the clock, the list of threads we started,
the list of services we expect, the notices the admin has to acknowledge, and the switch that starts and
stops the server.  Two dependencies: `postgres` (the blocking client) for Archivist, which pulls in tokio
behind the scenes though nothing of ours is async, and `argon2` 0.6 for Security, default features off,
with `alloc` (for a stored line that wants more memory than the arena) and `password-hash` (the PHC string).

## Skeleton

```
tools/
├── Cargo.toml
└── src/
    ├── lib.rs             pub mod archivist; clock; constellations; diskman; fingerprinter; notices; pending;
    │                        scribe; security; server; services; threads;
    ├── archivist.rs       the front door: start(), stop(), status(), config_path()
    │                        execute(sql, params) -> Pending<u64>, query(sql, params) -> Pending<Vec<Row>>
    │                        batch(sql) -> Pending<()>, transaction(name, |tx| ...) -> Pending<T>
    │                        type Pending<T>, type Param, enum ArchivistError { NotRunning, NotConnected,
    │                        Postgres }; hands on Row, Transaction, ToSql
    ├── archivist/
    │   ├── settings.rs    postgres.cfg as Archivist reads it: struct DbSettings, load() (via Constellations)
    │   ├── worker.rs      the one worker thread, its mailbox, struct Link (the connection, prepared statements)
    │   ├── schemas.rs     get_in_shape(): default schemas, then migrations; DEFAULT_SCHEMAS
    │   └── status.rs      struct Status, struct SlowJob, enum JobKind (read, write, other), the totals
    ├── clock.rs           Utc { year, month, day, hour, minute, second }
    │                        Utc::now(), Utc::from_unix(seconds)
    │                        date(), file_stamp() -> "2026_09_28", line_stamp() -> "02:16:43 PM - 09-28-26 Z"
    ├── scribe.rs          enum Channel { System, Network, Security, Database, Game, Script }
    │                      enum Priority { Debug, Info, Warn, Error }, RECENT_LINES (200)
    │                        start(dir), move_to(dir), current_file()
    │                        recent_lines(after) -> Vec<RecentLine { number, priority, text }>
    │                        debug / info / warn / error (channel, message)
    │                        debug_with / info_with / warn_with / error_with (channel, err, message)
    ├── diskman.rs         the front door: start(), stop(), finished(), status(), waiting_files()
    │                        write(path, bytes) -> Pending<()>, append(path, bytes) -> Pending<()>
    │                        read(path) -> Pending<Contents>, stream(path) -> Stream, as_text(bytes)
    │                        swap(original, replacement, SwapAt) -> Pending<()>, run_swaps(SwapAt),
    │                        forget_swap(original), remove(path) -> Pending<()>
    │                        type Contents = Arc<Vec<u8>>, enum DiskError, enum Piece, enum SwapAt, struct Status
    ├── diskman/
    │   ├── cache.rs       struct State, struct Entry (whole file, dirty, tail), the swap list, the rules; Limits
    │   └── worker.rs      the one worker thread: reads, writes, removes, swaps, the big write, streams
    ├── fingerprinter.rs   start(), stop(), new_uuid() -> io::Result<String>, new_token(), random_bytes(buffer)
    │                        looks_like_uuid(text), uuid_time(text) -> Option<Utc>
    ├── fingerprinter/     fill(bytes) per OS: linux.rs (getrandom), windows.rs (BCryptGenRandom), other.rs
    ├── security.rs        start(), stop()
    │                        hash_password(password) -> Ticket<String>
    │                        verify_password(password, stored) -> Ticket<bool>
    │                        verify_no_account(password) -> Ticket<()>
    │                        Ticket<T> { check(), wait_for(wait), wait(), place() -> Place { ahead, wait } }
    │                        check_password_rules(password) -> Result<(), String>, pad_login_time(started)
    │                        type Pending<T>, enum SecurityError { NotRunning, Failed }
    ├── security/          advise_huge_pages(start, bytes) per OS: linux.rs (madvise), windows.rs, other.rs
    ├── notices.rs         enum Level { Notice, Warn, Error }, struct Notice { id, when, level, source, text }
    │                        publish(level, source, text) -> id, newest(n), all(), ack(id), ack_all()
    ├── pending.rs         Pending<T, E> { check() never waits, wait_for(wait) waits so long, wait() does }
    │                        trait NotRunning
    ├── constellations.rs  the store: load(file), value / number / port / folder (file, key), values(file)
    │                        settings() -> Settings { scribe_log_dir, wgui_port }, log_dir(), content_dir()
    │                        path_of(file), waiting_path(file), config_path(), file_named(name)
    │                        is_loaded(file), file_values(file) (as it sits on disk)
    │                        save_waiting(file, text), waiting(file), discard_waiting(file), server_stopped()
    ├── constellations/
    │   ├── files.rs       the table: enum Reboot { Soft, Hard }, enum Kind, struct Setting, struct ConfigFile
    │   │                    GLOBALS (conductor_globals.cfg, hard), WGUI (wgui.cfg, hard),
    │   │                    POSTGRES (postgres.cfg, soft), NETWORKING (networking.cfg, soft),
    │   │                    GAME (game.cfg, soft), FILES
    │   └── text.rs        the one reader and writer: parse(file, text) -> Parsed, check(setting, value)
    │                        file_text(file, values), missing_text(file, seen), defaults(file); type Values
    ├── server.rs          enum State { Stopped, Starting, Running, Stopping }, enum Command { Start, Stop, Restart }
    │                        ask(command) -> Result<(), State>, next_command(wait) -> Option<Command>
    │                        set(state, note), status() -> Status { state, note, since }
    ├── services.rs        enum State { Expected, Starting, Running, Trouble, Stopped }, QUIET_LIMIT
    │                        set(name, state, note), seen(name), list() -> Vec<Service>
    │                        Service { name, state, note, since, seen_ago }, healthy(); the names as consts
    └── threads.rs         spawn(name, work) -> io::Result<JoinHandle<T>>, list() -> Vec<ThreadRecord>
                             name_this_thread(name), for main
                             ThreadRecord { name, started_by, started_at, os_id, running }
```

## DiskMan

The disk manager, named by Jacob.  Every file Conductor reads or writes goes through it.

Jacob's idea, nearly in his words: DiskMan is a layer between the server and the disk.  It buffers what's
written and uses a single background thread to keep streaming it to disk.  The disk can only go so fast, so
while it's writing, whatever comes in is edited in memory and marked dirty, and DiskMan knows to flush it.
Anything that isn't dirty can just be unloaded.

- **One worker thread**, `diskman`, and nobody else waits on the disk.  `write()`, `append()` and `read()`
  hand back a `Pending` straight away, the same one Archivist uses (`pending.rs`).
- **An entry per file**, in `cache.rs`: the whole file when we know it (after a read or a `write()`), dirty
  until it's on disk, and a tail of appends for a file we don't hold whole (Scribe's log).  A second
  `write()` before the first goes out replaces it in memory; only the newest reaches the disk, and everyone
  who was waiting hears back when it lands.  A file that's been read stays loaded; a read of one with
  appends still waiting gets the disk plus the appends.
- **A hand edit is noticed** (Jacob's pick, 2026-09-29).  Each entry keeps the file's modified time and
  size from when the copy last matched the disk.  A read of a clean copy has the worker ask the disk for
  those two again, never the bytes: the same, and the copy is the answer; different, gone, or unknown, and
  the file is read again.  The time is asked for before the bytes, so a change in between is caught next
  time rather than missed.  A copy with a write or appends of ours still on its way is newer than the disk
  and answered straight away, so a hand edit made meanwhile is written over.  Streams follow the same rule.
  The cost is one turn of the worker per read, and nothing that reads is in a hurry.  It's what makes
  "STOP SERVER, edit the file, START SERVER" true.
- **Clean files unload past 256 MB**, the one used longest ago first.  Dirty files don't count and are never
  unloaded, whatever their size: a few GB of world terrain is held until it's on disk.
- **Whole writes never leave half a file**: temp file (`name.diskman-tmp`, same folder), flushed, renamed
  over the old one, and on any Unix the folder flushed too (Windows closes the file before the rename and
  has no folder flush).  Appends are flushed but not all-at-once; a crash can cut off the last line.
- **The chunker**: a whole write over 8 MB goes into its temp file 1 MB at a time, and other files get their
  turn in between, so terrain never holds up a config or a log line.  One big write at a time; a newer
  `write()` of the same file drops it part way and starts over.  GameWorld's `omega.heights` (about 134 MB)
  is the first real one.  **`stream()`** reads a file back 1 MB at a time, 4 chunks ahead of the reader at
  most, without caching it.
- **A failed write is tried 3 times, 5 seconds apart, then given up** with a capitals Error naming the file.
  Jacob's call: a folder that can't be written would otherwise hang shutdown forever.  The `Pending` hears
  the first failure; DiskMan keeps trying behind it.
- **Nothing routine is logged.**  Every log line is itself a write, so a "wrote a file" line would be
  another write, forever.  Only failures and recoveries get a line; problems with Scribe's own file go to
  the console and the Services tab only.  **The lock is never held while touching the disk or logging**,
  which keeps callers free and stops a deadlock with Scribe, who hands DiskMan every line under its own lock.
- **First to start, last to stop.**  If shutdown waits on it more than a second, the console counts down
  from 60; at zero it says `SHOULD BE CLOSED, IF STILL RUNNING PLEASE FORCE QUIT` and lists what would be
  lost, again every 30 seconds, while it keeps waiting.  Jacob's wording.
- Folders aren't DiskMan's: making one and listing one stay with `std::fs`.  Paths go in full (from
  `content_dir()`); the same file under two spellings would be two files to it.
- **Swaps and removes**, for the config editor.  `swap(original, replacement, when)` renames one file over
  another: now, when the launcher says the server has stopped (`run_swaps(ServerStop)`, which it waits on),
  or as DiskMan's last act at shutdown.  DiskMan holds the list, Jacob's design: a change saved from the web
  admin sits in `name.cfg.wait4server` until its reboot, and DiskMan puts it in place on the way down.  A
  swap waits until both files have nothing left to go out, then drops whatever was held for them, except
  a write or append that landed while the rename ran with the lock let go: that one still goes out
  (2026-10-02; before that it was dropped and its caller heard "not running").  A
  replacement that's gone (discarded) is nothing to do.  At shutdown every swap still listed runs, the
  server-stop ones included: a hard reboot applies the lot.  A second swap for the same original replaces
  the first.  `remove(path)` deletes a file and drops what was held for it.  A failed swap is a capitals
  Error, and the old file stands.

Open: `stream()` has no user yet (the tests cover it with tiny sizes), and the Windows side builds and runs
but hasn't been tested beyond that.

## Fingerprinter

The UUID maker, in `fingerprinter.rs`.  The design is in the file's header; the short version:

- **Version 7 UUIDs**: the time in milliseconds, a 12-bit counter, then random bytes, so they sort in the
  order they were made, two in the same millisecond included.  Postgres 18's `uuidv7()` is the safety net on
  every table's `uuid` column.
- **Random bytes come straight from the OS**, never through DiskMan (which would cache them): `getrandom()`
  on Linux, `BCryptGenRandom()` on Windows, a refusal anywhere else.  `start()` asks for 16 bytes once so a
  machine that can't is caught at startup.
- `new_token()` is 32 random bytes as hex, for logins.  `random_bytes()` is the raw source for anything else;
  Security's salts, today.

## Security

The password hasher, in `security.rs`, modelled on Stratum's, spending less CPU per hash and more RAM where
that buys the same protection.

- **Argon2id, through the `argon2` crate.**  Nobody writes their own password hash.  What's stored is the
  PHC string, `$argon2id$v=19$m=65536,t=1,p=1$<salt>$<hash>`, which carries its own settings, so an old
  account still checks after the settings change.
- **64 MiB, one pass, one lane, a 32-byte hash, a 16-byte salt.**  Argon2's CPU time is close to memory
  times passes, and what an attacker's graphics card is short of is memory, so passes go down to one and
  memory as high as we can spare.  To make a hash harder later, raise the memory.  Jacob picked 64 MiB over
  128 (62 ms) and 256 (127 ms) on the numbers below: 30 ms a login.
- **One worker thread, `security`, one hash at a time**, jobs in order, `Pending` back to the caller.
  Stratum's tick-sim showed 50 logins hashing on 50 threads took 1.2 s each and blew the tick; one at a
  time, each took 72 ms and all 50 were done in 3.6 s.  **Jacob's rule: one login is hashed at a time, hard
  limit, and every other client waits in the queue** in the order it arrived.  Accounts and networking
  build on that line, not around it.
- **A `Ticket` says where a job stands.**  Every job gets a number under the worker's lock, the worker
  counts what it has finished and keeps a running average of one job (30 ms to start, then an eighth of
  each new time), and `place()` turns the three into "N ahead, about M ms".  The Connections tab shows it;
  telling the waiting client isn't built.
- **One arena of 64 MiB**, allotted on every START SERVER, owned by the worker, let go on every STOP.
  Otherwise the crate asks the OS for a fresh 64 MiB on every hash (16,384 page faults' worth of CPU that
  isn't ours).  Every hash runs in it through `hash_password_into_with_memory()`; a stored line made with
  more memory than the arena gets a one-off allocation instead of no answer.
- **The huge page hint on Linux**: `madvise(MADV_HUGEPAGE)` on the arena before its pages are touched, so
  Argon2's random jumps through 64 MiB land in 32 pages instead of 16,384 and the TLB covers all of it.
  Windows (large pages need a privilege and another allocation call) and everything else say no.  The
  Services tab note says whether `madvise()` took the ask.
- **A name with no account still costs a hash** (`verify_no_account()`), in the same line, and
  `pad_login_time()` makes every login attempt take at least 150 ms, so a stopwatch can't tell a real name
  from a made-up one.
- **The password rules are Jacob's**: 8 to 128 characters, printable ASCII, at least one digit, one capital
  and one symbol.  `check_password_rules()` says which one in words.
- **The salt comes from Fingerprinter**, so the crate's own random source (and the crates behind it) stays
  out of the build.
- A wrong password is `Ok(false)`.  A stored line that can't be read is `Err` and an Error on the Security
  channel, without the line in it.  Nothing here ever logs a password.

The benchmark, `cargo test -p conductor-tools --release argon2_cost -- --ignored --nocapture`, times memory
settings at one and two passes three ways: fresh memory each hash, the kept arena, and the arena with the
huge page hint.  On Jacob's machine, 2026-09-29, the middle of 5, one lane:

| Memory  | Passes | Fresh memory | Kept arena | Arena + huge pages |
|---------|--------|--------------|------------|--------------------|
| 19 MiB  | 1      | 9.0 ms       | 7.8 ms     | 7.7 ms             |
| 64 MiB  | 1      | 37.5 ms      | 29.7 ms    | 30.0 ms            |
| 64 MiB  | 2      | 69.8 ms      | 63.8 ms    | 62.2 ms            |
| 128 MiB | 1      | 75.4 ms      | 62.0 ms    | 62.8 ms            |
| 256 MiB | 1      | 156.9 ms     | 127.3 ms   | 127.9 ms           |

The kept arena saves about 20% at every size (the page faults); time is linear in memory; one pass is half
of two.  The huge page column equals the arena column because the dev kernel is on `[always]` and the arena
had huge pages before we asked.  Unoptimized, every number is six times bigger, hence `--release`.

What's open:

- Rayon lanes would cut a single hash's wall time across cores at the same CPU cost, but it's a crate and
  its threads bypass `threads::spawn()`.  Not taken.
- Not hashing at all on a reconnect (a token from Fingerprinter instead) is the biggest CPU saving there is.
  It belongs with reconnect tokens, under client management in TODO.md.
- Whether the huge pages actually landed only the kernel knows; reading that is in TODO.md.
- Passwords sit in ordinary `String`s while they're in line and aren't wiped after (the crate's `zeroize`
  feature is off).  Fine for a hobby server; written down so it's a choice, not an oversight.

## Notices

What the admin has to see and acknowledge, in `notices.rs`.

- **Every Warn and Error Scribe logs becomes one**, and code can raise one on purpose with
  `notices::publish(Level::Notice, source, text)`.
- **A notice stays until it's ACKed**, one at a time or ACK ALL, and then it's gone.  Looking at it doesn't
  count.
- **Memory only, since Conductor started.**  Jacob picked this over a journal file or a Postgres table: a
  restart wipes them.  (A table couldn't hold "the database dropped" anyway.)  **Capped at 1,000 open**
  (`MOST_OPEN`, 2026-10-02): past that the oldest go, and the oldest one left is rewritten to say how many
  have gone that way, so a Warn that keeps coming all weekend can't eat the memory or make the History tab
  copy thousands of lines a second.  The log has them all.
- Nothing in here writes to Scribe; Scribe calls in here before taking its own lock.  The bell, its tray and
  the Notifications History tab are in `conductor-wgui.md`.

## The server's switch

`server.rs`, behind the Server tab.

- **Conductor and the server are two things.**  The program is DiskMan, Scribe, Constellations and the web
  admin.  The server is the rest, and it's off until START SERVER.  This file only holds the state and the
  mailbox; the launcher knows what the pieces are (see `conductor-launcher.md`).
- **`ask()` flips the state itself**, under its lock, before posting the command: stopped becomes starting,
  running becomes stopping.  An ask that doesn't fit (start while running, stop while stopped, anything
  while starting or stopping) comes back `Err` with the state it found and changes nothing, so two clicks in
  a row can't both get through.
- The mailbox is one `mpsc` channel made on first use, the launcher's end behind a Mutex.  `next_command()`
  waits with a time limit so the launcher can look up between asks and see whether the web admin is still
  there.  Nothing in here writes to Scribe; the web admin and the launcher say what happened.
- The server pieces in this crate stop and start again in the same run: Archivist reads `postgres.cfg`
  again on every start (a start while running is a Warn that does nothing), Security's arena comes and goes
  with each START and STOP, and Fingerprinter's `stop()` only tells the Services tab.

## Scribe

- One file per UTC day, `Content/logs/2026_09_28.scribe.log`, appended to, rolling at midnight UTC.  Year
  first so the folder sorts by date.
- The line is Jacob's layout:
  `[ 02:16:43 PM - 09-28-26 Z ] - [ System / Info ] - [ message ] [ Caller: file, Line: n ]`.
  The caller comes from `#[track_caller]`, so nobody passes a file and line by hand.
- An error value goes in front of the message (`error_with()`), the way `ex.Message` did in C#.  Rust has
  no exceptions, so it takes anything that prints.
- **Scribe doesn't touch the disk.**  Each line is handed to DiskMan as an append, still under Scribe's
  lock so the order holds, and DiskMan decides when it goes out.  A file that won't write is DiskMan's to
  report: one note on stderr, the Services tab, and its usual retries.
- **Every line goes three places: the file, the terminal, and the last 200 in memory**, and a Warn or an
  Error also becomes a notice.  The terminal is the launcher's console, which is only Scribe's output; its
  write ignores a failure instead of using `println!`, which panics when stdout is gone.  The 200 are what
  the Log tab shows, numbered from 1 so the page can ask for the ones after the last it saw.
- Scribe never panics and never hands an error back from a log call.  A log that takes the server down is
  worse than no log.
- Scribe starts after DiskMan and before Constellations, on the default folder, so the config's complaints
  have somewhere to go.  `move_to()` follows the config once it's loaded.

Open: the few lines before `move_to()` stay in the default folder if the config points elsewhere; no size
limit, and no switch for Debug lines yet.  All in TODO.md.

## Constellations

The one owner of every config file, and what the Settings tab is drawn from.

- **One format, one reader, one table.**  Every config file is plain `key = value` lines with `#` comments,
  keys in any case, the later of two wins (TOML would have cost a crate to read a handful of lines).  Every
  file is written down once in `constellations/files.rs`: its name, its reboot, its channel, its comment,
  and every setting with a kind (text, secret, folder, port, a number with a range), a default and the
  comment above it.  `text.rs` reads and writes any file from that table, and the Settings tab draws every
  card from it (`/Opus/settings`), so a new file or setting needs no page work.  A new setting is one entry
  and a line wherever it's read; a new file is one entry, a place in `FILES`, and a `load()` where its piece
  starts.  `file_named(name)` finds a file by name, for the routes.
- **Every file lives in `Content/cfg/`** and is soft or hard as a whole, never a mix.  Soft: the server
  pieces read it on every START SERVER, so STOP SERVER and START SERVER is the reboot.  Hard: read at boot,
  so Conductor is shut down and run again.  A piece that needs both kinds gets two files.  Jacob's rule:
  anything about Constellations or Scribe is hard, and files are kept separate rather than one file with
  sections.
- **`load(file)`** reads the file (writes it with the defaults if it's missing, appends any setting it
  lacks, logs the lines it can't use as Warns) and keeps the values, behind a lock, by the file's name.
  `value()`, `number()`, `port()` and `folder()` read one; a file that hasn't been loaded reads as its
  defaults, which is what lets Scribe start on the default folder before the file is read.
- **A change from the web admin never touches the live file.**  `save_waiting(file, text)` checks every
  line first (a wrong one means nothing is written and the complaints go back to the page), writes the
  whole file, comments and all, to `name.cfg.wait4server`, and asks DiskMan to swap it in when the reboot
  comes: the server stopping for a soft file, Conductor's shutdown for a hard one.  Jacob's design: the live
  file always says what Conductor is running on.  `waiting(file)` reads what's waiting, for the page;
  `discard_waiting(file)` throws it away.  `server_stopped()` is the launcher's call from `stop_server()`,
  and waits on the soft swaps so the next START SERVER reads the new files.  `file_values(file)` reads a
  file as it sits on disk, so the Settings tab shows one not loaded this run (`postgres.cfg` before the
  first START SERVER) as the next start will read it.
- **A leftover `.wait4server` is applied at the next load** (a crash before the swap, or a save while the
  server was already stopped).  The admin wanted it either way.
- **Nothing in here stops the server.**  A file that can't be read is an Error and we run on the defaults,
  without writing over it.  A `.wait4server` that can't be swapped in is a Warn and the old file stands.
- `Content/` is found through `OPUS_CONTENT`, then by walking up from the working directory, then
  `./Content`.  No drive path is ever hardcoded.  A relative folder in a file is taken from `Content/`.
- A complaint never echoes a `Secret`'s value, and a line that isn't `key = value` is never echoed at all,
  since it might be the password line with the `=` forgotten.  The password can still be shown on the page
  (Jacob, 2026-09-29: Postgres only listens on this machine), but it never reaches the log.
- A hand edit to a live file is read at the next START SERVER (soft) or the next run (hard), since DiskMan
  notices it.

The files today, with their defaults:

- **`conductor_globals.cfg`**, hard, loaded by the launcher at boot: `scribe_log_dir` (`logs`), `wgui_port`
  (`9996`; moving to `wgui.cfg`, in TODO.md).
- **`wgui.cfg`**, hard, loaded by the launcher at boot, read by the web admin's login: `user_password`
  (`user`), `admin_password` (`admin`).  Kept as they are, not hashed (Jacob, 2026-09-29): Security only
  runs while the server does, and a login has to work before START SERVER.  `Text`, not `Secret`, so an
  empty one is refused; a `Text` complaint never echoes the value either.
- **`postgres.cfg`**, soft, loaded by Archivist's `start()`: `address` (`localhost`), `port` (`5432`),
  `database` (`opusdb`), `username` (`opus_game`), `password` (secret, empty), `query_time_limit_seconds`
  (0 to 3600, 10), `slow_job_ms` (1 to 600000, 250).
- **`networking.cfg`**, soft, loaded by networking's start: `bind_address` (`0.0.0.0`), `tcp_port`
  (`9997`), `udp_port` (`9998`), `certificate_file` (`certs/conductor.crt`), `private_key_file`
  (`certs/conductor.key`), `secret_word` (`potato`), `client_versions` (`0.0.1`, a comma list),
  `login_deadline_seconds` (1 to 600, 10), `login_threads` (1 to 256, 8), `max_waiting_logins` (1 to 10000,
  64), `token_deadline_seconds` (1 to 600, 30), `udp_timeout_seconds` (1 to 3600, 40), `access_list` (`off`,
  `whitelist` or `blacklist`), `whitelist_file` (`cfg/whitelist.cfg`), `blacklist_file`
  (`cfg/blacklist.cfg`).
- **`game.cfg`**, soft, loaded by GameWorld's start: `view_chunks` (1 to 16, 4).  Soft because "the world
  should need a reboot so the voxel engine or service restarts and rebuilds" (Jacob, 2026-09-30).

## Archivist

The database, named by Jacob.

- **One worker thread, one connection, jobs in order.**  `execute()`, `query()`, `batch()` and
  `transaction()` put a job in the mailbox and hand back a `Pending` straight away.  `check()` never waits
  (the game loop uses that one), `wait()` does (startup and admin use).  One worker, because two can finish
  jobs out of order, and a SELECT could miss the UPDATE sent just before it.  No async (tokio): the game
  already doesn't wait, and async would only add machinery.
- The worker keeps every statement it has prepared (up to 500), so a query sent over and over is planned by
  Postgres once.  The list is emptied on reconnect and after any `batch()`.
- Values always go in as `$1`, `$2` params, never pasted into the SQL.  `batch()` is for our own SQL only.
- `transaction(name, |tx| ...)` runs a closure on the worker inside a transaction, so one step can use the
  last one's result (insert an account, get its id, use it).  All of it commits or none of it does.  The
  rule: never wait on anything else inside it.
- `postgres.cfg` is loaded again by every `start()`, and Archivist takes the typed view (`DbSettings`).
  `query_time_limit_seconds` goes to Postgres as `statement_timeout`.  A missing file is written with an
  empty password and a capitals Error.  The file is committed on purpose: the password is a placeholder and
  Postgres only listens on localhost.  The password never reaches the log: `DbSettings` has a hand-written
  Debug, and a broken line isn't echoed.
- On every connect, before any job: the schemas in `Content/psql/defaults/schemas/` (only `CREATE ... IF NOT
  EXISTS`, baked in with `include_str!` and written back out if missing), then the migrations in
  `Content/psql/migrations/`, all read and written through DiskMan.
- **A schema file is frozen once its table exists.**  Every change after that is a migration named
  `0001_what_it_does.sql`, run once, in number order, in a transaction with its row in
  `archivist_migrations`.  A failure stops the rest.  A badly named file or two with one number stops them
  all.  Migrations skip the query time limit.
- A job that runs at least `slow_job_ms` is a Debug line with its time, its wait in the mailbox, and the
  first 80 characters of its SQL (never the values).  It was a Warn, and so a notice to ACK, until
  2026-10-02: the world save is over the limit as a matter of course, so it was one every save.  `status()` keeps running, connected, waiting, jobs done, slow
  jobs, the slowest and the last 5 slow ones, and counts `query()` jobs as reads, `execute()` as writes,
  and `batch()` and `transaction()` as other.  The web admin shows all of it.
- Archivist connects the moment it starts, so the log says straight away whether Postgres is there (with
  its `SELECT version()`).  If Postgres is down, jobs come back `NotConnected`, the first failure is logged
  once, and it tries again on a later job, no more than once every 5 seconds.  Nothing in Archivist stops
  the server.

Tables today:

| Table                  | Made by                         | What it is                                              |
|------------------------|---------------------------------|---------------------------------------------------------|
| `accounts`             | `schemas/accounts.sql`          | One row per account.  Columns below.                    |
| `player_characters`    | `schemas/player_characters.sql` | One row per character.  Columns below.                  |
| `archivist_migrations` | Archivist itself                | Which migrations have run, and when.  `uuid` from 0001. |

`archivist_migrations` is keyed by the migration's number and has no `id`: it's Archivist's bookkeeping, not
game data, and CLAUDE.md makes it the one exception to "`id` and `uuid` on everything" (Jacob, 2026-09-30).

`accounts`: `id`, `uuid` (migration 0001), `account_username` (8 to 32 of `a-z`, `0-9`, `_`, unique),
`owner_first_name` and `owner_last_name` (as the owner capitalizes them), `owner_email` (loosely checked,
one account per address ignoring case), `password_hash` (Security's PHC string), `created_at`,
`last_login_datetime` (empty until the first login; the moment the player last came in over UDP, written
by conductor-accounts).  Postgres checks the name and email itself, so even a bug in Conductor can't store
a bad one.  Migration 0002 adds `character_slot_1` to `character_slot_3`, each a `player_characters` `id` or
empty (`ON DELETE SET NULL`).

`player_characters`: `id`, `uuid`, `account_id` (`ON DELETE CASCADE`, so an account's characters go with it),
`character_name` (4 to 20 letters, a to z, only the first a capital; unique across the server ignoring
case), `position_x`, `position_y`,
`position_z` (`REAL`, where it last stood), `save_lua` (the whole GameObject as primlib's Lua text),
`created_at`, `saved_at`.  The name and position are columns so character select and the spawn never run
Lua.  Only conductor-accounts writes it.

What's open:

- Most of its log lines are Info and should be Debug (see TODO.md).
- A password with a space at either end loses it, since every value is trimmed.
- It only reconnects when a job comes; reconnecting on its own is in TODO.md.

## Threads

The list of threads our code started, in `threads.rs`.

- **Every thread goes through `threads::spawn(name, work)`**, never `std::thread::spawn`.  It records the
  name, the file and line that called it (`#[track_caller]`, the same trick Scribe uses), and when.
- The thread writes down the OS's own number for itself as its first act: `/proc/thread-self` on Linux,
  `GetCurrentThreadId` on Windows, nothing elsewhere.  conductor-monitor matches that number to what the OS
  reports, which is how a thread in the "in use" view gets our name and an "ours" mark.
- A thread is marked finished when its closure ends, a panic included (a guard that's dropped either way).
  Finished threads stay on the list; there are a handful, not thousands.
- Threads today: `main`, `diskman`, `security`, `archivist`, `account-desk`, `lua`, `gameworld`,
  `gameclock`, `monitor`, `net-tcp`, `net-login-1` and up, `net-dns`, `net-udp`, `wgui`.  The postgres
  crate starts some of its own, and those show up as "not ours".
- main can't be started by `spawn()`, so it puts itself on the list with `name_this_thread("main")` as the
  first line of `main()`.  It stays "running" for good, since main ending ends Conductor.

## Services

The list of services Conductor expects, in `services.rs`, and how each says it's doing: the Services tab,
Zabbix style.

- Nothing can look into a service from outside and tell whether it's alive, so **each one reports on
  itself**: `services::set(name, state, note)`, with a note that says what it's doing or what went wrong.
- **Every expected service is on the list from the start**, as "expected", so one that never started shows
  as missing.  `EXPECTED` has DiskMan, Scribe, Constellations, Fingerprinter, Security, Archivist, Network
  (TCP), Network (UDP), Account desk, Monitor, Lua, GameWorld, GameClock and Web admin, each with its
  thread's name if it has one.  A new service goes in `EXPECTED` and in the test
  `every_expected_service_is_there_from_the_start`, or `cargo test` fails.
- A service with a thread is **stopped once that thread has ended**, whatever it last said (a thread that
  panics says nothing on the way out).  This is worked out when the list is read, from `threads::list()`.
- A service can **check in** with `seen(name)`.  One that has checked in and then goes quiet for more than
  `QUIET_LIMIT` (5 seconds) isn't healthy.  The monitor, DiskMan, Security, Lua, GameWorld, the GameClock and
  the UDP side do; the others have no loop to check in from.  Healthy means running and not gone quiet; the
  page shows starting as yellow, not down.
- **Nothing in `services.rs` writes to Scribe.**  Scribe reports to the list, so a call the other way could
  leave each waiting on the other's lock.  Who reports what is the table in `conductor-wgui.md`.

## The clock

The standard library stops at seconds since 1970, so the calendar is worked out by hand in `clock.rs`.  The
tests pin it against dates we know: the epoch, a leap day in 2000, and the last second before midnight on
2026-09-28 rolling into the 29th.
