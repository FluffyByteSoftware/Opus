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
ticket for UDP, the UDP side the game will run on, and, since the sixth session of 2026-09-29, a ledger of
every connection at the door for the web admin's TCP tab.  `conductor-wgui` (lib) is the web admin at
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
says.  `main` sits on networking and everything before it; `unstable` and `testing` are one session
ahead of it, the TCP tab, which Jacob hasn't built yet.

**Built and tested on Linux (Nobara 44), 2026-09-29**, from `testing` at `c4cfd59`: the login and the
Settings tab.  `cargo test` passed 132 (15 monitor, 89 tools with the benchmark ignored, 28 web admin), and
a run did the whole loop: admin logged in, START SERVER, a save of `wgui_port` on the Settings tab, STOP
SERVER, SHUT DOWN, and the next run came up on the new port.  **Networking built and ran the same day**,
from `testing`: `cargo build` clean after one Cargo.toml fix (a feature that didn't exist), and the Python
client did the whole loop against a debug build: TLS 1.3 in 1 ms, a Ticket in 544 ms (debug Argon2 is
six times slower than release), UDP Connect accepted, keep-alives answered, Goodbye logged as a logout.
STOP SERVER and SHUT DOWN came down clean with networking in the mix.  `cargo test` hasn't been pasted
back yet.  Jacob committed `Cargo.lock` and `Content/certs/conductor.crt` from his machine onto `testing`,
and the session merged that into `unstable`.  The Windows code has never been built.  **The TCP tab
(the sixth session) has not been built anywhere yet.**

## Last session -- 2026-09-29 (the sixth that day)

**The TCP tab and KICK**, Jacob's pick: the web admin shows the login door.  His spec, in his words:
tabs that open after the server is running and the TCP listener and the UDP listener are online; TCP
lists all of the connections that have reached out to our listener in the last 5 minutes by IP address
(and DNS if known), their position in the login queue if not already logged in, no account information,
purely tracked by IP address; UDP (later) shows the account and the character.  Mid-session he added a
KICK for `admin` on the list, and then a whitelist and blacklist with a switch in the config and "the
ability to manage TCP connections", which went to TODO.md under the one-feature rule.  He also asked for
a rolling test checklist, which is `TEST_CHECKLIST.md` now.

**Not built or tested yet on his machine.**  The session can't run cargo; `cargo build` and `cargo
test` from `Conductor/dev` are the first thing, then the checklist.  The page's script was parsed with
node (a syntax check, nothing more) and the JSON shape was worked out by hand.

What we did:

- **`ledger.rs`** in conductor-networking: every connection numbered as it arrives, its Stage (queued,
  handshake, awaiting its Login, checking, in Security's line with `place()`'s numbers, asked about
  another session, done with an End that says how), swept five minutes after arrival once finished,
  capped at 1000 with the oldest finished going first, the first ending written wins.  `tcp.rs` writes
  to it at every stage change: the acceptor at accept (and for a hold or a full queue), the login thread
  through `serve()`, `talk()` (which now hands back an `End`), `log_in()` and `wait_in_line()`.
- **`dns.rs`** and `dns/{linux,windows,other}.rs`: reverse DNS on thread `net-dns`, one lookup per
  address per START SERVER, through the OS's `getnameinfo` in an `extern` block (Linux's C library,
  Windows's ws2_32), no crate.  The Windows file is written and never built, like the monitor's.
- **KICK**: `tcp::kick(id)`.  The `serving` map became `open`: a clone of every open socket from accept
  on, keyed by ledger number, so a queued connection can be kicked too (its login thread skips it).
  `conductor_networking::kick()` and `Kicked { Yes, NotOpen, NotListening }` for the web admin.
- **The web admin**: depends on conductor-networking now; `networking` in the status JSON (shape at the
  top of `json.rs`); `POST /Opus/wwwhook/tcp/kick?id=N` behind `only_admin()` with `X-Opus: tcp`; the
  TCP tab sixth in the sidebar, locked until both listeners are up on top of the database lock, two
  tiles and the table with KICK, and the page steps off it if a listener goes down while it's open.
- Docs: CLAUDE.md (nine tabs, the TCP tab, the kick route, the checklist in the hand-off), the two
  design docs, PROJECT_OPUS, README, TODO (the UDP tab, the lists with the questions they raise,
  macOS DNS), and the new `TEST_CHECKLIST.md`.  `connections_remember_seconds` in `networking.cfg` (the
  five minutes as a setting, at his say) came last.

What Jacob decided:

- The TCP tab shows addresses, never accounts.  Both listeners up before it opens.  Five minutes, and
  a setting for it (`connections_remember_seconds` in `networking.cfg`) at his say, the same session.
- KICK is admin's.  The route is `/Opus/wwwhook/tcp/kick`, under `wwwhook` like the server buttons; he
  OK'd the path.
- The whitelist and blacklist are the next session, his call, with a BAN on the tab as part of managing
  connections.  TODO.md has what it has to settle.
- A rolling test checklist, kept across sessions: `TEST_CHECKLIST.md`.

## What's waiting

- **Build and test this session's work**: `cargo build` and `cargo test` from `Conductor/dev`, then the
  2026-09-29 section of `TEST_CHECKLIST.md`.  Nothing from it has run on Jacob's machine.  The kick and
  the queue place have only unit tests behind them; the ledger and the DNS thread haven't been seen
  running.
- **The UDP tab** (Jacob's spec in TODO.md): the account of every player, and the character once there
  is one.  `sessions.rs` needs a `players()` first.
- **The whitelist and blacklist**, and a BAN on the TCP tab (TODO.md, with the questions).
- The items below are unordered.
- **The untried hand tests from the networking session** moved to the top of `TEST_CHECKLIST.md`: two
  clients on one account, `--go-quiet`, a wrong secret word, a wrong password and the hold, STOP SERVER
  with a player in the world, the `user` account in a real run.  `cargo test` after networking too.
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
- On GitHub, by hand: delete `claude/gracious-ramanujan-j5ojyq`, `testing_/charming-euler-jlyos7` and
  `testing_/youthful-tesla-879b9w` (this session's, unused).
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
