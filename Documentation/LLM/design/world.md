<!--
File:       Opus/Documentation/LLM/design/world.md
Component:  Documentation
Author:     Jacob Chacko
-->

# The world

The world is what the game is played in: the ground, what it's made of, and how it's cut up.  Third on
Jacob's map (the tick, then the world's voxels, then zones).  Being designed, 2026-09-30.  Nothing is built.

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
  and WOOD** (Jacob, 2026-09-30).  The kind is one number per block.
- **The voxels are for tearing things down, not the whole point** (Jacob, 2026-09-30): "The voxels aren't
  the entire point of our world just more for destructive view."
- **The world starts flat, in three layers, counted in blocks** (Jacob, 2026-09-30): "blocks at 0 are all
  dirt, blocks at -1 thru -15 are stone, blocks 1 and higher are air".  One block of dirt on top of 15 of
  stone (7.5 m), and air above.
- **The ceiling is +30, the floor below -15** (Jacob, 2026-09-30): "you can go 30 voxels HIGH before you
  hit the ceiling; you can go down to -15 voxels in the ground before its undiggable".  Read as: +30 is the
  last block that can be built, and -15 the last that can be dug, with -16 the floor nobody breaks (he
  said so: "-16 is undiggable").  That's
  46 blocks from -15 to +30, so **the world is two chunks tall**: a lower row of chunks from -16 to +15
  (the floor, the stone, the dirt and 15 of air) and an upper row from +16 to +47, all air at the start,
  with nothing allowed above +30.
- **8 km by 8 km to start** (Jacob, 2026-09-30), "it may _grow_ later".  16,384 blocks a side, 512 by 512
  chunks across, 524,288 chunks in all with two rows.
- **Every chunk starts the same**, so a chunk nobody has changed needn't be stored at all: it's made from
  the three layers when it's needed, and only a changed chunk is kept.  Put forward, 2026-09-30.
- **A changed chunk is saved as a file in `Content/world/chunks/`** (Jacob, 2026-09-30), through DiskMan,
  not in the database.  **`Content/world/` is gitignored** (his yes, the same day): it's the game's save.
- **It's saved on STOP SERVER, and by a global save every 15 minutes** (Jacob, 2026-09-30, put as a
  question).  The GameClock never waits on it: it hands over copies of what changed, and the writing is
  done on another thread.

## Still open

- How big that number is.  Two bytes (65,536 kinds, 64 KB a chunk before anything is squeezed) was put
  forward, so high-res textures never run out of kinds; a chunk that's all one kind (all air, all stone)
  kept as that one value.
- What "global" takes in: the chunks only, or primlib's objects in the database as well.  And whether 15
  minutes is fixed in code, like the tick, or a setting.
- What a chunk's file is called and holds.
- What each client is sent: the chunks near them, since the server decides what each client sees.
- Whether the world belongs to primlib beside its `World` of entities, or is a piece of its own.
- What a zone does in the game beyond its name: what grows and what spawns there, and whatever else a
  biome decides.
