// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/ChunkDownload.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The chunks around the character coming in after the map (protocol
// version 12): which ones we want, nearest first, the pieces of any that
// come in more than one, and the next request, the first 64 not in yet.
// "Not yet" from the server is asked again; "outside the view" and
// "unavailable" are the end of that chunk.  The nearest, the 3 by 3
// columns round the character's every row, are what PlayerReady waits on
// (Jacob, 2026-10-03: Minecraft's way); the rest come after, with the
// character in the world.  GameConnection holds one and only touches it
// holding its gate, from its listener and its sender.

using System;
using System.Collections.Generic;
using System.Text;
using Opus.World;

namespace Opus.Net
{
    public class ChunkDownload
    {
        // PlayerReady waits on the chunks this many columns round the
        // character's, every row: 3 by 3 by 11, 99 chunks, in and drawn.
        public const int NearColumns = 1;

        readonly ChunkPlace[] wanted;          // nearest first
        readonly Dictionary<ChunkPlace, int> index = new Dictionary<ChunkPlace, int>();
        readonly bool[] done;                  // in, or refused for good
        readonly bool[] near;
        int lowestNotDone;

        // A chunk in more than one piece, while it comes: each piece's
        // bytes in its place, null until it's here.
        readonly Dictionary<ChunkPlace, byte[][]> pieces = new Dictionary<ChunkPlace, byte[][]>();

        // The chunks the last request asked for.
        readonly List<int> asked = new List<int>();

        public readonly int View;
        public readonly short ColumnX;
        public readonly short ColumnZ;
        public readonly int NearCount;

        public int Done { get; private set; }
        public int NearDone { get; private set; }

        // When it started, and when the last chunk came in or was refused
        // for good, on GameConnection's clock.
        public readonly long Started;
        public long LastNew { get; private set; }

        // When the nearest were all in, from the start, or -1 before.
        public long NearTookMs { get; private set; }

        // For the Console's summary.
        int arrived;
        int outside;
        int unavailable;
        int unreadable;
        int notYet;
        int packets;
        int requests;
        long squeezedBytes;
        readonly Dictionary<ushort, long> blockCounts = new Dictionary<ushort, long>();

        // Every chunk within `view` chunks of the column the character
        // stands in, every row, east, west, north and south, the way the
        // server's view counts them.  Nearest first: by the larger of how
        // far east-west and north-south, then by how far the row is from
        // the character's.
        public ChunkDownload(float x, float y, float z, int view, long now)
        {
            View = view;
            ColumnX = (short)FloorDiv((int)Math.Floor(x), Chunk.Side);
            ColumnZ = (short)FloorDiv((int)Math.Floor(z), Chunk.Side);
            int standRow = FloorDiv((int)Math.Floor(y) - Chunk.LowestBlock, Chunk.Side);

            var places = new List<ChunkPlace>();
            for (int dz = -view; dz <= view; dz++)
            {
                for (int dx = -view; dx <= view; dx++)
                {
                    for (int row = 0; row < Chunk.Rows; row++)
                        places.Add(new ChunkPlace((short)(ColumnX + dx), (short)(ColumnZ + dz), (byte)row));
                }
            }
            places.Sort((a, b) =>
            {
                int byRing = Ring(a).CompareTo(Ring(b));
                if (byRing != 0)
                    return byRing;
                return Math.Abs(a.Row - standRow).CompareTo(Math.Abs(b.Row - standRow));
            });

            wanted = places.ToArray();
            done = new bool[wanted.Length];
            near = new bool[wanted.Length];
            for (int i = 0; i < wanted.Length; i++)
            {
                index[wanted[i]] = i;
                near[i] = Ring(wanted[i]) <= NearColumns;
                if (near[i])
                    NearCount++;
            }
            Started = now;
            LastNew = now;
            NearTookMs = -1;
        }

        public int Count
        {
            get { return wanted.Length; }
        }

        public bool AllDone
        {
            get { return Done == wanted.Length; }
        }

        public bool NearAllDone
        {
            get { return NearDone == NearCount; }
        }

        // Every chunk the last request asked for is in or refused for good,
        // so the next can go without waiting out the quarter second.
        public bool AskedAllIn
        {
            get
            {
                foreach (int i in asked)
                {
                    if (!done[i])
                        return false;
                }
                return true;
            }
        }

        // The next ChunkRequest: the first 64 not in yet, nearest first.
        // Only asked for while some aren't.
        public byte[] NextRequest()
        {
            asked.Clear();
            for (int i = lowestNotDone; i < wanted.Length && asked.Count < Protocol.ChunksAtOnce; i++)
            {
                if (!done[i])
                    asked.Add(i);
            }
            var packet = new PacketWriter(Protocol.ChunkRequest).U8((byte)asked.Count);
            foreach (int i in asked)
                packet.I16(wanted[i].X).I16(wanted[i].Z).U8(wanted[i].Row);
            requests++;
            return packet.ForUdp();
        }

        // A ChunkPiece off the wire.  The chunk's squeezed bytes once its
        // last piece is here, else null.  A piece of a chunk we didn't ask
        // for, or already have, or that doesn't add up, is let go.
        public byte[] TakePiece(ChunkPlace place, byte piece, byte count, byte[] bytes)
        {
            packets++;
            int i;
            if (!index.TryGetValue(place, out i) || done[i])
                return null;
            if (count == 0 || piece == 0 || piece > count)
                return null;
            if (count == 1)
                return bytes;

            byte[][] got;
            if (!pieces.TryGetValue(place, out got) || got.Length != count)
            {
                got = new byte[count][];
                pieces[place] = got;
            }
            got[piece - 1] = bytes;
            foreach (byte[] p in got)
            {
                if (p == null)
                    return null;
            }
            pieces.Remove(place);

            int length = 0;
            foreach (byte[] p in got)
                length += p.Length;
            var whole = new byte[length];
            int at = 0;
            foreach (byte[] p in got)
            {
                Array.Copy(p, 0, whole, at, p.Length);
                at += p.Length;
            }
            return whole;
        }

        // A chunk unsqueezed and on its way to the Ground.  False if it was
        // already done (a second copy).
        public bool Arrived(Chunk chunk, long now)
        {
            if (!Finish(chunk.Place, now))
                return false;
            arrived++;
            squeezedBytes += chunk.SqueezedBytes;
            for (int k = 0; k < chunk.Kinds.Length; k++)
            {
                long sofar;
                blockCounts.TryGetValue(chunk.Kinds[k], out sofar);
                blockCounts[chunk.Kinds[k]] = sofar + chunk.KindCounts[k];
            }
            return true;
        }

        // A chunk whose bytes wouldn't unsqueeze: the end of it, left empty,
        // the same as one the server can't send.
        public bool Unreadable(ChunkPlace place, long now)
        {
            if (!Finish(place, now))
                return false;
            unreadable++;
            return true;
        }

        // A ChunkRefused.  "Not yet" is asked again with the next request;
        // the other two are the end of that chunk.  True if that's news.
        public bool Refused(ChunkPlace place, byte why, long now)
        {
            if (why == Protocol.ChunkNotYet)
            {
                notYet++;
                return false;
            }
            if (!Finish(place, now))
                return false;
            if (why == Protocol.ChunkOutsideView)
                outside++;
            else
                unavailable++;
            return true;
        }

        bool Finish(ChunkPlace place, long now)
        {
            int i;
            if (!index.TryGetValue(place, out i) || done[i])
                return false;
            done[i] = true;
            Done++;
            if (near[i])
            {
                NearDone++;
                if (NearAllDone)
                    NearTookMs = now - Started;
            }
            LastNew = now;
            while (lowestNotDone < wanted.Length && done[lowestNotDone])
                lowestNotDone++;
            return true;
        }

        // How many chunks are missing, for the give-up's line.
        public string Missing()
        {
            return (wanted.Length - Done) + " of " + wanted.Length + " still out";
        }

        // The Console's line once they're all in, the test client's summary
        // over again, so the two can be held side by side.
        public string Summary(long now)
        {
            double seconds = Math.Max(now - Started, 1) / 1000.0;
            var line = new StringBuilder();
            line.Append("Game: the ground is in, " + wanted.Length + " chunks around column " + ColumnX + ", "
                        + ColumnZ + " (" + View + " each way, every row): " + arrived + " in, "
                        + (outside + unavailable) + " refused (" + outside + " outside the view, " + unavailable
                        + " unavailable), in " + seconds.ToString("0.00") + " s; " + squeezedBytes
                        + " bytes squeezed, " + packets + " packets, " + requests + " requests, " + notYet
                        + " \"not yet\"s.  The nearest " + NearCount + " took "
                        + (NearTookMs / 1000.0).ToString("0.00") + " s.  Blocks:");
            var kinds = new List<ushort>(blockCounts.Keys);
            kinds.Sort();
            for (int k = 0; k < kinds.Count; k++)
                line.Append((k == 0 ? " " : ", ") + Blocks.NameOf(kinds[k]) + " " + blockCounts[kinds[k]]);
            line.Append(".  " + unreadable + " didn't unsqueeze.");
            return line.ToString();
        }

        // How many columns out from the character's a chunk is.
        int Ring(ChunkPlace place)
        {
            return Math.Max(Math.Abs(place.X - ColumnX), Math.Abs(place.Z - ColumnZ));
        }

        static int FloorDiv(int a, int b)
        {
            int q = a / b;
            return (a % b != 0 && (a < 0) != (b < 0)) ? q - 1 : q;
        }
    }
}
