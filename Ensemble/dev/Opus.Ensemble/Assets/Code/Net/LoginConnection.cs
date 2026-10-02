// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/LoginConnection.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The login, over TLS on the server's TCP port, on a thread of its own so
// the screen keeps drawing.  It connects and brings TLS up while the
// password's key is still being made (Jacob's pick: "in the background
// while the player moves forward in login"), reads the server's Hello,
// sends the Login the moment the key is ready, and waits for the answer: a
// Ticket for the UDP port, or a LoginResult saying why not.  Then the
// connection closes.  Nothing else ever goes over TCP.
//
// Everything it has to say goes to Session on the main thread
// (MainThread.Post).  The key and the ticket's token are never logged.

using System;
using System.Diagnostics;
using System.IO;
using System.Net.Security;
using System.Net.Sockets;
using System.Security.Authentication;
using System.Security.Cryptography.X509Certificates;
using System.Threading;
using System.Threading.Tasks;
using Debug = UnityEngine.Debug;

namespace Opus.Net
{
    public class LoginConnection
    {
        // How long the server gets to take the connection.
        const int ConnectMs = 10000;

        // How long a read waits before the server counts as gone.  Long,
        // since a busy server can have us in its line for a while; it sends
        // an InLine once a second while we wait, which resets this.
        const int ReadMs = 30000;

        // How long the player gets to say what to do about the other
        // session, the same as the server's own wait.
        public const int ChoiceSeconds = 30;

        readonly string host;
        readonly ushort port;
        readonly byte[] certificate;
        readonly string version;
        readonly string username;
        readonly Task<string> key;

        TcpClient tcp;
        volatile bool cancelled;

        // Set when the server showed a certificate that isn't ours, so the
        // failed handshake can say why.
        volatile bool wrongCertificate;

        // The player's answer about the other session, from the main thread.
        readonly ManualResetEventSlim chosen = new ManualResetEventSlim(false);
        volatile bool logTheOtherOut;

        LoginConnection(string host, ushort port, byte[] certificate, string version, string username,
                        Task<string> key)
        {
            this.host = host;
            this.port = port;
            this.certificate = certificate;
            this.version = version;
            this.username = username;
            this.key = key;
        }

        // Starts the login on its own thread.  The key can still be being
        // made; the Login waits for it, and nothing before the Login does.
        public static LoginConnection Start(string host, ushort port, byte[] certificate, string version,
                                            string username, Task<string> key)
        {
            var login = new LoginConnection(host, port, certificate, version, username, key);
            var thread = new Thread(login.Run);
            thread.Name = "Login";
            thread.IsBackground = true;
            thread.Start();
            return login;
        }

        // The player's answer when the account is already in the world:
        // true logs the other session out, false hangs this one up.
        public void Choose(bool logOtherOut)
        {
            logTheOtherOut = logOtherOut;
            chosen.Set();
        }

        // Stops it wherever it is, without a word to Session.  For quitting
        // the game in the middle of a login.
        public void Cancel()
        {
            cancelled = true;
            chosen.Set();
            TcpClient open = tcp;
            if (open != null)
                open.Close();
        }

        // ---------------------------------------------------------------
        // The thread
        // ---------------------------------------------------------------

        void Run()
        {
            try
            {
                Talk();
            }
            catch (Exception e)
            {
                if (cancelled)
                    return;
                Ended(WhatWentWrong(e), false);
            }
            finally
            {
                TcpClient open = tcp;
                if (open != null)
                    open.Close();
            }
        }

        void Talk()
        {
            Status("Connecting to " + host + ":" + port + "...");
            var clock = Stopwatch.StartNew();

            tcp = new TcpClient();
            tcp.NoDelay = true;
            Task connecting = tcp.ConnectAsync(host, port);
            if (!connecting.Wait(ConnectMs))
            {
                // The connect fails once the socket is closed below.  Looked
                // at here, so it isn't reported later as an error nobody saw.
                connecting.ContinueWith(failed => failed.Exception, TaskContinuationOptions.OnlyOnFaulted);
                Ended("The server at " + host + ":" + port + " didn't answer.", false);
                return;
            }
            tcp.ReceiveTimeout = ReadMs;
            tcp.SendTimeout = ReadMs;

            using (var tls = new SslStream(tcp.GetStream(), false, CheckCertificate))
            {
                // TLS 1.2.  Unity's .NET stops there: it has no name for 1.3
                // (2026-10-02, the first compile), so Conductor takes 1.2 as
                // well as 1.3.
                tls.AuthenticateAsClient(host, null, SslProtocols.Tls12, false);
                Debug.Log("Login: TLS up with " + host + ":" + port + " in " + clock.ElapsedMilliseconds + " ms ("
                          + tls.SslProtocol + ").");

                PacketReader hello = ReadFrame(tls);
                if (hello.Kind != Protocol.Hello)
                {
                    Ended("The server didn't start with a Hello, so it's probably not a Forgotten Legends server.",
                          false);
                    return;
                }
                byte theirs = hello.U8();
                hello.End();
                if (theirs != Protocol.Version)
                {
                    Ended("This client speaks protocol version " + Protocol.Version + " and the server speaks "
                          + theirs + ". One of them needs updating.", false);
                    return;
                }

                // The key is usually ready by now.  If it isn't, this is
                // where we wait for it: the server gives us its login
                // deadline (10 seconds) from the moment we connected.
                if (!key.IsCompleted)
                    Status("Connected. Working out the password's key...");
                string made;
                try
                {
                    made = key.Result;
                }
                catch (Exception)
                {
                    // LoginForm says what went wrong in the Console.
                    Ended("The password's key couldn't be made.", false);
                    return;
                }

                Send(tls, new PacketWriter(Protocol.Login)
                    .String(version)
                    .String(Protocol.SecretWord)
                    .String(username)
                    .String(made));
                Status("Checking the password...");

                ReadAnswer(tls);
            }
        }

        // After the Login: InLine as often as the server likes, then a
        // Ticket or a LoginResult.
        void ReadAnswer(SslStream tls)
        {
            while (true)
            {
                PacketReader packet = ReadFrame(tls);
                switch (packet.Kind)
                {
                    case Protocol.InLine:
                    {
                        uint ahead = packet.U32();
                        uint ms = packet.U32();
                        packet.End();
                        if (ahead == 0)
                            Status("Checking the password...");
                        else
                            Status(ahead + (ahead == 1 ? " login" : " logins") + " ahead of you, about "
                                   + Math.Max(1u, (ms + 999) / 1000) + " s.");
                        break;
                    }

                    case Protocol.LoginResult:
                    {
                        byte answer = packet.U8();
                        string message = packet.String();
                        packet.End();
                        Debug.Log("Login: the server said " + answer + ", \"" + message + "\".");
                        if (answer != Protocol.LoginAlreadyLoggedIn)
                        {
                            Ended(message, answer == Protocol.LoginFailed);
                            return;
                        }
                        if (!AskAboutTheOtherSession(tls))
                            return;
                        break;
                    }

                    case Protocol.Ticket:
                    {
                        string token = packet.String();
                        ushort udpPort = packet.U16();
                        packet.End();
                        Debug.Log("Login: logged in.  The server's game port is UDP " + udpPort + ".");
                        if (!cancelled)
                            MainThread.Post(() => Session.TicketCame(this, host, udpPort, token));
                        return;
                    }

                    default:
                        Ended("The server sent " + Protocol.NameOf(packet.Kind) + " in the middle of the login, "
                              + "which this client doesn't expect.", false);
                        return;
                }
            }
        }

        // The account is already in the world from somewhere else.  The
        // player gets ChoiceSeconds to say whether to log that session out
        // or log this one off; no answer logs this one off and leaves the
        // other alone.  (So a shared account doesn't kick your brother off
        // because you wanted to play.)  True if the login carries on.
        bool AskAboutTheOtherSession(SslStream tls)
        {
            MainThread.Post(() => Session.AskedAboutOtherSession(this));
            bool answered = chosen.Wait(ChoiceSeconds * 1000);
            if (cancelled)
                return false;

            if (!answered || !logTheOtherOut)
            {
                // The server would close it at its own 30 seconds anyway;
                // saying so is tidier.
                try
                {
                    Send(tls, new PacketWriter(Protocol.SessionChoice).U8(Protocol.HangUp));
                }
                catch (Exception)
                {
                    // It hung up first.  Same ending.
                }
                Ended(answered
                    ? "Logged off. The other session is still playing."
                    : "No answer in " + ChoiceSeconds + " seconds, so this login was dropped. The other session "
                      + "is still playing.", false);
                return false;
            }

            Send(tls, new PacketWriter(Protocol.SessionChoice).U8(Protocol.LogTheOtherOut));
            // The server holds the ticket until the other session's
            // character has its save in the database, up to 5 seconds.
            Status("Logging the other session out...");
            return true;
        }

        // ---------------------------------------------------------------
        // Reading and writing
        // ---------------------------------------------------------------

        static void Send(SslStream tls, PacketWriter packet)
        {
            byte[] bytes = packet.ForTcp();
            tls.Write(bytes, 0, bytes.Length);
            tls.Flush();
        }

        // One frame: its length, then the packet.  Throws when the server
        // hangs up, goes quiet past ReadMs, or sends a frame it shouldn't.
        static PacketReader ReadFrame(SslStream tls)
        {
            byte[] head = ReadExactly(tls, 4);
            uint length = (uint)(head[0] | head[1] << 8 | head[2] << 16 | head[3] << 24);
            if (length < 1 || length > Protocol.LargestFrame)
                throw new ProtocolException("a frame " + length + " long");
            return new PacketReader(ReadExactly(tls, (int)length));
        }

        static byte[] ReadExactly(SslStream tls, int count)
        {
            var bytes = new byte[count];
            int got = 0;
            while (got < count)
            {
                int read = tls.Read(bytes, got, count - got);
                if (read == 0)
                    throw new EndOfStreamException();
                got += read;
            }
            return bytes;
        }

        // TLS asks this about the server's certificate.  Self-signed means
        // the usual checks always complain, so we ignore them: the one test
        // is whether it's the certificate we carry, byte for byte.
        bool CheckCertificate(object sender, X509Certificate shown, X509Chain chain, SslPolicyErrors errors)
        {
            bool ours = shown != null && ServerCertificate.Same(shown.GetRawCertData(), certificate);
            if (!ours)
                wrongCertificate = true;
            return ours;
        }

        // An exception, as words for the player.  The details go to the
        // Console.
        string WhatWentWrong(Exception e)
        {
            if (e is AggregateException && e.InnerException != null)
                e = e.InnerException;

            if (wrongCertificate)
            {
                Debug.LogWarning("Login: " + host + ":" + port + " showed a certificate that isn't the one in "
                                 + "ScreenRoot's Server Certificate slot, so it wasn't trusted.  If the server "
                                 + "made a new one, copy Content/certs/conductor.crt over "
                                 + "Assets/Data/Certs/conductor_crt.txt.");
                return "The server's certificate isn't the one this client trusts.";
            }
            if (e is AuthenticationException)
            {
                Debug.LogWarning("Login: TLS wouldn't come up with " + host + ":" + port + " (" + Explain(e) + ").");
                return "A secure connection to the server couldn't be made.";
            }
            if (e is SocketException)
            {
                var socket = (SocketException)e;
                Debug.Log("Login: couldn't connect to " + host + ":" + port + " (" + socket.SocketErrorCode + ", "
                          + e.Message + ").");
                if (socket.SocketErrorCode == SocketError.ConnectionRefused)
                    return "Nothing is listening at " + host + ":" + port + ".";
                if (socket.SocketErrorCode == SocketError.HostNotFound)
                    return "There's no server called " + host + ".";
                return "Couldn't reach the server at " + host + ":" + port + ".";
            }
            if (e is EndOfStreamException)
            {
                Debug.Log("Login: the server hung up.");
                return "The server hung up.";
            }
            if (e is ProtocolException)
            {
                Debug.LogWarning("Login: the server sent " + e.Message + ".");
                return "The server sent something this client can't read.";
            }
            if (e is IOException)
            {
                Debug.Log("Login: the connection broke (" + Explain(e) + ").");
                return "Lost the connection to the server.";
            }
            Debug.LogException(e);
            return "Something went wrong logging in.";
        }

        // An exception's message with its inner ones', since TLS errors in
        // Unity tend to keep the real reason a level down.
        static string Explain(Exception e)
        {
            string words = e.Message;
            for (Exception inner = e.InnerException; inner != null; inner = inner.InnerException)
                words += "; " + inner.Message;
            return words;
        }

        // ---------------------------------------------------------------
        // Telling Session
        // ---------------------------------------------------------------

        void Status(string words)
        {
            if (!cancelled)
                MainThread.Post(() => Session.LoginStatus(this, words));
        }

        void Ended(string why, bool wrongPassword)
        {
            if (!cancelled)
                MainThread.Post(() => Session.LoginEnded(this, why, wrongPassword));
        }
    }
}
