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
connected, and all four threads with their CPU time.  After that the CPU and memory panels were redone (below)
and merged into `main` at Jacob's call without being run, so they're the first thing to look at.  The build
and test output wasn't pasted, so "no warnings" and the test count (about 50 on Linux) are unconfirmed.  The
Windows code has never been built.

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
- After Jacob's first look: CPU became a 0 to 100% chart with one line per core (the whole machine's cores,
  since no OS says which core Conductor's threads ran on), and memory became a bar of the machine's RAM with
  Conductor, everything else, and free.  The monitor reads each core and the machine's RAM for that:
  `/proc/stat` and `/proc/meminfo` on Linux, `NtQuerySystemInformation` and `GlobalMemoryStatusEx` on
  Windows.  Checked by rendering the page with made-up numbers; not run against a real Conductor yet.
- The launcher's L/Q menu and its 7 tests are gone.
- CLAUDE.md: the new crates, the threads rule, the web admin as the only way in, and a "Linux, Windows, macOS"
  section.

What fought back:

- Scribe's panel on the page came out one column wide.  It wasn't the log, it was the page asking for a
  `span-12` CSS class that was never written.  Added.
- The memory bars were all full height, which read as "maxed out".  They were scaled to their own highest
  value, so steady memory filled every bar.  A quick fix (twice the highest) went in, and then Jacob asked
  for the real thing: memory against the machine's 64 GB, and CPU on a fixed 0 to 100% scale per core.

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
  expected to start and keep running, and anything that isn't gets a flashing red mark.
- **Then changed his mind: the next conversation is docking.**  He wants to talk through the web admin's
  layout and make it more flexible by docking the panels.  Services wait until after.

## What's waiting

- **Docking the web admin's panels.**  Jacob's pick for the next conversation, and it starts as a talk, not
  code.  Nothing is decided.  Things to pin down with him before building: what "docking" means to him
  (dragging panels to rearrange the grid, resizing them, hiding and showing them, popping one out into its
  own window, or tabs like an IDE), whether a layout is remembered, and where (the browser only, or
  Conductor saving it to `Content/`), and whether it can be done in plain JavaScript with no library, since
  the page pulls nothing from the web.
- **The expected services.**  The idea so far (not agreed yet, so talk it through first): a small services
  list in `conductor-tools` next to the thread list.  Every expected service is named up front as "expected,
  not started".  Each one reports on itself: starting, running, trouble with a reason, stopped, plus a "last
  seen" time for the ones with a thread.  A service that never started, stopped, or hasn't checked in for a
  few seconds flashes red on the page, and NOMINAL turns red.  What "healthy" means for each is the table in
  `design/conductor-wgui.md`.  Open: how long "hasn't checked in" is, whether a missing disk manager shows as
  "not built yet" or not at all, and whether the main thread gets named "main" while we're in there.
- Run the new page: the per-core CPU chart and the memory bar.  Merged without a run.
- Paste back `cargo build` and `cargo test`, to confirm no warnings and the test count.
- The Windows build, when getting to that machine is less of a hassle.
- The Debug switch in `conductor_globals.cfg`, and moving the routine log lines to Debug.
- `\dt` in psql to confirm `archivist_migrations` exists.
- Accounts (make, check, log in), which waits on Security for Argon2.
- The rest of Conductor's tools: the disk manager and Security.
- Picking Ensemble's engine.
