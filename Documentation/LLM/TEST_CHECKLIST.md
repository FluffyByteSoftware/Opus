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

The 2026-09-29 test run went through every check up to then, by subsystem, and all of them passed;
the one bug it found is fixed below.

## 2026-09-29 -- The Settings tab after a RESTART (the test run's bug 1)

Build: `cargo build` (the page is baked in; no Rust changed, so `cargo test` stays at 199).  Checked in
the session only for the script's syntax; nothing was run.

- [ ] Save a change to `slow_job_ms` on the Settings tab: "WAITING ON A SOFT REBOOT".  Stay on the tab
      (or open it again straight after pressing RESTART SERVER on the Control Panel).  Once the server
      is running again, the tab redraws on its own: the warning is gone and the new value is running,
      without clicking away.  Put the value back after.

## 2026-09-29 -- The account in memory (conductor-accounts)

Build: `cargo build` with no warnings; `cargo test` should come to 203 (4 new, in conductor-accounts).
Written in the session, not built there.  To see the row, from any folder:
`psql -h localhost -U opus_game -d opusdb -c "SELECT account_username, last_login_datetime FROM accounts;"`

- [ ] A login still works, and the time lands when the player leaves.  Run:
      `python3 conductor-networking/test_client.py throwaway_01 'Throwaway 1!' --leave-after 20`
      While it's in, `last_login_datetime` in the row hasn't moved; once it says Goodbye, it's the time
      of the login.
- [ ] STOP SERVER with the client in the world (no `--leave-after`): the row has the new login time.
- [ ] Log in from a second client and log the first one out: the row has the second login's time.
- [ ] A wrong password still gets the one failure answer, and nothing in the row changes.

## Parked

Nothing to run these on yet.

- [ ] From outside, when there's an outside machine: forward TCP 9997 and UDP 9998 on the router, run
      the client from a laptop on a phone hotspot with `--host 142.56.230.42`, and the Connections tab
      shows the outside address (with a Host name if its reverse DNS has one); blacklist it from the
      row's menu and the next try is closed at the door.
- [ ] The Windows build, whenever getting to that machine is less of a hassle.  The Windows code has
      never been compiled.
