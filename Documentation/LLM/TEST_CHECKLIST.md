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

## 2026-09-30 -- Lua's first step (`lua-parser`)

The first build compiles Lua itself with `gcc`, so it takes longer than usual, and it changes
`Cargo.lock`.

- [ ] `cargo build` builds, with no warnings from `lua-parser`.
- [ ] `cargo test -p conductor-lua-parser` passes all 8 (the runaway ones take about a second each).
- [ ] `cargo run -p conductor-launcher`, START SERVER: the Log tab has `scripts/hello.lua, line 3: Hello
      from Lua.` on the Script channel, then `Lua is up.  Ran 1 script(s) ... 0 with an error.`  The
      Services tab has Lua, running.
- [ ] With the server running, make four test scripts (one line each, four commands):
      `printf 'log.info("never shown")\nthis is not lua\n' > /opt/storage/Coding/Opus/Content/scripts/broken.lua`
      `echo 'for i = 1, 1000 do log.info("line " .. i) end' > /opt/storage/Coding/Opus/Content/scripts/flood.lua`
      `echo 'while true do end' > /opt/storage/Coding/Opus/Content/scripts/runaway.lua`
      `echo 'log.info(io, os, require, dofile, load)' > /opt/storage/Coding/Opus/Content/scripts/sandbox.lua`
      then RESTART SERVER.  The Log tab, in this order: a Warn that `scripts/broken.lua` didn't load, at
      line 2; `line 1` to `line 50` from flood.lua and a Warn that it hit its limit of 50; hello.lua's
      hello; about a second later a Warn that `scripts/runaway.lua` is still running after 1 s, its time
      limit; and `nil nil nil nil nil` from sandbox.lua.  Then `Ran 3 script(s) ..., 2 with an error.`
      The Services tab says Lua is in trouble.  Conductor keeps running and STOP SERVER works.
- [ ] Take them back out, and RESTART SERVER is back to hello alone:
      `rm /opt/storage/Coding/Opus/Content/scripts/broken.lua /opt/storage/Coding/Opus/Content/scripts/flood.lua /opt/storage/Coding/Opus/Content/scripts/runaway.lua /opt/storage/Coding/Opus/Content/scripts/sandbox.lua`

## Parked

Nothing to run these on yet.

- [ ] From outside, when there's an outside machine: forward TCP 9997 and UDP 9998 on the router, run
      the client from a laptop on a phone hotspot with `--host 142.56.230.42`, and the Connections tab
      shows the outside address (with a Host name if its reverse DNS has one); blacklist it from the
      row's menu and the next try is closed at the door.
- [ ] The Windows build, whenever getting to that machine is less of a hassle.  The Windows code has
      never been compiled.
