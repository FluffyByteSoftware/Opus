**EVERY SESSION PUSHES ITS CODE TO THE `unstable` BRANCH.  Never to a session
branch, whatever branch the session was opened on.  Jacob's rule, said again
on 2026-09-29 after a session pushed to its own generated branch.**

<!--
File:       Opus/CLAUDE.md
Component:  Opus
Author:     Jacob Chacko
-->

# Opus

Opus is the codename for this project: a multiplayer game made of a server and a
client. When I say "Opus" I mean this project, not the Claude model.

Opus is a separate project from Stratum and Mantle. Do not pull code from them
unless I explicitly ask. Their *workflow* carried over; their code did not.

Project root: `/opt/storage/Coding/Opus`

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
  - Engine / language: Unity 6000.6 (C#).  Jacob said so on 2026-09-29.  The
    project is on his machine and not committed yet.
  - Folder: `Ensemble/`
- **Documentation** -- project docs. `Documentation/LLM/` holds the working
  docs (status, TODOs, design, protocol) and is the source of truth for anything
  not in the code.
- **Shared protocol** -- whatever both sides need to agree on (message types,
  encoding) is documented in `Documentation/LLM/PROTOCOL.md` and is the single
  source of truth.

- **Soundcheck** (`Opus.Soundcheck`) -- the patcher, named 2026-09-30, not
  started.  It runs before the game and hands each client a certificate of
  its own, so the server can turn away any connection without one (mutual
  TLS).  LONGTERM_TODO.md has it.

If a future piece appears (tools, admin console, test client), ask me what
to call it before creating it.

---

## Folder layout

```
Opus/
├── CLAUDE.md              # this file
├── .gitignore
├── Conductor/             # server
│   ├── dev/               # source code -- a Cargo workspace
│   │   ├── tools/              # lib: the tools (DiskMan, Scribe, Constellations, Fingerprinter, Security, Archivist, ...)
│   │   ├── accounts/           # lib: the one way in to the accounts table, and the account desk
│   │   ├── monitor/            # lib: looks at the process once a second (RAM, CPU, disk, threads)
│   │   ├── networking/         # lib: the login over TLS on TCP, the game over UDP; test_client.py beside it
│   │   ├── lua-parser/         # lib (conductor-lua-parser): runs the Lua scripts in Content/scripts/, locked down
│   │   ├── primlib/            # lib (conductor-primlib): the game library -- entities, components, templates, blueprints
│   │   ├── gameworld/          # lib (conductor-gameworld): the ground -- blocks, chunks, regions and their files
│   │   ├── gameclock/          # lib (conductor-gameclock): the GameClock, the game loop
│   │   ├── wgui/               # lib: the web admin on 127.0.0.1, and the only way to shut down
│   │   └── launcher/           # bin: the program -- boots, then starts and stops the server on the Control Panel's say
│   └── build/             # compiled output -- never committed
├── Ensemble/              # client
│   ├── dev/               # source code
│   │   └── Opus.Ensemble/ # the Unity project.  Its Assets/, Packages/ and UserSettings/ are gitignored (Jacob, 2026-09-29)
│   └── build/             # compiled output -- never committed
├── Content/               # data both programs read and write -- committed, except Assets/, logs/ and world/
│   ├── Assets/            # purchased art -- never committed
│   ├── cfg/               # config files (conductor_globals, wgui, postgres, networking, whitelist, blacklist .cfg)
│   ├── certs/             # the TLS certificate (committed) and its key (never committed), made with openssl
│   ├── scripts/           # the Lua scripts, folders inside it and all (Jacob, 2026-09-30)
│   ├── logs/              # log files -- never committed
│   ├── world/             # the game's save: region.map, and Regions/<region>/ with the chunks players changed -- never committed
│   └── psql/
│       ├── defaults/schemas/ # database schemas as first made, one .sql file per table
│       └── migrations/    # every change to a table after that, numbered
└── Documentation/
    ├── HowTo/             # how-tos for a person (Jacob, 2026-09-30), public docs in my voice
    │   └── WINDOWS_INSTALL.md # building Conductor on Windows; grows as Windows is tried
    └── LLM/               # working docs
        ├── STATUS.md      # bridge between sessions
        ├── TODO.md        # pending work + future ideas
        ├── LONGTERM_TODO.md # the big features, a run of sessions each
        ├── PROJECT_OPUS.md# skeletal layout of the whole project
        ├── PROTOCOL.md    # server/client contract
        ├── REGION_MAP.md  # region.map, byte for byte: the world's other server/client contract
        ├── WRITINGSTYLE.md# my voice for public docs and comments
        ├── TEST_CHECKLIST.html # what's still to check on testing, with boxes to tick
        └── design/        # one markdown file per system or feature
```

Each component keeps two folders: `dev/` is where code is written, `build/` is
what the compiler makes. Runtime data for both components lives in one shared
`Content/` folder at the root (one per component was redundant). Files in it are
named so it's clear who owns them (`conductor_globals.cfg`, `*.scribe.log`).
Don't create new top-level folders without asking me.

`Content/` is committed, except `Content/Assets/` (the purchased art),
`Content/logs/` and `Content/world/` (the game's save; Jacob, 2026-09-30).
Even so, programs must create it and any file they need there
(config, logs, saves, schemas) with sensible defaults when it's missing, and
never crash because it's absent. Conductor finds it through the `OPUS_CONTENT`
environment variable, or by walking up from the working directory until it sees
a `Content/` folder, or by creating `./Content` when neither works.

---

## Start of every session

1. Read `Documentation/LLM/STATUS.md`, `Documentation/LLM/PROJECT_OPUS.md`, and `Documentation/LLM/TODO.md` to get
   your bearings. Read anything in `Documentation/LLM/design/` that touches today's work.
2. Re-read any source file before editing it. I sometimes hand-edit files
   between sessions, and my edits are the master copy. Never overwrite my
   changes with an older version from memory.
3. Tell me in a couple of lines where things stand, then ask what I want to work on.

## During a session

- **How we work.** I steer: I say what I want Opus to do. Your job is to make
  sure you understand what I mean, then write it out. When something I ask for
  could mean more than one thing, ask before building, and say what each
  reading would mean in practice. Don't fill gaps with guesses.
- **The test checklist.** `Documentation/LLM/TEST_CHECKLIST.html` is the rolling list of what to check on
  `testing`: every session that changes what Conductor does adds its checks there, and the reply that
  pushes to `testing` points at them.  Once he says a check passed, it comes out of the file.  Jacob's ask, 2026-09-29, so that once game features come, there's a
  reminder of what changed and what to look at in game.  It's a page he opens from the disk
  (2026-09-30; it was a .md): a check is an `<li class="check">`, a command a one-line
  `<pre class="cmd">` that gets a COPY button.  His ticks live in his browser only and
  no session sees them; he says which passed (each tick writes it into the page's message, and COPY MESSAGE copies it), the
  same as before.  A tick is keyed by the check's words, so rewording a check drops its tick.
  A tab left open from an earlier session shows that session's checks, ticks and all, until it's
  reloaded (2026-09-30: a message listing Lua checks long gone).  If a message names checks the file
  no longer has, say so and ask for a reload before touching anything.
- **Small increments.** Each conversation takes one small step, so the branch,
  the commits and STATUS.md read as a running history of what happened and why.
  If a step grows, stop at a sensible point and leave the rest for another
  conversation.
- **Design talk is written down as it settles.**  Each answer goes into
  its TODO.md entry (or design file) and is pushed as it comes, not saved
  for the hand-off, so a lost or deleted chat loses nothing (2026-09-30,
  after one was).  A design Jacob wants to talk through in a separate
  chat gets a brief in `Documentation/LLM/design/`, written for that chat
  to read cold, ending with the shape its summary comes back in.  (Tried
  once, 2026-09-30, for the ECS: the other chat lost the thread, and the
  talk went better back in the session, with the docs at hand.)
- **One feature per session.** If a new feature comes up mid-session, add it to
  `Documentation/LLM/TODO.md` and keep going on the current one. It gets its own session later.
- **Plan before building** anything bigger than a small fix: tell me the files
  you'll touch and the approach, and wait for my OK.  Check the plan against
  the rules already here first, and say when an ask runs into one (on
  2026-09-29, "passwords can't change while the server runs" ran into
  Security and Archivist being server pieces; Jacob turned it round once he
  saw why).
- **Things that can't be done yet** (because a dependency isn't built) go in
  `Documentation/LLM/TODO.md`, not half-implemented in code.
- **Future ideas** that come up in conversation also go in `Documentation/LLM/TODO.md`.
  A feature that's a run of sessions on its own (a scripting language, say)
  goes in `Documentation/LLM/LONGTERM_TODO.md` instead.
- **I'm hands-off on the files in `Opus/`.** You make every edit, CLAUDE.md
  included. Don't hand me a list of changes to make by hand; make them and tell
  me what changed.
- **I build, run, and test everything myself** and paste back the output.
  Do not run `cargo check`, `cargo build`, `cargo test`, the server, or the
  client. Stick to writing the code. When it's written, tell me exactly
  which commands to run, with your questions at the bottom of the reply (see
  "How to talk to me"). I paste back what happens and we go from there.
- Do not predict or number future sessions ("next session is X, then Y").
  I pick what to open next and I'm free to change my mind.  When I lay
  out an order myself, it's written down as mine, in my words, under
  "Jacob's map" in STATUS.md, so I don't forget it (2026-09-30: the tick,
  then the world's voxels, then zones), and it's still mine to change.

## End of every session (hand-off)

When I say we're wrapping up:

1. Update `Documentation/LLM/STATUS.md`. It holds only the last session plus any earlier session
   that directly matters for the next one. It is a bridge, not a rolling log.
   List what's waiting, unordered.  If the session's code hasn't been built
   by Jacob yet, STATUS.md says so plainly, so the next session starts by
   expecting compile fixes (2026-09-29: the account manager closed unbuilt).
2. Update `Documentation/LLM/TODO.md`, `Documentation/LLM/PROJECT_OPUS.md`, and any `Documentation/LLM/design/`
   files the session changed, so they match reality.
   Add the session's checks to `Documentation/LLM/TEST_CHECKLIST.html` (Jacob's ask, 2026-09-29): what to
   run on `testing` to see this session's change working, and what to look at in the game once there is
   one.  A check that passes is taken out, not struck through; the file is his reminder of what's
   left, not a history (Jacob, 2026-09-29; git keeps the old ones).
3. Update `README.md` if anything about the project's overview changed.
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
  on purpose, so I can't miss them).  They used to go at the top; Jacob
  moved them to the bottom on 2026-09-29.
- Keep replies short at first. Expand when I engage.
- **Every command I'm to paste is one line.**  Never wrapped with `\` over
  several lines; pasting a wrapped command breaks it.  Long is fine.  Jacob,
  2026-09-29.
- **My terminal sits in `Conductor/dev`**, where cargo runs.  Every command
  works from there: absolute paths (`/opt/storage/Coding/Opus/...`) for
  anything outside it, never a relative path with "from the repo root" in
  front.  A `mkdir -p Content/certs` given that way made a second `Content`
  inside `dev/`, and Conductor found that one first.  2026-09-29.
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
  only exceptions. (The repo is private today, but keep the code clean in case any of
  it is ever shared.)
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
  test run by hand, and its doc comment gives the exact command.
- Whenever you create a new crate, say explicitly whether it is a **bin** or a
  **lib**.
- **A crate's folder drops the `conductor-`; the crate keeps it.**  The
  folder is `Conductor/dev/tools/`, the crate in its `Cargo.toml` is
  `conductor-tools`, and code says `conductor_tools::`.  So `-p` in a cargo
  command takes the crate's name (`cargo test -p conductor-tools`), and a
  `path = "../tools"` takes the folder's.  Jacob, 2026-09-30.  Why the
  crate keeps it: in code the `conductor_` prefix tells our crates apart
  from everybody else's (his reason, the same day).
- Prefer clear ownership and simple types over heavy generics or macros.
- **The scripting language is Lua 5.4**, embedded through the `mlua`
  crate with Lua built in (Jacob, 2026-09-30).  A script never gets Lua's
  `io` or `os` libraries, or anything else that reaches the disk, the
  network or the database; it asks the game, and the game decides.
  It lives in `lua-parser` (`sandbox.rs` is the lock).  **Whatever a
  script says is wrong, and every error in one, is a Warn, never an
  Error**, and nothing a script does can take down what's underneath
  it: a time limit, a memory limit and a cap on its log lines, each
  a Warn when hit (Jacob, 2026-09-30).
- **The game library is `conductor-primlib`** (folder `primlib`, 2026-09-30),
  an ECS written by hand, no crate: an entity is a slot and a generation,
  a component is a plain struct with a `Store` in the `World`, and objects
  are made from a **template** (`NPC`: components and defaults) through a
  **blueprint** (`goblin_a`: its own changes).  A new kind of component
  goes in `components.rs` (the struct, `Kind`, `Kind::ALL`, `name()`,
  `Component`, `kind()`, `default_of()`) and in `world.rs` (a store, the
  four matches, two getters).  The world lives in memory and the game
  loop never waits on the database: copies are written to it on STOP
  SERVER and read back on START SERVER (Jacob, 2026-09-30).
- `conductor-launcher` is the program. New server pieces (networking, the game)
  are lib crates, not programs of their own.
- **Two kinds of restart.** A *soft reboot* is RESTART SERVER on the Control
  Panel: the server pieces come down and back up, the launcher and the page
  stay put. A *hard reboot* is Conductor, the whole program, run again. When
  the config editor comes, every setting gets tagged with the one it needs
  (where the log goes and the web admin's port are hard). Jacob's words,
  2026-09-29.  His example of a soft one was a tick rate, until he fixed
  the tick in code (2026-09-30).
- **Conductor and the server are two things.** The program (DiskMan, Scribe,
  Constellations, the web admin) is up from boot. The server (Fingerprinter,
  Security, Archivist, the account desk, Lua, GameWorld, the GameClock,
  networking, the monitor, and the rest of the game as it comes) only runs between START SERVER and STOP SERVER on the web admin's
  Control Panel: Conductor comes up with its door closed, and the admin opens
  it (and closes it) from there. Jacob's rule, 2026-09-29. The launcher does
  the calling on the Control Panel's say, so a new server piece goes in both
  `start_server()` and `stop_server()` in the launcher, and has to be able to
  stop and start again in the same run. Its state lives in `server.rs` in
  `conductor-tools`.
- Anything that can be slow (database, disk, network) runs on its own thread,
  and callers get the answer back later (Archivist's `Pending`). The game loop
  never waits on it. No async runtime.
- **Every thread goes through `threads::spawn(name, ...)`** in `conductor-tools`,
  never `std::thread::spawn` directly. That is what puts it on the web admin's
  "asked for" list with who started it and when.
- **Every service reports to `services.rs`** in `conductor-tools`: it's named
  in `EXPECTED` up front, says starting / running / trouble / stopped with a
  note, and checks in with `seen()` if it has a loop. That is what puts it on
  the web admin's Services tab.  A new service goes in two places there:
  `EXPECTED`, and the list in `every_expected_service_is_there_from_the_start`
  (Lua went into the first only, 2026-09-30, and `cargo test` failed).
- **Every file read and write goes through DiskMan** (`diskman.rs` in
  `conductor-tools`), never `std::fs` directly: `write()` for a whole file
  (temp file and rename), `append()`, `read()`, and `stream()` for big ones.
  Only folders (making one, listing one) stay with `std::fs`. DiskMan starts
  first and stops last.  A file it holds is checked against the disk's
  modified time and size on every read, so a hand edit is read (2026-09-29,
  Jacob's pick over rewording the soft files' comment).
- **DiskMan never logs routine work.** A log line is itself a DiskMan write, so
  a "wrote a file" line would loop forever. It logs failures only, and never
  while holding its own lock.
- **Every config file is Constellations'** (`constellations.rs` and `constellations/`
  in `conductor-tools`).  All of them live in `Content/cfg/`, fixed relative to Opus (there will
  never be a setting that moves the config folder; Jacob, 2026-09-29), in one format
  (`key = value`, `#` comments), and every one is written down once in
  `constellations/files.rs`: its settings, their kinds, defaults and
  comments, and whether the file is **soft** or **hard**.  A file is one or
  the other as a whole, never a mix; a piece that needs both gets two
  files, and files are kept separate rather than one file with sections.
  **The one exception** (Jacob, 2026-09-29): the whitelist and the
  blacklist are `Content/cfg/whitelist.cfg` and `blacklist.cfg`, one
  address or range a line, not `key = value`, and not in Constellations'
  table (they have their own tabs, not the Settings tab).  `networking.cfg`
  points at them.  They're read on every START SERVER, and a change from
  the page takes at once and rewrites the file: the only hot swap in
  Conductor.  Nothing else hot swaps.
  Soft (`postgres.cfg`, `networking.cfg`) is read on every START SERVER, so
  STOP SERVER and START SERVER applies a change.  Hard (`conductor_globals.cfg`: anything
  about Constellations, Scribe or the web admin's port; `wgui.cfg`: the web
  admin's accounts) is read at boot, so Conductor is shut down and run
  again.  Nothing but the two access lists hot swaps.  Jacob's
  rules, 2026-09-29.  Adding a setting is one entry in the table and a
  line wherever it's read (`constellations::value()` and friends); a piece
  never reads a config file itself.  A change from the web admin goes to
  `name.cfg.wait4server` beside the live file (`save_waiting()`), and
  DiskMan swaps it in when the reboot comes; the live file always says what
  Conductor is running on.
- **Every password hash goes through Security** (`security.rs` in
  `conductor-tools`): `hash_password()`, `verify_password()` and
  `verify_no_account()`, each handing back a `Ticket` (the `Pending`, plus
  `place()` for how many are ahead and about how long).  One worker thread,
  one arena of memory kept for the server's life, one login hashed at a
  time, hard limit, with everybody else in line.  Nothing else calls the
  argon2 crate, and nothing ever logs a password.
- **Every account goes through `conductor-accounts`** (2026-09-29).
  Nothing else writes SQL for the `accounts` table, and, once it exists,
  the `player_characters` table (Jacob, 2026-09-30: making or deleting a
  character writes both, so one crate writes them, in one transaction).
  Protogame and the game library call its functions.  **An account is
  never held in memory** (Jacob's rule, the day after it was): whatever
  needs one loads it from its row when it needs it (`load()`, `list()`),
  and every change goes straight back to the row, so there's one copy and
  nothing writes an old one over a new one.  Networking's book has the
  account's name and nothing else.  `Account` never has the password hash
  (`password_hash()` reads that on its own).  The last login is written
  the moment the player connects over UDP (`stamp_login()`), not at the
  TLS login (Jacob, for playtime).  Accounts are made by the admin on the
  web admin's Accounts tab, never by players, and only while the server
  is running.  Anything that hashes a password for the web admin goes
  through the account desk (`desk.rs`), so the web admin's one thread
  never waits in Security's line.
- **Every Warn and Error becomes a notice** on the web admin's bell, and stays
  there until I ACK it. So a Warn is for something actually wrong, never
  chatter. Code can raise one on purpose with `notices::publish()`.
- **The admin works through the web admin** (`conductor-wgui`). The console is
  only Scribe's output and takes no input. Anything an admin can do (shut down,
  and later accounts and config) is a page or a button there. It listens on
  `127.0.0.1` only. Never suggest binding it to anything else, and ask before
  adding a route that changes anything. Starting, restarting and stopping the
  server (`/Opus/wwwhook/start`, `/stop`, `/restart`), kicking a TCP
  connection or the player its login became (`/Opus/wwwhook/tcp/kick`), changing the access lists
  (`/Opus/wwwhook/networking/addip` and `/removeip`; Jacob's names, "add"
  and "remove" alone were too generic) and the Accounts tab's changes
  (`/Opus/wwwhook/accounts/create`, `/edit`, `/password`, `/delete`;
  its reads are under `/Opus/Content/accounts`, Jacob's paths,
  2026-09-29) are already agreed to.
  `wwwhook` is Jacob's name for a path the page posts to that makes something
  happen; the shutdown and ACK routes predate it and kept their paths.
  **Every new route goes through `/Opus/wwwhook/`**, and when a reply says
  "route" it names the whole path (`/Opus/wwwhook/tcp/kick`), so it's clear
  that's what is meant: a web admin path, not a file or a function.  Jacob,
  2026-09-29.  Ask before adding one.

### Networking (conductor-networking)

- **`REGION_MAP.md` is `region.map`'s contract**, byte for byte, the same
  way PROTOCOL.md is the packets' (Jacob asked for it, 2026-09-30).
  `gameworld/src/regionmap.rs` is written from it; a change to the layout
  bumps the file's version, and the code and the document change together,
  with a line in its version history.
- **The door waits on the world** (Jacob, 2026-09-30).  Networking isn't
  started in `start_server()`: the launcher's `take_commands()` starts it,
  TCP and UDP both, once `conductor_gameclock::ready()` says the chunks
  around 0,0,0 are in, since nobody gets in before there's a voxel to
  step on.
- **TCP is only the login; UDP is everything after.**  A client logs in over
  TLS on the TCP port, gets a ticket (a one-time token and the UDP port),
  and the TCP connection closes.  When the UDP session ends, for any
  reason, the player is gone and the client starts over at the login
  screen.  Nothing is kept for a reconnect.  Jacob's design, 2026-09-29.
- **Lower CPU, more RAM if it buys that.**  Jacob's steer for this crate.  A
  fixed pool of login threads, no thread per connection; no polling loop
  anywhere, every wait is the OS's own; players found by address in a map;
  fixed answers built once.  One login is hashed at a time, always, through
  Security's line.
- **`PROTOCOL.md` is the contract**, byte for byte.  `protocol.rs` and the
  test client are written from it; when either disagrees with the document,
  the code is what gets fixed.  A packet change bumps `PROTOCOL_VERSION`,
  and so does a new value in a packet's enum (a Kicked reason took it to 2
  on 2026-09-29, another, kicked by the admin, to 3 the same day, and
  account terminated to 4, also the same day): `protocol.rs`, PROTOCOL.md and `test_client.py` all
  change together, and the document gets a line saying what the version
  added.
- **The TLS pair is made by hand** with the openssl command in README.md, in
  `Content/certs/`.  The key is gitignored, the certificate committed.
  Conductor never makes one and never crashes without one: the Services tab
  says it's missing and the log says the command.
- **`test_client.py`** beside the crate is how networking is tested until
  Ensemble exists.  Python 3, standard library only.  I run it and paste
  back what it prints, the same as the server.
- **Client management** (a player limit, reconnect tokens, messaging a
  player from the web admin) is not this iteration.  It's in TODO.md as one
  heading.  Kicking from the web admin is built: KICK on the Connections tab.
- **The server decides what each client sees.**  Each player's packets
  carry only what that player may see; nothing is sent for the client to
  hide, since a changed client can read anything it's sent (Jacob,
  2026-09-30).
- **The access lists** (`access.rs`, 2026-09-29) are the one thing that
  changes without a reboot.  A change from the page is enforced the same
  moment: `enforce()` asks the door's verdict again for everybody online
  and drops whoever it now turns away, so a whitelist removal is a ban as
  much as a blacklisting.  Jacob's rule.

### Linux and Windows

- Development is Linux-first: Linux is where Conductor is built, run and
  tested. Windows code stays wired in behind the same functions.  It was
  first built on 2026-09-30, on a laptop (Rust's MSVC toolchain; see
  `Documentation/HowTo/WINDOWS_INSTALL.md`), and runs, START SERVER
  included, but hasn't had a database there: the laptop has no Postgres. macOS isn't a target; any OS but those two gets the
  fallback file.
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
- **The page is five sections across the top** (Jacob's layout,
  2026-09-30, picked from four mockups), and a side menu listing the open
  section's tabs:
  - CONTROL PANEL: Server, System, Conductor, Services, Storage
  - CONFIGURATION: Settings, Whitelist, Blacklist
  - LOGS: Log, Notifications History
  - ACCOUNT MANAGEMENT: Accounts
  - GAME MANAGEMENT: Connections

  The **Server** tab was the Control Panel tab; its section took the
  name.  A tab says its section with `data-section` in `page.html`.
  Anything new goes on one of the tabs, or is a new tab (or section) I
  agree to.  The notices are the bell and the LOGS section: the tray has
  HISTORY, which opens Notifications History.
- **The Accounts tab** (2026-09-29) is the game's accounts, `admin` only
  (`user` can't see the list).  The list, and a card per account opened by
  clicking its name: the owner's names and email (SAVE), a new password
  typed twice (CHANGE PASSWORD; every password is typed twice), and
  DELETE ACCOUNT, which takes a player in the world out with Kicked,
  reason 5, and the client says ACCOUNT TERMINATED (protocol version 4).
  NEW ACCOUNT opens a card for a new one.  The username never changes.
  Locked unless the server is running and the database connected.
- **The Connections tab** (2026-09-29; it was the TCP tab) is the door and
  the world, TCP first then UDP: every connection that reached the TCP
  listener since START SERVER, by address and DNS name, never by account,
  with where each one is (the queue, TLS, Security's line with its place,
  finished and how; a login whose player has left the world, or never
  came, reads LINKDEAD and why, Jacob's word, 2026-09-29) and a three-dot menu for `admin` (KICK, add the
  address to the whitelist, add it to the blacklist), in two views, Recent
  (the newest five) and Historical (the whole run; Jacob's ask,
  2026-09-29); then every player in
  the world over UDP, by account, with when they connected and how quiet
  they are.  The character goes there once there is one.
- **The Whitelist and Blacklist tabs** (2026-09-29) are the two access
  lists: the entries, REMOVE on each, an ADD field.  A change takes at once
  and writes the file.  Which list the door checks is `access_list` in
  `networking.cfg` (off, whitelist or blacklist), on the Settings tab, and
  takes on the next START SERVER.  A blacklisting while the blacklist is on
  is a ban, and so is taking an entry off the whitelist while the whitelist
  is on: every connection and player the door would now turn away is
  dropped at once, the player with a Kicked (reason 3, banned; protocol
  version 2).  KICK in a TCP row's three-dot menu kicks an open connection,
  or the player its login became (reason 4, kicked by the admin; protocol
  version 3; Jacob's ask, 2026-09-29), greyed when nothing is left to kick.  Connections, Whitelist and
  Blacklist are locked until both of networking's listeners are up; the lists can't be changed from the page while the
  server is stopped (edit the files by hand then).
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
- **The page has a login** (2026-09-29). Two accounts, fixed: `user` looks
  and touches nothing, `admin` does everything. Their passwords are the two
  settings in `wgui.cfg`, as they are, not hashed (Security only runs with
  the server, and a login has to work before START SERVER). A login card
  covers the page until you're in; every start of Conductor starts logged
  out, and a page reload doesn't. Every route but the page and
  `/Opus/login` needs the login's cookie (401 without), and every route
  that changes something needs `admin` (403 to `user`): a new route that
  changes anything goes behind `only_admin()` in `lib.rs`, and the page
  greys its button for `user` in `lockChanges()`. `login.rs` holds it.
  **No idle timeout, ever**: the login is about roles (who may change the
  server), not security; the page only listens on this machine. Jacob's
  words, 2026-09-29. `wgui_port` is moving from `conductor_globals.cfg`
  into `wgui.cfg` (his call the same day; in TODO.md until it's done).
- **The Settings tab is the config editor**, drawn from Constellations'
  table through `/Opus/settings`, with `/Opus/wwwhook/settings/save` and
  `/discard` behind it. A new config file or setting shows up there with
  no page work. A save goes to `.wait4server` and takes at the file's
  reboot; the tab never hot swaps anything.
- **Mockups go on a canvas, not in the repo.**  A new layout for the page
  can start as a few clickable mockups on a claude.ai design canvas, in
  the page's own colours with the real tab names, for Jacob to pick from
  and comment on.  They stay there ("just on the canvas", 2026-09-30);
  only the one he picks is built into `page.html`.
- When talking about the page, name the panel or tab ("the Log tab"), not the
  tool behind it. "Where does Scribe go?" read as moving the crate.
- **The tick is 250 ms, five checks of 50 ms, fixed in code** (Jacob,
  2026-09-30: "anything faster is gonna be a problem.  Slower is fine but
  faster becomes bad").  Never a setting.  It's run by **the GameClock**,
  `conductor-gameclock` (folder `gameclock`, `conductor_gameclock` in
  code; Jacob named it, 2026-09-30).  The checks run in
  order (input, AI, movement, broadcast, housekeeping), each timed from the
  cycle's start; a late one makes the next late and nothing is skipped.
  Only the GameClock's thread touches the `World`.  Jacob may call it "the
  heartbeat": that's his MUD's word for the tick and it stuck as lingo, but
  it "doesn't seem professional" for Opus, so the code and docs say the
  GameClock (2026-09-30).

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
  has already run; write a new one.
- **Every table has both an `id` and a `uuid`**, no exceptions.
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

- [FILL IN engine-specific conventions once Ensemble is set up]

---

## Git rules

- **Three branches, since 2026-09-29.**
  - `unstable` is where you write. Every session commits and pushes here.
    **First thing every session: `git fetch origin` and make sure the work
    starts from `origin/unstable`'s tip**, whatever branch the session was
    opened on. A session cut from `main` on 2026-09-29 was eleven commits
    behind and had to merge `testing` back in by hand.
  - `testing` is where I test. **When a round of edits is done and you want
    me to test it, push `unstable` onto `testing` yourself** (a fast-forward:
    `git push origin unstable:testing`), then tell me what to run. Don't wait
    to be asked.
  - `main` is the stable release. It moves only when I say so, from
    `testing`, never from `unstable`. Never push to `main` on your own.
- The session-branch-and-pull-request way (each session on its own branch,
  merged into `main`) is over. `infamous-saganism` was the last one; it's
  kept as history and isn't written to.
- If I've pushed to `unstable` from my machine, fetch and merge it before
  pushing. Never rebase or force-push over my commits, on any branch.
  The same goes for `main`: I sometimes commit there from my machine (on
  2026-09-30, `bind_address = 10.0.0.84` in `networking.cfg`, my machine's
  address and correct).  When `main` has a commit `unstable` doesn't, merge
  `main` into `unstable`, so the next release is a plain catch-up.
- Deleting a branch on GitHub can't be done from the session (the push is
  refused), so I do that by hand when one is finished with.
- **On the Windows laptop, git is Git GUI** (a clone in
  `C:\TEMP\download2`, 2026-09-30).  Git steps for that machine are
  Git GUI's menus (Remote > Fetch from > origin, Branch > Checkout),
  with the one-line command beside them.
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
    `git push` for it.
  - I've said to release: the commands that move `main` to `testing`.
  - A branch needs deleting on GitHub: the `git push origin --delete` line.
  If nothing on my side needs doing, say that too ("nothing to run in git"),
  so silence never means I missed something.
- The whole `Opus/` folder is one **private** repo: code, docs, and assets.
  It must stay private -- it holds purchased art assets that can't be
  redistributed. Never suggest making it public or pushing it anywhere else.
- Never commit build output, logs, purchased art, or engine caches: both
  `build/` folders, `Content/Assets/`, `Content/logs/`, `Content/world/`, Rust `target/`, Unity `Library/` `Temp/`
  `Obj/` `Logs/`, Godot `.godot/`. If something like that
  shows up in `git status`, tell me and suggest a `.gitignore` line.
- Large binary assets (models, textures, audio, `.blend`, `.unitypackage`) go
  through Git LFS. If you see one about to be committed without LFS, flag it.
- The hand-off ends with the work committed and pushed to `unstable`, and
  onto `testing` if it's ready for me to test.
