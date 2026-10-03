// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/World/Chunk.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// One chunk of the ground as the server sent it: 32 blocks a side, placed
// by x and z in chunks and its row, 0 to 10 from the bottom (row 0 is
// blocks -32 to -1).  A chunk all of one kind (most of a view is air) is
// kept as just that kind; any other as its 32,768 blocks.  The server
// sends a chunk squeezed as runs; PROTOCOL.md's "A chunk, squeezed" is the
// contract, byte for byte, and Unsqueeze() is written from it.

using System;
using System.IO;

namespace Opus.World
{
    // Where a chunk sits: x and z in chunks east and north from 0,0, and
    // its row.  The same three numbers as on the wire.
    public readonly struct ChunkPlace : IEquatable<ChunkPlace>
    {
        public readonly short X;
        public readonly short Z;
        public readonly byte Row;

        public ChunkPlace(short x, short z, byte row)
        {
            X = x;
            Z = z;
            Row = row;
        }

        public bool Equals(ChunkPlace other)
        {
            return X == other.X && Z == other.Z && Row == other.Row;
        }

        public override bool Equals(object other)
        {
            return other is ChunkPlace place && Equals(place);
        }

        public override int GetHashCode()
        {
            return (X << 16 | (ushort)Z) ^ Row << 8;
        }

        public override string ToString()
        {
            return X + "," + Z + " row " + Row;
        }
    }

    // The block numbers, the world's.  A number never changes once it's
    // out there.
    public static class Blocks
    {
        public const ushort Air = 0;
        public const ushort Dirt = 1;
        public const ushort Stone = 2;
        public const ushort Wood = 3;
        public const ushort Gold = 4;
        public const ushort Bedrock = 5;

        public static string NameOf(ushort kind)
        {
            switch (kind)
            {
                case Air: return "AIR";
                case Dirt: return "DIRT";
                case Stone: return "STONE";
                case Wood: return "WOOD";
                case Gold: return "GOLD";
                case Bedrock: return "BEDROCK";
                default: return "block " + kind;
            }
        }
    }

    public sealed class Chunk
    {
        public const int Side = 32;
        public const int BlockCount = Side * Side * Side;

        // The world is eleven chunks tall, and row 0 starts at block -32.
        public const int Rows = 11;
        public const int LowestBlock = -32;

        // The only way a chunk is squeezed so far: runs.
        const byte SqueezedAsRuns = 1;

        public readonly ChunkPlace Place;

        // How many bytes it came in, squeezed.
        public readonly int SqueezedBytes;

        // The kinds of block in it and how many of each, in the order they
        // first turn up.  Counted from the runs, for the Console's summary.
        public readonly ushort[] Kinds;
        public readonly int[] KindCounts;

        // Its blocks, in the server's order: (y * 32 + z) * 32 + x.  Null
        // when the chunk is all one kind, which is OnlyKind.
        readonly ushort[] blocks;
        public readonly ushort OnlyKind;

        Chunk(ChunkPlace place, int squeezedBytes, ushort[] kinds, int[] kindCounts, ushort[] blocks)
        {
            Place = place;
            SqueezedBytes = squeezedBytes;
            Kinds = kinds;
            KindCounts = kindCounts;
            this.blocks = blocks;
            OnlyKind = blocks == null ? kinds[0] : (ushort)0;
        }

        public bool AllOneKind
        {
            get { return blocks == null; }
        }

        // The block at x, y, z inside the chunk, each 0 to 31.
        public ushort BlockAt(int x, int y, int z)
        {
            if (blocks == null)
                return OnlyKind;
            return blocks[(y * Side + z) * Side + x];
        }

        // The chunk from its squeezed bytes.  Throws InvalidDataException
        // saying what's wrong: another way of squeezing, a bad run, too few
        // bytes, or bytes left over.
        public static Chunk Unsqueeze(ChunkPlace place, byte[] bytes)
        {
            try
            {
                using (var reader = new BinaryReader(new MemoryStream(bytes)))
                {
                    if (reader.ReadByte() != SqueezedAsRuns)
                        throw new InvalidDataException("not squeezed as runs");
                    int count = reader.ReadUInt16();
                    if (count < 1 || count > BlockCount)
                        throw new InvalidDataException(count + " kinds");
                    var kinds = new ushort[count];
                    for (int i = 0; i < count; i++)
                        kinds[i] = reader.ReadUInt16();
                    var kindCounts = new int[count];

                    // One kind: the runs are only read, to be sure of them.
                    ushort[] blocks = count == 1 ? null : new ushort[BlockCount];
                    int at = 0;
                    while (at < BlockCount)
                    {
                        int first = reader.ReadByte();
                        int length = first < 128 ? first + 1 : ((reader.ReadByte() << 7) | (first & 0x7F)) + 1;
                        int kind = count > 256 ? reader.ReadUInt16() : reader.ReadByte();
                        if (kind >= count || at + length > BlockCount)
                            throw new InvalidDataException("a bad run");
                        if (blocks != null)
                            Array.Fill(blocks, kinds[kind], at, length);
                        kindCounts[kind] += length;
                        at += length;
                    }
                    if (reader.BaseStream.Position != bytes.Length)
                        throw new InvalidDataException("bytes left over");
                    return new Chunk(place, bytes.Length, kinds, kindCounts, blocks);
                }
            }
            catch (EndOfStreamException)
            {
                throw new InvalidDataException("it ends part way");
            }
        }
    }
}
