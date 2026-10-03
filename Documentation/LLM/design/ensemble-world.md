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
  **Fixed**: "I'm going for the zomboid style where you can walk towards the camera but the camera only
  zooms in and out.  It doesn't swivel or rotate."  Cinemachine (3, already in the project) is set up by
  Jacob in the editor, following a **stand-in** the code puts where the character stands: a box 1 block
  wide and 2 tall, its feet at the server's y.  The other shape was the code building the camera rig.
- **The slots**: one material each for DIRT, STONE, WOOD, GOLD and BEDROCK.  An empty slot's faces aren't
  drawn, and the Console says so once.  (The stand-in was to have a slot too; as written it's a Cube in the
  scene, so its material is the Cube's own.)
- **Where the character stands**: a block at x, y, z fills x to x+1 each way, and the character's y is its
  feet, so a character at 0, 0, 0 stands in the GOLD, a block into Alpha's ground.  Drawn as it is; the
  spawn's height is a Conductor question in TODO.md.

## Built and working (Jacob, 2026-10-03: "THE GROUND WORKED")

**Measured** (Jacob, Linux, in the editor, a view of 8 at `world_size` 32): "World: 437 chunks meshed,
246903 faces, in 0.51 s (the worker's share 0.51 s, 1.16 ms a chunk); 302 chunks drawn in all."  The 135
meshed and not drawn had no faces: buried stone and BEDROCK with no air beside them.  All of the 0.51 s is
on the worker; the main thread only fills Unity's Mesh, 32 a frame.

## As written (2026-10-03, session 5)

- **`Code/World/ChunkMesher.cs`**: a chunk and the six round it into lists (`ChunkMesh`): four corners and
  a normal a face, two triangles, kept by block kind.  A face wherever a block that isn't air meets air;
  over the top row is air, under the bottom row solid.  Plain C# but for `Vector3`, so it runs on a
  worker.  Times itself.
- **`Scripts/World/GroundView.cs`** (a new folder under `Scripts/`): the five material slots and
  Meshes Per Frame (32).  It hears `Ground.Added` and sends a chunk to its worker thread ("Ground mesher")
  once it and the six round it are in; an all-air chunk is never sent.  `Update()` makes Unity's Mesh from
  up to 32 results a frame (32-bit indices only past 65,535 corners), a GameObject a chunk under
  GroundView, named "Chunk x,z row r", at its corner in blocks.  `Ground.Cleared` throws it all away,
  and a result for the old Ground is dropped when it comes back.  The Console's line when the worker runs
  out: "World: N chunks meshed, F faces, in S s (the worker's share W s, M ms a chunk); D chunks drawn in
  all."
- **`Scripts/World/CharacterStandIn.cs`**: on a Cube in the scene, scaled 1 by 2 by 1.  Hidden until
  `Session.ReachedWorld`, then moved so its bottom is at the character's feet (`Session.Standing`, new,
  from CharacterEnteredWorld's x, y, z); hidden again at `SessionOver`.  The Cinemachine camera follows it.
- **`Ground`** has `Added` and `Cleared`.

## PlayerReady once the ground is drawn (asked 2026-10-03, session 1; being talked through)

Jacob, after the ground worked: "can we make so the world starts 'loading in chunks' before the player is
drawn?  We need to sync this to the server too so that the server knows when the player client is ready".
Today PlayerReady goes once the nearest 99 chunks have *arrived* (`Session.NearGroundIn()`), and meshing
runs after, so the character can be put in the world, and the stand-in shown, before the ground round it
is on screen.  The server already waits for PlayerReady before it puts the character in, with no deadline
but the UDP timeout.  Open: what "ready" is (the nearest 99 drawn, or the whole view), whether the server
needs more than PlayerReady at a later moment, and what the loading bar says meanwhile.

**His answers**: ready is **the nearest 99 visible**: "I just don't want a situation where a player walks
forward and 'falls' until the server catches up.  I'd rather make them wait and have a more seamless
world loaded.  Hell we can give them a loading screen."  **PlayerReady at that later moment is enough**;
the server seeing the loading's progress is "over kill but I like where your head is at".  **The loading
bar stays up through the drawing**, saying so.

## Where the ground sits in Unity (asked 2026-10-03, session 1; being talked through)

Jacob, after the ground worked: "it loaded the whole damn world with the split perfectly" (Alpha flat to
the west, Omega's hills to the east; a grass material in place of GOLD for now), and "we need to origin
this at -1 on Y I think so characters I put down at 0 are on top of it".  Today a block at y fills y to
y+1, so the ground's top at 0 is at Unity's y 1, and anything stood at 0 is a block deep in it.  Open: a
client-only shift (the whole ground drawn a block lower, so a block's top face is at its own y) or the
server's spawn at y 1 with the drawing as it is; and which the server's own idea of "standing on" follows.
**His answer**: the drawing stays as it is: "actually I'll jsut set characters to stand on top of 0 that
seems easier".  So a block at y fills y to y+1, and a character standing on the ground at 0 is at y 1.
