<!--
File:       Opus/Documentation/LLM/design/conductor-launcher.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-launcher

A bin crate, and the program.  main() brings up the program (DiskMan, Scribe, Constellations, the web
admin), then sits on the Control Panel's mailbox starting and stopping the server as asked, and shuts down
when the web admin stops.

## Skeleton

```
conductor-launcher/
├── Cargo.toml         depends on conductor-tools, conductor-monitor, conductor-wgui
└── src/
    └── main.rs        threads::name_this_thread("main")
                       -> diskman::start -> scribe::start -> constellations::load(&GLOBALS) -> scribe::move_to
                       -> server::set(Stopped) -> wgui::start -> take_commands()
                            loop: wgui::has_ended()?  server::next_command() -> start_server() / stop_server()
                       -> stop_server() if running -> diskman::stop + wait_on_diskman()
                       start_server(): fingerprinter::start -> security::start -> archivist::start -> monitor::start
                       stop_server():  monitor::stop -> security::stop -> archivist::stop -> fingerprinter::stop
                                       -> constellations::server_stopped()
```

## What we decided

- The launcher is the program.  Everything else is a lib crate it starts.
- **The server starts from the Control Panel, not at boot** (2026-09-29).  Jacob's ask: the page Conductor
  greets you with is a control panel, and nothing but the log is up until START SERVER.  So main boots only
  what the page needs (DiskMan, Scribe, Constellations, the web admin) and `start_server()` /
  `stop_server()` are the list of what the server is: Fingerprinter, Security, Archivist, the monitor
  today, the network and the game later.  A new piece goes in both.  Security's 64 MiB arena comes and
  goes with the server, so a stopped Conductor holds none of it.
- **main does the starting and stopping, not the web admin's thread.**  The routes only drop a command in
  `server.rs`'s mailbox and answer; main picks it up within `COMMAND_WAIT` (250 ms).  Archivist's stop can
  wait on a long query, and the page keeps asking for its status the whole time.
- `has_ended()` replaced `wait()`: main checks between commands whether the web admin's thread is still
  there, so a SHUT DOWN (or the thread dying) still ends Conductor the way it did.
- **The console is only Scribe's output.**  Typing in it does nothing.  The admin works through the web admin
  (see `conductor-wgui.md`), and Shut Down there is how Conductor stops.  When the web admin's thread ends,
  main carries on into the shutdown and the program ends, and the console with it.
- If the web admin can't start, Conductor doesn't run: the log says why and it shuts down.
- **DiskMan is first in and last out** (2026-09-28).  Every file goes through it, Scribe's log included, so it
  starts before Scribe; and anything else may hand it files on the way out, so it stops after Archivist.
- **Shutdown waits on DiskMan.**  If it takes more than a second, the console counts down from 60 every 5
  seconds.  At zero: `SHOULD BE CLOSED, IF STILL RUNNING PLEASE FORCE QUIT`, with the files that would be
  lost, again every 30 seconds.  It never quits on its own; force quitting is the admin's call.
- The last line, "Conductor has shut down.", comes after DiskMan has finished, so it only reaches the console.
- **`stop_server()` ends with `constellations::server_stopped()`** (2026-09-29): with every server piece
  down, DiskMan swaps any `postgres.cfg.wait4server` saved from the web admin over the live file, and the
  launcher waits on it, so a RESTART SERVER reads the new file.  The hard files' swaps happen inside
  DiskMan's own stop at shutdown; the launcher does nothing for them.
- Every server start and stop is four Info lines (starting, running, stopping, stopped).  They'll want the
  Debug switch like everything else.
- Ctrl-C still kills it outright, and now that can lose what DiskMan is holding.  TODO.md has catching it.

## History

Until 2026-09-28 the launcher ran a text menu: L to view the end of the log (`L -n 50`, `L all`), Q to shut
down.  It went when the web admin came in, along with its 7 tests.  The page's Scribe terminal replaces L.
Until 2026-09-29 main started Archivist and the monitor itself at boot and waited on the web admin's thread;
the Control Panel took that over.
