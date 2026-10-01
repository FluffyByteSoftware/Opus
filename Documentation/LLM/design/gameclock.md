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
count at 0, and a clean STOP SERVER and START SERVER).  Housekeeping takes in the chunks GameWorld sends and
saves the world; input brings players' characters in and out of the world through the mailbox; AI, movement
and broadcast are empty, since nothing in the world moves and the protocol has no input packet.

**Ready for the spawn** (2026-10-01, built and tested, every check passed): the mailbox, the players' list,
and the world save.  See "Players and the world save" below.

## Skeleton

```
gameclock/
├── Cargo.toml     depends on conductor-tools, conductor-primlib, conductor-gameworld and conductor-accounts
└── src/
    ├── lib.rs     start(), stop(), ready(); enter() and leave() handed on from players.rs; Game (the world, the
    │                terrain, the players, the world save, the saves on their way); the GameClock's thread, the
    │                schedule, the tallies, the Warn; save_world()
    ├── checks.rs  the five checks, in order, each a function that gets the Game
    ├── players.rs the mailbox (Note: Enter, Leave; enter(), leave()); Players, the characters in the world by
    │                their row's id: take_notes(), snapshot(), count()
    └── saving.rs  world_save_every(); WorldSave (when the next is due); Writes (the saves on their way to the
                     database, looked at every housekeeping)
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
  Networking reaches the world through a mailbox the input check empties (below).  The world is made fresh
  on every START SERVER; saving primlib's other copies on STOP SERVER and loading them back is in
  `design/primlib.md`.
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

## Players and the world save

Jacob's answers, 2026-09-30 and 2026-10-01, preparing the game library for the spawn.  `design/primlib.md`,
"The character and saving", has his words.

- **The mailbox** (`players.rs`).  `conductor_gameclock::enter(blueprint)` asks for a player's character to
  be put in the world, `leave(character_id)` for one to be taken out; both leave a note and come straight
  back, and say so if the GameClock isn't running.  `enter()` turns away a blueprint without a
  `PlayerCharacter` with a row behind it (an id above 0).  The input check empties the mailbox every cycle.
  Nothing calls them yet: the spawn's network side is networking's (Jacob's map in STATUS.md).
- **The slow part is done before the note.**  Whoever asks reads the row, reads the save through
  lua-parser and makes the blueprint with `character_from_save()` on its own thread; the GameClock only
  spawns it.
- **The players' list**: each character in the world by its row's id, to its entity.  A character asked in
  while it's already there is a Warn, and the one there stands.
- **Leaving takes the copy out**: saved, then despawned at once.  Asked out while not in is a Debug line.
- **The world save**: every `world_save_seconds` (`game.cfg`, 150 by default, 30 to 1800), counted from
  the moment the ground is in, housekeeping copies every player's character as it stands (`Save::of()`
  and the position), in one cycle, so it's the world at one moment.  The copies go to
  `conductor_accounts::characters::save_all()`, which turns each into Lua text and writes its row on
  Archivist's thread, one after another, in one transaction: the whole world save lands or none of it, and
  the GameClock never waits.  The next is counted from when the last was due.
- **The saves on their way** (`Writes`): a write that fails only says so in its `Pending`, so the GameClock
  keeps them and looks at each in housekeeping, never waiting.  Written is a Debug line; fewer rows than
  characters is a Warn (a row deleted while its character was in the world); a failure is an Error.
- **STOP SERVER**: the GameClock's thread, once it stops beating, saves the world one last time and waits up
  to 10 seconds for its saves to land, so the log says whether they did.  That's after the game loop, so
  nothing in it waits.  The launcher stops the GameClock before Archivist, so Archivist is there to write.
  A `leave()` that arrives after the mailbox closes is turned away, and its character is saved by this.
- **The Services tab** adds how many players are in the world and how the world saves are going.
- **Its cost**: the snapshot copies plain data; a timing test (`snapshot_of_ten_thousand`, `#[ignore]`,
  run with `--release`) says how long 10,000 take.  **Measured 2026-10-01 on Jacob's machine: 32.88 ms for
  10,000 characters**, about 3.3 microseconds each, so a few hundred players is about a millisecond.  The
  guess beforehand was a few milliseconds for 10,000; it's ten times that, and 10,000 copies would take two
  thirds of housekeeping's 50 ms.  Most of it is likely the small allocations in `Save::of()` (every field
  name and template name copied as a `String`), which is where to look first if NPC copies join the world
  save in their thousands.  If it ever grows too big for one cycle, spreading it over several loses the one
  moment, so it comes back to Jacob.

## Open

- What each check does, as the pieces come: the input mailbox and the input packet (a protocol version
  bump), a brain component for the AI, movement into `Transform`, the broadcast (only what each player may
  see), the spawn system in housekeeping.
- Whether a check's group of objects is picked by its components (the AI check runs over everything with a
  brain), which is how an ECS usually does it.
- The Tick evaluator tab under GAME MANAGEMENT: what it shows, and the numbers the GameClock keeps for it
  (TODO.md).
