<!--
File:       Opus/Documentation/LLM/TEST_CHECKLIST.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Test Checklist

The rolling list of what to check on `testing`.  Every session that changes what Conductor (or, later,
Ensemble) does adds its checks under a heading with the date and the feature, and the reply that pushes
`unstable` onto `testing` points here.  Once a check is done it's struck through with the date it passed
(or a note on what went wrong), never deleted, so the list reads as a history of what was looked at.
Jacob's ask, 2026-09-29: once game features come, this is the reminder of what changed and what to look
at in game.

Every command is one line, from `Conductor/dev`.

## The test run, by subsystem (2026-09-29)

Every open check from the sections below, gathered into one pass on `testing` and put in order by the
piece it tests, so a sitting can take one subsystem and stop.  Jacob's ask, 2026-09-29.  The sections
below stay as the history of what each session asked for; a check is struck here when it passes.  Where
an old check no longer matched the build it was rewritten here, and says so.

The account is `throwaway_01` / `Throwaway 1!`.  The client is `conductor-networking/test_client.py`
(inside `Conductor/dev`, so the path works from there).  A browser console is F12, Console tab.

### 0. The build

- [ ] `cargo build` clean, no warnings; `cargo test` passes, 199.  Only `test_client.py` changed since
      the last build (`--pause-before-login`), so the count doesn't move.

### 1. The web admin before START SERVER

- [ ] The sidebar: Control Panel, System, Conductor, Services, Storage, Notifications History, then a
      rule and a NETWORK ADMIN heading with Connections, Whitelist and Blacklist indented under it, all
      three greyed, then Log and Settings.
- [ ] The Settings tab, `networking.cfg`'s card: `access_list` (off), `whitelist_file`
      (cfg/whitelist.cfg), `blacklist_file` (cfg/blacklist.cfg); no `connections_remember_seconds`.  If
      your `networking.cfg` still has that line, START SERVER warns about it once; take the line out.

### 2. Constellations and DiskMan: hand edits

- [ ] A soft file: START SERVER, STOP SERVER, change `slow_job_ms` in `Content/cfg/postgres.cfg` by hand
      (Conductor still running), START SERVER.  The Settings tab shows the new value as running.  Put it
      back after.
- [ ] A list file: server stopped, add `10.0.0.1` to `Content/cfg/blacklist.cfg` by hand, START SERVER.
      The Blacklist tab shows it, and the log has the Debug line "Read 1 entry from .../blacklist.cfg."
      Take it out the same way.  (Rewritten: the old access-lists check wanted Conductor shut down for
      this; DiskMan reads a hand edit now, so the server stopped is enough.)
- [ ] STOP SERVER and START SERVER with no hand edits come up clean, and the Storage tab's cache hits
      count up as before.

### 3. Networking: the TLS pair

- [ ] Move `Content/certs/conductor.key` away, STOP SERVER, START SERVER: the Services tab has
      Network (TCP) in trouble and Network (UDP) stopped ("Not started: the TCP side couldn't."), the
      log has an Error starting "NOBODY CAN LOG IN.", and the three Network Admin tabs stay greyed.  Put
      the key back and RESTART SERVER: both running.  (Rewritten: the old check said both in trouble.)

### 4. Networking: the door (TCP)

- [ ] START SERVER: the log has the Debug line "Access lists: off.  0 on the whitelist and 0 on the
      blacklist, neither looked at." and two "Read 0 entries from ..." lines.  The three Network Admin
      tabs unlock.
- [ ] The Conductor tab's Threads, "Asked for": `net-dns` is there and running.
- [ ] Run `python3 conductor-networking/test_client.py --leave-after 3 throwaway_01 'Throwaway 1!'` six
      times.  The Connections tab's Recent shows the newest five and the count says "the newest 5 of 6
      since START SERVER"; Historical shows all six.  Each reads "Logged in and handed a ticket for UDP"
      while its player is in (in green), and the rows stay as long as the server runs.
- [ ] While one is in the world (make it `--leave-after 20`), the UDP table shows its address,
      `throwaway_01` in green, the connected stamp, playing for counting up as DD:HH:MM:SS, quiet for at
      0 s or 1 s.
- [ ] A wrong password, then a right one straight after, in one line:
      `python3 conductor-networking/test_client.py throwaway_01 wrong; python3 conductor-networking/test_client.py throwaway_01 'Throwaway 1!'`
      The first row ends "Refused: wrong secret word, name or password", greyed.  The second is
      "Closed at the door: on hold after a failed login", and the log's Debug line says how many ms of
      the hold were left.
- [ ] A wrong secret word: `python3 conductor-networking/test_client.py --secret wrong throwaway_01 'Throwaway 1!'`
      Refused, and no Security line in the log (it never reached a hash).
- [ ] Two in Security's line, started together in one line:
      `python3 conductor-networking/test_client.py --leave-after 5 --leave-other-alone throwaway_01 'Throwaway 1!' & python3 conductor-networking/test_client.py --leave-after 5 --leave-other-alone throwaway_01 'Throwaway 1!'; wait`
      One of them prints an InLine ("1 ahead, about N ms"); its row may show "In Security's line" for a
      blink.  One gets in, the other is told the account is logged in and hangs up.  (Rewritten: two
      terminals by hand are too slow to catch the line.)
- [ ] STOP SERVER: the three tabs grey, `net-dns` reads finished on the Conductor tab, and on the next
      START SERVER the Connections tab is empty.
- [ ] Nothing else on the page changed: the Storage, Services and Settings tabs look as they did.

### 5. Networking: leaving the world (UDP)

- [ ] Goodbye: `python3 conductor-networking/test_client.py --leave-after 10 throwaway_01 'Throwaway 1!'`.
      When it says Goodbye, the UDP row goes and its TCP row greys to "LINKDEAD: said Goodbye".
- [ ] Quiet: `python3 conductor-networking/test_client.py --go-quiet throwaway_01 'Throwaway 1!'`.  About
      40 seconds on the log says the player was dropped, the UDP row goes, and the TCP row reads
      "LINKDEAD: went quiet past the UDP timeout".  Ctrl-C the client after.
- [ ] STOP SERVER with a player in the world (a client with no `--leave-after`): the client prints a
      Kicked "server stopping" and "Back to the login screen."
- [ ] Both views: the LINKDEAD rows are in Recent (while among the newest five) and in Historical.

### 6. Networking: KICK

- [ ] A LINKDEAD or refused row's three dots: KICK is there, greyed, and its tooltip says there's
      nothing to kick.
- [ ] An open connection: `python3 conductor-networking/test_client.py --pause-before-login 8 throwaway_01 'Throwaway 1!'`,
      and while it pauses its row reads "Waiting for its Login".  KICK from its three dots, confirm: the
      row reads "Kicked by the admin", the client says the server hung up (or the connection broke), and
      the log has "The admin kicked ... at the door."  Eight seconds is short for the menu; if it's too
      tight, set `login_deadline_seconds` to 30 on the Settings tab, STOP SERVER and START SERVER, and
      pause 25.  Put it back to 10 after.  (Rewritten: the old check said Ctrl-Z after TLS, which can't
      be caught, and "no button" on a finished row, which KICK on any row changed; the 404 for a row
      with nothing left is covered by the web admin's tests.)

### 7. Networking: the access lists

- [ ] The three dots on a TCP row: the menu opens beside the row with the address at the top, KICK,
      ADD <address> TO WHITELIST, ADD <address> TO BLACKLIST.  Clicking elsewhere, or Escape, closes it.
- [ ] Whitelist tab: the tile says OFF and "Off: nobody is checked at the door ... does nothing until
      access_list in networking.cfg says whitelist."  ADD `127.0.0.1`: the entry appears with REMOVE, the
      line says it's on the whitelist and the whitelist isn't switched on, the log has "The admin added
      127.0.0.1 to the whitelist.", and `Content/cfg/whitelist.cfg` has the line.  ADD it again: "was on
      the whitelist already."  ADD `potato`: a red line, nothing added.  ADD `10.0.0.5/24`: kept as
      `10.0.0.0/24`.  Enter in the field adds too.
- [ ] REMOVE `10.0.0.0/24`: asks first, the row goes, the file loses the line, the log says so.
- [ ] Blacklist mode: add `127.0.0.1` to the blacklist, set `access_list` to `blacklist` on the Settings
      tab, STOP SERVER, START SERVER.  The log's Info line says "Access lists: blacklist, 1 ...".  The
      Blacklist tab's tile reads BLACKLIST in green; the Whitelist tab's reads BLACKLIST in yellow with
      "The blacklist is on, not this list."  Run the client: it fails, and the row reads "Closed at the
      door: blacklisted", with the Debug line "127.0.0.1:... is on the blacklist.  Closed at the door."
- [ ] The ban on a player: REMOVE `127.0.0.1` from the blacklist, run the client with no `--leave-after`
      so it sits in the world, then from its TCP row's three dots ADD 127.0.0.1 TO BLACKLIST, confirm.
      The panel's line says "0 connection(s) closed at the door, 1 player(s) dropped", the UDP row goes,
      the client prints Kicked "banned" and "Back to the login screen.", the log has "Banned:
      throwaway_01 at 127.0.0.1:... was dropped from the world.", and the TCP row reads "LINKDEAD:
      banned".  Run the client again: closed at the door.
- [ ] The ban on an open connection: REMOVE `127.0.0.1` again, run the client with
      `--pause-before-login 8` (or longer, as in 6), and while it pauses ADD its address TO BLACKLIST
      from its row.  The row reads "Banned: the access lists changed and the address isn't let in", and
      the log has "The access lists changed: 1 connection(s) banned at the door."
- [ ] A range: blacklist still on, ADD `127.0.0.0/8`, clear `127.0.0.1`; the client is closed at the door.
      Clear the blacklist after.
- [ ] Whitelist mode, empty: set `access_list` to `whitelist` with the whitelist empty, STOP SERVER,
      START SERVER.  A Warn on the bell: "... the whitelist is empty: NOBODY CAN LOG IN."  The client is
      turned away, "Closed at the door: not on the whitelist".
- [ ] ADD `127.0.0.1` to the whitelist: the next run of the client logs in, no reboot.  With it sitting
      in the world, REMOVE `127.0.0.1` (the confirm warns): "1 player(s) dropped", Kicked "banned", and
      its next run is closed at the door.
- [ ] Add `127.0.0.0/8` and `127.0.0.1` both, log the client in, REMOVE `127.0.0.1` alone: "0 player(s)
      dropped", the client stays in (the range still covers it).  Set `access_list` back to `off`, clear
      both lists, STOP SERVER, START SERVER.
- [ ] `access_list = potato` in `networking.cfg` by hand, server stopped: START SERVER logs an Error and
      runs with nobody checked.  Put it back to `off`.

### 8. The web admin: `user` and `admin`

With the server running, so the Network Admin tabs are open.

- [ ] Log in as `user`: every button that changes something is greyed (the server buttons, SHUT DOWN,
      TEST NOTIFICATION, both ACK ALLs, the Settings fields, the list ADD fields and buttons, every
      REMOVE, the three dots).  The Whitelist, Blacklist and Connections tabs still open.
- [ ] Forced as `user`, in the browser console:
      `fetch('/Opus/wwwhook/stop',{method:'POST',headers:{'X-Opus':'server'}}).then(r=>r.text()).then(console.log)`
      prints "Only admin can do that. ..." and the server keeps running.
- [ ] LOG OUT, log in as `admin` without reloading: all of them open (the server buttons as the
      server's state allows).
- [ ] With the Settings tab open when you LOG OUT: after the `admin` login its fields and SAVE open
      without leaving the tab.  The same on the Whitelist or Blacklist tab with an entry on it: REMOVE
      opens.
- [ ] LOG OUT, log in as `user` again: everything greys again.

### Parked

Not in this run: nothing to run them on today.

- The check from an outside machine (the access lists section): a laptop on a phone hotspot.
- The Windows build.

## Still open from earlier sessions

Its open checks are carried into the test run above.

Hand tests the networking session left untried (STATUS.md had them; they live here now):

- [x] ~~`cargo test` from `Conductor/dev` after networking.~~  2026-09-29: passed with the TCP tab's, 178 in all.
- [x] ~~Two clients on one account: the second gets the kick-or-hang-up prompt ("Already logged in.  Log
      the other session out?"; `--leave-other-alone` on `test_client.py` answers no without asking);
      answer `y` and the first hears a Kicked over UDP.~~  2026-09-29, from `testing` with this session's
      DiskMan change in: "throwaway_01 is already in the world ... Asking 10.0.0.84:44194 what to do.",
      then "logged the other session on throwaway_01 out" and the second in the world.  The same run
      saved `bind_address` on the Settings tab and RESTART SERVER came up on the new address.
- [ ] `--go-quiet`: the player is dropped after the 40-second UDP timeout and the log says so.
- [ ] A wrong secret word: refused without a hash (no Security line in the log).
- [ ] A wrong password, then a connection straight after from the same machine: closed at the door for
      two seconds (the hold), and the log's Debug line says how much of the hold was left.
- [ ] STOP SERVER with a player in the world: the player hears a Kicked.
- [ ] The `user` login on the web admin in a real run: every button greyed, and a 403 if one is forced.

## 2026-09-29 -- The TCP tab and KICK

Its open checks are carried into the test run above.

Build and tests: `cargo build` and `cargo test` from `Conductor/dev`.  New tests: 5 in `ledger.rs`, 3 in
`dns.rs`, 2 in `dns/linux.rs`, 2 in `json.rs`, 1 in the web admin's `lib.rs`.  One of the `dns.rs` tests
does a real reverse lookup of `127.0.0.1`, which asks the resolver; it passes with a name or without one.
~~Build and tests~~ 2026-09-29: `cargo clean && cargo build && cargo test` clean, 178 passed (15 monitor,
44 networking, 89 tools with the benchmark ignored, 30 web admin), no warnings.

- [x] ~~Before START SERVER: the TCP tab is greyed in the sidebar, like the other data tabs.~~  2026-09-29.
- [x] ~~After START SERVER with the database connected: the TCP tab is clickable, and empty ("0 in the last
      5 minutes").  The top tile says where TCP and UDP listen.~~  2026-09-29, screenshot.
- [ ] With the TLS files moved away and the server restarted: both network services in trouble, and the
      TCP tab stays greyed (it wants both listeners up).  Put the files back after.
- [x] ~~Run the Python client and watch the tab: the row appears, moves through the stages, and ends as
      "Logged in and handed a ticket for UDP".  The address is the client's, with its port.~~  2026-09-29,
      screenshot.  The stages go by in half a second, so KICK is up for about that long on a login that
      isn't held up; by design (TCP is only the login).  One miss: the finished row's "Logged in" is
      grey, not green (the greyed-row style outweighs `.good-ink`; a one-line CSS fix, in TODO.md).
- [x] ~~Host: `127.0.0.1` shows `localhost` once the lookup is back.  The log has a Debug line
      "Reverse DNS: ... is ...".~~  2026-09-29: both, in the same second as the connection.
- [ ] A wrong password: the row ends as "Refused: wrong secret word, name or password", greyed.  The
      connection closed at the door two seconds later shows as "Closed at the door: on hold after a
      failed login".
- [x] ~~Ago counts up once a second~~ 2026-09-29.  ~~A finished row disappears five minutes after it
      arrived.~~  2026-09-29: dropped, the ledger keeps the whole run now (the access lists session).
- [x] ~~The Settings tab shows `connections_remember_seconds` (300) on `networking.cfg`'s card.  Save it as
      10, STOP SERVER and START SERVER, run the client once: its row goes ten seconds after it arrived,
      and the tab's count says "in the last 10 seconds".  Put it back to 300 after.~~  2026-09-29: the
      setting is gone with the five minutes.
- [ ] KICK as `admin`: start the client with a long wait before it sends its Login (or Ctrl-Z it after
      TLS), press KICK on its row, confirm.  The row reads "Kicked by the admin", the client's connection
      drops, and the log has "The admin kicked ... at the door."  Pressing KICK again on a finished row
      isn't possible (no button); a kick sent by hand for a finished number gets a 404.
- [ ] KICK as `user`: the button is greyed.
- [ ] Two clients at once (two terminals): both rows show; the second one, while the first is in
      Security's line, says "In Security's line: 1 ahead, about N ms" for a second or two.
- [ ] STOP SERVER: the tab greys again and the list is empty on the next START SERVER.
- [ ] The Conductor tab's Threads, "Asked for": `net-dns` is there, running with the server and finished
      after STOP SERVER.
- [ ] Nothing else on the page changed: the Storage, Services and Settings tabs look as they did.

## 2026-09-29 -- The access lists, the Network Admin subsection

Its open checks are carried into the test run above.

Build and tests: `cargo build` and `cargo test` from `Conductor/dev`.  New tests: 10 in `access.rs`, 2 in
`sessions.rs`, 2 in the web admin's `lib.rs`, 2 in `json.rs`; one changed in `settings.rs`, one in
`json.rs` (the status shape grew) and one in `protocol.rs` (reason 3).  **The protocol is version 2
now**: the Python client says 2 too, so an old copy of it stops at the Hello.  The page was rendered headless with made-up numbers in the session
and clicked through (the three tabs, the row menu, ADD, REMOVE); that isn't Conductor.

- [x] ~~Build and tests: `cargo build` clean, no warnings, `cargo test` passes.~~  2026-09-29: clean, 194
      passed (15 monitor, 56 networking, 89 tools with the benchmark ignored, 34 web admin).  The first
      run read both list files (0 entries) and said "Access lists: off".
- [ ] Before START SERVER: the sidebar has a rule and a NETWORK ADMIN heading under Notifications
      History, with Connections, Whitelist and Blacklist indented under it, all three greyed, then Log
      and Settings.  Nothing else in the sidebar moved.
- [ ] The Settings tab shows three new settings on `networking.cfg`'s card: `access_list` (off),
      `whitelist_file` (cfg/whitelist.cfg), `blacklist_file` (cfg/blacklist.cfg), and
      `connections_remember_seconds` is gone.  If your `networking.cfg` still has that line (a hand
      edit), START SERVER warns about it once; take the line out.
- [ ] START SERVER: the log has a Debug line "Access lists: off.  0 on the whitelist and 0 on the
      blacklist, neither looked at." and two "Read 0 entries from .../whitelist.cfg" lines (or "Wrote an
      empty ..." on a first run if the committed files aren't there).  The three tabs unlock with the
      TCP tab's old rule.
- [ ] Connections: the TCP table with two small tabs in its head, Recent (on) and Historical, then a UDP
      table.  Run the client six times: Recent shows the newest five and the count says "the newest 5
      of 6 since START SERVER"; Historical shows all six.  A finished row stays as long as the server
      runs.  Once the client is in the world a UDP row shows its address, `throwaway_01` in green, the
      connected stamp, playing for counting up as DD:HH:MM:SS, quiet for at 0 s or 1 s.  Goodbye (or
      `--leave-after 20`) takes the UDP row away.  The finished TCP row's "Logged in" reads green now
      (the CSS line from TODO.md went in with this).
- [ ] The three dots on a TCP row (`admin`): the menu opens beside the row with the address at the top,
      KICK on an open row only, ADD <address> TO WHITELIST, ADD <address> TO BLACKLIST.  Click anywhere
      else, or Escape, closes it.  As `user` the dots are greyed.
- [ ] Whitelist tab: the tile says OFF and "Off: nobody is checked at the door ... does nothing until
      access_list in networking.cfg says whitelist."  ADD `127.0.0.1`: the entry appears with REMOVE, the
      line says "127.0.0.1 is on the whitelist.  The whitelist isn't switched on ..., so it does nothing
      yet.", the log has "The admin added 127.0.0.1 to the whitelist.", and `Content/cfg/whitelist.cfg`
      has the line.  ADD it again: "was on the whitelist already."  ADD `potato`: a red line in
      Conductor's words, nothing added.  ADD `10.0.0.5/24`: it's kept as `10.0.0.0/24`.  Enter in the
      field adds too.
- [ ] REMOVE `10.0.0.0/24`: asks first, the row goes, the file loses the line, the log says so.
- [ ] STOP SERVER: the three tabs grey.  The lists can't be touched from the page.  Edit
      `Content/cfg/blacklist.cfg` by hand with Conductor shut down (not just the server stopped; TODO.md
      says why): add `127.0.0.1`, run Conductor, START SERVER, and the Blacklist tab shows it.
- [ ] Blacklist mode: on the Settings tab set `access_list` to `blacklist`, STOP SERVER, START SERVER.  The
      log's Info line says "Access lists: blacklist, 1 entry turned away."  The Blacklist tab's tile reads
      BLACKLIST in green; the Whitelist tab's reads BLACKLIST in yellow with "The blacklist is on, not this
      list."  Run the client: it fails to connect, and the Connections tab has a row "Closed at the door:
      blacklisted" with a Debug line "127.0.0.1:... is on the blacklist.  Closed at the door."
- [ ] The ban: REMOVE `127.0.0.1` from the blacklist, run the client (it logs in and sits in the world),
      then use its finished TCP row's three dots (the UDP table has no menu; the address is the same),
      ADD 127.0.0.1 TO BLACKLIST, confirm.  The line in the TCP panel's head says "0 connection(s)
      closed at the door, 1 player(s) dropped" (the TCP row finished half a second after it arrived),
      the UDP row goes, the client prints a Kicked with reason "banned" and "Back to the login screen.",
      and the log has "Banned: throwaway_01 at 127.0.0.1:... was dropped from the world."  Run the
      client again: closed at the door.
- [ ] The ban on an open TCP connection: with 127.0.0.1 off the blacklist, Ctrl-Z the client after TLS
      (or start it with a long pause before its Login), then ADD its address TO BLACKLIST from its row.
      The row reads "Banned: the access lists changed and the address isn't let in", the log has "The
      access lists changed: 1 connection(s) banned at the door."
- [ ] Whitelist mode: set `access_list` to `whitelist` with the whitelist empty, STOP SERVER, START SERVER.
      A Warn on the bell: "access_list is whitelist and the whitelist is empty: NOBODY CAN LOG IN."  The
      client is turned away, "Closed at the door: not on the whitelist".  ADD `127.0.0.1` to the
      whitelist: the next run of the client logs in, no reboot.  With the client sitting in the world,
      REMOVE `127.0.0.1` from the whitelist (the confirm warns): the line says "1 player(s) dropped",
      the client prints Kicked "banned", and its next run is closed at the door.  Add `127.0.0.0/8` and
      `127.0.0.1` both, log the client in, REMOVE `127.0.0.1` alone: "0 player(s) dropped", the client
      stays in (the range still covers it).  Put `access_list` back to `off` after, and clear both
      lists.
- [ ] A range: with the blacklist on, ADD `127.0.0.0/8`; the client from 127.0.0.1 is closed at the
      door.
- [ ] `access_list = potato` in `networking.cfg` (by hand, server stopped): START SERVER logs an Error
      and runs with nobody checked.
- [ ] The `user` login: the three tabs open, the ADD fields and buttons and every REMOVE and the three
      dots are greyed.
- [ ] From outside (when there's an outside machine): forward TCP 9997 and UDP 9998 on the router, run
      the client from a laptop on a phone hotspot with `--host 142.56.230.42`, and the Connections tab
      shows the outside address (with a Host name if its reverse DNS has one); blacklist it from the
      row's menu and the next try is closed at the door.

## 2026-09-29 -- The documentation pass: the page's greying

Its open checks are carried into the test run above.

Build: `cargo build` from `Conductor/dev` (the page is baked in, so it needs a build; no Rust changed).
Checked in the session only by driving the page's two functions headless with made-up states; that isn't
Conductor.

- [ ] Log in as `user`: every button that changes something is greyed (the server buttons, SHUT DOWN,
      TEST NOTIFICATION, both ACK ALLs, the list ADD fields and buttons, the Settings fields).
- [ ] LOG OUT, log in as `admin` without reloading: all of them open (the server buttons as the
      server's state allows).  Before the fix, SHUT DOWN and the rest stayed greyed until a reload.
- [ ] The same with the Settings tab open when you LOG OUT: after the `admin` login its fields and SAVE
      open without leaving the tab.  The same on the Whitelist or Blacklist tab with an entry on it: its
      REMOVE opens.
- [ ] LOG OUT, log in as `user` again: everything greys again.

## 2026-09-29 -- DiskMan notices a hand edit

Its open checks are carried into the test run above.

Build and tests: `cargo build` and `cargo test` from `Conductor/dev`.  New tests: 1 in `diskman.rs` (a
write of ours, then a hand edit, then a hand delete) and 1 in `diskman/cache.rs`; two in `cache.rs`
changed for the new argument.  Nothing was built in the session.

- [x] ~~`cargo build` clean, no warnings; `cargo test` passes, 196 (91 tools) with the two new ones.~~
      2026-09-29, from `testing` after `cargo clean`: no warnings,
      199 passed (15 monitor, 59 networking, 91 tools with the benchmark ignored, 34 web admin).
- [ ] A soft file: START SERVER, STOP SERVER, change `slow_job_ms` in `Content/cfg/postgres.cfg` by hand
      (Conductor still running), START SERVER.  The Settings tab shows the new value as running.  Before
      this, it showed the old one until Conductor was run again.  Put it back after.
- [ ] A list file: with the server stopped, add `10.0.0.1` to `Content/cfg/blacklist.cfg` by hand, START
      SERVER: the Blacklist tab shows it, and the log has the Debug line "Read 1 entry from .../blacklist.cfg."
      Take it out the same way.
- [ ] Nothing else changed: STOP SERVER and START SERVER with no hand edits still come up clean, and the
      Storage tab's cache hits count up the same as before.

## 2026-09-29 -- LINKDEAD on the Connections tab

Its open checks are carried into the test run above.

Build and tests: `cargo build` and `cargo test` from `Conductor/dev`.  New tests: 2 in `ledger.rs`; the
book's tests in `sessions.rs` changed for the row numbers and check the LINKDEAD rows; one in `json.rs`
grew a LINKDEAD row.  Nothing was built in the session.

- [x] ~~`cargo build` clean, no warnings; `cargo test` passes, 198 with the two new ones.~~  The same run:
      199 with KICK's on top.
- [x] ~~Two clients on one account, `y` to log the first out: the first client's TCP row greys and reads
      "LINKDEAD: logged out by a second login from 10.0.0.84:<the second's port>", and the second's row
      stays green "Logged in and handed a ticket for UDP".  The UDP table has only the second.~~
      2026-09-29: Jacob's screenshot showed the row "... logged out by a second login from 10.0.0.84:34526",
      and he said it was all working.
- [ ] A client with `--leave-after 10`: once it says Goodbye, its row reads "LINKDEAD: said Goodbye".
- [ ] A client with `--go-quiet`: 40 seconds on, "LINKDEAD: went quiet past the UDP timeout".
- [ ] A ban from the row's menu (the blacklist on): "LINKDEAD: banned".
- [ ] Both views: the LINKDEAD rows are in Recent (while among the newest five) and Historical alike.

## 2026-09-29 -- KICK on any row of the Connections tab

Its open checks are carried into the test run above.

Build and tests: `cargo build` and `cargo test` from `Conductor/dev`.  New tests: 1 in `sessions.rs`;
`protocol.rs` and `ledger.rs` each check the new reason.  **The protocol is version 3 now**: the Python
client says 3 too, so an old copy of it stops at the Hello.  The menu was opened headless in the session
with made-up rows (an open connection, a live login, a LINKDEAD one); that isn't Conductor.

- [x] ~~`cargo build` clean, no warnings; `cargo test` passes, 199.~~  2026-09-29, from `testing` after
      `cargo clean`: no warnings, 199 passed (15 monitor, 59 networking, 91 tools with the benchmark
      ignored, 34 web admin).
- [x] ~~Log the client in and leave it in the world.  Its TCP row's three dots: KICK is red and live.
      KICK, confirm ("Kick the player who logged in from ... out of the world?"): the client prints a
      Kicked "kicked by the admin" and "Back to the login screen.", the UDP table empties, the row reads
      "LINKDEAD: kicked by the admin", and the log has "The admin kicked throwaway_01 at ... out of the
      world."  Run the client again: it logs straight back in.~~  2026-09-29: the log has the kick three
      times, the client logged straight back in after the first, and the third came after a second login
      had logged the first out.  Jacob: "we looking good".
- [ ] A LINKDEAD or refused row's three dots: KICK is there, greyed, and its tooltip says there's nothing
      to kick.
- [ ] An open connection (the client Ctrl-Z'd after TLS): KICK closes it at the door, as before.
- [ ] As `user`: the three dots are greyed, as before.
