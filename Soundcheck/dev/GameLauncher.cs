// File:       Opus/Soundcheck/dev/GameLauncher.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Finds the game and starts it with the ticket.  The game lives beside this
// program, since the two ship as one package; --game on the command line
// points at a build somewhere else (Jacob's, while Soundcheck runs out of
// bin/Debug/), and that path is remembered in the player folder, since
// Ensemble starts Soundcheck again on its way out and can't pass --game
// along.  The ticket goes in the game's environment, never on its command
// line (design/soundcheck.md: a command line is readable by every user on
// the machine).  The token is never logged.

using System;
using System.Diagnostics;
using System.IO;
using System.Text.Json;

namespace Opus.Soundcheck
{
    public static class GameLauncher
    {
        // The contract with Ensemble (its Net/Ticket.cs): what goes in its
        // environment.
        public const string ServerVariable = "OPUS_SERVER";
        public const string UdpPortVariable = "OPUS_UDP_PORT";
        public const string TokenVariable = "OPUS_TOKEN";
        public const string LauncherVariable = "OPUS_SOUNDCHECK";

        // The way back: Ensemble starts Soundcheck again with why the
        // session ended, and whether it was something gone wrong ("1"), for
        // the status box.
        public const string SessionOverVariable = "OPUS_SESSION_OVER";
        public const string SessionTroubleVariable = "OPUS_SESSION_TROUBLE";

        // Why the last session ended, as Ensemble left it in our
        // environment, or null when the launcher wasn't started by the game.
        public static string SessionOver(out bool trouble)
        {
            string why = Environment.GetEnvironmentVariable(SessionOverVariable);
            trouble = Environment.GetEnvironmentVariable(SessionTroubleVariable) == "1";
            return string.IsNullOrEmpty(why) ? null : why;
        }

        // What Unity calls the program: Linux, then Windows.
        static readonly string[] Names = { "Ensemble.x86_64", "Ensemble", "Ensemble.exe" };

        // Where --game's path is remembered.  Nothing secret in it.
        public const string DevFileName = "soundcheck_dev.json";

        public static string DevFilePath
        {
            get { return PlayerFiles.PathOf(DevFileName); }
        }

        class DevFile
        {
            public string Game { get; set; }
        }

        static readonly JsonSerializerOptions Pretty = new JsonSerializerOptions { WriteIndented = true };

        // The game to start: --game's path (kept for next time), else the
        // game beside this program, else the path an earlier --game left.
        // Null, with why, when there's none.
        public static string Find(string asked, out string why)
        {
            why = null;
            if (!string.IsNullOrEmpty(asked))
            {
                if (!File.Exists(asked))
                {
                    why = "there's no game at " + asked + " (--game)";
                    return null;
                }
                string full = Path.GetFullPath(asked);
                Remember(full);
                return full;
            }

            string folder = AppContext.BaseDirectory;
            foreach (string name in Names)
            {
                string beside = Path.Combine(folder, name);
                if (File.Exists(beside))
                    return beside;
            }

            string remembered = Remembered();
            if (remembered == null)
            {
                why = "there's no game beside the launcher (" + folder + ").  Start Soundcheck with --game and the "
                      + "game's path to point at a build";
                return null;
            }
            if (!File.Exists(remembered))
            {
                why = "there's no game beside the launcher (" + folder + "), and the one --game pointed at last "
                      + "time is gone (" + remembered + ")";
                return null;
            }
            Log.Say("Play: using the game --game pointed at last time, " + remembered + ".");
            return remembered;
        }

        // Starts the game with the ticket in its environment.  False, with
        // why, when it couldn't be started.
        public static bool Start(string game, string host, ushort udpPort, string token, out string why)
        {
            try
            {
                var start = new ProcessStartInfo(game);
                start.UseShellExecute = false;
                start.WorkingDirectory = Path.GetDirectoryName(game);
                start.EnvironmentVariables[ServerVariable] = host;
                start.EnvironmentVariables[UdpPortVariable] = udpPort.ToString();
                start.EnvironmentVariables[TokenVariable] = token;
                start.EnvironmentVariables[LauncherVariable] = OwnPath();
                // The last session's farewell doesn't go round again.
                start.EnvironmentVariables.Remove(SessionOverVariable);
                start.EnvironmentVariables.Remove(SessionTroubleVariable);
                Process.Start(start);
                why = null;
                return true;
            }
            catch (Exception e)
            {
                why = "the game at " + game + " couldn't be started (" + e.Message + ")";
                return false;
            }
        }

        // This program's own path, for the game's way back.  Under
        // `dotnet run` the process can be dotnet itself, which started
        // again alone is no launcher, so then it's the program beside the
        // dll.
        public static string OwnPath()
        {
            string path = Environment.ProcessPath ?? Environment.GetCommandLineArgs()[0];
            string name = Path.GetFileNameWithoutExtension(path);
            if (string.Equals(name, "dotnet", StringComparison.OrdinalIgnoreCase))
            {
                string program = "Opus.Soundcheck" + (RuntimeInfo.IsWindows ? ".exe" : "");
                string host = Path.Combine(AppContext.BaseDirectory, program);
                if (File.Exists(host))
                    return host;
            }
            return path;
        }

        static string Remembered()
        {
            string path = DevFilePath;
            if (!File.Exists(path))
                return null;
            try
            {
                DevFile file = JsonSerializer.Deserialize<DevFile>(File.ReadAllText(path));
                return file == null || string.IsNullOrEmpty(file.Game) ? null : file.Game;
            }
            catch (Exception e)
            {
                Log.Warn("Play: " + path + " couldn't be read (" + e.Message + ").");
                return null;
            }
        }

        static void Remember(string game)
        {
            string path = DevFilePath;
            string temp = path + ".tmp";
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(path));
                File.WriteAllText(temp, JsonSerializer.Serialize(new DevFile { Game = game }, Pretty));
                File.Move(temp, path, true);
            }
            catch (Exception e)
            {
                Log.Warn("Play: " + path + " couldn't be written (" + e.Message + ").");
            }
        }
    }
}
