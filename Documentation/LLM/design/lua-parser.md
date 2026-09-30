<!--
File:       Opus/Documentation/LLM/design/lua-parser.md
Component:  Documentation
Author:     Jacob Chacko
-->

# lua-parser

A lib crate, and a server piece.  The folder is `Conductor/dev/lua-parser/` and the crate is
`conductor-lua-parser`, so code says `conductor_lua_parser::start()`.  Jacob named it, 2026-09-30.
"Parser" is a loose name: `mlua` does the parsing, and this crate mostly runs scripts.

It's Lua's first step.  On START SERVER it runs every `.lua` file under `Content/scripts/` once, each in a
locked-down Lua of its own, and the only thing a script can call from outside is the log.  It proves Lua
runs inside Conductor and can't hurt it.  Templates and blueprints exist in Rust (`conductor-primlib`);
writing them as scripts is primlib's part 2 (`design/primlib.md`), and behaviour scripts are in
LONGTERM_TODO.md with the language's other open questions, Jacob's sample among them.

## Skeleton

```
lua-parser/
├── Cargo.toml         depends on conductor-tools, conductor-primlib and mlua 0.12 ("lua54", "vendored")
└── src/
    ├── lib.rs         start(), stop()
    │                    the lua thread: make Content/scripts/ if it's missing, find every .lua file
    │                    under it (folders inside included, links to folders not followed), sort them,
    │                    read each through DiskMan and hand it to sandbox::run(); then check in with
    │                    the services list once a second until stop() drops the Sender
    │                  name_of(): the path from Content/ with / on every OS, scripts/npcs/goblin.lua
    ├── sandbox.rs     run(name, source) -> Result<(), String>: a fresh Lua, the script, the Lua dropped
    │                  evaluate(name, source, read): the same, handing what the script returns to read
    │                  TIME_LIMIT 1 s, MEMORY_LIMIT 64 MB, LOG_LINES 50, LINE_CHARS 1000
    └── save.rs        read_save(name, text) -> Result<Save, String>: a saved GameObject back from its Lua
                       text, in the same locked-down Lua (`design/primlib.md`)
```

## Decided

- **Lua 5.4 through `mlua`, built from source** ("vendored"), so nothing is installed but a C compiler (on
  Windows, Visual Studio's; see `Documentation/HowTo/WINDOWS_INSTALL.md`).  Jacob said yes to the
  dependency, 2026-09-30.  It brings its own few crates along (`mlua-sys`; `cc`, `lua-src`, `pkg-config`
  and `cfg-if` to build Lua; `bstr`, `parking_lot`, `rustc-hash`, `num-traits`, `either`, `libc`).  `mlua`
  0.12 needs Rust 1.88; Jacob's is 1.98.
- **One thread, `lua`.**  Running a folder of scripts can take a while (a second each, at worst), and START
  SERVER shouldn't wait on it.  The thread stays up once they've run, checking in, because that's where the
  game's scripts will live once there are objects for them to drive.  STOP SERVER lets the script it's on
  finish and skips the rest.
- **Order in the launcher**: after the account desk and before GameWorld and the GameClock on START SERVER,
  so the scripts have run before the world is made and the door opens (the world will be made of them one
  day); after GameWorld on STOP SERVER, once nobody is left in the world.
- **What a script gets**: Lua's `string`, `table`, `math`, `utf8` and `coroutine`, and the base library
  without `dofile`, `loadfile` (they read files), `load` (it takes raw bytecode, which Lua doesn't check, so
  a bad bit of it can crash the process) and `warn` (it writes to the console past Scribe), and without
  `string.dump`.  Never `io`, `os`, `package` (so no `require`) or `debug`.  `mlua`'s own "safe" default
  still loads `io` and `os`, so the list is picked by hand.
- **The log is all it can call**: `log.debug()`, `log.info()`, `log.warn()`, `log.error()`, and `print()`
  as `log.debug()`.  Each takes any number of values and writes them with a space between, on the `Script`
  channel, starting with the file and line that called it: `scripts/hello.lua, line 3: Hello from Lua.`
- **Everything wrong in a script is a Warn, never an Error** (Jacob: "we would want to pass off any
  errors or warnings from LUA as _warnings_ to scribe").  So `log.error()` writes a Warn, and so does a
  script that won't load, one that fails partway, and one that hits a limit.  An Error is for Conductor
  itself.  The Services tab says "trouble" (red) when any script failed, and the log says which (Jacob:
  "yeah it should... and the log points you to where it's broken.").  One bad script out of many turns the
  row red until it's fixed and the server restarted.
- **A script can't crush what's underneath it** (Jacob, the same answer):
  - A **time limit** of a second.  `mlua`'s hook checks the clock every 10,000 Lua instructions and stops
    the script past it.  It's the *global* hook, on purpose: the plain one only covers the Lua thread it's
    set on, and a runaway loop inside a coroutine would never be stopped.
  - A **memory limit** of 64 MB per script.  Past it, the allocation fails and the script stops.
  - **50 log lines** a run, since a Warn is a notice on the bell and a script logging in a loop would bury
    it.  The 51st is a Warn saying the rest are dropped.  A line over 1,000 characters is cut, and a line
    break in one is made a space.
  - The numbers are a guess (Jacob: "we'll go with your suggestion for now"), fixed in `sandbox.rs`.
- **A fresh Lua per script**, dropped when the script ends.  Nothing is shared between scripts yet, so one
  can't break another.
- **A changed script takes on RESTART SERVER** (or STOP and START).  No hot loading.

## Open

- **The gap in the time limit.**  A script that wraps its runaway loop in `pcall` catches the stop, the same
  as any other error.  It only buys itself a moment: the stop keeps firing every 10,000 instructions, and
  sooner or later one lands outside the `pcall`.  Taking `pcall` away would take away every script's way of
  handling its own errors, which seems worse.
- **Settings for the limits**, if they need to change without a build.
- **Windows**: built there on 2026-09-30 (MSVC compiles Lua), but a script's run hasn't been checked there.
- Everything in LONGTERM_TODO.md's scripting entry and primlib's part 2: templates and blueprints as
  scripts, behaviour scripts attached to objects the way Unity does it, how the folder is laid out for them.
