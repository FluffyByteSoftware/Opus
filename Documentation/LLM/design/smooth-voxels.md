<!--
File:       Opus/Documentation/LLM/design/smooth-voxels.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Smooth voxels

**Stage 1 is settled** (the discussion chat's summary, brought back by Jacob in session 11, 2026-10-03,
below under "Stage 1, as settled"; the mesher, the map, the density and the kinds in session 12).
**Being built a step at a time** (under "As written" at the end of stage 1).  Jacob: "Prepare a hand off to a new conversation
with yourself that we will begin implementation of this system."  The brief the discussion chat was given
is kept after it, as it was, so what it knew is on record.

## Stage 1, as settled (the discussion chat, 2026-10-03)

### Settled

- **A voxel stays 1 m.**  A 0.5 m voxel was put to him and turned round: "okay voxels are the same 1x1x1
  cube."  A player is still about 2 x 1 x 1.
- **Soft ground, crisp structures, 7 Days to Die's split**: "Crisp sharp edges and rounded ones :S Like nice
  soft hills contrasted by jagged cliff mountains."  **Cliffs are ground, not blocks**: "cliffs are terrain
  -- smooth mesher."  So the **terrain kinds** (dirt, stone and the like) are drawn by a smooth mesher and
  the **structure kinds** (brick, plank and the like) by the cube mesher there is now, both out of the same
  chunks.  No dual contouring: a cliff's edge is rounded over about a voxel, which he took after seeing
  7DTD's ground.
- **A terrain voxel gets a density**: its kind as now, and how solid it is.  The surface is drawn where the
  density crosses halfway.  A structure voxel has none (it's a whole cube).
- **The height and the depth stay**: -32 to +319, BEDROCK at -31 and -32.  A floor at -5 was put and turned
  round: "I do want you to be able to go down below for like tombs and stuff... but I think now the
  majority of the world is just stone and that will save on chunk space."  A chunk of untouched stone costs
  nothing (made again when it's needed, and it squeezes to nearly nothing).
- **Caves, overhangs and places underground**: "Yes I do."  "you walk to a graveyard and there's like a
  tunnel and shaft that leads down into the catacombs.  SO its under the city sorta.  There's also sewers a
  classic fantasy location for rogues."  One way in each: "there would be a single entry way for them."  So
  the world is made from a **3D density** (a height for the ground and carving in three dimensions), not a
  height a column.
- **The server makes all of it, and the ground changes both ways while the game runs**: "the server does
  during world generation and we may have spells that can grow mountains."  No ground shaped by hand.  A
  blast lowers the density in a sphere (strongest in the middle, fading out); a spell that grows raises
  it.  A changed chunk is saved whole, as now.
- **Buildings are voxels placed by hand in Treble, and stamped into the world by Conductor**: "Opus.Treble
  -- its a unity powered application that's meant to place the voxels we've 'defined' down and build
  prefabs out and save them to a .fbm (fluffybyte model) which can then be called on by conductor during
  world generation to place points of interest like 7 days."  A `.fbm` is a grid of voxels, 7DTD's way:
  "no reason to reinvent the wheel unless you can do ti better."  And: "we're gonna build prefabs to
  represent the 'building' components but they break apart like a voxel in terrain."  So a building lives
  in the same chunks as the ground, and damage carves it like anything else; there's no separate prefab
  mesh.  The catacombs and the sewers are built this way, in hollows in the stone.
- **Materials blend with Shader Graph**: "Shader Graph is fine."  The first step past "no shader of ours"
  (`design/ensemble-world.md`).  The shader does one fixed fade, about a voxel, wherever two kinds meet.
  **How wide a change is, is the generator's**: it hands out kinds over a strip of 1 to 5 voxels, soft
  (grass to dirt, about 5) or hard (soil to rock, 1 or 2 at random).  Jacob: "probably three voxels? so I'd
  say ~1 to ~5 voxels depending on how sharp the change is?" and "the terrain generator should know when to
  use 5 voxels and when to use random 1 to 2."  The exact rule was left to the session ("trust your
  instinct on this one"): a start, to tune by eye.  The same rule does a biome's edge, in place of the step
  between Alpha and Omega.
- **The move check**: the density sampled at the feet, the waist and the head for "inside the ground"; a
  "ground height at x, z, looking down from y" that finds the crossing under the feet and goes between the
  two densities.  A step up of 1 m is free.  **A slope**: "you can't walk up anything steeper than 45
  degrees."
- **Damage over the wire**: the changed chunks are sent again.  "a for now, I don't know."  Sending the edit
  itself (middle, radius, strength) is for later.
- **No distance drawn**: "its a fixed camera so not far 8 chunks is probably far enough and the skybox will
  take care of the rest of that."  Nothing past the chunks in view: no far mesh, no levels of detail.

### Settled in session 12 (Jacob, 2026-10-03)

Put to him at the start of building, the four questions the hand-off left open.

- **Surface nets** for the smooth mesher, not marching cubes.  Ensemble's only: the server reads the
  density itself.
- **The simple overworld map is dropped**: "we are dropping it... we don't need it anymore."  The offer at
  PLAY, the pieces, the red map bar, `map_cooldown_seconds`, `overworld.rs` on both sides and
  SIMPLE_OVERWORLD_MAP.md all go, and PlayerReady stops carrying the map's hash: a protocol bump, all four
  programs, a step of its own.  **Its shape** (Jacob's OK, session 12): PLAY's answer keeps only where the
  character will stand and how far it sees, as **GroundOffer** (`0x40`, "your pick here"); the two map
  packets (`0x41`, `0x42`) are retired; PlayerReady carries its ask number only; protocol version 17.
  **`map_cooldown_seconds` goes with it** ("yes because its no longer a risk"): it was there so PLAY
  couldn't be spammed for 16 MB at a time.  **The leftover map files are deleted by hand** ("delete by
  hand you give me a copy paste line"), the server's and the player's, not by code.
- **A density is one byte, 0 empty to 255 full, and 128 and over is solid** ("your proposal seems fine").
  **Every voxel holds one, air included**: the surface sits between a solid voxel and the air beside it,
  and the air's density is what says where.  (The summary's "a structure voxel has none" is taken as:
  it's always full, and nothing reads it.)  **A voxel's kind is AIR exactly when its density is under
  128**, so everything that asks "is this solid?" of a kind today (movement, spawn points, the cube
  mesher) stays right.
- **The kinds**: **DIRT and STONE are terrain** (smooth).  **WOOD is structure** (cubes; "i'll make it look
  like planks").  **GOLD is dropped**: the origin is no longer marked, and its number, 4, is never used
  again.  **MASONED_STONE is new**, number 6, structure: grey bricks, what a wall is made of ("It looks like gray
  bricks yeah... a wall will be made of it").  **BEDROCK is terrain** ("Probably terrain"): it's never
  broken, so it stays full, and a cave reaching down to it meets it smooth.  **AIR is neither** ("AIR:
  NOTHING").

### Leaning

- **Sending the edit for damage**, later, with "send the chunk again if in doubt" behind it.

### Open

- **The `.fbm` format**: as plain as can be (its size, the voxel kinds, a ground-level mark, maybe spawn
  points), read by Rust and by C#.  A contract of its own when it comes.
- **Block shapes for structures** (ramps, half blocks, arches, 7DTD's): not for stage 1.
- **How a density is held**: a byte a voxel (settled in session 12, above); a chunk goes from 64 KB to
  96 KB before squeezing.  What it costs squeezed is a guess until measured.
- **The generator's 3D rule** for caves, overhangs, the catacombs' hollows and their one way in; how a
  building flattens or fills the ground under it (7DTD's ground-level mark).
- Whether a blast changes the kind at its rim too (scorched dirt).
- **The cost of a blast**: meshes and colliders made again for up to 8 chunks and their neighbours.  Timed
  once it exists.
- How the mesher draws where a structure voxel meets a terrain voxel.
- Where the Shader Graph blend gets "which kinds at this corner" from the mesher.
- **A tuning note**: with a 45-degree limit and 1 m voxels, slopes the generator makes will often sit near
  the limit; walkable slopes want to be clearly gentler, cliffs clearly steeper.

### What it touches

- **What a voxel holds**: terrain voxels get a density; structure voxels stay a kind.
- **Making the world**, the biggest change: a 3D density with carving, the rule for how wide a change of
  kind is, buildings stamped from `.fbm`s, placed from the seed so a stamped chunk still counts as
  untouched.
- **Saving and streaming chunks**: small; the same files and the same stream; a changed smooth chunk
  squeezes less well; the simple overworld map may go.
- **The client's mesher and materials**: a second, smooth mesher for the terrain kinds beside the cube one;
  the Shader Graph blend.
- **Collisions on the client**: the same (a mesh collider a chunk), a new shape of mesh; the cost of
  making it again after an edit to watch.
- **The server's move checks**: the density sampled, the ground's height between two densities, the
  45-degree slope; the 1 m step stays.
- **Damage**: edits in a sphere that lower (blasts) or raise (spells) the density; changed chunks sent
  again for now.
- **The distance**: closed.  Nothing past 8 chunks, the skybox past that.
- **The height**: unchanged, -32 to +319.

### As written

**Step 1, what a voxel holds** (session 12, 2026-10-03; Jacob's OK; **written, not built**).  Conductor's
`gameworld`, and the kinds in the other three.  No packet changes shape, but the kinds a chunk carries do,
so **protocol version 16** (Jacob: "bump protocol version up"), all four programs together.
- `block.rs`: `Density`, a byte, `EMPTY` 0, `HALF` 128, `FULL` 255, `is_solid()` at 128 and over, and
  `fits()`, the rule that a kind and its density agree (AIR under halfway, terrain halfway and over,
  structure only full).  `Block::is_terrain()` (DIRT, STONE, BEDROCK), `plain_density()` (empty for AIR,
  full for the rest).  GOLD gone, 4 never used again; MASONED_STONE 6.
- `chunk.rs`: a density beside every kind; the file at version 3, the densities after the kinds; version 2
  still read at plain densities; a file whose kinds and densities disagree turned away.  96 KB a chunk in
  memory: with 3,179 chunks held at `view_chunks` 8, about 100 MB more than before, the sum; **measured**
  (Jacob, 2026-10-03, the Conductor tab with the server up): about 810 MB before, 918 after, 108 MB more.
- `build.rs`: no GOLD at 0,0,0, so 0,0 is Omega's dirt at whatever height its heights file has, and the
  spawn point's height follows it.  Every voxel at its plain density.
- `squeeze.rs`: the kinds only, as before; a chunk unsqueezed comes back at plain densities.
- `test_client.py`: the block names, 4 out and 6 in.  Checked: the view came to 88,742 bytes, 4 fewer
  (the GOLD's run), no GOLD in it, and 0,0,0 is AIR in Jacob's world (Omega's dirt there is under 0), so
  Tester, saved on the GOLD, comes in standing over the ground.
- Ensemble: `Blocks` (`Chunk.cs`) loses Gold and gains MasonedStone; GroundView's GOLD slot is gone and a
  MASONED_STONE slot is new (empty until Jacob gives it a material; nothing makes it yet).
- Version 16 in `protocol.rs`, `test_client.py`, Ensemble's and Soundcheck's `Protocol.cs`, PROTOCOL.md.

---

# The brief, as the discussion chat had it

Written in session 11 (2026-10-03) for Jacob to paste into a separate, non-code chat.  Jacob: "we are going
to be eventually moving away from cube shape voxels into smoother ones... but I'm not sure how that works".
The ECS was talked through in another chat once and that chat lost the thread, so this brief carries the
facts it needs, and asks for a summary at the end that comes back through Jacob into the docs.  The docs in
the code chat stay the source of truth: whatever is settled here is written into this file, `design/world.md`
and TODO.md by the code chat.

---

## To the chat reading this

You're helping Jacob, an amateur hobbyist, think through moving his game's world from cube voxels
(Minecraft's look) to smooth ones (EverQuest Next's and Landmark's look).  **This is a discussion, not a
coding session**: explain how smooth voxels work, lay out the choices and what each would mean for his
game, and help him decide.  Don't write the game's code; a short sketch to show an idea is fine.

How to talk to him:
- He's comfortable in C and C#, still learning Rust.  Explain plainly, no jargon without saying what it means.
- Keep replies short at first.  Expand when he engages.
- When something he says could mean more than one thing, ask, and say what each reading would mean in
  practice.  Don't fill gaps with guesses.
- Put your questions for him at the bottom of each reply under this header, large on purpose:
  `# >>>>>>>>>> QUESTIONS FOR JACOB <<<<<<<<<<`
- A cost (memory, time, bandwidth) is a guess until it's measured; say so when you give one.
- If you're not sure of a fact (what EverQuest Next actually did, say), say you're not sure.  Don't invent
  facts about his project either: if it's not below, ask him.

**When he says he's done**, write a summary under these four headings, for him to paste back into the code
chat:
1. **Settled**: each decision, with his own words quoted where he gave them.
2. **Leaning**: what he likes but hasn't settled.
3. **Open**: questions still to answer.
4. **What it touches**: which parts of the game (the list under "What smooth voxels would touch" below).

---

## The game

**Forgotten Legends** (codename Opus) is a multiplayer game Jacob is building: a server called **Conductor**
(Rust) that owns the world and decides everything, and a client called **Ensemble** (Unity 6, C#) that draws
it.  Third person, a fixed camera above and behind like Project Zomboid's.  Players walk EverQuest's way
(the client moves its own character and the server checks each move and pulls it back if it's wrong).  NPCs
(goblins that mine, hunt and trade, a dragon that attacks cities) are coming.  **The ground is never
mined by players**: "we're only going to have voxels being destroyed by combat and effects.  You won't mine
the chunks of the world awway."  Buildings are separate from the ground, made by whichever species built
them.

## The world as it is now (cubes)

- **A block is a cube 1 m a side**, Minecraft's size.  A player is 2 blocks tall.
- **A block holds only what it's made of**: one number, two bytes (up to 65,536 kinds).  Today AIR, DIRT,
  STONE, WOOD, GOLD and BEDROCK.  64 KB a chunk before squeezing.
- **A chunk is 32 x 32 x 32 blocks.**  Chunks stack: the world is **11 chunks tall, y = -32 to +319**.
  - +319 is the top.  -31 and -32 are BEDROCK, the floor nobody breaks.
  - The ground today sits around 0: one block of dirt on 30 of stone.  Above it, about 314 blocks of air.
- **The world is square**, `world_size` x 1024 blocks a side (Jacob runs 32: 32,768 blocks, about 33 km),
  with 0,0,0 in the middle, marked by a GOLD block.
- **Two test regions**: Alpha (west half) dead flat; Omega (east half) rolling hills at most 5 blocks up or
  down, from two layers of noise, each column's height worked out once and kept in a file.  Where they meet
  there's a sharp step.
- **A chunk nobody changed isn't stored**: it's made again from its region's rule when needed.  A changed
  chunk is saved whole as its own file, and that file always wins.
- **The server streams chunks to the client**: the client asks for the chunks around it, nearest first;
  each is squeezed as runs of the same block kind (most chunks are air, so they squeeze to almost nothing);
  a whole view (8 chunks each way, 3,179 chunks) was 88,746 bytes and 0.04 s on a LAN.  The client also gets
  a rough height map of the whole world once, for the distance (not drawn yet).
- **The client draws a mesh a chunk**: a face only where a solid block meets air, one material a block kind
  (plain colours for now).  A chunk is drawn once all six of its neighbours are in.  437 chunks meshed in
  about half a second, on a worker thread.  **No custom shaders yet** (a rule so far: Unity's own materials).
- **Collisions**: a Unity mesh collider on each chunk's mesh; characters are capsules.
- **The server checks moves against the blocks**: not inside a block, standing on something, not hanging in
  the air, one-block steps up are free.  It reads a block by its x, y, z.

## What Jacob wants

- **Smoother ground than cubes**, EverQuest Next's style: "we want the EQ Next style voxels not Minecraft
  really".
- Earlier, on the same thing: "possibly trimming down the number of voxels per column".
- Mountains that read as mountains (asked about while picturing one: a watch tower is 15 to 25 m, a
  mountain that reads as one 100 m and up).
- Combat and effects (a dragon's fire, spells) can destroy ground, and the damage stays.

## What smooth voxels would touch

For the summary's last heading.  Each is a part of the game a change could reach:
- **What a voxel holds** (today a kind number; smooth ones usually add how solid it is, a "density").
- **Making the world** (today a height per column; smooth ground can have overhangs, cliffs and caves,
  which a height per column can't).
- **Saving and streaming chunks** (the size of a chunk, how well it squeezes).
- **The client's mesher** (how the surface is built from the voxels) and its materials (blending dirt into
  stone at the surface usually needs a shader).
- **Collisions** on the client.
- **The server's move checks** (inside the ground, standing on it, steps up).
- **Damage** (how a blast changes the ground, and how that's saved and sent).
- **The distance** (far ground drawn cheaply, levels of detail).
- **The height range** (whether 320 blocks of sky and 30 of stone still make sense).

## Questions to start from

The other chat can take these in any order:
1. How do smooth voxels work at all, compared with cubes?  The common ways of turning voxels into a smooth
   surface, in plain words, and which look like EQ Next.
2. Does the whole world go smooth, or smooth ground with blocky things (buildings, walls) beside it?
3. Does the ground need sharp edges anywhere (cliffs, built walls), or is it all rounded?
4. Does a voxel stay 1 m, or get bigger or smaller?  What that does to memory and to how detailed the
   ground looks.
5. Caves and overhangs: wanted?
6. What a blast does to smooth ground, and how it looks.
7. How the surface's materials meet (grass into dirt into stone).
8. What the server needs to know to check a move on smooth ground.
