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

All of it builds with no warnings, and all 19 tests pass on Jacob's machine (12 in the tools, 7 in the
launcher).  `cargo run -p conductor-launcher` showed the menu, kept the log off the terminal, and L read the
day's log back with the new `Caller:` paths.

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
  view-log command.  Built, tested and run on Jacob's machine.

What fought back:

- Jacob edited `.gitignore` on GitHub while the branch was open (`Content/Assets/` there, `Content/` on the
  branch), and the pull request had a conflict.  It was merged with `Content/` kept, so the whole folder is
  ignored.
- Commits from the session got blocked by a permission prompt at first, so the work sat uncommitted until
  Jacob asked how to pull it.  Pulling a session branch took a minute to figure out too.  The commands are
  in "What's waiting".

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

- Merging this session's branch, `claude/tone-communication-style-z5v1au`, into `main` (open a pull request
  on GitHub and merge it, then `git checkout main` and `git pull origin main`).  To try a session branch
  before merging: `git fetch origin <branch>` then `git checkout <branch>`.
- Not tried yet: the bad-value, unknown-key and lost-log-file checks from the hand-back message.
- The rest of Conductor's tools: the disk manager and Security.
- PostgreSQL, once a crate is OK'd.
- Picking Ensemble's engine.
