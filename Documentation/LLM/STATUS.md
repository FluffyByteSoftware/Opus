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
ticket for UDP, the UDP side the game will run on, a ledger of every connection at the door for the web
admin's Connections tab, and, since the seventh session of 2026-09-29, a whitelist and a blacklist of
addresses checked at the door.  `conductor-wgui` (lib) is the web admin at
`http://127.0.0.1:9996/Opus`, and the only way to start and stop the server and to shut Conductor
down.  `conductor-launcher` (bin) boots the program and waits on the Control Panel.  Ensemble hasn't been
started.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The web admin has a login: `admin` / `admin` does everything,
`user` / `user` looks, both passwords in `wgui.cfg`.  The server (Fingerprinter, Security, Archivist,
networking, the monitor, and whatever comes later) only starts when START SERVER is pressed on the web
admin's Control Panel, and STOP SERVER takes it back down with Conductor still running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests (the session
pushes `unstable` onto it when a round is ready), `main` is the stable release, moved only when Jacob
says.  `main` sits on the TCP tab session; `unstable` and `testing` carry the access lists session on top
of it, untested.

**Built and tested on Linux (Nobara 44), 2026-09-29**, from `testing` at `c4cfd59`: the login and the
Settings tab.  `cargo test` passed 132 (15 monitor, 89 tools with the benchmark ignored, 28 web admin), and
a run did the whole loop: admin logged in, START SERVER, a save of `wgui_port` on the Settings tab, STOP
SERVER, SHUT DOWN, and the next run came up on the new port.  **Networking built and ran the same day**,
from `testing`: `cargo build` clean after one Cargo.toml fix (a feature that didn't exist), and the Python
client did the whole loop against a debug build: TLS 1.3 in 1 ms, a Ticket in 544 ms (debug Argon2 is
six times slower than release), UDP Connect accepted, keep-alives answered, Goodbye logged as a logout.
STOP SERVER and SHUT DOWN came down clean with networking in the mix.  `cargo test` hasn't been pasted
back yet.  Jacob committed `Cargo.lock` and `Content/certs/conductor.crt` from his machine onto `testing`,
and the session merged that into `unstable`.  The Windows code has never been built.  The TCP tab was
built and seen working the same day.  **The access lists (the seventh session) have not been built
anywhere yet.**

## Last session -- 2026-09-29 (the seventh that day)

**The whitelist and the blacklist**, Jacob's pick, with the web admin's Network Admin subsection.  His
words for it at the open: a whitelist, a blacklist, and the ability to manage them under a new page in
the wgui; on the TCP page, click three dots ("in style like a : colon") to add the address to either as
a quick menu.  His answers mid-session: a blacklisting kicks the session at once, UDP too; subnets with
slashes; the lists are `whitelist.cfg` and `blacklist.cfg`, pointed at from `networking.cfg`, an
exception to Constellations' `key = value`, not editable from the page while the server is stopped
(edit them on disk then), rebuilt into memory on every START SERVER; a sidebar subsection "Network
Admin" with Connections (TCP then UDP on one page), Whitelist and Blacklist under it; the routes are
`addip` and `removeip` ("add" and "remove" were too generic).  He also said, mid-session, that the
next session is documentation clean-up and management (TODO.md).

**Not built anywhere yet.**  The page was rendered headless with made-up numbers and clicked through
(no script errors, the tabs unlock, the menu opens, ADD and REMOVE post the right paths and header);
the Rust was read over by eye and no more.  `cargo build` and `cargo test` are Jacob's, on `testing`.

What we did:

- **`access.rs`** in conductor-networking: `Mode` (off, whitelist, blacklist), `List`, `Entry` (an
  address or a range, host bits cleared, IPv4-in-IPv6 checked as IPv4), `Verdict`; `start(mode, paths)`
  reads both files through DiskMan (writes an empty one with a heading where there isn't one) on every
  START SERVER, `add()` and `remove()` change the list in memory and write the file behind it, not
  waited on.  A bad line in a file is a Warn and skipped.  An empty whitelist with the whitelist on is a
  Warn: nobody can log in.
- **`networking.cfg`**: `access_list` (off), `whitelist_file` (cfg/whitelist.cfg), `blacklist_file`
  (cfg/blacklist.cfg), in `files.rs`, `settings.rs` and the committed file.  The two list files are
  committed too, empty with their headings, exactly as `access.rs` would write them.
- **The door**: the acceptor asks `access::verdict()` before the failure hold; `End::Blacklisted` and
  `End::NotWhitelisted` on the ledger, Debug lines.  UDP's Connect asks too and answers a listed address
  with silence.
- **The ban**: `list_address()` in networking's `lib.rs`; a blacklisting while the blacklist is on calls
  `tcp::close_matching()` (every open socket from inside the entry shut, `End::Banned`) and
  `sessions::drop_where()` (every player at such an address dropped, told nothing).  `Listed` says what
  it did (was_new, enforced, tcp_closed, players_dropped); `Unlisted` for the other way.
- **`sessions::players()`**: every player copied out (address, account, connected when, playing for,
  quiet for), newest first; `Player` got `connected_at` and `connected` for it.  `Status` grew
  `in_world`, `access`, `whitelisted`, `blacklisted`.
- **The web admin**: `GET /Opus/networking`, `POST /Opus/wwwhook/networking/addip` and `/removeip`
  (`?list=&entry=`, `X-Opus: networking`, admin only, 400 with the reason for a bad entry, 409 while
  not running); `unescape()` for the `%2F` in a range; `json::access()`, `listed()`, `unlisted()`; the
  status shape grew (top of `json.rs`).  The page: the NETWORK ADMIN heading and three indented tabs
  under the TCP tab's lock; Connections is the old TCP tab (renamed; its `tcp-*` ids kept) plus a UDP
  table; the three-dot menu (`#row-menu`, outside the table, closed on a click elsewhere, Escape or a
  tab change); the Whitelist and Blacklist tabs from one `drawList()`; `lockChanges()` greys the ADD
  fields for `user`.  The grey-not-green finished row from TODO.md got its one CSS line.
- Docs: CLAUDE.md (the exception to Constellations' rule, the eleven tabs, the three tabs, the two
  routes), both design docs, PROJECT_OPUS, README, TODO (the whitelist item and the UDP tab item
  closed out; the Kicked reason, the whitelist edge, the switch at once, the doc session, the DiskMan
  hand-edit caveat, two ideas), TEST_CHECKLIST.

What Jacob decided (all 2026-09-29): the five answers above, and the sidebar layout.  The session's
own calls, said in the reply and open to change: taking an address off a list kicks nobody; a banned
player is told nothing (a Kicked reason is a protocol change, TODO.md); an entry on a list that isn't
switched on is kept and does nothing; the Network Admin group sits where the TCP tab was, between
Storage and Notifications History.

## What's waiting

- **The 2026-09-29 access lists section of `TEST_CHECKLIST.md`**, all of it, on `testing`: the build
  first.  If the build fails, the fix comes back here.
- **The rest of the TCP tab's checks** in the same file: a wrong password on the tab, KICK, the
  five-minute drop, the setting at 10, `net-dns` on the threads list, `user` seeing the buttons greyed.
- **The next session is documentation clean-up and management**, Jacob's word mid-session.
- The items below are unordered.
- **A hand edit to a list file between a STOP SERVER and a START SERVER isn't seen**: DiskMan serves the
  copy it holds.  With Conductor shut down it's fine.  TODO.md under DiskMan; worth a word to Jacob if
  it bites in testing.
- **The untried hand tests from the networking session** at the top of `TEST_CHECKLIST.md`: two clients
  on one account, `--go-quiet`, a wrong secret word, a wrong password and the hold, STOP SERVER with a
  player in the world, the `user` account in a real run.
- **The throwaway account.**  No code for it: the Argon2 line was made outside Conductor at Security's
  settings (64 MiB, one pass, one lane) and Jacob inserts the row by hand.  The account is `throwaway_01`
  with the password `Throwaway 1!`, a test row on a database that only listens on his machine.  The line
  to run in psql is in the 2026-09-29 networking session's last reply; if it's lost, any Argon2id line at
  those settings does, since the stored line carries its own settings.  The real account flow (making one
  over the protocol) is its own session.
- **Where the test client lives** and what it's called.  It's `Conductor/dev/conductor-networking/
  test_client.py` for now.
- **Move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`** (Jacob, 2026-09-29).  One entry
  moves in `files.rs`; `Settings` / `settings()` in `constellations.rs` and the launcher's
  `conductor_wgui::start(...)` call read it from `WGUI` instead; both committed `Content/cfg/` files
  change; the boot line "Settings from ..." and the docs follow.  Both files are hard, so nothing about
  reboots changes.  A file that lacks the setting gets it appended with the default on the next load; the
  stale line in `conductor_globals.cfg` would be Warned about once, so the committed file drops it.
- On GitHub, by hand: delete `claude/gracious-ramanujan-j5ojyq`, `testing_/charming-euler-jlyos7`,
  `testing_/youthful-tesla-879b9w` and `testing_/gracious-carson-6zddl8` (this session's, unused).
- Archivist retrying on its own every 5 seconds while disconnected.  Asked, not answered.
- The Debug switch in `conductor_globals.cfg`.  Networking's chatter is already Debug; the launcher's
  start and stop lines and Archivist's aren't yet.
- Catching Ctrl-C.
- The Windows build, whenever getting to that machine is less of a hassle.  `dns/windows.rs` joins the
  monitor's Windows file as never built.
- **Ensemble is Unity 6000.6**, Jacob said at the close, 2026-09-29.  The Unity project is
  `Ensemble/dev/Opus.Ensemble/`, on his machine and untracked, while he goes through his catalogue of
  assets.  He added its `Assets/`, `Packages/` and `UserSettings/` to `.gitignore` himself the same day
  (his lines, kept as he wrote them), which keeps the purchased art out and, for now, the project's own
  scripts too; that's his to revisit when Ensemble gets a session.  Before any of it is committed, a
  look together: what a Unity project commits (`Assets/`, `Packages/`, `ProjectSettings/`; the caches
  are already in `.gitignore`), where the purchased art goes (CLAUDE.md says `Content/Assets/`, never
  committed, but Unity wants assets under `Ensemble/dev/Assets`), and LFS for anything big.  Its own
  session.
