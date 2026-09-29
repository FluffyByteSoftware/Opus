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

## Still open from earlier sessions

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

Build and tests: `cargo build` and `cargo test` from `Conductor/dev`.  New tests: 1 in `diskman.rs` (a
write of ours, then a hand edit, then a hand delete) and 1 in `diskman/cache.rs`; two in `cache.rs`
changed for the new argument.  Nothing was built in the session.

- [ ] `cargo build` clean, no warnings; `cargo test` passes, 196 (91 tools) with the two new ones.
- [ ] A soft file: START SERVER, STOP SERVER, change `slow_job_ms` in `Content/cfg/postgres.cfg` by hand
      (Conductor still running), START SERVER.  The Settings tab shows the new value as running.  Before
      this, it showed the old one until Conductor was run again.  Put it back after.
- [ ] A list file: with the server stopped, add `10.0.0.1` to `Content/cfg/blacklist.cfg` by hand, START
      SERVER: the Blacklist tab shows it, and the log has the Debug line "Read 1 entry from .../blacklist.cfg."
      Take it out the same way.
- [ ] Nothing else changed: STOP SERVER and START SERVER with no hand edits still come up clean, and the
      Storage tab's cache hits count up the same as before.

## 2026-09-29 -- LINKDEAD on the Connections tab

Build and tests: `cargo build` and `cargo test` from `Conductor/dev`.  New tests: 2 in `ledger.rs`; the
book's tests in `sessions.rs` changed for the row numbers and check the LINKDEAD rows; one in `json.rs`
grew a LINKDEAD row.  Nothing was built in the session.

- [ ] `cargo build` clean, no warnings; `cargo test` passes, 198 with the two new ones.
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

Build and tests: `cargo build` and `cargo test` from `Conductor/dev`.  New tests: 1 in `sessions.rs`;
`protocol.rs` and `ledger.rs` each check the new reason.  **The protocol is version 3 now**: the Python
client says 3 too, so an old copy of it stops at the Hello.  The menu was opened headless in the session
with made-up rows (an open connection, a live login, a LINKDEAD one); that isn't Conductor.

- [ ] `cargo build` clean, no warnings; `cargo test` passes, 199.
- [ ] Log the client in and leave it in the world.  Its TCP row's three dots: KICK is red and live.
      KICK, confirm ("Kick the player who logged in from ... out of the world?"): the client prints a
      Kicked "kicked by the admin" and "Back to the login screen.", the UDP table empties, the row reads
      "LINKDEAD: kicked by the admin", and the log has "The admin kicked throwaway_01 at ... out of the
      world."  Run the client again: it logs straight back in.
- [ ] A LINKDEAD or refused row's three dots: KICK is there, greyed, and its tooltip says there's nothing
      to kick.
- [ ] An open connection (the client Ctrl-Z'd after TLS): KICK closes it at the door, as before.
- [ ] As `user`: the three dots are greyed, as before.
