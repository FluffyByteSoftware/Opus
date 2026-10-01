<!--
File:       Opus/README.md
Component:  Opus
Author:     Jacob Chacko
-->

# Opus

Opus is a multiplayer game I'm building as a hobby.  It's two programs: **Conductor**, the server, and
**Ensemble**, the client players run.  Conductor is authoritative -- it owns the game state, clients ask,
and Conductor decides.

It is early.  The server has its foundations, a login, a web page to run it from, a world of blocks and a
game loop ticking over it, but nobody can stand in that world yet, and Ensemble is only its project
settings.  Things will change and things will break.

**The first goal, 0.0.1, is a player spawned in the world and able to chat.**

## Where it stands

| Piece                                            | State                                                  |
|--------------------------------------------------|--------------------------------------------------------|
| The tools: disk, log, config, hashing, database  | Built and tested                                       |
| The web admin, the only way to run Conductor     | Built and tested                                       |
| The login over TLS, then a session over UDP      | Built and tested; a player logs in and stays connected |
| Accounts, made by the admin                      | Built and tested                                       |
| The monitor: CPU, memory, disk, threads          | Built and tested                                       |
| Lua scripting                                    | First step: scripts run on START SERVER, locked down   |
| The game library (entities and components)       | Built and tested; ready for characters to come in      |
| The character: its template, its save, its table | Built and tested                                       |
| Character select: list, make, delete, reset home | Built and tested with the test client                  |
| The world (GameWorld)                            | Made and loaded; nothing changes a block yet           |
| The game loop (the GameClock)                    | Ticking; takes characters in and out, saves the world  |
| Spawning in the world, movement, chat            | Not started                                            |
| Ensemble                                         | Unity project settings only                            |

Conductor is written and tested on Linux (Nobara and Fedora).  It builds and runs on Windows too, START
SERVER included, but hasn't met a database there yet.

## How it's put together

**Conductor and the server are two things.**  Conductor is the program: run it, and it brings up only the
disk manager, the log, the config files and the web admin.  The door is closed.  The server is the rest
-- the password hasher, the database, the world, the game loop, the network -- and none of it runs until
somebody presses START SERVER on the web admin.  STOP SERVER takes it back down with Conductor still
running.  So a changed port is a STOP and a START away (a **soft reboot**), and only a few settings need
Conductor itself run again (a **hard reboot**).

Conductor is a Cargo workspace of ten crates, one folder each under `Conductor/dev/`:

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
- **networking** -- the front door.  TCP is only the login: TLS 1.3, a username and password, and the
  player gets a ticket for UDP, where everything after happens, starting with character select (list,
  make, delete, reset home), answered by Protogame on a thread of its own.  A whitelist and a blacklist,
  which take at once without a reboot, because a ban that waited for a STOP SERVER wouldn't be much of a
  ban.
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
- **gameworld** -- the ground.  8 km a side, seamless, in blocks 50 cm a side (a player is 4 blocks
  tall), chunks of 32 blocks a side, two chunks tall.  Regions are biomes: Alpha to the west of 0,0,0 is
  flat, Omega to the east rolls in hills, and the block at 0,0,0 is gold.  The first START SERVER makes
  the world (about 20 seconds); every one after reads it from `Content/world/`.
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

The design behind each piece is in `Documentation/LLM/design/`, and what the server and a client say to
each other, byte for byte, is `Documentation/LLM/PROTOCOL.md` (version 5).

## What it needs

- **Rust**, edition 2024, 1.88 or newer.  Four crates from outside: `postgres`, `argon2`, `rustls` and
  `mlua`.  Everything else is the standard library and what the OS already has.
- **A C compiler**, because `mlua` builds Lua from source.  `gcc` on Linux; on Windows, Visual Studio's
  Build Tools and Rust's MSVC toolchain (`Documentation/HowTo/WINDOWS_INSTALL.md` walks through it).
- **PostgreSQL 18**, on the same machine (every table's `uuid` falls back on its `uuidv7()`).
- **openssl**, once, to make the TLS certificate.
- **Python 3**, for the test client.

## Running it

**The TLS certificate.**  Conductor doesn't make one.  Make it once from the `Opus` folder; the key stays
out of git, and the certificate goes in, since a client needs a copy to trust:

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
| `game.cfg`              | soft   | `view_chunks`: how far around a player the world is loaded         |
| `whitelist.cfg`         | --     | One address or range a line.  A change from the page takes at once |
| `blacklist.cfg`         | --     | The same, for the blacklist                                        |

## Talking to it

Until Ensemble can, `Conductor/dev/networking/test_client.py` stands in for it: Python 3, standard library
only.  It logs in, takes the ticket to UDP, lists the account's characters, keeps alive, and prints every
packet both ways.  `--create Name`, `--delete Name` and `--reset-home Name` do the rest of character
select.  Make a test account on the web admin's Accounts tab first (players can't make one), then, from the
`Opus` folder:

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
│   └── dev/Opus.Ensemble/         the Unity 6000.6 project (its project settings, for now)
├── Content/                       what both programs read and write
│   ├── cfg/                       the config files and the two access lists
│   ├── certs/                     the TLS certificate (committed) and its key (never)
│   ├── scripts/                   the Lua scripts
│   ├── psql/                      the tables as first made, and every change since, numbered
│   ├── world/                     the game's save: region.map and the regions' files, never committed
│   ├── logs/                      one log file per UTC day, never committed
│   └── Assets/                    the purchased art, never committed
└── Documentation/
    ├── HowTo/                     how-tos, like building on Windows
    └── LLM/                       the working docs: status, TODOs, the protocol, region.map, the designs
```

Build output, logs, the game's save, the purchased art and the engines' caches stay out of git.  Large
binary assets go through Git LFS.  The repo is private and stays that way: the art in it can't be
redistributed.
