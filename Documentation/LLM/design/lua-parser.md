<!--
File:       Opus/Documentation/LLM/design/lua-parser.md
Component:  Documentation
Author:     Jacob Chacko
-->

# lua-parser

A lib crate, and a server piece.  The folder is `Conductor/dev/lua-parser/` (the first without the
`conductor-` in front; the others lose theirs when that rename comes), and the crate is
`conductor-lua-parser`, so code says `conductor_lua_parser::start()`.  Jacob named it, 2026-09-30.
"Parser" is a loose name: `mlua` does the parsing, and this crate mostly runs scripts.

It's Lua's first step.  On START SERVER it runs every `.lua` file under `Content/scripts/` once, each in a
locked-down Lua of its own, and the only thing a script can call from outside is the log.  It proves Lua
runs inside Conductor and can't hurt it.  Templates, blueprints and behaviour scripts wait for the ECS
(LONGTERM_TODO.md has the language's open questions, Jacob's sample among them).

## Skeleton

```
lua-parser/
├── Cargo.toml         depends on conductor-tools and mlua 0.12 ("lua54", "vendored")
└── src/
    ├── lib.rs         start(), stop()
    │                    the lua thread: make Content/scripts/ if it's missing, find every .lua file
    │                    under it (folders inside included, links to folders not followed), sort them,
    │                    read each through DiskMan and hand it to sandbox::run(); then check in with
    │                    the services list once a second until stop() drops the Sender
    │                  name_of(): the path from Content/ with / on every OS, scripts/npcs/goblin.lua
    └── sandbox.rs     run(name, source) -> Result<(), String>: a fresh Lua, the script, the Lua dropped
                       TIME_LIMIT 1 s, MEMORY_LIMIT 64 MB, LOG_LINES 50, LINE_CHARS 1000
```

## Decided

- **Lua 5.4 through `mlua`, built from source** ("vendored"), so nothing is installed but a C compiler.
  Jacob said yes to the dependency, 2026-09-30.  It brings its own few crates along (`mlua-sys`; `cc`,
  `lua-src`, `pkg-config` and `cfg-if` to build Lua; `bstr`, `parking_lot`, `rustc-hash`, `num-traits`,
  `either`, `libc`).  `mlua`
  0.12 needs Rust 1.88; Jacob's is 1.98.
- **One thread, `lua`.**  Running a folder of scripts can take a while (a second each, at worst), and
  START SERVER shouldn't wait on it.  The thread stays up once they've run, checking in, because that's
  where the game's scripts will live once there are objects for them to drive.  STOP SERVER lets the
  script it's on finish and skips the rest.
- **Order in the launcher**: after the account desk and before networking on START SERVER, so the scripts
  have run before the door opens (the world will be made of them one day); after networking on STOP
  SERVER, once nobody is left in the world.
- **What a script gets**: Lua's `string`, `table`, `math`, `utf8` and `coroutine`, and the base library
  without `dofile`, `loadfile` (they read files), `load` (it takes raw bytecode, which Lua doesn't check, so
  a bad bit of it can crash the process) and `warn` (it writes to the console past Scribe), and without
  `string.dump`.  Never `io`, `os`, `package` (so no `require`) or `debug`.  Note that `mlua`'s own "safe"
  default still loads `io` and `os`; the list is picked by hand for that reason.
- **The log is all it can call**: `log.debug()`, `log.info()`, `log.warn()`, `log.error()`, and `print()`
  as `log.debug()`.  Each takes any number of values and writes them with a space between, on the
  `Script` channel, starting with the file and line that called it:
  `scripts/hello.lua, line 3: Hello from Lua.`
- **Everything wrong in a script is a Warn, never an Error** (Jacob: "we would want to pass off any
  errors or warnings from LUA as _warnings_ to scribe").  So `log.error()` writes a Warn, and so does a
  script that won't load, one that fails partway, and one that hits a limit.  An Error is for Conductor
  itself.  The Services tab says "trouble" (red) when any script failed, and the log says which.  Jacob,
  2026-09-30, when he saw it red: "yeah it should... and the log points you to where it's broken."  One
  bad script out of many turns the row red until it's fixed and the server restarted.
- **A script can't crush what's underneath it** (Jacob, the same answer):
  - A **time limit** of a second.  `mlua`'s hook checks the clock every 10,000 Lua instructions and stops
    the script past it.  It's the *global* hook, on purpose: the plain one only covers the Lua thread
    it's set on, and a runaway loop inside a coroutine would never be stopped.
  - A **memory limit** of 64 MB per script.  Past it, the allocation fails and the script stops.
  - **50 log lines** a run, since a Warn is a notice on the bell and a script logging in a loop would
    bury it.  The 51st is a Warn saying the rest are dropped.  A line over 1,000 characters is cut, and
    a line break in one is made a space.
  - The numbers are a guess (Jacob: "we'll go with your suggestion for now"), fixed in `sandbox.rs`.
    They could become settings later.
- **A fresh Lua per script**, dropped when the script ends.  Nothing is shared between scripts yet, so
  one can't break another.
- **A changed script takes on RESTART SERVER** (or STOP and START).  No hot loading.

## Open

- **The gap in the time limit.**  A script that wraps its runaway loop in `pcall` catches the stop, the
  same as any other error.  It only buys itself a moment: the stop keeps firing every 10,000
  instructions, and sooner or later one lands outside the `pcall`.  Taking `pcall` away would take away
  every script's way of handling its own errors, which seems worse.
- **Settings for the limits**, if they need to change without a build.
- Everything in LONGTERM_TODO.md's scripting entry: templates, blueprints, behaviour scripts attached to
  objects the way Unity does it, how the folder is laid out for them.

## Tested

Built and tested on Linux (Nobara 44, Rust 1.98.1) on 2026-09-30, the day it was written.  The first
build compiled Lua with `gcc` without trouble.  The one slip was mine, in `conductor-tools`: the Lua row
went into `EXPECTED` in `services.rs` but not into the test that lists every expected service, so
`cargo test` failed there until it was added.

- All 8 of the crate's tests pass; the two runaway ones take about a second each.
- START SERVER: `hello.lua` says hello on the Script channel, and the Services tab has Lua running,
  checking in once a second.
- Four throwaway scripts and a RESTART SERVER, all as they should be: a syntax error named its file and
  line 2; a loop logging a thousand lines stopped at 50 with a Warn; a `while true do end` was stopped at
  its second; `io`, `os`, `require`, `dofile` and `load` all read `nil`.  Lua went red on the Services
  tab (two scripts failed), and Conductor carried on.  The flood's first run printed `line1` fifty times;
  that was a typo in the script (`.. 1` for `.. i`), not the log.
- Taking the four scripts back out and restarting: hello alone.  And a build with no warnings from
  `lua-parser`.  Every Lua check passed; none are left on TEST_CHECKLIST.html.
- **Never built on Windows.**  `mlua` builds Lua with the C compiler, which on Windows is Visual
  Studio's.
