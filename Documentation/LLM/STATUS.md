<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is six crates.  `conductor-tools` (lib) holds DiskMan, Scribe, Constellations, Fingerprinter,
Security, Archivist, the notices, the clock, the thread list, the services list and the server's switch
(`server.rs`).  `conductor-accounts` (lib) is the one way in to the accounts table, and the account desk.
`conductor-monitor` (lib) looks at the process and every process on the machine once a second.
`conductor-networking` (lib) is the front door: a login over TLS on TCP that hands a player a ticket for
UDP, the UDP side the game will run on, a ledger of every connection at the door, and a whitelist and a
blacklist of addresses checked at the door.  `conductor-wgui` (lib) is the web admin at
`http://127.0.0.1:9996/Opus`, and the only way to start and stop the server and to shut Conductor down.
`conductor-launcher` (bin) boots the program and waits on the Control Panel.  Ensemble is Unity 6000.6,
on Jacob's machine, not in the repo.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The web admin has a login: `admin` / `admin` does everything,
`user` / `user` looks, both passwords in `wgui.cfg`.  The server (Fingerprinter, Security, Archivist, the
account desk, networking, the monitor, and whatever comes later) only starts when START SERVER is pressed
on the web admin's Control Panel, and STOP SERVER takes it back down with Conductor still running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests, `main` is the
stable release, moved only when Jacob says.  At this close `unstable` and `testing` are on the same
commit, the account manager; `main` is still on the accounts-in-memory hand-off (`0a82d90`).

**Built and tested on Linux (Nobara 44)**: everything up to the accounts-in-memory session (204 tests).
**This session's code has not been built yet.**  It was written and pushed to `testing` without a
compile (Jacob builds); only the page was checked, by a script syntax check and a headless render with
made-up data.  Expect a round of compile fixes first.  The Windows code has never been built.

## Last session -- 2026-09-29, the account manager

Jacob's pick: an account manager on the web admin.  His list: create an account, list them, delete one,
change its fields (the password included), and "write accounts to the database".

How the design moved, in order, since each step was his call:

- First he asked for passwords to be untouchable while the server runs, so a change never waits in
  Security's line.  That can't work (Security and Archivist only run with the server), so he turned it
  round: **accounts are only changed while the server is running**.
- "Write accounts to DB" was the question of an account held in memory with its player: an edit from the
  page would be written over when they left.  His answer: **an account is never held in memory**.  It's
  loaded from its row when needed and every change goes straight back.  That undid the holding built the
  session before.
- Deleting an account whose player is online **kicks them, and the client says ACCOUNT TERMINATED**.

What we did:

- **Accounts never held.**  Networking's book has the account's name only; the login no longer loads the
  account; nothing is saved when a player leaves; the login time is written straight to the row at the
  UDP connect (`stamp_login()`).  So a Ctrl-C of Conductor no longer loses login times either.
- **`conductor-accounts`**: `list()`, `taken()`, `edit()`, `set_password()`, `delete()`, and the field
  checks (the table's rules, in words, each complaint tagged with its field).  `create()` answers
  `NameTaken` / `EmailTaken`.  The name rule moved here from `tcp.rs` (`username_allowed()`).
- **The account desk** (`desk.rs`), a new server piece on thread `account-desk`, on the Services tab:
  new accounts and new passwords are hashed and written there, and the page asks after the job, so the
  web admin's one thread never waits in Security's line.  Started after Security and Archivist, stopped
  before them, finishing what it was handed.
- **The Accounts tab** under a new GAME ADMIN heading, `admin` only (`user` can't see the list, Jacob's
  call).  The list; a card per account (click its name): the owner's details (SAVE), a new password typed
  twice (every password is), DELETE ACCOUNT; NEW ACCOUNT's card.  The username never changes.  Reads at
  `/Opus/Content/accounts` (and `/job?id=N`), changes at `/Opus/wwwhook/accounts/create`, `/edit`,
  `/password`, `/delete`: Jacob's paths.
- **Protocol version 4**: Kicked reason 5, account terminated.  `protocol.rs`, PROTOCOL.md and the test
  client together.  The Connections row reads "LINKDEAD: account terminated".
- **A page fix in passing**: every `note` coloured red, yellow or green was showing grey (the Whitelist
  tab's bad entry, the Settings tab's complaints).  It keeps its colour now.
- To TODO.md: the sidebar rethink (menus and submenus; Jacob's pick for next).  To LONGTERM_TODO.md: a
  separate account management program, one day, for changing accounts beside a running Conductor.

## What's waiting

- **The web admin's layout: a better menu for the pages** (Jacob's pick at this close).  Twelve tabs
  under two subsection headings is getting cluttered.  In TODO.md.
- **Building and testing the account manager.**  The checks are under today's heading in
  TEST_CHECKLIST.md.  The build changes `Cargo.lock` (three crates now depend on `conductor-accounts`),
  which wants committing.
- **The protogame library**: what takes over after the login and builds up the UDP session.  The account
  is a name now, read from the row when needed.  In TODO.md.
- **Drop `conductor-` from the crate folders**, folders only.  In TODO.md.
- **Playtime metrics**: a table of play sessions.  In TODO.md.
- **Move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`** (Jacob, 2026-09-29).  In TODO.md.
- **The stale words in the code**, in TODO.md.
- **Where the test client lives** and what it's called.  It's `Conductor/dev/conductor-networking/
  test_client.py` for now.
- Archivist retrying on its own every 5 seconds while disconnected.  Asked, not answered.
- The Debug switch in `conductor_globals.cfg`.
- Catching Ctrl-C.
- The Windows build, whenever getting to that machine is less of a hassle.
- **Ensemble**, its own session.  The Unity project is `Ensemble/dev/Opus.Ensemble/`, on Jacob's machine
  and untracked.  Before any of it is committed, a look together: what a Unity project commits, where the
  purchased art goes, and LFS for anything big.
