<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is six crates.  `conductor-tools` (lib) holds DiskMan, Scribe, Constellations, Fingerprinter,
Security, Archivist, the notices, the clock, the thread list, the services list and the server's switch
(`server.rs`).  `conductor-accounts` (lib) is an account as the server holds it in memory.
`conductor-monitor` (lib) looks at the process and every process on the machine once a second.
`conductor-networking` (lib) is the front door: a login over TLS on TCP that hands a player a ticket for
UDP, the UDP side the game will run on, a ledger of every connection at the door, and a whitelist and a
blacklist of addresses checked at the door.  `conductor-wgui` (lib) is the web admin at
`http://127.0.0.1:9996/Opus`, and the only way to start and stop the server and to shut Conductor down.
`conductor-launcher` (bin) boots the program and waits on the Control Panel.  Ensemble is Unity 6000.6,
on Jacob's machine, not in the repo.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The web admin has a login: `admin` / `admin` does everything,
`user` / `user` looks, both passwords in `wgui.cfg`.  The server (Fingerprinter, Security, Archivist,
networking, the monitor, and whatever comes later) only starts when START SERVER is pressed on the web
admin's Control Panel, and STOP SERVER takes it back down with Conductor still running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests, `main` is the
stable release, moved only when Jacob says.  At this close all three are on the same commit: Jacob said
to merge to `main`.

**Built and tested on Linux (Nobara 44), 2026-09-29**: everything through the accounts crate.
`cargo build` with no warnings, `cargo test` 204 passed (4 accounts, 15 monitor, 60 networking, 91 tools
with the benchmark ignored, 34 web admin).  Jacob cleared every open check.  The Windows code has never
been built.

## Last session -- 2026-09-29, the account in memory

Jacob's pick: an accounts crate holding a real structure for an account from the database.

What we did:

- **`conductor-accounts`, a lib.**  `Account` is the account in memory, Jacob's words: "our rust
  representation of an account from the database", which can be dumped back to it.  Every column but the
  password hash (left out so it can never be logged or shown): `id()`, `uuid()`, `username()`,
  `created_at()` read only; `first_name`, `last_name`, `email`, `last_login` changeable.  `load(name)`,
  `account.save()` (writes only if something changed since the row was read or written; "the same if
  it's the same, don't even bother writing"), `password_hash(name)` for the login, and `Account::new()`
  plus `create(account, hash)` for making one.  Everything hands back Archivist's `Pending`.
- **The login moved onto it.**  `tcp.rs` has no SQL of its own now.  It reads the hash, checks the
  password, deals with an account already in the world, then loads the `Account` (after the kick, so
  the kicked session's save is in the row first: Archivist has one worker and goes in order).
- **The book holds the account.**  In `sessions.rs` a ticket holds it, then the player.  Every way out of
  the book (Goodbye, quiet, kicked, banned, replaced, a ticket that ran out, STOP SERVER) saves it once
  the lock is let go.  `sweep_in()` lost its two `retain()`s for find-then-remove, through the new
  `remove_ticket_in()` and the old `remove_player_in()`.
- **The login time is the UDP connect** (Jacob: "it's their UDP connection time we want", for playtime
  metrics later).  Stamped in memory when the ticket is used, written when the player leaves.  Checked:
  the row read 20:02:25Z, the second the log said the player was in the world.  If Conductor dies
  without a clean stop, the login times of everyone in the world are lost (Ctrl-C isn't caught yet).
- **Security's 64 MiB**: Jacob saw 74 MB on the page with the server up and nobody in.  That's the
  arena, touched at START SERVER on purpose; the accounts are a few hundred bytes each.
- **The folder rename** asked for mid-session went to TODO.md: folders only, crates keep `conductor-`.

## What's waiting

- **The protogame library** (Jacob's pick at this close): what takes over after the login, holds a
  reference to the player's account, and builds up the UDP session.  To build on: `sessions.rs` (the
  book: tickets, players, each holding its `Account`, saved as it leaves), `udp.rs` (keep-alives,
  Goodbye, the sweep), and conductor-accounts.  Open until the session asks: its name (a lib, by the
  rule for server pieces), what moves out of networking into it, and whether the game loop starts here.
- **Drop `conductor-` from the crate folders**, folders only; the crates keep their names.  In TODO.md.
- **Playtime metrics**: a table of play sessions.  In TODO.md.
- **Making accounts from the web admin** (game account management).  `Account::new()` and `create()` are
  ready; the form, its route and the checks on the fields aren't.  Jacob: the admin makes accounts,
  players don't.
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
