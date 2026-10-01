<!--
File:       Opus/Documentation/LLM/REGION_MAP.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- region.map

How to read `Content/world/region.map`, down to the byte.  Written for somebody who has never seen
Conductor's code: whoever writes Ensemble's reader, or me in six months with a hex viewer open.  Conductor's
half is `Conductor/dev/gameworld/src/regionmap.rs`.  This document is the contract, the same as
PROTOCOL.md is for the packets: when the code and this disagree, it's the code that gets fixed.

File version **2**.  The version goes up when anything about the layout changes, and it sits near the
front of the file, so a reader built for another version can stop right there instead of reading garbage.

## What it is

The world (8 km a side, 0,0,0 in the middle) is cut into **regions**.  A region is a zone is a biome, "a
biome name / zone or collection of chunks" (Jacob, 2026-09-30).  The world is also cut into **chunks**,
cubes of 32 blocks a side (32 m), and every chunk is in exactly one region, never across two.

`region.map` is the list of regions, and then one byte for every chunk in the world saying which region
it's in.  That's all.  In Jacob's words it's "a summary that tells the chunks how to assemble themselves
for the server _AND_ the client": a chunk nobody has changed has no file of its own, so the map is the
only place that says what it is, and its region says how its ground is made (flat, or from a heights
file).

What's **not** in it: the blocks themselves, the hills, the players, anything that changes while the game
runs.  It's written once, when the world is made, and after that it's only ever read.

## Where it lives and who touches it

- **Where**: `Content/world/region.map`.  `Content/world/` is the game's save, and it's gitignored.
- **Who writes it**: GameWorld (`conductor-gameworld`), once, the first START SERVER that finds no map.  It
  writes Omega's heights file first and the map last, so if the server stops or dies part way through
  making the world there's no map, and the next START SERVER starts the making over.  It's never written
  again after that.
- **Who reads it**: GameWorld, on every START SERVER.  Ensemble will too, once it's decided how the client
  gets a copy (sent at login, or handed out by Soundcheck; not settled).
- **Don't edit it by hand.**  A byte out of place and the server turns the whole file away (see "When a
  file is wrong").  To make a new world, stop the server and delete `Content/world/`.

## The ground rules

- **Little-endian.**  Every number that takes more than one byte is stored small end first.  The number
  512 is `0x0200`, and in the file it's the two bytes `00 02`.  x86 and ARM machines keep numbers this way
  in memory already, and C#'s `BinaryReader` always reads this way, so there's nothing to swap.
- **Signed and unsigned.**  `u8`, `u16` and `u64` are unsigned (0 and up).  `i16` is signed, and a negative
  one is stored the usual way (two's complement): -256 is `0xFF00`, so `00 FF` in the file.
- **Sizes**: `u8` is 1 byte, `u16` and `i16` are 2, `u64` is 8.
- **Text** is UTF-8 with its length in front, never a zero at the end.
- **No padding, nothing lined up.**  Each field starts right where the last one ended.
- **Nothing after the grid.**  The file ends where the grid does.

## The layout

The first 28 bytes are always where they are.  After that, where things sit depends on how long the
region names are, so a reader walks the region list to find where the grid starts.

| Offset | Size | Type | What it is | First world |
|---|---|---|---|---|
| 0 | 8 | 8 ASCII bytes | The tag, `OPUSRMAP`.  Anything else isn't a region map. | `OPUSRMAP` |
| 8 | 2 | u16 | The version.  This document is version 2. | 2 |
| 10 | 8 | u64 | The seed the world was made from.  Kept so a lost heights file can be made again, exactly as it was. | random |
| 18 | 2 | i16 | **West**: the x of the westmost chunk. | -128 |
| 20 | 2 | i16 | **South**: the z of the southmost chunk. | -128 |
| 22 | 2 | u16 | **Width**: how many chunks east-west. | 256 |
| 24 | 2 | u16 | **Depth**: how many chunks north-south. | 256 |
| 26 | 1 | u8 | **Rows**: how many chunks are stacked up and down.  Has to be 11 today. | 11 |
| 27 | 1 | u8 | **Count**: how many regions are in the list that follows.  1 to 255. | 2 |
| 28 | varies | | The regions, one after another (below). | Alpha, Omega |
| after them | width x depth x rows | u8 each | **The grid**: one byte per chunk, the region it's in (below). | 720,896 bytes |

**Each region** in the list is:

| Size | Type | What it is |
|---|---|---|
| 1 | u8 | Its **ground**: how a chunk nobody has changed is built.  `0` is flat (dirt at 0, stone under it down to -30, BEDROCK at -31 and -32, air over it).  `1` is heights (the same layers, with the dirt at the height its heights file gives for each column).  Nothing else is allowed. |
| 1 | u8 | How many bytes its name takes.  1 to 255. |
| that many | UTF-8 | Its name, `Alpha`.  The name is also its folder, `Content/world/Regions/Alpha/`, and, in lowercase, the front of its files' names, `alpha_-015_003_01.chunk`. |

A region's **number** is its place in this list, counting from 0.  In the first world Alpha is 0 and
Omega is 1.  That number is what the grid holds.

## The grid

After the last region comes one byte for every chunk the map covers: `width x depth x rows` of them,
720,896 for the first world.  Each byte is the number of the region that chunk is in.

They go in this order:

1. The **bottom row** of chunks first (row 0), then row 1 above it, and so on up to the top (row 10).
2. Inside a row, the **southmost line** first, going north.
3. Inside a line, **west to east**.

So to find the byte for chunk `x`, `z` in row `row`:

```text
index  = (row * depth + (z - south)) * width + (x - west)
offset = where the grid starts + index
```

A chunk is outside the map when `x - west` isn't from 0 to width - 1, `z - south` isn't from 0 to
depth - 1, or `row` isn't from 0 to rows - 1.  Outside the map is outside the world: there's no ground
there.

## Which way is which

- **x** is east-west, bigger to the east.  **z** is north-south, bigger to the north.  **y** is up and down,
  bigger going up.  (Unity has y up and z forward too, so a Unity scene with north along +z lines up with no
  swapping.)
- A **block** is 1 m a side, the same as Minecraft's.  The world runs from block -4096 to block 4095 each
  way, with 0,0,0 (the GOLD block) in the middle.
- A **chunk** is 32 blocks a side.  Chunk 0,0 has its south-west corner at block 0,0, so chunk `x` covers
  blocks `32x` to `32x + 31`.  Chunks run -128 to 127 each way.
- **Rows**: eleven, from y -32 to +319, 32 blocks each.  Row 0 is y -32 to -1, row 1 is 0 to 31 (the
  ground), and row `r` is `32r - 32` to `32r - 1`, up to row 10, 288 to 319.  -32 and -31 are the BEDROCK
  floor; what can be dug and built in is -30 to +319, Minecraft's top.

From a block to its chunk, divide by 32 and **round down**, toward minus infinity, not toward zero:

```text
chunk x = floor(block x / 32)
chunk z = floor(block z / 32)
row     = floor((block y + 32) / 32)
```

Rounding down matters for anything below 0.  Block x -1 is in chunk -1, not chunk 0.  In C#, `-1 / 32` is
0, which is wrong here; `(int)Math.Floor(-1 / 32.0)` is -1, which is right.  (Rust has `div_euclid` for
this, and Python's `//` already rounds down.)

## A worked example

Here are the first 42 bytes of a first world whose seed happened to be `0x0123456789ABCDEF`:

```text
4f 50 55 53 52 4d 41 50  02 00  ef cd ab 89 67 45 23 01  80 ff  80 ff  00 01  00 01  0b  02
00 05 41 6c 70 68 61  01 05 4f 6d 65 67 61
```

Read left to right:

| Bytes | Means |
|---|---|
| `4f 50 55 53 52 4d 41 50` | `OPUSRMAP`, the tag |
| `02 00` | version 2 |
| `ef cd ab 89 67 45 23 01` | the seed, 0x0123456789ABCDEF (small end first, so it reads backwards) |
| `80 ff` | west, -128 |
| `80 ff` | south, -128 |
| `00 01` | width, 256 |
| `00 01` | depth, 256 |
| `0b` | rows, 11 |
| `02` | 2 regions follow |
| `00 05 41 6c 70 68 61` | region 0: ground 0 (flat), a name 5 bytes long, `Alpha` |
| `01 05 4f 6d 65 67 61` | region 1: ground 1 (heights), a name 5 bytes long, `Omega` |

The grid starts at byte 42, and the file is 42 + 720,896 = **720,938 bytes**.

**Which region is chunk -15, 3 in row 1, the one with the ground in it?**

```text
index  = (1 * 256 + (3 - -128)) * 256 + (-15 - -128)
       = 387 * 256 + 113
       = 99,185
offset = 42 + 99,185 = 99,227
```

The byte at 99,227 is `00`: region 0, Alpha.  Its file, if anybody ever changes it, is
`Content/world/Regions/Alpha/alpha_-015_003_01.chunk`.

**Chunk 0, 0 in the top row, 10?**  `(10 * 256 + 128) * 256 + 128 = 688,256`, so byte 688,298.  It's
`01`, Omega: 0 is east of the line, and Omega is everything east of it.

**The chunk a player standing at block -100, 7, 50 is in:** x is floor(-100 / 32) = -4 (not -3), z is
floor(50 / 32) = 1, the row is floor((7 + 32) / 32) = 1.  So chunk -4, 1, row 1, which is Alpha.

## Reading it, step by step

1. Read 8 bytes.  If they aren't `OPUSRMAP`, stop: it isn't a region map.
2. Read the version (u16).  If it isn't one you know, stop.  Don't guess at a newer layout.
3. Read the seed (u64), west (i16), south (i16), width (u16), depth (u16) and rows (u8).  If rows isn't
   11, stop.
4. Read the count (u8), then that many regions: ground (u8, 0 or 1, anything else stop), name length (u8),
   the name (UTF-8, not empty).
5. Read `width x depth x rows` bytes: the grid.  If the file runs out first, stop.
6. If there's anything left in the file after the grid, stop.
7. If any byte in the grid is `count` or more (a region that isn't in the list), stop.

Conductor does exactly these checks, in this order.

## When a file is wrong

Conductor never half-reads a map.  Any one of the checks above failing, or the file being there but
unreadable, is an Error in the log and on the bell, GameWorld goes to trouble on the Services tab with
what was wrong (".../Content/world/region.map isn't right: the file ends before the chunks"), and that
run has no ground.  Since no chunk can be had around 0,0,0, the door never opens either: nobody gets in.

Conductor never writes over a map it can't read.  Somebody's world is in there.  A missing map is
different: that's a new world, and it gets made.

## Reading it in C# (for Ensemble)

A sketch, not code from the repo yet.  `BinaryReader` always reads little-endian, whatever the machine,
which is what we want.

```csharp
using System.IO;
using System.Text;

public enum Ground : byte { Flat = 0, Heights = 1 }

public sealed class RegionMap
{
    public ulong Seed;
    public int West, South, Width, Depth, Rows;
    public string[] Names;
    public Ground[] Grounds;
    public byte[] Grid;

    public static RegionMap Read(string path)
    {
        using var reader = new BinaryReader(File.OpenRead(path));
        if (Encoding.ASCII.GetString(reader.ReadBytes(8)) != "OPUSRMAP")
            throw new InvalidDataException("not a region map");
        ushort version = reader.ReadUInt16();
        if (version != 2)
            throw new InvalidDataException($"version {version}, and this reads version 2");

        var map = new RegionMap();
        map.Seed = reader.ReadUInt64();
        map.West = reader.ReadInt16();
        map.South = reader.ReadInt16();
        map.Width = reader.ReadUInt16();
        map.Depth = reader.ReadUInt16();
        map.Rows = reader.ReadByte();
        if (map.Rows != 11)
            throw new InvalidDataException($"{map.Rows} rows, and the world has 11");

        int count = reader.ReadByte();
        map.Names = new string[count];
        map.Grounds = new Ground[count];
        for (int i = 0; i < count; i++)
        {
            byte ground = reader.ReadByte();
            if (ground > 1)
                throw new InvalidDataException($"a region's ground is {ground}");
            map.Grounds[i] = (Ground)ground;
            int length = reader.ReadByte();
            map.Names[i] = Encoding.UTF8.GetString(reader.ReadBytes(length));
            if (map.Names[i].Length == 0)
                throw new InvalidDataException("a region has no name");
        }

        int cells = map.Width * map.Depth * map.Rows;
        map.Grid = reader.ReadBytes(cells);
        if (map.Grid.Length != cells)
            throw new InvalidDataException("the file ends before the chunks");
        if (reader.BaseStream.Position != reader.BaseStream.Length)
            throw new InvalidDataException("there's more after the chunks than there should be");
        foreach (byte number in map.Grid)
            if (number >= count)
                throw new InvalidDataException($"a chunk is in region {number}, and there are {count}");
        return map;
    }

    /// The region number chunk x, z, row is in, or -1 outside the world.
    public int RegionAt(int x, int z, int row)
    {
        int across = x - West, up = z - South;
        if (across < 0 || across >= Width || up < 0 || up >= Depth || row < 0 || row >= Rows)
            return -1;
        return Grid[(row * Depth + up) * Width + across];
    }

    /// The chunk a block is in.  Rounds down, so block -1 is in chunk -1.
    public static (int x, int z, int row) ChunkOf(int blockX, int blockY, int blockZ)
    {
        return (FloorDiv(blockX, 32), FloorDiv(blockZ, 32), FloorDiv(blockY + 32, 32));
    }

    static int FloorDiv(int a, int b) => (int)System.Math.Floor((double)a / b);
}
```

## Looking at a real one

From `Conductor/dev`, this prints the header, the regions, and which region one chunk is in (the three
numbers at the end are the chunk's x, z and row).  Python 3, nothing to install:

```text
python3 -c "import struct,sys;d=open(sys.argv[1],'rb').read();t,v,s,w,so,wd,dp,r,c=struct.unpack_from('<8sHQhhHHBB',d);p=[28];rs=[(d[p[0]+2:p[0]+2+d[p[0]+1]].decode(),['flat','heights'][d[p[0]]],p.__setitem__(0,p[0]+2+d[p[0]+1]))[:2] for _ in range(c)];print(t.decode(),'version',v,'seed',s,'| chunks x',w,'to',w+wd-1,'z',so,'to',so+dp-1,'rows',r,'| regions',rs,'| grid at byte',p[0],'| size',len(d),'should be',p[0]+wd*dp*r);x,z,row=map(int,sys.argv[2:5]);i=p[0]+(row*dp+z-so)*wd+x-w;print('chunk',x,z,'row',row,'-> byte',i,'-> region',d[i],rs[d[i]][0])" /opt/storage/Coding/Opus/Content/world/region.map -15 3 1
```

For a first world it prints something like:

```text
OPUSRMAP version 2 seed 81985529216486895 | chunks x -128 to 127 z -128 to 127 rows 11 | regions [('Alpha', 'flat'), ('Omega', 'heights')] | grid at byte 42 | size 720938 should be 720938
chunk -15 3 row 1 -> byte 99227 -> region 0 Alpha
```

(Your seed will be different.)  For the raw bytes, this shows the first 64:

```text
xxd -l 64 /opt/storage/Coding/Opus/Content/world/region.map
```

## Limits, and what changing it means

- **255 regions at most**, since a region's number is one byte.  **255 bytes of name at most.**
- **The world's size is in the header**, not fixed in the format, so a bigger world ("it may _grow_ later",
  Jacob) is the same version with bigger numbers.  Width and depth can go to 65,535 chunks, and west and
  south anywhere from -32,768 to 32,767.
- **Rows is 11** because the world is eleven chunks tall, -32 to +319.  A taller world, or rows that start
  somewhere else, means a new version.
- **Anything else** (more than 255 regions, a region per block column instead of per chunk, a new kind of
  ground, something new in the header) means the version goes up, `regionmap.rs` and this document change
  together, and this document gets a line saying what the new version added.  A reader then turns an old
  or new file away cleanly at step 2 instead of misreading it.
- **A new ground** (a third way to build an untouched chunk) is a new value in a region's ground byte, so
  it's a new version too: an older reader would stop at it anyway, but the version says why.

## The other two files, briefly

`region.map` says which region a chunk is in.  Two more files say what's in it, and their layouts are in
`design/world.md` under "The files":

- `Regions/<Region>/<region>.heights`: for a region whose ground is heights (Omega), how high the dirt is
  in every column.  Made with the map, from the same seed.
- `Regions/<Region>/<region>_<x>_<z>_<row>.chunk`: a chunk somebody has changed, every block in it.  If a
  chunk has one of these, it wins over whatever its region's ground would have built.

## Version history

- **1** (2026-09-30): the first.  Blocks 50 cm, 512 by 512 chunks, two rows from -16.
- **2** (2026-10-01): blocks 1 m, Minecraft's size (Jacob: "more in line with the size of a Minecraft
  voxel").  The layout is the same; the numbers in it changed: 256 by 256 chunks (still 8 km), eleven rows
  from -32 to +319, BEDROCK at -31 and -32.  A version 1 map is turned away, and the world made again.
