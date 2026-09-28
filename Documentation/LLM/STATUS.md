<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor has its first tools and a program to run them.  `Conductor/dev/` is a Cargo workspace with two
crates: `conductor-tools` (lib: Scribe, Constellations, the clock) and `conductor-launcher` (bin: starts the
tools and runs the admin's menu).  The menu has L to view the log and Q to shut down, and that's all.
Ensemble hasn't been started.

**The workspace split hasn't been built yet.**  Everything before it built and passed its 7 tests on Jacob's
machine.  The split, the Scribe and Constellations changes, and the launcher were written after that, and
the first `cargo build` / `cargo test` on them is still to come.

## Last session -- 2026-09-28

The first real session.  We set up the docs and wrote Conductor's first tools.

What we did:

- Put the file headers on the docs and the git dotfiles.
- Wrote Scribe, the log.  Every line looks like
  `[ 02:16:43 PM - 09-28-26 Z ] - [ System / Info ] - [ message ] [ Caller: file, Line: n ]`, goes to a file
  named for the UTC date in `Content/logs/`, and rolls to a new file at midnight UTC.  An error value can be
  passed along with the message (`error_with()` and friends) and it prints in front, the way `ex.Message`
  did in the C# version.
- Wrote Constellations, the config.  It reads `Content/cfg/conductor_globals.cfg`, writes one with the
  defaults if it's missing, and holds the settings in a checked struct.  A bad line is a Warn in the log
  and that setting keeps its default.  One key so far: `scribe_log_dir`.
- Wrote the date math by hand (`clock.rs`), so there are no crates yet.
- Built and tested that first version on Jacob's machine: 7 tests passed, and `cargo run` found the real
  `Content/` folder by walking up from `Conductor/dev`.
- Then split it into the workspace, made Scribe write to the file only, and added the launcher with its
  view-log command.  This part is unbuilt (see above).

What fought back:

- Jacob edited `.gitignore` on GitHub while the branch was open (`Content/Assets/` there, `Content/` on the
  branch), and the pull request had a conflict.  It was merged with `Content/` kept, so the whole folder is
  ignored.
- Commits from the session got blocked by a permission prompt at first, so the work sat uncommitted until
  Jacob asked how to pull it.

What Jacob decided:

- Runtime data for both components lives in one `Opus/Content/` at the root, not one per component.
- Conductor finds `Content/` through `OPUS_CONTENT`, then by walking up from the working directory, then
  falls back to `./Content`.
- All log time is UTC and ends in `Z`.  The twelve-hour clock and the bracket layout are Jacob's.
- Scribe writes to the file only.  The terminal belongs to the launcher's menu, and L is how the log gets
  read: the last 25 lines, `L -n N`, or `L all`.  The one exception is a lost log file, and then lines print
  to the terminal.
- Scribe starts first on the default folder, and moves to the configured one after Constellations loads.
- The launcher is the program, the way Stratum's was.  Networking and the game become lib crates later.
- Stratum's code is reference only.  Jacob uploads the Stratum file for a piece when it's time to build it.
- The code gets written in the session.  Jacob builds, runs and tests it himself and pastes back the result.
- The repo is private, so assets don't need special exclusion for now.

## What's waiting

- The first build and test of the workspace split and the launcher.
- The rest of Conductor's tools: the disk manager and Security.
- PostgreSQL, once a crate is OK'd.
- Picking Ensemble's engine.
