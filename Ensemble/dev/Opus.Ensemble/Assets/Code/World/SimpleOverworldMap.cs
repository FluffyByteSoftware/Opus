// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/World/SimpleOverworldMap.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The simple overworld map: the world's rough shape, for drawing the
// distance.  Every 16 by 16 blocks of ground is a patch, and a patch is two
// numbers, its average height and the block most often on top, which the
// distance is coloured by.  The server sends it at every PLAY, and the game
// keeps it in the player's folder as simple_overworld.map, written over
// each time.  Documentation/LLM/SIMPLE_OVERWORLD_MAP.md is the contract,
// byte for byte; when this file and that one disagree, this file is what
// gets fixed.

using System.IO;
using System.Text;

namespace Opus.World
{
    public sealed class SimpleOverworldMap
    {
        // The file's name in the player's folder (PlayerFiles.PathOf()).
        public const string FileName = "simple_overworld.map";

        // The version this reads.  A file of another stops right there.
        public const ushort Version = 1;

        const string Tag = "OPUSOVWM";
        const int HeaderBytes = 28;

        // The map the game has now: the one fetched at the last PLAY, or
        // null before one.
        public static SimpleOverworldMap Current;

        public ulong Seed;
        public int West, South, Side, Width, Depth;
        public short[] Heights;   // one a patch, the south line first, west to east
        public ushort[] Tops;     // the block kind on top, the same order

        // The map from the file's bytes, checked the way the contract says:
        // the tag, the version, a side that isn't 0, and exactly as many
        // bytes as the header says patches.  Throws InvalidDataException
        // saying what's wrong.  BinaryReader always reads little-endian,
        // whatever the machine.
        public static SimpleOverworldMap FromBytes(byte[] bytes)
        {
            if (bytes.Length < HeaderBytes)
                throw new InvalidDataException("the map is shorter than its header");
            using (var reader = new BinaryReader(new MemoryStream(bytes)))
            {
                if (Encoding.ASCII.GetString(reader.ReadBytes(8)) != Tag)
                    throw new InvalidDataException("not a simple overworld map");
                ushort version = reader.ReadUInt16();
                if (version != Version)
                    throw new InvalidDataException("version " + version + ", and this reads version " + Version);

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
                if (bytes.Length - HeaderBytes != patches * 4)
                    throw new InvalidDataException("the map isn't the size its header says");
                map.Heights = new short[patches];
                map.Tops = new ushort[patches];
                for (long i = 0; i < patches; i++)
                {
                    map.Heights[i] = reader.ReadInt16();
                    map.Tops[i] = reader.ReadUInt16();
                }
                return map;
            }
        }

        // The patch block x, z is in, as its index, or -1 outside the map.
        // Rounds down, so block -1 is in the patch west of block 0's.
        public long PatchAt(int blockX, int blockZ)
        {
            int across = FloorDiv(blockX - West, Side);
            int up = FloorDiv(blockZ - South, Side);
            if (across < 0 || across >= Width || up < 0 || up >= Depth)
                return -1;
            return (long)up * Width + across;
        }

        static int FloorDiv(int a, int b)
        {
            int q = a / b;
            return (a % b != 0 && (a < 0) != (b < 0)) ? q - 1 : q;
        }
    }
}
