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
  clock: for now a chunk or a voxel is simply flagged with the zone it's in.  Which of the two carries the
  flag is open.
- **25 to 50 players at once is the hope; 25 is more than Jacob can picture playing** (2026-09-30).  One
  GameClock on one thread is sized for that.
- **The world is blocky** (Jacob, 2026-09-30): you see the cubes, the way Minecraft looks, not smooth ground
  drawn over the voxels.
- **The blocks are smaller than Minecraft's, with high-res textures** (Jacob, 2026-09-30): "our average
  block is smaller (1/4 the size of a minecraft block should work)".  Whether that's a quarter the width
  or a quarter the volume is open.

## Still open

- **Whether the zone flag is on the chunk or on the voxel.**  On the chunk, a biome's edge runs in steps a
  chunk wide and the flag costs next to nothing.  On the voxel, the edge can run anywhere, and every voxel
  carries it.
- **"1/4 the size" of a Minecraft block: a quarter the width, or a quarter the volume.**  A quarter the
  width is 25 cm a side: a player is about seven blocks tall, and a cubic metre holds 64 blocks where
  Minecraft's holds one.  A quarter the volume is about 63 cm a side: a player about three blocks tall,
  and four blocks to a cubic metre.
- The chunk's size, and whether chunks are cubes stacked up and down or columns the world's full height.
- What a voxel holds.
- The world's size; flat, generated, or drawn by hand.
- Where it's kept and when it's saved (the database, files through DiskMan, or both).
- What each client is sent: the chunks near them, since the server decides what each client sees.
- Whether the world belongs to primlib beside its `World` of entities, or is a piece of its own.
- What a zone does in the game beyond its name: what grows and what spawns there, and whatever else a
  biome decides.
