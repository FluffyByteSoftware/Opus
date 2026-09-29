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

- [ ] `cargo test` from `Conductor/dev` after networking.  The build and the run passed; the tests haven't
      been pasted back.
- [ ] Two clients on one account: the second gets the kick-or-hang-up prompt (`--kick` and `--spare` on
      `test_client.py`); with `--kick` the first hears a Kicked over UDP.
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

- [ ] Before START SERVER: the TCP tab is greyed in the sidebar, like the other data tabs.
- [ ] After START SERVER with the database connected: the TCP tab is clickable, and empty ("0 in the last
      5 minutes").  The top tile says where TCP and UDP listen.
- [ ] With the TLS files moved away and the server restarted: both network services in trouble, and the
      TCP tab stays greyed (it wants both listeners up).  Put the files back after.
- [ ] Run the Python client (`python3 conductor-networking/test_client.py`, the usual line) and watch the
      tab: the row appears, moves through TLS and "Waiting for its Login" and "Checking the login", and
      ends green as "Logged in and handed a ticket for UDP".  The address is the client's, with its port.
- [ ] Host: `127.0.0.1` shows `localhost` (or whatever `/etc/hosts` calls it) once the lookup is back, a
      second or so after the row appears.  The log has a Debug line "Reverse DNS: ... is ...".
- [ ] A wrong password: the row ends as "Refused: wrong secret word, name or password", greyed.  The
      connection closed at the door two seconds later shows as "Closed at the door: on hold after a
      failed login".
- [ ] Ago counts up once a second; a finished row disappears five minutes after it arrived.
- [ ] The Settings tab shows `connections_remember_seconds` (300) on `networking.cfg`'s card.  Save it as
      10, STOP SERVER and START SERVER, run the client once: its row goes ten seconds after it arrived,
      and the tab's count says "in the last 10 seconds".  Put it back to 300 after.
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
