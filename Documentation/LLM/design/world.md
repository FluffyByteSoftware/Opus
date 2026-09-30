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
- **It's cut up in four levels**, each made of the one under it:

  ```
  World        everything
   └─ Zone     a biome
       └─ Chunk
           └─ Voxel
  ```

  A zone is a **biome**, so zones are the world's big areas by what kind of place they are, not a grid laid
  over it.  Zones are made of chunks, and chunks of voxels.  Jacob, 2026-09-30.

## Still open

- What shape a zone is: any set of whole chunks (a biome's ragged edge, drawn a chunk at a time), or a
  rectangle of chunks.
- The chunk's size, and whether chunks are cubes stacked up and down or columns the world's full height.
- A voxel's size in the game's units, and what a voxel holds.
- The world's size; flat, generated, or drawn by hand.
- Where it's kept and when it's saved (the database, files through DiskMan, or both).
- What each client is sent: the chunks near them, since the server decides what each client sees.
- Whether the world belongs to primlib beside its `World` of entities, or is a piece of its own.
- What a zone does in the game: who hears what, what gets ticked, spawn areas, loading and unloading what
  nobody is near, and how the GameClock handles one.
