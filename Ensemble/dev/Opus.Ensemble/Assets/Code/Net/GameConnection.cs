// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/GameConnection.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The game, over UDP on the server's game port, from the Ticket on.  Two
// threads: one listens for whatever the server sends, the other sends on
// time: the Connect every half second until the server answers, then a
// KeepAlive once a second, and an ask at character select every half
// second until it's answered.  It sends nothing on its own between those,
// so it waits on the next thing due, not on a timer.
//
// The session ends with a Kicked, with the server going quiet, or with
// Close() (LOG OUT, quitting).  Whichever way, there's no reconnect: the
// player goes back to the login screen and starts over.

using System;
using System.Diagnostics;
using System.Net;
using System.Net.Sockets;
using System.Threading;
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

        // Rung to wake the sender early: a new ask, or closing.
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

        // An ask at character select, sent with the next ask number and then
        // its fields, all of them strings (a name; a uuid and the typed
        // word; a uuid).  It replaces any ask still waiting.  From the main
        // thread.
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
                            if (now - askFirstSent >= AskForMs)
                            {
                                byte kind = askPacket[0];
                                askPacket = null;
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
                    MainThread.Post(() => Session.EnteredWorld(this, name));
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
}
