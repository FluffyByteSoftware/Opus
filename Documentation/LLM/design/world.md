<!--
File:       Opus/Documentation/LLM/design/world.md
Component:  Documentation
Author:     Jacob Chacko
-->

# The world

The world is what the game is played in: the ground, what it's made of, and how it's cut up.  All of it was
designed with Jacob on 2026-09-30, and part one (`conductor-gameworld`) was built and tested the same day, so
every decision and quote below is his from that day unless it says otherwise.

## Where it stands

**Part one is built and tested on Linux** (Jacob's first world took 19 seconds to make).  Every run check
passed, the door waiting on the world, the sharp divide and BEDROCK included.  The last change, only
housekeeping running until the ground is in, is pulled on Jacob's machine and its check is still on
TEST_CHECKLIST.html.

`conductor-gameworld` (lib, folder `gameworld`, `conductor_gameworld` in code; Jacob named it) is the world,
not primlib.  It's a server piece with its own thread, `gameworld`, and a GameWorld line on the Services tab,
and rebuilds the world from its files on every START SERVER.  It reads `region.map`, or makes the world if
there isn't one: a seed from Fingerprinter, Omega's heights file, then `region.map` last, so a stop part way
leaves no half world.  (To make a new world, stop the server and delete `Content/world/`.)  A heights file
that has gone missing is made again from the map's seed, with a Warn.  Then it answers the GameClock's asks:
each chunk from its own file if it has one, otherwise built from its region's ground.  A chunk file that
doesn't read right is a Warn and that chunk stays out of the game, never built over, since it may be the only
copy of somebody's digging.

The GameClock's `Terrain` (gameworld's `terrain.rs`) asks for the chunks around 0,0,0 on START SERVER and
takes them in during housekeeping.  Until they're all in, only housekeeping runs ("I don't want NPCs acting
while the server world isn't ready"; `design/gameclock.md`) and the door stays shut
(`design/conductor-launcher.md`).  Jacob asked for the door after his first run, when networking listened
through the 19 seconds: nobody gets in before there's a voxel to step on (TCP and UDP both, "may as well").

**Part two is saving** (under "Saving" below).  Nothing changes a chunk yet, so it comes with the first thing
that does (digging, or a way to set a block for testing).

## The files

All under `Content/world/`, gitignored (it's the game's save).  Every number is little-endian.

`region.map`: which region every chunk is in, since a chunk nobody changed has no file to say so.  It's
binary, not text: "its a summary that tells the chunks how to assemble themselves for the server _AND_ the
client".  **`Documentation/LLM/REGION_MAP.md`** has it, byte for byte, with a worked example, the checks a
reader makes, a C# reader for Ensemble and a one-line Python look at a real file (Jacob's ask: "a thorough
document that explains how to read our new binary map file").  That document is the contract with Ensemble,
like PROTOCOL.md, so it isn't copied here, and a change to the layout bumps its version.  524,330 bytes for
the first world.

`Regions/<Region>/<region>.heights` (Omega's, today):

```text
8 bytes   OPUSHGHT
u16       version, 1
u64       the seed it was made from, the same as region.map's
i16       the westmost column's x
i16       the southmost column's z
u16       how many columns east-west
u16       how many columns north-south
i8 x every column   the dirt's height, -5 to 5: the south line first,
                    in a line west to east.
```

134,217,754 bytes for Omega.

`Regions/<Region>/<region>_<x>_<z>_<row>.chunk` (`alpha_-015_003_0.chunk`), one per changed chunk:

```text
8 bytes   OPUSCHNK
u16       version, 1
i16       x, the chunk's place east-west
i16       z, north-south
u8        row, 0 (lower) or 1 (upper)
u16 x 32768  the blocks, bottom layer first; in a layer, the south row
             first; in a row, west to east.  (y * 32 + z) * 32 + x.
```

The block numbers: AIR 0, DIRT 1, STONE 2, WOOD 3, GOLD 4, BEDROCK 5.  A number never changes once it's
out there.

## What's settled

### The whole

- **The world is the total sum of everything**, cut up into chunks, and chunks into voxels.
- **The world is seamless**: one `World`, one GameClock, no borders to cross.  The playable area comes from
  loading and ticking only the chunks near a player (not zones on clocks of their own, each its own `World`
  and thread with a handoff between).
- **25 to 50 players at once is the hope; 25 is more than Jacob can picture playing.**  One GameClock on one
  thread is sized for that.
- **The voxels are for tearing things down, not the whole point**: "The voxels aren't the entire point of our
  world just more for destructive view."
- **The GameClock's thread holds the terrain** ("GameClock Thread seems right").  The chunks in memory are
  the GameClock's, the way primlib's `World` is, so digging a block in a check is a change in memory with no
  lock and no waiting.  GameWorld's own thread does the slow part: reading and writing chunk files through
  DiskMan, and handing chunks over when they're ready.

### Blocks and chunks

- **The world is blocky**: you see the cubes, the way Minecraft looks, not smooth ground drawn over the
  voxels.
- **The blocks are smaller than Minecraft's, with high-res textures**: "our average block is smaller (1/4 the
  size of a minecraft block should work)".
- **A block is 50 cm a side**: "A player is to be 4 blocks tall at 2 meters."  Half a Minecraft block's
  width, an eighth of its volume, eight blocks to a cubic metre.
- **A block holds what it's made of, and that's all, for now.**  The first kinds are **DIRT, STONE, AIR and
  WOOD**, then **GOLD** for the origin block and **BEDROCK** for the floor ("like bedrock type now").  The
  kind is one number per block, **two bytes, up to 65,536 kinds** ("I think 65k will be enough"): 64 KB a
  chunk before anything is squeezed.
- **A chunk is a cube of 32 blocks a side, 16 m** ("we'll start with the 32 a side"), 32,768 blocks.  Chunks
  stack up and down, not columns the world's full height.  Not 64 a side: that's eight times what Ensemble
  redraws when one block changes, and biome steps 32 m wide.

### The shape

- **8 km by 8 km to start**, "it may _grow_ later".  16,384 blocks a side, 512 by 512 chunks across,
  524,288 chunks in all with two rows.
- **0,0,0 is the middle of the world**: "I want it to span -8096 to 8096 and at 0, 0 its gold".  **The span
  is in blocks** ("voxels lol"), taken as **-8192 to 8191** each way, since 8096 isn't a whole number of
  32-block chunks and 8192 is.  Chunks run -256 to 255.  File names take negative numbers.
- **The origin block, 0,0,0, is GOLD**, so the middle of everything can be seen, whatever is around it.
- **The world starts flat, in three layers, counted in blocks**: "blocks at 0 are all dirt, blocks at -1 thru
  -15 are stone, blocks 1 and higher are air".  One block of dirt on top of 15 of stone (7.5 m), and air
  above.
- **The ceiling is +30, the floor below -15**: "you can go 30 voxels HIGH before you hit the ceiling; you can
  go down to -15 voxels in the ground before its undiggable".  Read as: +30 is the last block that can be
  built, -15 the last that can be dug, and -16 the floor nobody breaks ("-16 is undiggable"), all BEDROCK.
  That's 46 blocks from -15 to +30, so **the world is two chunks tall**: a lower row from -16 to +15 (the
  floor, the stone, the dirt and 15 of air) and an upper row from +16 to +47, all air at the start, with
  nothing allowed above +30.
- **Every player starts at 0,0,0, for now.**  So until there's movement, "the chunks near a player" is the
  chunks around 0,0,0, and that's what GameWorld loads on START SERVER.
- **A player is sent the chunks 4 each way around them, to start**: 64 m, "but it might need to be 8"
  (128 m).  With the world two chunks tall, that's a 9 by 9 square, both rows, 162 chunks a player.  **The 4
  is `view_chunks` in `game.cfg`** (1 to 16), so trying 8 is the Settings tab and a STOP SERVER and START
  SERVER.

### Regions

- **A region, a zone and a biome are one thing**: "a biome name / zone or collection of chunks".  It's a
  label we mark, not a clock.  **Each chunk is flagged with the zone it's in**, not each voxel, so a biome's
  edge runs in steps a chunk wide.  **A chunk is in one zone only**, never across two, and `region.map`
  says which.
- **The world starts with two regions, Alpha and Omega, split evenly**: **Alpha west, Omega east "from
  origin 0,0,0"**, with the gold block on the line between (on Omega's side).
- **Alpha is purely flat**, the three layers above.  **Omega is bumpy**: "random noise with +/- 5 on the Y",
  so its ground rises and falls up to 5 blocks (2.5 m) either side of 0, in **smooth rolling hills**, with
  **one block of dirt on top and stone under it down to -15**, as in Alpha.  The noise is written by hand,
  no crate.  So the GOLD can sit inside a hill or with air under it.
- **Where Alpha meets Omega is a sharp divide**: "it just suddenly becomes the other biome - in a real build
  and not this test, we'll have a blending technique".  So there can be a step of up to 5 blocks at x = 0.

### Saving

- **A chunk nobody has changed isn't stored at all**: it's made from its region's rule when it's needed
  (flat for Alpha, the heights file for Omega), and only a changed chunk is kept.  Put forward in the
  session rather than Jacob's call, and it stands.
- **A chunk's own file always wins** (Jacob's yes, after asking what happens to a hole): the first time
  anybody digs or builds in a chunk, it's saved whole as its own `.chunk` file, and from then on that file
  is the truth for it.
- **A changed chunk is saved as a file under `Content/world/`** (gitignored), through DiskMan, not in the
  database.  **Chunk files go in a folder named after their region**: "when we generate the world we are
  going to name regions and name the chunk folders after them then place that chunk in there".  **The
  numbers are padded to three digits**: `alpha_-015_003_0.chunk`, east, north, then row.
- **Omega's bumps are saved, not made again**: "Omega chunks will need to be saved with their bumpiness
  before server shuts down".  Made once, when the world is made, and kept, so a change to the noise code
  later never reshapes hills already there.  **Kept as one heights file**: one number per 50 cm column, how
  high the dirt is, -5 to +5, about 134 MB for half of 8 km (every Omega chunk whole would be about 8 GB).
- **It's saved on STOP SERVER, and by a global save every 15 minutes.**  A chunk that changes is marked, and
  the GameClock hands copies of the changed ones to GameWorld to write, never waiting on it.
- **The global save is the terrain only**: "whatever is in memory about the voxel states", dumped to disk.
  Not primlib's objects, which go to the database on STOP SERVER (`design/primlib.md`).
- **The 15 minutes is a setting in `game.cfg`**, `save_minutes`, not fixed in code like the tick; it goes in
  with part two, when something reads it.  **`game.cfg` is soft** (in Constellations' table, read on every
  START SERVER): "the world should need a reboot so the voxel engine or service restarts and rebuilds".

## Still open

- What a zone does in the game beyond its name: what grows and what spawns there, and whatever else a
  biome decides.
