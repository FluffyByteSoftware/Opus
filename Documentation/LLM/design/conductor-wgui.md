<!--
File:       Opus/Documentation/LLM/design/conductor-wgui.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-wgui

A lib crate.  The web admin: a small web server on a thread of its own that shows how Conductor is doing and
is the only way to shut it down.  Named by Jacob.  Modeled on how the TLP at Jacob's work is laid out:
critical service status at a glance.  The look came from a mockup Gemini drew (dark slate panels, emerald
accents, a terminal box), redone in plain CSS.

## Skeleton

```
conductor-wgui/
├── Cargo.toml         depends on conductor-tools and conductor-monitor, nothing else
└── src/
    ├── lib.rs         start(port) -> bool, wait(); the thread, route(), host_is_ours()
    ├── http.rs        read_request(), parse_head(), respond(); struct Request
    ├── json.rs        status(snapshot, services, disk, open_notices, newest_notices, lines, log_file),
    │                    notices(open), threads_of(pid, threads) -> String;
    │                    a small Object builder, text() escaping
    └── page.html      the one page, baked in with include_str!
```

## Routes

| Method | Path                   | What it does                                                        |
|--------|------------------------|---------------------------------------------------------------------|
| GET    | `/`                    | Sends the browser to `/Opus`                                        |
| GET    | `/Opus`                | The page                                                            |
| GET    | `/Opus/status?after=N` | The monitor's look, the services, DiskMan, the notices, the log     |
| GET    | `/Opus/threads?pid=N`  | One process's threads, for the System tab.  Reads only.             |
| GET    | `/Opus/notices`        | Every open notice, for the Notifications History tab.  Reads only.  |
| POST   | `/Opus/notices/ack?id=N` | Clears one notice.  Needs `X-Opus: ack`                           |
| POST   | `/Opus/notices/ack-all`| Clears every notice.  Needs `X-Opus: ack`                           |
| POST   | `/Opus/notices/test`   | Raises a test notice.  Needs `X-Opus: ack`                          |
| POST   | `/Opus/shutdown`       | Shuts Conductor down.  Needs the `X-Opus: shut-down` header         |

The JSON shapes are written out at the top of `json.rs`.  The page's script is the other half of them.

## What we decided

- **Plain HTTP on 127.0.0.1 only**, port `wgui_port` in `conductor_globals.cfg` (9996).  Jacob first wanted
  HTTPS with a self-signed certificate, then picked plain HTTP: nothing leaves the machine, and HTTPS would
  cost a crate and a browser warning every time.  When Security brings in TLS for the game, this can use it.
- No crate for the web server.  It's the standard library's `TcpListener`, one request at a time, one per
  connection, with a 2 second limit to send the request.  One admin asking once a second doesn't need more.
- **Offline.**  The page pulls nothing from the internet.  Gemini's mockup used Tailwind and Google Fonts from
  the web; the page keeps the colours and uses the fonts already on the machine.
- **The console takes no input.**  The launcher starts everything and waits on the web admin.  Shut Down on
  the page stops the web admin's thread, main wakes up, the monitor and Archivist stop, and the program ends,
  which ends the console with it.  Ctrl-C still kills it outright, without the clean-up.  Closing the console
  window kills it too (TODO.md has the ways around that).
- If the web admin can't start (the port is taken), Conductor shuts straight back down with a capitals Error,
  since there would be no way to stop it cleanly.
- Another web page open in the same browser could try to reach 127.0.0.1 too.  Two checks stop it: the `Host`
  header must be `127.0.0.1:<port>` or `localhost:<port>`, and the shutdown needs an `X-Opus` header, which a
  browser won't let another site's page add without asking us first (and we never say yes).
- The page draws everything with `textContent`, never `innerHTML` with our data, so a log line with `<` in it
  shows as text.
- Mockup parts left out because there's nothing behind them yet: TPS, network streams, Argon2 load, "restart
  loop".  No made-up numbers on the page.
- **Tabs, not docking** (2026-09-28).  Jacob first asked for panels you could drag into slots like an IDE,
  then dropped it: what he wanted was more at a glance, and tabs do that.  Saved layouts per user wait for
  the web admin to have users.
- **The last tab picked is remembered by the browser** (`localStorage`), and nothing else.  A settings file in
  `Content/web/` was planned and dropped with docking.
- **While the database isn't connected, the page shows nothing but that.**  "The whole point is to draw
  attention to the user that the DB is offline and the game can't run right now."  SHUT DOWN is the only
  thing that works.  The page keeps asking and drawing underneath, so it unlocks the moment Archivist
  connects.
- **Notices** (2026-09-28): every Warn and Error, and anything raised on purpose, waits on the bell until
  it's ACKed.  Jacob asked for them the same session as DiskMan, so a file DiskMan can't write reaches the
  admin.  Talked through: "viewed" clearing them was dropped for a manual ACK; saving them to a file or
  Postgres was dropped for memory only.  The three notice routes that change things were OK'd by Jacob.
- **The bell and its tray stay above the database lock**, like the rest of the header.  The Notifications
  History tab is locked like every other tab.
- Another process's threads are asked for one process at a time, only while it's picked and the System tab is
  open.  Every process's threads every second would be thousands of rows nobody is looking at.

## The page

**Sidebar**: the OP logo, and six tabs under it: System, Conductor, Services, Storage, Notifications
History, Log.  Conductor opens
first, unless the browser remembers another.  The Services tab gets a flashing red dot when a service is down.

**Header, on every tab**: the name; a status line (NOMINAL, or what's wrong: `DATABASE NOT CONNECTED`,
`2 SERVICES DOWN`); a DB pill (DB ONLINE green, DB CONNECTING grey, DB OFFLINE flashing red); uptime as
DD:HH:MM:SS; SHUT DOWN (asks first); and the bell in the corner.

**The bell**: a red badge counts the open notices, 1 to 5, then `5+`.  Clicking it pulls out a tray over
whatever tab is open with the newest five, each a card with its level, where it came from, when, the text
and an ACK button, and ACK ALL at the top (asks first).  Each card fades after 30 seconds; the notice itself
stays open until it's ACKed.  Click the bell again to close the tray.

**The database lock**: anything but DB ONLINE blurs and greys everything under the header and the sidebar
tabs, and makes them unclickable (the keyboard too, with `inert`).  A card over it says "CONNECTING TO THE
DATABASE", or, flashing red, "DATABASE OFFLINE -- the game can't run right now", what Archivist says, and
when the page first saw it offline.  The browser tab's title turns to "DB OFFLINE".  For the first 10
seconds after Conductor starts it's "connecting", not offline, to give Archivist its first try.

**System**: the whole machine's CPU (every core averaged), memory in use against the total, and how many
processes there are and how many won't let Conductor read them.  Then every process, busiest first: name,
PID, CPU % (its share of the whole machine, the same measure as Conductor's big number), memory, threads.
Conductor's row is green with a bar down its left edge and a CONDUCTOR tag, and stays in the list whatever
the filter box says; a note says where it ranks.  Clicking a process shows its threads beside the list: OS
id, name, core % since the last look (worked out by the page from two answers a second apart), and CPU time.
It starts on Conductor, whose threads come from the status answer with our names on them.

**Conductor**: CPU is Conductor's share of the whole machine as the big number, and a chart of the last 60
seconds with one line per core, each showing how busy that core was (whatever was using it), always on a 0 to
100% scale.  All the lines are one colour; hovering one names it, and a readout under the chart lists every
core's percent now.  Per core is the whole machine's view on purpose: no OS says which core each of
Conductor's threads ran on.  Memory is Conductor's use as the big number, and a bar the width of the
machine's RAM: Conductor in green, everything else in use in blue, the empty track free.  Conductor's part is
a sliver at 5 MB out of 64 GB, so it always gets at least 3 pixels.  Then disk read and written per second
with totals; the machine (OS, process id, cores, last look); and Threads, with two tabs of its own: **In
use** (every OS thread, name, OS id, core %, CPU time; ones we didn't start are greyed and marked) and **Asked
for** (our threads, running or finished, when, and the file and line that started it).

**Services**: one row per service from `services.rs`: a dot (green healthy, yellow starting, flashing red
anything else), the name, the state, what it last said, since when, and how long since it last checked in
(the monitor and DiskMan).

**Storage**: Archivist -- connected or not, jobs waiting, jobs done split into read / written / other, slow
jobs and the slowest, the last slow job.  Beside it, DiskMan: files and bytes waiting to write (and reads
waiting), files and bytes held in memory, bytes written (whole writes, append batches, the slowest), bytes
read (from disk and from memory), failures (failing now, given up on), the last failure, and the big write
under way with a progress bar, and open streams.  The dot flashes red when DiskMan isn't running or a file
is failing.

**Notifications History**: every open notice, newest first: when, level (coloured), where from, what
happened, and an ACK button.  ACK ALL (asks first) and TEST NOTIFICATION at the top.  The page asks
`/Opus/notices` once a second, only while this tab is open.

**Log**: Scribe's terminal, the height of the window, coloured by priority, keeping the last 500 lines, and
staying at the bottom unless the admin has scrolled up.

If the server stops answering, the page covers itself with a note and stops asking.  After SHUT DOWN it says
Conductor is shutting down, and that the console counts down while DiskMan finishes.

## Services

Built on 2026-09-28, Zabbix style, the way the TLP at Jacob's work does it.  The list lives in
`conductor-tools/src/services.rs` (see `conductor-tools.md`).  What each one reports:

| Service        | Running when                                  | Trouble when                                   |
|----------------|-----------------------------------------------|------------------------------------------------|
| DiskMan        | Its thread is up; checks in every second      | A file is failing to write (see the log)      |
| Scribe         | It has a log file for today                   | DiskMan can't write the file: console only    |
| Constellations | The config loaded, or it wrote the defaults   | The file can't be read or written: defaults   |
| Fingerprinter  | The OS handed over random bytes at startup    | It wouldn't: nothing can get a UUID           |
| Security       | Its arena is allotted; checks in every second | Its thread wouldn't start                     |
| Archivist      | It's connected to Postgres                    | It can't connect, or lost the connection      |
| Monitor        | Its thread is looking once a second           | Never; stuck shows as gone quiet after 5 s    |
| Web admin      | It's listening                                | Never; if it can't listen, Conductor stops    |

Any of them shows stopped once its thread has ended, whatever it last said.

## What's open

- No login.  Anything running on this machine can reach it.  It matters more once there are buttons that
  change things (accounts, config).
- Accounts and config management, from the old launcher menu's plans, go here.  The settings shown and
  changed live are planned in TODO.md.
- The lock can't lift until Archivist reconnects, and Archivist only tries when a job comes in.  TODO.md.
