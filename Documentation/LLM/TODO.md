<!--
File:       Opus/Documentation/LLM/TODO.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- TODO

The big features, the ones that are a run of sessions each, are in LONGTERM_TODO.md instead.

## Deferred

Things that wait on a piece that doesn't exist yet, or on Jacob wanting them.

### The game

- **Chat** (Jacob, 2026-09-30: the 0.0.1 goal is "get a player spawned in the world and able to chat").
  Nothing designed: who hears whom (everybody, or those nearby), what the packets are (a protocol bump),
  whether the web admin sees it.
- **Protogame**: the game-adjacent piece between a logged-in player and the world (Jacob's word).  Today
  `sessions.rs` in networking has the account's name and the UDP side only keeps the player alive.
  Messages to and from a character are protogame's.  Settled for its database side, the first step:
  - **`player_characters`**, a new table (its own schema file, `id` and `uuid` like every table).  Each
    row has its account's `id` (`account_id`, `ON DELETE CASCADE`, so deleting an account wipes its
    characters).
  - **Three slots on the account**: `character_slot_1` to `character_slot_3` on `accounts`, each the `id`
    of a `player_characters` row or empty (`ON DELETE SET NULL`).  `accounts.sql` is frozen, so it's a
    migration (`0002_...`).  The link is in two places, so making or deleting a character is two writes
    in one transaction.
  - **`conductor-accounts` writes the SQL for both tables**; protogame and the game library call its
    functions.
  - **`CharacterSnapshot`** lives in `conductor-accounts` beside `Account`: the surface of a character
    (its name, where it is) for whatever needs one outside the world (character select, the web admin).
    Read from the row, so it's the last save, and never written back.  The character itself lives in the
    game library, and only it writes its row.
  - Actor, Agent and Character (anything that acts; one the computer controls; one with a human on top)
    were named before primlib, and are likely sets of components now rather than types.

  - **The flow** (Jacob, 2026-09-30): "account logs in (done) -> character selection -> selected character
    spawns in world at its last save loc (0,0,0 for now)".  Character selection is the start of the UDP
    connection, so a character is picked after the UDP connect, not at the TLS login (protocol version 5).
  - **The character is a template** (primlib's), hydrated from the account: Jacob's first step of the
    three on his map in STATUS.md.  Part A, the GameObject side (Living, Character, `PlayerCharacter`,
    saving as Lua text), is written: `design/primlib.md`, "The character and saving", has Jacob's
    answers.  **Part B** is what's left of this step:
    - **`player_characters`**, as above, with the name and the last position as columns of their own and
      the rest of the save as Lua text in one column (the mix, Jacob's yes: character select lists names
      and the spawn reads the position without running anything).  Every UPDATE rewrites the row whole,
      so how often it's saved doesn't pick the shape; what SQL needs to see does.
    - **The account's slots point at the character by `id`**, per CLAUDE.md ("refer to CLAUDE on this").
    - **Written, not run yet** (2026-09-30): the schema file `player_characters.sql` and migration
      `0002_character_slots_on_accounts.sql`.  My picks in them, Jacob's to turn round before the first
      START SERVER freezes them: a name is 1 to 32 characters, unique across the server whatever the
      capitals; the position is three `REAL` columns (an f32 each, like the Transform).
    - **The functions in `conductor-accounts`**: make, list, load and save a character, and delete one.
  - **The player makes a character at character select**, from the client, so nothing makes one until
    step 2, and it's tested through the game then: "We'll build it to test it through the game".

  Open: which messages protogame carries; how the test client shows it working.
- **The GameClock's checks**: an input mailbox and an input packet, a brain for the AI, movement into
  `Transform`, the broadcast (only what each player may see).  `design/gameclock.md`.
- **A spawn system** (Jacob, 2026-09-30): keeps count of the NPCs in the world and spawns more from their
  blueprint when a kind runs low ("when the number of goblin_as is growing low").  So a copy needs to know
  its blueprint.  It goes in the housekeeping check and has to wait for `conductor_gameclock::ready()`.
- **Saving primlib's copies** on STOP SERVER, with their UUIDs and internal names (`goblin_archer_1`),
  and loading them back on START SERVER.  `design/primlib.md`.
- **The world's part two**, sending chunks to a client, loading around players who move: LONGTERM_TODO.md
  and `design/world.md`.

### Networking

- **Client management**, not this iteration:
  - A player limit: "The server is full."  Today `max_waiting_logins` caps the door and nothing caps the
    world.
  - A login token on reconnect (`fingerprinter::new_token()`), so a player who drops and comes back
    doesn't pay for a hash.  Against today's rule on purpose (a dropped UDP session is gone, start over),
    so it's a design change when it comes, not a fix.
  - A session id in every UDP packet, so a home router changing the port mid-session doesn't end it.
  - Messaging a player from the web admin.
- **A client certificate for every client** (mutual TLS).  Today the server never asks a client for
  one; the test client's `--cert` is the client checking the server.  It waits on Soundcheck
  (LONGTERM_TODO.md).
- **The whitelist and blacklist changeable while the server is stopped** (Jacob, 2026-09-30).  Today the
  tabs are locked until both listeners are up, and `addip` / `removeip` answer 409 while networking isn't
  running.  It would take: the two tabs open while stopped, like Settings; `/Opus/networking` reading the
  files when networking isn't running; a change while stopped written straight to the file through
  DiskMan, with nothing to enforce.  Open: whether the lists stay locked without the database, and
  whether Connections keeps its own lock.  CLAUDE.md's rule gets rewritten with it.
- **`access_list` switchable from the page at once.**  Today it takes on the next START SERVER, while the
  lists take at once.  Jacob's call if the reboot is a bother.
- The protocol version in the Hello is `4` and the client versions are a list in `networking.cfg`.
  Whether Ensemble reports a version string or a number is Ensemble's call.
- Reverse DNS on macOS: `dns/other.rs` hands back no name.  macOS has `getnameinfo` with its own
  `sockaddr` layout (a length byte first).  Waits on a Mac.

### The web admin

- **A Tick evaluator tab under GAME MANAGEMENT** (Jacob, 2026-09-30): how the GameClock is keeping time.
  Today the Services tab's GameClock line is all there is.  Open: what it shows (each check's time, the
  longest and the latest, late cycles, maybe a graph of the last minute); the numbers the GameClock keeps
  for it (like the monitor's `latest()`); its read path under `/Opus/`, asked for when it's built.
  Nothing on it changes anything, so no `wwwhook` route.
- **The character on the Connections tab's UDP list**, beside the account, once there are characters.
- **A list of blocked names** (Jacob, 2026-09-30), its own session:
  - `Content/cfg/blocked_names.txt`, beside the two access lists, one entry a line and not in
    Constellations' table, so CLAUDE.md's "one exception" becomes three files.
  - A Blocked Names tab under CONFIGURATION, with REMOVE on each and an ADD field.
  - For now it's curse words, and it checks account usernames.
  - A blocked word of 4 letters or more blocks any name with the whole word in it: "shit" blocks
    "Shitfox", "bastard" blocks "Bastardfox", but "bastard" doesn't block "Starlight".  A word under 4
    letters blocks nothing ("ass" lets "Ass" and "Cassandra" through).
  - Read on START SERVER; a change from the page waits for the next one (Jacob: "we don't need this to be
    hot swappable").
- **Move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`** (Jacob's call, 2026-09-29).  A new
  setting in `wgui.cfg`'s table in `constellations/files.rs`, the old one taken out of the globals, and
  the launcher reading it from there.  Still a hard reboot.
- Hash the two passwords in `wgui.cfg` through Security.  Security only runs with the server, and a login
  has to work before START SERVER, so it waits on Security being up from boot, or a hash on the caller's
  thread.  Plain text is fine while the page only listens on this machine.
- HTTPS.  rustls is in the build, so it waits only on wanting it, and on the browser's warning for a
  self-signed certificate.

### The rest

- **Scribe: the Debug switch** in `conductor_globals.cfg` that drops Debug lines when off.  Then go
  through every log line and move the routine ones to Debug, per CLAUDE.md.  Archivist's connect, schema
  and settings lines first, and the launcher's four start and stop lines.
- **Archivist: reconnect on its own** every few seconds while disconnected.  Today it only tries when a
  job comes in, so the web admin's database lock never lifts by itself.  Not answered yet.
- **Launcher: catch Ctrl-C** and shut down cleanly (or ignore it).  Ctrl-C loses whatever DiskMan hasn't
  written yet.  Without a crate it's a signal handler on Linux and a console handler on Windows.
- **Playtime metrics** (Jacob: "cool metrics later").  Only the latest login is kept, and nothing records
  when a player left.  It needs a table of play sessions (account, in, out, how it ended), a row written
  as each player leaves.  A new table, so its own session.
- **Where the test client lives** and what it's called, once it outgrows `networking/`.
- Monitor on macOS: `proc_pidinfo` / `proc_pid_rusage` from libproc.  Waits on a Mac to test on.
- Ensemble has no way to find `Content/` yet.  Ensemble's first session's call.
- Where the purchased art lives, and whether it goes in the repo through LFS.  `Content/Assets/` is
  ignored for now.  Jacob's call when the client needs it.
- **Stale words in the code**, for whichever session next touches each file:
  - `access.rs`: a Warn the admin sees says "the web admin's Networking tab" (the tabs are Whitelist and
    Blacklist), and a comment the same.
  - `dns.rs`, `dns/other.rs` and a comment in the web admin's `lib.rs` still say "the TCP tab".
  - `security.rs` says the arena is kept for as long as Conductor runs (it goes with the server).
  - `snapshot.rs` says uptime is a moment less than Conductor's (it's since START SERVER).
  - `tcp.rs`'s header says a stop has no deadline (it has 2 seconds).
  - `json.rs`'s notes and the Server tab's note on the page leave out the Settings tab and networking.
  - The header of `constellations.rs` names only `postgres.cfg` as soft.
  - The monitor's `Cargo.toml` header leaves out the process list.
  - `json.rs` and the web admin's `Cargo.toml` say "the Network Admin tabs", and `json.rs` says the page
    shows "the Control Panel" while the server is stopped (the Server tab).
  - "The Control Panel" where the Server tab is meant: `accounts.rs:202` (text the admin sees), the launcher's
    boot line (`main.rs:78`), and comments in the web admin's `lib.rs`; `page.html:20` has a history note.
  - The web admin's `lib.rs`: "KICK on the TCP tab"; "when Security brings in TLS for the game" (networking
    has it); the header's list of what needs `X-Opus` leaves out LOG OUT, the settings, the kick, the lists
    and the accounts.
  - The launcher's `main.rs`: `stop_server()` says networking goes first (the monitor does), and the header
    calls `start_server()` / `stop_server()` the list of what the server is (networking opens from
    `take_commands()`).  Its boot line names the soft files and leaves out `game.cfg`.
  - Networking's `lib.rs` and `Cargo.toml` say it starts on START SERVER (it's once the ground is in), and
    `lib.rs` and `udp.rs` say there's no game yet.
  - The monitor's `lib.rs` header leaves out the process list, per-core load and the machine's RAM.
  - `protocol.rs` says the client will most likely be C# (it is), and `LoginAnswer::Unavailable`'s comment
    leaves out Fingerprinter failing to make a token.
  - gameworld: `Ground::Flat`'s doc leaves out BEDROCK at -16; `lib.rs` says saving chunks "comes next".
  - `test_client.py`'s usage lines only work from inside `networking/` (the terminal sits in
    `Conductor/dev`), and its `--cert` example is relative to the working directory.
  - `security/windows.rs`, `fingerprinter/windows.rs` and `fingerprinter.rs` say the Windows code was never
    built (it was, 2026-09-30).  The tools' `Cargo.toml` header lists only some of what's in the crate.
  - `accounts.sql`'s header says a player gets a clear message (the admin makes accounts now).  The file is
    frozen, so it stays.
- `RegionMap::from_bytes` never checks for a count of 0 regions.  REGION_MAP.md says 1 to 255, and a map of
  0 is refused anyway unless its width or depth is also 0.  The code is what gets fixed, if it's worth it.

## Ideas

Things we thought of along the way.  None of them are promised.

- The world: blending one biome into the next where two regions meet.  Today Alpha and Omega meet in a
  sharp divide.  Jacob: "in a real build and not this test, we'll have a blending technique".
- Scribe: a size limit that starts a second file for the day (`2026_09_28.1.scribe.log`).
- Scribe: the lines logged before `move_to()` stay in the default folder if the config points elsewhere.
  Not worth fixing while both are the same folder.
- Scribe: a lost log file is only retried at midnight UTC or on `move_to()`.  A retry every few minutes
  would get it back sooner after a full disk is cleaned up.
- Scribe: the caller shows the path Rust compiled with (`launcher/src/main.rs`).  Trim to the file name
  if that gets noisy.
- Constellations: a stray `Content/` inside `Conductor/dev` shadows the real one, since the walk up takes
  the nearest.  Skipping a `Content` whose parent holds a `Cargo.toml` would rule it out.
- The Storage tab could show `swaps_waiting` from DiskMan's status (it's in the struct, not the JSON).
- Archivist: a password that starts or ends with a space loses it, since every value is trimmed.  Quotes
  around the value would fix it, if it ever matters.
- Archivist: more than one worker, if one ever can't keep up.  Two workers can finish jobs out of order
  (a SELECT missing the UPDATE sent just before it), so it needs one player's jobs kept on one worker.
- Monitor: a SQL read / write split by rows, not just by jobs; Postgres's own view
  (`pg_stat_database`) as an Archivist job every few seconds.
- The Storage tab: "last read / last write" by file.  DiskMan sees every file, so it's ready when wanted.
- DiskMan on Windows: it leans on `fs::rename` replacing a file (its writes and the `.wait4server` swap
  both), and skips flushing the folder.  It runs there; nobody has looked closer.
- Notices: no cap.  A flood of Warns left alone for days keeps growing in memory.
- Web admin: keep the CPU and memory history on the server, so a page opened late sees the last minutes.
- Web admin: a Debug on / off switch for the Log tab, once Scribe has its switch.
- Web admin: saved page layouts per account, and more accounts than `user` and `admin`.
- Web admin: the Settings tab could show each default, with a "back to default" click.
- Web admin: pin Conductor to the top of the System tab's process list, if busiest-first buries it.
- Web admin: a setting that starts the server when Conductor boots, for a machine nobody sits at.
- Web admin: a line on the Server tab saying what a RESTART is for (a changed soft file is read again).
- Security: raise the memory (128 MiB, say) once the benchmark says what 64 MiB costs.
- Security: the `parallel` (rayon) feature would split a hash across cores.  A crate, and its threads
  bypass `threads::spawn()`, so it's not taken.
- Security: say on the Services tab whether the huge pages actually landed (`AnonHugePages` in
  `/proc/self/smaps`).
- Security: wipe passwords from memory once they're hashed (`zeroize`).  Off for now.
- Networking: make the TLS pair on first start (the `rcgen` crate) instead of openssl by hand.  Jacob
  picked the command for now.
- Networking: the private key sits in DiskMan's cache while Conductor runs.  Reading it past DiskMan
  would keep it out, if it ever matters.
- Networking: the login threads' `serving` list and the failure hold are two small maps under two locks,
  and the access lists a Vec walked on every accept.  Fine at this size; look here first if the door ever
  sees thousands.
- Networking: a ban from the page rewrites the list file, so a hand-written comment in it is lost.
- Running Conductor with no console window.  Closing the console kills it today without a clean shutdown.
  Ways out: start it detached (`setsid`, `nohup`), a systemd or Windows service, or a Windows build with
  no console.  Jacob's pick when it matters.
