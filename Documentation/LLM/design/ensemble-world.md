<!--
File:       Opus/Documentation/LLM/design/ensemble-world.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Ensemble -- the world on screen

Started 2026-10-03, session 5.  Jacob: "next step is to render the voxels in Ensemble to the player".  The chunks
come in as `design/ensemble-networking.md` says ("The chunks, Ensemble's half") and sit in the Ground
(`Code/World/Ground.cs`); this file is how they're drawn, and the camera that looks at them.

## Settled (Jacob, 2026-10-03, session 5)

- **The chunks only**: the distance from the simple overworld map is a session of its own.
- **Minecraft's and 7 Days to Die's way at the edges**: a chunk is drawn only once all six chunks round it
  are in (above row 10 counts as air, below row 0 as nothing), so the outermost ring of the view is never
  drawn: at `view_chunks` 8 the player sees 7 out.  The other shape was the edge drawn as walls, a cut-out
  diorama, with chunks drawn again as their neighbours come.
- **A mesh a chunk, faces only between a solid block and air**; an all-air chunk has none.  The arrays
  built on a worker thread, Unity's Mesh filled on the main thread.  A line in the Console says how many,
  how many faces and how long (a guess at the cost is no use; the line is the measurement).
- **The materials are Jacob's**: "we'll make the materials on my end -- for now they're just colors".  A
  block kind is a submesh with its own material, from a slot each in the Inspector.  No shader of ours.
- **The camera**: "we're gonna be using a cinemachine following camera in the style of zomboid except a bit
  more direct".  Project Zomboid's is a fixed, angled view from above that follows the character.
- **Where the character stands**: a block at x, y, z fills x to x+1 each way, and the character's y is its
  feet, so a character at 0, 0, 0 stands in the GOLD, a block into Alpha's ground.  Drawn as it is; the
  spawn's height is a Conductor question in TODO.md.
