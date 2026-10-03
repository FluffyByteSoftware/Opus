// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/GameConnection.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The game, over UDP on the server's game port, from the Ticket on.  Two
// threads: one listens for whatever the server sends, the other sends on
// time: the Connect every half second until the server answers, then a
// KeepAlive once a second, and an ask (at character select, or a line
// typed in the chat box) every half second until it's answered.  It sends
// nothing on its own between those, so it waits on the next thing due, not
// on a timer.  It hears the chat going out to everybody, and puts an answer
// that came in Spans back together.  At PLAY it fetches the simple
// overworld map (protocol version 11): the server's answer is the offer,
// and the sender asks for the pieces 64 at a time, as soon as the last 64
// are in or every quarter second, until they're all here.  Then the chunks
// around the character (protocol version 12), the same way: 64 at a time,
// nearest first, each unsqueezed on the listener before it goes to the
// main thread.
//
// The session ends with a Kicked, with the server going quiet, or with
// Close() (LOG OUT, quitting).  Whichever way, there's no reconnect: the
// player goes back to the launcher and starts over.

using System;
using System.Diagnostics;
using System.Net;
using System.Net.Sockets;
using System.Threading;
using Opus.World;
using Debug = UnityEngine.Debug;

namespace Opus.Net
{
    public class GameConnection
    {
        // The Connect goes every half second, for up to 10 seconds.
        const long ConnectEveryMs = 500;
        const long ConnectForMs = 10000;

        // A KeepAlive once a second, and the server sends one straight back.
        const long KeepAliveEveryMs = 1000;

        // Nothing from the server in this long and it counts as gone.  The
        // server's own wait for us is 40 seconds; we give up sooner, since a
        // player staring at a frozen screen wants to know.
        const long QuietForMs = 15000;

        // An ask is sent again every half second until it's answered, for
        // up to 10 seconds.
        const long AskEveryMs = 500;
        const long AskForMs = 10000;

        // An answer in Spans is given up 2 seconds after its first piece
        // if the rest haven't all come (Jacob, 2026-10-02: "2s is fine").
        const long PiecesForMs = 2000;

        // The map at PLAY: what's missing is asked for again after a
        // quarter second, and with no new piece in 10 seconds it's given up.
        const long MapAskAgainMs = 250;
        const long MapStallMs = 10000;

        // The chunks the same: asked again after a quarter second, and with
        // no new chunk (in, or refused for good) in 10 seconds given up.
        const long ChunkAskAgainMs = 250;
        const long ChunkStallMs = 10000;

        readonly ushort port;
        readonly UdpClient udp;
        readonly byte[] connectPacket;
        readonly Stopwatch clock = Stopwatch.StartNew();

        // Everything below is shared by the two threads and the main
        // thread, so it's only touched holding the gate.
        readonly object gate = new object();
        bool connected;
        bool closed;
        long nextConnect;
        long nextKeepAlive;
        long lastHeard;

        // The ask waiting for its answer, or null.  One at a time, the same
        // as the server takes them.
        uint lastAsk;
        byte[] askPacket;
        long askFirstSent;
        long nextAsk;

        // The pieces of the answer to that ask, while it comes in Spans:
        // each piece's bytes in its place, null until it's here.
        byte[][] pieces;
        int piecesIn;
        long firstPiece;

        // The simple overworld map while it comes in, or null, and when its
        // next request is due.
        MapDownload map;
        long nextMapAsk;

        // How far along the map is, in whole percent, last time Session was
        // told, so it's told a hundred times and not four thousand.
        int mapPercentSaid;

        // Where the character will stand and how many chunks each way it
        // sees, from the offer (version 12).
        float standX;
        float standY;
        float standZ;
        int view;

        // The chunks around it while they come in, or null, and when the
        // next request is due.
        ChunkDownload chunks;
        long nextChunkAsk;

        // How many of the nearest were in last time Session was told.
        int nearDoneSaid;

        // Rung to wake the sender early: a new ask, the map's last pieces
        // in, or closing.
        readonly AutoResetEvent wake = new AutoResetEvent(false);

        GameConnection(string host, ushort port, string token)
        {
            this.port = port;
            connectPacket = new PacketWriter(Protocol.Connect).String(token).ForUdp();

            // Connected, in UDP's sense: it only sends to the server, and
            // only hears from the server.
            udp = new UdpClient();
            udp.Connect(host, port);
        }

        // Starts the session with the Ticket's token.  Throws if the socket
        // can't be made, which Session reports.
        public static GameConnection Start(string host, ushort port, string token)
        {
            var game = new GameConnection(host, port, token);

            var listener = new Thread(game.Listen);
            listener.Name = "Game listen";
            listener.IsBackground = true;
            listener.Start();

            var sender = new Thread(game.SendOnTime);
            sender.Name = "Game send";
            sender.IsBackground = true;
            sender.Start();
            return game;
        }

        // An ask, sent with the next ask number and then its fields, all of
        // them strings (a name; a uuid and the typed word; a uuid; the line
        // typed in the chat box).  It replaces any ask still waiting.  From
        // the main thread.
        public void Ask(byte kind, params string[] fields)
        {
            var packet = new PacketWriter(kind);
            lock (gate)
            {
                if (closed)
                    return;
                lastAsk++;
                packet.U32(lastAsk);
                foreach (string field in fields)
                    packet.String(field);
                askPacket = packet.ForUdp();
                pieces = null;
                askFirstSent = clock.ElapsedMilliseconds;
                nextAsk = askFirstSent;
            }
            wake.Set();
        }

        // Ends it from our side: LOG OUT, or the game closing.  A Goodbye
        // tells the server at once, rather than after its 40 seconds.
        public void Close(bool sayGoodbye)
        {
            lock (gate)
            {
                if (closed)
                    return;
                closed = true;
                if (sayGoodbye && connected)
                    SendNow(new[] { Protocol.Goodbye });
            }
            udp.Close();
            wake.Set();
        }

        // The map is in and kept: the chunks around where the character
        // will stand, from the offer.  How many PlayerReady waits on, the
        // nearest, and the column they're round.  From the main thread.
        public int FetchChunks(out short columnX, out short columnZ)
        {
            int near;
            columnX = 0;
            columnZ = 0;
            lock (gate)
            {
                if (closed)
                    return 0;
                chunks = new ChunkDownload(standX, standY, standZ, view, clock.ElapsedMilliseconds);
                nextChunkAsk = 0;
                nearDoneSaid = 0;
                near = chunks.NearCount;
                columnX = chunks.ColumnX;
                columnZ = chunks.ColumnZ;
                Debug.Log("Game: asking for " + chunks.Count + " chunks around column " + chunks.ColumnX + ", "
                          + chunks.ColumnZ + ", " + view + " each way, the nearest " + near + " first.");
            }
            wake.Set();
            return near;
        }

        // ---------------------------------------------------------------
        // Sending, on time
        // ---------------------------------------------------------------

        void SendOnTime()
        {
            while (true)
            {
                long waitMs;
                lock (gate)
                {
                    if (closed)
                        return;
                    long now = clock.ElapsedMilliseconds;

                    if (!connected)
                    {
                        if (now >= ConnectForMs)
                        {
                            EndHolding("The server's game port (UDP " + port + ") didn't answer.");
                            return;
                        }
                        if (now >= nextConnect)
                        {
                            SendNow(connectPacket);
                            nextConnect = now + ConnectEveryMs;
                        }
                        waitMs = nextConnect - now;
                    }
                    else
                    {
                        if (now - lastHeard >= QuietForMs)
                        {
                            Debug.Log("Game: nothing from the server in " + QuietForMs / 1000 + " s.");
                            EndHolding("The server stopped answering.");
                            return;
                        }
                        if (now >= nextKeepAlive)
                        {
                            SendNow(new[] { Protocol.KeepAlive });
                            nextKeepAlive = now + KeepAliveEveryMs;
                        }
                        waitMs = Math.Min(nextKeepAlive, lastHeard + QuietForMs) - now;

                        if (askPacket != null)
                        {
                            if (pieces != null && now - firstPiece >= PiecesForMs)
                            {
                                byte kind = askPacket[0];
                                Debug.Log("Game: only " + piecesIn + " of " + pieces.Length + " pieces of the "
                                          + "answer to " + Protocol.NameOf(kind) + " came in " + PiecesForMs / 1000
                                          + " s.");
                                askPacket = null;
                                pieces = null;
                                MainThread.Post(() => Session.AskUnanswered(this, kind));
                            }
                            else if (now - askFirstSent >= AskForMs)
                            {
                                byte kind = askPacket[0];
                                askPacket = null;
                                pieces = null;
                                Debug.Log("Game: no answer to " + Protocol.NameOf(kind) + " in " + AskForMs / 1000
                                          + " s.");
                                MainThread.Post(() => Session.AskUnanswered(this, kind));
                            }
                            else
                            {
                                if (now >= nextAsk)
                                {
                                    SendNow(askPacket);
                                    nextAsk = now + AskEveryMs;
                                }
                                waitMs = Math.Min(waitMs, nextAsk - now);
                                if (pieces != null)
                                    waitMs = Math.Min(waitMs, firstPiece + PiecesForMs - now);
                            }
                        }

                        if (map != null)
                        {
                            if (now - map.LastNew >= MapStallMs)
                            {
                                string why = "no new piece of it in " + MapStallMs / 1000 + " s (" + map.Have + " of "
                                             + map.Count + " in)";
                                map = null;
                                Debug.Log("Game: gave up on the world's map: " + why + ".");
                                MainThread.Post(() => Session.MapFailed(this, why));
                            }
                            else
                            {
                                if (now >= nextMapAsk)
                                {
                                    SendNow(map.NextRequest());
                                    nextMapAsk = now + MapAskAgainMs;
                                }
                                waitMs = Math.Min(waitMs, Math.Min(nextMapAsk, map.LastNew + MapStallMs) - now);
                            }
                        }

                        if (chunks != null)
                        {
                            if (now - chunks.LastNew >= ChunkStallMs)
                            {
                                string why = "no new chunk in " + ChunkStallMs / 1000 + " s (" + chunks.Missing()
                                             + ")";
                                chunks = null;
                                Debug.Log("Game: gave up on the chunks: " + why + ".");
                                MainThread.Post(() => Session.GroundFailed(this, why));
                            }
                            else
                            {
                                if (now >= nextChunkAsk)
                                {
                                    SendNow(chunks.NextRequest());
                                    nextChunkAsk = now + ChunkAskAgainMs;
                                }
                                waitMs = Math.Min(waitMs, Math.Min(nextChunkAsk, chunks.LastNew + ChunkStallMs) - now);
                            }
                        }
                    }
                }
                wake.WaitOne((int)Math.Max(1, waitMs));
            }
        }

        // Holding the gate.  A send that fails is left to the resends and
        // the quiet timer; UDP promises nothing anyway.
        void SendNow(byte[] packet)
        {
            try
            {
                udp.Send(packet, packet.Length);
            }
            catch (SocketException)
            {
            }
            catch (ObjectDisposedException)
            {
            }
        }

        // ---------------------------------------------------------------
        // Listening
        // ---------------------------------------------------------------

        void Listen()
        {
            var from = new IPEndPoint(IPAddress.Any, 0);
            while (true)
            {
                byte[] data;
                try
                {
                    data = udp.Receive(ref from);
                }
                catch (SocketException e) when (e.SocketErrorCode == SocketError.ConnectionReset
                                                || e.SocketErrorCode == SocketError.ConnectionRefused)
                {
                    // The OS heard "nobody's on that port" back from one of
                    // our sends.  Maybe the server isn't up yet; the Connect
                    // keeps trying and the quiet timer has the last word.
                    continue;
                }
                catch (Exception e)
                {
                    lock (gate)
                    {
                        if (closed)
                            return;
                    }
                    Debug.Log("Game: the UDP socket broke (" + e.Message + ").");
                    End("Lost the connection to the server.");
                    return;
                }

                if (data.Length == 0)
                    continue;
                lock (gate)
                {
                    if (closed)
                        return;
                    // Anything at all from the server counts as hearing it.
                    lastHeard = clock.ElapsedMilliseconds;
                }

                try
                {
                    Heard(new PacketReader(data));
                }
                catch (ProtocolException e)
                {
                    Debug.LogWarning("Game: the server sent " + e.Message + ".  Ignored.");
                }
            }
        }

        void Heard(PacketReader packet)
        {
            switch (packet.Kind)
            {
                case Protocol.KeepAlive:
                    return;

                case Protocol.ConnectResult:
                {
                    byte answer = packet.U8();
                    string message = packet.String();
                    packet.End();
                    lock (gate)
                    {
                        // A resent Connect gets the same answer again.
                        if (connected)
                            return;
                        if (answer == Protocol.Welcome)
                        {
                            connected = true;
                            nextKeepAlive = clock.ElapsedMilliseconds + KeepAliveEveryMs;
                        }
                    }
                    Debug.Log("Game: the server said " + answer + ", \"" + message + "\".");
                    if (answer == Protocol.Welcome)
                        MainThread.Post(() => Session.Welcomed(this));
                    else
                        End(message);
                    return;
                }

                case Protocol.Kicked:
                {
                    uint reason = packet.U32();
                    packet.End();
                    Debug.Log("Game: kicked, reason " + reason + ".");
                    End(Protocol.KickedSays(reason));
                    return;
                }

                // The chat going out to everybody in the world, finished
                // lines, oldest first.  No ask number: it isn't answered.
                case Protocol.ChatDelivery:
                {
                    byte count = packet.U8();
                    var lines = new string[count];
                    for (int i = 0; i < count; i++)
                        lines[i] = packet.String();
                    packet.End();
                    MainThread.Post(() => Session.ChatCame(this, lines));
                    return;
                }

                case Protocol.Span:
                    Piece(packet);
                    return;

                case Protocol.PleaseWait:
                    Waiting(packet);
                    return;

                case Protocol.OverworldMapPiece:
                    MapPiece(packet);
                    return;

                case Protocol.ChunkPiece:
                    ChunkPieceCame(packet);
                    return;

                case Protocol.ChunkRefused:
                    ChunkRefusedCame(packet);
                    return;
            }

            if (!Protocol.CarriesAsk(packet.Kind))
            {
                Debug.Log("Game: the server sent " + Protocol.NameOf(packet.Kind) + ", which this client doesn't "
                          + "handle yet.  Ignored.");
                return;
            }

            // An answer to an ask.  Only the one we're waiting on counts; an
            // answer to an older one, or a second copy, is ignored.
            uint ask = packet.U32();
            lock (gate)
            {
                if (askPacket == null || ask != lastAsk)
                    return;
                askPacket = null;
            }
            Answered(packet);
        }

        // The server is working on our ask and says it'll take a moment
        // (a character still being saved from its last session, say): the
        // words to show meanwhile.  Not the answer, which follows under the
        // same ask number.  The ask's clock starts again, so the server's
        // wait isn't counted against the 10 seconds we give it.
        void Waiting(PacketReader packet)
        {
            uint ask = packet.U32();
            string words = packet.String();
            packet.End();
            lock (gate)
            {
                if (askPacket == null || ask != lastAsk)
                    return;
                askFirstSent = clock.ElapsedMilliseconds;
            }
            Debug.Log("Game: the server says to wait, \"" + words + "\".");
            MainThread.Post(() => Session.AskWaiting(this, words));
        }

        // One piece of an answer too big for one packet.  The pieces' bytes
        // put back together in order are the answer, type byte and all, read
        // as if it had come whole.  A piece for an older ask, or one we
        // already have (the ask's resend brings every piece again), is
        // ignored.
        void Piece(PacketReader packet)
        {
            uint ask = packet.U32();
            byte piece = packet.U8();
            byte count = packet.U8();
            byte[] bytes = packet.Rest();
            if (count == 0 || piece == 0 || piece > count)
                throw new ProtocolException("a Span saying it's piece " + piece + " of " + count);

            byte[] whole;
            lock (gate)
            {
                if (askPacket == null || ask != lastAsk)
                    return;
                if (pieces == null || pieces.Length != count)
                {
                    pieces = new byte[count][];
                    piecesIn = 0;
                    firstPiece = clock.ElapsedMilliseconds;
                    // The sender keeps the 2 seconds, so it has to know.
                    wake.Set();
                }
                if (pieces[piece - 1] == null)
                {
                    pieces[piece - 1] = bytes;
                    piecesIn++;
                }
                if (piecesIn < count)
                    return;

                int length = 0;
                foreach (byte[] p in pieces)
                    length += p.Length;
                whole = new byte[length];
                int at = 0;
                foreach (byte[] p in pieces)
                {
                    Array.Copy(p, 0, whole, at, p.Length);
                    at += p.Length;
                }
                pieces = null;
            }
            Debug.Log("Game: an answer in " + count + " pieces, " + whole.Length + " bytes, put back together.");
            Heard(new PacketReader(whole));
        }

        // One piece of the map: its number, then its bytes.  One for a map
        // we aren't fetching, or that we already have, is let go.  The last
        // one hands the whole map to Session, which checks it and keeps it.
        void MapPiece(PacketReader packet)
        {
            uint number = packet.U32();
            byte[] bytes = packet.Rest();
            long received;
            uint size;
            byte[] whole = null;
            long tookMs = 0;
            lock (gate)
            {
                long now = clock.ElapsedMilliseconds;
                if (map == null || !map.Take(number, bytes, now))
                    return;
                received = map.Received;
                size = map.Size;
                if (map.Done)
                {
                    whole = map.Whole();
                    tookMs = now - map.Started;
                    map = null;
                }
                else if (map.AskedAllIn)
                {
                    // The next 64 can go now.
                    nextMapAsk = 0;
                    wake.Set();
                }
            }

            int percent = (int)(received * 100 / size);
            if (whole != null)
            {
                double seconds = System.Math.Max(tookMs, 1) / 1000.0;
                Debug.Log("Game: the world's map is in, " + size + " bytes in " + seconds.ToString("0.00") + " s ("
                          + (size / seconds / 1000000.0).ToString("0.0") + " MB/s).");
                MainThread.Post(() => Session.MapArrived(this, whole));
            }
            else if (percent != mapPercentSaid)
            {
                mapPercentSaid = percent;
                MainThread.Post(() => Session.MapProgress(this, received, size));
            }
        }

        // One piece of a chunk: its place, which piece of how many, then
        // the bytes.  Once the chunk's all here it's unsqueezed, here on the
        // listener, and goes to the main thread for the Ground.
        void ChunkPieceCame(PacketReader packet)
        {
            var place = new ChunkPlace(packet.I16(), packet.I16(), packet.U8());
            byte piece = packet.U8();
            byte count = packet.U8();
            byte[] bytes = packet.Rest();

            byte[] squeezed;
            lock (gate)
            {
                if (chunks == null)
                    return;
                squeezed = chunks.TakePiece(place, piece, count, bytes);
            }
            if (squeezed == null)
                return;

            Chunk chunk = null;
            string trouble = null;
            try
            {
                chunk = Chunk.Unsqueeze(place, squeezed);
            }
            catch (System.IO.InvalidDataException e)
            {
                trouble = e.Message;
            }

            lock (gate)
            {
                if (chunks == null)
                    return;
                long now = clock.ElapsedMilliseconds;
                if (chunk != null)
                {
                    if (!chunks.Arrived(chunk, now))
                        return;
                    MainThread.Post(() => Session.ChunkArrived(this, chunk));
                }
                else
                {
                    if (!chunks.Unreadable(place, now))
                        return;
                    Debug.LogWarning("Game: chunk " + place + " didn't unsqueeze (" + trouble + ").  Left empty.");
                }
                ChunkDone(now);
            }
        }

        // A chunk the server won't send, and why.  "Not yet" is asked
        // again; the other two are the end of it.
        void ChunkRefusedCame(PacketReader packet)
        {
            var place = new ChunkPlace(packet.I16(), packet.I16(), packet.U8());
            byte why = packet.U8();
            packet.End();
            lock (gate)
            {
                if (chunks == null)
                    return;
                long now = clock.ElapsedMilliseconds;
                if (!chunks.Refused(place, why, now))
                    return;
                if (why == Protocol.ChunkUnavailable)
                    Debug.LogWarning("Game: the server can't send chunk " + place + " this run.  Left empty.");
                ChunkDone(now);
            }
        }

        // Holding the gate, after a chunk came in or was refused for good:
        // how far along the nearest are, PlayerReady once they're all in,
        // the summary once every chunk is, and the next 64 at once if the
        // last 64 are done.
        void ChunkDone(long now)
        {
            if (chunks.NearDone != nearDoneSaid)
            {
                nearDoneSaid = chunks.NearDone;
                int have = chunks.NearDone;
                int need = chunks.NearCount;
                MainThread.Post(() => Session.GroundProgress(this, have, need));
                if (chunks.NearAllDone)
                    MainThread.Post(() => Session.NearGroundIn(this));
            }
            if (chunks.AllDone)
            {
                string summary = chunks.Summary(now);
                chunks = null;
                MainThread.Post(() => Session.GroundAllIn(this, summary));
            }
            else if (chunks.AskedAllIn)
            {
                nextChunkAsk = 0;
                wake.Set();
            }
        }

        // The answer to the ask we were waiting on, its ask number read.
        void Answered(PacketReader packet)
        {
            switch (packet.Kind)
            {
                case Protocol.CharacterListDelivery:
                {
                    byte count = packet.U8();
                    var characters = new CharacterEntry[count];
                    for (int i = 0; i < count; i++)
                    {
                        characters[i] = new CharacterEntry
                        {
                            Uuid = packet.String(),
                            Name = packet.String(),
                            Slot = packet.U8(),
                            Playable = packet.U8() != 0,
                        };
                    }
                    packet.End();
                    Debug.Log("Game: the account has " + count + (count == 1 ? " character." : " characters."));
                    MainThread.Post(() => Session.CharactersCame(this, characters));
                    return;
                }

                case Protocol.CharacterCreateResult:
                case Protocol.CharacterDeleteResult:
                {
                    byte kind = packet.Kind;
                    byte answer = packet.U8();
                    string message = packet.String();
                    packet.End();
                    Debug.Log("Game: " + Protocol.NameOf(kind) + " " + answer + ", \"" + message + "\".");
                    if (kind == Protocol.CharacterCreateResult)
                        MainThread.Post(() => Session.CreateAnswered(this, answer, message));
                    else
                        MainThread.Post(() => Session.DeleteAnswered(this, answer, message));
                    return;
                }

                case Protocol.CharacterEnteredWorld:
                {
                    string uuid = packet.String();
                    string name = packet.String();
                    float x = packet.F32();
                    float y = packet.F32();
                    float z = packet.F32();
                    packet.End();
                    Debug.Log("Game: " + name + " (" + uuid + ") is in the world at " + x + ", " + y + ", " + z
                              + ".");
                    var standing = new UnityEngine.Vector3(x, y, z);
                    MainThread.Post(() => Session.EnteredWorld(this, name, standing));
                    return;
                }

                // PLAY's answer (version 11): the character is loaded, and
                // this is the map to fetch first.  The sender starts asking
                // for it at once.
                case Protocol.OverworldMapOffer:
                {
                    uint size = packet.U32();
                    ushort pieceBytes = packet.U16();
                    uint count = packet.U32();
                    string hash = packet.String();
                    // Where the character will stand and how many chunks
                    // each way it sees (version 12), for asking for the
                    // chunks around it once the map is in.
                    float x = packet.F32();
                    float y = packet.F32();
                    float z = packet.F32();
                    byte sees = packet.U8();
                    packet.End();
                    Debug.Log("Game: the world's map is " + size + " bytes in " + count + " pieces, SHA-256 " + hash
                              + ".");
                    try
                    {
                        lock (gate)
                        {
                            map = new MapDownload(size, pieceBytes, count, hash, clock.ElapsedMilliseconds);
                            nextMapAsk = 0;
                            mapPercentSaid = 0;
                            standX = x;
                            standY = y;
                            standZ = z;
                            view = sees;
                        }
                    }
                    catch (ProtocolException e)
                    {
                        string why = "the server offered " + e.Message;
                        MainThread.Post(() => Session.MapFailed(this, why));
                        return;
                    }
                    wake.Set();
                    MainThread.Post(() => Session.MapOffered(this, hash, size));
                    return;
                }

                case Protocol.CommandAccepted:
                {
                    packet.End();
                    Debug.Log("Game: accepted.");
                    MainThread.Post(() => Session.AskAccepted(this));
                    return;
                }

                case Protocol.CommandRefused:
                {
                    string why = packet.String();
                    packet.End();
                    Debug.Log("Game: refused, \"" + why + "\".");
                    MainThread.Post(() => Session.AskRefused(this, why));
                    return;
                }

                // /who's answer: the time it ran, whether it's /who list,
                // and the characters in the world, A to Z.
                case Protocol.WhoDelivery:
                {
                    uint seconds = packet.U32();
                    bool listed = packet.U8() == 1;
                    ushort count = packet.U16();
                    var characters = new WhoEntry[count];
                    for (int i = 0; i < count; i++)
                    {
                        var character = new WhoEntry { Name = packet.String() };
                        if (listed)
                        {
                            character.X = packet.I32();
                            character.Y = packet.I32();
                            character.Z = packet.I32();
                        }
                        characters[i] = character;
                    }
                    packet.End();
                    Debug.Log("Game: /who, " + count + (count == 1 ? " character" : " characters")
                              + (listed ? ", listed." : "."));
                    var who = new WhoAnswer { Seconds = seconds, Listed = listed, Characters = characters };
                    MainThread.Post(() => Session.WhoCame(this, who));
                    return;
                }

                default:
                    Debug.Log("Game: the server answered with " + Protocol.NameOf(packet.Kind) + ", which this "
                              + "client doesn't handle yet.");
                    return;
            }
        }

        // ---------------------------------------------------------------
        // Ending
        // ---------------------------------------------------------------

        // From the listener: the session is over from the server's side.
        void End(string why)
        {
            lock (gate)
            {
                EndHolding(why);
            }
        }

        // Holding the gate.  Closes the socket, wakes the sender so it
        // stops, and tells Session once.
        void EndHolding(string why)
        {
            if (closed)
                return;
            closed = true;
            udp.Close();
            wake.Set();
            MainThread.Post(() => Session.GameEnded(this, why));
        }
    }

    // One of the account's characters, as character select lists them.
    public class CharacterEntry
    {
        public string Uuid;
        public string Name;
        public int Slot;          // 1 to 3
        public bool Playable;     // false: its save wouldn't load this run, so it's greyed out
    }

    // /who's answer, as the server sent it.  The chat box draws it.
    public class WhoAnswer
    {
        public uint Seconds;          // when it ran, in seconds since midnight UTC
        public bool Listed;           // /who list: each character with where it stands
        public WhoEntry[] Characters; // A to Z
    }

    // A character in /who's answer.  X, Y and Z are whole blocks, and only
    // in a /who list.
    public class WhoEntry
    {
        public string Name;
        public int X;
        public int Y;
        public int Z;
    }
}
