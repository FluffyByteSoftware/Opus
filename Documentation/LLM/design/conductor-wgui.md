<!--
File:       Opus/Documentation/LLM/design/conductor-wgui.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-wgui

A lib crate.  The web admin: a small web server on a thread of its own that shows how Conductor is doing,
starts and stops the server, and is the only way to shut Conductor down.  Named by Jacob.  Modeled on how the TLP at Jacob's work is laid out:
critical service status at a glance.  The look came from a mockup Gemini drew (dark slate panels, emerald
accents, a terminal box), redone in plain CSS.

## Skeleton

```
conductor-wgui/
├── Cargo.toml         depends on conductor-tools, conductor-monitor and conductor-networking (the TCP tab)
└── src/
    ├── lib.rs         start(port) -> bool, has_ended(); the thread, route(), log_in(), only_admin(),
    │                    server_command(), settings_states(), settings_save(), settings_discard(), tcp_kick(),
    │                    networking_list(), unescape(), host_is_ours()
    ├── http.rs        read_request() (head, then the body Content-Length says), parse_head(), respond();
    │                    struct Request
    ├── login.rs       the two accounts and the live logins: log_in(), role_of(), log_out(), the cookie lines,
    │                    fields(); enum Role, enum Login
    ├── json.rs        status(switch, role, snapshot, services, disk, networking, open_notices, newest_notices, lines, log_file),
    │                    login(role), notices(open), threads_of(pid, threads), settings(files), problems(list),
    │                    access(lists), changed() -> String; a small Object builder, text() escaping
    └── page.html      the one page, baked in with include_str!
```

## Routes

| Method | Path                   | What it does                                                        |
|--------|------------------------|---------------------------------------------------------------------|
| GET    | `/`                    | Sends the browser to `/Opus`                                        |
| GET    | `/Opus`                | The page                                                            |
| POST   | `/Opus/login`          | `name = ...` and `password = ...` lines in the body.  Needs `X-Opus: login`.  A good one sets the cookie; a wrong one is a 403 that doesn't say which half |
| POST   | `/Opus/logout`         | Forgets the cookie's login.  Needs `X-Opus: login`                  |
| GET    | `/Opus/status?after=N` | The server's switch, who's logged in, the monitor's look, the services, DiskMan, networking's door, the notices, the log |
| GET    | `/Opus/threads?pid=N`  | One process's threads, for the System tab.  Reads only.             |
| GET    | `/Opus/notices`        | Every open notice, for the Notifications History tab.  Reads only.  |
| POST   | `/Opus/notices/ack?id=N` | Clears one notice.  Needs `X-Opus: ack`                           |
| POST   | `/Opus/notices/ack-all`| Clears every notice.  Needs `X-Opus: ack`                           |
| POST   | `/Opus/notices/test`   | Raises a test notice.  Needs `X-Opus: ack`                          |
| POST   | `/Opus/wwwhook/start`   | Asks the launcher to start the server.  Needs `X-Opus: server`.  409 if it isn't stopped |
| POST   | `/Opus/wwwhook/stop`    | Asks the launcher to stop it.  Needs `X-Opus: server`.  409 if it isn't running |
| POST   | `/Opus/wwwhook/restart` | Stop, then start.  Needs `X-Opus: server`.  409 if it isn't running |
| GET    | `/Opus/settings`       | Every config file and setting: kind, comment, default, running value, waiting value.  Reads only |
| POST   | `/Opus/wwwhook/settings/save?file=<name>` | SAVE on the Settings tab.  `key = value` lines in the body.  Needs `X-Opus: settings`.  400 with the complaints as JSON if a line is wrong, and nothing written; 500 the same way if the disk says no |
| POST   | `/Opus/wwwhook/settings/discard?file=<name>` | DISCARD: throws the file's `.wait4server` away.  Needs `X-Opus: settings` |
| POST   | `/Opus/wwwhook/tcp/kick?id=N` | KICK on the Connections tab: closes connection N at the door.  Needs `X-Opus: tcp`.  404 if it isn't open, 409 if TCP isn't listening |
| GET    | `/Opus/networking`     | Both access lists, for the Whitelist and Blacklist tabs.  Reads only; `running` is false with empty lists while the server is stopped |
| POST   | `/Opus/wwwhook/networking/addip?list=<whitelist or blacklist>&entry=<address or range>` | ADD on a list tab, or the Connections tab's menu.  Takes at once; a blacklisting while the blacklist is on is a ban.  Needs `X-Opus: networking`.  400 with the reason in words for an entry that isn't one, 409 while networking isn't running |
| POST   | `/Opus/wwwhook/networking/removeip?list=...&entry=...` | REMOVE on a list tab, the same way; off the whitelist while the whitelist is on, a ban too |
| POST   | `/Opus/shutdown`       | Shuts Conductor down.  Needs the `X-Opus: shut-down` header         |

Everything but `/`, the page and `/Opus/login` needs the login cookie, and answers `401 Unauthorized`
without it; the page shows its login card on any 401.  Every route that changes something (the ACKs, the
test notice, the server buttons, the settings, the kick, the lists, SHUT DOWN) needs `admin`, and answers `403 Forbidden` to
`user`.  The JSON shapes are written out at the top of `json.rs`.  The page's script is the other half of
them.

## What we decided

- **Plain HTTP on 127.0.0.1 only**, port `wgui_port` in `conductor_globals.cfg` (9996).  Jacob first wanted
  HTTPS with a self-signed certificate, then picked plain HTTP: nothing leaves the machine, and HTTPS would
  cost a crate and a browser warning every time.  When Security brings in TLS for the game, this can use it.
- No crate for the web server.  It's the standard library's `TcpListener`, one request at a time, one per
  connection, with a 2 second limit to send the request.  One admin asking once a second doesn't need more.
- **Offline.**  The page pulls nothing from the internet.  Gemini's mockup used Tailwind and Google Fonts from
  the web; the page keeps the colours and uses the fonts already on the machine.
- **The console takes no input.**  The launcher boots the program and waits on the web admin.  Shut Down on
  the page stops the web admin's thread, main wakes up, stops the server if it's running, and the program
  ends, which ends the console with it.  Ctrl-C still kills it outright, without the clean-up.  Closing the console
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
- **The Control Panel** (2026-09-29).  Jacob: the page Conductor greets you with is a control panel, with
  only the log reachable in the tabs, no notifications while nothing is started, and START / RESTART / STOP
  SERVER or SHUTDOWN; once the server is running, the pages as they were.  So the server (Fingerprinter,
  Security, Archivist, the monitor, and whatever comes later) is off until START SERVER, and the tab is
  first in the sidebar.  This is the "Manage System" screen the Security session's hand-off said was next;
  Jacob called it the control panel when it opened, so that's its name.  The three server routes answer at once and the launcher does the work, so the web admin's one
  thread is never stuck behind an Archivist that's finishing a long query on the way down; the page sees
  the state change through the status.  The state flips to starting or stopping in `server::ask()` itself,
  under its lock, so two clicks can't both get through.  The routes live under `/Opus/wwwhook/`, Jacob's
  name for a path the page posts to that makes something happen (2026-09-29).  Only the three server
  routes are there; SHUT DOWN and the ACKs kept their old paths.
- **The Control Panel and the Log stay above the database lock**, like the header and the bell, so the
  server can be stopped while the database is offline and the log read to see why.  Jacob's call, the same
  day: "stay open".  Both sections sit outside the blurred content block for that.
- **SHUT DOWN lives on the Control Panel, with STOP SERVER, and nowhere else.**  The header had its own
  from the first web admin session; Jacob had it go once the Control Panel had one.
- **The bell is hidden while the server isn't running.**  Notices raised anyway (a config complaint at boot,
  say) are still there once it is, and in the log meanwhile.
- **The login** (2026-09-29).  Two accounts, fixed: `user`, who can open every tab and change nothing, and
  `admin`, who can do everything.  Their passwords are the two settings in `wgui.cfg`, Constellations'
  and hard, read at boot; the defaults are `user` and `admin`.  Jacob's calls: the web admin gets its own
  file rather than the accounts going in `conductor_globals.cfg` or a file of their own; `user` is read
  only and `admin` edits config files; a card over the whole page until you're in; a login every time
  Conductor is started, and one that survives reloading the page.  The passwords are kept as they are,
  not hashed: Security is a server piece and only runs between START SERVER and STOP SERVER, and the
  login has to work before START SERVER.  Fine while the page only listens on this machine, the same call
  as the Postgres password.  A login is a random token from Fingerprinter's `new_token()` (it goes
  straight to the OS, so it works with the server stopped) in an `HttpOnly; SameSite=Strict` cookie, kept
  in `login.rs`'s list in memory, so Conductor shutting down forgets every login and a page reload
  doesn't.  The cookie lasts 30 days on the browser's side, so closing the browser doesn't log you out
  either while Conductor runs.  No idle timeout, ever (Jacob, at the wrap-up): the login is about roles,
  who may change the server, not security, and the page only listens on this machine.  A wrong login says "Wrong name or password" whichever
  half was wrong, and is an Info line in the log, not a Warn: a typo on a local page isn't a notice.  LOG
  OUT sits at the bottom of the sidebar with who's logged in; the header still has no buttons but the
  bell.  For `user`, every button that changes something is greyed, and Conductor turns the ask away
  anyway.
- **The Settings tab** (2026-09-29), the config editor's web admin half.  Eighth in the sidebar, after
  Log (Jacob's pick), and always clickable like the Control Panel and the Log: it sits outside the
  blurred content, so a setting can be changed while the server is stopped or the database is offline.
  One card per file from Constellations' table, drawn from `/Opus/settings` when the tab opens and after
  every SAVE or DISCARD, never once a second, so nothing redraws under somebody's typing.  A card says
  what the file is for and which reboot it needs, in plain words, then a field per setting with the
  key, its kind in words (a port 1 to 65535, a number with its range, a folder, text, a secret), and the
  comment under it.  A file that hasn't been loaded this run (`postgres.cfg` before the first START
  SERVER) shows the values in the file and says so.  The password shows as it is (Jacob's call).  SAVE
  sends every field as `key = value` lines in the file's order, so Constellations' "line 3" complaint
  lands beside the third field, and DISCARD (asks first) throws the waiting file away.  After a save the
  card carries a yellow WAITING ON A HARD (or SOFT) REBOOT tag, each changed field says what's running
  under it, and nothing changes until that reboot.  The page needed a body reader in `http.rs` for it,
  which the login uses too.
- **The TCP tab** (2026-09-29), Jacob's pick for the session after networking: "the connections that have
  reached out to our listener in the last 5 minutes listed by IP address (and DNS if known), their
  position in the login queue (if not already logged in)", and no account information, "purely tracked
  by IP address".  Then, the same session, a KICK on each row for `admin`.  Sixth in the sidebar, after
  Storage, and locked until the server is running, the database is connected and both of networking's
  listeners are up (his words: after the TCP listener and the UDP listener are online), so it sits
  under the database lock like the other data tabs, with one lock more.  Drawn once a second from the
  `networking` part of the status: two tiles (where TCP and UDP listen, and how many connections are
  open, waiting, in Security's line and finished lately), then the table, newest first, with the
  address, its reverse DNS name when one has come back, when it arrived and how long ago, where it is in
  words (a queued one says how many are ahead of it; one in Security's line says how many jobs are ahead
  and about how long; a finished one says how it ended, greyed, green if it logged in), and KICK on
  every open one.  The ledger behind it, the DNS thread and the kick are networking's; see
  `conductor-networking.md`.
- **The Network Admin subsection** (2026-09-29, the session after): Jacob's layout, "a subsection on the
  left for Network Admin and underneath it: Connections (which will show TCP and UDP ordered by type on
  the page) and then WHITELIST and BLACKLIST".  So the TCP tab became **Connections**, under a small
  NETWORK ADMIN heading with a rule above it, below Notifications History and above the Log (his
  placing), with the two list tabs after it, and all three take the TCP tab's lock (both listeners up
  on top of the database).  Connections is the TCP table as it was,
  then a UDP table of every player in the world (address, account, connected when, playing for, quiet
  for; the account is the point there, unlike the door).  KICK moved into a three-dot menu on each row
  (his ask: "in style like a : colon"), with ADD TO WHITELIST and ADD TO BLACKLIST under it, on finished
  rows too; the menu lives outside the table, since the table is drawn again every second.  The
  The TCP table got two views the same session, Recent (the newest five) and Historical (every
  connection since START SERVER), so the ledger stopped forgetting after five minutes and the setting
  for that went.  The
  Whitelist and Blacklist tabs are one card each: a tile saying whether that list is the one the door
  checks (`access_list` in networking.cfg, on the Settings tab) and what it means if not, the count, the
  entries with REMOVE on each, and an ADD field in the head.  A change takes at once, and the tab says
  what it did (an entry that was there already, a list that isn't switched on, how many connections
  and players a ban dropped; a whitelist removal with the whitelist on is a ban too, his rule, and
  REMOVE there says so before asking); a bad entry's reason shows in red, in Conductor's words.  The two
  routes are `addip` and `removeip`, his names ("add" and "remove" were too generic), under
  `/Opus/wwwhook/networking/`.  The lists can't be changed while the server is stopped, by his rule:
  the tabs are locked then, and the files can be edited by hand.

## The page

**The login card**: covers the whole page until Conductor says who's logged in.  Name, password, LOG IN,
and what went wrong under them.  It's up on every 401: the first load, after LOG OUT, and after Conductor
has been run again.  The status loop stops while it's up and a login starts it again.

**Sidebar**: the OP logo, and eleven tabs under it: Control Panel, System, Conductor, Services, Storage,
Notifications History, then a rule and a NETWORK ADMIN heading with Connections, Whitelist and Blacklist
indented under it, then Log, Settings.  The Control Panel shows whenever the server isn't running; once it
is, the page moves to the tab the browser remembers, Conductor by default.  A locked tab is greyed and
can't be clicked: with the server stopped that's everything but the Control Panel, the Log and the
Settings, and with it running the database lock decides, and the three Network Admin tabs need both network
listeners up on top of that.  The Services tab gets a flashing red dot when a
service is down, only while the server is running (its pieces being down is the normal state before
that).  At the bottom: who's logged in, whether they can change things, and LOG OUT.

**Header, on every tab**: the name; a status line (NOMINAL, or what's wrong: `DATABASE NOT CONNECTED`,
`2 SERVICES DOWN`, or `SERVER STOPPED` while it is); a pill (DB ONLINE green, DB CONNECTING grey, DB OFFLINE
flashing red, or SERVER STOPPED / STARTING / STOPPING grey); uptime as DD:HH:MM:SS, dashes while the server
is stopped; and the bell in the corner, hidden while the server isn't running.

**Control Panel**: the server's state big (STOPPED plain, STARTING and STOPPING yellow, RUNNING green), the
launcher's note under it and since when; then START SERVER (green), RESTART SERVER (green, asks first),
STOP SERVER (red, asks first) and SHUT DOWN (red, asks first), each greyed when it doesn't fit the state.
Beside it a short services list (dot, name, state, what it says) from the same list as the Services tab; a
piece that's expected or stopped is grey while the server is down and red once it's up.  The buttons post
to the server routes and the next status answer sets them right again; a 409 comes back as an alert with
Conductor's words.

**The bell**: a red badge counts the open notices, 1 to 5, then `5+`.  Clicking it pulls out a tray over
whatever tab is open with the newest five, each a card with its level, where it came from, when, the text
and an ACK button, and ACK ALL at the top (asks first).  Each card fades after 30 seconds; the notice itself
stays open until it's ACKed.  Click the bell again to close the tray.

**The database lock**: anything but DB ONLINE blurs and greys the data tabs under the header and locks
their sidebar buttons (the keyboard too, with `inert`); the Control Panel, the Log and the Settings stay
clickable.  A card over it says "CONNECTING TO THE
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

**Connections**: two tiles, then TCP, every connection that reached the login door since START SERVER,
newest first, in two views picked by the small tabs in the panel's head (Jacob's ask, 2026-09-29):
Recent, the newest five, and Historical, the whole run.  Each row: address, host (reverse DNS, or `--`),
arrived (UTC) and seconds ago, where it is in words, and a three-dot button (greyed for `user`) that
opens a small menu by the row: KICK on an open
one (asks first), ADD <address> TO WHITELIST, ADD <address> TO BLACKLIST (asks first, since it's a ban
while the blacklist is on).  What the menu did shows in the panel's head.  A finished row is greyed and
says how it ended; a logged-in one is green.  No account name on the TCP table.  Under it, UDP: every
player in the world, newest first: address, account (green), connected (UTC), playing for (as
DD:HH:MM:SS), quiet for (yellow from 5 seconds).  Locked while either listener is down, and the page
steps off it to the default tab if that happens while it's open.

**Whitelist** and **Blacklist**: one each.  Two tiles: ACCESS LIST (what `access_list` says, green when it's
this list, yellow when it's the other, plain when off, with a line saying what that means for this
list) and the count.  Then the card: the list's name, an ADD field and button in the head (Enter adds
too), a line saying what the last change did, a red line for a bad entry in Conductor's words, and the
entries with REMOVE on each (asks first).  "Nothing on it." when empty.  Greyed for `user`.  Locked with
the Connections tab.

**Notifications History**: every open notice, newest first: when, level (coloured), where from, what
happened, and an ACK button.  ACK ALL (asks first) and TEST NOTIFICATION at the top.  The page asks
`/Opus/notices` once a second, only while this tab is open.

**Log**: Scribe's terminal, the height of the window, coloured by priority, keeping the last 500 lines, and
staying at the bottom unless the admin has scrolled up.  Always open: with the server stopped and under
the database lock both, so the admin can read why something went wrong.

**Settings**: one card per config file, two to a row: the file's name (with a yellow WAITING tag when a
change is saved and not yet applied), what it's for, the reboot it needs, a field per setting with its
kind and comment, then SAVE and DISCARD.  A complaint from a failed save shows under the field it's about;
one that isn't about a line (the disk saying no) shows under the card.  Read only for `user`: the fields
and buttons are greyed and a line at the top says so.  Always open, like the Log.  Past four files
(`PICKER_FROM`, Jacob's number) a picker at the top shows one card at a time, marking a file with a
change waiting; up to four, every card is on the page at once.

If Conductor stops answering, the page covers itself with a note and stops asking.  After SHUT DOWN it says
Conductor is shutting down, that the server stops first if it's running, and that the console counts down
while DiskMan finishes.

## Services

Built on 2026-09-28, Zabbix style, the way the TLP at Jacob's work does it.  The list lives in
`conductor-tools/src/services.rs` (see `conductor-tools.md`).  What each one reports:

| Service        | Running when                                  | Trouble when                                   |
|----------------|-----------------------------------------------|------------------------------------------------|
| DiskMan        | Its thread is up; checks in every second      | A file is failing to write (see the log)      |
| Scribe         | It has a log file for today                   | DiskMan can't write the file: console only    |
| Constellations | The config loaded, or it wrote the defaults   | The file can't be read or written: defaults   |
| Fingerprinter  | The OS gave it random bytes on START SERVER   | It wouldn't: nothing can get a UUID           |
| Security       | Its arena is allotted; checks in every second | Its thread wouldn't start                     |
| Archivist      | It's connected to Postgres                    | It can't connect, or lost the connection      |
| Network (TCP)  | The acceptor is listening for logins          | The TLS files are missing, or it can't listen  |
| Network (UDP)  | Its thread is listening; checks in every second | It can't listen (TCP comes back down too)     |
| Monitor        | Its thread is looking once a second           | Never; stuck shows as gone quiet after 5 s    |
| Web admin      | It's listening                                | Never; if it can't listen, Conductor stops    |

Any of them shows stopped once its thread has ended, whatever it last said.  Fingerprinter, Security,
Archivist and the monitor are the server: expected until the first START SERVER, stopped after a STOP SERVER.

## What's open

- The passwords in `wgui.cfg` are plain text.  Hashing them through Security means Security up from boot,
  or a hash on the caller's thread; a decision for another day, in TODO.md.
- Two accounts and no more.  A list of named accounts is an idea in TODO.md.
- `wgui_port` is moving from `conductor_globals.cfg` into `wgui.cfg` (Jacob, 2026-09-29).  In TODO.md.
- The Connections tab's UDP list has no character column yet: there are no characters.
- Game account management (make, delete, list, finger, change password), from the old launcher menu's
  plans, goes here once there are game accounts.
- The lock can't lift until Archivist reconnects, and Archivist only tries when a job comes in.  TODO.md.
  A STOP SERVER and a START SERVER is the way round it today.
