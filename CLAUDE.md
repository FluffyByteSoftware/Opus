**EVERY SESSION PUSHES ITS CODE TO THE `unstable` BRANCH.  Never to a session
branch, whatever branch the session was opened on.  Jacob's rule.**

<!--
File:       Opus/CLAUDE.md
Component:  Opus
Author:     Jacob Chacko
-->

# Opus

Opus is the codename for this project: a multiplayer game made of a server and a
client. When I say "Opus" I mean this project, not the Claude model.

**The game's name is Forgotten Legends** (2026-10-01): it's what players see
(Unity's Product Name, under Company Name FluffyByte Studios).  The project,
its folders, its crates and its code stay Opus ("its the games name but not
the directory path").

Opus is a separate project from Stratum and Mantle. Do not pull code from them
unless I explicitly ask. Their *workflow* carried over; their code did not.

Project root: `/opt/storage/Coding/Opus`

**The 0.0.1 goal is a player spawned in the world and able to chat**
(Jacob, 2026-09-30).

---

## Components

- **Conductor** -- the game server. Authoritative: it owns the game state and the
  tick loop. Clients ask, Conductor decides.
  - Language: Rust, edition 2024
  - Folder: `Conductor/`
  - Runs on: Linux (Nobara and Fedora). Development is Linux-first; Windows
    code stays wired in but can sit untested. macOS isn't a target. See
    "Linux and Windows" under the Rust rules.
- **Ensemble** -- the game client that players run.
  - Engine / language: Unity 6000.6 (C#).  Its project settings and our
    four folders under `Assets/` are committed; the rest is on Jacob's
    machine.
  - Folder: `Ensemble/`
- **Soundcheck** (`Opus.Soundcheck`) -- the patcher, not started.  It runs
  before the game and hands each client a certificate of its own, so the
  server can turn away any connection without one (mutual TLS).
  LONGTERM_TODO.md has it.
- **Documentation** -- project docs. `Documentation/LLM/` holds the working
  docs (status, TODOs, design, protocol) and is the source of truth for anything
  not in the code.
- **Shared contracts** -- whatever both sides need to agree on is written down
  once: the packets in `Documentation/LLM/PROTOCOL.md`, the world's
  `region.map` in `Documentation/LLM/REGION_MAP.md`.

If a future piece appears (tools, admin console, test client), ask me what
to call it before creating it.

---

## Folder layout

```
Opus/
├── CLAUDE.md              # this file
├── README.md              # the front page: what Opus is, where it stands, how to run it
├── .gitignore, .gitattributes
├── Conductor/             # server
│   ├── dev/               # source code -- a Cargo workspace
│   │   ├── tools/         # lib: DiskMan, Scribe, Constellations, Fingerprinter, Security, Archivist, ...
│   │   ├── accounts/      # lib: the one way in to the accounts table, and the account desk
│   │   ├── monitor/       # lib: looks at the process and the machine once a second
│   │   ├── networking/    # lib: the login over TLS on TCP, the game over UDP; test_client.py beside it
│   │   ├── lua-parser/    # lib: runs the Lua scripts in Content/scripts/, locked down
│   │   ├── primlib/       # lib: the game library -- entities, components, templates, blueprints
│   │   ├── gameworld/     # lib: the ground -- blocks, chunks, regions and their files
│   │   ├── gameclock/     # lib: the GameClock, the game loop
│   │   ├── wgui/          # lib: the web admin on 127.0.0.1, and the only way to shut down
│   │   └── launcher/      # bin: the program -- boots, then starts and stops the server on the Server tab's say
│   └── build/             # compiled output -- never committed
├── Ensemble/              # client
│   ├── dev/
│   │   └── Opus.Ensemble/ # the Unity project.  Its Packages/ and UserSettings/ are gitignored
│   │       └── Assets/    # gitignored (the purchased art), all but our four:
│   │           ├── Editor/    # editor plugins, under Tools > Opus
│   │           ├── Code/      # the plain C#: the networking, say
│   │           ├── Scripts/   # the scripts
│   │           └── Data/      # our own data files: layouts in Data/Layouts/, styles in Data/Styles/
│   └── build/             # compiled output -- never committed
├── Content/               # data both programs read and write -- committed, except Assets/, logs/ and world/
│   ├── Assets/            # purchased art -- never committed
│   ├── cfg/               # config files (conductor_globals, wgui, postgres, networking, game, whitelist, blacklist)
│   ├── certs/             # the TLS certificate (committed) and its key (never committed), made with openssl
│   ├── scripts/           # the Lua scripts, folders inside it and all
│   ├── logs/              # log files -- never committed
│   ├── world/             # the game's save: region.map, and Regions/<region>/ -- never committed
│   └── psql/
│       ├── defaults/schemas/ # database schemas as first made, one .sql file per table
│       └── migrations/    # every change to a table after that, numbered
└── Documentation/
    ├── HowTo/             # how-tos for a person, public docs in my voice
    │   └── WINDOWS_INSTALL.md # building Conductor on Windows; grows as Windows is tried
    └── LLM/               # working docs
        ├── STATUS.md      # bridge between sessions
        ├── TODO.md        # pending work + future ideas
        ├── LONGTERM_TODO.md # the big features, a run of sessions each
        ├── PROJECT_OPUS.md # skeletal layout of the whole project
        ├── PROTOCOL.md    # server/client contract: the packets
        ├── REGION_MAP.md  # server/client contract: region.map, byte for byte
        ├── WRITINGSTYLE.md # my voice for public docs and comments
        ├── TEST_CHECKLIST.html # what's still to check on testing, with boxes to tick
        └── design/        # one markdown file per system or feature
```

Each component keeps two folders: `dev/` is where code is written, `build/` is
what the compiler makes. Runtime data for both components lives in one shared
`Content/` folder at the root. Files in it are named so it's clear who owns them
(`conductor_globals.cfg`, `*.scribe.log`). Don't create new top-level folders
without asking me.

Programs create `Content/` and any file they need there (config, logs, saves,
schemas) with sensible defaults when it's missing, and never crash because it's
absent. Conductor finds it through the `OPUS_CONTENT` environment variable, or
by walking up from the working directory until it sees a `Content/` folder, or
by creating `./Content` when neither works.

---

## Start of every session

1. Read `Documentation/LLM/STATUS.md`, `Documentation/LLM/PROJECT_OPUS.md`, and
   `Documentation/LLM/TODO.md` to get your bearings. Read anything in
   `Documentation/LLM/design/` that touches today's work.
2. Re-read any source file before editing it. I sometimes hand-edit files
   between sessions, and my edits are the master copy. Never overwrite my
   changes with an older version from memory.
3. Tell me in a couple of lines where things stand, then ask what I want to work on.

## During a session

- **How we work.** I steer: I say what I want Opus to do. Your job is to make
  sure you understand what I mean, then write it out. When something I ask for
  could mean more than one thing, ask before building, and say what each
  reading would mean in practice. Don't fill gaps with guesses.  (2026-10-01:
  "a lockout on a character for like 1 second" was built as a lock on
  leaving, and I meant a lock on loading.  Say back where a thing sits and
  what it does to whom before writing it.)
- **The test checklist.** `Documentation/LLM/TEST_CHECKLIST.html` is the rolling
  list of what to check on `testing`, so that once game features come there's a
  reminder of what changed and what to look at in game.  Every session that
  changes what Conductor does adds its checks there, and the reply that pushes
  to `testing` points at them.  Once I say a check passed, it comes out of the
  file.  It's a page I open from the disk: a check is an `<li class="check">`,
  a command a one-line `<pre class="cmd">` that gets a COPY button.  My ticks
  live in my browser only; I say which passed (each tick writes it into the
  page's message, and COPY MESSAGE copies it).  A tick is keyed by the check's
  words, so rewording a check drops its tick.  A tab left open from an earlier
  session shows that session's checks until it's reloaded: if a message names
  checks the file no longer has, say so and ask for a reload before touching
  anything.  The page's count ("8 of 9 passed") includes the Parked checks.
- **Small increments.** Each conversation takes one small step, so the branch,
  the commits and STATUS.md read as a running history of what happened and why.
  If a step grows, stop at a sensible point and leave the rest for another
  conversation.
- **Design talk is written down as it settles.**  Each answer goes into its
  TODO.md entry (or design file) and is pushed as it comes, not saved for the
  hand-off, so a lost chat loses nothing.  Design is talked through here, with
  the docs at hand; a brief for a separate chat was tried once (the ECS) and
  the other chat lost the thread.  I may be writing a GDD with another chat
  (2026-09-30); what it settles comes in through me, and goes into these docs
  like any other answer.
- **One feature per session.** If a new feature comes up mid-session, add it to
  `Documentation/LLM/TODO.md` and keep going on the current one. It gets its own
  session later.  When one is done, say so and offer the hand-off; if I say
  keep going, the next one starts in the same session (2026-09-30 took four:
  "we're not even at 40% token use"), still planned, OKed, built and checked
  on its own, with its own commits and its own paragraph in STATUS.md.
- **Plan before building** anything bigger than a small fix: tell me the files
  you'll touch and the approach, and wait for my OK.  Check the plan against
  the rules here first, and say when an ask runs into one; I may turn the ask
  round once I see why.
- **Things that can't be done yet** (because a dependency isn't built) go in
  `Documentation/LLM/TODO.md`, not half-implemented in code.
- **Future ideas** that come up in conversation also go in `Documentation/LLM/TODO.md`.
  A feature that's a run of sessions on its own goes in
  `Documentation/LLM/LONGTERM_TODO.md` instead.
- **I'm hands-off on the files in `Opus/`.** You make every edit, CLAUDE.md
  included. Don't hand me a list of changes to make by hand; make them and tell
  me what changed.
- **I build, run, and test everything myself** and paste back the output.
  Do not run `cargo check`, `cargo build`, `cargo test`, the server, or the
  client. Stick to writing the code. When it's written, tell me exactly
  which commands to run, with your questions at the bottom of the reply (see
  "How to talk to me"). I paste back what happens and we go from there.
  Conductor is run from a terminal, not from inside RustRover: on
  2026-10-01 RustRover's code analysis running beside the server locked
  the whole machine up, and from a terminal it ran clean.
- Do not predict or number future sessions ("next session is X, then Y").
  I pick what to open next and I'm free to change my mind.  When I lay
  out an order myself, it's written down as mine, in my words, under
  "Jacob's map" in STATUS.md, so I don't forget it, and it's still mine to
  change.

## End of every session (hand-off)

When I say we're wrapping up:

1. Update `Documentation/LLM/STATUS.md`. It holds only the last session plus any
   earlier session that directly matters for the next one. It is a bridge, not
   a rolling log.  List what's waiting, unordered.  If the session's code
   hasn't been built by me yet, STATUS.md says so plainly, so the next session
   starts by expecting compile fixes.
2. Update `Documentation/LLM/TODO.md`, `Documentation/LLM/PROJECT_OPUS.md`, and
   any `Documentation/LLM/design/` files the session changed, so they match
   reality.  Add the session's checks to `Documentation/LLM/TEST_CHECKLIST.html`:
   what to run on `testing` to see the change working, and what to look at in
   the game once there is one.  A check that passes is taken out, not struck
   through; the file is my reminder of what's left, not a history (git keeps
   the old ones).
3. Update `README.md` if anything about the project's overview or where it
   stands changed.
4. Update this CLAUDE.md with anything the session taught us, and tell me what
   changed.
5. Make no code changes during hand-off unless there's a glaring bug, and if so,
   tell me first.

---

## How to talk to me

- I'm an amateur hobbyist. Comfortable in C and C#, still learning Rust.
- Explain Rust plainly. Don't translate Rust into C terms unless I ask.
- **Questions for me go at the bottom of your reply, under a big, loud
  header** (`# >>>>>>>>>> QUESTIONS FOR JACOB <<<<<<<<<<` -- large and annoying
  on purpose, so I can't miss them).
- Keep replies short at first. Expand when I engage.
- **Every command I'm to paste is one line.**  Never wrapped with `\` over
  several lines; pasting a wrapped command breaks it.  Long is fine.
- **My terminal sits in `Conductor/dev`**, where cargo runs.  Every command
  works from there: absolute paths (`/opt/storage/Coding/Opus/...`) for
  anything outside it, never a relative path with "from the repo root" in
  front.  One of those once made a second `Content` inside `dev/`, and
  Conductor found that one first.
- When you change files, end with a short list of which files changed and why.

---

## Code rules (all components)

- Simple and readable over clever. If there's a clever way and a plain way, use
  the plain way.
- **All time is UTC.** Any time shown to a person ends with `Z`.
- **Most log lines are Debug.** Routine things (loaded a file, connected, ran
  the schemas, a job finished) go to Scribe as Debug. Info is for the few
  milestones an admin cares about (starting, shutting down, a service coming up
  or going down). Warn and Error are for things that are actually wrong. A
  switch in the config turns Debug lines off, so a finished server's log reads
  clean instead of chatty.
- Ask before adding any new dependency (crate, package, plugin). Minimal
  dependencies is the default.
- No references to AI, Claude, or assistants anywhere in source code, comments,
  the README, or commit messages. `CLAUDE.md` and `Documentation/LLM/` are the
  only exceptions. (The repo is private today, but keep the code clean in case
  any of it is ever shared.)
- Comments and public docs are written in my voice per `Documentation/LLM/WRITINGSTYLE.md`.
- Line width: 120 columns; comments wrap at 78. Details in `Documentation/LLM/WRITINGSTYLE.md`.
- **Every file starts with a header** saying where it lives, as a path from the
  repo root beginning with `Opus/` (never the full drive path), which component
  it belongs to, and who wrote it, then a line or three on what the file is for.
  When a file moves or is renamed, its `File:` line changes with it.

  Rust (`//!` so it shows up in `cargo doc`):
  ```rust
  //! File:       Opus/Conductor/dev/src/main.rs
  //! Component:  Conductor
  //! Author:     Jacob Chacko
  //!
  //! What this file is for, in a sentence or three.
  ```
  C#:
  ```csharp
  // File:       Opus/Ensemble/dev/Assets/Scripts/Net/Connection.cs
  // Component:  Ensemble
  // Author:     Jacob Chacko
  // What this file is for (only if the name doesn't already say it).
  ```
  TOML, `.gitignore`, shell scripts and `.conf` files use the same lines with `#`.
  Markdown uses an HTML comment at the very top, so it doesn't show when rendered:
  ```markdown
  <!--
  File:       Opus/Documentation/LLM/STATUS.md
  Component:  Documentation
  Author:     Jacob Chacko
  -->
  ```
  The only files without a header are ones that can't hold comments or that a
  tool generates and rewrites: JSON, Unity's `.meta` / `.unity` / `.asset` /
  `.prefab` files, lock files, and anything under `build/` or `Content/` (the
  schema files and configs in `Content/` included).

## Rust rules (Conductor)

- **Never use `#[allow(dead_code)]`** or any other lint suppression. I would
  rather see the warnings.
- **Benchmarks run with `--release`.** `cargo test` builds unoptimized, and
  unoptimized Argon2 read six times slow. A timing test is an `#[ignore]`
  test run by hand, and its doc comment gives the exact command.  A cost
  given in a plan is a guess until a timing test measures it, and the
  plan says so (the world save's snapshot was guessed at "a few
  milliseconds" for 10,000 characters and measured at 33).
- Whenever you create a new crate, say explicitly whether it is a **bin** or a
  **lib**.
- **A crate's folder drops the `conductor-`; the crate keeps it.**  The
  folder is `Conductor/dev/tools/`, the crate in its `Cargo.toml` is
  `conductor-tools`, and code says `conductor_tools::`.  So `-p` in a cargo
  command takes the crate's name (`cargo test -p conductor-tools`), and a
  `path = "../tools"` takes the folder's.  The crate keeps it because in code
  the `conductor_` prefix tells our crates apart from everybody else's.
- Prefer clear ownership and simple types over heavy generics or macros.
- `conductor-launcher` is the program. New server pieces (networking, the game)
  are lib crates, not programs of their own.
- **Conductor and the server are two things.** The program (DiskMan, Scribe,
  Constellations, the web admin) is up from boot. The server (Fingerprinter,
  Security, Archivist, the account desk, Lua, GameWorld, the GameClock, the
  monitor, networking, and the rest of the game as it comes) only runs between
  START SERVER and STOP SERVER on the web admin's Server tab: Conductor comes
  up with its door closed, and the admin opens it (and closes it) from there.
  The launcher does the calling, so a new server piece goes in both
  `start_server()` and `stop_server()` in the launcher (networking's start is
  the one exception, below), and has to be able to
  stop and start again in the same run. The switch's state lives in
  `server.rs` in `conductor-tools`.
- **The door waits on the world.**  Networking isn't started in
  `start_server()`: the launcher's `take_commands()` starts it, TCP and UDP
  both, once `conductor_gameclock::ready()` says the chunks around 0,0,0 are
  in, since nobody gets in before there's a voxel to step on.
- **Two kinds of restart.** A *soft reboot* is STOP SERVER and START SERVER
  (or RESTART SERVER) on the Server tab: the server pieces come down and back
  up, the launcher and the page stay put. A *hard reboot* is Conductor, the
  whole program, run again.  Every config file is one or the other (see
  Constellations below).
- Anything that can be slow (database, disk, network) runs on its own thread,
  and callers get the answer back later (Archivist's `Pending`). The game loop
  never waits on it. No async runtime.  **Archivist doesn't log a job that
  fails**: the failure is in its `Pending` and nowhere else, so a caller that
  drops one never hears.  The GameClock keeps its saves' `Pending`s and looks
  at them in housekeeping without waiting.
- **Every thread goes through `threads::spawn(name, ...)`** in `conductor-tools`,
  never `std::thread::spawn` directly. That is what puts it on the web admin's
  "asked for" list with who started it and when.
- **Every service reports to `services.rs`** in `conductor-tools`: it's named
  in `EXPECTED` up front, says starting / running / trouble / stopped with a
  note, and checks in with `seen()` if it has a loop. That is what puts it on
  the web admin's Services tab.  A new service goes in two places there:
  `EXPECTED`, and the list in `every_expected_service_is_there_from_the_start`
  (missing the second fails `cargo test`).
- **Every file read and write goes through DiskMan** (`diskman.rs` in
  `conductor-tools`), never `std::fs` directly: `write()` for a whole file
  (temp file and rename), `append()`, `read()`, and `stream()` for big ones.
  Only folders (making one, listing one) stay with `std::fs`. DiskMan starts
  first and stops last.  A file it holds is checked against the disk's
  modified time and size on every read, so a hand edit is read.
- **DiskMan never logs routine work.** A log line is itself a DiskMan write, so
  a "wrote a file" line would loop forever. It logs failures only, and never
  while holding its own lock.
- **Every config file is Constellations'** (`constellations.rs` and
  `constellations/` in `conductor-tools`).  All of them live in `Content/cfg/`,
  fixed relative to Opus (there will never be a setting that moves the config
  folder), in one format (`key = value`, `#` comments), and every one is
  written down once in `constellations/files.rs`: its settings, their kinds,
  defaults and comments, and whether the file is **soft** or **hard**.  A file
  is one or the other as a whole, never a mix; a piece that needs both gets two
  files, and files are kept separate rather than one file with sections.
  - Soft (`postgres.cfg`, `networking.cfg`, `game.cfg`) is read on every START
    SERVER, so STOP SERVER and START SERVER applies a change.
  - Hard (`conductor_globals.cfg`: anything about Constellations, Scribe or the
    web admin's port; `wgui.cfg`: the web admin's accounts) is read at boot, so
    Conductor is shut down and run again.
  - Adding a setting is one entry in the table and a line wherever it's read
    (`constellations::value()` and friends); a piece never reads a config file
    itself.  A change from the web admin goes to `name.cfg.wait4server` beside
    the live file (`save_waiting()`), and DiskMan swaps it in when the reboot
    comes; the live file always says what Conductor is running on.
  - **The one exception**: the whitelist and the blacklist are
    `Content/cfg/whitelist.cfg` and `blacklist.cfg`, one address or range a
    line, not `key = value`, and not in Constellations' table (they have their
    own tabs, not the Settings tab).  `networking.cfg` points at them.  They're
    read on every START SERVER, and a change from the page takes at once and
    rewrites the file.  **Nothing else in Conductor hot swaps.**
- **Every password hash goes through Security** (`security.rs` in
  `conductor-tools`): `hash_password()`, `verify_password()` and
  `verify_no_account()`, each handing back a `Ticket` (the `Pending`, plus
  `place()` for how many are ahead and about how long).  One worker thread,
  one arena of memory kept for the server's life, one login hashed at a
  time, hard limit, with everybody else in line.  Nothing else calls the
  argon2 crate, and nothing ever logs a password.
- **Every account goes through `conductor-accounts`.**  Nothing else writes
  SQL for the `accounts` table or the `player_characters` table (making or
  deleting a character writes both, so one crate writes them, in one
  transaction; `characters.rs`).  A character is three to an account, in
  the first empty slot, and only the player deletes one.  Protogame and the game library call its
  functions.  **An account is never held in memory**: whatever needs one loads
  it from its row when it needs it (`load()`, `list()`), and every change goes
  straight back to the row, so there's one copy and nothing writes an old one
  over a new one.  Networking's book has the account's name and nothing else.
  `Account` never has the password hash (`password_hash()` reads that on its
  own).  The last login is written the moment the player connects over UDP
  (`stamp_login()`), not at the TLS login, since that's when playing starts.
  Accounts are made by the admin on the web admin's Accounts tab, never by
  players, and only while the server is running.  Anything that hashes a
  password for the web admin goes through the account desk (`desk.rs`), so the
  web admin's one thread never waits in Security's line.
- **Every Warn and Error becomes a notice** on the web admin's bell, and stays
  there until I ACK it. So a Warn is for something actually wrong, never
  chatter. Code can raise one on purpose with `notices::publish()`.
- **The admin works through the web admin** (`conductor-wgui`). The console is
  only Scribe's output and takes no input. Anything an admin can do (start and
  stop the server, shut down, accounts, settings, the access lists) is a page
  or a button there. It listens on `127.0.0.1` only. Never suggest binding it
  to anything else.
  - **Every new route goes through `/Opus/wwwhook/`** (my name for a path the
    page posts to that makes something happen; the shutdown and ACK routes
    predate it and kept their paths).  **Ask before adding one.**  When a reply
    says "route" it names the whole path (`/Opus/wwwhook/tcp/kick`), so it's
    clear it's a web admin path, not a file or a function.
  - Already agreed to: starting, restarting and stopping the server
    (`/Opus/wwwhook/start`, `/stop`, `/restart`); kicking a TCP connection or
    the player its login became (`/Opus/wwwhook/tcp/kick`); the access lists
    (`/Opus/wwwhook/networking/addip` and `/removeip`; "add" and "remove"
    alone were too generic); the Settings tab (`/Opus/wwwhook/settings/save`
    and `/discard`); the Accounts tab's changes
    (`/Opus/wwwhook/accounts/create`, `/edit`, `/password`, `/delete`, with
    its reads under `/Opus/Content/accounts`); the Characters tab's read
    (`/Opus/Content/characters`).

### Lua, the game library, the world and the GameClock

- **The scripting language is Lua 5.4**, embedded through the `mlua`
  crate with Lua built in.  A script never gets Lua's `io` or `os`
  libraries, or anything else that reaches the disk, the network or the
  database; it asks the game, and the game decides.  It lives in
  `lua-parser` (`sandbox.rs` is the lock).  **Whatever a script says is
  wrong, and every error in one, is a Warn, never an Error**, and nothing a
  script does can take down what's underneath it: a time limit, a memory
  limit and a cap on its log lines, each a Warn when hit.
- **The game library is `conductor-primlib`**, an ECS written by hand, no
  crate: an entity is a slot and a generation, a component is a plain struct
  with a `Store` in the `World`, and objects are made from a **template**
  (`NPC`: components and defaults) through a **blueprint** (`goblin_a`: its
  own changes).  A template can take in another whole (`Character` takes in
  `Living`, my `inherit STD_LIVING;`), and every copy remembers the templates
  it came from (`world.is(entity, "Living")`).  A new kind of component goes
  in `components.rs` (the struct with its `saved()` and `load()`, `Kind`,
  `Kind::ALL`, `name()`, `Component`, `kind()`, `default_of()`, and
  `Component`'s `saved()` and `load()`) and in `world.rs` (a store, the four
  matches, two getters).  The world lives in memory and the game loop never
  waits on the database: copies are written to it on STOP SERVER and read
  back on START SERVER.
- **What's saved is picked per field**, the plain way: `saved()` right under
  a component's struct names the fields a save keeps, and a field it doesn't
  name isn't saved.  A GameObject is saved as Lua text (`primlib/src/save.rs`)
  and read back by `lua-parser`'s `read_save()` in the locked-down Lua; a
  character is made from the Character template with its save laid over it
  (`gameobject.rs`).
- **The ground is `conductor-gameworld`** (`design/world.md` has all of it).
  **A block is 1 m a side**, Minecraft's size (2026-10-01; it was 50 cm),
  chunks 32 a side, eleven chunks tall, -32 to +319, BEDROCK at -31 and
  -32.  **The world's size is `world_size` in `game.cfg`**, 1024 blocks
  a side per step, 2 to 32, 16 to start (2026-10-01); a change deletes
  the world on disk and makes a new one at the next START SERVER.  The code counts in blocks only and never
  in metres, so a change of size is the numbers and what the docs say
  they mean.
  GameWorld's thread does the slow part (making the world, reading
  `region.map`, the heights and chunk files through DiskMan, building a chunk
  nobody changed); the chunks in memory are the GameClock's `Terrain`,
  touched only on its thread.  A block's number never changes once it's out
  there (AIR 0 to BEDROCK 5).  A chunk's own file always wins over its
  region's ground, and a bad file is never built over: it may be the only
  copy of somebody's digging.
- **`REGION_MAP.md` is `region.map`'s contract**, byte for byte, the same way
  PROTOCOL.md is the packets'.  `gameworld/src/regionmap.rs` is written from
  it; a change to the layout bumps the file's version, and the code and the
  document change together, with a line in its version history.
- **The tick is 250 ms, five checks of 50 ms, fixed in code** ("anything
  faster is gonna be a problem.  Slower is fine but faster becomes bad").
  Never a setting.  It's run by **the GameClock**, `conductor-gameclock`.
  Jacob's "tick" is one of these 250 ms cycles, not a 50 ms check.
  The checks run in order (input, AI, movement, broadcast, housekeeping),
  each timed from the cycle's start; a late one makes the next late and
  nothing is skipped.  Only the GameClock's thread touches the `World` and
  the `Terrain`.  Until the ground around 0,0,0 is in
  (`conductor_gameclock::ready()`), only housekeeping runs: no NPC acts
  before there's ground under it.  I may call it "the heartbeat" (my MUD's
  word for the tick), but it "doesn't seem professional" for Opus, so the
  code and docs say the GameClock.
- **Players' characters come into the world through the GameClock's
  mailbox** (`gameclock/src/players.rs`): `conductor_gameclock::enter()`
  with a finished blueprint (the slow part, the row and the save, done on
  the caller's thread) and `leave()`, which saves the character and
  despawns it at once.  Nothing is left standing in the world for a
  reconnect.  `leave()` marks the character "saving" until its leaving
  save has landed (`saving()`, `wait_until_saved()`), so nobody brings it
  back in on the save before.
- **The world save is global** (Jacob, 2026-10-01): every
  `world_save_seconds` (`game.cfg`, 150, from 30 to 1800), counted from
  `ready()`, one cycle copies every player's character (`Save::of()`, plain
  data, no text, no database), and Archivist turns each into Lua text and
  writes it, all in one transaction (`characters::save_all()`), so the
  database holds the world at one moment or the save before.  The GameClock
  only ever copies; "I'm just worried about blocking or lagging the regular
  tick".  STOP SERVER saves the world one last time.

### Networking (conductor-networking)

- **TCP is only the login; UDP is everything after.**  A client logs in over
  TLS on the TCP port, gets a ticket (a one-time token and the UDP port),
  and the TCP connection closes.  When the UDP session ends, for any
  reason, the player is gone and the client starts over at the login
  screen.  Nothing is kept for a reconnect.
- **Lower CPU, more RAM if it buys that.**  My steer for this crate.  A
  fixed pool of login threads, no thread per connection; no polling loop
  anywhere, every wait is the OS's own; players found by address in a map;
  fixed answers built once.  One login is hashed at a time, always, through
  Security's line.
- **`PROTOCOL.md` is the contract**, byte for byte.  `protocol.rs` and the
  test client are written from it; when either disagrees with the document,
  the code is what gets fixed.  A packet change bumps `PROTOCOL_VERSION`,
  and so does a new value in a packet's enum (each new Kicked reason did):
  `protocol.rs`, PROTOCOL.md and `test_client.py` all change together, and
  the document gets a line saying what the version added.  It's at 6
  (the spawn, 2026-10-01).
- **Character select is Protogame's** (`protogame.rs` in networking, its own
  thread and Services line): "the character selection and character
  construction are proto game then become game objects after load".  The
  UDP thread hands it each ask and never waits on the database; every ask
  carries a u32 ask number, and the book keeps the last answer, so a lost
  answer is sent again, never the ask done twice.  CommandAccepted and
  CommandRefused are the general answers, for reuse.
- **The spawn is Protogame's too** (2026-10-01, protocol version 6):
  `UserPressPlay` picks a character, Protogame reads its row and save and
  hands it to `conductor_gameclock::enter()`, and the client gets
  `CharacterEnteredWorld`.  Every way a player leaves the book takes their
  character out of the world and saves it.  **There's no way back to
  character select from the world**: "you log out back to log in screen
  every time", camping out included.
- **A character is locked both ways** (`sessions.rs`): for 1 second when
  it starts loading, and for 1 second when it leaves the world and as long
  after as its save is on its way.  A pick of a locked character gets
  Kicked, reason 6, and the client logs in again.  A login that logs the
  other session out waits for that session's character's save before its
  ticket goes out, up to 5 seconds, then Login Unavailable.
- **The TLS pair is made by hand** with the openssl command in README.md, in
  `Content/certs/`.  The key is gitignored, the certificate committed.
  Conductor never makes one and never crashes without one: the Services tab
  says it's missing and the log says the command.
- **`test_client.py`** beside the crate is how networking is tested until
  Ensemble exists.  Python 3, standard library only.  I run it and paste
  back what it prints, the same as the server.
- **Client management** (a player limit, reconnect tokens, messaging a
  player from the web admin) is not this iteration.  It's in TODO.md.
  Kicking from the web admin is built: KICK on the Connections tab.
- **The server decides what each client sees.**  Each player's packets
  carry only what that player may see; nothing is sent for the client to
  hide, since a changed client can read anything it's sent.
- **The access lists** (`access.rs`) are the one thing that changes without
  a reboot.  A change from the page is enforced the same moment: `enforce()`
  asks the door's verdict again for everybody online and drops whoever it
  now turns away, so a whitelist removal is a ban as much as a blacklisting.

### Linux and Windows

- Development is Linux-first: Linux is where Conductor is built, run and
  tested. Windows code stays wired in behind the same functions.  It builds
  and runs on a Windows laptop (Rust's MSVC toolchain; see
  `Documentation/HowTo/WINDOWS_INSTALL.md`), START SERVER included, but
  hasn't had a database there: the laptop has no Postgres.  macOS isn't a
  target; any OS but those two gets the fallback file.
- There is no "which OS" setting in the config. The compiler knows what it is
  building for, and `#[cfg(target_os = "linux")]` / `#[cfg(windows)]` pick the
  code. OS-specific code gets one file per OS behind a common set of functions
  (see `monitor/src/probe/`), plus a fallback file for anything else
  that builds and reports "not measured here yet" instead of failing.
- Talk to the OS through what it already has, not a crate: `/proc` files on
  Linux, kernel32 (and ntdll) through an `extern` block on Windows. `unsafe`
  lives only in those OS files, each block with a comment saying why it holds.
  Ask before reaching for a crate like `sysinfo` or `windows-sys`.
- Paths are built with `Path::join`, never by gluing strings with `/` or `\`.
- Anything that can only be tested on the other OS gets said so in the reply,
  with the commands to run it there. Getting a build onto my Windows machine
  is a hassle, so Windows code can sit untested for a while. STATUS.md says
  so for as long as it does.

### The web admin's page

- `page.html` and `json.rs` are two halves of one contract: the JSON's shape is
  written at the top of `json.rs`, and a change to one needs the other.
- No made-up numbers on the page. If there's nothing behind a panel yet, the
  panel waits.
- The page pulls nothing from the internet, and puts our data in with
  `textContent`, never `innerHTML`.
- I look at the page myself and send screenshots. When a layout class or style
  is added, make sure the CSS for it exists.
- Checking `page.html` by rendering it in a headless browser with made-up
  numbers is fine (it isn't running Conductor). Say that's all it was.
- **The page is five sections across the top**, and a side menu listing the
  open section's tabs:
  - CONTROL PANEL: Server, System, Conductor, Services, Storage
  - CONFIGURATION: Settings, Whitelist, Blacklist
  - LOGS: Log, Notifications History
  - ACCOUNT MANAGEMENT: Accounts
  - GAME MANAGEMENT: Connections, Characters

  A tab says its section with `data-section` in `page.html`.  Anything new
  goes on one of the tabs, or is a new tab (or section) I agree to.  The
  notices are the bell and the LOGS section: the tray has HISTORY, which
  opens Notifications History.  (If I say "the Control Panel" I may mean the
  Server tab; it had that name before the sections.)
- **The Accounts tab** is the game's accounts, `admin` only (`user` can't see
  the list).  The list, and a card per account opened by clicking its name:
  the owner's names and email (SAVE), a new password typed twice (CHANGE
  PASSWORD; every password is typed twice), and DELETE ACCOUNT, which takes a
  player in the world out with Kicked, reason 5, and the client says ACCOUNT
  TERMINATED.  NEW ACCOUNT opens a card for a new one.  The username never
  changes.  Locked unless the server is running and the database connected.
- **The Connections tab** is the door and the world, TCP first then UDP:
  every connection that reached the TCP listener since START SERVER, by
  address and DNS name, never by account, with where each one is (the queue,
  TLS, Security's line with its place, finished and how; a login whose player
  has left the world, or never came, reads LINKDEAD and why) and a three-dot
  menu for `admin` (KICK, add the address to the whitelist, add it to the
  blacklist), in two views, Recent (the newest five) and Historical (the
  whole run); then every player over UDP, by account, with the character
  they're playing ("character select", greyed, until they pick one), when
  they connected and how quiet they are.
- **The Characters tab** (2026-09-30) is every player's character, look
  only, for `admin` and `user` both: name, UUID, x, y, z (as of its last
  save) and account.  Asked for when the tab opens and on REFRESH, never
  once a second.  Editing characters and NPCs from GAME MANAGEMENT is to
  come (TODO.md).
- **The Whitelist and Blacklist tabs** are the two access lists: the entries,
  REMOVE on each, an ADD field.  A change takes at once and writes the file.
  Which list the door checks is `access_list` in `networking.cfg` (off,
  whitelist or blacklist), on the Settings tab, and takes on the next START
  SERVER.  A blacklisting while the blacklist is on is a ban, and so is taking
  an entry off the whitelist while the whitelist is on: every connection and
  player the door would now turn away is dropped at once, the player with a
  Kicked (reason 3, banned).  KICK in a TCP row's three-dot menu kicks an open
  connection, or the player its login became (reason 4, kicked by the admin),
  greyed when nothing is left to kick.  Connections, Whitelist and Blacklist
  are locked until both of networking's listeners are up; the lists can't be
  changed from the page while the server is stopped (edit the files by hand
  then).
- The Server tab, the Log and the Settings are always clickable. Until
  the server is running they're the only tabs that are, and the bell is
  hidden. The Server tab is the only place the server is started,
  restarted and stopped, and the only place SHUT DOWN is; the header has
  the bell at the top right and the sections under it. LOG OUT is at the
  bottom of the side menu.
- While the server is running and the database isn't connected, the data
  tabs are blurred and locked. Anything new on the page sits under that lock;
  only the header (the bell, its tray and the sections included), the
  Server tab, the Log and the Settings stay above it.
- **The page has a login.** Two accounts, fixed: `user` looks and touches
  nothing, `admin` does everything. Their passwords are the two settings in
  `wgui.cfg`, as they are, not hashed (Security only runs with the server,
  and a login has to work before START SERVER). A login card covers the page
  until you're in; every start of Conductor starts logged out, and a page
  reload doesn't. Every route but the page and `/Opus/login` needs the
  login's cookie (401 without), and every route that changes something needs
  `admin` (403 to `user`): a new route that changes anything goes behind
  `only_admin()` in `lib.rs`, and the page greys its button for `user` in
  `lockChanges()`. `login.rs` holds it.  **No idle timeout, ever**: the login
  is about roles (who may change the server), not security; the page only
  listens on this machine.  `wgui_port` is moving from
  `conductor_globals.cfg` into `wgui.cfg` (in TODO.md until it's done).
- **The Settings tab is the config editor**, drawn from Constellations'
  table through `/Opus/settings`. A new config file or setting shows up there
  with no page work. A save goes to `.wait4server` and takes at the file's
  reboot; the tab never hot swaps anything.
- **Mockups go on a canvas, not in the repo.**  A new layout for the page
  can start as a few clickable mockups on a claude.ai design canvas, in
  the page's own colours with the real tab names, for me to pick from
  and comment on.  They stay there; only the one I pick is built into
  `page.html`.
- When talking about the page, name the panel or tab ("the Log tab"), not the
  tool behind it. "Where does Scribe go?" read as moving the crate.

## Database (Conductor)

- PostgreSQL 18, running locally on my dev machine. It listens on `localhost`
  only. Keep it that way: never suggest opening it to the network.
- Database: `opusdb`. Tables live in the `public` schema.
- Conductor connects as the role `opus_game` over TCP to `localhost:5432`
  with password auth (`scram-sha-256`).
- **Tables are created as `opus_game`**, so the server owns them. `seliris`
  owns the database itself but should not own game tables.
- Never hardcode the password in source. Archivist (in `conductor-tools`)
  reads the address, port, database, username and password from
  `Content/cfg/postgres.cfg`. That file is committed on purpose: the password
  is a placeholder and Postgres only listens on this machine.
- The Postgres crate is `postgres` (the blocking client). Archivist runs it on
  its own thread so it never blocks the rest of the server. Ask before adding
  any other database crate.
- Do not run `psql`, migrations, or anything that touches the live database,
  and never edit Postgres's own config (`pg_hba.conf`, `postgresql.conf`).
  Write the SQL; I run it and paste back the output.
- Default schemas live in `Content/psql/defaults/schemas/`, one `.sql` file
  per table. They only `CREATE ... IF NOT EXISTS`, and Archivist runs them on
  every connect. Each one is also baked into Conductor with `include_str!`
  (listed in `DEFAULT_SCHEMAS` in `archivist/schemas.rs`) so a missing file gets
  written back out.
- **A schema file is frozen once its table exists.** Every change after that
  is a migration in `Content/psql/migrations/`, named `0001_what_it_does.sql`.
  Archivist runs each one exactly once, in number order, in a transaction, and
  records it in the `archivist_migrations` table. Never edit a migration that
  has already run; write a new one.  A new schema file can still change until
  its table is made: before changing one that's been pushed, ask me to look
  (I check in DataGrip).  If the table's there, the change is a migration.
- **No game data is without an `id` and a `uuid`** (Jacob, 2026-09-30: "no
  gamelib data is without an id or uuid").  Every table that holds the game's
  data (accounts, characters, primlib's copies, anything the game names) has
  both.  The one exception is Archivist's own bookkeeping,
  `archivist_migrations`, keyed by the migration's number, with a `uuid` and
  no `id`.
  - `id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY` is the table's own
    number, local to that table. Tables point at each other by `id`.
  - `uuid UUID NOT NULL UNIQUE DEFAULT uuidv7()` is the game's name for the
    row, unique across the whole server. It's what the game, the client, the
    web admin and the logs use.
  - UUIDs are version 7: the time first, so they sort in the order they were
    made. Conductor hands over one from Fingerprinter (`new_uuid()`) on every
    insert; the default is only a safety net.
  - A new table has both from its first schema file. The `postgres` crate
    can't send a `String` into a UUID column on its own, so the SQL casts:
    `$1::text::uuid` going in, `uuid::text` coming out.

## Client rules (Ensemble)

- **Our folders under `Assets/`** (Jacob, 2026-10-01): `Assets/Editor/` for
  the editor plugins, `Assets/Code/` for the plain C# (the networking, say),
  `Assets/Scripts/` for the scripts, `Assets/Data/` for our own data files
  (the screens' layouts in `Assets/Data/Layouts/`, their stylesheets in
  `Assets/Data/Styles/`).  Those four, and their `.meta` files,
  are the only part of `Assets/` that's committed (the root `.gitignore`);
  everything else in `Assets/` is the purchased art and what Unity makes
  from it, and stays on Jacob's machine.  So purchased art, and anything
  copied out of it, never goes in those four.  Jacob keeps the purchased
  art in `Assets/Purchased/`, and art copied out of it (the animations)
  under `Assets/Art/`.  A new folder of ours under
  `Assets/` goes past Jacob first, and into the `.gitignore` with its
  `.meta`.
- Unity makes a `.meta` beside every file and folder.  A session can't run
  Unity, so a new file's `.meta` comes from Jacob's machine: the reply gives
  him the `git add` (never skipped: a commit without it commits nothing),
  `git status --short` to look at before committing, `git commit`,
  `git pull --no-rebase --no-edit` and
  `git push origin HEAD:testing HEAD:unstable`, once Unity has compiled
  (it imports, and makes the `.meta`s, when its window gets focus).  The
  add is always the two folders, never a list of files:
  `git add -A /opt/storage/Coding/Opus/Ensemble/dev/Opus.Ensemble/Assets /opt/storage/Coding/Opus/Ensemble/dev/Opus.Ensemble/ProjectSettings`.
  The `.gitignore` keeps out all of `Assets/` but our four folders, so
  that's exactly ours.  A list of files fails whole when one isn't there
  yet (2026-10-01), and `-A` on the whole project swept in what Unity
  makes beside `Assets/` (`UIElementsSchema/`, the `.sln`).  Unity
  rewrites `ProjectSettings.asset` now and then, and writes it only on
  File > Save Project or on closing, which is why a setting changed in
  Player Settings can be missing from a commit.
- An editor plugin is a menu item under **Tools > Opus** (the first,
  2026-10-01, is Tools > Opus > Copy Anims From FBX Pack).
- **Jacob runs Unity**, the same as Conductor: the session writes the C#, he
  opens the editor and pastes back the Console.
- [More conventions as Ensemble grows]

---

## Git rules

- **Three branches.**
  - `unstable` is where you write. Every session commits and pushes here.
    **First thing every session: `git fetch origin` and make sure the work
    starts from `origin/unstable`'s tip**, whatever branch the session was
    opened on (a session cut from `main` once started eleven commits behind).
  - `testing` is where I test. **When a round of edits is done and you want
    me to test it, push `unstable` onto `testing` yourself** (a fast-forward:
    `git push origin unstable:testing`), then tell me what to run. Don't wait
    to be asked.
  - `main` is the stable release. It moves only when I say so, from
    `testing`, never from `unstable`. Never push to `main` on your own.
- The session-branch-and-pull-request way (each session on its own branch,
  merged into `main`) is over, and the generated branch a session opens on is
  never pushed to.
- If I've pushed to `unstable` from my machine, fetch and merge it before
  pushing. Never rebase or force-push over my commits, on any branch.
  The same goes for `main`: I sometimes commit there from my machine
  (`bind_address = 10.0.0.84` in `networking.cfg` is one, my machine's
  address and correct).  When `main` has a commit `unstable` doesn't, merge
  `main` into `unstable`, so the next release is a plain catch-up.
- Deleting a branch on GitHub can't be done from the session (the push is
  refused), so I do that by hand when one is finished with.
- **On the Windows laptop, git is Git GUI** (a clone in
  `C:\TEMP\download2`).  Git steps for that machine are Git GUI's menus
  (Remote > Fetch from > origin, Branch > Checkout), with the one-line
  command beside them.
- **Tell me explicitly when to touch git from my terminal, and give the exact
  commands.** I don't keep the branch model in my head; you do. Every time
  one of these happens, the reply says so in a line of its own, with the
  commands to paste:
  - You've pushed to `testing` and it's my turn to test: `git fetch origin`
    and `git checkout testing` (or `git pull` if I'm already on it), then
    the build and run commands.
  - You've pushed to `unstable` and I've hand-edited or built on my machine:
    `git pull` on `unstable`, so my copy matches before I do anything.
  - A build changed `Cargo.lock`: the `git add`, `git commit -m` and
    `git push` for it, with a `git pull --no-rebase --no-edit` before the
    push and the push naming both branches (`git push origin HEAD:testing
    HEAD:unstable`), since the session may have pushed again while I built.
  - I've said to release: the commands that move `main` to `testing`.
  - A branch needs deleting on GitHub: the `git push origin --delete` line.
  If nothing on my side needs doing, say that too ("nothing to run in git"),
  so silence never means I missed something.
- The whole `Opus/` folder is one **private** repo: code, docs, and assets.
  It must stay private -- it holds purchased art assets that can't be
  redistributed. Never suggest making it public or pushing it anywhere else.
- Never commit build output, logs, the game's save, purchased art, or engine
  caches: both `build/` folders, `Content/Assets/`, `Content/logs/`,
  `Content/world/`, Rust `target/`, Unity `Library/` `Temp/` `Obj/` `Logs/`.
  If something like that shows up in `git status`, tell me and suggest a
  `.gitignore` line.
- Large binary assets (models, textures, audio, `.blend`, `.unitypackage`) go
  through Git LFS. If you see one about to be committed without LFS, flag it.
- The hand-off ends with the work committed and pushed to `unstable`, and
  onto `testing` if it's ready for me to test.
