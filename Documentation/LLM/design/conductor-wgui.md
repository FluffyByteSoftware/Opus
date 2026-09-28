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
    ├── json.rs        status(snapshot, services, lines, log_file), threads_of(pid, threads) -> String;
    │                    a small Object builder, text() escaping
    └── page.html      the one page, baked in with include_str!
```

## Routes

| Method | Path                   | What it does                                                        |
|--------|------------------------|---------------------------------------------------------------------|
| GET    | `/`                    | Sends the browser to `/Opus`                                        |
| GET    | `/Opus`                | The page                                                            |
| GET    | `/Opus/status?after=N` | The monitor's latest look, the services, and Scribe's lines after N |
| GET    | `/Opus/threads?pid=N`  | One process's threads, for the System tab.  Reads only.             |
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
- Another process's threads are asked for one process at a time, only while it's picked and the System tab is
  open.  Every process's threads every second would be thousands of rows nobody is looking at.

## The page

**Sidebar**: the OP logo, and five tabs under it: System, Conductor, Services, Storage, Log.  Conductor opens
first, unless the browser remembers another.  The Services tab gets a flashing red dot when a service is down.

**Header, on every tab**: the name; a status line (NOMINAL, or what's wrong: `DATABASE NOT CONNECTED`,
`2 SERVICES DOWN`); a DB pill (DB ONLINE green, DB CONNECTING grey, DB OFFLINE flashing red); uptime as
DD:HH:MM:SS; and SHUT DOWN (asks first).

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
(the monitor only).  A grey "Disk manager -- not built yet" row at the bottom, which is never red.

**Storage**: Archivist -- connected or not, jobs waiting, jobs done split into read / written / other, slow
jobs and the slowest, the last slow job.  Beside it, a "not built yet" panel for the disk manager.

**Log**: Scribe's terminal, the height of the window, coloured by priority, keeping the last 500 lines, and
staying at the bottom unless the admin has scrolled up.

If the server stops answering, the page covers itself with a note and stops asking.

## Services

Built on 2026-09-28, Zabbix style, the way the TLP at Jacob's work does it.  The list lives in
`conductor-tools/src/services.rs` (see `conductor-tools.md`).  What each one reports:

| Service        | Running when                                  | Trouble when                                   |
|----------------|-----------------------------------------------|------------------------------------------------|
| Scribe         | It has today's log file open                  | The file won't open or write: console only    |
| Constellations | The config loaded, or it wrote the defaults   | The file can't be read or written: defaults   |
| Archivist      | It's connected to Postgres                    | It can't connect, or lost the connection      |
| Monitor        | Its thread is looking once a second           | Never; stuck shows as gone quiet after 5 s    |
| Web admin      | It's listening                                | Never; if it can't listen, Conductor stops    |
| Disk manager   | Doesn't exist yet: grey on the page           |                                               |

Any of them shows stopped once its thread has ended, whatever it last said.

## What's open

- No login.  Anything running on this machine can reach it.  It matters more once there are buttons that
  change things (accounts, config).
- Accounts and config management, from the old launcher menu's plans, go here.  The settings shown and
  changed live are planned in TODO.md.
- The lock can't lift until Archivist reconnects, and Archivist only tries when a job comes in.  TODO.md.
