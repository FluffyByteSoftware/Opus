<!--
File:       Opus/Documentation/LLM/TODO.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- TODO

The big features, the ones that are a run of sessions each, are in LONGTERM_TODO.md instead.

## Bugs

Found in the 2026-09-29 test run.  None open: bug 1 (the Settings tab keeping a "waiting" warning
after a RESTART SERVER) was the page's snapshot, and was fixed the same day.

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
- **Protogame** (Jacob's pick at the 2026-09-30 close, called the protocore for a day and protogame
  again since).  What takes over after the login: "game adjacent", not the game itself.  Today
  `sessions.rs` in networking has the account's name and the UDP side only keeps the player alive.
  Settled in the design talk, 2026-09-30:
  - **Two libraries.**  Protogame is the game-adjacent piece.  The game library is the world, and it
    holds **Actor**, **Character** and **Agent**: an Actor is anything that acts in the world, an Agent
    is an Actor the computer controls, and a Character is an Actor with a human controller on top.
    Character isn't in protogame.  **The game library is going to be an ECS** (entity, component,
    system; Jacob, 2026-09-30): a character is a few components on an entity, not one type, so how
    Actor, Agent and Character look is being rethought with it.  The design steps back to the ECS
    before the character's row gets its columns.
  - **`CharacterSnapshot`** lives in `conductor-accounts`, beside `Account` (it reads the row, and
    protogame uses it; in protogame it would make a circle of crates): the surface of a character (its
    name, where it is in the world), for whatever needs one outside the world (character select, the
    web admin).  Read from the character's row when it's needed, so it's the last save to the
    database, and never written back.  The player character itself lives in the game library, and
    only it writes its row.  Messages to and from a character are protogame's.
  - **The first step is the database side** (Jacob, 2026-09-30): the table, the slots and the
    functions in `conductor-accounts`, no protocol and no world.  It waited on the ECS, for what's
    in the row; the ECS is in now (`conductor-primlib`, 2026-09-30), so what a character's row holds
    can be settled from its components.
  - **`player_characters`**, a new table (its own schema file, `id` and `uuid` like every table).
    Each row has its account's `id` (`account_id`), a quick way back to the account.  Deleting an
    account wipes its characters.
  - **Three character slots on the account**: `character_slot_1`, `character_slot_2` and
    `character_slot_3` on `accounts`, each the `id` of a `player_characters` row, or empty.
    `accounts.sql` is frozen, so it's a migration (`0002_...`).
  So the link is in two places, the slots and the row, and making or deleting a character is two
  writes in one transaction.
  - **`conductor-accounts` writes the SQL for both tables**, so nothing double-writes.  Making,
    deleting and saving a character and reading its snapshot are functions there; protogame and the
    game library call them and never write SQL.
  - **Postgres does the wiping**: `account_id` is `ON DELETE CASCADE`, so an account's characters go
    with its row, and each slot is `ON DELETE SET NULL`, so a deleted character empties its slot.
  Still open: the rest of the ECS (below); whether the world and the voxels are primlib's (and
  LONGTERM_TODO.md's "The world" entry); which messages protogame carries (the client's UDP traffic to
  its character, chat between characters, or the game telling a character what happened); who makes a
  character (the player from the client, or the admin); whether one is chosen at the TLS login or after
  the UDP connect (protocol version 5 either way); the chunk and world sizes and where the world is
  kept; the tick rate; how the test client shows it working.  More than one session's step, so the
  first one picks where to stop.
  - **The ECS, so far** (Jacob, 2026-09-30, talked through here after the separate chat didn't
    work out): component driven.  An object is packed with components, and the components make it
    into something else.  The exception is a character, which will probably have some baked
    properties, always there, beside its components.  Jacob's sample of an NPC in the scripting
    language (in LONGTERM_TODO.md) shows the shape: a **template** is a named set of components
    (`NPC`: Position, Rotation, Scale, ShortName, LongName, Titles, Health, Endurance, Mana), `create`
    makes an object from it and sets the values, and Conductor sends it to the clients as it changes.
    `goblin_a` is a blueprint: `spawn goblin_a x 100` makes a hundred, each with its own values.  Who
    sees what isn't a setting on the components: the server builds each player's packets and puts in
    only what that player may see (Jacob, 2026-09-30).
  - **Built, 2026-09-30**: the ECS waited on the scripting language (Lua 5.4 through `mlua`, in
    `lua-parser`), and then went in as `conductor-primlib`: entities, the world, ten components,
    templates and blueprints, built and tested.  `design/primlib.md` has it.  An object's behaviour
    is a behaviour script added to it; GOAP for the thinking, maybe.
  - **Opened, 2026-09-30** (Jacob): the session that built the "GameObject", an entity storage system.
    Components are added to an entity to change how it behaves and how it can be dealt with on the
    server and in the client.  "We'll cheat a little" with templates or blueprints that set out the
    components a whole kind of object needs or is expected to have: an NPC's has health, position,
    rotation and the rest.  `design/primlib.md` has what was settled and what's open.
  - **The game library is `conductor-primlib`** (Jacob, 2026-09-30: "prim for primitive"), folder
    `Conductor/dev/primlib/`, a lib crate.  Written by hand, no crate for the ECS.
  - **Template, blueprint, copy** (Jacob, 2026-09-30).  `NPC` is a template: a cheat sheet of
    components with their defaults, "so I don't write the same 50 lines in 50 npcs".  `goblin_a` is
    a blueprint, "an actual NPC file" asked what a new goblin_a looks like.  A blueprint can drop a
    component its template gave it and add ones the template doesn't have.  Jacob's picture of the
    Lua: a `setup()` the template packs its components and defaults into, and an `awake()` called
    right before a copy is put in the world, where the object's own code takes a component away or
    adds more.  **Part 2 is the Lua** (Jacob, 2026-09-30): templates and blueprints written as
    scripts, after the Rust side is in.  The first part is Rust only.
  - **Rotation** (now part of `Transform`) is three angles in degrees, the way Unity's inspector
    shows it (Jacob: "whatever the standard is").  See `design/primlib.md`.
  - **A spawn system** (Jacob, 2026-09-30, its own feature): keeps track of the NPCs in the world
    and spawns more from their blueprint when a kind runs low ("when the number of goblin_as is
    growing low").  It needs to know which blueprint each copy came from.
  - **The ECS, open**: where behaviour lives (in the components, the way a Unity script does, or in
    systems that run over every object with a given set of components); which properties a character
    has baked, and whether other objects have any; which components a character is; how a character's
    components are saved (a column each on `player_characters`, a table per component, or one column
    holding them all); how many entities and what tick the world is sized for.  And whether a spawned
    NPC is saved at all: Jacob's first thought was that a copy becomes a database row once it's
    spawned and its values are managed through the row until it's destroyed ("Flat files made this
    easier").  That ran into the game loop never waiting on the database, and Jacob's answer is that
    a copy is written to the database as a row of its own when the server is stopped.  Every copy has
    a UUID and an internal name (`goblin_archer_1`).  `design/primlib.md` has it, and what's open.
- **Client management**, Jacob's words for the lot of it, 2026-09-29: not this iteration.  The point of
  this one was handing a client from TCP to UDP and logging them off.  Waiting in here:
  - A player limit: "The server is full." (Stratum had 50, with a few more TCP connections so a full
    server could still say so).  Today `max_waiting_logins` caps the door and nothing caps the world.
  - A login token on reconnect (`fingerprinter::new_token()`), so a player who drops and comes back doesn't
    pay for a hash.  The biggest CPU saving Security can't make on its own.  Against today's rule on
    purpose (a dropped UDP session is gone, start over), so it's a design change when it comes, not a fix.
  - A session id in every UDP packet, so a home router changing the port mid-session doesn't end it.
  - Messaging a player from the web admin.  Seeing who's on (the Connections tab's UDP list) and
    kicking (KICK in a TCP row's menu) were built on 2026-09-29.
- **The test client connecting without `--cert`** (Jacob, 2026-09-30, after the first login from outside,
  from his work laptop over the internet, went through): "it didn't reject the client with no cert."
  What happened: the server never asks a client for a certificate (`with_no_client_auth()` in
  `tls.rs`); a player proves who they are with the username and password inside the TLS.  `--cert` is
  the other way round, the client checking the *server's* certificate, as PROTOCOL.md says a client
  does.  The test client looks for `Content/certs/conductor.crt` three folders up from itself, and on
  the laptop it wasn't there, so it fell back to checking nothing, said so, and carried on.  Two
  readings were put to him (the test client refusing to go on unchecked, or clients with certificates
  of their own), and his answer is the second, **mutual TLS**: "the server should be rejecting a client
  connecting with no valid certificate ... The clients should be given a cert by the patcher before the
  game is launched... then both the client should be able to trust the server and vice-versa."  It
  needs the patcher, **Opus.Soundcheck** (Jacob's name), with one certificate per client; it waits on
  that, and LONGTERM_TODO.md has the rest.
- **Web admin: the character on the Connections tab's UDP list**, beside the account, once there are
  characters.  The list itself (address, account, connected when, playing for, quiet for) was built with
  the access lists, 2026-09-29.  Kicking is built, from the TCP row the player's login came through.
- **The whitelist and blacklist changeable from the page while the server is stopped** (Jacob,
  2026-09-30, going through the 09-29 checks).  Today it runs into a rule in CLAUDE.md: the Whitelist
  and Blacklist tabs are locked until both of networking's listeners are up, and `addip` / `removeip`
  answer 409 while networking isn't running, so a stopped server means editing the files by hand.  What
  it would take: the two tabs open while stopped, the way Settings is; `/Opus/networking` reading the
  files when networking isn't running (today it answers `running: false` with empty lists); and a change
  while stopped written straight to `whitelist.cfg` / `blacklist.cfg` through DiskMan, with nothing to
  enforce (nobody's connected) and the next START SERVER reading it as it does now.  Open when it comes:
  whether the lists stay locked while the server runs without the database, like today, and whether
  Connections keeps its own lock.  CLAUDE.md's rule gets rewritten with it.
- **Networking: `access_list` switchable from the page at once.**  Today the switch is in `networking.cfg`
  and takes on the next START SERVER, while the lists themselves take at once.  Jacob's call if the
  reboot is a bother.
- Networking: reverse DNS on macOS.  `dns/other.rs` hands back no name; macOS has `getnameinfo` with its
  own `sockaddr` layout (a length byte first).  Waits on a Mac, like the monitor.
- Networking: the protocol version in the Hello is `4` and the client versions are a list in
  `networking.cfg`.  Whether Ensemble reports a version string or a number is Ensemble's call.
- **The world tick** (Jacob, 2026-09-30, at the end of primlib's first part: "next session ... we need
  to put the world tick in").  Jacob's pick for next.  primlib's `World` exists but nothing runs it:
  the tick is the game loop that owns the world and steps it forward.  It's a server piece, so it goes
  in `start_server()` and `stop_server()` in the launcher, on its own thread through `threads::spawn()`,
  reporting to `services.rs` (`EXPECTED` and the test's list).  Open for then: the tick rate (CLAUDE.md
  has a line waiting for it; a soft-reboot setting, by the rule); what runs each tick, and in what
  order; what happens when a tick runs long; whether the world is made fresh on every START SERVER.
  Saving the copies on STOP SERVER and loading them back (`design/primlib.md`) lean on it too.
  - **Jacob's design, 2026-09-30 (the heartbeat)**: a full tick cycle is **250 ms**, cut into **five
    checks of 50 ms** each.  Each check touches its own group of objects and does whatever it needs
    to: one takes in the players' input, one sends positions out to everybody else, one runs the AI's
    brains, "and so on".  Argon2 stays on its one thread on the side (Security's worker), and with
    that it never ran far over 250 ms.  Jacob remembered a heartbeat stashed in `conductor-tools`
    from an earlier go; there isn't one in Opus's history (it was likely Stratum or Mantle, whose code
    doesn't carry over), so it's written fresh.
  - **Jacob's answers, the same day**: 50 milliseconds a check, five to a 250 ms cycle.  The checks,
    in order: input, AI, movement, the position broadcast, housekeeping ("I have no idea what order
    they should go in", so it's his best guess and can move).  A check that runs long makes the next
    one late, and the cycle catches up where it can; nothing is skipped.  **The rate is fixed in
    code, not a setting**: "from all the testing I did before anything faster is gonna be a problem.
    Slower is fine but faster becomes bad."  The crate is `conductor-gameclock` (folder `gameclock`),
    and code calls it `conductor_heartbeat`.
- **A Tick evaluator tab under GAME MANAGEMENT** (Jacob, 2026-09-30, while the heartbeat was
  planned): a tab beside Connections that shows how the heartbeat is keeping time.  For now the
  Services tab's line for it is all there is (cycles, late ones, the longest).  Open for when it's
  built: what it shows (each check's time, the longest and the latest, how many cycles ran late,
  maybe a graph of the last minute); where the numbers come from (the heartbeat keeping them for the
  page, like the monitor's `latest()`); its read path under `/Opus/`, asked for when it's built.
  Nothing on it changes anything, so no `wwwhook` route.
- Web admin: the Control Panel (built 2026-09-29; the "Manage System" screen) starts and stops the server,
  which today is Fingerprinter, Security, Archivist, networking and the monitor.  The game loop goes in
  `start_server()` and `stop_server()` in the launcher when it exists, and comes up and down with the
  rest.
- **A list of blocked names** (Jacob, 2026-09-30, while the sections were drawn): names nobody gets to use,
  one a line, with a Blocked Names page under the web admin's CONFIGURATION heading (REMOVE on each, an
  ADD field, like the Whitelist and Blacklist tabs).  Its own session.  Jacob's answers:
  - The file is `Content/cfg/blocked_names.txt`, beside `whitelist.cfg` and `blacklist.cfg`.  Like
    them, one entry a line and not in Constellations' table, so CLAUDE.md's "one exception" becomes
    three files.
  - For now it's curse words, and it checks account usernames.
  - A blocked word of 4 letters or more blocks any name with the whole word in it: "shit" blocks
    "Shitfox", "bastard" blocks "Bastardfox".  Only the whole word counts, never a piece of it, so
    "bastard" doesn't block "Starlight".  A word under 4 letters blocks nothing ("ass" lets "Ass"
    and "Cassandra" through).  Jacob's examples, 2026-09-30.
  - Read on START SERVER, like the soft files; a change from the page waits for the next one.  Not a
    hot swap (Jacob: "we don't need this to be hot swappable").
- **Web admin: move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`.**  Jacob's call at the
  2026-09-29 wrap-up, once the web admin had a file of its own.  The steps are in STATUS.md.
- Launcher: catch Ctrl-C and shut down cleanly (or ignore it).  Since DiskMan, there is something to save
  on shutdown: Ctrl-C loses whatever it hasn't written yet.  Catching it on both Linux and Windows without a crate means a
  signal handler on one and a console handler on the other.
- Ensemble has no way to find `Content/` yet.  The engine is Unity 6000.6 now; how it finds the folder is
  Ensemble's first session's call.
- Where the purchased art lives, and whether it goes in the repo through LFS.  `Content/Assets/` is ignored
  for now, so it stays out of git.  Jacob's call when the client needs it.
- **Playtime metrics** (Jacob, 2026-09-29: "cool metrics later").  `last_login_datetime` is when a player
  came in over UDP, but only the latest one is kept, and nothing records when they left.  Playtime needs
  a table of play sessions (account, in, out, how it ended), a row written as each player leaves the
  book.  A new table, so its own session.
- **Stale words in the code**, found the same day, for whichever session next touches each file:
  `access.rs` has a Warn the admin sees that says "the web admin's Networking tab" (the tabs are Whitelist
  and Blacklist), and a comment the same; `dns.rs`, `dns/other.rs` and a comment in the web admin's
  `lib.rs` still say "the TCP tab"; `security.rs` says the arena is kept for as long as
  Conductor runs (it goes with the server); `snapshot.rs` says uptime is a moment less than Conductor's
  (it's since START SERVER); `tcp.rs`'s header says a stop has no deadline (it has 2 seconds);
  `json.rs`'s notes and the Control Panel's note on the page leave out the Settings tab and networking;
  the header of `constellations.rs` names only `postgres.cfg` as soft; the monitor's `Cargo.toml` header
  leaves out the process list; `json.rs` and the web admin's `Cargo.toml` say "the Network Admin tabs" (there's
  no such heading since the sections, 2026-09-30), and `json.rs` says the page shows "the Control Panel"
  while the server is stopped (the Server tab).

## Ideas

Things we thought of along the way.  None of them are promised.

- Scribe: a size limit that starts a second file for the day (`2026_09_28.1.scribe.log`) on top of the
  midnight rollover.  Stratum had one.  Not picked for now.
- Scribe: the handful of lines logged before `move_to()` stay in the default folder if the config points
  somewhere else.  Holding them in memory until the folder is known would fix it.  Not worth it while both
  folders are the same one.
- Scribe: a lost log file only gets retried at midnight UTC or on `move_to()`.  A retry every few minutes
  would get it back sooner after, say, a full disk is cleaned up.
- Scribe: the caller shows the path Rust compiled with (`launcher/src/main.rs`).  Trim to the file
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
