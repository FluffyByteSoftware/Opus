<!--
File:       Opus/README.md
Component:  Opus
Author:     Jacob Chacko
-->

# Opus

Opus is a multiplayer game I'm building as a hobby.  Opus is the project's codename; the game itself is
**Forgotten Legends**.  It's three programs: **Conductor**, the server; **Ensemble**, the game players
run; and **Soundcheck**, the launcher they open first, which logs them in, checks the game's files and
starts Ensemble.  Conductor is authoritative -- it owns the game state, clients ask, and Conductor decides.

It is early.  The server has its foundations, a login, a web page to run it from, a world of blocks and a
game loop ticking over it, and a player can pick a character, stand in that world and chat with whoever
else is there, though nothing moves yet.  The server has the say on where everything stands: each player
is sent everyone in their view, and a character saved inside the ground is stood back on top of it.
Ensemble has an editor tool for the art, and screens built from layout files: it starts with the launcher's
ticket, makes, deletes and picks a character at character select, and puts it in the world on the ground
drawn around it, with everyone nearby drawn standing there with their name over their head (a capsule each
until there are models), and a HUD the player can move, lock and (the chat window) resize.
Soundcheck checks the game's files against the manifest its admin mode published to a web folder (1.3 s
for 655 MB), fetches whatever's off a file at a time, logs in over TLS 1.3 with the password turned into a
key before it's sent or kept, and starts Ensemble with the ticket; all of it built and tested on Linux,
none of it on Windows yet.  Things will change and things will break.

**0.0.1 is released (2026-10-02): a player logs in, picks a character, and stands in the world chatting.**
The next is 0.0.13, the world served up to the client; the versions ahead are in
`Documentation/LLM/WAYPOINTS.md`.

## Where it stands

| Piece                                            | State                                                  |
|--------------------------------------------------|--------------------------------------------------------|
| The tools: disk, log, config, hashing, database  | Built and tested                                       |
| The web admin, the only way to run Conductor     | Built and tested                                       |
| The login over TLS, then a session over UDP      | Built and tested; a player logs in and stays connected |
| Accounts, made by the admin                      | Built and tested                                       |
| The monitor: CPU, memory, disk, threads          | Built and tested                                       |
| Lua scripting                                    | First step: scripts run on START SERVER, locked down   |
| The game library (entities and components)       | Built and tested; players' characters come and go      |
| The character: its template, its save, its table | Built and tested                                       |
| Character select: list, make, delete, reset home | Built and tested with the test client                  |
| Spawning in the world, and leaving it, saved     | Built and tested with the test client                  |
| The world (GameWorld)                            | Made and loaded, a density in every voxel (step 1 of   |
|                                                  | smooth voxels); nothing changes a block yet            |
| The game loop (the GameClock)                    | Ticking; takes characters in and out, saves the world  |
| Chat: `/chat` to everybody in the world          | Built and tested with the test client                  |
| `/who`, the anti-flood                           | Built and tested                                       |
| Ensemble                                         | An editor tool; the screens and the HUD, from layouts  |
| Ensemble's character select and PLAY             | Built and tested; the HUD comes up over the scene      |
| Ensemble's chat window, `/who`'s box, `/camp`    | Built and tested, EverQuest's keys included            |
| The HUD moved, locked, chat resized, by a player | Built and tested: right-click LOCK, chat's font size,  |
|                                                  | a layout file a character                              |
| The password's key, made on the client           | Both halves built and tested (protocol version 7)      |
| Ensemble starting from the launcher's ticket     | Built and tested: the start screen, dev mode           |
| A pick inside the character's lock waits         | Built and tested (PleaseWait, protocol version 10)     |
| The world's map sent at PLAY                     | Dropped (protocol 17): nothing is drawn past the view  |
| The chunks around the player, client-pulled      | Built and tested (protocol 12), Ensemble and the test  |
|                                                  | client; the player waits until the nearest are drawn   |
| The ground on screen in Ensemble                 | Built and tested: a mesh a chunk, cubes until smooth   |
| Spawn points: on top of the highest block        | Built and tested: new characters and RESET HOME        |
| Characters seen in the world, the server's say   | Built and tested (protocol 14): everybody in view as a |
|                                                  | capsule with their name over it, the camera on yours   |
| A character saved inside the ground              | Built and tested: stood on top of its column at PLAY   |
| Movement: the server judges every move           | Built and tested (protocol 15), Conductor and the test |
|                                                  | client: it walks, the server pulls back; Ensemble next |
| Smooth voxels, stage 1                           | Being built: a density in every voxel (protocol 16-17) |
| Soundcheck, the launcher                         | Built and tested on Linux: the login over TLS 1.3,     |
|                                                  | Remember Me, PLAY and the way back, debug mode, admin  |
|                                                  | mode's PUBLISH, the check at start, the patch          |

Conductor is written and tested on Linux (Nobara and Fedora).  It builds and runs on Windows too, START
SERVER included, but hasn't met a database there yet.

## Installation

To run a released version rather than build one, get the two packages from the repo's Releases page
(private, like the repo) and follow
[INSTALLATION_INSTRUCTIONS.md](Documentation/HowTo/INSTALLATION_INSTRUCTIONS.md).  In short:

1. **The server** needs a Linux machine with PostgreSQL 18 and `openssl`.  Unpack `Opus-Conductor-0.0.1`,
   make the database and its role (two lines of SQL; Conductor makes its own tables), put the TLS key
   beside the certificate in `Content/certs/`, set the Postgres password, the address to listen on and the
   web admin's passwords in `Content/cfg/`, and run `conductor-launcher`.  The web admin is at
   <http://127.0.0.1:9996/Opus> on that machine: START SERVER, wait for the world, make the players'
   accounts on the Accounts tab.
2. **A player** unpacks `Opus-Ensemble-0.0.1`, runs it, types the server's address and port on the login
   screen, and logs in with the account the admin made.  (That's the released 0.0.1.  Since then the login
   has moved into Soundcheck, the launcher, which will start the game; the next release ships the two
   together.)

How a release is made, tag and packages, is in [RELEASE.md](Documentation/HowTo/RELEASE.md).

## How it's put together

**Conductor and the server are two things.**  Conductor is the program: run it, and it brings up only the
disk manager, the log, the config files and the web admin.  The door is closed.  The server is the rest
-- the password hasher, the database, the world, the game loop, the network -- and none of it runs until
somebody presses START SERVER on the web admin.  STOP SERVER takes it back down with Conductor still
running.  So a changed port is a STOP and a START away (a **soft reboot**), and only a few settings need
Conductor itself run again (a **hard reboot**).

Conductor is a Cargo workspace of eleven crates, one folder each under `Conductor/dev/`:

- **tools** -- the pieces everything leans on, each with a name since they come up a lot.  **DiskMan**
  does every file read and write on a thread of its own, so nothing waits on the disk.  **Scribe** is the
  log.  **Constellations** owns every config file.  **Fingerprinter** makes version 7 UUIDs and tokens.
  **Security** hashes passwords (Argon2id), one at a time, with everybody else in line, since fifty
  logins hashing at once on fifty threads is what blew the tick in the last server I wrote.
  **Archivist** is the database, PostgreSQL on its own thread; callers get the answer back later.
- **accounts** -- the one way in to the accounts table and the players' characters.  An account is never
  held in memory: it's read from its row when needed and every change goes straight back, so there's only
  ever one copy.  Three characters to an account, each with a name that's unique on the server.
- **monitor** -- once a second, CPU, memory, disk and threads for Conductor and the machine.
- **networking** -- the front door.  TCP is only the login: TLS (1.3; 1.2 was let in while the Unity
  client logged in itself, and is on its way out), a
  username and password, and the player gets a ticket for UDP, where everything after happens, starting
  with character select (list, make, delete, reset home), answered by Protogame on a thread of its own,
  and then the pick: the character is loaded from its save and put in the world, and taken out and saved
  when the player goes.
  A whitelist and a blacklist, which take at once without a reboot, because a ban that waited for a STOP
  SERVER wouldn't be much of a ban.
- **player-commands** -- what a player types once they're in the world: `/chat` and `/who` so far, one
  file each in a table with the anti-flood, so a new command is a file and a line.  Networking hands it
  each line and never names the crate; the launcher wires the two together.
- **lua-parser** -- the game's content is going to be written in Lua 5.4.  For now every script under
  `Content/scripts/` runs once on START SERVER, with no way to reach the disk, the network or the
  database, and a time limit, a memory limit and a cap on its log lines, so a bad quest can't be a bad
  server.
- **primlib** -- the game library, an ECS (entity, component, system).  An entity is just a number, and
  the components on it (a position, a health pool, a name) make it a goblin or a sword.  A template
  (`NPC`) sets out components and defaults, "so I don't write the same 50 lines in 50 npcs", and a
  blueprint (`goblin_a`) starts from one and changes what it needs.  A template can take in another whole,
  like `inherit STD_LIVING;` in the old Discworld mudlib: a player's Character takes in Living (a name,
  health, endurance and mana).  A character is saved as Lua text in its database row, each component
  naming the fields it keeps, and made back from the template with the save laid over it.
- **gameworld** -- the ground.  16,384 blocks a side to start (`world_size` in `game.cfg`), seamless, in
  blocks 1 m a side, Minecraft's size (a player is 2 blocks tall), chunks of 32 blocks a side, eleven
  chunks tall: BEDROCK at -31, up to +319.  Regions are biomes: Alpha to the west of 0,0,0 is flat, Omega
  to the east rolls in hills, and the block at 0,0,0 is gold.  The first START SERVER makes the world;
  every one after reads it from `Content/world/`, unless `world_size` has changed, which makes a new one.
- **gameclock** -- the GameClock, the game loop.  A cycle is 250 ms, five checks of 50 ms: input, AI,
  movement, broadcast, housekeeping.  The rate is fixed in code on purpose: from my testing on an earlier
  go at this, anything faster than 250 ms is a problem.  Nobody gets in, and nothing acts, until the
  ground around 0,0,0 is loaded.  Players' characters come into the world and leave it through a
  mailbox, and the world is saved every `world_save_seconds` (`game.cfg`, 2.5 minutes to start): one
  cycle copies every character, and the database writes them behind the game loop.
- **wgui** -- the web admin at `http://127.0.0.1:9996/Opus`, and the only way to run Conductor (the
  console shows the log and takes no input).  It only listens on this machine.  Two logins: `admin` does
  everything, `user` looks.  Five sections across the top: CONTROL PANEL (start and stop, the machine,
  Conductor, the services, storage), CONFIGURATION (every setting, the two access lists), LOGS,
  ACCOUNT MANAGEMENT and GAME MANAGEMENT (who's connecting and who's in the world, with KICK, and every
  player's character).
- **launcher** -- the program itself.  Boots, then starts and stops the server on the web admin's say.

**Soundcheck** is a .NET 10 program with an Avalonia window, in `Soundcheck/dev/`.  The idea is the one
Monsters and Memories uses: the launcher logs you in, not the game.  It does the TLS login and gets the
ticket, checks every file of the installed game against the manifest for its platform
(`linux_manifest.json` or `windows_manifest.json`, written by Soundcheck's own admin mode, which also
copies the folder we ship into the web folder at `http://opusensemble.duckdns.org:8553/download/`), fetches
what's wrong a file at a time, and starts Ensemble with the ticket, which goes straight to character select
over UDP.  When the session ends, Ensemble starts Soundcheck again with the reason and closes.  All of it
is built and tested on Linux; none of it on Windows yet.

The design behind each piece is in `Documentation/LLM/design/`, and what the server and a client say to
each other, byte for byte, is `Documentation/LLM/PROTOCOL.md` (version 11).  The manifest's shape is
`Documentation/LLM/PATCH_MANIFEST.md`.

## What it needs

- **Rust**, edition 2024, 1.88 or newer.  Six crates from outside: `postgres`, `argon2`, `pbkdf2` and
  `sha2` in the tools, `rustls` in networking and `mlua` in the Lua parser.  Everything else is the
  standard library and what the OS already has.
- **A C compiler**, because `mlua` builds Lua from source.  `gcc` on Linux; on Windows, Visual Studio's
  Build Tools and Rust's MSVC toolchain (`Documentation/HowTo/WINDOWS_INSTALL.md` walks through it).
- **PostgreSQL 18**, on the same machine (every table's `uuid` falls back on its `uuidv7()`).
- **openssl**, once, to make the TLS certificate.
- **Python 3**, for the test client.
- **.NET 10**, for Soundcheck.  The Avalonia packages come down from NuGet on the first build.

## Running it

This is running Conductor from a build of your own.  To install a released package instead, the server and
the client both, see [INSTALLATION_INSTRUCTIONS.md](Documentation/HowTo/INSTALLATION_INSTRUCTIONS.md); how a
release is made is in [RELEASE.md](Documentation/HowTo/RELEASE.md).

**The TLS certificate.**  Conductor doesn't make one.  Make it once from the `Opus` folder; the key stays
out of git, and the certificate goes in, since the launcher needs a copy to trust (Soundcheck carries it as
`Soundcheck/dev/Certs/conductor.crt`, so a new certificate is copied there too; Ensemble never speaks TLS
and has none):

```
mkdir -p Content/certs && openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes -keyout Content/certs/conductor.key -out Content/certs/conductor.crt -days 3650 -subj "/CN=Opus Conductor" -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
```

Without it Conductor still runs, the network stays down, and the log gives the same command with the full
paths filled in.

**The database.**  Postgres wants a database `opusdb` and a role `opus_game` with the password in
`Content/cfg/postgres.cfg`, allowed to create tables in `public`.  Conductor makes its own tables the first
time it connects.  Keep Postgres listening on `localhost` only: that file holds its password as it is.
Without Postgres, Conductor still runs and the web page says the database is offline.

**Build, test and run**, from `Conductor/dev`:

```
cargo build
cargo test
cargo run -p conductor-launcher
```

Then open <http://127.0.0.1:9996/Opus>, log in as `admin` (password `admin` out of the box, in
`Content/cfg/wgui.cfg`), and press START SERVER.  SHUT DOWN on the same tab closes Conductor cleanly.
Ctrl-C kills it outright and can lose whatever hadn't been written yet.

Conductor finds `Content/` by walking up from wherever it's run.  If it can't, point it there with
`OPUS_CONTENT=/path/to/Opus/Content`.

The password benchmark only runs when asked, and always optimized, since Argon2 is six times slower
without: `cargo test -p conductor-tools --release argon2_cost -- --ignored --nocapture`

**Soundcheck**, from the same terminal:

```
dotnet build /opt/storage/Coding/Opus/Soundcheck/dev
dotnet run --project /opt/storage/Coding/Opus/Soundcheck/dev
dotnet run --project /opt/storage/Coding/Opus/Soundcheck/dev -- --admin
```

The first opens the login (after checking the game's files); the second, admin mode, which copies a build
folder into the web folder and writes its manifest there, for the platform you tick.  What it says as it
goes is on the terminal.

## The config files

All in `Content/cfg/`.  Conductor writes any that's missing with its defaults, and every setting's comment
is in the file and on the web admin's Settings tab.  A change from the page waits beside the file as
`name.cfg.wait4server` until the reboot the file needs, so the live file always says what Conductor is
running on.

| File                    | Reboot | What's in it                                                       |
|-------------------------|--------|--------------------------------------------------------------------|
| `conductor_globals.cfg` | hard   | Where the log goes, and the web admin's port (9996)                |
| `wgui.cfg`              | hard   | The web admin's two passwords                                      |
| `postgres.cfg`          | soft   | Where Postgres is, how to log in, the time limits                  |
| `networking.cfg`        | soft   | Ports, TLS files, secret word, client versions, which access list  |
| `game.cfg`              | soft   | `world_size`, `view_chunks`, `world_save_seconds`                  |
| `whitelist.cfg`         | --     | One address or range a line.  A change from the page takes at once |
| `blacklist.cfg`         | --     | The same, for the blacklist                                        |

## Talking to it

Soundcheck logs in and Ensemble does the rest.  For poking at the server without them,
`Conductor/dev/networking/test_client.py` stands in for both: Python 3, standard library only.  It logs in,
takes the ticket to UDP, lists the account's characters, keeps alive, and prints every packet both ways.
`--create Name`, `--delete Name` and `--reset-home Name` do the rest of character select, and `--play Name`
brings that character into the world.  `--type '/chat Yo yo yo!'` types a line in the
chat window once it's there (`/who` too), and every chat it hears is printed.  Make a
test account on the web admin's Accounts tab first (players can't make one), then, from the `Opus` folder:

```
python3 Conductor/dev/networking/test_client.py some_account 'Its password 1!'
```

`--help` lists its switches: another host or port, another certificate, a wrong version or secret word,
saying Goodbye after a while, going quiet to watch the timeout, and more.

## Layout

```
Opus/
├── Conductor/
│   ├── dev/                       the Cargo workspace, one folder per crate (above)
│   └── build/                     compiled output, never committed
├── Ensemble/
│   └── dev/Opus.Ensemble/         the Unity 6000.6 project: its settings, and Assets/Editor, Code, Scripts and Data
├── Soundcheck/
│   ├── dev/                       the .NET 10 project: the login, the manifest, the two screens
│   └── build/                     compiled output, never committed
├── Content/                       what both programs read and write
│   ├── cfg/                       the config files and the two access lists
│   ├── certs/                     the TLS certificate (committed) and its key (never)
│   ├── scripts/                   the Lua scripts
│   ├── psql/                      the tables as first made, and every change since, numbered
│   ├── world/                     the game's save: region.map and the regions' files, never committed
│   ├── logs/                      one log file per UTC day, never committed
│   ├── patch/                     spare; the manifests live in the web folder now, never committed
│   └── Assets/                    the purchased art, never committed
└── Documentation/
    ├── HowTo/                     how-tos: installing a release, making one, building on Windows
    └── LLM/                       the working docs: status, TODOs, the protocol, region.map, the designs
```

Build output, logs, the game's save, the purchased art and the engines' caches stay out of git.  Large
binary assets go through Git LFS.  The repo is private and stays that way: the art in it can't be
redistributed.
