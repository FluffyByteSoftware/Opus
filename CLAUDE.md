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
  - Engine / language: [FILL IN -- e.g. Unity 6 (C#) or Godot 4 (C#)]
  - Folder: `Ensemble/`
- **Documentation** -- project docs. `Documentation/LLM/` holds the working
  docs (status, TODOs, design, protocol) and is the source of truth for anything
  not in the code.
- **Shared protocol** -- whatever both sides need to agree on (message types,
  encoding) is documented in `Documentation/LLM/PROTOCOL.md` and is the single
  source of truth.

If a future third piece appears (tools, admin console, test client), ask me what
to call it before creating it.

---

## Folder layout

```
Opus/
├── CLAUDE.md              # this file
├── .gitignore
├── Conductor/             # server
│   ├── dev/               # source code -- a Cargo workspace
│   │   ├── conductor-tools/    # lib: the tools (DiskMan, Scribe, Constellations, Fingerprinter, Security, Archivist, ...)
│   │   ├── conductor-monitor/  # lib: looks at the process once a second (RAM, CPU, disk, threads)
│   │   ├── conductor-wgui/     # lib: the web admin on 127.0.0.1, and the only way to shut down
│   │   └── conductor-launcher/ # bin: the program -- boots, then starts and stops the server on the Control Panel's say
│   └── build/             # compiled output -- never committed
├── Ensemble/              # client
│   ├── dev/               # source code (the engine project lives here)
│   └── build/             # compiled output -- never committed
├── Content/               # data both programs read and write -- committed, except Assets/ and logs/
│   ├── Assets/            # purchased art -- never committed
│   ├── cfg/               # config files (conductor_globals.cfg, postgres.cfg)
│   ├── logs/              # log files -- never committed
│   └── psql/
│       ├── defaults/schemas/ # database schemas as first made, one .sql file per table
│       └── migrations/    # every change to a table after that, numbered
└── Documentation/
    └── LLM/               # working docs
        ├── STATUS.md      # bridge between sessions
        ├── TODO.md        # pending work + future ideas
        ├── PROJECT_OPUS.md# skeletal layout of the whole project
        ├── PROTOCOL.md    # server/client contract
        ├── WRITINGSTYLE.md# my voice for public docs and comments
        └── design/        # one markdown file per system or feature
```

Each component keeps two folders: `dev/` is where code is written, `build/` is
what the compiler makes. Runtime data for both components lives in one shared
`Content/` folder at the root (one per component was redundant). Files in it are
named so it's clear who owns them (`conductor_globals.cfg`, `*.scribe.log`).
Don't create new top-level folders without asking me.

`Content/` is committed, except `Content/Assets/` (the purchased art) and
`Content/logs/`. Even so, programs must create it and any file they need there
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
- **Small increments.** Each conversation takes one small step, so the branch,
  the commits and STATUS.md read as a running history of what happened and why.
  If a step grows, stop at a sensible point and leave the rest for another
  conversation.
- **One feature per session.** If a new feature comes up mid-session, add it to
  `Documentation/LLM/TODO.md` and keep going on the current one. It gets its own session later.
- **Plan before building** anything bigger than a small fix: tell me the files
  you'll touch and the approach, and wait for my OK.
- **Things that can't be done yet** (because a dependency isn't built) go in
  `Documentation/LLM/TODO.md`, not half-implemented in code.
- **Future ideas** that come up in conversation also go in `Documentation/LLM/TODO.md`.
- **I'm hands-off on the files in `Opus/`.** You make every edit, CLAUDE.md
  included. Don't hand me a list of changes to make by hand; make them and tell
  me what changed.
- **I build, run, and test everything myself** and paste back the output.
  Do not run `cargo check`, `cargo build`, `cargo test`, the server, or the
  client. Stick to writing the code. When it's written, put your questions at
  the top of the reply, then tell me exactly which commands to run. I paste back what happens and
  we go from there.
- Do not predict or number future sessions ("next session is X, then Y").
  I pick what to open next and I'm free to change my mind.

## End of every session (hand-off)

When I say we're wrapping up:

1. Update `Documentation/LLM/STATUS.md`. It holds only the last session plus any earlier session
   that directly matters for the next one. It is a bridge, not a rolling log.
   List what's waiting, unordered.
2. Update `Documentation/LLM/TODO.md`, `Documentation/LLM/PROJECT_OPUS.md`, and any `Documentation/LLM/design/`
   files the session changed, so they match reality.
3. Update `README.md` if anything about the project's overview changed.
4. Update this CLAUDE.md with anything the session taught us, and tell me what
   changed.
5. Make no code changes during hand-off unless there's a glaring bug, and if so,
   tell me first.

---

## How to talk to me

- I'm an amateur hobbyist. Comfortable in C and C#, still learning Rust.
- Explain Rust plainly. Don't translate Rust into C terms unless I ask.
- **Questions for me go at the top of your reply under a visible `## Questions`
  header** so I don't miss them.
- Keep replies short at first. Expand when I engage.
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
- Prefer clear ownership and simple types over heavy generics or macros.
- `conductor-launcher` is the program. New server pieces (networking, the game)
  are lib crates, not programs of their own.
- **Conductor and the server are two things.** The program (DiskMan, Scribe,
  Constellations, the web admin) is up from boot. The server (Fingerprinter,
  Security, Archivist, the monitor, and the network and the game when they
  exist) only runs between START SERVER and STOP SERVER on the web admin's
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
  the web admin's Services tab.
- **Every file read and write goes through DiskMan** (`diskman.rs` in
  `conductor-tools`), never `std::fs` directly: `write()` for a whole file
  (temp file and rename), `append()`, `read()`, and `stream()` for big ones.
  Only folders (making one, listing one) stay with `std::fs`. DiskMan starts
  first and stops last.
- **DiskMan never logs routine work.** A log line is itself a DiskMan write, so
  a "wrote a file" line would loop forever. It logs failures only, and never
  while holding its own lock.
- **Every password hash goes through Security** (`security.rs` in
  `conductor-tools`): `hash_password()`, `verify_password()` and
  `verify_no_account()`, each handing back a `Ticket` (the `Pending`, plus
  `place()` for how many are ahead and about how long).  One worker thread,
  one arena of memory kept for the server's life, one login hashed at a
  time, hard limit, with everybody else in line.  Nothing else calls the
  argon2 crate, and nothing ever logs a password.
- **Every Warn and Error becomes a notice** on the web admin's bell, and stays
  there until I ACK it. So a Warn is for something actually wrong, never
  chatter. Code can raise one on purpose with `notices::publish()`.
- **The admin works through the web admin** (`conductor-wgui`). The console is
  only Scribe's output and takes no input. Anything an admin can do (shut down,
  and later accounts and config) is a page or a button there. It listens on
  `127.0.0.1` only. Never suggest binding it to anything else, and ask before
  adding a route that changes anything. Starting, restarting and stopping the
  server (the three `/Opus/wwwhook/` routes) are already agreed to.

### Linux and Windows

- Development is Linux-first: Linux is where Conductor is built, run and
  tested. Windows code stays wired in behind the same functions, but nobody
  is building it for now. macOS isn't a target; any OS but those two gets the
  fallback file.
- There is no "which OS" setting in the config. The compiler knows what it is
  building for, and `#[cfg(target_os = "linux")]` / `#[cfg(windows)]` pick the
  code. OS-specific code gets one file per OS behind a common set of functions
  (see `conductor-monitor/src/probe/`), plus a fallback file for anything else
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
- The page is seven tabs down the left sidebar, under the OP logo: Control
  Panel, System, Conductor, Services, Storage, Notifications History, Log.
  Anything new goes on one of them, or is a new tab I agree to.
- The Control Panel and the Log are always clickable. Until the server is
  running they're the only tabs that are, and the bell is hidden. The Control
  Panel is the only place the server is started, restarted and stopped, and
  the only place SHUT DOWN is; the header has no buttons but the bell.
- While the server is running and the database isn't connected, the data
  tabs are blurred and locked. Anything new on the page sits under that lock;
  only the header (the bell and its tray included), the Control Panel and the
  Log stay above it.
- When talking about the page, name the panel or tab ("the Log tab"), not the
  tool behind it. "Where does Scribe go?" read as moving the crate.
- [FILL IN the tick rate once there is a game loop]

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
- Deleting a branch on GitHub can't be done from the session (the push is
  refused), so I do that by hand when one is finished with.
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
  `build/` folders, `Content/Assets/`, `Content/logs/`, Rust `target/`, Unity `Library/` `Temp/`
  `Obj/` `Logs/`, Godot `.godot/`. If something like that
  shows up in `git status`, tell me and suggest a `.gitignore` line.
- Large binary assets (models, textures, audio, `.blend`, `.unitypackage`) go
  through Git LFS. If you see one about to be committed without LFS, flag it.
- The hand-off ends with the work committed and pushed to `unstable`, and
  onto `testing` if it's ready for me to test.
