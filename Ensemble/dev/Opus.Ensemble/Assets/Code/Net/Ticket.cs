// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/Ticket.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The ticket for the game's UDP port: where the server is, and the
// one-time token the login earned.  The game never logs in itself; the
// launcher (Soundcheck) does, and starts the game with the ticket in its
// environment, four variables.  Reading them is here.  In the editor there
// is no launcher to start the game, so Soundcheck's --debug writes the
// ticket to a file in the player folder instead, and dev mode reads that
// file here too.  A token is good once, for 30 seconds, and is never
// logged.

using System;
using System.Globalization;
using System.IO;
using UnityEngine;

namespace Opus.Net
{
    public class Ticket
    {
        // The contract with Soundcheck (design/soundcheck.md): what it puts
        // in the game's environment when it starts it.
        public const string ServerVariable = "OPUS_SERVER";
        public const string UdpPortVariable = "OPUS_UDP_PORT";
        public const string TokenVariable = "OPUS_TOKEN";
        public const string LauncherVariable = "OPUS_SOUNDCHECK";

        // How long a token is good for, the server's rule.
        public const int GoodForSeconds = 30;

        public string Host;
        public ushort UdpPort;
        public string Token;

        // The ticket Soundcheck started the game with, or null.  `why` is
        // null when there's simply none (the game was started by hand), and
        // says what's wrong when there is one that doesn't hold up.
        public static Ticket FromEnvironment(out string why)
        {
            string host = Environment.GetEnvironmentVariable(ServerVariable);
            string portText = Environment.GetEnvironmentVariable(UdpPortVariable);
            string token = Environment.GetEnvironmentVariable(TokenVariable);
            if (string.IsNullOrEmpty(host) && string.IsNullOrEmpty(portText) && string.IsNullOrEmpty(token))
            {
                why = null;
                return null;
            }
            return Check(host, portText, token, out why);
        }

        // Soundcheck's path, from the environment, or null when the game
        // wasn't started by it.
        public static string LauncherPath()
        {
            string path = Environment.GetEnvironmentVariable(LauncherVariable);
            return string.IsNullOrEmpty(path) ? null : path;
        }

        static Ticket Check(string host, string portText, string token, out string why)
        {
            ushort port;
            if (string.IsNullOrEmpty(host))
            {
                why = "the ticket has no server address";
                return null;
            }
            if (!ushort.TryParse(portText, out port) || port == 0)
            {
                why = "the ticket's UDP port (\"" + portText + "\") isn't a port";
                return null;
            }
            if (string.IsNullOrEmpty(token))
            {
                why = "the ticket has no token";
                return null;
            }
            why = null;
            return new Ticket { Host = host, UdpPort = port, Token = token };
        }

#if UNITY_EDITOR
        // ---------------------------------------------------------------
        // Dev mode: the editor's game, with no launcher to start it
        // ---------------------------------------------------------------

        public const string DevFileName = "debug_ticket.json";

        public static string DevFilePath
        {
            get { return PlayerFiles.PathOf(DevFileName); }
        }

        // The file as Soundcheck writes it (its Net/DebugTicket.cs).
        [Serializable]
        class DevFile
        {
            public string Host;
            public int UdpPort;
            public string Token;
            public string Issued;
        }

        // The Issued stamp of the last ticket taken from the file, so the
        // same one is never taken twice: a token is good once.
        static string lastTaken;

        // A fresh ticket from Soundcheck's file, or null.  `why` says what's
        // wrong with a file that can't be used, and is null when there's
        // nothing to say: no file, one already taken, or one older than a
        // token lives.
        public static Ticket FromDevFile(out string why)
        {
            why = null;
            string path = DevFilePath;
            if (!File.Exists(path))
                return null;

            DevFile file;
            try
            {
                file = JsonUtility.FromJson<DevFile>(File.ReadAllText(path));
            }
            catch (Exception e)
            {
                why = path + " can't be read (" + e.Message + ")";
                return null;
            }
            if (file == null || string.IsNullOrEmpty(file.Issued))
            {
                why = path + " isn't a ticket";
                return null;
            }
            if (file.Issued == lastTaken)
                return null;

            DateTime issued;
            if (!DateTime.TryParseExact(file.Issued, "yyyy-MM-dd'T'HH:mm:ss'Z'", CultureInfo.InvariantCulture,
                                        DateTimeStyles.AssumeUniversal | DateTimeStyles.AdjustToUniversal, out issued))
            {
                why = path + " has a time the game can't read (" + file.Issued + ")";
                return null;
            }
            if ((DateTime.UtcNow - issued).TotalSeconds > GoodForSeconds)
                return null;

            Ticket ticket = Check(file.Host, file.UdpPort.ToString(), file.Token, out why);
            if (ticket != null)
                lastTaken = file.Issued;
            return ticket;
        }
#endif
    }
}
