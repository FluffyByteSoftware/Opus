<!--
File:       Opus/Documentation/LLM/design/gameclock.md
Component:  Documentation
Author:     Jacob Chacko
-->

# gameclock (the GameClock)

A lib crate and a server piece: `Conductor/dev/gameclock/`, the crate `conductor-gameclock`, so code says
`conductor_gameclock::start()`.  Jacob named it the GameClock, 2026-09-30.  He may call it "the heartbeat",
his MUD's word for the tick, but the code and the docs say the GameClock ("Opus is like a more grown up code
base").

It's the game loop.  It owns primlib's `World` and GameWorld's `Terrain` (the chunks in memory) and steps
them forward on a fixed beat.

## Where it stands

Built and tested on Linux: no warnings, its tests pass, and every run check passed (GameClock green on the
Services tab at about 240 cycles a minute, its thread near nothing on the CPU, a login leaving the late
count at 0, and a clean STOP SERVER and START SERVER).  Only housekeeping does anything yet (it takes in the
chunks GameWorld sends); the other four checks are empty, since nothing in the world moves and the protocol
has no input packet.  The last change, only housekeeping running until the ground is in, is waiting on its
check in TEST_CHECKLIST.html.

## Skeleton

```
gameclock/
├── Cargo.toml     depends on conductor-tools, conductor-primlib and conductor-gameworld
└── src/
    ├── lib.rs     start(), stop(), ready(); the GameClock's thread, the schedule, the tallies, the Warn
    └── checks.rs  the five checks, in order, each a function that gets the world and the terrain
```

## Decided

- **The beat** (Jacob's design, from an earlier go at this): a full cycle is **250 ms**, cut into **five
  checks of 50 ms**.  Each check touches its own group of objects and does whatever it needs to.
- **The rate is fixed in code**, not a setting (`CHECK_MS` and the length of `checks::ALL`).  Jacob: "from
  all the testing I did before anything faster is gonna be a problem.  Slower is fine but faster becomes
  bad."  There's nothing for it on the Settings tab.
- **The order**: input, AI, movement, broadcast, housekeeping.  What the players asked for comes in, the AI
  decides, everything moves, the positions go out (so a player is sent this cycle's, not last cycle's), and
  housekeeping last.  Jacob's best guess ("I have no idea what order they should go in"); it can move.
- **The schedule**: check *n* is due 50 ms × *n* after its cycle started, measured from the start and not
  from the end of the check before, so nothing drifts.  The wait between checks is `recv_timeout` on the
  stop channel, so it's the OS's own wait and STOP SERVER is heard at once.
- **A check that runs long** (Jacob's pick of two): the next check is late and runs as soon as it can.
  Nothing is skipped.  A cycle whose last check finishes past its 250 ms is **late**, and the next cycle
  starts straight away on a fresh schedule, instead of rushing through checks to make up the time.
- **What a late cycle says**: a Debug line (by how much, and the slowest check).  Only a cycle a full second
  or more over is a Warn, and at most one a minute, since every Warn rings the bell.
- **What it shows**: the Services tab's GameClock line, "Beating.  N cycles, M late.  The busiest spent X ms
  of its 250 in the checks.", then how many of the chunks around 0,0,0 are in, and while they aren't all
  in, that only housekeeping runs.  It checks in (`seen()`) every cycle.
- **The world and the terrain are only touched on the GameClock's thread**, so they have no lock.
  Networking and the web admin will reach the world through a mailbox the input check empties (not built:
  nothing sends yet).  The world is made fresh on every START SERVER; saving the copies on STOP SERVER and
  loading them back is in `design/primlib.md`.
- **The ground comes in first.**  On START SERVER the terrain asks GameWorld for every chunk within
  `view_chunks` (in `game.cfg`) of 0,0,0, where every player starts for now: 162 at the default of 4.
  Housekeeping takes in whatever has arrived, never waiting.  Once every one is in, `ready()` turns true
  (an Info line says so), and the launcher opens the door on it (`design/conductor-launcher.md`).  A run
  where one of them can't be had never turns ready.
- **Nothing acts before the ground is in** (Jacob: "I don't want NPCs acting while the server world isn't
  ready").  The GameClock beats from START SERVER as always, but until `ready()` only housekeeping runs;
  input, AI, movement and broadcast each get their 50 ms and do nothing with them.  The spawn system, when
  it goes in housekeeping, has to wait for `ready()` itself.
- **Argon2 stays on Security's own thread.**  On Jacob's earlier go, with that, a cycle never ran far over
  250 ms.

## Open

- What each check does, as the pieces come: the input mailbox and the input packet (a protocol version
  bump), a brain component for the AI, movement into `Transform`, the broadcast (only what each player may
  see), the spawn system in housekeeping.
- Whether a check's group of objects is picked by its components (the AI check runs over everything with a
  brain), which is how an ECS usually does it.
- The Tick evaluator tab under GAME MANAGEMENT: what it shows, and the numbers the GameClock keeps for it
  (TODO.md).
