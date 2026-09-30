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
- **Zones are back in question** (Jacob, the same day): "I'm not sure we need zones."  His first thought
  was a zone as a biome, then as a separate area on its own clock, apart from the other zones, for more
  playable area.  Against that, a seamless world "might be better".  The two ways are under "Still open".

## Still open

- **Zones, or a seamless world.**
  - *Zones on their own clocks*: each zone is its own `World` with its own GameClock and thread, and going
    from one to the next is a handoff (a loading screen, likely).  A zone nobody is in can sleep.  It uses
    more than one CPU core, but nobody sees or reaches across a border.  It runs into the rule that one
    GameClock owns the one `World`, so that rule would change.
  - *Seamless*: one `World`, one GameClock, no borders.  The playable area comes from only loading and
    ticking the chunks near a player, so the world can be far bigger than what's awake.  Everything runs
    on the GameClock's one thread, which is the limit once there are a lot of players spread wide.
  - A biome could still be a zone in name only: a label on an area (what grows, what spawns), not a clock.
- The chunk's size, and whether chunks are cubes stacked up and down or columns the world's full height.
- A voxel's size in the game's units, and what a voxel holds.
- The world's size; flat, generated, or drawn by hand.
- Where it's kept and when it's saved (the database, files through DiskMan, or both).
- What each client is sent: the chunks near them, since the server decides what each client sees.
- Whether the world belongs to primlib beside its `World` of entities, or is a piece of its own.
- What a zone does in the game: who hears what, what gets ticked, spawn areas, loading and unloading what
  nobody is near, and how the GameClock handles one.
