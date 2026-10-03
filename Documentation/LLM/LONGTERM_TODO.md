<!--
File:       Opus/Documentation/LLM/LONGTERM_TODO.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Long-term TODO

The big features: a run of sessions each, and waiting less on any one piece than on the game existing
around them.  TODO.md holds what's deferred and the small ideas; this holds what Opus is going to be.
Entries are added as they come up and stay until they're built.  Nothing here is ordered; Jacob picks.

## The scripting language

Content (items, quests, NPCs, whatever the world is made of) is written in **Lua 5.4**, embedded through
`mlua` with Lua built in (Jacob: "Lua is a fine language for this actually").  The first step is in
(`lua-parser`, `design/lua-parser.md`): every script under `Content/scripts/` runs once on START SERVER,
locked down, with the log the only thing it can call.  Templates and blueprints exist in Rust
(`conductor-primlib`); writing them as scripts is primlib's part 2 (`design/primlib.md`).

**Behaviour**: an object gets a behaviour script added to it that controls what it does.  Jacob would
like GOAP (goal-oriented action planning) for the thinking; not settled.  And maybe "just develop like
Unity -- I attach a script to an object and it fires off behaviour, or comes prepackaged with behaviour to
manipulate the object."  He wants to think it over.

Still to settle:

- **What it looks like**: Jacob's NPC sample below, and an item and a quest in the same shape, in Lua,
  before any code.
- **Whether a script is attached to an object** the way a Unity component is, whether a template or
  blueprint comes with its behaviour attached, and how `Content/scripts/` is laid out for that.
- **Whether Ensemble sees any of it** (an item's description, say).  The protocol's question.
- **Hot loading**: no, for now.  Scripts load on START SERVER and again on RESTART SERVER.

**Jacob's first sample**, written before Lua was picked (MudOS / LPC in spirit).  A template is a named
set of components; `create` makes a blueprint from it and sets the values:

```
define template as "NPC"
{
	// adds the position X, Y, Z floats to this game object
	AddComponent(Position);
	// adds the rotation directions, X, Y, Z floats for this game object
	AddComponent(Rotation);
	// Adds the scalar values for X, Y, Z floats for this game object
	AddComponent(Scale);
	// Adds a "short name" to refer to this item as (this is usually lower case, examples goblin instead of
	// "a Goblin Warrior")
	AddComponent(ShortName);
	// Adds a "long name" with proper capitalization and title or unique name if necessary (ie: "a Goblin
	// Warrior named, Serah")
	AddComponent(LongName);
	// Adds a titles component like "the Cursed" this is an array with one value selected as CurrentTitle;
	AddComponent(Titles);
	AddComponent(Health);
	AddComponent(Endurance);
	AddComponent(Mana);
}

// Tell our script parser that we are creating a gameobject called goblin_a that is a template NPC (comes
// prefixed with these components)
create NPC goblin_a{
	// Spawns this at 0 0 0 facing 0 0 0
	Position.SetPosition(0, 0, 0);
	Rotation.SetRotation(0, 0, -90);
	Scale.SetScale(1,1,1);
	ShortName.Set("goblin");
	LongName.Set("goblin archer");
	Titles.Set("the plucky");
	Health.SetMax(2000);
	// No need to Health.Set it, it auto sets to max unless specified
	Endurance.SetMax(200);
	Mana.SetMax(2);
}
```

`spawn goblin_a x 100` makes a hundred goblins, each with its own values.

## The world

`conductor-gameworld` is part one: a seamless world `world_size` times 1024 blocks a side (16 to start), in
1 m blocks and chunks of 32 a side, eleven chunks tall (-32 to +319), cut into regions (a region was a zone
was a biome; Jacob split the three in session 11, below in "The world asleep and awake", not yet in code).  The first world is Alpha (flat) and Omega (hills).  `design/world.md` has it.  What's left, a session or more each:

- **Saving changed chunks** (part two): on STOP SERVER and with the one world save, every `world_save_seconds`
  (Jacob, 2026-10-03: one save for the characters and the ground; `save_minutes` dropped).
  Comes with the first thing that changes a block.
- **Sending chunks to a client**: only the ones near it, since the server decides what each client sees.
  A protocol change.  And how Ensemble gets `region.map`.
- **Loading around players who move**, instead of only around 0,0,0.
- **What a region does** beyond its name: what grows and what spawns there.
- **Blending one biome into the next** where two regions meet.
- **A bigger world** ("it may _grow_ later"): `world_size` went to 16 on 2026-10-01, twice as wide, and a
  change today remakes the world.  `region.map` keeps its size in its header, so growing one without
  remaking it is possible one day; keeping the digging across a change is in TODO.md.

## Smooth voxels (Jacob, 2026-10-03, session 11; stage 1 settled, not built)

`design/smooth-voxels.md` has all of it.  The ground's kinds drawn smooth (a density a terrain voxel, a
smooth mesher), structures as cubes, 7 Days to Die's split; voxels stay 1 m; the height stays -32 to +319;
caves, overhangs, catacombs and sewers from a 3D density; the ground changed both ways by blasts and
spells; materials blended with Shader Graph; the move check on the density, with a 45-degree slope; damage
sent as the changed chunks again; nothing drawn past the view.  Jacob: "Prepare a hand off to a new
conversation with yourself that we will begin implementation of this system."

## Opus.Treble (Jacob, 2026-10-03, session 11; named, not started)

"Opus.Treble -- its a unity powered application that's meant to place the voxels we've 'defined' down and
build prefabs out and save them to a .fbm (fluffybyte model) which can then be called on by conductor during
world generation to place points of interest like 7 days."  A building is a grid of voxels, made by hand in
Treble and stamped into the world by Conductor; it breaks apart like the ground.  The `.fbm` is a contract
of its own when it comes (`design/smooth-voxels.md`, "Open").  **A core Opus project**, its folder
`Treble/` at the top, `dev/` and `build/` like Ensemble's and Soundcheck's (Jacob, asked: "yes its a core
Opus project now").  Made when it's started, not before.

## The world asleep and awake (Jacob, 2026-10-03, session 11; being talked through)

Waits on NPCs (0.0.3 on WAYPOINTS.md) and on saving changed chunks (above).  Players wake the world two ways:
their client is sent the ground and the objects around them, and the server keeps the ground around them
loaded (`gameclock/src/ground.rs`, since session 10).  Jacob: "I want to have unique actors (NPCs) that can
also wake the world up around them.  There won't be many, maybe 2 or 3 in the world at a time.  They keep
it awake because they are the ones that might be moving from one city to another in game... Also if a
dragon is spawned in the world it should keep attacking until its dealt with."  So a waking NPC is the
server half of a player: the ground around it stays loaded and the world there goes on, with nobody
watching.

**His answers** (session 11):
- **The world sleeps where nothing wakes it, and catches up when it wakes**: "my original intent is that
  they go to sleep but when something heats the voxels up around them - they wake up and the "zone" can
  play catch up multiplying hte goblins if enough time has spread for reproduction to occur."
- **NPCs with an economy**: "while foxes and a deer might not matter -- the goblins will be producing
  things, mining, hunting, etc. so they will have their own controller determining their resource needs
  and what resources are available.  Then the goblins will have to figure out how to get those resources.
  That should naturally drive some conflict in the world too."  So a group of goblins has a controller
  above the single goblin's brain (the GOAP talk under "The scripting language" is the single goblin's).
- **How far round a waker stays awake**: "Its probably going to need to be a part of the Actor's scripts?
  Some way of defining it if its less than 2 or more than 2".  A value on the actor, set by its script;
  a player's is 2 chunks each way today.
- **A dragon is dealt with more than one way**: "You could slay it that's one way.  You could try to
  negotiate peace with it.  You could try to scare it."  Talking an NPC round and scaring one are each a
  system of their own, still to come.
- **2 or 3 wakers is how many he expects**, not a limit the server holds to.  Each costs about what a
  player's ground does on the server (about 18 MB at 2 chunks each way, a guess, not measured).
- **What a dragon does while nobody's there stays done**: "The damage happens that's the point.  The
  dragon is an existential threat!"  So the ground it burns is saved, which is why this waits on saving
  changed chunks.

**His answers** (session 11, the second round):
- **Cold, warm and hot, a chunk at a time**: "When a unique actor gets near a voxel chunk it warms and then
  gets hot based on the actors proximity to it.  The hot state means the actor is in the chunk, the warm is
  that the actor is nearby.  IN a warm state we'll have the system do "catch up" ticks calculating the
  position of everything where it should be as if things had never gone to sleep.  Then when its hot this
  should be ready to go for the player."  So a cold chunk is asleep; warm is the catching up, done ahead of
  the actor; hot is live.
- **A region, a biome and a zone are three things** (until now they were one, `design/world.md`,
  "Regions"; the code and `region.map` still have them as one):
  - **A region** "is a chunk of the map -- mostly used for cartography."
  - **A biome** "is going to be a distinct area marked by what type of terrain appears there, and affects
    the "animal" creatures that spawn there.  An example is an ElfForest biome.  It will effect what types
    of trees or vegetation grow there as well.  The buildings are all separate and based on which species
    constructed them."
  - **A zone** "is like a region but for the server to use to break the world up into its own management
    areas.  Zones will be groupings of chunks within squared areas of the overall world.  They're also going
    to be given dynamic names based on thec ontents, an area that contains mostly elven forest might be
    called "the Elven Forest of Gilai"".  "A zone is just how we group a set of chunks together to say:
    "This region of space has this NPC manager on it".  NPCs can leave their zones but will be treated as
    "guests" by other zone managers."
  - "most of this is going to be proceedurally generated at the start."
- **The ground isn't mined**: "we're only going to have voxels being destroyed by combat and effects.  You
  won't mine the chunks of the world awway."  So catching up changes the NPCs and their stores, not the
  ground.  And: "In fact we're gonna change this section to be about smoothing out our voxel world" (TODO.md,
  "Fewer voxels a column, EQ Next's style").

Still open: how far warm reaches and how far hot reaches, and whether a player's view has to be hot; what
catches up when a chunk warms (the NPCs in it, or its whole zone's manager); a zone's size; whether the
zone manager is the goblins' controller or above it; whether a goblin party out for resources wakes the
world; whether a dragon dealt with stops waking it.

## Soundcheck, the patcher, and a certificate for every client

**Opus.Soundcheck** is the launcher (Jacob's name).  **Started 2026-10-02, and the patcher half is built**:
`design/soundcheck.md` has it (C# on .NET 10 with Avalonia; the login moved out of Ensemble into it; at
start, a check of every file of the installed client against the manifest in the web folder, patched a
file at a time; PLAY is a second login and starts Ensemble with the ticket; admin mode publishes a build
into the web folder), and what's left of it is in TODO.md under "Soundcheck".  What stays here is the
certificate: one day Soundcheck
gives the client a certificate, and the server turns away any connection without a valid one ("then both
the client should be able to trust the server and vice-versa").  Mutual TLS: today only the server shows a
certificate (`with_no_client_auth()` in `tls.rs`), and a player proves who they are with the password.

What it buys: anything that isn't our client is turned away in the TLS handshake, before it gets near
Security's line, so a bot throwing logins at the door never costs a hash.  What it doesn't: a changed
client still has its certificate, so it keeps out strangers, not cheaters (the server deciding what each
client sees handles those).

Open when it opens:

- **One certificate per client**, so one can be taken back from one player.  That needs our own
  certificate authority (a key that signs the clients' certificates, which the server trusts), something
  to do the signing (Soundcheck asking the server, most likely), and a ban list of certificates.  rustls
  checks certificates but doesn't make them; making them in Rust is a crate (`rcgen`, Jacob's call), or
  openssl by hand.
- **How it proves who's asking** before it hands a certificate out: the account's password, most likely,
  so a certificate belongs to an account, not just an install.
- **The test client** gets one too, and a `--client-cert`.
- **PROTOCOL.md**: the handshake asks for the client's certificate.  Whether that bumps the version (it
  happens before the Hello, so an old client fails in the handshake, not on the version).

## A program of its own for account management

Jacob: "eventually we will make a separate account management program I think for running the
modifications and updating the database while its running."  Today the Accounts tab's changes wait in
Security's line with the logins; a separate program would do its own hashing and write to the database
beside a running Conductor.

Open when it opens: its name (Jacob names a new piece before it's made), how it and a running Conductor
agree on an account that's in the world while it's changed, and whether the Accounts tab stays.
