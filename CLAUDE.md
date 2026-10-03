**EVERY SESSION PUSHES ITS CODE TO THE `unstable` BRANCH.  Never to a session
branch, whatever branch the session was opened on.  Jacob's rule.**

<!--
File:       Opus/CLAUDE.md
Component:  Opus
Author:     Jacob Chacko
-->

# Opus

Opus is the codename for this project: a multiplayer game made of a server, a
client and a launcher. When I say "Opus" I mean this project, not the Claude
model.

**The game's name is Forgotten Legends** (2026-10-01): it's what players see.
The project, its folders, its crates and its code stay Opus ("its the games
name but not the directory path").  Unity's Player Settings say Company Name
**FluffyByte** and Product Name **Opus.Ensemble**.

Opus is a separate project from Stratum and Mantle. Do not pull code from them
unless I explicitly ask. Their *workflow* carried over; their code did not.

Project root: `/opt/storage/Coding/Opus`

**0.0.1 is released** (2026-10-02): a player logs in, picks a character and
chats.  The next milestone is movement, 0.0.0.12 (Jacob's number).  His map
is in STATUS.md.

**This file is the rules and where to look.**  It holds what applies in every
session (how we work, git, the code rules, and the rules a piece can't be
touched without) and points at the document that has the rest.  The detail
of a piece lives in its design file, and a session reads the one for the
concern in front of it before touching that piece.  A rule that only matters
inside one piece goes in that piece's design file, not here (2026-10-03).

---

## Where to look

| The concern                                  | Read                                                      |
|----------------------------------------------|-----------------------------------------------------------|
| Where things stand, what's waiting, his map  | `Documentation/LLM/STATUS.md`                             |
| What's deferred, ideas, the clean-up list    | `Documentation/LLM/TODO.md`; big ones, `LONGTERM_TODO.md` |
| Every folder and file, the named pieces      | `Documentation/LLM/PROJECT_OPUS.md`                       |
| The front page, building and running it      | `README.md`; `Documentation/HowTo/` to install, release, Windows|
| My voice, the file headers, line widths      | `Documentation/LLM/WRITINGSTYLE.md`                       |
| What to check on `testing`                   | `Documentation/LLM/TEST_CHECKLIST.html`                   |
| The packets, byte for byte                   | `Documentation/LLM/PROTOCOL.md`                           |
| `region.map`, byte for byte                  | `Documentation/LLM/REGION_MAP.md`                         |
| The HUD's layout and catalog files           | `Documentation/LLM/HUD_FORMATS.md`; the brief, `HUD_LAYOUT_SYSTEM.md`|
| The patcher's manifests and the web folder   | `Documentation/LLM/PATCH_MANIFEST.md`                     |
| The password's key, both halves              | `Documentation/LLM/design/client-security.md`             |
| The tools: DiskMan, Scribe, Constellations, Fingerprinter, Security, Archivist | `design/conductor-tools.md` |
| Accounts, characters' rows, the account desk | `design/conductor-accounts.md`                            |
| The monitor                                  | `design/conductor-monitor.md`                             |
| The door: TLS, UDP, Protogame, the spawn, chat, `/who`, the commands, the lists | `design/conductor-networking.md` |
| The web admin: routes, every tab, the login  | `design/conductor-wgui.md`                                |
| The launcher's boot, start and stop order    | `design/conductor-launcher.md`                            |
| Lua and the sandbox                          | `design/lua-parser.md`                                    |
| The game library: entities, components, templates, saving | `design/primlib.md`                          |
| The world: blocks, chunks, regions, its files| `design/world.md`                                         |
| The GameClock: the beat, the checks, the mailbox, the world save | `design/gameclock.md`                  |
| Ensemble's screens, widgets, the chat window | `design/ensemble-hud.md`                                  |
| Ensemble's net code, the ticket, the start screen | `design/ensemble-networking.md`                      |
| Soundcheck: the check, the login, PLAY, admin and debug modes | `design/soundcheck.md`                   |
| The 0.0.1 review's findings                  | `Documentation/LLM/CODE_REVIEW_0.0.1.md`                  |

---

## Components

- **Conductor** -- the game server.  Authoritative: it owns the game state and
  the tick loop.  Clients ask, Conductor decides.  Rust, edition 2024, a Cargo
  workspace in `Conductor/dev/`.  Runs on Linux (Nobara and Fedora);
  Linux-first, Windows wired in and sometimes untested, macOS not a target.
- **Ensemble** -- the game client players run.  Unity 6000.6 (C#), in
  `Ensemble/dev/Opus.Ensemble/`.  Its project settings and our four folders
  under `Assets/` are committed; the rest is on Jacob's machine.
- **Soundcheck** (`Opus.Soundcheck`) -- the launcher players open (2026-10-02).
  It checks the installed client against its manifest, does the login (TLS
  1.3 to the Ticket) and starts Ensemble with the ticket.  C#, .NET 10,
  Avalonia 11, not Unity, in `Soundcheck/dev/`.  Linux and Windows.  One day
  it hands each client a certificate of its own (LONGTERM_TODO.md).
- **Content/** -- the data the programs read and write: `cfg/`, `certs/`,
  `scripts/`, `psql/`, and the uncommitted `logs/`, `world/`, `Assets/`
  (the purchased art) and `patch/`.  Files are named so it's clear who owns
  them.  Programs create `Content/` and any file they need there with
  sensible defaults, and never crash because it's absent.  Conductor finds it
  through `OPUS_CONTENT`, or by walking up from the working directory, or by
  making `./Content`.
- **Documentation/** -- `HowTo/` for a person, `LLM/` the working docs, the
  source of truth for anything not in the code; `REPORT.html` the easy
  version with diagrams.
- **Shared contracts** -- whatever two sides have to agree on is written down
  once, byte for byte, and the code is written from it; when they disagree,
  the code is what gets fixed.  The packets (PROTOCOL.md), `region.map`
  (REGION_MAP.md), the HUD's files (HUD_FORMATS.md), the manifests
  (PATCH_MANIFEST.md), the password's key (`design/client-security.md`).  A
  change bumps the contract's version, and the code and the document change
  together with a line in its history.  A recipe both sides compute gets a
  worked example (an input and the exact output) each side is checked
  against.

Each component keeps `dev/` (where code is written) and `build/` (what the
compiler makes, never committed).  Don't create new top-level folders
without asking me.  If a future piece appears (tools, admin console, test
client), ask me what to call it before creating it.  PROJECT_OPUS.md has
every folder and file.

---

## Start of every session

1. `git fetch origin` and start from `origin/unstable`'s tip ("Git rules").
2. Read `Documentation/LLM/STATUS.md`, `PROJECT_OPUS.md` and `TODO.md` to get
   your bearings, and the design file for whatever today's work touches.
3. Re-read any source file before editing it.  I sometimes hand-edit files
   between sessions, and my edits are the master copy.  Never overwrite my
   changes with an older version from memory.
4. Tell me in a couple of lines where things stand, then ask what I want to
   work on.

## During a session

- **How we work.** I steer: I say what I want Opus to do.  Your job is to
  make sure you understand what I mean, then write it out.  When something I
  ask for could mean more than one thing, ask before building, and say what
  each reading would mean in practice.  Don't fill gaps with guesses.  Say
  back where a thing sits and what it does to whom before writing it ("a
  lockout on a character for like 1 second" was once built as a lock on
  leaving, and I meant a lock on loading).
- **Plan before building** anything bigger than a small fix: tell me the
  files you'll touch and the approach, and wait for my OK.  Check the plan
  against the rules here first, and say when an ask runs into one; I may turn
  the ask round once I see why.
- **Small increments.** Each conversation takes one small step, so the
  branch, the commits and STATUS.md read as a running history of what
  happened and why.  If a step grows, stop at a sensible point and leave the
  rest for another conversation.
- **One feature per session.** A new feature that comes up mid-session goes
  in TODO.md and gets its own session later.  When one is done, say so and
  offer the hand-off; if I say keep going, the next one starts in the same
  session, still planned, OKed, built and checked on its own, with its own
  commits and its own line in STATUS.md.
- **Design talk is written down as it settles.**  Each answer goes into its
  TODO.md entry or design file and is pushed as it comes, not saved for the
  hand-off, so a lost chat loses nothing.  Design is talked through here,
  with the docs at hand, never in a brief for a separate chat (tried once;
  the other chat lost the thread).  What another chat settles (a GDD, say)
  comes in through me.  **I redesign out loud, and every shape is written
  down as it comes**, in my words, so the one I settle on has the others
  beside it for why; the code follows the last OK.
- **Things that can't be done yet** (a dependency isn't built) go in TODO.md,
  not half-implemented in code.  **Future ideas** go in TODO.md too; a feature
  that's a run of sessions on its own goes in LONGTERM_TODO.md.
- **The test checklist.** `Documentation/LLM/TEST_CHECKLIST.html` is the
  rolling list of what to check on `testing`.  Every session that changes
  what a program does adds its checks there, and the reply that pushes to
  `testing` points at them.  Once I say a check passed, it comes out of the
  file.  It's a page I open from the disk: a check is an `<li class="check">`,
  a command a one-line `<pre class="cmd">` that gets a COPY button.  My ticks
  live in my browser only; I say which passed (each tick writes it into the
  page's message, and COPY MESSAGE copies it).  A tick is keyed by the
  check's words, so rewording a check drops its tick.  A tab left open from
  an earlier session shows that session's checks until it's reloaded: if a
  message names checks the file no longer has, say so and ask for a reload
  before touching anything.  A check there's nothing to run on yet goes
  under Parked, counted with the rest, and a Parked check in a "passed"
  message is asked about, not taken out.  Whether a check becomes a GitHub
  issue instead is my call (issue #10 is one).  A number the session needs
  back (a timing, say) is asked for under the QUESTIONS header too, not only
  inside a check, or it never comes back.
- **A review's finding is a claim until the code says so.**  A review goes
  in as a file with file and line for every finding, a "checked and fine"
  list so nothing is re-investigated, and a line on each finding saying when
  it was fixed (`CODE_REVIEW_0.0.1.md` is the shape; one of its claims was
  wrong and the build caught it).  I pick what's fixed and when.
- **A guess at a cause is written as a guess** until the build says so; a
  comment that guessed wrong gets corrected.  A library's tracker and source
  (raw.githubusercontent.com reaches the session) are read before a third
  guess.  **A package version is looked up, never remembered** (one pinned
  from memory was two years stale).
- **I'm hands-off on the files in `Opus/`.** You make every edit, CLAUDE.md
  included.  Don't hand me a list of changes to make by hand; make them and
  tell me what changed.
- **I build, run, and test everything myself** and paste back the output.
  Do not run `cargo check`, `cargo build`, `cargo test`, `dotnet`, the
  server, or the client.  Stick to writing the code.  When it's written, tell
  me exactly which commands to run, with your questions at the bottom of the
  reply.  Conductor is run from a terminal, not from inside RustRover (its
  code analysis beside the server once locked the machine up).  When a key
  does nothing in my terminal, ask me to try it on something plain
  (`sleep 30`, Ctrl-C) before changing code.
- Do not predict or number future sessions ("next session is X, then Y").
  I pick what to open next and I'm free to change my mind.  When I lay out
  an order myself, it's written down as mine, in my words, under "Jacob's
  map" in STATUS.md, and it's still mine to change.

## End of every session (hand-off)

When I say we're wrapping up:

1. Update `Documentation/LLM/STATUS.md`: where things stand, the last session
   in detail, and the rolling day (below).  List what's waiting, unordered.
   If the session's code hasn't been built by me yet, say so plainly, so the
   next session starts by expecting compile fixes.  **STATUS.md holds the
   last 24 hours of sessions as well** (2026-10-03, my "oh shit" fallback):
   a line each, "we did X, Y, Z", headed by the session's number and date.
   **Sessions are numbered from 0** (2026-10-03, the docs clean-up), each the
   one after; the number goes on the "Last session" heading too.  Older than
   a day, a line comes out, unless it still bears on what's next.
2. Update `TODO.md`, `PROJECT_OPUS.md` and the `design/` files the session
   changed, so they match reality.  Add the session's checks to
   `TEST_CHECKLIST.html`.  A check that passes is taken out, not struck
   through; the file is my reminder of what's left, not a history.
3. Update `README.md` if anything about the project's overview or where it
   stands changed.
4. Update this CLAUDE.md with any rule the session taught us, and tell me
   what changed.
5. Make no code changes during hand-off unless there's a glaring bug, and if
   so, tell me first.

---

## How to talk to me

- I'm an amateur hobbyist.  Comfortable in C and C#, still learning Rust.
- Explain Rust plainly.  Don't translate Rust into C terms unless I ask.
- **Questions for me go at the bottom of your reply, under a big, loud
  header** (`# >>>>>>>>>> QUESTIONS FOR JACOB <<<<<<<<<<` -- large and annoying
  on purpose, so I can't miss them).
- Keep replies short at first.  Expand when I engage.
- **Every command I'm to paste is one line.**  Never wrapped with `\` over
  several lines; pasting a wrapped command breaks it.  Long is fine.
- **My terminal sits in `Conductor/dev`**, where cargo runs.  Every command
  works from there: absolute paths (`/opt/storage/Coding/Opus/...`) for
  anything outside it, never a relative path with "from the repo root" in
  front (one of those once made a second `Content` inside `dev/`, and
  Conductor found that one first).
- When talking about the web admin, name the tab ("the Log tab"), not the
  tool behind it ("Where does Scribe go?" read as moving the crate).  If I
  say "the Control Panel" I may mean the Server tab; it had that name once.
- A reply that sends me into Unity's editor says which window (Hierarchy,
  Project, Inspector).  Unity calls two things a "UI Document", a UXML file
  and the component on a GameObject; we use only the component, with no
  Source Asset, so say which is meant.
- When you change files, end with a short list of which files changed and why.

---

## Code rules (all components)

- Simple and readable over clever.  If there's a clever way and a plain way,
  use the plain way.  Prefer clear ownership and simple types over heavy
  generics or macros.
- **All time is UTC.** Any time shown to a person ends with `Z`.  The one
  exception is `/who`'s time (Jacob, 2026-10-02): seconds since midnight UTC
  to the client, shown in the player's own time zone.
- **Most log lines are Debug.** Routine things (loaded a file, connected, ran
  the schemas, a job finished) are Debug.  Info is for the few milestones an
  admin cares about (starting, shutting down, a service coming up or going
  down).  Warn and Error are for things that are actually wrong.  A switch in
  the config will turn Debug lines off (TODO.md), so a finished server's log
  reads clean.
- **Every Warn and Error becomes a notice** on the web admin's bell, and
  stays there until I ACK it.  So a Warn is for something actually wrong,
  never chatter.  Code can raise one on purpose with `notices::publish()`.
- Ask before adding any new dependency (crate, package, plugin).  Minimal
  dependencies is the default.
- No references to AI, Claude, or assistants anywhere in source code,
  comments, the README, or commit messages.  `CLAUDE.md` and
  `Documentation/LLM/` are the only exceptions.  (The repo is private today,
  but keep the code clean in case any of it is ever shared.)
- Comments and public docs are written in my voice per WRITINGSTYLE.md.
  Line width 120 columns; comments wrap at 78.
- **Every file starts with a header** saying where it lives, as a path from
  the repo root beginning with `Opus/` (never the full drive path), which
  component it belongs to, and who wrote it, then a line or three on what the
  file is for.  When a file moves or is renamed, its `File:` line changes
  with it.  Rust uses `//!`, C# `//`, TOML, `.gitignore`, shell scripts and
  `.conf` files `#`, Markdown an HTML comment at the very top.  The only
  files without a header are ones that can't hold comments or that a tool
  generates and rewrites: JSON, Unity's `.meta` / `.unity` / `.asset` /
  `.prefab` files, lock files, and anything under `build/` or `Content/`.
  WRITINGSTYLE.md has the headers written out.
- Paths are built with `Path::join` (or `Path.Combine`), never by gluing
  strings with `/` or `\`.
- **Nothing ever logs a password, a key or a token.**  A key logs in as well
  as the password does.
- **Benchmarks run with `--release`.** `cargo test` builds unoptimized, and
  unoptimized Argon2 read six times slow.  A timing test is an `#[ignore]`
  test run by hand, and its doc comment gives the exact command.  A cost
  given in a plan is a guess until a timing test measures it, and the plan
  says so (the world save's snapshot was guessed at "a few milliseconds"
  for 10,000 characters and measured at 33).

## Rust rules (Conductor)

- **Never use `#[allow(dead_code)]`** or any other lint suppression.  I
  would rather see the warnings.
- Whenever you create a new crate, say explicitly whether it is a **bin** or
  a **lib**.  `conductor-launcher` is the program; new server pieces are lib
  crates, not programs of their own.
- **A crate's folder drops the `conductor-`; the crate keeps it.**  The
  folder is `Conductor/dev/tools/`, the crate is `conductor-tools`, and code
  says `conductor_tools::`.  So `-p` takes the crate's name (`cargo test -p
  conductor-tools`) and a `path = "../tools"` the folder's.
- **Conductor and the server are two things.** The program (DiskMan, Scribe,
  Constellations, the web admin) is up from boot.  The server (Fingerprinter,
  Security, Archivist, the account desk, Lua, GameWorld, the GameClock, the
  monitor, networking, and the rest of the game as it comes) only runs
  between START SERVER and STOP SERVER on the web admin's Server tab.  A new
  server piece goes in both `start_server()` and `stop_server()` in the
  launcher, and has to stop and start again in the same run.  **The door
  waits on the world**: networking is the one piece `start_server()` doesn't
  start; `take_commands()` opens it once `conductor_gameclock::ready()` says
  the chunks around 0,0,0 are in.  `design/conductor-launcher.md` has the
  order and why.
- **Two kinds of restart.** A *soft reboot* is STOP SERVER and START SERVER
  (or RESTART SERVER) on the Server tab.  A *hard reboot* is Conductor run
  again.  Every config file is one or the other.
- **Anything that can be slow (database, disk, network) runs on its own
  thread**, and callers get the answer back later (a `Pending`).  The game
  loop never waits on it.  No async runtime.  Archivist doesn't log a job
  that fails: the failure is in its `Pending` and nowhere else, so whoever
  sends a job looks at its `Pending` (the GameClock keeps its saves' and
  looks in housekeeping, without waiting).
- **Every thread goes through `threads::spawn(name, ...)`**, never
  `std::thread::spawn`.  **Every service reports to `services.rs`**: named
  in `EXPECTED`, says starting / running / trouble / stopped, checks in with
  `seen()` if it has a loop; a new one goes in `EXPECTED` and in the test
  `every_expected_service_is_there_from_the_start`.
- **Every file read and write goes through DiskMan**, never `std::fs`
  directly (only folders, and `/proc`, stay with `std::fs`).  DiskMan starts
  first and stops last, and never logs routine work (a log line is itself a
  DiskMan write).
- **Every config file is Constellations'**: all in `Content/cfg/`, one
  format (`key = value`, `#` comments), every one written down once in
  `constellations/files.rs` with its settings and whether it's soft or hard
  as a whole.  Adding a setting is one entry in the table and a line
  wherever it's read; a piece never reads a config file itself.  A change
  from the web admin waits as `name.cfg.wait4server` until the file's
  reboot; the live file always says what Conductor is running on.  **The one
  exception**: the whitelist and the blacklist, one address or range a line,
  not in the table, read on every START SERVER and changed from the page at
  once.  **Nothing else in Conductor hot swaps.**
- **Every password hash goes through Security**, one worker thread, one
  login hashed at a time, hard limit, everybody else in line.  Nothing else
  calls the argon2 crate.  **What's hashed is the password's key, never the
  password** (`design/client-security.md`): the client sends the key, and
  the account desk makes it from what the admin types with Security's
  `password_key()`, the same recipe to the byte.  Nothing else calls the
  pbkdf2 crate.
- **Every account goes through `conductor-accounts`.**  Nothing else writes
  SQL for `accounts` or `player_characters`.  **An account is never held in
  memory**: whatever needs one loads it from its row, and every change goes
  straight back, so there's one copy.  Networking's book has the account's
  name and nothing else.  Accounts are made by the admin on the Accounts
  tab, never by players, only while the server runs; a character is three to
  an account, and only the player deletes one.  Anything that hashes a
  password for the web admin goes through the account desk, so the web
  admin's one thread never waits in Security's line.
- **The admin works through the web admin** (`conductor-wgui`).  The console
  is only Scribe's output and takes no input.  It listens on `127.0.0.1`
  only; never suggest binding it to anything else.  **Every new route goes
  through `/Opus/wwwhook/`** (my name for a path the page posts to that
  makes something happen), and **ask before adding one**; a reply says the
  whole path.  `design/conductor-wgui.md` lists every route there is.
- **The scripting language is Lua 5.4** through `mlua`, locked down in
  `lua-parser`: a script never reaches the disk, the network or the
  database; it asks the game.  **Whatever a script says is wrong, and every
  error in one, is a Warn, never an Error**, and nothing a script does can
  take down what's underneath it.
- **The game library is `conductor-primlib`**, an ECS written by hand, no
  crate: templates (`NPC`) and blueprints (`goblin_a`), a template taking in
  another whole (my `inherit STD_LIVING;`).  The world lives in memory and
  the game loop never waits on the database.  **What's saved is picked per
  field**, the plain way, in each component's `saved()`; a GameObject is
  saved as Lua text.  `design/primlib.md` has the recipe for a new
  component.
- **The ground is `conductor-gameworld`** (`design/world.md`).  A block is
  1 m a side, Minecraft's size; chunks 32 a side; the world eleven chunks
  tall, -32 to +319; `world_size` in `game.cfg` is the width, 1024 blocks a
  step.  The code counts in blocks only, never in metres.  A block's number
  never changes once it's out there.  A chunk's own file always wins over
  its region's ground, and a bad file is never built over: it may be the
  only copy of somebody's digging.
- **The tick is 250 ms, five checks of 50 ms, fixed in code** ("anything
  faster is gonna be a problem.  Slower is fine but faster becomes bad").
  Never a setting.  It's **the GameClock** (`conductor-gameclock`); I may
  call it "the heartbeat", but the code and docs say the GameClock.  My
  "tick" is a 250 ms cycle.  Only the GameClock's thread touches the `World`
  and the `Terrain`; everything reaches them through its mailboxes.  Until
  the ground around 0,0,0 is in, only housekeeping runs.  **The world save
  is global**, every `world_save_seconds`, one cycle copying every
  character and Archivist writing them in one transaction behind the tick.
  `design/gameclock.md` has it.
- **Memory has room; CPU is the tight side** (Jacob, 2026-10-01, at about
  800 MB for all of Conductor at `world_size` 32: "I think I'm really just
  CPU limited").  When a design trades one for the other, say so, and lean
  toward spending RAM to save CPU.  Networking's steer is the same: a fixed
  pool of login threads, no polling loop anywhere, players found by address
  in a map, fixed answers built once.
- **TCP is only the login; UDP is everything after.**  A client logs in
  over TLS, gets a ticket, and the TCP connection closes.  When the UDP
  session ends, for any reason, the player is gone and the client starts
  over at the login.  Nothing is kept for a reconnect, and there's no way
  back to character select from the world.  **The server decides what each
  client sees**: nothing is sent for the client to hide.  **A player's
  character comes into the world through the GameClock's mailbox** and
  every way a player leaves the book takes it out and saves it.
- **A crate that leans on one that calls it gets a slot.**  Rust won't
  build two crates that name each other, so the lower one keeps the types
  both need and a `set_*()` for a plain function, and the launcher fills it:
  `conductor_player_commands::wire()` fills networking's runner and the
  GameClock's chat and `/who list` senders.  **What a player types is
  `conductor-player-commands`**: a table of commands, one file each, with
  the anti-flood in the one lookup; a new command is a new file and a new
  line.  Admin commands, when they come, are "a permissions difference but
  the commands will otherwise be the same".
- **The TLS pair is made by hand** with the openssl command in README.md,
  in `Content/certs/`: the key gitignored, the certificate committed and
  **copied to `Soundcheck/dev/Certs/`** (Ensemble never speaks TLS).
  Conductor never makes one and never crashes without one.  **No `.key`
  file goes in git, anywhere**, and a reply giving a `git add` says to check
  `git status` for one.
- **`test_client.py`** beside the networking crate stands in for Soundcheck
  and Ensemble both, the whole way from the login to the chat.  Python 3,
  standard library only.  I run it and paste back what it prints.
- **The access lists** are the one thing that changes without a reboot, and
  a change is enforced the same moment: a whitelist removal is a ban as much
  as a blacklisting.

### Linux and Windows

- Development is Linux-first: Linux is where Conductor is built, run and
  tested.  Windows code stays wired in behind the same functions and builds
  and runs on a Windows laptop (`Documentation/HowTo/WINDOWS_INSTALL.md`),
  but hasn't had a database there.  macOS isn't a target; any OS but those
  two gets the fallback file.
- There is no "which OS" setting.  `#[cfg(target_os = "linux")]` /
  `#[cfg(windows)]` pick the code: one file per OS behind a common set of
  functions (see `monitor/src/probe/`), plus a fallback file that builds
  and reports "not measured here yet" instead of failing.
- Talk to the OS through what it already has, not a crate: `/proc` on
  Linux, kernel32 (and ntdll) through an `extern` block on Windows.
  `unsafe` lives only in those OS files, each block with a comment saying
  why it holds.  Ask before reaching for a crate like `sysinfo` or
  `windows-sys`.
- Anything that can only be tested on the other OS gets said so in the
  reply, with the commands to run it there.  Getting a build onto my
  Windows machine is a hassle, so Windows code can sit untested for a
  while.  STATUS.md says so for as long as it does.

### The web admin's page

- `page.html` and `json.rs` are two halves of one contract: the JSON's
  shape is written at the top of `json.rs`, and a change to one needs the
  other.
- No made-up numbers on the page.  If there's nothing behind a panel yet,
  the panel waits.
- The page pulls nothing from the internet, and puts our data in with
  `textContent`, never `innerHTML`.
- I look at the page myself and send screenshots.  When a layout class or
  style is added, make sure the CSS for it exists.  Checking `page.html` in
  a headless browser with made-up numbers is fine; say that's all it was.
- **The page is five sections across the top** (CONTROL PANEL,
  CONFIGURATION, LOGS, ACCOUNT MANAGEMENT, GAME MANAGEMENT) and a side menu
  listing the open section's tabs.  Anything new goes on one of the tabs,
  or is a new tab (or section) I agree to.  The Server tab is the only
  place the server is started, restarted and stopped, and the only place
  SHUT DOWN is.
- **What's locked when**: until the server is running only the Server tab,
  the Log and the Settings open, and the bell is hidden; while the server
  runs and the database isn't connected the data tabs are blurred and
  locked, and anything new sits under that lock; Connections, Whitelist and
  Blacklist wait for both of networking's listeners; Accounts is `admin`
  only.  **The page has a login**, two fixed accounts: `user` looks and
  touches nothing, `admin` does everything.  A new route that changes
  anything goes behind `only_admin()`, and the page greys its button for
  `user` in `lockChanges()`.  **No idle timeout, ever**: the login is about
  roles, not security.
- **The Settings tab is the config editor**, drawn from Constellations'
  table, so a new config file or setting shows up with no page work.  It
  never hot swaps anything.
- **Mockups go on a canvas, not in the repo**: a few clickable mockups on a
  claude.ai design canvas, in the page's own colours with the real tab
  names, for me to pick from.  Only the one I pick is built into
  `page.html`.
- `design/conductor-wgui.md` has every tab and route.

## Database (Conductor)

- PostgreSQL 18, running locally on my dev machine.  It listens on
  `localhost` only.  Keep it that way: never suggest opening it to the
  network.
- Database `opusdb`, tables in `public`; Conductor connects as `opus_game`
  over TCP to `localhost:5432` with `scram-sha-256`.  **Tables are created
  as `opus_game`**, so the server owns them (`seliris` owns the database).
- Never hardcode the password in source.  Archivist reads it from
  `Content/cfg/postgres.cfg`, committed on purpose: the password is a
  placeholder and Postgres only listens on this machine.
- The Postgres crate is `postgres` (the blocking client), on Archivist's own
  thread.  Ask before adding any other database crate.
- Do not run `psql`, migrations, or anything that touches the live
  database, and never edit Postgres's own config.  Write the SQL; I run it
  and paste back the output.
- Default schemas live in `Content/psql/defaults/schemas/`, one `.sql` file
  per table, `CREATE ... IF NOT EXISTS` only, run on every connect, and
  baked in with `include_str!` (`DEFAULT_SCHEMAS`) so a missing file gets
  written back out.
- **A schema file is frozen once its table exists.** Every change after
  that is a migration in `Content/psql/migrations/`, `0001_what_it_does.sql`,
  run once, in order, in a transaction, recorded in `archivist_migrations`.
  Never edit a migration that has run.  Before changing a schema file
  that's been pushed, ask me to look (I check in DataGrip).
- **No game data is without an `id` and a `uuid`** (Jacob, 2026-09-30).
  `id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY` is the table's own
  number, and tables point at each other by it; `uuid UUID NOT NULL UNIQUE
  DEFAULT uuidv7()` is the game's name for the row, version 7 so they sort
  by time, handed over from Fingerprinter on every insert.  The one
  exception is `archivist_migrations`.  The `postgres` crate can't send a
  `String` into a UUID column, so the SQL casts: `$1::text::uuid` in,
  `uuid::text` out.

## Client rules (Ensemble)

- **Our folders under `Assets/`** (Jacob, 2026-10-01): `Editor/` (editor
  plugins, each a menu item under **Tools > Opus**), `Code/` (the plain C#),
  `Scripts/` (the scripts), `Data/` (our data files: `Layouts/`, `Styles/`).
  Those four and their `.meta` files are all of `Assets/` that's committed;
  the rest is the purchased art (`Assets/Purchased/`, copies of it under
  `Assets/Art/`) and what Unity makes, on Jacob's machine.  Purchased art,
  and anything copied out of it, never goes in those four.  A new folder of
  ours under `Assets/` goes past Jacob first, and into the `.gitignore` with
  its `.meta`.
- **Jacob runs Unity**: the session writes the C#, he opens the editor and
  pastes back the Console.  A slot filled in the Inspector lives in the
  scene, which isn't committed, so the reply that asks for one says File >
  Save after.  **A purchased font is a slot too** (Chat Font is Retro, from
  his Font Nation pack), never a copy in our four folders; `fc-query -f
  '%{spacing}'` on the `.ttf` says whether it's monospaced (100).
- **The `.meta` round.**  Unity makes a `.meta` beside every file and
  folder, and a session can't run Unity, so a new file's `.meta` comes from
  Jacob's machine and the pushed tree doesn't compile until it has.  **Until
  `git meta` lands, every reply says so again**, and the next session's
  first check is `git ls-files` on `Assets/`.  The steps go in the order
  they're run, numbered: the pull first (the new files have to be on the
  disk), then Unity's focus (it imports, and makes the `.meta`s, when its
  window gets focus) and its Console, then **`git meta`** last, Jacob's
  alias: the add of the two folders and `Cargo.lock`, `git status --short`
  (new `.meta`s show with an `A`), a commit, `git pull --no-rebase
  --no-edit` and `git push origin HEAD:testing HEAD:unstable`, stopping at
  the first step that fails.  A reply says `git meta`, for a `.meta` round
  and a changed lock file alike.  The add is always the two folders, never
  a list of files and never `-A` on the whole project:
  `git add -A /opt/storage/Coding/Opus/Ensemble/dev/Opus.Ensemble/Assets /opt/storage/Coding/Opus/Ensemble/dev/Opus.Ensemble/ProjectSettings`.
  Unity writes `ProjectSettings.asset` only on File > Save Project or on
  closing, which is why a setting changed in Player Settings can be missing
  from a commit.  **A deletion round is Jacob's too**: the session's harness
  refuses a `git rm` of many files, so the files that go are one `git rm -r
  -q` line with absolute paths and globs, run right after the pull and
  before Unity gets focus, and `git meta` commits the deletions with the
  `.meta`s.
- **Ensemble never logs in and never sees a password** (2026-10-02; the
  login is Soundcheck's).  It has no TCP and no TLS: it reads the ticket
  from its environment (`OPUS_SERVER`, `OPUS_UDP_PORT`, `OPUS_TOKEN`,
  `OPUS_SOUNDCHECK`), sends Connect, and the first screen is character
  select.  With no ticket it shows the start screen; in the editor that's
  dev mode, watching for the ticket Soundcheck's `--debug` writes.  Every
  way out goes back to the launcher with why (`OPUS_SESSION_OVER`,
  `OPUS_SESSION_TROUBLE`).  `design/soundcheck.md` has the contract,
  `design/ensemble-networking.md` the client's side.
- **The net code is `Assets/Code/Net/`**, namespace `Opus.Net`: the
  connection runs on threads of its own and never touches the screen; what
  it hears goes through `MainThread.Post()` to `Session`, which owns the
  flow and tells the screens through its events.  Keep-alives are sent from
  a thread, not `Update()`, so they go on with the window in the background.
  A packet change touches Ensemble's `Protocol.cs` and Soundcheck's both.
- **The screens are built from layouts by our own builder** (my "our UI
  builder" is that, not Unity's UI Builder): a screen's pieces are
  **widgets** (never "UI elements"), each a `Widget` with one line in
  `WidgetRegistry`, placed by a JSON layout in `Assets/Data/Layouts/`
  (HUD_FORMATS.md is the contract).  **ScreenRoot** owns every screen and
  which one is showing; a new screen gets its slots there, and each screen
  carries its own style sheet.  Only the HUD's layout is ever the player's;
  every other screen's is shipped.  `design/ensemble-hud.md` has the HUD,
  the chat window and its keys (EverQuest's: Enter or `/` to the field,
  Enter, Escape or a click away back to `GameFocus`, the game's
  place-holder for the focus, never to nothing).
- **Every file the game keeps for a player goes in one folder**,
  `~/.config/unity3d/FluffyByte/Opus.Ensemble/` on Linux and the same under
  `AppData\LocalLow` on Windows, through `PlayerFiles.PathOf()`; nothing
  builds a path of its own.  Soundcheck uses the same folder.
- **Sizes on the HUD are judged on Jacob's 1440p monitor**, in the layout's
  2560 x 1440 pixels (the chat window's first 350 x 200 was "holy shit" too
  small once seen).  A font size goes in the `.uss` as a plain number.
- **A number written out goes through `Translator.NumberToWords()`**
  (`Assets/Code/Translator.cs`), British with the "and" ("IN the honor of
  Discworld!").

## Launcher rules (Soundcheck)

- **Plain C# on .NET 10 with Avalonia 11**, and nothing else: the TLS, the
  hashing and the JSON are .NET's own.  Ask before any other package.
  Avalonia stays on the 11 line (12 is out and untried); a `NU1903` on a
  package it pulls in is fixed by the newest 11.3 patch first.
- **The login is Soundcheck's; the game is Ensemble's** ("remove login from
  the game like monsters and memories did").  Soundcheck does everything
  that's TCP, and **PLAY is a second login** whose Ticket starts the game
  with the ticket in its environment, never on its command line.  **The
  check comes first, before the login, and blocks it**; **the manifests and
  the files come from the web folder, not from Conductor**
  (PATCH_MANIFEST.md); **two modes, one program** (`--admin` publishes a
  build into the web folder); **`--debug` skips the check** and writes the
  ticket to a file for the editor's game.  `design/soundcheck.md` has all of
  it, with the shapes it went through in my words.
- **The ported files are Soundcheck's now** (`Net/`, `Security/`), with no
  Unity in them, and Ensemble's copies are gone, so a fix goes in one place;
  `Protocol.cs` and `Packets.cs` are in both, since the game speaks the UDP
  half.  Remember Me is one file, the same `remembered_login.json` in the
  same player folder.
- **The login's thread talks to the screen through `ILoginListener`**, and
  the screen puts every call back on the window's thread with
  `Dispatcher.UIThread.Post()`.  Anything slow (the key, hashing a folder)
  runs on a worker thread and reports back the same way, so the window
  never stops drawing.  **A worker's last progress message can land after
  the words that follow it**, so every progress message carries the phase
  it was sent in and a late one is dropped.
- **Soundcheck ends the process itself on Avalonia's `Exit` event**, before
  Avalonia's own shutdown runs (which dies on KDE's Wayland, Avalonia issue
  19523).  Anything that has to happen at close happens before then, not in
  a shutdown hook.
- **A platform guard the compiler knows is `OperatingSystem.IsWindows()`**,
  not our own; the CA1416 check on a Unix-only call only sees the first.
- **The certificate is a file beside the program**, `conductor.crt`, copied
  from `Soundcheck/dev/Certs/` at build.  **The version is the csproj's
  `<Version>`**, sent with the Login; `client_versions` has to list it, and
  it's one of RELEASE.md's places to bump.
- **Soundcheck says what it does on the terminal** (`Log.cs`), one line a
  message, UTC with a `Z`.  Nothing logs a password, a key or a token.
- **Jacob builds and runs it**: `dotnet build` and `dotnet run --project`
  with the absolute path, from `Conductor/dev`.  No `.meta` round, so a
  reply gives the plain `git add` of `Soundcheck/dev`.  A test of
  Soundcheck starting the game needs a game built from the code being
  tested (`Ensemble/build/Linux/<version>/`), and a reply that changes
  Ensemble's half says "a fresh build first".

---

## Git rules

- **Three branches.**
  - `unstable` is where you write.  Every session commits and pushes here.
    **First thing every session: `git fetch origin` and make sure the work
    starts from `origin/unstable`'s tip**, whatever branch the session was
    opened on (sessions have started eleven and sixty-four commits behind).
  - `testing` is where I test.  **When a round of edits is done and you want
    me to test it, push `unstable` onto `testing` yourself** (`git push
    origin unstable:testing`), then tell me what to run.  Don't wait to be
    asked.
  - `main` is the stable release.  It moves only when I say so, from
    `testing`, never from `unstable`.  Never push to `main` on your own.
- The session-branch-and-pull-request way is over, and the generated branch
  a session opens on is never pushed to.
- If I've pushed to `unstable` from my machine, fetch and merge it before
  pushing.  Never rebase or force-push over my commits, on any branch.  The
  same goes for `main`: I sometimes commit there from my machine
  (`bind_address = 10.0.0.84` in `networking.cfg` is one, correct).  When
  `main` has a commit `unstable` doesn't, merge `main` into `unstable`.
- **A release**: `main` is fast-forwarded from `testing` on my say (`git
  push origin origin/testing:main`), then the tag (the version as it is, no
  `v`) and the packages are mine to make by `Documentation/HowTo/RELEASE.md`.
  The version is in four places that all change together: every crate's
  `Cargo.toml` and `Cargo.lock`, Unity's `bundleVersion`, Soundcheck's
  `<Version>`, and the tag.  A fix to a released version is a new tag, never
  the old one moved.
- **The session's clone is shallow.**  `git fetch --deepen=200 origin
  unstable` (or `--unshallow`) before trusting a branch count or a
  merge-base; a shallow clone once said `main` and `testing` had no common
  ancestor when they were a plain fast-forward.
- Deleting a branch on GitHub can't be done from the session, so I do that
  by hand when one is finished with.
- **On the Windows laptop, git is Git GUI** (a clone in `C:\TEMP\download2`).
  Git steps for that machine are Git GUI's menus, with the one-line command
  beside them.
- **Tell me explicitly when to touch git from my terminal, and give the
  exact commands.** I don't keep the branch model in my head; you do.  Every
  time one of these happens, the reply says so in a line of its own:
  - You've pushed to `testing` and it's my turn to test: `git fetch origin`
    and `git checkout testing` (or `git pull` if I'm already on it), then
    the build and run commands.
  - You've pushed to `unstable` and I've hand-edited or built on my machine:
    `git pull` on `unstable`, so my copy matches before I do anything.
  - A build changed `Cargo.lock`, or Unity made `.meta`s: `git meta`.
  - I've said to release: the commands that move `main` to `testing`.
  - A branch needs deleting on GitHub: the `git push origin --delete` line.
  If nothing on my side needs doing, say that too ("nothing to run in git"),
  so silence never means I missed something.
- The whole `Opus/` folder is one **private** repo: code, docs, and assets.
  It must stay private -- it holds purchased art that can't be
  redistributed.  Never suggest making it public or pushing it anywhere else.
- Never commit build output, logs, the game's save, purchased art, or engine
  caches: both `build/` folders, `Content/Assets/`, `Content/logs/`,
  `Content/world/`, Rust `target/`, Unity's `Library/` `Temp/` `Obj/`
  `Logs/`, and its `.csproj` and solution files.  If something like that
  shows up in `git status`, tell me and suggest a `.gitignore` line.
- Large binary assets (models, textures, audio, `.blend`, `.unitypackage`) go
  through Git LFS.  If you see one about to be committed without LFS, flag
  it.
- The hand-off ends with the work committed and pushed to `unstable`, and
  onto `testing` if it's ready for me to test.
