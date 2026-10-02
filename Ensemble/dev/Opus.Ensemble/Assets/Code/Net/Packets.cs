// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/Packets.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Writing a packet and reading one, the way PROTOCOL.md lays out the bytes:
// numbers little-endian (lowest byte first), a string as a u32 byte count
// and then that many bytes of UTF-8.  A TCP packet goes in a frame with its
// length in front; a UDP packet doesn't, since UDP keeps packets whole.

using System;
using System.Collections.Generic;
using System.Text;

namespace Opus.Net
{
    // A packet the server sent that can't be read: too short, a string that
    // isn't UTF-8, bytes left over at the end.
    public class ProtocolException : Exception
    {
        public ProtocolException(string message) : base(message)
        {
        }
    }

    public class PacketWriter
    {
        readonly List<byte> bytes = new List<byte>();

        // Every packet starts with its type.
        public PacketWriter(byte kind)
        {
            bytes.Add(kind);
        }

        public PacketWriter U8(byte value)
        {
            bytes.Add(value);
            return this;
        }

        public PacketWriter U32(uint value)
        {
            bytes.Add((byte)value);
            bytes.Add((byte)(value >> 8));
            bytes.Add((byte)(value >> 16));
            bytes.Add((byte)(value >> 24));
            return this;
        }

        public PacketWriter String(string text)
        {
            byte[] utf8 = Encoding.UTF8.GetBytes(text);
            U32((uint)utf8.Length);
            bytes.AddRange(utf8);
            return this;
        }

        // The packet as it goes over UDP: the type, then the payload.
        public byte[] ForUdp()
        {
            return bytes.ToArray();
        }

        // The packet as it goes over TCP: the frame's length (the type and
        // the payload, not counting the length itself), then the packet.
        public byte[] ForTcp()
        {
            var framed = new byte[4 + bytes.Count];
            uint length = (uint)bytes.Count;
            framed[0] = (byte)length;
            framed[1] = (byte)(length >> 8);
            framed[2] = (byte)(length >> 16);
            framed[3] = (byte)(length >> 24);
            bytes.CopyTo(framed, 4);
            return framed;
        }
    }

    public class PacketReader
    {
        // Strict: bytes that aren't UTF-8 throw instead of turning into a
        // question mark.
        static readonly UTF8Encoding Utf8 = new UTF8Encoding(false, true);

        readonly byte[] data;
        int at;

        // The packet's type, its first byte.
        public byte Kind { get; private set; }

        // A packet, its type first.  An empty one can't be read at all.
        public PacketReader(byte[] packet)
        {
            if (packet == null || packet.Length == 0)
                throw new ProtocolException("an empty packet");
            data = packet;
            Kind = packet[0];
            at = 1;
        }

        public byte U8()
        {
            Need(1);
            return data[at++];
        }

        public ushort U16()
        {
            Need(2);
            ushort value = (ushort)(data[at] | data[at + 1] << 8);
            at += 2;
            return value;
        }

        public uint U32()
        {
            Need(4);
            uint value = (uint)(data[at] | data[at + 1] << 8 | data[at + 2] << 16 | data[at + 3] << 24);
            at += 4;
            return value;
        }

        // A 32-bit float, its four bytes the same order as a u32's.  The
        // bytes go back through BitConverter in this machine's own order,
        // so it reads right on any machine.
        public float F32()
        {
            return BitConverter.ToSingle(BitConverter.GetBytes(U32()), 0);
        }

        public string String()
        {
            uint length = U32();
            if (length > data.Length - at)
                throw new ProtocolException("a string longer than the packet");
            string text;
            try
            {
                text = Utf8.GetString(data, at, (int)length);
            }
            catch (ArgumentException)
            {
                // DecoderFallbackException is an ArgumentException.
                throw new ProtocolException("a string that isn't UTF-8");
            }
            at += (int)length;
            return text;
        }

        // Called after the last field: anything left over means the packet
        // isn't what we think it is.
        public void End()
        {
            if (at != data.Length)
                throw new ProtocolException((data.Length - at) + " bytes left over after "
                    + Protocol.NameOf(Kind) + "'s last field");
        }

        void Need(int count)
        {
            if (data.Length - at < count)
                throw new ProtocolException(Protocol.NameOf(Kind) + " is shorter than it should be");
        }
    }
}
