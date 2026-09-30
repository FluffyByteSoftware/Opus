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

## Parked

Nothing to run these on yet.

- [ ] From outside, when there's an outside machine: forward TCP 9997 and UDP 9998 on the router, run
      the client from a laptop on a phone hotspot with `--host 142.56.230.42`, and the Connections tab
      shows the outside address (with a Host name if its reverse DNS has one); blacklist it from the
      row's menu and the next try is closed at the door.
- [ ] The Windows build, whenever getting to that machine is less of a hassle.  The Windows code has
      never been compiled.
