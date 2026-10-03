<!--
File:       Opus/Documentation/LLM/design/smooth-voxels.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Smooth voxels -- a brief for a discussion chat

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
