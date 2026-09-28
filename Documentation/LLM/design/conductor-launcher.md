<!--
File:       Opus/Documentation/LLM/design/conductor-launcher.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-launcher

A bin crate, and the program.  main() brings the tools up in order, hands the terminal to the admin's menu,
and shuts down when the menu returns.  Modeled on Stratum's launcher, written fresh.

## Skeleton

```
conductor-launcher/
├── Cargo.toml         depends on conductor-tools
└── src/
    ├── main.rs        scribe::start -> constellations::load -> scribe::move_to -> launcher::run -> shut down
    └── launcher.rs    run()                      the menu loop
                       parse_command(line)        the letter and its arguments
                       view_log_count(args)       25 lines, -n N, or all
                       last_lines(path, n, chunk) reads back from the end of the file
                       all_lines(path)            the whole file
```

## What we decided

- The launcher is the program.  Networking and the game will be lib crates it starts, the way Stratum's
  were.
- One letter per choice, either case.  `list` or `quit` isn't a choice, so a typo can't shut the server
  down.  An Escape pressed by accident is ignored.
- Q, or Ctrl-D closing the terminal, shuts down.
- The menu today:

```
Opus Conductor

  L) View the log      L, L -n 50, or L all
  Q) Shut down
```

- L shows the last 25 lines of the file Scribe is writing today, `L -n N` the last N, and `L all` the whole
  file.  A bad argument gets one line saying what L takes.
- The last N lines are read from the end of the file in 16 KB chunks, so a day-long log isn't loaded whole
  to show 25 lines.  Bytes that aren't valid text (a crash mid-line) show as a placeholder rather than
  failing.

## What's open

- S) Start / stop the server, account management and config management come as their pieces exist.
- Ctrl-C isn't switched off.  Nothing needs saving on the way out yet.
- L only shows today's file.
