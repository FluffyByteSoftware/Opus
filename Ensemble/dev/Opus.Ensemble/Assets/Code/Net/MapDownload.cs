// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/MapDownload.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The simple overworld map coming in at PLAY, a piece at a time (protocol
// version 11): which pieces are here, the bytes so far, and the next
// request.  GameConnection holds one while the map comes, and only
// touches it holding its gate.  The pieces are asked for 64 at a time from
// the lowest one missing; whatever didn't come is asked for again.  The
// chunks will come the same way one day ("we may as well use the same
// tool set").

namespace Opus.Net
{
    public class MapDownload
    {
        // Far more than the biggest world's map (16,777,244 bytes), so an
        // offer claiming more is one we won't make room for.
        const uint LargestMap = 64 * 1024 * 1024;

        public readonly uint Size;
        public readonly int PieceBytes;
        public readonly int Count;
        public readonly string Hash;

        readonly byte[] bytes;
        readonly bool[] have;
        int lowestMissing;

        // The pieces the last request asked for.
        int askedFrom;
        int askedCount;

        // How many pieces are here, and how many bytes they make.
        public int Have { get; private set; }
        public long Received { get; private set; }

        // When the offer came, and when the last new piece came, on
        // GameConnection's clock.
        public readonly long Started;
        public long LastNew { get; private set; }

        // The offer's numbers.  Throws ProtocolException for an offer that
        // doesn't add up: pieces that don't cover the size, or a size we
        // won't make room for.
        public MapDownload(uint size, ushort pieceBytes, uint count, string hash, long now)
        {
            if (size == 0 || size > LargestMap)
                throw new ProtocolException("an OverworldMapOffer of " + size + " bytes");
            if (pieceBytes == 0 || count != (size + pieceBytes - 1) / pieceBytes)
                throw new ProtocolException("an OverworldMapOffer of " + size + " bytes in " + count + " pieces of "
                                            + pieceBytes);
            Size = size;
            PieceBytes = pieceBytes;
            Count = (int)count;
            Hash = hash;
            bytes = new byte[size];
            have = new bool[count];
            Started = now;
            LastNew = now;
        }

        public bool Done
        {
            get { return Have == Count; }
        }

        // Every piece the last request asked for is here, so the next can go
        // without waiting out the quarter second.
        public bool AskedAllIn
        {
            get
            {
                for (int i = askedFrom; i < askedFrom + askedCount; i++)
                {
                    if (!have[i])
                        return false;
                }
                return true;
            }
        }

        // A piece off the wire.  True if it's new; a second copy, one past
        // the end, or one the wrong size is let go.
        public bool Take(uint number, byte[] piece, long now)
        {
            if (number >= Count || have[number])
                return false;
            long at = (long)number * PieceBytes;
            long expected = number == Count - 1 ? Size - at : PieceBytes;
            if (piece.Length != expected)
                return false;
            System.Array.Copy(piece, 0, bytes, at, piece.Length);
            have[number] = true;
            Have++;
            Received += piece.Length;
            LastNew = now;
            while (lowestMissing < Count && have[lowestMissing])
                lowestMissing++;
            return true;
        }

        // The next OverworldMapRequest: the lowest piece missing and up to 63
        // after it.  Only asked for while the map isn't done.
        public byte[] NextRequest()
        {
            askedFrom = lowestMissing;
            askedCount = System.Math.Min(Protocol.MapPiecesAtOnce, Count - lowestMissing);
            return new PacketWriter(Protocol.OverworldMapRequest).U32((uint)askedFrom).U8((byte)askedCount)
                .ForUdp();
        }

        // The whole map, once it's done.
        public byte[] Whole()
        {
            return bytes;
        }
    }
}
