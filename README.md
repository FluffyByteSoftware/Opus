<!--
File:       Opus/README.md
Component:  Opus
Author:     Jacob Chacko
-->

# Opus

Opus is a multiplayer game I'm building as a hobby.  It's two programs: **Conductor**, the server, and
**Ensemble**, the client players run.  Conductor is authoritative -- it owns the game state, clients ask,
and Conductor decides.

It is early.  Conductor has its first tools (a log and a config file) and a small admin menu, and that's
all.  There is no game yet, and Ensemble hasn't been started.  Things will change and things will break.

## Building and running Conductor

Conductor is Rust, edition 2024, with no crates beyond the standard library so far.  From the repo root:

```
cd Conductor/dev
cargo build
cargo test
cargo run -p conductor-launcher
```

That starts the launcher, which is the admin's menu.  L shows the end of the log (`L -n 50` for more,
`L all` for all of it), and Q shuts down.

Conductor keeps its runtime data in `Content/` at the repo root.  A fresh checkout doesn't have one, and
that's fine -- the first run makes it, along with a config file full of defaults.  If Conductor is run from
somewhere it can't find `Content/` by walking up the folders, point it there:

```
OPUS_CONTENT=/path/to/Opus/Content cargo run -p conductor-launcher
```

- `Content/cfg/conductor_globals.cfg` -- the settings.  Edit it with the server stopped.
- `Content/logs/` -- one log file per day, named for the UTC date.

## Ensemble

Not started.  The engine isn't picked yet.

## Layout

```
Opus/
├── Conductor/          the server
│   └── dev/            a Cargo workspace
│       ├── conductor-tools/      the log, the config, the clock
│       └── conductor-launcher/   the program and its admin menu
├── Ensemble/           the client (not started)
├── Content/            runtime data -- never committed, made on first run
└── Documentation/      design notes and working docs
```

Build output, `Content/` and the engines' caches stay out of git.  Large binary assets go through Git LFS.
