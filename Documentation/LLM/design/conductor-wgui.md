<!--
File:       Opus/Documentation/LLM/design/conductor-wgui.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-wgui

A lib crate.  The web admin: a small web server on a thread of its own that shows how Conductor is doing,
starts and stops the server, and is the only way to shut Conductor down.  Named by Jacob.  Modeled on how the
TLP at Jacob's work is laid out: critical service status at a glance.  Dark slate panels, emerald accents, a
terminal box, in plain CSS.

## Skeleton

```
wgui/
├── Cargo.toml         depends on conductor-tools, conductor-monitor, conductor-networking (Connections and
│                        the lists) and conductor-accounts (the Accounts tab); no crate for the web server
└── src/
    ├── lib.rs         start(port) -> bool, has_ended(); the thread (serve(), handle()), route(), log_in(),
    │                    only_admin(), server_command(), tcp_kick(), networking_list(), unescape(),
    │                    settings_states(), settings_file(), settings_save(), settings_discard(),
    │                    host_is_ours(); struct Answer, enum Next, enum ListChange
    ├── accounts.rs    the Accounts tab's routes: list(), job(), create(), edit(), password(), delete(); the
    │                    checks they share (allowed(), changing()), account_named(), the body's fields()
    ├── characters.rs  the Characters tab's route: list()
    ├── http.rs        read_request() (head, then the body Content-Length says), parse_head(), respond();
    │                    struct Request with header() and query_value()
    ├── login.rs       the two accounts and the live logins: log_in(), role_of(), log_out(), cookie_line(),
    │                    clear_cookie_line(), fields(); enum Role, enum Login
    ├── json.rs        status(switch, role, snapshot, services, disk, networking, open_notices,
    │                    newest_notices, lines, log_file), login(role), notices(open), threads_of(pid,
    │                    threads), settings(files), problems(list), access(lists), changed(changed),
    │                    accounts(list), characters(list), account_job(number, outcome) -> String; struct FileState; a small
    │                    Object builder, text() escaping
    └── page.html      the one page, baked in with include_str!
```

## Routes

- `GET /` is a 303 to `/Opus`; `GET /Opus` (or `/Opus/`) is the page.
- `POST /Opus/login` (`X-Opus: login`): `name = ...` and `password = ...` lines in the body.  A good one sets
  the cookie; a wrong one is a 403 that doesn't say which half.  `POST /Opus/logout` (`X-Opus: login`)
  forgets the cookie's login.
- `GET /Opus/status?after=N`: the server's switch, who's logged in, the monitor's look, the services, DiskMan,
  networking's door, the notices, and the log after line N.  The page asks once a second.
- `GET /Opus/threads?pid=N`: one process's threads, for the System tab.
- `GET /Opus/notices`: every open notice, for Notifications History.  `POST /Opus/notices/ack?id=N`,
  `/Opus/notices/ack-all` and `/Opus/notices/test` (`X-Opus: ack`) clear one, clear all, raise a test notice.
- `POST /Opus/wwwhook/start`, `/stop`, `/restart` (`X-Opus: server`): the Server tab's buttons.  409 when it
  doesn't fit where the server is (a start while it isn't stopped, a stop or restart while it isn't running).
- `GET /Opus/settings`: every config file and setting (kind, comment, default, running value, waiting value).
- `POST /Opus/wwwhook/settings/save?file=<name>` (`X-Opus: settings`): SAVE, `key = value` lines in the body.
  400 with the complaints as JSON if a line is wrong, and nothing written; 500 the same way if the disk says
  no.  `POST /Opus/wwwhook/settings/discard?file=<name>` (`X-Opus: settings`) throws the `.wait4server` away.
- `POST /Opus/wwwhook/tcp/kick?id=N` (`X-Opus: tcp`): closes connection N at the door if it's open, or kicks
  the player its login became out of the world (`from_world` in the answer).  404 when nothing from the row
  is left, 409 if TCP isn't listening.
- `GET /Opus/networking`: both access lists; `running` is false with empty lists while the server is stopped.
- `POST /Opus/wwwhook/networking/addip?list=<whitelist|blacklist>&entry=<address or range>` and `/removeip`
  with the same query (`X-Opus: networking`).  400 with the reason in words for an entry that isn't one, 409
  while networking isn't running.
- `GET /Opus/Content/accounts` (or with the slash): every game account.  503 if the database doesn't answer
  in 5 seconds.  `GET /Opus/Content/accounts/job?id=N`: where job N on the account desk is (working, done or
  failed, and what happened); 404 for a job it doesn't know.
- `POST /Opus/wwwhook/accounts/create`: `username`, `first_name`, `last_name`, `email`, `password`,
  `password_again`.  400 with the complaints as JSON, each with its field's name in front; 202 with the
  account desk's job number.
- `POST /Opus/wwwhook/accounts/edit?name=<account>`: `first_name`, `last_name`, `email`, straight to the row.
  400 for a field (an email another account has), 404 for no such account.
- `POST /Opus/wwwhook/accounts/password?name=<account>`: `password`, `password_again`.  202 with the job number.
- `POST /Opus/wwwhook/accounts/delete?name=<account>`: deletes the row, then takes its player out of the world
  with Kicked, account terminated.  `{ deleted, kicked }`.
- `POST /Opus/shutdown` (`X-Opus: shut-down`): shuts Conductor down.

- `GET /Opus/Content/characters` (or with the slash): every player's character, for `admin` and `user` both.
  409 while the server isn't running, 503 if the database doesn't answer in 5 seconds.

The account routes are `admin` only, reads included, answer 409 while the server isn't running, and the four
that change something need `X-Opus: accounts`.  A known path asked with the wrong method is a `405`, anything
else a `404`.  A missing `id`, `pid`, `list` or `name` is a `400`; a `file` that isn't a config file is a
`404`.  Everything but `/`, the page and `/Opus/login` needs the login cookie (`401` without; the page shows
its login card on any 401), and every route that changes something needs `admin` (`403` to `user`).  The JSON
shapes are written out at the top of `json.rs`; the page's script is the other half of them.

## What we decided

- **Plain HTTP on 127.0.0.1 only**, port `wgui_port` in `conductor_globals.cfg` (9996).  Nothing leaves the
  machine, and HTTPS would cost a browser warning every time for a self-signed certificate.
- **No crate for the web server.**  The standard library's `TcpListener`, one request at a time, one per
  connection, with a 2 second limit to send the request.  One admin asking once a second doesn't need more.
- **Offline.**  The page pulls nothing from the internet and uses the fonts already on the machine.
- **The console takes no input.**  SHUT DOWN on the page stops the web admin's thread, main wakes up, stops the
  server if it's running, and the program ends, console and all.  Ctrl-C and closing the console window still
  kill it outright, without the clean-up (TODO.md).  If the web admin can't start (the port is taken),
  Conductor shuts straight back down with a capitals Error, since there'd be no way to stop it cleanly.
- **Other pages in the same browser are kept out.**  Any site could have the browser post to 127.0.0.1.  Three
  checks: the `Host` header must be `127.0.0.1:<port>` or `localhost:<port>`, the login cookie is
  `SameSite=Strict`, and every route that changes something (the login and LOG OUT included) needs an `X-Opus`
  header, which a browser won't let another site's page add without asking us first (and we never say yes).  A
  SHUT DOWN, a server button, a kick, a list change or an account change without it is a Warn, so it reaches
  the bell; the others are a plain 403.
- `textContent`, never `innerHTML` with our data, so a log line with `<` in it shows as text.
- **No made-up numbers.**  A panel waits until something is behind it (TPS, network streams and Argon2 load
  are still waiting).
- **The last tab picked is remembered by the browser** (`localStorage`, `opus.tab`), never the Server tab, and
  nothing else.  Nothing about the sections is remembered (asked, "no need").
- **The server is off until START SERVER.**  The Server tab greets you, and with the server stopped it, the Log
  and the Settings are all that open; no notifications either.  The server routes answer at once and the
  launcher does the work, so the web admin's one thread is never stuck behind an Archivist finishing a long
  query on the way down; the page sees the change through the status.  The state flips to starting or stopping
  in `server::ask()` itself, under its lock, so two clicks can't both get through.
- **SHUT DOWN lives on the Server tab, with STOP SERVER, and nowhere else.**
- **`/Opus/wwwhook/`** is Jacob's name for a path the page posts to that makes something happen (2026-09-29).
  SHUT DOWN, the ACKs, the login and LOG OUT kept their older paths.
- **While the database isn't connected, the page shows nothing but that.**  "The whole point is to draw
  attention to the user that the DB is offline and the game can't run right now."  Above the lock stay the
  header (the bell, its tray and the sections), the Server tab, the Log and the Settings, so the server can be
  stopped and the log read to see why (Jacob: "stay open"); they sit outside the blurred content block.  The
  page keeps asking underneath, so it unlocks the moment Archivist connects.
- **Notices**: every Warn and Error, and anything raised on purpose, waits on the bell until it's ACKed by hand
  (viewing doesn't clear it), in memory only.  The bell is hidden while the server isn't running; notices
  raised anyway (a config complaint at boot, say) are still there once it is, and in the log meanwhile.
- Another process's threads are asked for one process at a time, only while it's picked and the System tab is
  open.  Every process's threads every second would be thousands of rows nobody is looking at.
- **The login** (2026-09-29).  Two accounts, fixed: `user`, who opens every tab but Accounts and changes
  nothing, and `admin`, who does everything.  Their passwords are the two settings in `wgui.cfg` (hard, read
  at boot; defaults `user` and `admin`), kept as they are, not hashed: Security only runs between START SERVER
  and STOP SERVER, and the login has to work before START SERVER.  Fine while the page only listens on this
  machine, the same call as the Postgres password.  Jacob's calls: the web admin gets its own file; a card over
  the whole page until you're in; a login every time Conductor is started, and one that survives a reload.
  - A login is a random token from Fingerprinter's `new_token()` (straight from the OS, so it works with the
    server stopped) in an `HttpOnly; SameSite=Strict` cookie that lasts 30 days, kept in `login.rs`'s list in
    memory.  So shutting Conductor down forgets every login; a reload or closing the browser doesn't.
  - No idle timeout, ever (Jacob): the login is about roles, who may change the server, not security, and the
    page only listens on this machine.
  - A wrong login says "Wrong name or password" whichever half was wrong, and is an Info line, not a Warn: a
    typo on a local page isn't a notice.
  - For `user` every button that changes something is greyed, and Conductor turns the ask away anyway.
    `lockChanges()` sets each one from the role on every answer, both ways, since one page can see `user` and
    `admin` a LOG OUT apart; the Settings and list tabs are asked for again when the role changes.
- **The sections** (2026-09-30): Jacob's layout, five across the top with his names, and the side menu lists
  only the open section's tabs.  The notices are reached by the bell and by the LOGS section (his words).

## The page

**The login card** covers the whole page until Conductor says who's logged in: name, password, LOG IN, and
what went wrong.  It's up on every 401 (the first load, and after Conductor has been run again), and LOG OUT
puts it up with "Logged out.".  The status loop stops while it's up.

**Sections and the side menu**:

| Section | Tabs |
|---|---|
| CONTROL PANEL | Server, System, Conductor, Services, Storage |
| CONFIGURATION | Settings, Whitelist, Blacklist |
| LOGS | Log, Notifications History |
| ACCOUNT MANAGEMENT | Accounts |
| GAME MANAGEMENT | Connections, Characters |

Opening a tab opens its section.  A click on a section keeps the open tab if it's in that section, or opens its
first tab that isn't locked; if all are locked the side menu shows them greyed with a line saying why ("These
open once the server is running.", the database, networking's listeners, or "Only admin can open the
accounts.").  The top of the tab says section over name.  The Server tab shows whenever the server isn't
running; once it is, the page moves to the remembered tab, Conductor by default.  A locked tab is greyed:
with the server stopped that's all but Server, Log and Settings; running, the database lock decides,
Connections, Whitelist and Blacklist also need both of networking's listeners up, and Accounts needs `admin`.
The sections are never locked.  The Services tab and the CONTROL PANEL section get a flashing red dot when a
service is down, only while the server is running.  At the bottom of the side menu: who's logged in, whether
they can change things, and LOG OUT.

**Header**: the OP logo; the name; a status line (NOMINAL, `DATABASE NOT CONNECTED`, `2 SERVICES DOWN`, or
`SERVER STOPPED`); a pill (DB ONLINE green, DB CONNECTING grey, DB OFFLINE flashing red, or SERVER STOPPED /
STARTING / STOPPING grey); uptime as DD:HH:MM:SS, dashes while stopped; the bell in the corner.  The sections
are its second row.

**Server**: the state big (STOPPED plain, STARTING and STOPPING yellow, RUNNING green), the launcher's note and
since when; START SERVER (green), RESTART SERVER (green), STOP SERVER (red) and SHUT DOWN (red), all but START
asking first, each greyed when it doesn't fit the state.  Beside it a short services list from the Services
tab's list; a piece expected or stopped is grey while the server is down and red once it's up.  A 409 comes
back as an alert with Conductor's words.

**The bell**: a red badge counts the open notices, 1 to 5, then `5+`.  Clicking it pulls out a tray over the
tab with the newest five, each a card (level, source, when, text, ACK), and HISTORY (opens Notifications
History) and ACK ALL (asks first) at the top.  With nothing open the tray still opens, with HISTORY only.  A
card fades after 30 seconds; the notice stays open until ACKed.

**The database lock**: anything but DB ONLINE blurs and greys the data tabs and locks their buttons (the
keyboard too, with `inert`).  A card says "CONNECTING TO THE DATABASE", or, flashing red, "DATABASE OFFLINE --
the game can't run right now", what Archivist says, and when the page first saw it offline; the browser tab's
title turns to "DB OFFLINE".  For the first 10 seconds after every START SERVER it's "connecting", not offline,
to give Archivist its first try (by the monitor's uptime, which starts over with the server).

**System**: the machine's CPU (every core averaged), memory in use against the total, how many processes and
how many won't let Conductor read them.  Every process, busiest first: name, PID, CPU % (its share of the whole
machine, like Conductor's big number), memory, threads.  Conductor's row is green with a bar and a CONDUCTOR
tag, stays whatever the filter says, and a note says where it ranks.  Clicking a process shows its threads: OS
id, name, core % since the last look (from two answers a second apart), CPU time.  It starts on Conductor,
whose threads come from the status with our names on them.

**Conductor**: CPU as Conductor's share of the machine, and a 60 second chart with one line per core, always 0
to 100%, one colour, hover to name a line, a readout of every core now.  Per core is the machine's view on
purpose: no OS says which core each of Conductor's threads ran on.  Memory as Conductor's use, and a bar the
width of the machine's RAM (Conductor green, the rest in use blue, the track free; Conductor always gets at
least 3 pixels, being a sliver at 5 MB out of 64 GB).  Disk read and written per second with totals; the
machine (OS, process id, cores, last look); and Threads, two tabs: **In use** (every OS thread, name, OS id,
core %, CPU time; ones we didn't start greyed) and **Asked for** (our threads, running or finished, when, and
the file and line that started it).

**Services**: a row per service: a dot (green healthy, yellow starting, flashing red anything else), the name,
the state, what it last said, since when, and how long since it last checked in.

**Storage**: Archivist (connected or not, jobs waiting, jobs done as read / written / other, slow jobs, the
slowest and the last).  DiskMan: files and bytes waiting to write (and reads waiting), files and bytes held in
memory, bytes written (whole writes, append batches, the slowest), bytes read (disk and memory), failures
(failing now, given up on), the last failure, the big write under way with a bar, open streams.  The dot
flashes red when DiskMan isn't running or a file is failing.

**Connections** is the door and the world, and "will show TCP and UDP ordered by type on the page".  Two tiles
(where TCP and UDP listen; connections open, waiting, in Security's line and finished since START SERVER;
players in the world and tickets not yet used).  Then TCP, Jacob's ask: "listed by IP address (and DNS if
known), their position in the login queue (if not already logged in)", and no account information, "purely
tracked by IP address".  Every connection since START SERVER, newest first, in two views, Recent (the newest
five) and Historical (the whole run; his ask).  The ledger keeps up to 10,000, dropping the oldest finished
first.  A row: address, host (reverse DNS, or `--`), arrived (UTC) and seconds ago, where it is in words (a
queued one says how many are ahead, one in Security's line how many jobs and about how long), and a three-dot
menu (his ask: "in style like a : colon"; greyed for `user`): KICK (asks first; greyed with nothing left to
kick), ADD <address> TO WHITELIST, ADD <address> TO BLACKLIST (asks first, since it's a ban while the
blacklist is on), on finished rows too.  The menu lives outside the table, which is drawn again every second.
A finished row is greyed and says how it ended; a logged-in one is green while its player is in the world, and
reads "LINKDEAD:" and why once they've left (a second login, Goodbye, the timeout, a ban, or a ticket never
used).  Under it, UDP, where the account is the point: every player in the world, newest first: address,
account (green), connected (UTC), playing for (DD:HH:MM:SS), quiet for (yellow from 5 seconds).  Connections,
Whitelist and Blacklist are locked until both listeners are up (his words: after the TCP listener and the UDP
listener are online); if one goes down while the tab is open, the page steps off it.  The ledger, the DNS
thread and the kick are networking's; see `conductor-networking.md`.

**Whitelist** and **Blacklist**: an ACCESS LIST tile (what `access_list` in `networking.cfg` says, green when
it's this list, yellow when it's the other, plain when off, with what that means for this list) and the count.
Then the list: an ADD field in the head (Enter adds too), what the last change did (an entry already there, a
list that isn't switched on, how many connections and players a ban dropped), a bad entry's reason in red in
Conductor's words, and the entries with REMOVE (asks first).  A change takes at once.  A whitelist removal
with the whitelist on is a ban too (his rule), and REMOVE says so.  The routes are `addip` and `removeip`, his
names ("add" and "remove" were too generic).  Greyed for `user`, and locked while the server is stopped (edit
the files by hand then).

**Accounts** (2026-09-29): create, list, delete, change an account's fields and password.  `admin` only
(`user` can't see the list), and only while the server is running (Jacob's call: Security and Archivist are
server pieces).  The list: name (a link to its card; a card, not a "finger" command), owner, email, last login
(`never`) and ONLINE (green IN THE WORLD, from the status each second; the list itself is asked for when the
tab opens and after a change), with NEW ACCOUNT in its head.  A card: the name, an IN THE WORLD tag, the UUID,
made and last login, then THE OWNER (first name, last name, email, SAVE), A NEW PASSWORD (twice, CHANGE
PASSWORD) and DELETE ACCOUNT (asks first, saying so when the player is in the world).  The new account card:
name, first name, last name, email, the password twice, CREATE.  A password is typed twice, every time; the
username never changes.  A field's complaint shows in red under it; the outcome beside the button, "Working:
waiting its turn in Security's line." while the account desk has it.  Only the tag redraws each second, never
the card under somebody's typing.  Every change goes straight to the row (an account is never held in memory;
see `conductor-accounts.md`), so editing one whose player is online is safe.  Deleting one whose player is
online takes them out with Kicked, reason 5, and the client says ACCOUNT TERMINATED (his words); protocol
version 4.  The list, an edit and a delete are waited on (5 seconds at most); a new account and a new password
need a hash, so they go to the account desk and the page asks after the job every half second, and the web
admin's one thread never waits in Security's line.

**Characters** (2026-09-30), under GAME MANAGEMENT: every player's character, look only.  Jacob: "I just want
it to show their name, their X,Y,Z, and which account they're connected to", and the UUID.  So the columns are
name, UUID, x, y, z (as of the last save, to two places) and account, by name.  `admin` and `user` both see it
(Jacob's pick, though it shows account names, which the Accounts tab keeps from `user`).  Locked like the
other data tabs: the server running and the database connected.  The list is asked for when the tab opens and
on REFRESH, never once a second, since it's a database read; an ask that fails is asked again by the status
loop once the database is there.  Editing a character or an NPC from here is its own conversation (TODO.md).

**Notifications History**: every open notice, newest first (when, level, source, text, ACK), with ACK ALL
(asks first) and TEST NOTIFICATION.  The page asks `/Opus/notices` once a second, only while it's open.

**Log**: Scribe's terminal, the height of the window, coloured by priority, the last 500 lines, staying at the
bottom unless the admin has scrolled up.  Always open.

**Settings** (2026-09-29), the config editor's web admin half, always open, so a setting can be changed while
the server is stopped or the database is offline.  One card per config file, two to a row: the name, what it's
for and which reboot it needs, a field per setting with the key, its kind in words (a port 1 to 65535, a
number with its range, a folder, text, a secret) and its comment, then SAVE and DISCARD (asks first).  It's
drawn from `/Opus/settings` when the tab opens, after every SAVE or DISCARD, and whenever the server's state
(or when it got there) changes, never once a second, so nothing redraws under somebody's typing.  SAVE sends
every field as `key = value` lines in the file's order, so Constellations' "line 3" complaint lands beside the
third field; one that isn't about a line (the disk saying no) shows under the card.  After a save the card has
a yellow WAITING ON A HARD (or SOFT) REBOOT tag, each changed field says what's running under it, and nothing
changes until that reboot.  A file not loaded this run (`postgres.cfg` before the first START SERVER) shows
the file's values and says so.  A password shows as it is to `admin`, and as nothing to `user`, who
couldn't change it and shouldn't be able to read the admin's off the page (2026-10-02; before that it
showed to both).  Read only for `user`, with a line saying so.  Past four files (`PICKER_FROM`, Jacob's number) a picker shows one card at a time, marking a file
with a change waiting.

If Conductor stops answering, the page covers itself with a note and stops asking.  After SHUT DOWN it says
Conductor is shutting down, the server first if it's running, and that the console counts down while DiskMan
finishes.

## Services

Zabbix style, the way the TLP at Jacob's work does it.  The list is `EXPECTED` in `tools/src/services.rs` (see
`conductor-tools.md`).  What each one reports:

| Service        | Running when                                      | Trouble when                                    |
|----------------|---------------------------------------------------|-------------------------------------------------|
| DiskMan        | Its thread is up; checks in every second          | A file is failing to write (see the log)        |
| Scribe         | It has a log file for today                       | DiskMan can't write the file: console only      |
| Constellations | The config loaded, or it wrote the defaults       | The file can't be read or written: defaults     |
| Fingerprinter  | The OS gave it random bytes on START SERVER       | It wouldn't: nothing can get a UUID             |
| Security       | Its arena is allotted; checks in every second     | Its thread wouldn't start                       |
| Archivist      | It's connected to Postgres                        | It can't connect, or lost the connection        |
| Network (TCP)  | The acceptor is listening for logins              | The TLS files are missing, or it can't listen   |
| Network (UDP)  | Its thread is listening; checks in every second   | It can't listen (TCP comes back down too)       |
| Account desk   | Its thread is up, waiting for account jobs        | Never; its thread couldn't start shows stopped  |
| Monitor        | Its thread is looking once a second               | Never; stuck shows as gone quiet after 5 s      |
| Lua            | The scripts ran, none with an error               | A script had an error (the log says which)      |
| GameWorld      | The world is read (or made); hands out chunks     | It can't read the world: no ground this run     |
| GameClock      | It's beating; checks in four times a second       | Never; stuck shows as gone quiet after 5 s      |
| Web admin      | It's listening                                    | Never; if it can't listen, Conductor stops      |

Any of them shows stopped once its thread has ended, whatever it last said.  Everything from Fingerprinter to
the GameClock is the server: expected until the first START SERVER, stopped after a STOP SERVER.

## What's open

- The passwords in `wgui.cfg` are plain text.  Hashing them through Security means Security up from boot, or a
  hash on the caller's thread; a decision for another day, in TODO.md.
- Two accounts and no more.  More accounts, and saved page layouts per account, are an idea in TODO.md.
- `wgui_port` is moving from `conductor_globals.cfg` into `wgui.cfg` (Jacob, 2026-09-29).  In TODO.md.
- HTTPS.  rustls is in the build for the game's login now, so it waits only on wanting it and on the browser's
  warning for a self-signed certificate.  In TODO.md.
- The lock can't lift until Archivist reconnects, and Archivist only tries when a job comes in.  TODO.md.  A
  STOP SERVER and a START SERVER is the way round it today.
