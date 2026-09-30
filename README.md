<!--
File:       Opus/README.md
Component:  Opus
Author:     Jacob Chacko
-->

# Opus

Opus is a multiplayer game I'm building as a hobby.  It's two programs: **Conductor**, the server, and
**Ensemble**, the client players run.  Conductor is authoritative -- it owns the game state, clients ask,
and Conductor decides.

It is early.  What exists today is the server's skeleton: the tools every other piece leans on, a monitor,
a web page to run it all from, and a front door that logs a player in over TLS and hands them a ticket to
UDP, where the game will go.  There is no game yet, accounts are made by the admin on the web admin
only, and Ensemble isn't in the repo.  Things will change and things will break.

## What's built

### Conductor and the server are two things

Conductor is the program.  When it's run, it brings up only what the admin needs to run everything else:
the disk manager, the log, the config files and the web admin.  That's it.  The door is closed.

The server is the rest -- the UUID maker, the password hasher, the database connection, the network and
the monitor, and the game once there is one.  None of it runs until somebody presses START SERVER on the
web admin's Control Panel, and STOP SERVER takes it all back down with Conductor still running.  So a
changed database password or a new port is a STOP and a START away, not a restart of the whole program.

That gives two kinds of reboot.  A **soft reboot** is STOP SERVER and START SERVER (or RESTART SERVER):
the server comes down and back up, and the program and the page stay put.  A **hard reboot** is Conductor
shut down and run again.  Every config file is one or the other as a whole.

### The tools (`conductor-tools`)

The pieces that know nothing about the game but that everything else leans on.  Each has a name, since
they come up a lot.

- **DiskMan** -- every file Conductor reads or writes goes through it, on one thread of its own, so
  nothing else ever waits on the disk.  Writes are held in memory and marked dirty until they're out, a
  second write of the same file before the first lands just replaces it, and a whole-file write goes to
  a temp file and is renamed over the old one, so a crash halfway can't leave half a file.  Reads are
  cached; clean files are let go past 256 MB.  A big write goes out 1 MB at a time so it never holds up a
  log line.  It starts first and stops last, and shutdown waits for it.
- **Scribe** -- the log.  One file per UTC day (`Content/logs/2026_09_29.scribe.log`), the console, and
  the last 200 lines in memory for the web page.  Every line says when, which channel (System, Network,
  Security, Database, Game), how bad (Debug, Info, Warn, Error), and the file and line that logged it.
  Scribe never panics and never hands an error back: a log that takes the server down is worse than no
  log.
- **Constellations** -- every config file.  One format (`key = value`, `#` comments), one reader, and one
  table in the code where every file and every setting is written down with its kind, its default and
  its comment.  A missing file is written with the defaults, and a missing setting is added to the end.
  A change from the web page never touches the live file: it waits beside it as `name.cfg.wait4server`
  until the reboot that file needs, and DiskMan swaps it in on the way down.  So the live file always
  says what Conductor is running on.
- **Fingerprinter** -- version 7 UUIDs (the time first, so they sort in the order they were made) and
  login tokens, from the OS's own random source.
- **Security** -- password hashing, Argon2id at 64 MiB, one pass, one lane: about 30 ms a login on my
  machine.  One worker, one hash at a time, hard limit, with everybody else in line; fifty logins hashing
  at once on fifty threads is what blew the tick in the last server I wrote.  It keeps its 64 MiB for as
  long as the server runs rather than asking the OS for it on every hash, which saves about a fifth.  A
  name with no account costs a hash too, and every login takes at least 150 ms, so a stopwatch can't tell
  a real name from a made-up one.
- **Archivist** -- the database: PostgreSQL on its own thread, one connection, jobs in order.  Callers
  get the answer back later and never wait on it.  On every connect it runs the table schemas and then
  any migration that hasn't run yet, each once, in number order, in a transaction.  A slow job is a
  warning with its time and the start of its SQL, never its values.
- **Notices** -- every Warn and Error becomes a notice on the web page's bell, and stays there until
  somebody acknowledges it.  Memory only, since Conductor started.
- **Threads** and **Services** -- every thread Conductor starts is on a list with who started it and
  when, and every service it expects reports on itself (starting, running, trouble, stopped, with a note
  saying why).  Both show on the web page.
- **The server's switch** -- stopped, starting, running, stopping, and a mailbox the web page's buttons
  post to.  Two clicks in a row can't both get through.
- **The clock** -- UTC dates and times, worked out by hand, since the standard library stops at seconds
  since 1970.  All time in Opus is UTC, and any time shown to a person ends with `Z`.

### Accounts (`conductor-accounts`)

The one way in to the `accounts` table.  An account is never held in memory: whatever needs one reads it
from its row when it needs it, and every change goes straight back to the row, so there's only ever one
copy and nothing can write an old one over a new one.  A player in the world is just their account's name
to the server.  Their last login is written the moment they come in over UDP, not at the TLS login before
it, since that's when playing starts.  The password hash is read on its own, only by the login, so it can
never end up in a log line.

Accounts are made by the admin on the web admin's Accounts tab, never by players.  Making one and
changing a password both need a hash, which waits in Security's line with the logins, so those go to the
account desk, a thread of its own, and the page asks after the job until it's done.

### The monitor (`conductor-monitor`)

Once a second, on its own thread, it looks at Conductor and at every process on the machine: CPU (for the
process, per thread, and per core for the machine), memory against the machine's total, disk read and
written, and every thread with its CPU time.  It talks to the OS through what's already there -- `/proc`
on Linux, kernel32 on Windows -- with no crate.  It measures and the web admin shows; it knows nothing
about HTTP.

### The front door (`conductor-networking`)

TCP is only the login, and UDP is everything after.

1. A client connects to the TCP port (9997) and TLS 1.3 comes up.  The server says Hello with the
   protocol version, and the client sends one Login: its version, the secret word, the name and the
   password.
2. The cheap checks go first -- an old client, a wrong secret word, a name that can't be one -- so none of
   those cost a hash.  Then the password goes into Security's line, and the client is told once a second
   how many logins are ahead of it and about how long.
3. If the account is already in the world, the client is asked whether to log the other session out or
   hang up.  (So a shared account doesn't kick your brother off because you wanted to play.)
4. A good login gets a **ticket**: a one-time token and the UDP port (9998).  The TCP connection closes.
5. The client takes the token to UDP, and from then on it's a keep-alive each way once a second (the
   server echoes every one, so a client can tell when the server's gone), and the game once there is
   one.

When the UDP session ends, for any reason, the player is gone and starts over at the login screen.
Nothing is kept for a reconnect.  A player quiet for 40 seconds is dropped; one who says Goodbye, is
logged out from elsewhere, is banned, or is there when the server stops is told why first.

The rest of the door:

- **A fixed pool of login threads**, not a thread per connection, with a queue in front and a cap on it.
  No thread polls; every wait is the OS's own, armed with exactly the time left.
- **A failed login puts the address on a 2-second hold**, at the door, not in a sleeping thread.
- **A ledger** of every connection that reached the door since START SERVER, with where each one is (the
  queue, TLS, Security's line with its place, and how it ended), and reverse DNS looked up on a thread
  of its own.  By address only, never by account.
- **A whitelist and a blacklist** of addresses and ranges (`1.2.3.4`, `1.2.3.0/24`), and a switch saying
  which one the door checks, or neither.  A change from the web page takes at once and rewrites the file,
  the one thing in Conductor that doesn't wait for a reboot, because a ban that waited for a STOP SERVER
  wouldn't be much of a ban.  Blacklisting an address while the blacklist is on, or taking it off the
  whitelist while the whitelist is on, drops everybody at that address on the spot.

The whole contract, byte for byte, is in `Documentation/LLM/PROTOCOL.md`.  It's version 4.

### Lua (`lua-parser`)

The game's content is going to be written in Lua 5.4, run inside Conductor through the `mlua` crate, with
Lua built from source along with it.  For now it's the first step: on START SERVER every `.lua` file
under `Content/scripts/` (and the folders inside it) runs once, each in a Lua of its own, and
`hello.lua` says hello on the Log tab.  RESTART SERVER runs them again, so a changed script takes then.

A script can't reach the disk, the network or the database: Lua's `io`, `os`, `package` and `debug`
libraries are never loaded, and `dofile`, `loadfile`, `load` and `string.dump` are taken out.  It gets
`string`, `table`, `math`, `utf8`, `coroutine`, and a log: `log.debug()`, `log.info()`, `log.warn()` and
`log.error()` (`print()` is `log.debug()`).  Whatever a script says is wrong is a Warn, never an Error.

A script can't take the server down either.  One with an error is a Warn with its file and line, and the
rest still run.  One still going after a second is stopped, one that holds more than 64 MB is stopped,
and past 50 log lines in a run the rest are dropped.

### The game library (`conductor-primlib`)

"Prim" for primitive.  It's where the world's objects will live, as an ECS (entity, component, system): an
entity is only a number, and the components hung on it (a position, a health pool, a name) are what make
it a goblin or a sword.  A template (`NPC`) is a cheat sheet of components and their defaults, so I don't
write the same 50 lines in 50 NPCs, and a blueprint (`goblin_a`) starts from a template and changes what
it needs.  The world spawns copies of a blueprint, each with its own values.  For now it's Rust only; the
GameClock runs a world of it, and the templates and blueprints get written in Lua later.

### The GameClock (`conductor-gameclock`)

The game loop.  It owns the world and steps it forward on a fixed beat: a full cycle is 250 ms, cut into
five checks of 50 ms, and each check does its own job on its own group of objects.  In order: the
players' input, the AI's brains, movement, the positions going out to everybody, and housekeeping.  The
checks are empty for now, since nothing in the world moves yet.

The rate is fixed in the code on purpose.  From my testing on an earlier go at this, anything faster than
250 ms is a problem; slower is fine.  A check that runs long makes the next one late, and nothing is
skipped.  A cycle that runs past its 250 ms just starts the next one straight away, and the Services tab
counts how many did.  Password hashing stays on its own thread, so a login never makes a cycle late.

### The web admin (`conductor-wgui`)

A small web server on `http://127.0.0.1:9996/Opus`, and the only way to run Conductor: the console shows
the log and takes no input.  It only ever listens on 127.0.0.1, so it can't be reached from anywhere
else, and it pulls nothing from the internet.

It asks for a login first.  There are two accounts: `admin` can do everything, and `user` can look at
everything and change nothing.  Their passwords are in `Content/cfg/wgui.cfg` (`admin` and `user` out of
the box).  Every time Conductor starts, everybody starts logged out.  Until you're in, the page is only
the login.  With the server stopped, only the Server tab, the Log and the Settings open; the rest wait
for START SERVER.  So does the bell: a Warn at boot (a leftover line in a config file, say) is in the Log
tab meanwhile, and on the bell once the server is up.

The page is five sections across the top, and the side menu lists the tabs of the one that's open:

- **CONTROL PANEL**
  - **Server** -- the server's state, START SERVER, RESTART SERVER, STOP SERVER and SHUT DOWN, and a
    short list of the services.
  - **System** -- the whole machine: every core, memory, and every process, busiest first.  Click one to
    see its threads.
  - **Conductor** -- its own CPU with a 60-second chart per core, memory against the machine's, disk, and
    its threads (every one the OS knows about, and the ones we asked for, with who started them).
  - **Services** -- every service and how it says it's doing.
  - **Storage** -- Archivist's jobs and slow ones, and what DiskMan has waiting, held and written.
- **CONFIGURATION**
  - **Settings** -- every config file, a card each, drawn straight from Constellations' table.  A save
    waits for the file's reboot and says so.
  - **Whitelist** and **Blacklist** -- the two lists of addresses, with ADD and REMOVE.
- **LOGS**
  - **Log** -- the log as it's written, coloured by how bad each line is.
  - **Notifications History** -- every open notice, each with an ACK.  The bell's tray opens it too.
- **ACCOUNT MANAGEMENT**
  - **Accounts**, `admin` only -- every game account, and a card for each (click its name) to change the
    owner's names and email, give it a new password (typed twice), or delete it.  Deleting an account
    whose player is in the world takes them out, and the client says ACCOUNT TERMINATED.  NEW ACCOUNT
    makes one.  It only works while the server is running.
- **GAME MANAGEMENT**
  - **Connections** -- the door (every TCP connection since START SERVER, in a Recent view of the newest
    five and a Historical one of the whole run) and the world (every player on UDP, by account, with how
    long they've been in and how quiet they are).  Each connection has a menu to kick it or put its
    address on either list.

Until the server is running, only the Server tab, the Log and the Settings can be opened.  While the
server runs without a database, everything else is blurred and locked, since the game can't run without
it.  Connections, Whitelist and Blacklist also wait for both of the network's listeners to be up.

## What it needs

- **Rust**, edition 2024, 1.88 or newer.  Four crates from outside: `postgres` for the database, `argon2`
  for password hashing, `rustls` for TLS and `mlua` for Lua.  Everything else is the standard library and
  what the OS already has.
- **A C compiler**, because `mlua` builds Lua from its C source.  Nobara and Fedora have `gcc`; on Windows
  it's Visual Studio's (the Build Tools, with "Desktop development with C++"), and Rust has to be on its
  MSVC toolchain (`rustup default stable-x86_64-pc-windows-msvc`).  Rust's GNU toolchain stops at
  "error calling dlltool" unless MinGW is installed too.
  `Documentation/HowTo/WINDOWS_INSTALL.md` walks through it.
- **PostgreSQL 18**, on the same machine.  18 because every table's `uuid` column falls back on its
  `uuidv7()`.
- **openssl**, once, to make the TLS certificate.
- **Python 3**, for the test client.

Conductor is written and tested on Linux (Nobara and Fedora).  The Windows code is written behind the
same functions.  It builds and runs on Windows (`Documentation/HowTo/WINDOWS_INSTALL.md`), but hasn't been
tried there with a database yet.  macOS builds and runs, but the monitor can't measure anything there and
the door can't look up names.

## Running it

Everything below is from the repo root.

**The TLS certificate.**  Conductor doesn't make one.  Make it once; the key stays out of git, and the
certificate goes in, since a client needs a copy to trust.  From the `Opus` folder itself:

```
mkdir -p Content/certs && openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes -keyout Content/certs/conductor.key -out Content/certs/conductor.crt -days 3650 -subj "/CN=Opus Conductor" -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
```

Without it Conductor still runs.  The TCP side shows trouble, the UDP side stays stopped, and the log gives
the same command with the full paths filled in, so it works pasted from any folder.

**The database.**  Postgres wants a database `opusdb` and a role `opus_game` with a password that matches
`password` in `Content/cfg/postgres.cfg`, allowed to create tables in the `public` schema.  Conductor makes
its own tables the first time it connects, as `opus_game`, so the server owns them.  Keep Postgres
listening on `localhost` only: the config file holds its password as it is.  If Postgres isn't there,
Conductor still runs and the web page says the database is offline.

**Build, test and run:**

```
cd Conductor/dev
cargo build
cargo test
cargo run -p conductor-launcher
```

Then open <http://127.0.0.1:9996/Opus>, log in as `admin`, and press START SERVER.  SHUT DOWN on the same
tab closes Conductor cleanly.  Ctrl-C and closing the console kill it outright, and can lose whatever
DiskMan hadn't written yet.

Conductor keeps its data in `Content/` at the repo root and finds it by walking up from wherever it's run.
If it can't, point it there:

```
OPUS_CONTENT=/path/to/Opus/Content cargo run -p conductor-launcher
```

**The password benchmark.**  Argon2 is six times slower unoptimized, so it's a test that only runs when
asked, and always with `--release`:

```
cargo test -p conductor-tools --release argon2_cost -- --ignored --nocapture
```

## The config files

All of them live in `Content/cfg/`.  Conductor writes any that's missing with its defaults.  Each can be
edited by hand, or from the web page's Settings tab.

| File                    | Reboot | What's in it                                                          |
|-------------------------|--------|-----------------------------------------------------------------------|
| `conductor_globals.cfg` | hard   | Where the log goes, and the web admin's port (9996)                   |
| `wgui.cfg`              | hard   | The web admin's two passwords                                         |
| `postgres.cfg`          | soft   | Where Postgres is, how to log in, the time limits                     |
| `networking.cfg`        | soft   | Ports, TLS files, secret word, client versions, deadlines, which list |
| `whitelist.cfg`         | --     | One address or range a line.  Read on START SERVER, written at once   |
| `blacklist.cfg`         | --     | The same, for the blacklist                                           |

Every setting's comment is in the file itself, and on the Settings tab.

A hand edit takes at the file's reboot: STOP SERVER, edit, START SERVER for a soft one, and Conductor
shut down and run again for a hard one.  DiskMan notices the file changed on disk and reads it again.

## Talking to it

Until Ensemble exists, `Conductor/dev/networking/test_client.py` stands in for it: Python 3,
standard library only.  It logs in, takes the ticket to UDP, keeps alive, and prints every packet both
ways:

```
python3 Conductor/dev/networking/test_client.py some_account 'Its password 1!'
```

Its switches: `--host` and `--tcp-port` for another server, `--cert` for another certificate,
`--version` and `--secret` to claim a different client version or secret word, `--leave-other-alone` to
hang up rather than log out a session already in the world, `--leave-after N` to say Goodbye after N
seconds, `--go-quiet` to send nothing after connecting and watch the timeout drop it, and
`--pause-before-login N` to sit N seconds after TLS before the Login, so the connection can be caught open.

There's no way to make an account over the protocol, on purpose: a test account is made on the web
admin's Accounts tab.

## Ensemble

Unity 6000.6, in C#.  It's on my machine and not in the repo yet.

## Layout

```
Opus/
├── Conductor/
│   ├── dev/                       a Cargo workspace
│   │   ├── tools/                 DiskMan, Scribe, Constellations, Security, Archivist and the rest
│   │   ├── accounts/              the accounts table, read on demand, never held; the account desk
│   │   ├── monitor/               looks at the process and the machine once a second
│   │   ├── networking/            the login over TLS, the game over UDP, the access lists; test_client.py
│   │   ├── lua-parser/            runs the Lua scripts, locked down
│   │   ├── primlib/               the game library: entities, components, templates and blueprints
│   │   ├── gameclock/             the GameClock: the game loop, five checks of 50 ms to a 250 ms cycle
│   │   ├── wgui/                  the web admin
│   │   └── launcher/              the program: boots, then runs the server on the Control Panel's say
│   └── build/                     compiled output, never committed
├── Ensemble/                      the client (on my machine, not in the repo yet)
├── Content/
│   ├── cfg/                       the config files and the two access lists
│   ├── certs/                     the TLS certificate (committed) and its key (never)
│   ├── scripts/                   the Lua scripts, hello.lua for now
│   ├── psql/defaults/schemas/     the tables as first made, one file each
│   ├── psql/migrations/           every change to a table since, numbered
│   ├── logs/                      one log file per UTC day, never committed
│   └── Assets/                    the purchased art, never committed
└── Documentation/                 design notes, the protocol, and the working docs
```

Build output, logs, the purchased art in `Content/Assets/` and the engines' caches stay out of git.  Large
binary assets go through Git LFS.  The repo is private and stays that way: the art in it can't be
redistributed.
