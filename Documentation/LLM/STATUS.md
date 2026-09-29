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
second.  `conductor-networking` (lib, new this session) is the front door: a login over TLS on TCP that
hands a player a ticket for UDP, and the UDP side the game will run on.  `conductor-wgui` (lib) is the web
admin at `http://127.0.0.1:9996/Opus`, and the only way to start and stop the server and to shut Conductor
down.  `conductor-launcher` (bin) boots the program and waits on the Control Panel.  Ensemble hasn't been
started.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The web admin has a login: `admin` / `admin` does everything,
`user` / `user` looks, both passwords in `wgui.cfg`.  The server (Fingerprinter, Security, Archivist,
networking, the monitor, and whatever comes later) only starts when START SERVER is pressed on the web
admin's Control Panel, and STOP SERVER takes it back down with Conductor still running.

**The branches**: `unstable` is where the sessions write, `testing` is where Jacob tests (the session
pushes `unstable` onto it when a round is ready), `main` is the stable release, moved only when Jacob
says.  `main` sits on the login and the Settings tab; `unstable` and `testing` carry networking on top.

**Built and tested on Linux (Nobara 44), 2026-09-29**, from `testing` at `c4cfd59`: the login and the
Settings tab.  `cargo test` passed 132 (15 monitor, 89 tools with the benchmark ignored, 28 web admin), and
a run did the whole loop: admin logged in, START SERVER, a save of `wgui_port` on the Settings tab, STOP
SERVER, SHUT DOWN, and the next run came up on the new port.  **Networking has never been built**: it was
written in a session with no compiler at hand, so the first `cargo build` is the test.  The Windows code
has never been built.

## Last session -- 2026-09-29 (the fifth that day)

**Networking**, Jacob's pick: `conductor-networking`, named by him.  Written against the five Stratum
files he uploaded as a model (lib, protocol, sessions, tcp, tls, udp, and the old Python test script),
not ported.  His steer: lower CPU, more RAM where that buys it, one login hashed at a time.  Not built,
not run.

What we did:

- **The crate**: `lib.rs` (start, stop, status), `settings.rs`, `tls.rs`, `protocol.rs`, `sessions.rs`,
  `tcp.rs`, `udp.rs`, and `test_client.py` beside it.  In `start_server()` after Archivist and
  `stop_server()` before Security.  Two services, "Network (TCP)" and "Network (UDP)".  One new crate,
  `rustls` 0.23 with `ring`, TLS 1.3 only.
  The design doc, `design/conductor-networking.md`, has every decision and the reasons.
- **The shape**: TCP is only the login.  Hello, one Login (version, secret word, username, password), then a
  Ticket (token and UDP port) or a LoginResult, and the connection closes.  UDP is everything after:
  Connect with the token, KeepAlive both ways once a second, Goodbye, Kicked.  A UDP session that ends any
  way is gone; the client starts over at the login.  `PROTOCOL.md` is written out in full, bytes and a
  worked example.
- **Lower CPU**: a fixed pool of login threads (`login_threads`, 8) fed by the acceptor through a bounded
  queue (`max_waiting_logins`, 64), instead of Stratum's thread per connection; no polling anywhere (every
  read is armed with the time left to its deadline, and `stop()` wakes threads by knocking and by
  shutting their sockets); the failure hold is a closed door, not a sleeping thread; a player is a map
  lookup by address; the fixed answers are built once; a UDP packet is read in place.  The version, the
  secret word and the name's shape are checked before anything costs a hash.
- **The client is told its place in Security's line**: `Pending::wait_for()` and `Ticket::wait_for()` are
  new in `conductor-tools`, and the login thread sends an InLine between one-second waits.
- **`networking.cfg`**, a new soft file in Constellations' table with twelve settings (address, both
  ports, the two TLS file paths, the secret word, the client versions, the login deadline, the pool and
  queue sizes, the token deadline at 30 s, the UDP timeout at 40 s).  The committed file is in
  `Content/cfg/`.
- **`Content/certs/`** is the new folder for the TLS pair, made by hand with the openssl command in
  README.md; `*.key` is gitignored.  Conductor makes the folder and, when the files are missing, logs the
  command in capitals and shows both network services in trouble without stopping the rest.
- Docs: PROTOCOL.md, the networking design doc, the tools and web admin design docs (services, files,
  `wait_for`), TODO (client management as one heading), PROJECT_OPUS, README, CLAUDE.md (a Networking
  section under the Rust rules).

What Jacob decided:

- The crate is `conductor-networking`.  rustls, as in Stratum.  The certificate lives in `Content/certs/`
  and is made with openssl by hand.
- Client management (a player limit, reconnect tokens, kicking from the web admin) waits.  The point of
  this iteration is handing a client from TCP to UDP and logging them off.  The UDP timeout is 40 seconds.
- A login for an account already in the world gets a packet asking the new client whether to kick the old
  session (Stratum's SessionChoice).  Built.
- Testing is a Python script, as it was for Stratum.  Ours is `test_client.py` beside the crate, in the
  repo; Stratum's was scratch.  Whether ours stays in the repo is his call (asked at hand-off).

## What's waiting

- **Build it.**  `cargo build` and `cargo test` from `Conductor/dev`.  The rustls calls follow what
  Stratum compiled with, but nothing here has met a compiler.  Cargo.lock changes; Jacob commits it.
- **Make the TLS pair** with the command in README.md, then START SERVER, then the Python client.  With no
  account in the table every login ends in "Invalid Credentials", which still walks TLS, the framing,
  Archivist, Security's line and the hold.
- **The throwaway account.**  No code for it: the Argon2 line was made outside Conductor at Security's
  settings (64 MiB, one pass, one lane) and Jacob inserts the row by hand.  The account is `throwaway_01`
  with the password `Throwaway 1!`, a test row on a database that only listens on his machine.  The line
  to run in psql is in the 2026-09-29 session's last reply; if it's lost, any Argon2id line at those
  settings does, since the stored line carries its own settings.  The real account flow (making one
  over the protocol) is its own session.
- **Where the test client lives** and what it's called.  It's `Conductor/dev/conductor-networking/
  test_client.py` for now.
- **Move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`** (Jacob, 2026-09-29).  One entry
  moves in `files.rs`; `Settings` / `settings()` in `constellations.rs` and the launcher's
  `conductor_wgui::start(...)` call read it from `WGUI` instead; both committed `Content/cfg/` files
  change; the boot line "Settings from ..." and the docs follow.  Both files are hard, so nothing about
  reboots changes.  A file that lacks the setting gets it appended with the default on the next load; the
  stale line in `conductor_globals.cfg` would be Warned about once, so the committed file drops it.
- The web admin shows nothing of networking yet; `conductor_networking::status()` has the numbers.
- The `user` account hasn't been tried in a real run yet, only in the tests and the headless check.
- On GitHub, by hand: delete `claude/gracious-ramanujan-j5ojyq` and `testing_/charming-euler-jlyos7`.
- Archivist retrying on its own every 5 seconds while disconnected.  Asked, not answered.
- The Debug switch in `conductor_globals.cfg`.  Networking's chatter is already Debug; the launcher's
  start and stop lines and Archivist's aren't yet.
- Catching Ctrl-C.
- The Windows build, whenever getting to that machine is less of a hassle.
- Picking Ensemble's engine.
