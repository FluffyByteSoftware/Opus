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
saves the world; input brings players' characters in and out of the world through the mailbox; broadcast
sends out the chat (2026-10-02, "Chat" below), answers `/who` and, since session 9, tells each player what
they see of the world ("The view" below); since session 10 input judges every move a player's client sends
and movement stops a walker gone quiet ("Movement" below, built and tested in session 12); AI is empty.

**Ready for the spawn** (2026-10-01, built and tested, every check passed): the mailbox, the players' list,
and the world save.  See "Players and the world save" below.

## Skeleton

```
gameclock/
├── Cargo.toml     depends on conductor-tools, conductor-primlib, conductor-gameworld and conductor-accounts
└── src/
    ├── chat.rs    the chat's mailbox: chat(line), set_chat_sender(send); broadcast(), for the broadcast check
    ├── movement.rs players walking their characters (protocol version 15): moved(), the mailbox;
    │                WALK_BLOCKS_PER_SECOND, turn_degrees_per_second(); Moved, PullBack; Movement (each
    │                character's track), take_moves() for input, stop_the_quiet() for movement
    ├── ground.rs  the ground following the players: follow(), for housekeeping; Ground
    ├── view.rs    what each player sees of the world (protocol version 14): next_object_number(),
    │                ask_about(), set_view_sender(); Motion, Hydrate, News; View (what each player's client
    │                knows), its news() for the broadcast check
    ├── who.rs     /who's mailbox: who(WhoAsked), set_who_sender(send), Standing; answer(), for
    │                the broadcast check; block_of()
    ├── lib.rs     start(), stop(), ready(); enter() and leave() handed on from players.rs, chat() and
    │                set_chat_sender() from chat.rs, who() and set_who_sender() from who.rs; Game (the world, the
    │                terrain, the players, the world save, the saves on their way); the GameClock's thread, the
    │                schedule, the tallies, the Warn; save_world()
    ├── checks.rs  the five checks, in order, each a function that gets the Game
    ├── players.rs the mailbox (Note: Enter, Leave; enter(), leave()); the "saving" marks (saving(),
    │                wait_until_saved()); Players, the characters in the world by their row's id: take_notes(),
    │                snapshot(), standing(), count()
    └── saving.rs  world_save_every(); WorldSave (when the next is due); Writes (the saves on their way to the
                     database, looked at every housekeeping; send() and send_leaving())
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
  `view_chunks` (in `game.cfg`) of 0,0,0, where every player starts for now: 891 at the default of 4, nine
  by nine and eleven rows.
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
  Networking calls them (2026-10-01): Protogame's `enter()` when a player picks a character with
  UserPressPlay, and the book's `leave()` whenever that player's session ends, however it ends
  (`design/conductor-networking.md`, "The spawn").
- **The slow part is done before the note.**  Whoever asks reads the row, reads the save through
  lua-parser and makes the blueprint with `character_from_save()` on its own thread; the GameClock only
  spawns it.
- **The players' list**: each character in the world by its row's id, to its entity.  A character asked in
  while it's already there is a Warn, and the one there stands.
- **Leaving takes the copy out**: saved, then despawned at once.  Asked out while not in is a Debug line.
- **The "saving" mark** (2026-10-01, Jacob: "do we have any way to force a save on the connection being
  kicked before the new one pops in?").  `leave()` marks the character on the caller's thread, before its
  note goes in the mailbox, so there's no gap; the mark comes off when its leaving save has its answer
  (landed, or failed with the Error), at once if it wasn't in the world, and all of them on START SERVER and
  when the GameClock stops.  Only a leaving save takes marks off: a world save sent just before has the
  old copy.  `saving(id)` asks, and `wait_until_saved(id, limit)` waits on a Condvar (no polling) for a
  login thread; the GameClock itself never waits on it.  Networking uses both
  (`design/conductor-networking.md`, "The spawn").
- **The world save**: every `world_save_seconds` (`game.cfg`, 150 by default, 30 to 1800), counted from
  the moment the ground is in, housekeeping copies every player's character as it stands (`Save::of()`
  and the position), in one cycle, so it's the world at one moment.  The copies go to
  `conductor_accounts::characters::save_all()`, which turns each into Lua text and writes its row on
  Archivist's thread, one after another, in one transaction: the whole world save lands or none of it, and
  the GameClock never waits.  The next is counted from the save before.
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

## Chat (2026-10-02)

Jacob: "we send a packet to all users including the person who sent the message on the next "chat"
GameClock tick that carries chat (which should be every beat)".  So the chat goes out from the broadcast
check, once a 250 ms cycle.  Networking reads a player's `/chat` and leaves the finished line
(`[Chat] Jacob: Yo yo yo!`) with `chat()`, which comes straight back; the broadcast check takes every line
left since the cycle before, in order, and hands them to the function in its slot (`set_chat_sender()`,
filled by `conductor_player_commands::wire()` from the launcher, after the GameClock starts), which sends
them to everybody in the world.  The GameClock knows nobody's address, and can't call networking itself:
networking depends on the GameClock, and Rust won't build two crates that each need the other.  A plain
function handed over solves it; the positions will go out the same way.
The mailbox is open only while the GameClock runs, and STOP SERVER drops what's in it.  A cycle with no
chat costs a lock.  The sending is one UDP send per player in the world per cycle that has chat, on the
GameClock's thread; a guess, not measured.  `design/conductor-networking.md` has the rest.

## /who (2026-10-02; every /who since 2026-10-03)

The broadcast check answers `/who` too: networking leaves each ask (who asked, from where, its ask
number) with `who()`, and the check reads every player's character's `ShortName`, the block its
`Transform` stands in (each axis rounded down) and how long since it came into the world (`standing()`)
once, however many asked that cycle, and hands them with each ask to the commands crate's `send()`, in the
slot `wire()` fills (`set_who_sender()`).  Same shape as the chat, same reason.  `Players` notes the
`Instant` each character was spawned beside its entity, and `standing()` hands them back in that order,
the one in longest first (Jacob: "Oldest log in goes at the top, newest at the bottom").  Until
2026-10-03 this was `/who list` only, and a plain `/who` was answered from networking's book.
`design/conductor-networking.md` has the rest.

## The view (2026-10-03, session 9)

Jacob: "The server will be the authority, always on where the object actually is in the world.  The client
is just a dumb renderer."  `design/ensemble-world.md` ("The player in the world") has his answers;
PROTOCOL.md ("The world's objects") the packets.  The GameClock's half, `view.rs`:

- **Every object has a number**, handed out by `enter()` from a counter (`next_object_number()`, from 1,
  never 0, never used again in the run), carried on the note into the world with the character's uuid, and
  kept on the players' list.  `enter()` hands it back, so networking can put it in the
  CharacterEnteredWorld before the character is spawned.
- **The broadcast check** reads every object once a cycle: where it is, which way it faces, and whether that
  changed since the cycle before.  For each player in the world it works out what their client is to be
  told: every object within `view_chunks` of their column it hasn't had whole (a Hydrate), every one it has
  that moved, every one it has that's out of view or out of the world (gone), and every fourth cycle, once a
  second, a roll call of everything it's believed to have.  `View` keeps what each player's client was sent,
  by their character's row id, and forgets a player whose character has left.
- **What the client asks about** (`ask_about()`, a mailbox like `/who`'s) is forgotten in `View` at the next
  broadcast, so it's sent whole again if it's in view, or sent gone if it isn't.
- **It's sent through a slot**, `set_view_sender()`, filled by networking as it starts; the GameClock only
  hands over plain data (`News`).
- **Velocity is the `Transform`'s** since movement (session 10): a walking character's goes out with it.
  A player isn't sent their own character's moves, and each player's news carries their pull-back and,
  when it changes, the column of chunks their character is in (for which chunks networking lets them
  have).
- **The cost**: every player against every object, every cycle, O(players x objects).  **Measured
  2026-10-03 on Jacob's machine: 24.28 ms for 500 players in sight of each other, every one moved**
  (`view_of_five_hundred`, `#[ignore]`, `--release`), about half the broadcast's 50 ms.  That's the worst
  case (250,000 pairs, every one sending a move); it grows with the square, so about 700 players all in one
  place would fill the check.  If it ever matters, the first thing to try is keeping the objects by their
  column of chunks, so each player only looks at the squares near them instead of at everybody (TODO.md).

## Movement (2026-10-03, session 10; built and tested in session 12)

Jacob's answers are in `design/ensemble-world.md` ("Movement"), in his words; PROTOCOL.md ("Movement") has
the packets.  EverQuest's way: a player's client walks its own character and says where it went, and the
server takes each move or pulls it back to its last good spot.  His earlier server "kept rubber banding",
so `movement.rs` is built against the four usual reasons for it (its header has them).  The GameClock's
half:

- **The mailbox.**  Networking hands each PlayerMoved to `moved()`, stamped with the `Instant` it came in,
  and the input check takes them all once a cycle, in the order they came (`take_moves()`).  Once a cycle
  and not every 50 ms (Jacob asked; the reply in session 10): what's taken only goes out in the broadcast,
  and the player walking sees their own move at once on their own screen.
- **The last good spot is the `Transform`.**  `Movement` keeps the rest for each character (its track):
  the last move number taken, when the last good spot was, the last pull-back's number and whether moves
  wait for it, the last spot it stood on something, when it left the ground, its Warns.
- **The check** (`verdict()`), against the time since the last good spot (2 seconds at most):
  `WALK_BLOCKS_PER_SECOND` (4, fixed in code) along the ground plus anything up past a free step of one
  block, with a block let go silently; up to `movement_tolerance_blocks` (`game.cfg`, 16) more is taken
  with a Warn, past it pulled back with a Warn (Jacob: "the being 1-16 blocks past a point expected to be
  at"); a turn more than 90 degrees past `turn_degrees_per_second` (`player.cfg`, 450) is pulled back with
  a Warn ("if your rotation is more than 90 deg off that should flag a warning"); a place that isn't a
  number, ground the server hasn't loaded, the middle of the collider inside a block, or walking further
  into a character standing still is pulled back with a Debug line; and 2 seconds in the air without
  falling a block goes back to the last spot it stood on.  Down is free.
- **The Warns** ring the bell, so they're at most one a minute a character, the next one counting the ones
  held back.
- **A pull-back** puts the `Transform` back on the spot, standing still, numbers it, and leaves it for the
  broadcast (`take_pull_backs()`), which puts it in that player's news; networking sends a
  MoveCorrection.  Moves that say an older pull-back are dropped and the pull-back sent again.
- **A taken move's velocity** is held to walking along the ground (and 60 blocks a second falling): it's
  only what everybody else's screens carry the character along by.
- **The movement check** stops a walker whose client has said nothing for 2 seconds (`stop_the_quiet()`).
- **The ground follows the players** (`ground.rs`, in housekeeping once `ready()`): a character coming in
  or walking into another column has the chunks `FOLLOW_CHUNKS` (2) each way asked for; every 16 cycles,
  if anybody moved column, the chunks more than `KEEP_CHUNKS` (3) from every player, and outside the spawn
  point's `view_chunks`, are let go.  The server only needs the ground near a character to check its moves
  (what a client sees comes from GameWorld's squeezed copies), so about 18 MB a player alone instead of
  200 MB for a whole view.  `Terrain` remembers what's on its way and what couldn't be had.
- **Its cost**: a guess, not measured.  Every move is checked against every player standing still, so a
  cycle's checks grow with players times moves.  If it shows up in a late cycle, a timing test comes first.

## Gravity (session 13, 2026-10-03; being talked through, not built)

Jacob, opening session 13: **"set up gravity on the server now (Conductor) so that clients cannot cheat
and fly around"**.  Gravity alone is the server's; walking stays the client's (session 12's "gravity
sorry").

**How a client flies today** (round one's rules, read from `movement.rs`):

- **The hanging rule only asks for one block.**  It goes back to the first move off the ground: 2 seconds
  later the character has to be a block under where it left.  After that it's never asked again, so a
  client walks off a ledge, drops one block, and flies anywhere under that height for as long as it likes.
- **Every move gets a free block up.**  The step up is counted from the last move, not from the ground, so
  in the air a client climbs a block a move, ten a second at a move every 100 ms, and anything it lands
  standing on in under 2 seconds (a cliff top) is taken.
- **Up past the step counts as walking**, so a client walks straight up a wall at 4 blocks a second, and
  up to 16 blocks past that is taken with a Warn.
- **Nothing drops anybody on the server.**  The hanging rule only runs when a move comes in, so a client
  that goes quiet in the air hangs there for everybody else, and so would an NPC, or a player whose
  ground is blown away.

**The first plan**, put to him (kept for why; his answers below turned it):

- **The server drops everything**, in the movement check, every cycle: an object with a `Transform` and a
  `Collider` standing on nothing speeds up downward and lands on the first block under its collider.  A
  new file, `gravity.rs`, keeps each fall (how fast, where it left the ground, where it was a cycle
  before).  The fall is in the `Transform`'s velocity, so everybody watching sees it fall through
  ObjectsMoved.  Ground the server hasn't got still counts as something to stand on.
- **How fast**: a guess to pick, fixed in code like the walk: about 32 blocks a second squared (near
  Minecraft's, from memory, not looked up) or the real 9.8, either way no faster than `FASTEST_FALL`, 60.
- **The player's own client** isn't told when the server drops its character: it falls on its own
  (Ensemble's round two will fall at the same numbers).  Its moves are held to the fall instead:
- **A move that ends in the air** may be no higher than the server had the character a cycle before, plus
  a block.  Higher is flying: pulled back to where the server has it.  No jumping until jumping comes.
- **A move that ends standing** may be no more than a block above where the character last stood, plus
  the flat distance it walked (smooth voxels' 45-degree slope, brought in early), and up no longer counts
  as walking.  Steeper is climbing a wall: pulled back.
- **The hanging rule goes**, and so does "back to the last spot it stood on": the server's own fall
  takes their place.
- **No packet changes**, so the protocol stays at 17; PROTOCOL.md's "Movement" gets the new rules.
- **The test client** gets `--fly BLOCKS`, a move that says it went that many blocks straight up, to be
  pulled back.
- **The cost**: a footing check for each object every cycle, a guess of nothing much until an `#[ignore]`
  timing test says (500 falling).

**His answers to the first plan** (2026-10-03):

- **Flying has to be possible**: "hold on we need to make sure we add a means for players to fly because
  I totally want flight magic and flying creatures in this".  So gravity is something an object has, and
  something it can be let off.
- **How fast**: "Minecrafts speed but you need to look it up".  Looked up in Minestom, an open-source
  Minecraft server (`Minestom/Minestom` on GitHub: `registry/RegistryData.java` has every entity's
  `acceleration` 0.08 and `drag` 0.02 as the defaults, and `collision/PhysicsUtils.java`'s
  `updateVelocity()` runs `y = (y - gravity) * (1 - drag)` once a tick), at Minecraft's 20 ticks a second:
  each tick a falling thing's downward speed goes up 0.08 blocks a tick and is then cut to 98 %, so it
  tops out at 3.92 blocks a tick, 78.4 a second.  That's the emulator's, said as such; the Minecraft
  wiki doesn't reach the session, and a web search's summary of it says the same numbers.  Our check is
  50 ms, a Minecraft tick exactly, so a cycle runs that step five times.  `FASTEST_FALL` (60) goes up to
  78.4 to match.
- **Whether the client hears the server drop it**: "yes we will need a protocol revision for it".  Asked
  back which revision he means (below).
- **Caught flying**: a Warn "only if they don't have a flying effect on them or some legitimate means to
  be flying it would be like speed hacking".
- **The 45-degree slope**: "now".
- **Who falls**: "every GameObject in the game code (I think our LUA) that has a gravity component;
  actors will all have gravity as a component or maybe we call it mass?  PHysics component?  I dont' know
  yet".  The templates are still Rust today (`primlib/src/gameobject.rs`); Lua templates are primlib's
  part two, not built, and the component would go to Lua with the rest.
- **Building**: "lets discuss if we need to".

**The second shape**, put to him, not settled:

- **A component**, in primlib, on the Living template (so every actor has it), holding whether gravity
  pulls the object and whether it may fly right now (a flying creature's blueprint says yes; a flying
  effect, once there are effects, says yes while it lasts).  Its name is his to pick: `Physics`, `Mass`
  or `Gravity`.  An object that may fly is left alone by gravity and its client's moves aren't held to a
  fall; no Warn.  Its speed in the air is walking's until speed modifiers come.
- **The protocol, version 18**: CharacterEnteredWorld ends with whether the character may fly and the
  fall's two numbers (as it already ends with the walk and the turn), and a new packet tells a client
  when its character's flight changes.  Ensemble and Soundcheck have to read 18 (a Unity round).
- **Turning flight on to test it** with no spells yet: open.

## Open

- What each check does, as the pieces come: a brain component for the AI, NPCs walking in movement, the
  spawn system in housekeeping.  Players' moves go into `Transform` in input since session 10 (above).
- Whether a check's group of objects is picked by its components (the AI check runs over everything with a
  brain), which is how an ECS usually does it.
- The Tick evaluator tab under GAME MANAGEMENT: what it shows, and the numbers the GameClock keeps for it
  (TODO.md).
