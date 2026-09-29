<!--
File:       Opus/README.md
Component:  Opus
Author:     Jacob Chacko
-->

# Opus

Opus is a multiplayer game I'm building as a hobby.  It's two programs: **Conductor**, the server, and
**Ensemble**, the client players run.  Conductor is authoritative -- it owns the game state, clients ask,
and Conductor decides.

It is early.  Conductor has its first tools (a disk manager every file goes through, a log, config files,
a UUID maker, a password hasher and a PostgreSQL connection), a monitor that watches the process and the
machine, a web page to run it from, and a front door: a login over TLS that hands a player a ticket for
UDP, where the game will go.  There is no game yet, no way to make an account yet, and Ensemble hasn't
been started.  Things will change and things will break.

## Building and running Conductor

Conductor is Rust, edition 2024.  Three crates so far: `postgres`, for the database, `argon2`, for
password hashing, and `rustls`, for TLS.  The web admin and the monitor use only the standard library and
what the OS already has.  From the repo root:

```
cd Conductor/dev
cargo build
cargo test
cargo run -p conductor-launcher
```

That starts Conductor.  The console shows the log as it's written and takes no input.  Everything else is
done from the web admin at <http://127.0.0.1:9996/Opus>, in a browser on the same machine.  It asks for
a login first: `admin` (password `admin`) can do everything, `user` (password `user`) can look and change
nothing, and both passwords are in `Content/cfg/wgui.cfg`.  It opens on the Control Panel, where START
SERVER brings the server up (the database connection, the network and the monitor, and the game once
there is one), STOP SERVER takes it back down, and SHUT DOWN closes Conductor.  Until the server is running, the only
other tabs that work are the log and the settings.  Once it is, there's a tab each for the machine and
every process on it, Conductor's own CPU, memory, disk and threads, its services, the database and the
disk manager, the open notifications, the log, and the settings, where every config file can be changed
from the page and the change takes at the next restart of whatever reads it.  Every warning and error
lands on a bell in the corner and stays there until somebody acknowledges it.  While the database is
offline, the page shows that and nothing else, since the game can't run without it.  It only listens on
127.0.0.1, so it can't be reached from anywhere else.  The port is `wgui_port` in
`conductor_globals.cfg`.

Conductor runs on Linux (Nobara and Fedora are what it's built on) and Windows.  macOS builds and runs, but
the web admin can't measure CPU, memory or disk there yet.

Conductor needs PostgreSQL running on the same machine, with a database `opusdb` and a role `opus_game`
that can create tables in it.  Conductor makes its own tables the first time it connects.  If Postgres isn't
there, Conductor still runs, it just has no database, and the log says so.

Conductor keeps its data in `Content/` at the repo root.  Anything it needs there and doesn't find, it makes
with defaults.  If Conductor is run from somewhere it can't find `Content/` by walking up the folders, point
it there:

```
OPUS_CONTENT=/path/to/Opus/Content cargo run -p conductor-launcher
```

- `Content/cfg/conductor_globals.cfg` -- the program's settings (the log folder, the web admin's port).  Read
  once at boot, so a change means running Conductor again.
- `Content/cfg/wgui.cfg` -- the web admin's two accounts, `user` and `admin`, and their passwords.  Read
  once at boot too.
- `Content/cfg/postgres.cfg` -- where Postgres is and how to log in.  Read every time the server starts, so a
  change means STOP SERVER and START SERVER on the web admin's Control Panel.
- `Content/cfg/networking.cfg` -- where Conductor listens for players (TCP 9997 for logins, UDP 9998 for
  the game), the TLS files, and the deadlines.  Read every time the server starts too.
- `Content/certs/` -- the TLS certificate and its key.  Conductor doesn't make them; make them once, from
  the repo root, and the certificate is what a client trusts:

  ```
  mkdir -p Content/certs && openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
    -keyout Content/certs/conductor.key -out Content/certs/conductor.crt -days 3650 \
    -subj "/CN=Opus Conductor" -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
  ```

  The key stays out of git; the certificate goes in, since clients need a copy.
- `Content/psql/` -- the tables as first made, and the numbered changes to them since.
- `Content/logs/` -- one log file per day, named for the UTC date.

## Talking to it

The protocol is in `Documentation/LLM/PROTOCOL.md`: a client logs in over TLS on the TCP port and is handed a
token and the UDP port, and everything after that goes over UDP.  Until Ensemble exists,
`Conductor/dev/conductor-networking/test_client.py` stands in for it (Python 3, standard library only):

```
python3 Conductor/dev/conductor-networking/test_client.py some_account 'Its password 1!'
```

## Ensemble

Not started.  The engine isn't picked yet.

## Layout

```
Opus/
├── Conductor/          the server
│   └── dev/            a Cargo workspace
│       ├── conductor-tools/      the disk, the log, the config, UUIDs, password hashing, the database, notices
│       ├── conductor-monitor/    watches the process: CPU, memory, disk, threads
│       ├── conductor-networking/ the login over TLS, the game over UDP
│       ├── conductor-wgui/       the web admin
│       └── conductor-launcher/   the program: starts it all
├── Ensemble/           the client (not started)
├── Content/            configs, the TLS certificate, database schemas, logs
└── Documentation/      design notes and working docs
```

Build output, logs, the purchased art in `Content/Assets/` and the engines' caches stay out of git.  Large
binary assets go through Git LFS.
