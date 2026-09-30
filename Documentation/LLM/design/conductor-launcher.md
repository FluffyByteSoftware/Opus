<!--
File:       Opus/Documentation/LLM/design/conductor-launcher.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-launcher

A bin crate, and the program.  main() brings up the program (DiskMan, Scribe, Constellations, the web
admin), then sits on the Control Panel's mailbox starting and stopping the server as asked, opens the door
once the world is ready, and shuts down when the web admin stops.

## Skeleton

```
launcher/
├── Cargo.toml         depends on conductor-tools, conductor-accounts, conductor-monitor, conductor-networking,
│                        conductor-lua-parser, conductor-gameworld, conductor-gameclock, conductor-wgui
└── src/
    └── main.rs        threads::name_this_thread("main")
                       -> diskman::start -> scribe::start -> constellations::load(&GLOBALS) -> scribe::move_to
                       -> constellations::load(&WGUI) -> server::set(Stopped) -> wgui::start(wgui_port)
                       -> take_commands()
                            loop: wgui::has_ended()?  server::next_command(COMMAND_WAIT)
                                    -> start_server() / stop_server(), or Restart: stop_server() then start_server()
                                  then, if Running with the door shut and gameclock::ready(): networking::start()
                       -> stop_server() unless stopped -> wait_on_diskman() (which calls diskman::stop first)
                       start_server(): fingerprinter -> security -> archivist -> account desk -> lua -> gameworld
                                       -> gameclock -> networking::wait_for_world() -> monitor
                       stop_server():  monitor -> networking -> gameclock -> gameworld -> lua -> account desk
                                       -> security -> archivist -> fingerprinter
                                       -> constellations::server_stopped()
```

## What we decided

- The launcher is the program.  Everything else is a lib crate it starts.
- **The server starts from the Control Panel, not at boot** (2026-09-29).  Jacob's ask: the page Conductor
  greets you with is a control panel, and nothing but the log is up until START SERVER.  So main boots only
  what the page needs (DiskMan, Scribe, Constellations, the web admin), and `start_server()` /
  `stop_server()` are the list of what the server is.  A new piece goes in both.  Security's 64 MiB arena
  comes and goes with the server, so a stopped Conductor holds none of it.
- **main does the starting and stopping, not the web admin's thread.**  The routes only drop a command in
  `server.rs`'s mailbox and answer; main picks it up within `COMMAND_WAIT` (250 ms).  Archivist's stop can
  wait on a long query, and the page keeps asking for its status the whole time.  Between commands main
  checks `has_ended()`, so a SHUT DOWN (or the web admin's thread dying) ends Conductor.
- **The door waits on the world** (2026-09-30): nobody gets in before there's a voxel to step on.
  `start_server()` doesn't start networking, only `networking::wait_for_world()`, which puts both Network
  lines on the Services tab at "waiting on the world".  The command loop, awake every 250 ms anyway, starts
  networking (TCP and UDP both) the first time it sees the server running and `conductor_gameclock::ready()`
  true; every START, STOP or RESTART counts the door as shut again.  Until then the server reads running,
  so STOP SERVER works, and `networking::stop()` says on the Services tab that the door never opened.  A
  run where a chunk around 0,0,0 can't be had never opens it.
- **The console is only Scribe's output.**  Typing in it does nothing.  The admin works through the web
  admin (see `conductor-wgui.md`), and SHUT DOWN there is how Conductor stops.  If the web admin can't
  start, Conductor doesn't run: the log says why and it shuts down.
- **The order inside the server** has a reason at each step.  Fingerprinter before Security, since Security
  takes its salts from it.  The account desk comes after Security and Archivist and goes before them, since
  its jobs are a hash and then a write: its `stop()` finishes the jobs already handed in while both are
  still up.  GameWorld before the GameClock, so the GameClock has somebody to ask for chunks, and after it
  on the way down, once nobody is left to ask.  The GameClock after Lua, so there's a world before there
  are players.  Networking opens the door only once the three a login leans on (Fingerprinter, Security,
  Archivist) are up and the world is ready; on the way down it shuts the door and tells every player
  before those three go.  Security stops before Archivist, so a hash on its way to the accounts table still
  lands.
- **DiskMan is first in and last out.**  Every file goes through it, Scribe's log included, so it starts
  before Scribe; and anything else may hand it files on the way out, so it stops after Archivist.
- **Shutdown waits on DiskMan.**  If it takes more than a second, the console counts down from 60 every 5
  seconds.  At zero: `SHOULD BE CLOSED, IF STILL RUNNING PLEASE FORCE QUIT`, with the files that would be
  lost, again every 30 seconds.  It never quits on its own; force quitting is the admin's call.  The last
  line, "Conductor has shut down.", comes after DiskMan has finished, so it only reaches the console.
- **`stop_server()` ends with `constellations::server_stopped()`**: with every server piece down, DiskMan
  swaps every soft file's `.wait4server` saved from the web admin (`postgres.cfg`, `networking.cfg` and
  `game.cfg` today) over the live file, and the launcher waits on it, so a RESTART SERVER reads the new
  files.  A swap that fails is a Warn and the old file stands.  The hard files' swaps happen inside
  DiskMan's own stop at shutdown; the launcher does nothing for them.
- The boot says, in Info lines, where `Content/` is, where the settings came from, and which files need
  which reboot, then "Conductor is up.  The server waits on START SERVER from the Control Panel."
- Loose ends, each in TODO.md: the web admin's port is still `wgui_port` in `conductor_globals.cfg`, to move
  into `wgui.cfg`; every server start and stop is four Info lines (starting, running, stopping, stopped),
  which want the Debug switch; and Ctrl-C still kills Conductor outright, which can lose what DiskMan holds.
