<!--
File:       Opus/Documentation/LLM/design/conductor-tools.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-tools

A lib crate.  The pieces the rest of Conductor leans on but that know nothing about the game: the log, the
config, and the clock.  No dependencies.

## Skeleton

```
conductor-tools/
├── Cargo.toml
└── src/
    ├── lib.rs             pub mod clock; pub mod constellations; pub mod scribe;
    ├── clock.rs           Utc { year, month, day, hour, minute, second }
    │                        Utc::now(), Utc::from_unix(seconds)
    │                        date(), file_stamp() -> "2026_09_28", line_stamp() -> "02:16:43 PM - 09-28-26 Z"
    ├── scribe.rs          enum Channel { System, Network, Security, Database, Game }
    │                      enum Priority { Debug, Info, Warn, Error }
    │                        start(dir), move_to(dir), current_file()
    │                        debug / info / warn / error (channel, message)
    │                        debug_with / info_with / warn_with / error_with (channel, err, message)
    └── constellations.rs  struct Settings { scribe_log_dir }
                             load(), settings(), log_dir(), content_dir(), config_path()
```

## Scribe

What we decided:

- One file per UTC day, `Content/logs/2026_09_28.scribe.log`, appended to, rolling at midnight UTC.  Year
  first so the folder sorts by date.
- The line is Jacob's layout:
  `[ 02:16:43 PM - 09-28-26 Z ] - [ System / Info ] - [ message ] [ Caller: file, Line: n ]`
- The caller comes from `#[track_caller]`, so nobody passes a file and line by hand.
- An error value goes in front of the message (`error_with()`), the way `ex.Message` did in C#.  Rust has
  no exceptions, so it takes anything that prints.
- **The file is the only place a line goes.**  The terminal belongs to the launcher's menu, and L is how the
  log gets read.  The exception is a line with no file to go to (before `start()`, or a file that can't be
  opened or written).  That one prints to the terminal, because a line nobody can see anywhere is worse than
  one in the middle of the menu.
- A lost file gets one note on stderr and is tried again at the next midnight or `move_to()`.
- Scribe never panics and never hands an error back from a log call.  A log that takes the server down is
  worse than no log.
- Scribe starts before Constellations, on the default folder, so the config's complaints have somewhere to
  go.  `move_to()` follows the config once it's loaded.

What's open:

- The few lines before `move_to()` stay in the default folder if the config points elsewhere.
- No size limit, no minimum priority.  Both are in TODO as ideas.

## Constellations

What we decided:

- `Content/cfg/conductor_globals.cfg`, plain `key = value` lines, `#` for comments, keys in any case.
  TOML would have cost a crate to read a handful of lines.
- `Content/` is found through `OPUS_CONTENT`, then by walking up from the working directory, then
  `./Content`.  No drive path is ever hardcoded, so a fresh checkout anywhere runs.
- A checked struct: every key has a type and a check.  A bad line is a Warn with its line number, and that
  setting keeps its default.  An unknown key is a complaint too, so a typo doesn't go silent.  If a key
  shows up twice the later one wins, with a complaint.
- **Nothing in here stops the server.**  A missing file gets written with the defaults.  A file that can't
  be read is an Error and we run on the defaults, without writing over it.
- Loaded once, at startup.  A relative path in the file is taken from `Content/`.
- A new setting touches four places, all in `constellations.rs`: the struct, `default_settings()`,
  `apply_setting()` and `file_text()`.

Settings today:

| Key              | Default | What it is                                   |
|------------------|---------|----------------------------------------------|
| `scribe_log_dir` | `logs`  | The folder Scribe writes into, under Content |

What's open:

- No reload.  The launcher's config menu will need one, which means Constellations stops being load-once.
- The file is written straight to disk.  It goes through the disk manager once that exists.

## The clock

The standard library stops at seconds since 1970, so the calendar is worked out by hand in `clock.rs`.  The
tests pin it against dates we know: the epoch, a leap day in 2000, and the last second before midnight on
2026-09-28 rolling into the 29th.
