<!--
File:       Opus/Documentation/LLM/design/world.md
Component:  Documentation
Author:     Jacob Chacko
-->

# The world

The world is what the game is played in: the ground, what it's made of, and how it's cut up.  Third on
Jacob's map (the tick, then the world's voxels, then zones).  Designed 2026-09-30, and part one written the
same day: `conductor-gameworld`.

## Where it stands

**Part one is built and ran** (2026-09-30: Jacob's first world took 19 seconds to make).  The door waiting
on the world, the sharp divide and BEDROCK came after, and aren't built yet.

`conductor-gameworld` (lib, folder `Conductor/dev/gameworld/`) is a server piece with its own thread,
`gameworld`, and a GameWorld line on the Services tab.  On START SERVER it reads `region.map`, or makes the
world if there isn't one (a seed from Fingerprinter, Omega's heights file, then `region.map` last, so a stop
part way leaves no half world).  A heights file that has gone missing is made again from the map's seed, with
a Warn.  Then it answers the GameClock's asks: each chunk from its own file if it has one, otherwise built
from its region's ground.

The GameClock holds a `Terrain` (gameworld's `terrain.rs`): on START SERVER it asks for the chunks within
`view_chunks` of 0,0,0 (162 at 4), and its housekeeping check takes in whatever has arrived, never waiting.
Its Services line says how many are in.

**The door waits on the world** (Jacob, 2026-09-30, after his first run: the world took 19 seconds to
make, and networking was listening the whole time).  Nobody gets in before there's a voxel to step on:
the launcher no longer starts networking with the rest of the server.  The GameClock sets a flag once every
chunk around 0,0,0 is in its terrain (`conductor_gameclock::ready()`), and the launcher's command loop,
which already wakes every 250 ms, starts networking (TCP and UDP both, Jacob's "may as well") the first
time it sees it.  Until then the server reads running, so STOP SERVER works, and the two Network lines on
the Services tab say they're waiting on the world.  A run where a chunk there can't be had never opens the
door.

`game.cfg` is in Constellations' table (soft) with one setting, `view_chunks` (1 to 16, default 4).
`save_minutes` goes in with part two, when something reads it.

**Part two**: saving.  A chunk that changes is marked, and the changed ones go to GameWorld to write on
STOP SERVER and every `save_minutes` (15).  Nothing changes a chunk yet, so part two comes with the first
thing that does (digging, or a way to set a block for testing).

**Where Alpha meets Omega is a sharp divide** (Jacob, 2026-09-30): "it just suddenly becomes the other
biome - in a real build and not this test, we'll have a blending technique".  So there can be a step of up
to 5 blocks at x = 0.  (Part one first went out with Omega's hills fading in over its first chunk; Jacob
turned that down the same day.)  The GOLD block is at 0,0,0 whatever is around it, and 0,0,0 is on Omega's
side, so it can sit inside a hill or with air under it.

**The floor at -16 is BEDROCK**, a kind of its own (Jacob, 2026-09-30: "like bedrock type now").

## The files

All under `Content/world/`, gitignored.  Every number is little-endian.  `region.map` is a contract with
Ensemble as well, so a change to it bumps its version.

`region.map`:

```text
8 bytes   OPUSRMAP
u16       version, 1
u64       the seed the world was made from
i16       the westmost chunk's x (-256)
i16       the southmost chunk's z (-256)
u16       how many chunks east-west (512)
u16       how many chunks north-south (512)
u8        how many rows up and down (2)
u8        how many regions, then for each:
            u8   its ground: 0 flat, 1 heights
            u8   its name's length, then the name, UTF-8
u8 x every chunk   the number of the region it's in, counted from 0 in
                   the list above: the lower row first; in a row, the
                   south line first; in a line, west to east.
```

524,330 bytes for the first world.

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

- **The world is the total sum of everything.**  Jacob, 2026-09-30.
- **It's cut up into chunks, and chunks into voxels.**  Jacob, 2026-09-30.
- **The world is seamless** (Jacob, 2026-09-30): one `World`, one GameClock, no borders to cross.  The
  playable area comes from loading and ticking only the chunks near a player.  (Zones on their own clocks,
  each its own `World` and thread with a handoff between, were weighed the same day and passed over.)
- **A zone is a region we mark, and it coincides with a biome** (Jacob, 2026-09-30).  It's a label, not a
  clock: **each chunk is flagged with the zone it's in**, not each voxel (his pick the same day).  So a
  biome's edge runs in steps a chunk wide.
- **25 to 50 players at once is the hope; 25 is more than Jacob can picture playing** (2026-09-30).  One
  GameClock on one thread is sized for that.
- **The world is blocky** (Jacob, 2026-09-30): you see the cubes, the way Minecraft looks, not smooth ground
  drawn over the voxels.
- **The blocks are smaller than Minecraft's, with high-res textures** (Jacob, 2026-09-30): "our average
  block is smaller (1/4 the size of a minecraft block should work)".
- **A block is 50 cm a side** (Jacob, 2026-09-30): "A player is to be 4 blocks tall at 2 meters."  That's
  half a Minecraft block's width, an eighth of its volume, eight blocks to a cubic metre.
- **A chunk is a cube of 32 blocks a side, 16 m** (Jacob, 2026-09-30: "we'll start with the 32 a side"),
  32,768 blocks.  Chunks stack up and down, not columns the world's full height.  64 a side (32 m) was
  weighed and passed over: eight times what Ensemble redraws when one block changes, and biome steps 32 m
  wide.
- **A block holds what it's made of, and that's all, for now.**  The first kinds are **DIRT, STONE, AIR
  and WOOD** (Jacob, 2026-09-30).  The kind is one number per block, **two bytes, up to 65,536 kinds**
  ("I think 65k will be enough", Jacob, the same day): 64 KB a chunk before anything is squeezed.
- **The origin block, 0,0,0, is GOLD** (Jacob, 2026-09-30), a fifth kind, so the middle of everything can
  be seen.  It sits in the dirt layer, at 0.
- **The voxels are for tearing things down, not the whole point** (Jacob, 2026-09-30): "The voxels aren't
  the entire point of our world just more for destructive view."
- **The world starts flat, in three layers, counted in blocks** (Jacob, 2026-09-30): "blocks at 0 are all
  dirt, blocks at -1 thru -15 are stone, blocks 1 and higher are air".  One block of dirt on top of 15 of
  stone (7.5 m), and air above.
- **The ceiling is +30, the floor below -15** (Jacob, 2026-09-30): "you can go 30 voxels HIGH before you
  hit the ceiling; you can go down to -15 voxels in the ground before its undiggable".  Read as: +30 is the
  last block that can be built, and -15 the last that can be dug, with -16 the floor nobody breaks (he
  said so: "-16 is undiggable").  That's 46 blocks from -15 to +30, so **the world is two chunks tall**:
  a lower row of chunks from -16 to +15 (the floor, the stone, the dirt and 15 of air) and an upper row
  from +16 to +47, all air at the start, with nothing allowed above +30.
- **8 km by 8 km to start** (Jacob, 2026-09-30), "it may _grow_ later".  16,384 blocks a side, 512 by 512
  chunks across, 524,288 chunks in all with two rows.
- **A chunk nobody has changed isn't stored at all**: it's made from its region's rule when it's needed
  (flat for Alpha; Omega's saved bumps), and only a changed chunk is kept.  Put forward, 2026-09-30.
- **A changed chunk is saved as a file under `Content/world/`** (Jacob, 2026-09-30), through DiskMan,
  not in the database.  **`Content/world/` is gitignored** (his yes, the same day): it's the game's save.
- **It's saved on STOP SERVER, and by a global save every 15 minutes** (Jacob, 2026-09-30).  The GameClock
  never waits on it: it hands over copies of what changed, and the writing is done on another thread.
- **The global save is the terrain only** (Jacob, 2026-09-30): "whatever is in memory about the voxel
  states", dumped to disk.  Not primlib's objects, which go to the database on STOP SERVER as before.
- **The 15 minutes is a setting, in a new config file, `game.cfg`** (Jacob, 2026-09-30).  Not fixed in
  code like the tick.  **`game.cfg` is soft** (his pick the same day): read on every START SERVER, "the
  world should need a reboot so the voxel engine or service restarts and rebuilds".
- **A player is sent the chunks 4 each way around them, to start** (Jacob, 2026-09-30): 64 m, "but it
  might need to be 8" (128 m).  With the world two chunks tall, that's a 9 by 9 square of chunks, both
  rows, 162 chunks a player.  **The 4 is a setting in `game.cfg`** (Jacob, 2026-09-30), so trying 8 is
  the Settings tab and a STOP SERVER and START SERVER.
- **The world is its own crate, `conductor-gameworld`** (lib), folder `Conductor/dev/gameworld/`, and
  `conductor_gameworld` in code (Jacob named it, 2026-09-30).  Not in primlib.  It's a server piece: it
  starts on START SERVER and stops on STOP SERVER, and rebuilds the world from its files each time.
- **The GameClock's thread holds the terrain** (Jacob, 2026-09-30: "GameClock Thread seems right").  The
  chunks in memory are the GameClock's, the way primlib's `World` is, so digging a block in a check is a
  change in memory with no lock and no waiting.  `conductor-gameworld` has a thread of its own, a service
  on the Services tab, for the slow part: reading and writing chunk files through DiskMan, handing chunks
  over when they're ready.
- **Chunk files go in a folder named after their region** (Jacob, 2026-09-30): "when we generate the world
  we are going to name regions and name the chunk folders after them then place that chunk in there".
  One file per changed chunk, his example being `Content/world/Regions/Alpha/alpha_03_-15_0.chunk`.
- **A region, a zone and a biome are one thing**: "a biome name / zone or collection of chunks" (Jacob,
  2026-09-30).  **A chunk is in one zone only**, never across two.
- **Which chunk is in which region is written in `Content/world/region.map`** (Jacob, 2026-09-30), since
  a chunk nobody changed has no file to say so.  Gitignored with the rest of `Content/world/`.  **It's a
  binary file**, not text (Jacob, the same day): "its a summary that tells the chunks how to assemble
  themselves for the server _AND_ the client".  So its layout is a contract with Ensemble, written down
  under "The files" below.
- **Every player starts at 0,0,0, for now** (Jacob, 2026-09-30).  So until there's movement, "the chunks
  near a player" is the chunks around 0,0,0, and that's what GameWorld loads on START SERVER.
- **The world starts with two regions, Alpha and Omega** (Jacob, 2026-09-30).  **Alpha is purely flat**,
  the three layers above.  **Omega is bumpy**: "random noise with +/- 5 on the Y", so its ground rises and
  falls up to 5 blocks (2.5 m) either side of 0, in **smooth rolling hills**, with **one block of dirt on
  top and stone under it down to -15**, as in Alpha (Jacob, 2026-09-30).  The noise is written by hand, no
  crate.  **The world is split evenly between them**, half each (Jacob, 2026-09-30): **Alpha west, Omega
  east "from origin 0,0,0"**.
- **Omega's bumps are saved, not made again** (Jacob, 2026-09-30): "Omega chunks will need to be saved
  with their bumpiness before server shuts down".  Made once, when the world is made, and kept, so a
  change to the noise code later never reshapes hills already there.  **Kept as one heights file**
  (Jacob's yes, 2026-09-30): one number per 50 cm column, how high the dirt is, -5 to +5, about 134 MB
  for half of 8 km, written once when the world is made.  Omega's untouched chunks are built from it.
  Saving every Omega chunk in full (about 8 GB) was weighed and passed over.
- **A chunk's own file always wins** (Jacob's yes, 2026-09-30, after asking what happens to a hole): the
  first time anybody digs or builds in a chunk, it's saved whole as its own `.chunk` file, and from then on
  that file is the truth for it.  Only a chunk nobody has touched is built from its region's rule (the
  flat layers for Alpha, the heights file for Omega).
- **0,0,0 is the middle of the world** (Jacob, 2026-09-30, turning round his "lower left corner" the same
  day): "I want it to span -8096 to 8096 and at 0, 0 its gold".  Alpha is everything west of 0, Omega
  everything east, and the gold block sits on the line between.  File names take negative numbers.
  **The span is in blocks** ("voxels lol", Jacob, 2026-09-30): the 8 km already settled, taken as **-8192
  to 8191** each way, since 8096 isn't a whole number of 32-block chunks and 8192 is.  Chunks run -256 to
  255.
- **A file's numbers are padded to three digits** (Jacob, 2026-09-30): `alpha_-015_003_0.chunk`, east,
  north, then row.

## Still open

- What's in a chunk's file beyond its blocks (a version number, at least).
- What a zone does in the game beyond its name: what grows and what spawns there, and whatever else a
  biome decides.
