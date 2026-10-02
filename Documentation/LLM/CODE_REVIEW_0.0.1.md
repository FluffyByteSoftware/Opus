<!--
File:       Opus/Documentation/LLM/CODE_REVIEW_0.0.1.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Conductor code review for 0.0.1 (2026-10-02)

Every `.rs` file in Conductor's eleven crates read in full (27,000 lines, 88 files), plus `page.html`, the
SQL and the protocol documents, against the rules in CLAUDE.md.  The ask: "evaluate for any flaws, or
techniques that are inefficient".  Each finding below was checked against the surrounding code before it
went in; the ones under "Checked and fine" were looked at and found right, so nobody re-checks them.

The short version: the code is in good shape.  No lint suppressions, every file has its header, every
packet parser is safe on any bytes, locks are short and never nested across modules, no deadlocks found,
every thread goes through `threads::spawn`, `std::fs` only ever touches folders (and `/proc`).  Four real
bugs, a handful of things that will get noisy or expensive in ordinary running, and a tail of
inefficiencies and stale comments.  Nothing found corrupts the database or crashes Conductor in a release
build.

Each entry is a file and line, what's wrong, when it bites, and the fix.  Severity: **Bug** (wrong
behaviour, reachable), **Risk** (reachable by abuse or by bad luck, or a rule broken), **Inefficiency**,
**Cleanliness** (dead code, stale words).  Line numbers are as of `7d85f1f`.

**The four bugs and the seven risks under "worth fixing before the tag" were fixed the same day**
(2026-10-02; each says so below; all built and tested, 365 tests).  The rest is
Jacob's to pick from, before the tag or after (CODE_REVIEW in TODO.md).

---

## Bugs

**B1. A double login can give one account two players.**  *Fixed 2026-10-02: `issue_in` refuses a ticket
over a player in the world (`Issued::Playing`), and `tcp.rs`'s login goes round again, asking the client once.*
`networking/src/sessions.rs:530-540`, `issue_in()`.  It only clears an old *ticket* for the account; if
`accounts[name]` says `Playing(address)`, the player stays in `players` and the line
`book.accounts.insert(name, Ticket(token))` wipes the entry that pointed at them.  The comment says the
caller checked `playing()` first, but the check (`tcp.rs:672`) and the issue (`tcp.rs:731`) aren't under one
lock.  The realistic window: after LogTheOtherOut, `wait_for_save()` (`tcp.rs:705-716`) sits up to 5 s, and
a third login on the account (or its own earlier unused ticket arriving over UDP) makes it Playing again.
Then the second login's ticket overwrites it; when that ticket is used, `connect_in` finds `Ticket`, not
`Playing`, so the orphan isn't removed.  Two players, one account.  The second can pick the same character:
`lock_for_loading` finds no lock (nobody left), Protogame calls `enter()`, the GameClock refuses the
duplicate spawn ("the one there stands", `gameclock/src/players.rs:205`) but `sessions::entered()` still
writes the character on player 2 and sends CharacterEnteredWorld; when player 2 leaves, their `leave(id)`
despawns and saves player 1's copy from under them.
*Fix:* do the "is it playing" check and the issue in one `with_book`: `issue_in` hands back
`Issued(token)` / `AlreadyPlaying(address)` / `Kicked(address, character_id)`, and `talk()` acts on that.
A book test: `issue_in` on a Playing account must not leave a player that `accounts` no longer points at.

**B2. `region.map` with a corrupt header panics GameWorld's thread.**  *Fixed 2026-10-02: `checked_mul`, and a
test with width and depth at 65,535.*
`gameworld/src/regionmap.rs:231`: `reader.take((width * depth * rows as i32) as usize, ...)` multiplies
three `i32`s read off the disk; `width` and `depth` are `u16 as i32`, so 65,535 × 65,535 × 11 overflows.
In a debug build (what `cargo build` and `cargo run` make, what Jacob runs) that's a panic, "attempt to
multiply with overflow": the thread dies inside `read_world()`, the Services line sits at "Starting", the
GameClock never turns ready, STOP SERVER logs "GameWorld's thread had already died".  In release it wraps,
and most wrapped values happen to be rejected by `take()`, which is luck.  REGION_MAP.md says width and
depth "can go to 65,535", so a legitimate big map reaches it too, not only two flipped bytes.
*Fix:* `let cells = (width as usize).checked_mul(depth as usize).and_then(|n| n.checked_mul(rows as
usize)).ok_or("the grid is too big to be ours")?;` and a test with width = depth = 0xFFFF.

**B3. The same overflow on the heights file.**  *Fixed 2026-10-02, the same way.*
`gameworld/src/heights.rs:64`: `reader.take((width * depth) as usize, "the heights")` on the same two
numbers.  The heights file is the big one on disk (134 MB at `world_size` 16), so a damaged header is the
likelier of the two.  Same fix.  (`heights.rs:98` has the same expression in `make()`, where the numbers
are ours, so that one's fine.)

**B4. Deleting an account on a slow database never kicks its player.**  *Fixed 2026-10-02: the player is taken
out on the late answer too, since the row goes when Archivist gets there.*
`wgui/src/accounts.rs:178-187`: `conductor_networking::terminate(&name)` only runs on `Some(Ok(_))`.  On
`None` (Archivist slower than `DATABASE_WAIT`, 5 s) the route answers 503 "it may still happen", and it
does: the job is in Archivist's mailbox, the row and its characters go a moment later (`ON DELETE
CASCADE`), but nobody calls `terminate()`, so the player keeps playing on a deleted account until they
leave; their leaving save then finds no row (`characters::save()` gives `Ok(0)`), and the Warn in
`gameclock/src/saving.rs` `report()` is the only trace.
*Fix:* kick first and delete second (ask Jacob: the comment at :48-51 says he asked for row-first), or keep
the `Pending` and kick when it lands.

---

## Risks

### Worth fixing before the tag

**R1. The read-only `user` can read every password.**  *Fixed 2026-10-02: a Secret goes out as `""` to `user`.  The web admin's two passwords were `Kind::Text`, not
`Secret` (the review had that wrong), so they're a new `Kind::Password`, a Secret that can't be empty.*
`wgui/src/lib.rs:345-347` serves `/Opus/settings` to any logged-in role, and `json.rs:546-566`
`setting_state()` writes every `Kind::Secret` value as it is (`running`, `waiting`, `default`): `wgui.cfg`'s
`admin_password` and `postgres.cfg`'s `password` among them.  The test at `lib.rs:780-800` pins it.  So
`user` is one GET away from being `admin`.  CLAUDE.md says the login is about roles, not security, and the
page only listens on this machine; still, it makes the role mean nothing.
*Fix:* pass `role` into `json::settings()` and write `""` for a Secret when `!role.can_change()`; the page
already greys the fields for `user`.

**R2. Idle TLS connections tie up the login pool for the price of a connect.**  *Fixed 2026-10-02:
`HANDSHAKE_WAIT` of 3 s, and `MOST_OPEN_PER_ADDRESS` of 4 (`End::TooManyFromOne`).*
`networking/src/tcp.rs:487` (the deadline runs from arrival) and `:554-586` (the handshake runs on a login
thread under the whole `login_deadline`).  Defaults: 8 threads, 64 queued, 10 s.  A client that connects
and sends nothing holds a thread for 10 s; eight of them every 10 s hold every thread, and the 64 behind
them age out Unserved.  About seven zero-byte connects a second keeps every real player out.
*Fix:* a short first-byte deadline for the handshake (2 to 3 s) separate from the login deadline, and a
cap on open connections per address in the acceptor (the `open` map has the peers).

**R3. A spoofable UDP packet costs a disk-written Info line.**  *Fixed 2026-10-02: Debug.*
`networking/src/udp.rs:308`: `scribe::info("Refused a UDP connect from {from}")` for every Connect with an
unknown token; `:290` the same at Debug for a banned address.  Info can't be switched off, and every line
is a DiskMan append.  A flood of 69-byte Connects from spoofed addresses is an unbounded log (the 25-byte
reply itself is fine: smaller than the ask, so no amplification).
*Fix:* Debug, or once per address per minute (a small map like `RECENT_FAILURES`).

**R4. A persistent accept or receive failure floods the bell.**  *Fixed 2026-10-02: said once, and again only
after a success.*
`tcp.rs:446-455` warns "TCP accept failed" and sleeps 100 ms, so an out-of-file-handles listener raises
ten notices a second, each staying until ACKed; `udp.rs:249-254` is the same for the receive.  The
acceptor already does it right for the full queue (`said_full`).
*Fix:* say it once, and again only after a success.

**R5. A DiskMan write that lands during a config swap is dropped, and its `Pending` says "not running".**
*Fixed 2026-10-02: `forget_files()` keeps an entry with something waiting, with a test.*
`tools/src/diskman/worker.rs:271-282` with `cache.rs:453-463` and `:496-499`.  `take_due_swaps()` only takes
a swap whose two files are quiet, but nothing marks them busy afterwards.  Between the take (lock let go)
and `forget_files()` (lock taken again after the rename), a `write()` or `append()` to either path makes a
new dirty entry, and `forget_files()` removes it unconditionally: the bytes and the reply `Sender` go, and
the caller's `Pending::wait()` comes back `DiskError::NotRunning` while DiskMan is running.  The case: STOP
SERVER swapping `networking.cfg` at the instant the admin presses SAVE on the Settings tab again; the
second save is lost and the page shows a 500.
*Fix:* in `forget_files()`, only remove an entry when `!entry.waiting()`, or drop just `content` and
`on_disk` and keep the dirty tail and its waiters.

**R6. Notices grow without bound.**  *Fixed 2026-10-02: `MOST_OPEN` of 1,000, the oldest left rewritten to
say how many went, with a test.*
`tools/src/notices.rs:79-83`: every Warn and Error pushes onto `open` with no cap, and `all()` clones the
whole list for the History tab on every poll.  With R4, or R7, or a Lua script tripping its limits, a
server left over a weekend holds thousands, each poll copying them all.
*Fix:* cap `open` (1,000, say), dropping the oldest and leaving one "N older notices were dropped" notice.

**R7. Archivist Warns for every job over `slow_job_ms`, which the world save will trip routinely.**  *Fixed
2026-10-02: Debug, the count and the last few kept for the page.*
`tools/src/archivist/status.rs` `record()`.  `characters::save_all()` writes every character in one
transaction every `world_save_seconds`; at the 250 ms default a save of a few hundred characters is a
Warn, so a notice to ACK every save.  CLAUDE.md: a Warn is never chatter.
*Fix:* Debug for the line and keep the count for the page, or a separate, higher threshold for the Warn.

### Soon

**R8. Scribe writes to the console under its lock, so a stalled stdout stalls every logging thread,
the GameClock included.**
`tools/src/scribe.rs:275-301`: `writeln!(std::io::stdout(), ...)` runs under `SCRIBE`.  If stdout is a pipe
nobody drains (a service manager, `| less`), or the terminal is paused with Ctrl-S, every `scribe::*` call
blocks on that one write, and the tick loop logs.
*Fix:* take the line number under the lock, then write to stdout and hand the line to DiskMan after
letting it go, or give the console its own thread the way the file has one.

**R9. The Lua time limit can't stop a single C call.**
`lua-parser/src/sandbox.rs:105-114` (the hook), `:121` (`StdLib::STRING` in the sandbox).  The limit is a
count hook, which fires between VM instructions and never inside a C function.  `string.find`, `match`,
`gmatch` and `gsub` are backtracking matchers, so `("a"):rep(2000):find(("a*"):rep(12) .. "b")` runs
for effectively ever inside one C call, under the recursion cap, and the memory limit doesn't help since
nothing is allocated.  That's a script hanging the `lua` thread for the run (STOP SERVER then waits on
`join()` forever, `lib.rs:94`).  The same sandbox runs `read_save()`, so a tampered `player_characters.save`
row hangs Protogame's thread on UserPressPlay, and every Play behind it.  The header's "one gap I know
of" (`pcall`) isn't this one.
*Fix, for the save path:* a save is data and needs no library.  Give `read_save()` its own
`Lua::new_with(StdLib::NONE, ...)` with no `log` or `print`: it closes this for the database path and also
stops a tampered save from ringing the bell 51 times per load (`log.warn` and `log.error` are Warns,
`sandbox.rs:164-174`).  For scripts (Jacob's own files), say so in the header beside the `pcall` gap, or
wrap the four string functions in Rust with a cap on the subject's length.

**R10. The reverse DNS thread's backlog outlives STOP SERVER, and a flood is a resolver call per address
before the verdict.**
`networking/src/dns.rs:160-169`: `work()` loops `while let Ok(address) = queue.recv()`; dropping the
Sender in `stop()` doesn't discard the addresses already queued, so the old thread keeps resolving its
backlog after `stop()` gives up at 1 s.  `start()` (`:89`) clears `NAMES` and spawns a second `net-dns`,
and the old one then writes last run's answers into the new cache.  Also `ledger::arrived()`
(`ledger.rs:295-300`) calls `dns::ask()` for every connection *before* the access verdict (`tcp.rs:391`), so
a blacklisted flood is one serial resolver call per distinct address.
*Fix:* a stopping flag the worker checks per address, and ask for a name only once the verdict is
Allowed.

**R11. A failed login-time stamp is lost, and two comments say otherwise.**
`accounts/src/lib.rs:295-303` says "Archivist logs a write that fails"; `networking/src/sessions.rs:348-354`
drops the `Pending` on that promise.  Archivist doesn't (`tools/src/archivist/worker.rs:270-274` maps the
error into the `Pending` and nothing else; CLAUDE.md says the same).  So a stamp that never lands is never
seen.
*Fix:* delete the false sentence, and either keep the `Pending` and look at it in the UDP sweep (the
GameClock's pattern for its saves) or accept it's lost and say so.

**R12. The client version string is logged whole at Info, with no hold.**
`tcp.rs:765-769`: Outdated logs `login.client_version` with `{:?}`, up to about 4 KB of the client's text
per connection, and Outdated deliberately starts no hold.
*Fix:* log the first 40 characters, and consider the 2-second hold for Outdated too.

**R13. A refused spawn is invisible to the player and to networking.**
`gameclock/src/players.rs:172-177` and `:295-299`: `enter()`'s `Ok` means "note left", and Protogame
(`protogame.rs:388-397`) sends CharacterEnteredWorld on it.  If the GameClock then refuses the spawn
(the row id is already in `in_world`), the client is told it's in the world while the other copy stands.
Only reachable through B1 today; the loading and leaving locks hold on the legitimate path.
*Fix:* a reply channel on `Note::Enter`, or at least a Warn naming the account.

**R14. One stalled request stalls the whole web admin, SHUT DOWN included.**
`wgui/src/lib.rs:184-200` with `http.rs:62-104`: the 2 s `TIME_LIMIT` is per `read()`, not per request, so a
local client sending a byte every 1.9 s holds the one thread as long as it likes.  Separately, the
database routes block the same thread up to `DATABASE_WAIT` (`accounts.rs:51`, `characters.rs:321`),
during which the status poll, STOP SERVER and SHUT DOWN all wait.  Local-only, but the web admin is the
only way to stop Conductor.
*Fix:* an `Instant` at accept and a per-request deadline (5 s), and a shorter wait for the list reads.

**R15. The desk forgets a job still running once more than 50 are in flight.**
`accounts/src/desk.rs:177-186`: `finish()` trims the map by oldest number, and `Working` entries are in the
same map, so a burst of more than `KEEP_RESULTS` jobs drops the oldest unfinished one; the page then gets
404 "doesn't know that job" while the account is still made.  One admin makes it unlikely.
*Fix:* only evict entries whose `progress != Working`.

**R16. A player over UDP is their address and nothing more.**
`udp.rs:279-283` (Goodbye), `:346-351` (PlayerCommand): any host that can put a known player's source
address on a datagram can log them out or chat as them.  The design doc lists a per-packet session id
under "NAT rebinding", which is also the fix.  Noted once; not a 0.0.1 blocker.

**R17. `Chunk::block()` has no range check.**
`gameworld/src/chunk.rs:269-276, 326-328`: `index(x, y, z) = (y*32 + z)*32 + x`, so `block(32, 0, 0)` reads
(0, 0, 1) silently and `block(-1, 0, 0)` panics on the Vec index.  Nothing calls it wrongly today; the first
movement code that converts a world coordinate with `%` instead of `rem_euclid` will get a wrong block,
not an error.
*Fix:* a `debug_assert!` in `index()`: free in release, caught in Jacob's debug runs.

**R18. An out-of-range number in a save becomes an infinite position.**
`primlib/src/save.rs:173-184`: `read_f32` checks `is_finite()` on the f64 and then does `as f32`;
`x = 1e300` is finite as f64 and `inf` as f32.  Self-heals at the next save (written as `0`), but one run
has a character at infinity.  Only a hand- or tamper-edited row gets there.
*Fix:* `&& (number as f32).is_finite()`.

**R19. Small state leaks across STOP SERVER and START SERVER.**
- `tools/src/services.rs:352-372, 389-404`: `last_seen` is never cleared, so a service shows unhealthy for
  up to a second after every START SERVER (Security sets Running before its first `seen()`).  Clear it in
  `set()` on Starting or Stopped.
- `tools/src/archivist/worker.rs:70, 119-128`: `WAITING` is never reset, so a worker that died with jobs
  queued leaves phantom "jobs waiting" on the page for the rest of the run.  `WAITING.store(0)` in
  `start()`.
- `tcp.rs:282-296` with `:799`: a login thread that outlives `STOP_WAIT` inside `password_hash().wait()`
  (no time limit; Protogame uses `wait_for(10 s)`) can reach `sessions::issue()` after `sessions::clear()`,
  and the book isn't wiped at start, so a ticket with last run's `door` number sits in the new run's book
  until the sweep.  `sessions::clear()` from `networking::start()` too, and `wait_for` on the login path.
- `tools/src/diskman/worker.rs:177-193`: after `stop()` the loop keeps taking appends until `all_done()`,
  so a thread logging in a tight loop on the way down keeps DiskMan from ever ending.  A bounded number of
  drain rounds would close it.

**R20. The Debug switch the rules describe doesn't exist.**
CLAUDE.md: "A switch in the config turns Debug lines off".  `GLOBALS` has only `scribe_log_dir` and
`wgui_port`; `scribe.rs:80-81` says everything is written for now, and TODO.md agrees.  Not a code bug; the
rule overstates what's built.  A line in CLAUDE.md until the switch lands.

**R21. `/proc` is read with `std::fs`, against the DiskMan rule, with no written exception.**
`monitor/src/probe/linux.rs:53-60, 81-84, 94-101`.  Going through DiskMan would be wrong here (its
modified-time cache can't see `/proc` change), so the code is right and the rule is silent.  One line in
CLAUDE.md under "Linux and Windows", and one in the file's header.

---

## Inefficiencies

**I1. 82% of the terrain in memory is air, built one block at a time.**  (TODO.md has "an all-air chunk
that costs nothing"; this is the measurement.)  `gameworld/src/build.rs:27-55`, `terrain.rs:403-421`.  Rows
2 to 10 have their bottom at y ≥ 32 and the ground never goes above +5, so every chunk in those nine rows
is all AIR.  At `view_chunks` 4 that's 729 of 891 chunks, 46.7 MB of the 57 MB `Terrain`, and `untouched()`
still runs `layer()` and `set()` 32,768 times per chunk to make them (24 million calls at every START
SERVER).  At 8 it's 2,601 of 3,179 chunks, 166 MB of 203 MB; at 16, 627 MB of 767 MB.
*Fix, either on its own:* (a) in `untouched()`, when `bottom > MOST` return `Chunk::filled(pos,
Block::AIR)` at once; (b) a `Uniform(Block)` / `Blocks(Vec<Block>)` enum for the chunk's storage, promoted
on the first `set()` of a different block, so an air chunk costs a few bytes and the file format stays.

**I2. The status answer, and the page, carry and redraw everything every second whatever tab is open.**
`wgui/src/lib.rs:282-297` builds `conductor_networking::status()` (clones the whole ledger, up to 10,000
connections), every process on the machine and every thread, once a second, for one poll.
`page.html:2001-2016` (`drawTcp`) then clears and rebuilds the TCP table, `:1365-1389` the process table,
`:1474-1495` both thread tables and `:3155` the accounts table, every second, hidden tabs included.
*Fix:* make `connections` and `processes` opt-in query flags the page sets from `currentTab` (it already
does exactly this for `/Opus/threads`), and return early in the draw functions when the tab isn't showing.

**I3. The monitor reads every process's `stat` every second for a list nobody may be looking at.**
`monitor/src/probe/linux.rs:114-133` (and `windows.rs:326-352`, an `OpenProcess` per process).  `probe.rs:166`
already makes other processes' threads on demand "for a list nobody is looking at"; the processes
themselves (hundreds of file reads a second on a desktop) aren't.
*Fix:* read them only while the System tab is open, keeping `/proc/self` once a second.

**I4. DiskMan walks its whole file map three times per loop iteration, and iterates once per log line.**
`tools/src/diskman/worker.rs:143-172` calling `cache.rs:507-514` (`ready_to_flush`), `:659-680`
(`unload_extra`, which sums every clean entry's length every call) and `:694-733` (`status()`, computed
every iteration only to read `files_failing`).  Every Scribe line makes the loop busy; with the clean cache
at 256 MB of 32 KB chunk files that's about 8,000 entries scanned three times per log line.
*Fix:* running counters (`clean_bytes_held`, `dirty_count`, `failing_count`) kept where entries change, a
queue of paths with something to flush, and `unload_extra()` only when the counter is over the limit.

**I5. The UDP broadcast sends under the UDP lock.**  `networking/src/udp.rs:199-209` (`tell_all`) and
`:187-220` (`tell`, `tell_answer`): a chat to N players is N × M `send_to` calls under the `UDP` mutex, so
Protogame's and the TCP side's `tell()` wait behind it.  The socket is already an `Arc`: clone it out,
drop the guard, then send.

**I6. `constellations::value()` clones the whole `Values` map on every call.**
`tools/src/constellations.rs:198-202, 228-230`: `value()` → `values()` → `BTreeMap::clone()` of every key and
value, then `remove(key)`.  Thirteen calls at networking's start, one per web login.  Not hot today, but
it's the obvious accessor and will be reached for from a tick.
*Fix:* lock `LOADED` and `get(file.name).and_then(|v| v.get(key)).cloned()`.

**I7. `threads::THREADS` grows with every soft reboot and is cloned whole twice a second.**
`tools/src/threads.rs:40-43, 111-115`: finished records are kept forever (about 18 per START SERVER), and
`monitor/snapshot.rs:115` and `services::list()` each clone the Vec once a second.
*Fix:* keep the last 50 finished records, or prune ones older than an hour.

**I8. `Save::of()` copies every string twice.**  `primlib/src/save.rs:254-270`: `world.component()` clones
the component (every `String` in it), then `saved()` copies the same strings into `Fields`.  The design
doc names it as where to look first after the measured 3.3 µs per character.  Call the typed getters and
each struct's `saved()` directly.  Not urgent at 25 to 50 players.

**I9. Smaller ones.**
- `networking/src/ledger.rs:329-336`: `snapshot()` takes the DNS `NAMES` lock once per entry, up to 10,000
  times per poll.  One lock around the loop.
- `networking/src/sessions.rs:600, 613`: `finish_ask` and `entered` copy an answer the caller already owns
  as a `Vec<u8>`.  Take it by value.
- `tools/src/archivist/status.rs:501-503, 552-559`: `short_label()` builds the shortened SQL for every job
  and only uses it when slow.  *Done 2026-10-02, with R7.*
- `tools/src/scribe.rs:301`: `format!("{line}\n")` copies the whole line to add one byte.
- `tools/src/diskman/worker.rs:45, 194-198, 569`: the one polling loop in the crate, a 5 ms retry while a
  stream's reader is slower than the disk.  Dormant: `stream()` has no caller outside the crate.
- `networking/src/tcp.rs:281-284` and `dns.rs:114-117`: 10 ms polling loops on `is_finished()` in `stop()`,
  against the crate's own "no thread polls".  A `(Mutex<usize>, Condvar)` of running workers gives a timed
  wait.  `launcher/src/main.rs:222-249` sleeps 50 ms on `diskman::finished()` the same way.
- `gameworld/src/regionmap.rs:231`: `.to_vec()` copies the grid out of DiskMan's `Arc<Vec<u8>>`, which
  DiskMan keeps anyway (2.9 MB at size 16, 11.5 at 32).  `Heights` holds the `Arc` and indexes past the
  header; the same trick is free.

---

## Cleanliness

- `tools/src/security.rs:147-154` and `:522`: the arena is said to live "for as long as Conductor runs" and
  "kept for good"; it's a local in `run()` (`:425`), allotted on START SERVER and freed on STOP.  The
  top-of-file text is right.
- `tools/src/diskman/worker.rs:134`: DiskMan's note is set once to "Nothing waiting to be written." and
  only changes when a file starts or stops failing, so it says that with gigabytes queued.
- `tools/src/archivist/schemas.rs:402-404`: "Archivist ran N of M schema file(s)" is Info on every connect,
  reconnects included; the rule says ran the schemas is Debug.  `:414` uses `Path::exists()` around DiskMan
  (harmless; `diskman::read().wait()` with `is_not_found()` is the in-house way).
- `tools/src/security.rs:384-387` refuses a second `start()` if a `Worker` is stored, even one whose thread
  died; `archivist/worker.rs:77` checks `is_finished()`.  Match Security to Archivist.
- `tools/src/clock.rs:107-110`: "not something to handle", followed by handling it.
- `tools/src/server.rs:14`, `constellations.rs:14`, `constellations/files.rs:15, 24, 37`,
  `constellations/text.rs:123`: "Control Panel" for what's the Server tab now.  `text.rs:123` is what's
  written into every `.cfg` file's comment, so that's the one worth fixing.
- Unused public API, `pub` so no warning: `fingerprinter::uuid_time()`, `archivist::batch()`,
  `diskman::stream()` / `Stream` / `Piece`, `Account::save()` and `Account::id()` (`accounts/src/lib.rs:119,
  150-159`; the web admin uses `edit()`), `primlib/src/save.rs:133-135, 199-210` `put_bool` / `read_bool`
  (no component saves a bool).
- `wgui/src/accounts.rs:210-220`: `changing()` calls `only_admin()` and then `allowed()`, which calls it
  again.
- `wgui/src/lib.rs:379-390`: a hand-kept list of every path so a wrong method gets 405; a route added to
  the match but not here answers 404.  Derive both from one table.
- `wgui/src/http.rs:194`: `std::thread::spawn` in a test helper.  Test-only; `threads::spawn` would keep it
  honest.
- `monitor/src/probe/windows.rs:240-242`: one umbrella comment for all 15 `unsafe` blocks; the rule asks
  for each block to say why it holds.  `read_cores()` (`:386-391`) most.
- `networking/src/tcp.rs:621-643`: a junk first packet is answered with `refuse()` without
  `pad_login_time()`; PROTOCOL.md says every answer takes at least 150 ms.  Harmless; the words disagree.
- `networking/src/protocol.rs:403-407`: `Packet` derives `Debug` while `LoginRequest` deliberately doesn't,
  for the key's sake; a `{:?}` of a Login `Packet` would print the key.  Nothing prints it today.
- `networking/src/ledger.rs:555-557`: `every_ending_has_words` leaves out `Blacklisted`, `NotWhitelisted`
  and `Banned`.  *Added 2026-10-02, with `TooManyFromOne`.*
- `networking/src/sessions.rs:213`: `Connected::Again(String)` carries an account nobody reads.
- `networking/src/udp.rs:345`: "a command, `/chat` so far" (`/who` exists).
- `gameclock/src/chat.rs:10-12`, `who.rs`, and `design/gameclock.md` ("Chat", "/who list"): all say
  networking hands the sender over "at its start".  It's `conductor_player_commands::wire()`, called by
  the launcher after `conductor_gameclock::start()`.  Works; the words are wrong.
- `gameclock/src/players.rs:281-283`: "Disconnected can't happen while the GameClock runs" isn't quite
  so: `stop()` closes the mailbox before it drops `STOP`, so an input check between the two sees it.
  Handled the same as Empty; the comment should say so.
- `gameclock/src/saving.rs:68-79` vs `design/gameclock.md`: the doc says the next save counts from when
  the last was due; the code counts from `now`.  Make the doc match.
- `design/gameclock.md`, "The ground comes in first": "162 at the default of 4" is the two-row number; it's
  891 (9 × 9 × 11), which world.md has.
- `gameworld/src/chunk.rs:207-208`: "above or below both rows"; there are eleven.
- `wgui/src/page.html:588-590`: "START SERVER brings up Fingerprinter, Security, Archivist and the
  monitor"; it brings up the desk, Lua, GameWorld, the GameClock and (later) networking too.
- `launcher/src/main.rs:69-70`: "a changed postgres.cfg or networking.cfg needs STOP SERVER and START
  SERVER"; `game.cfg` is soft too.
- `wgui/src/json.rs:71` and `wgui/Cargo.toml:8`: "Network Admin tabs"; the sections are CONFIGURATION and
  GAME MANAGEMENT.
- `Cargo.toml` (every crate): `version = "0.1.0"`, Cargo's default, against a release called 0.0.1.
  Nothing reads it.  *Set to 0.0.1 on 2026-10-02, Unity's Version too.*  No `[profile.release]` in the workspace either; `lto = true` and `codegen-units = 1`
  are the usual two for a release binary, worth a measurement, never `panic = "abort"` (the poisoned-lock
  recovery everywhere is built to survive a thread's panic).

---

## Checked and fine

So nobody looks again:

- **Packet parsers**: `take_packet`, `take_string`, `take_u32`, every `read_*`, `take_datagram`,
  `read_session_choice` are safe on any byte sequence: lengths checked before slicing, the frame length
  capped at 4096 before waiting for it, UDP over 1200 bytes dropped; `spans()`, `who_delivery()`,
  `chat_deliveries()` can't panic on their counts.  ConnectResult is smaller than any Connect it answers.
- **Locks**: `with_book` drains `gone` and `leaving` and calls the ledger and the GameClock after the book's
  lock is let go; `lock_for_loading` lets the book go before asking `saving()`; `typed::run` copies the
  function out before calling; `enforce()` holds `open` and the book while calling `access::verdict`, and
  `access` never takes either.  Scribe → Recent → DiskMan is the one order in tools, and DiskMan never
  holds its lock while logging or touching the disk.  `desk::stop()` drops its guard before `join()`.
- **Tokens and asks**: 32 OS-random bytes, used once, bound to the first address, a same-address resend
  answered from the book, replaced by a newer login, swept at `token_deadline`.  One ask in flight per
  player, the kept answer replayed, `/who list` keeps the ask open until the GameClock answers; a player
  who leaves mid-`play()` has the character taken straight back out.  The 1-second loading and leaving
  locks plus Protogame being one thread cover the double-spawn race on the legitimate path.
- **STOP then START**: the book, the ledger, the access lists, the DNS cache, `READY`, both mailboxes,
  `SAVING`, the chat and who boxes are all reset; the `SENDER` statics persist on purpose and are re-set by
  `wire()`; Protogame drains its mailbox after UDP is down and a late `enter()` is undone through
  `leave_world()` while the GameClock is still up.  `start_server()` and `stop_server()` are mirror images.
- **The ECS**: `destroy` bumps the generation and `is_alive` checks slot, alive and generation; `despawn`
  empties all eleven stores and the templates store before `destroy`, so a reused slot starts empty and an
  old handle gets `None` everywhere.
- **The save round trip**: `quote()` escapes `\\`, `"`, `\n`, `\r`, `\t` and every other control as `\ddd`;
  f32 → f64 → `{:?}` → `strtod` → f32 is exact; NaN and inf are written as `0` on purpose; `read_u32`
  rejects NaN; `convert_table` rejects mixed and gapped tables and functions; binary chunks are refused by
  mlua in safe mode.
- **The sandbox**: `io`, `os`, `package`, `require`, `debug`, `dofile`, `loadfile`, `load`, `warn`,
  `string.dump` all absent; `collectgarbage("stop")` can't beat the allocator limit (Lua's emergency GC on
  a failed allocation ignores the user stop); the hook is global so coroutines are covered; every script
  error is a Warn.
- **The GameClock**: `due()` from the cycle's start (no drift), `wait_until` is `recv_timeout` (no
  polling) and still looks for the stop when overdue, no subtraction that can panic, the tick is `const`.
  `leave()` marks before the note; notes are in order; a Leave unread at STOP SERVER leaves the character
  in `in_world` and the final `save_world()` writes it; `forget_saving()` runs on start and stop and wakes
  waiters; Archivist is one FIFO thread so a leaving save lands after any earlier world save of the same
  character.
- **Chunk and region math**: `div_euclid` everywhere a block becomes a chunk; `slot()` bounds-checks;
  BEDROCK at -32 and -31, STONE to ground-1, DIRT at ground, AIR above; a truncated map, heights or chunk
  file fails cleanly through `Reader` (apart from B2 and B3); a bad chunk file is a Warn and never built
  over.
- **Security**: `check_hash()` compares in constant time, parses the params from the stored line, refuses a
  line missing its salt or hash; `verify_no_account()` costs a real hash; `pad_login_time()` floors every
  attempt at 150 ms; `DbSettings` has a hand-written `Debug` that hides the password; `text.rs` never
  echoes a Secret.  No password reaches a log line.
- **The web admin**: the cookie is 32 OS-random bytes, HttpOnly, SameSite=Strict, Path=/Opus; every
  state-changing route needs the cookie, `only_admin()` and an `X-Opus` header, and the Host header is
  checked first, so CSRF from another tab is closed.  No `innerHTML`, `insertAdjacentHTML` or `eval`;
  everything goes in by `textContent`, and `json::text()` escapes quotes, backslashes and every control.
  `/proc` parsing counts fields from the last `)` and gives `None` on anything missing; `threads_of(pid)`
  takes a parsed `u32`, so no path traversal.
- **SQL**: UUIDs cast both ways; character creation locks the account row `FOR UPDATE` and checks slots
  and name in the transaction that writes; `delete` is scoped to the account; schemas run before
  migrations so `0002`'s foreign key resolves; `MIGRATION_TABLE` has no `uuid` but `0001` adds it.
- **The rules**: no `#[allow]`; `threads::spawn` is the only spawner (one `std::thread::spawn` in a test);
  every file has its header; every time is UTC; `PROTOCOL_VERSION` 9 matches PROTOCOL.md, `test_client.py`
  and Ensemble; every service in `EXPECTED` is in the test; `save_waiting()` merging with the running
  values is right because the page sends every field of the card.
