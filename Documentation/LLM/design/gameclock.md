<!--
File:       Opus/Documentation/LLM/design/gameclock.md
Component:  Documentation
Author:     Jacob Chacko
-->

# gameclock (the heartbeat)

A lib crate and a server piece: `Conductor/dev/gameclock/`, the crate `conductor-gameclock`, and code calls it
`conductor_heartbeat` (Jacob's names, 2026-09-30).  It's the only crate whose name in code isn't its crate name
with an underscore; the `[lib] name` in its `Cargo.toml` does that.  So `cargo test -p conductor-gameclock`,
and `conductor_heartbeat::start()` in the launcher.

It's the game loop.  It owns primlib's `World` and steps it forward on a fixed beat.

## Where it stands

Written 2026-09-30, **not built yet**.  The beat, the schedule, the tallies, the Services line, start and
stop.  The five checks are empty: nothing in the world moves yet, and the protocol has no input packet.

## Skeleton

```
gameclock/
├── Cargo.toml     depends on conductor-tools and conductor-primlib; [lib] name = "conductor_heartbeat"
└── src/
    ├── lib.rs     start(), stop(); the heartbeat's thread, the schedule, the tallies, the Warn
    └── checks.rs  the five checks, in order, each a function that gets the world
```

## Decided

- **The beat** (Jacob's design, from an earlier go at this, 2026-09-30): a full cycle is **250 ms**, cut into
  **five checks of 50 ms**.  Each check touches its own group of objects and does whatever it needs to.
- **The rate is fixed in code**, not a setting (`CHECK_MS` and the length of `checks::ALL`).  Jacob: "from
  all the testing I did before anything faster is gonna be a problem.  Slower is fine but faster becomes
  bad."  So CLAUDE.md's waiting "tick rate" line is filled in, and there's nothing on the Settings tab.
- **The order**: input, AI, movement, broadcast, housekeeping.  What the players asked for comes in, the AI
  decides, everything moves, the positions go out (so a player is sent this cycle's, not last cycle's), and
  housekeeping last.  Jacob's best guess ("I have no idea what order they should go in"); it can move.
- **The schedule**: check *n* is due 50 ms × *n* after its cycle started, measured from the start and not
  from the end of the check before, so nothing drifts.  The wait between checks is `recv_timeout` on the
  stop channel, so it's the OS's own wait and STOP SERVER is heard at once.
- **A check that runs long** (Jacob's pick, 2026-09-30, of two): the next check is late and runs as soon
  as it can, and the cycle catches up where it can.  Nothing is skipped.  A cycle whose last check finishes
  past its 250 ms is **late**, and the next cycle starts straight away on a fresh schedule, instead of
  rushing through checks to make up the time.
- **What a late cycle says**: a Debug line (by how much, and the slowest check).  Only a cycle a full second
  or more over is a Warn, and at most one a minute, since every Warn rings the bell.
- **What it shows**: the Services tab's Heartbeat line, "Beating.  N cycles, M late.  The busiest spent X ms
  of its 250 in the checks."  It checks in (`seen()`) every cycle.  A Tick evaluator tab under GAME
  MANAGEMENT is for later (TODO.md).
- **The world is only touched on the heartbeat's thread**, so it has no lock.  Networking and the web admin
  will reach it through a mailbox the input check empties (not built: nothing sends yet).
- **The world is made fresh on every START SERVER.**  Saving the copies on STOP SERVER and loading them back
  is in `design/primlib.md`.
- **Its place in the launcher**: started after Lua and before networking, so there's a world before there
  are players; stopped after networking, so nobody is left in it.
- **Argon2 stays on its own thread** (Security's worker, as it always was).  On Jacob's earlier go, with
  that, a cycle never ran far over 250 ms.

## Open

- What each check does, as the pieces come: the input mailbox and the input packet (a protocol version
  bump), a brain component for the AI, movement into `Transform`, the broadcast (only what each player may
  see), the spawn system in housekeeping.
- Whether a check's group of objects is picked by its components (the AI check runs over everything with a
  brain), which is how an ECS usually does it.
- The Tick evaluator tab: what it shows, and the numbers the heartbeat keeps for it (TODO.md).
