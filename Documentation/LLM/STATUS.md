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
stable release, moved only when Jacob says.  At the close of the test run all three are on the same
commit: Jacob said to merge everything to `main`.

**Built and tested on Linux (Nobara 44), 2026-09-29**: everything through KICK on any row, and the
whole test run.  `cargo clean`, `cargo build` with no warnings, `cargo test` 199 passed (15 monitor, 59
networking, 91 tools with the benchmark ignored, 34 web admin).  Every hand check in TEST_CHECKLIST.md
passed on `testing`.  `main` moved up to `testing` at the close of this session, Jacob's word.  The
Windows code has never been built.

## Last session -- 2026-09-29, the test run

Jacob's pick: go through every open check in TEST_CHECKLIST.md on `testing`.

What we did:

- **The checklist, by subsystem.**  Every open check gathered into one run in the order of the piece it
  tests (the build, the page before START SERVER, Constellations and DiskMan, the TLS pair, the door,
  leaving the world, KICK, the access lists, `user` and `admin`), with a one-line command each.  Checks
  that no longer matched the build were rewritten: with the TLS files gone, TCP is in trouble and UDP
  stopped (not both in trouble); KICK is on every row now; Ctrl-Z after TLS can't be caught; a hand
  edit to a list file only needs the server stopped.
- **`--pause-before-login N`** on `test_client.py` (Jacob's name): sits N seconds after TLS before the
  Login, so the connection can be kicked or banned while open.  Past `login_deadline_seconds` (10) the
  server hangs up first.  A hang-up during the login prints a line, not a traceback.
- **The run.**  Everything passed.  Jacob's notes went into the web admin's design doc and README: before
  login the page is only the login card; with the server stopped only the Control Panel, the Log and
  the Settings open, for both accounts; `user` reads the Log and the Settings; the bell waits for START
  SERVER, and a Warn from boot is in the Log tab meanwhile.
- **Bug 1, fixed.**  After RESTART SERVER the Settings tab still said a change saved on it was waiting.
  The server was right (clicking away and back cleared it): the tab was only drawn when opened, and
  opened in the middle of the restart it kept a snapshot from before the stop swapped the change in.
  `page.html` now asks again whenever the server's state, or when it got there, changes while the tab
  is open.  Its check is the one left in TEST_CHECKLIST.md; the syntax was checked in the session, no
  more.  Worth knowing: a change saved from the Settings tab and still waiting wins over a hand edit to
  the same file, since it's swapped in over it at the stop.
- **The TLS error's command** (Jacob's yes): `tls::make_pair()` builds it from the full paths
  `networking.cfg` gives, in quotes, "from any folder".  The old relative one, pasted from
  `Conductor/dev`, is what made a stray `Content` there once.
- **TEST_CHECKLIST.md cleared** (Jacob: it's his reminder, not a history).  A passed check is taken out
  now, not struck through; CLAUDE.md says so.  Left: the Settings tab's check, and the parked two (an
  outside machine, the Windows build).

## What's waiting

- **An accounts crate** (Jacob's pick at this close): a crate that holds a real structure for an
  account, read from the database.  What's there to build on: the `accounts` table
  (`Content/psql/defaults/schemas/accounts.sql`, frozen, so any change is a migration; `uuid` came with
  0001), with Postgres checking the name (8 to 32 of `a-z`, `0-9`, `_`) and the email itself; Security's
  `hash_password()` and `check_password_rules()` (8 to 128 printable ASCII, a digit, a capital, a
  symbol); Fingerprinter's `new_uuid()`; Archivist's `transaction()` and `Pending`; and the login, which
  today reads `password_hash` by `account_username` and stamps `last_login_datetime` straight from
  `tcp.rs`.  Open until the session asks: the crate's name and that it's a lib; what the structure
  holds; whether the login's two queries move into it; and who makes an account (a player over the
  protocol, a new packet and a protocol version; the admin from the web admin; or both).  TODO.md has
  "Accounts: making one" and the web admin's game account management.
- **The Settings tab's check** in TEST_CHECKLIST.md (bug 1's fix, built on `main` but not looked at).
- **The throwaway account.**  `throwaway_01` / `Throwaway 1!`, inserted by hand with an Argon2id line
  made outside Conductor at Security's settings (64 MiB, one pass, one lane).
- **The stale words in the code**, in TODO.md.
- **Move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`** (Jacob, 2026-09-29).  One entry
  moves in `files.rs`; `Settings` / `settings()` in `constellations.rs` and the launcher's
  `conductor_wgui::start(...)` call read it from `WGUI` instead; both committed `Content/cfg/` files
  change; the boot line "Settings from ..." and the docs follow.  Both files are hard, so nothing about
  reboots changes.
- **Where the test client lives** and what it's called.  It's `Conductor/dev/conductor-networking/
  test_client.py` for now.
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
