<!--
File:       Opus/Documentation/LLM/SIMPLE_OVERWORLD_MAP.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- simple_overworld.map

How to read `Content/world/simple_overworld.map`, down to the byte.  Written for whoever writes Ensemble's
reader, or me with a hex viewer open.  Conductor's half is `Conductor/dev/gameworld/src/overworld.rs`.
This document is the contract, the same as REGION_MAP.md is for `region.map`: when the code and this
disagree, it's the code that gets fixed.

File version **1**.  The version goes up when anything about the layout changes, and it sits near the
front of the file, so a reader built for another version can stop right there.

## What it is

The world's rough shape, for the client to draw the distance with.  The ground is cut into **patches**,
16 by 16 blocks, and each patch is two numbers: **how high it is** (the average of its columns' top
blocks) and **what it's made of on top** (the kind most of its columns have on top, which the client
colours it by).  Designed with Jacob on 2026-10-03.

What's **not** in it, on purpose: anything under the ground, and any single block.  Jacob: "didn't want to
give them the entire worlds voxel information so they could find all the secrets".  Near the player, the
real chunks come from the server and win over it.  Not squeezed ("no squeezing concern").

## Where it lives and who touches it

- **Where**: `Content/world/simple_overworld.map`, in the game's save, gitignored.  A `world_size` change
  deletes it with the rest of the world.
- **Who writes it**: GameWorld (`conductor-gameworld`), on START SERVER, when it's missing or is another
  world's (another seed or size), before the first chunk goes out.  So the door doesn't open without it.
  If it can't be written, the door stays shut that run, with an Error on the bell.  It's made from the
  ground's rules, never from anybody's digging, so it's always safe to write over.
- **Who reads it**: Ensemble.  Conductor sends it over UDP at every PLAY, in pieces of 1,024 bytes, behind
  a loading bar, with its SHA-256 (PROTOCOL.md, "The map at PLAY", protocol version 11), and Ensemble keeps
  it as `simple_overworld.map` in the player's folder (`PlayerFiles.PathOf()`), written over every time.
  GameWorld keeps the file's bytes in memory until STOP SERVER (`conductor_gameworld::overworld_map()`),
  and networking sends them as they are.
- **When it changes**: today, only with a new world.  Once blocks can change, the one world save
  (`world_save_seconds`) will write it again when a top block has changed.

## The layout

Every number is little-endian.  28 bytes of header, then 4 bytes a patch.

| Bytes | Type      | What it is |
|-------|-----------|------------|
| 8     | ASCII     | `OPUSOVWM`, the tag |
| 2     | u16       | The version, `1` |
| 8     | u64       | The seed the world was made from, the same as `region.map`'s, so a map from another world is known for one |
| 2     | i16       | The westmost block's x: the west edge of the first patch (-8192 at `world_size` 16) |
| 2     | i16       | The southmost block's z (-8192) |
| 2     | u16       | A patch's side, in blocks (16) |
| 2     | u16       | How many patches east-west (1024 at `world_size` 16) |
| 2     | u16       | How many patches north-south (1024) |
| 4 x every patch | | The patches: the south line first, and in a line, west to east |

Each patch:

| Bytes | Type | What it is |
|-------|------|------------|
| 2     | i16  | Its height: the average of the y of its 256 columns' top blocks, rounded to the nearest whole block, a half going away from 0 (1.5 is 2, -1.5 is -2) |
| 2     | u16  | Its top: the block kind most of its columns have on top, the lower number on a tie.  The same numbers as a chunk's blocks: AIR 0, DIRT 1, STONE 2, WOOD 3, GOLD 4, BEDROCK 5 |

A column's **top block** is its highest block that isn't AIR.  The patch `across` from the west edge and
`up` from the south edge (both counted in patches) is at byte `28 + (up * width + across) * 4`, and
covers blocks x `west + across * side` to 15 more, z `south + up * side` to 15 more.

Sizes: **4,194,332 bytes** at `world_size` 16 and **16,777,244** at 32 (the largest), 28 + 4 x patches.

## A worked example

A made-up file, two patches east-west and one north-south, from block -16,-8, world seed 42.  Every byte,
in order (`overworld.rs` reads it back in a test, and writes it again byte for byte):

```text
4F 50 55 53 4F 56 57 4D   "OPUSOVWM"
01 00                     version 1
2A 00 00 00 00 00 00 00   seed 42
F0 FF                     west -16
F8 FF                     south -8
10 00                     a patch is 16 blocks a side
02 00                     2 patches east-west
01 00                     1 patch north-south
00 00 01 00               patch 0,0: height 0, DIRT          blocks x -16 to -1, z -8 to 7
FD FF 02 00               patch 1,0: height -3, STONE        blocks x 0 to 15, z -8 to 7
```

36 bytes.  In a real world the first two numbers after the tag are always a multiple of 16, but a reader
doesn't need them to be.

## Reading it, step by step

1. Read 8 bytes.  If they aren't `OPUSOVWM`, it isn't this file: stop.
2. Read the version.  If it isn't 1, stop and say so.
3. Read the seed, west, south, side, width and depth.  A side of 0 is wrong: stop.
4. The rest of the file must be exactly `width * depth * 4` bytes.  Check that before making anything that
   size: a damaged header can claim billions of patches.
5. Read the patches, the south line first, west to east.

## Reading it in C# (for Ensemble)

The sketch it started as.  Ensemble's own is `Assets/Code/World/SimpleOverworldMap.cs` (session 2), the same
checks, reading the bytes fetched at PLAY (`FromBytes()`) rather than a path.  `BinaryReader` always reads
little-endian, whatever the machine.

```csharp
using System.IO;
using System.Text;

public sealed class SimpleOverworldMap
{
    public ulong Seed;
    public int West, South, Side, Width, Depth;
    public short[] Heights;   // one a patch, the south line first, west to east
    public ushort[] Tops;     // the block kind on top, the same order

    public static SimpleOverworldMap Read(string path)
    {
        using var reader = new BinaryReader(File.OpenRead(path));
        if (Encoding.ASCII.GetString(reader.ReadBytes(8)) != "OPUSOVWM")
            throw new InvalidDataException("not a simple overworld map");
        ushort version = reader.ReadUInt16();
        if (version != 1)
            throw new InvalidDataException($"version {version}, and this reads version 1");

        var map = new SimpleOverworldMap();
        map.Seed = reader.ReadUInt64();
        map.West = reader.ReadInt16();
        map.South = reader.ReadInt16();
        map.Side = reader.ReadUInt16();
        map.Width = reader.ReadUInt16();
        map.Depth = reader.ReadUInt16();
        if (map.Side == 0)
            throw new InvalidDataException("its patches are 0 blocks a side");

        long patches = (long)map.Width * map.Depth;
        if (reader.BaseStream.Length - reader.BaseStream.Position != patches * 4)
            throw new InvalidDataException("the file isn't the size its header says");
        map.Heights = new short[patches];
        map.Tops = new ushort[patches];
        for (long i = 0; i < patches; i++)
        {
            map.Heights[i] = reader.ReadInt16();
            map.Tops[i] = reader.ReadUInt16();
        }
        return map;
    }

    /// The patch block x, z is in, as its index, or -1 outside the map.
    /// Rounds down, so block -1 is in the patch west of block 0's.
    public long PatchAt(int blockX, int blockZ)
    {
        int across = FloorDiv(blockX - West, Side), up = FloorDiv(blockZ - South, Side);
        if (across < 0 || across >= Width || up < 0 || up >= Depth)
            return -1;
        return (long)up * Width + across;
    }

    static int FloorDiv(int a, int b) => (int)System.Math.Floor((double)a / b);
}
```

## Looking at a real one

From `Conductor/dev`, this prints the header and the patch a block is in (the two numbers at the end are
the block's x and z).  Python 3, nothing to install:

```text
python3 -c "import struct,sys;d=open(sys.argv[1],'rb').read();t,v,s,w,so,sd,wd,dp=struct.unpack_from('<8sHQhhHHH',d);print(t.decode(),'version',v,'seed',s,'| blocks x',w,'to',w+wd*sd-1,'z',so,'to',so+dp*sd-1,'| patch',sd,'blocks,',wd,'by',dp,'| size',len(d),'should be',28+wd*dp*4);x,z=map(int,sys.argv[2:4]);a,u=(x-w)//sd,(z-so)//sd;h,k=struct.unpack_from('<hH',d,28+(u*wd+a)*4);print('block',x,z,'-> patch',a,u,'-> height',h,'top',k,['AIR','DIRT','STONE','WOOD','GOLD','BEDROCK'][k] if k<6 else 'UNKNOWN')" /opt/storage/Coding/Opus/Content/world/simple_overworld.map 0 0
```

## Limits, and what changing it means

- **The size is in the header**, not fixed in the format: a reader takes west, south, side, width and
  depth from it and never assumes a world size or a patch size.  Width and depth go to 65,535 patches.
- **Heights from -32,768 to 32,767**, far more than the world's -32 to +319, so a mountain needs no new
  version.
- **Anything else** (a third number a patch, the patches in another order, squeezing) means the version
  goes up, `overworld.rs` and this document change together, and a line goes in the history below.

## Version history

- **1** (2026-10-03): the first.  Patches 16 by 16, an average height and the commonest top block each.
