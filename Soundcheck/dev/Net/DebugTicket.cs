// File:       Opus/Soundcheck/dev/Net/DebugTicket.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Debug mode's hand-off.  Normally Soundcheck starts Ensemble and puts the
// ticket in its environment, but an Ensemble running inside Unity's editor
// is already running, so there's nothing to start: in debug mode (--debug)
// the ticket is written to this file in the player folder instead, and the
// editor's Ensemble reads it.  Debug mode only: a token on the disk is a
// token somebody else on the machine could read, which is fine for Jacob
// testing a fix and for nobody else.  The token is good once, for 30
// seconds, so a stale file is harmless.

using System;
using System.IO;
using System.Text.Json;
using Opus.Soundcheck;

namespace Opus.Net
{
    public class DebugTicket
    {
        public const string FileName = "debug_ticket.json";

        public string Host { get; set; }
        public ushort UdpPort { get; set; }
        public string Token { get; set; }

        // When it was written, UTC with a Z, so the reader can tell a fresh
        // ticket from last week's.
        public string Issued { get; set; }

        static readonly JsonSerializerOptions Pretty = new JsonSerializerOptions { WriteIndented = true };

        public static string FilePath
        {
            get { return PlayerFiles.PathOf(FileName); }
        }

        // Writes the ticket whole: a temp file, then over the old one.
        // Throws if it can't; the screen says so.
        public static void Save(string host, ushort udpPort, string token)
        {
            var ticket = new DebugTicket
            {
                Host = host,
                UdpPort = udpPort,
                Token = token,
                Issued = DateTime.UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss'Z'"),
            };
            string path = FilePath;
            string temp = path + ".tmp";
            Directory.CreateDirectory(Path.GetDirectoryName(path));
            File.WriteAllText(temp, JsonSerializer.Serialize(ticket, Pretty));
            File.Move(temp, path, true);
        }
    }
}
