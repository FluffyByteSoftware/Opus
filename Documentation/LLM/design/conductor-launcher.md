<!--
File:       Opus/Documentation/LLM/design/conductor-launcher.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-launcher

A bin crate, and the program.  main() brings everything up in order, then waits on the web admin, and shuts
down when the web admin stops.

## Skeleton

```
conductor-launcher/
├── Cargo.toml         depends on conductor-tools, conductor-monitor, conductor-wgui
└── src/
    └── main.rs        threads::name_this_thread("main")
                       -> scribe::start -> constellations::load -> scribe::move_to -> archivist::start
                       -> monitor::start -> wgui::start + wgui::wait
                       -> monitor::stop -> archivist::stop
```

## What we decided

- The launcher is the program.  Everything else is a lib crate it starts.
- **The console is only Scribe's output.**  Typing in it does nothing.  The admin works through the web admin
  (see `conductor-wgui.md`), and Shut Down there is how Conductor stops.  When the web admin's thread ends,
  main carries on into the shutdown and the program ends, and the console with it.
- If the web admin can't start, Conductor doesn't run: the log says why and it shuts down.
- Ctrl-C still kills it outright.  Nothing needs saving on the way out yet.

## History

Until 2026-09-28 the launcher ran a text menu: L to view the end of the log (`L -n 50`, `L all`), Q to shut
down.  It went when the web admin came in, along with its 7 tests.  The page's Scribe terminal replaces L.
