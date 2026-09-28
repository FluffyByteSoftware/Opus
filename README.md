<!--
File:       Opus/README.md
Component:  Opus
Author:     Jacob Chacko
-->

# Opus

Opus is a multiplayer game I'm building as a hobby.  It's two programs: **Conductor**, the server, and
**Ensemble**, the client players run.  Conductor is authoritative -- it owns the game state, clients ask,
and Conductor decides.

It is early.  Conductor has its first tools (a log, a config file and a PostgreSQL connection) and a small
admin menu, and that's all.  There is no game yet, and Ensemble hasn't been started.  Things will change
and things will break.

## Building and running Conductor

Conductor is Rust, edition 2024.  The one crate so far is `postgres`, for the database.  From the repo root:

```
cd Conductor/dev
cargo build
cargo test
cargo run -p conductor-launcher
```

That starts the launcher, which is the admin's menu.  L shows the end of the log (`L -n 50` for more,
`L all` for all of it), and Q shuts down.

Conductor needs PostgreSQL running on the same machine, with a database `opusdb` and a role `opus_game`
that can create tables in it.  Conductor makes its own tables the first time it connects.  If Postgres isn't
there, Conductor still runs, it just has no database, and the log says so.

Conductor keeps its data in `Content/` at the repo root.  Anything it needs there and doesn't find, it makes
with defaults.  If Conductor is run from somewhere it can't find `Content/` by walking up the folders, point
it there:

```
OPUS_CONTENT=/path/to/Opus/Content cargo run -p conductor-launcher
```

- `Content/cfg/conductor_globals.cfg` -- the settings.  Edit it with the server stopped.
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
│       ├── conductor-tools/      the log, the config, the database, the clock
│       └── conductor-launcher/   the program and its admin menu
├── Ensemble/           the client (not started)
├── Content/            configs, database schemas, logs
└── Documentation/      design notes and working docs
```

Build output, logs, the purchased art in `Content/Assets/` and the engines' caches stay out of git.  Large
binary assets go through Git LFS.
