<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is four crates now.  `conductor-tools` (lib) holds Scribe, Constellations, Archivist, the clock and
the thread list.  `conductor-monitor` (lib) looks at the process once a second.  `conductor-wgui` (lib) is the
web admin at `http://127.0.0.1:9996/Opus`, and the only way to shut the server down.  `conductor-launcher`
(bin) starts all of it and waits on the web admin; its console is only Scribe's output.  Ensemble hasn't
been started.

Jacob ran it on Nobara Linux 44 (kernel 7.2.6) and the page worked: CPU, memory, disk, the machine, Archivist
connected, and all four threads with their CPU time.  Two things on the page were off and got fixed after
(below), and that fix hasn't been run yet.  The build and test output wasn't pasted, so "no warnings" and the
test count (about 47 on Linux) are unconfirmed.  The Windows code has never been built.

## Last session -- 2026-09-28

The web admin and the monitor.

What we did:

- **conductor-monitor** (lib, named by Jacob as the "probe into the system").  Once a second it reads memory,
  CPU, disk reads and writes, every OS thread with its share of a core, our own thread list, and Archivist's
  status.  Linux reads `/proc`.  Windows asks kernel32 (and ntdll for the version).  macOS builds and runs but
  measures nothing yet.  No crate for any of it.
- **conductor-wgui** (lib, named by Jacob).  A web server on 127.0.0.1 on the standard library's
  `TcpListener`: `/Opus` is the page, `/Opus/status` the JSON it asks for once a second, and a POST to
  `/Opus/shutdown` shuts down.  The page follows Gemini's mockup colours in plain CSS and pulls nothing from
  the web.  Threads show two ways: **in use** (every OS thread and its CPU) and **asked for** (ours, who
  started each and when).  Another site in the same browser can't shut it down: the Host header has to be
  ours, and the shutdown needs an `X-Opus` header.
- **threads::spawn()** in the tools.  Every thread we start goes through it, which is how the page knows
  who asked for what.
- **Scribe** prints every line to the console as well as the file, and keeps the last 200 in memory for the
  page.  It writes to stdout without `println!`, which would panic on a closed pipe.
- **Archivist** counts `query()` jobs as reads and `execute()` as writes.
- `wgui_port = 9996` in `conductor_globals.cfg`.
- The launcher's L/Q menu and its 7 tests are gone.
- CLAUDE.md: the new crates, the threads rule, the web admin as the only way in, and a "Linux, Windows, macOS"
  section.

What fought back:

- Scribe's panel on the page came out one column wide.  It wasn't the log, it was the page asking for a
  `span-12` CSS class that was never written.  Added.
- The memory bars were all full height, which read as "maxed out".  They were scaled to their own highest
  value, so steady memory filled every bar.  Now scaled to twice that, so steady sits at half height.

What Jacob decided:

- The names: conductor-monitor measures, conductor-wgui shows.  Kept apart on purpose.
- Plain HTTP on 127.0.0.1.  He asked for HTTPS with a self-signed certificate first, then picked HTTP: nothing
  leaves the machine, and TLS comes with Security for the game anyway.
- The console takes no input once the web admin is up.  Shut Down on the page ends the program, and the
  console with it.
- Linux and Windows both, Linux preferred, macOS if it can be done.  No OS setting in the config, since the
  compiler already knows.
- Windows testing waits.  His Windows machine means a thumb drive and a lot of getting up.
- Memory per thread can't be shown (no OS tracks it), so memory is per process and CPU is per thread.
- **Next: services, the way Zabbix or the TLP at work does it.**  An "expected services" section on the page:
  Scribe, Constellations, Archivist, the monitor, the web admin, and the disk manager once it exists.  Each is
  expected to start and keep running, and anything that isn't gets a flashing red mark.  Jacob picked this
  for the next conversation.

## What's waiting

- **The expected services.**  The idea so far (not agreed yet, so talk it through first): a small services
  list in `conductor-tools` next to the thread list.  Every expected service is named up front as "expected,
  not started".  Each one reports on itself: starting, running, trouble with a reason, stopped, plus a "last
  seen" time for the ones with a thread.  A service that never started, stopped, or hasn't checked in for a
  few seconds flashes red on the page, and NOMINAL turns red.  What "healthy" means for each is the table in
  `design/conductor-wgui.md`.  Open: how long "hasn't checked in" is, whether a missing disk manager shows as
  "not built yet" or not at all, and whether the main thread gets named "main" while we're in there.
- Run the page fix: Scribe's panel full width, memory bars at half height.
- Paste back `cargo build` and `cargo test`, to confirm no warnings and the test count.
- The Windows build, when getting to that machine is less of a hassle.
- The Debug switch in `conductor_globals.cfg`, and moving the routine log lines to Debug.
- `\dt` in psql to confirm `archivist_migrations` exists.
- Accounts (make, check, log in), which waits on Security for Argon2.
- The rest of Conductor's tools: the disk manager and Security.
- Picking Ensemble's engine.
