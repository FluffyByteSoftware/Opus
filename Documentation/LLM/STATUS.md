<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is five crates.  `conductor-tools` (lib) holds DiskMan, Scribe, Constellations, Fingerprinter,
Security, Archivist, the notices, the clock, the thread list, the services list and the server's switch
(`server.rs`).  `conductor-monitor` (lib) looks at the process and every process on the machine once a
second.  `conductor-networking` (lib) is the front door: a login over TLS on TCP that hands a player a
ticket for UDP, the UDP side the game will run on, a ledger of every connection at the door, and a
whitelist and a blacklist of addresses checked at the door.  `conductor-wgui` (lib) is the web admin at
`http://127.0.0.1:9996/Opus`, and the only way to start and stop the server and to shut Conductor down.
`conductor-launcher` (bin) boots the program and waits on the Control Panel.  Ensemble is Unity 6000.6,
on Jacob's machine, not in the repo.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The web admin has a login: `admin` / `admin` does everything,
`user` / `user` looks, both passwords in `wgui.cfg`.  The server (Fingerprinter, Security, Archivist,
networking, the monitor, and whatever comes later) only starts when START SERVER is pressed on the web
admin's Control Panel, and STOP SERVER takes it back down with Conductor still running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests, `main` is the
stable release, moved only when Jacob says.  `main` sits on the access lists hand-off; `unstable` and
`testing` are ahead of it by this session's documentation commits, one fix to `page.html`, and DiskMan
noticing hand edits (not built yet).

**Built and tested on Linux (Nobara 44), 2026-09-29**: everything through the access lists.  `cargo
build` clean with no warnings, `cargo test` 194 passed (15 monitor, 56 networking, 89 tools with the
benchmark ignored, 34 web admin).  The Python client did the whole login loop against a debug build, and
the TCP tab was seen working.  The access lists' tabs haven't been looked at in a real run yet.  The
Windows code has never been built.

## Last session -- 2026-09-29, the documentation pass

Jacob's pick: prune and review `Documentation/LLM/`, update it to what's been built, and overhaul
README.md.  Then, at Jacob's word, one fix to `page.html` and DiskMan noticing hand edits.

What we did:

- **Every design doc and PROTOCOL.md read against the code.**  The launcher's doc had networking missing
  from the start and stop order; the monitor's didn't say it's a server piece or that uptime counts from
  START SERVER; the web admin's had the Settings and TCP tabs in their old places, "SHUT DOWN is the only
  thing that works" under the database lock, and two cross-site checks where there are three; the tools
  doc had Security's arena kept for Conductor's whole run (it goes with the server) and configs "read
  once".  Networking's had a stop with no time limit (it has 2 seconds) and both services in trouble when
  TCP fails (UDP says stopped).  PROTOCOL.md was right byte for byte; it gained what it never said: the
  name rule, the 150 ms floor, which answers start the hold, and which Connects get no answer.  Still
  version 2.
- **README.md rewritten**: the program and the server, every tool by name, the monitor, the door and the
  access lists, the web admin's tabs, what it needs, how to run it, the config files, the test client's
  switches, and the layout.  The old one said Conductor runs on Windows; it's written for Windows and
  never built there, and says so now.
- **TODO.md**: the doc-session item out, the protocol version and the Control Panel item brought up to
  date, and a list of stale words in code comments and one Warn, found reading the code, for whichever
  session next touches those files.
- **The page's greying, fixed** (Jacob: "fix it now").  `lockChanges()` only ever greyed, so `user`,
  LOG OUT, then `admin` in the same page left SHUT DOWN, TEST NOTIFICATION, both ACK ALLs and the two
  list ADD fields dead until a reload.  Now it sets each from the role both ways, SHUT DOWN stays greyed
  while one is on its way, and the Settings and list tabs are asked for again when the role changes
  while one is open.  Checked headless with made-up states only.
- **DiskMan notices a hand edit** (Jacob's pick, after the question of whether to fix the soft files'
  comment or DiskMan).  Every held file keeps its modified time and size from when the copy last matched
  the disk, and a read of a clean copy asks the disk for those two first (on DiskMan's thread, never the
  bytes): changed, and the file is read again.  A copy with a write of ours on its way still wins.  So
  STOP SERVER, edit `postgres.cfg` or `networking.cfg` or a list or the TLS files, START SERVER, and the
  edit is read.  `cache.rs` (`Stamp`, `Held`, `checks`, `after_check()`, `current()`,
  `note_on_disk()`), `worker.rs` (the checks, the stamps after its own writes, streams), `diskman.rs`
  (`read()`); two new tests.  Not built yet: Jacob builds.
- **TEST_CHECKLIST.md**: the two-clients check named `--kick` and `--spare`, which the client never had.
- PROJECT_OPUS.md: the tab count, the `.gitignore` line, the access lists as a named piece.

## The session before -- 2026-09-29, the access lists

It matters here because its hand checks are all open.  The whitelist and the blacklist (`access.rs`),
`access_list` in `networking.cfg`, the Network Admin subsection on the page (Connections, Whitelist,
Blacklist), the three-dot menu on a connection, Recent and Historical views of the door, and a ban that
drops a player with a Kicked reason 3 (**protocol version 2**).  A blacklisting with the blacklist on, or
a whitelist removal with the whitelist on, drops everybody the door would now turn away.  The design is
in `design/conductor-networking.md` and `design/conductor-wgui.md`.

## What's waiting

- **The 2026-09-29 access lists section of `TEST_CHECKLIST.md`**, everything after the build line.
- **The rest of the TCP tab's checks** and **the untried networking hand tests** in the same file.
- **The page's greying fix**: its check at the bottom of `TEST_CHECKLIST.md`.
- **The stale words in the code**, in TODO.md.
- **DiskMan noticing hand edits**: its checks at the bottom of `TEST_CHECKLIST.md`, and `cargo test`.
- **The throwaway account.**  No code for it: the Argon2 line was made outside Conductor at Security's
  settings (64 MiB, one pass, one lane) and Jacob inserts the row by hand.  The account is `throwaway_01`
  with the password `Throwaway 1!`, a test row on a database that only listens on his machine.  Any
  Argon2id line at those settings does, since the stored line carries its own settings.  The real account
  flow (making one over the protocol) is its own session.
- **Where the test client lives** and what it's called.  It's `Conductor/dev/conductor-networking/
  test_client.py` for now.
- **Move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`** (Jacob, 2026-09-29).  One entry
  moves in `files.rs`; `Settings` / `settings()` in `constellations.rs` and the launcher's
  `conductor_wgui::start(...)` call read it from `WGUI` instead; both committed `Content/cfg/` files
  change; the boot line "Settings from ..." and the docs follow.  Both files are hard, so nothing about
  reboots changes.  A file that lacks the setting gets it appended with the default on the next load; the
  stale line in `conductor_globals.cfg` would be Warned about once, so the committed file drops it.
- Archivist retrying on its own every 5 seconds while disconnected.  Asked, not answered.
- The Debug switch in `conductor_globals.cfg`.  Networking's chatter is already Debug; the launcher's
  start and stop lines and Archivist's aren't yet.
- Catching Ctrl-C.
- The Windows build, whenever getting to that machine is less of a hassle.  `dns/windows.rs` joins the
  monitor's Windows file as never built.
- **Ensemble**, its own session.  The Unity project is `Ensemble/dev/Opus.Ensemble/`, on Jacob's machine
  and untracked.  He added its `Assets/`, `Packages/` and `UserSettings/` to `.gitignore` himself (his
  lines, kept as he wrote them), which keeps the purchased art out and, for now, the project's own
  scripts too.  Before any of it is committed, a look together: what a Unity project commits, where the
  purchased art goes (CLAUDE.md says `Content/Assets/`, but Unity wants assets under the project's
  `Assets/`), and LFS for anything big.
