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
    ├── json.rs        status(snapshot, lines, log_file) -> String; a small Object builder, text() escaping
    └── page.html      the one page, baked in with include_str!
```

## Routes

| Method | Path                  | What it does                                                        |
|--------|-----------------------|---------------------------------------------------------------------|
| GET    | `/`                   | Sends the browser to `/Opus`                                        |
| GET    | `/Opus`               | The page                                                            |
| GET    | `/Opus/status?after=N`| The monitor's latest look and Scribe's lines after N, as JSON       |
| POST   | `/Opus/shutdown`      | Shuts Conductor down.  Needs the `X-Opus: shut-down` header         |

The JSON's shape is written out at the top of `json.rs`.  The page's script is the other half of it.

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
  which ends the console with it.  Ctrl-C still kills it outright, without the clean-up.
- If the web admin can't start (the port is taken), Conductor shuts straight back down with a capitals Error,
  since there would be no way to stop it cleanly.
- Another web page open in the same browser could try to reach 127.0.0.1 too.  Two checks stop it: the `Host`
  header must be `127.0.0.1:<port>` or `localhost:<port>`, and the shutdown needs an `X-Opus` header, which a
  browser won't let another site's page add without asking us first (and we never say yes).
- The page draws everything with `textContent`, never `innerHTML` with our data, so a log line with `<` in it
  shows as text.
- Mockup parts left out because there's nothing behind them yet: TPS, network streams, Argon2 load, "restart
  loop".  No made-up numbers on the page.

## The page

Header: the name, a status line (NOMINAL when Archivist is connected, DATABASE NOT CONNECTED otherwise),
uptime as DD:HH:MM:SS, and SHUT DOWN (asks first).  Then CPU and memory, each with a number and bars for the
last 30 seconds; disk read and written per second with totals; the machine (OS, process id, cores, last look).
Archivist: connected or not, jobs waiting, jobs done split into read / written / other, slow jobs and the
slowest, the last slow job.  Threads, with two tabs: **In use** (every OS thread, name, OS id, core %, CPU
time; ones we didn't start are greyed and marked) and **Asked for** (our threads, running or finished, when,
and the file and line that started it).  Scribe's terminal at the bottom, coloured by priority, keeping the
last 500 lines, and staying at the bottom unless the admin has scrolled up.

If the server stops answering, the page covers itself with a note and stops asking.

## What's open

- No login.  Anything running on this machine can reach it.  It matters more once there are buttons that
  change things (accounts, config).
- Accounts and config management, from the old launcher menu's plans, go here.
