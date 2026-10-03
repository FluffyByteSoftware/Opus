// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/World/Ground.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The chunks the game has, by place: the ground around the character, as
// it came from the server after PLAY.  Main thread only (Session puts them
// here as they come).  Emptied when the session ends.  Nothing draws them
// here; GroundView hears Added and Cleared and draws them.  Conductor
// calls its own the Terrain; here that's Unity's name for its own
// terrain, so it's the Ground.

using System;
using System.Collections.Generic;

namespace Opus.World
{
    public static class Ground
    {
        static readonly Dictionary<ChunkPlace, Chunk> chunks = new Dictionary<ChunkPlace, Chunk>();

        // A chunk came in; every chunk went (the session ended, or a new
        // PLAY).  For whatever draws them.
        public static event Action<Chunk> Added;
        public static event Action Cleared;

        public static int Count
        {
            get { return chunks.Count; }
        }

        public static void Put(Chunk chunk)
        {
            chunks[chunk.Place] = chunk;
            if (Added != null)
                Added(chunk);
        }

        // The chunk at a place, or null if it isn't here.
        public static Chunk Get(ChunkPlace place)
        {
            Chunk chunk;
            return chunks.TryGetValue(place, out chunk) ? chunk : null;
        }

        // The block at world block x, y, z, or null if its chunk isn't
        // here.  Rounds down, so block -1 is in chunk -1.
        public static ushort? BlockAt(int x, int y, int z)
        {
            int row = FloorDiv(y - Chunk.LowestBlock, Chunk.Side);
            if (row < 0 || row >= Chunk.Rows)
                return null;
            var place = new ChunkPlace((short)FloorDiv(x, Chunk.Side), (short)FloorDiv(z, Chunk.Side), (byte)row);
            Chunk chunk = Get(place);
            if (chunk == null)
                return null;
            return chunk.BlockAt(x - place.X * Chunk.Side, y - Chunk.LowestBlock - row * Chunk.Side,
                                 z - place.Z * Chunk.Side);
        }

        // How many chunks are kept as their blocks, not as one kind.
        public static int HeldAsBlocks()
        {
            int held = 0;
            foreach (Chunk chunk in chunks.Values)
            {
                if (!chunk.AllOneKind)
                    held++;
            }
            return held;
        }

        public static void Clear()
        {
            chunks.Clear();
            if (Cleared != null)
                Cleared();
        }

        static int FloorDiv(int a, int b)
        {
            int q = a / b;
            return (a % b != 0 && (a < 0) != (b < 0)) ? q - 1 : q;
        }
    }
}
