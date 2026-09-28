<!--
File:       Opus/README.md
Component:  Opus
Author:     Jacob Chacko
-->

# Opus

Opus is a multiplayer game I'm building as a hobby.  It's two programs: **Conductor**, the server, and
**Ensemble**, the client players run.  Conductor is authoritative -- it owns the game state, clients ask,
and Conductor decides.

It is early.  Conductor has its first tools (a disk manager every file goes through, a log, a config file
and a PostgreSQL connection), a monitor that watches the process and the machine, and a web page to run it
from, and that's all.  There is no game yet, and
Ensemble hasn't been started.  Things will change and things will break.

## Building and running Conductor

Conductor is Rust, edition 2024.  The one crate so far is `postgres`, for the database.  The web admin and
the monitor use only the standard library and what the OS already has.  From the repo root:

```
cd Conductor/dev
cargo build
cargo test
cargo run -p conductor-launcher
```

That starts Conductor.  The console shows the log as it's written and takes no input.  Everything else is
done from the web admin at <http://127.0.0.1:9996/Opus>, in a browser on the same machine.  It has a tab
each for the machine and every process on it, Conductor's own CPU, memory, disk and threads, its services,
the database and the disk manager, the open notifications, and the log, plus the Shut Down button.  Every
warning and error lands on a bell in the corner and stays there until somebody acknowledges it.  While the database is offline, the page shows that
and nothing else, since the game can't run without it.  It only listens on 127.0.0.1, so it can't be
reached from anywhere else.  The port is `wgui_port` in `conductor_globals.cfg`.

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

- `Content/cfg/conductor_globals.cfg` -- the settings (the log folder, the web admin's port).  Edit it with
  the server stopped.
- `Content/cfg/postgres.cfg` -- where Postgres is and how to log in.
- `Content/psql/` -- the tables as first made, and the numbered changes to them since.
- `Content/logs/` -- one log file per day, named for the UTC date.

## Ensemble

Not started.  The engine isn't picked yet.

## Layout

```
Opus/
├── Conductor/          the server
│   └── dev/            a Cargo workspace
│       ├── conductor-tools/      the disk, the log, the config, the database, notices, the clock
│       ├── conductor-monitor/    watches the process: CPU, memory, disk, threads
│       ├── conductor-wgui/       the web admin
│       └── conductor-launcher/   the program: starts it all
├── Ensemble/           the client (not started)
├── Content/            configs, database schemas, logs
└── Documentation/      design notes and working docs
```

Build output, logs, the purchased art in `Content/Assets/` and the engines' caches stay out of git.  Large
binary assets go through Git LFS.
