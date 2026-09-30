<!--
File:       Opus/Documentation/LLM/TEST_CHECKLIST.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Test Checklist

What's still to check on `testing`: Jacob's reminder, not a history.  Every session that changes what
Conductor (or, later, Ensemble) does adds its checks under a heading with the date and the feature, and
the reply that pushes `unstable` onto `testing` points here.  Once a check passes it's taken out, and a
heading goes with its last check; git keeps what was here.  A check that fails stays, with a note on
what went wrong, until it passes.  Jacob's ask, 2026-09-29: once game features come, this is the
reminder of what changed and what to look at in game.

Every command is one line, from `Conductor/dev`.  The test account is `throwaway_01` / `Throwaway 1!`,
and the client is `conductor-networking/test_client.py`.

## 2026-09-30 -- the web admin's sections

Only `page.html` changed, so this is all looking at the page.

- [ ] The header is two rows: the logo, the name and status line, the pill, the uptime and the bell, then
      CONTROL PANEL | CONFIGURATION | LOGS | ACCOUNT MANAGEMENT | GAME MANAGEMENT under them.
- [ ] Each section's side menu, in this order: Server, System, Conductor, Services, Storage /
      Settings, Whitelist, Blacklist / Log, Notifications History / Accounts / Connections.  The tab
      you're on says its section and its name at the top.
- [ ] With the server stopped: CONTROL PANEL > Server shows.  Clicking CONFIGURATION opens Settings, LOGS
      opens Log.  ACCOUNT MANAGEMENT and GAME MANAGEMENT show their one tab greyed with "These open once
      the server is running." and leave the open tab where it was.
- [ ] START SERVER: the page moves to the remembered tab (Conductor the first time) and its section.
      Every tab opens as it did before, by the same rules.  Logged in as `user`, ACCOUNT MANAGEMENT says
      "Only admin can open the accounts."
- [ ] STOP SERVER from any section goes back to CONTROL PANEL > Server.
- [ ] Stop Postgres with the server running: the side menu of a section with nothing open says it's
      waiting on the database, and the blur and its card are as before.
- [ ] A service down while running: the red dot flashes on the Services tab and on CONTROL PANEL.
- [ ] The bell: the tray has HISTORY beside ACK ALL, and HISTORY opens LOGS > Notifications History and
      closes the tray.  With no open notices the bell still opens the tray, saying "No open notices.",
      with HISTORY only.
- [ ] The Log tab's black box fills down to near the bottom of the window without the page scrolling.
- [ ] With the server running, reload the page: you land back on the same tab, in its section.

## 2026-09-29 -- the account manager, and accounts never held

The test client speaks protocol version 4 now, and an older copy of it is turned away at the Hello.
`newplayer_01` below is any name you make on the page; the database check is
`psql -h localhost -U opus_game -d opusdb -c "SELECT account_username, owner_email, last_login_datetime FROM accounts ORDER BY account_username;"`.

- [ ] `cargo build` with no warnings, and `cargo test` all passing.
- [ ] The Server tab and the Services tab list **Account desk**: grey while stopped, running after
      START SERVER, stopped again after STOP SERVER.
- [ ] ACCOUNT MANAGEMENT has **Accounts** under it (it was a GAME ADMIN heading when this was written).
      Greyed while the server is stopped; open once it runs and the database is connected; greyed when
      logged in as `user`.
- [ ] The list shows `throwaway_01` with its owner, email and last login, and ONLINE says IN THE WORLD
      while the test client is logged in on it, `--` once it's gone.
- [ ] NEW ACCOUNT, with mistakes: name `Jacob`, first name empty, email `nope`, password `short`, then a
      good password and a different one in the second box.  Each complaint shows in red under its field
      and nothing is made.
- [ ] NEW ACCOUNT, done right: "Working: waiting its turn..." then "Made the account ...", its card
      opens, it's on the list, and the log says "The admin made the account ...".  The same name again
      says there's already an account called that; `throwaway_01`'s email again says another account has
      it.
- [ ] `python3 conductor-networking/test_client.py newplayer_01 'Its password 1!'` logs in with the new
      account.  Its last login on the list (reopen the tab) and in the database is the second the log
      says it came into the world.
- [ ] With that client still in the world, change the owner's email on its card and SAVE: "Saved.".
      Then Ctrl-C the client.  The database still has the new email (the old design would have written
      the old one back as the player left).
- [ ] CHANGE PASSWORD on it, the two boxes different: red, nothing sent.  The same twice: "Working..."
      then "Changed the password...".  The old password now fails at the login and the new one works.
- [ ] DELETE ACCOUNT with the client in the world: the question says they'll be taken out; the client
      prints `<- Kicked ACCOUNT TERMINATED` and goes back to the login screen; the Connections row reads
      "LINKDEAD: account terminated"; the account is off the list and out of the database; logging in
      with it again fails.
- [ ] Ctrl-C Conductor itself while a player is in the world: their last login is still in the database
      (it's written at the connect now, not when they leave).
- [ ] While you're on the Whitelist tab: a bad entry's reason is red now, not grey (a page-wide fix that
      came with this; the Settings tab's complaints and WAITING lines got their colours back the same way).

## Parked

Nothing to run these on yet.

- [ ] From outside, when there's an outside machine: forward TCP 9997 and UDP 9998 on the router, run
      the client from a laptop on a phone hotspot with `--host 142.56.230.42`, and the Connections tab
      shows the outside address (with a Host name if its reverse DNS has one); blacklist it from the
      row's menu and the next try is closed at the door.
- [ ] The Windows build, whenever getting to that machine is less of a hassle.  The Windows code has
      never been compiled.
