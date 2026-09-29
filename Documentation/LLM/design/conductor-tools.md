<!--
File:       Opus/Documentation/LLM/design/conductor-tools.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-tools

A lib crate.  The pieces the rest of Conductor leans on but that know nothing about the game: the disk, the
log, the config, the UUIDs, the password hashing, the database, the clock, the list of threads we started,
the list of services we expect, the notices the admin has to acknowledge, and the switch that starts and
stops the server.  Two dependencies: `postgres` (the blocking Postgres client) for Archivist, which pulls
in tokio behind the scenes though nothing of ours is async, and `argon2` for Security, with its default
features off.

## Skeleton

```
conductor-tools/
├── Cargo.toml
└── src/
    ├── lib.rs             pub mod archivist; clock; constellations; diskman; fingerprinter; notices; pending;
    │                        scribe; security; server; services; threads;
    ├── archivist.rs       the front door: start(), stop(), status(), config_path()
    │                        execute(sql, params) -> Pending<u64>, query(sql, params) -> Pending<Vec<Row>>
    │                        batch(sql) -> Pending<()>, transaction(name, |tx| ...) -> Pending<T>
    │                        type Pending<T> (from pending.rs), enum ArchivistError, type Param
    ├── archivist/
    │   ├── settings.rs    postgres.cfg as Archivist reads it: struct DbSettings, load() (via Constellations)
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
    │                        swap(original, replacement, SwapAt) -> Pending<()>, run_swaps(SwapAt),
    │                        forget_swap(original), remove(path) -> Pending<()>
    │                        type Contents = Arc<Vec<u8>>, enum DiskError, enum Piece, enum SwapAt, struct Status
    ├── diskman/
    │   ├── cache.rs       struct State, struct Entry (whole file, dirty, tail), the swap list, the rules; Limits
    │   └── worker.rs      the one worker thread: reads, writes, removes, swaps, the big write, streams
    ├── fingerprinter.rs   start(), new_uuid() -> io::Result<String>, new_token(), random_bytes(buffer)
    │                        looks_like_uuid(text), uuid_time(text) -> Option<Utc>
    ├── fingerprinter/     fill(bytes) per OS: linux.rs (getrandom), windows.rs (BCryptGenRandom), other.rs
    ├── security.rs        start(), stop()
    │                        hash_password(password) -> Ticket<String>
    │                        verify_password(password, stored) -> Ticket<bool>
    │                        verify_no_account(password) -> Ticket<()>
    │                        Ticket<T> { check(), wait_for(wait), wait(), place() -> Place { ahead, wait } }
    │                        check_password_rules(password) -> Result<(), String>, pad_login_time(started)
    │                        type Pending<T> (from pending.rs), enum SecurityError { NotRunning, Failed }
    ├── security/          advise_huge_pages(start, bytes) per OS: linux.rs (madvise), windows.rs, other.rs
    ├── notices.rs         enum Level { Notice, Warn, Error }, struct Notice { id, when, level, source, text }
    │                        publish(level, source, text) -> id, newest(n), all(), ack(id), ack_all()
    ├── pending.rs         Pending<T, E> { check() never waits, wait_for(wait) waits so long, wait() does }
    │                        trait NotRunning
    ├── constellations.rs  the store: load(file), value / number / port / folder (file, key), values(file)
    │                        settings() -> Settings { scribe_log_dir, wgui_port }, log_dir(), content_dir()
    │                        path_of(file), waiting_path(file), config_path()
    │                        save_waiting(file, text), waiting(file), discard_waiting(file), server_stopped()
    ├── constellations/
    │   ├── files.rs       the table: enum Reboot { Soft, Hard }, enum Kind, struct Setting, struct ConfigFile
    │                        GLOBALS (conductor_globals.cfg, hard), WGUI (wgui.cfg, hard),
    │                        POSTGRES (postgres.cfg, soft), NETWORKING (networking.cfg, soft), FILES
    │   └── text.rs        the one reader and writer: parse(file, text) -> Parsed, check(setting, value)
    │                        file_text(file, values), missing_text(file, seen), defaults(file); type Values
    ├── server.rs          enum State { Stopped, Starting, Running, Stopping }, enum Command { Start, Stop, Restart }
    │                        ask(command) -> Result<(), State>, next_command(wait) -> Option<Command>
    │                        set(state, note), status() -> Status { state, note, since }
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
- **Swaps and removes** (2026-09-29, for the config editor).  `swap(original, replacement, when)` renames
  one file over another: now, when the launcher says the server has stopped (`run_swaps(ServerStop)`,
  which it waits on), or as DiskMan's last act at shutdown, once everything else is written.  DiskMan
  holds the list, Jacob's design: a change saved from the web admin sits in `name.cfg.wait4server` until
  its reboot, and DiskMan puts it in place on the way down.  A swap only runs once both files have
  nothing waiting to go out, and it drops whatever was held for them, so the next read goes to the disk.
  A replacement that isn't there any more (discarded) is nothing to do.  At shutdown every swap still in
  the list runs, the server-stop ones included: a hard reboot applies the lot.  A second swap for the
  same original replaces the first.  `remove(path)` deletes a file and drops what was held for it.
  Neither logs when it works; a failed swap is a capitals Error, and the old file stands.

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

The password hasher, in `security.rs`.  Built 2026-09-29, modelled on Stratum's, with the emphasis this
time on spending less CPU per hash and more RAM where that buys the same protection.

What we decided:

- **Argon2id, through the `argon2` crate** (0.6, default features off).  Nobody writes their own password
  hash.  What's stored is the PHC string, `$argon2id$v=19$m=65536,t=1,p=1$<salt>$<hash>`, which carries its
  own settings, so an old account still checks after the settings change.
- **The settings: 64 MiB, one pass, one lane, a 32-byte hash, a 16-byte salt.**  Argon2's CPU time is
  close to memory times passes, and what an attacker's graphics card is short of is memory, so the trade is
  passes down to one and memory as high as we can spare.  To make a hash harder later, raise the memory.
  Jacob picked 64 MiB at one pass over 128 (62 ms) and 256 (127 ms) on the numbers below: 30 ms a login.
- **One worker thread, `security`, one hash at a time**, jobs in order, `Pending` back to the caller the
  same as Archivist and DiskMan.  Stratum's tick-sim showed 50 logins hashing on 50 threads took 1.2 s each
  and blew the tick; one at a time, each took 72 ms and all 50 were done in 3.6 s.  **Jacob's rule: one
  login is hashed at a time, hard limit, and every other client waits in the queue** in the order it
  arrived.  Accounts and networking build on that line, not around it.
- **A `Ticket` says where a job stands.**  Every job gets a number under the worker's lock, the worker
  counts what it has finished and keeps a running average of one job (starts at the benchmark's 30 ms, then
  an eighth of each new time), and `place()` turns the three into "N ahead, about M ms".  Jacob asked for
  both so a waiting client can be told; the telling is networking's, later.
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
- The benchmark, `cargo test -p conductor-tools --release argon2_cost -- --ignored --nocapture`, times five memory
  settings at one and two passes three ways: fresh memory each hash, the kept arena, and the arena with the
  huge page hint.  The gaps between the columns are the page faults and the TLB, measured.

The numbers, from the benchmark on Jacob's machine on 2026-09-29 (`--release`, the middle of 5, one lane):

| Memory  | Passes | Fresh memory | Kept arena | Arena + huge pages |
|---------|--------|--------------|------------|--------------------|
| 19 MiB  | 1      | 9.0 ms       | 7.8 ms     | 7.7 ms             |
| 64 MiB  | 1      | 37.5 ms      | 29.7 ms    | 30.0 ms            |
| 64 MiB  | 2      | 69.8 ms      | 63.8 ms    | 62.2 ms            |
| 128 MiB | 1      | 75.4 ms      | 62.0 ms    | 62.8 ms            |
| 256 MiB | 1      | 156.9 ms     | 127.3 ms   | 127.9 ms           |

What it says: the kept arena saves about 20% at every size (the page faults); time is linear in memory;
one pass is half of two.  The huge page column equals the arena column because the dev kernel is on
`[always]` and the arena had huge pages before we asked.  Unoptimized (`cargo test` without `--release`)
every number is six times bigger, which is why the benchmark says to use `--release`.

What's open:

- The queue place is told to the client now: networking's login thread waits on the `Ticket` a second at
  a time with `wait_for()` (2026-09-29) and sends an InLine with `place()`'s numbers between waits.
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

## The server's switch

`server.rs`, built 2026-09-29 for the web admin's Control Panel.

What we decided:

- **Conductor and the server are two things.**  The program is DiskMan, Scribe, Constellations and the web
  admin.  The server is the rest, and it's off until START SERVER.  This file only holds the state and the
  mailbox; the launcher knows what the pieces are (see `conductor-launcher.md`).
- **`ask()` flips the state itself**, under its lock, before posting the command: stopped becomes starting,
  running becomes stopping.  An ask that doesn't fit (start while running, stop while stopped, anything
  while starting or stopping) comes back `Err` with the state it found, and changes nothing.  So two clicks
  in a row can't both get through, without the launcher having to sort that out.
- The mailbox is one `mpsc` channel made on first use, the launcher's end behind a Mutex.  `next_command()`
  waits with a time limit so the launcher can look up between asks and see whether the web admin is still
  there.
- Nothing in here writes to Scribe.  The web admin and the launcher say what happened in their own words.
- The one test walks the whole switch in order, since the switch is one for the whole program.

Since Archivist and the monitor can now stop and start again: Archivist's settings ride with its worker
onto the thread (they were in a write-once `OnceLock`) and `postgres.cfg` is read again on every start; a
start while running is a Warn and does nothing, for both.  The monitor's stop drops its last look.
Fingerprinter has a `stop()` that only tells the Services tab.  Security could already stop and start
again (its `stop()` takes the worker out), so it went in as it was; its 64 MiB arena is allotted on each
START SERVER and let go on each STOP.

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

Rebuilt on 2026-09-29 as the one owner of every config file, for the config editor that's coming to the
web admin.  Before that it was `conductor_globals.cfg` alone, loaded once into a `OnceLock`, and Archivist
had a reader of its own for `postgres.cfg` that was nearly the same code.

What we decided:

- **One format, one reader, one table.**  Every config file is plain `key = value` lines with `#` comments,
  keys in any case, the later of two wins (TOML would have cost a crate to read a handful of lines).  Every
  file is written down once in `constellations/files.rs`: its name, its reboot, its channel, its comment,
  and every setting with a kind (text, secret, folder, port, a number with a range), a default and the
  comment above it.  `text.rs` reads and writes any file from that table, and the web admin's Settings tab
  draws every card from it (`/Opus/settings`), so a new file or setting shows up there with no page work.
  Adding a setting is one entry in the table and a line wherever it's read; adding a file is one entry and
  a `load()` where its piece starts.  `file_named(name)` finds a file by its name, for the routes.
- **Every file lives in `Content/cfg/`** and is soft or hard as a whole, never a mix.  Soft
  (`postgres.cfg`): the server pieces read it on every START SERVER, so STOP SERVER and START SERVER is
  the reboot.  Hard (`conductor_globals.cfg`: Constellations, Scribe and the web admin's port; `wgui.cfg`:
  the web admin's accounts): read at boot, so Conductor is shut down and run again.  A piece that needs both kinds gets two files.  Jacob's rule:
  anything about Constellations or Scribe is hard, and files are kept separate rather than one file with
  sections.  `scribe_log_dir` was going to hot swap through `scribe::move_to()`; Jacob made it hard.
- **The store is a lock, not a `OnceLock`.**  `load(file)` reads the file (writes it with the defaults if
  it's missing, appends any setting it lacks with the default, logs the lines it can't use as Warns) and
  keeps the values by the file's name.  `value()`, `number()`, `port()` and `folder()` read one; a file
  that hasn't been loaded reads as its defaults, which is what lets Scribe start on the default folder
  before the file is read.  The launcher loads `GLOBALS` at boot; Archivist's `start()` loads `POSTGRES`,
  so every START SERVER reads it again.
- **A change from the web admin never touches the live file.**  `save_waiting(file, text)` checks every
  line first (a wrong one means nothing is written and the complaints come back for the page), writes the
  whole file, comments and all, to `name.cfg.wait4server` beside the live one, and asks DiskMan to rename
  it over the live file when the reboot comes: the server stopping for a soft file, Conductor's shutdown
  for a hard one.  Jacob's design: the live file always says what Conductor is running on, and DiskMan
  holds the list and does the swap on the way down.  `waiting(file)` reads what's waiting, for the page;
  `discard_waiting(file)` throws it away.  `server_stopped()` is the launcher's call from `stop_server()`,
  and it waits on the soft swaps so the next START SERVER reads the new files.  `file_values(file)` reads
  a file as it sits on disk, for the Settings tab to show one that hasn't been loaded this run
  (`postgres.cfg` before the first START SERVER) as the file says, since that is what the next start
  reads.
- **A leftover `.wait4server` is applied at the next load.**  If Conductor crashed before the swap, or the
  change was saved while the server was already stopped, `load()` finds the waiting file and swaps it in
  before reading.  The admin wanted it either way.
- **Nothing in here stops the server.**  A file that can't be read is an Error and we run on the defaults,
  without writing over it.  A `.wait4server` that can't be swapped in is a Warn and the old file stands.
- `Content/` is found through `OPUS_CONTENT`, then by walking up from the working directory, then
  `./Content`.  No drive path is ever hardcoded, so a fresh checkout anywhere runs.  A relative folder in
  a file is taken from `Content/`.
- A complaint never echoes a `Secret`'s value, and a line that isn't `key = value` is never echoed at all,
  since it might be the password line with the `=` forgotten.  The password can still be shown on the page
  (Jacob, 2026-09-29: Postgres only listens on this machine), but it never reaches the log.

The files today:

| File                     | Reboot | Read by     | Settings                                                   |
|--------------------------|--------|-------------|------------------------------------------------------------|
| `conductor_globals.cfg`  | hard   | the launcher at boot | `scribe_log_dir` (`logs`), `wgui_port` (`9996`)   |
| `wgui.cfg`               | hard   | the launcher at boot; the web admin's login reads the values | `user_password` (`user`), `admin_password` (`admin`), both text, neither empty |
| `postgres.cfg`           | soft   | Archivist on START SERVER | `address`, `port`, `database`, `username`, `password` (secret), `query_time_limit_seconds` (0 to 3600, 10), `slow_job_ms` (1 to 600000, 250) |
| `networking.cfg`         | soft   | networking on START SERVER | `bind_address` (`0.0.0.0`), `tcp_port` (`9997`), `udp_port` (`9998`), `certificate_file` (`certs/conductor.crt`), `private_key_file` (`certs/conductor.key`), `secret_word` (`potato`), `client_versions` (`0.0.1`, a comma list), `login_deadline_seconds` (1 to 600, 10), `login_threads` (1 to 256, 8), `max_waiting_logins` (1 to 10000, 64), `token_deadline_seconds` (1 to 600, 30), `udp_timeout_seconds` (1 to 3600, 40), `access_list` (`off`, or `whitelist` or `blacklist`), `whitelist_file` (`cfg/whitelist.cfg`), `blacklist_file` (`cfg/blacklist.cfg`) |

The two passwords in `wgui.cfg` are kept as they are, not hashed (Jacob, 2026-09-29): Security only runs
while the server does, and a login has to work before START SERVER.  They're `Text`, not `Secret`, so an
empty one is refused; a `Text` complaint says the key is empty and never echoes the value, so nothing is
lost by it.

What's open:

- Hand edits to a live file while Conductor holds it aren't seen (DiskMan serves the copy in memory).
  Today that only matters for `postgres.cfg` between one START SERVER and the next.

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
  `slow_job_ms` (250).  Since 2026-09-29 the file is Constellations' (`POSTGRES` in
  `constellations/files.rs`, a soft file): Archivist's `start()` has it loaded again and takes the typed
  view (`DbSettings`).  A missing file is written with an empty password and a capitals Error.  A setting
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
- Threads today: `main`, `diskman`, `security`, `archivist`, `net-tcp`, `net-login-1` and up, `net-dns`,
  `net-udp`, `monitor`, `wgui`.  The postgres crate starts some of its own, and those show up as "not ours".
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
  Security, Archivist, Network (TCP), Network (UDP), Monitor, Web admin, each with the name of its thread
  if it has one.  Adding a service means adding it there.
- A service with a thread is **stopped once that thread has ended**, whatever it last said.  A thread that
  panics says nothing on the way out.  This is worked out when the list is read, from `threads::list()`.
- A service can **check in** with `seen(name)`.  One that has checked in and then goes quiet for more than
  `QUIET_LIMIT` (5 seconds) isn't healthy.  The monitor, DiskMan, Security and the UDP side do; the others
  have no loop to check in from.
- Healthy means running and not gone quiet.  The page shows starting as yellow, not down.
- **Nothing in `services.rs` writes to Scribe.**  Scribe reports to the list, so a call the other way could
  leave each waiting on the other's lock.
- Who reports what is the table in `conductor-wgui.md`.

## The clock

The standard library stops at seconds since 1970, so the calendar is worked out by hand in `clock.rs`.  The
tests pin it against dates we know: the epoch, a leap day in 2000, and the last second before midnight on
2026-09-28 rolling into the 29th.
