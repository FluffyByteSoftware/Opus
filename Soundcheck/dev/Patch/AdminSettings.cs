// File:       Opus/Soundcheck/dev/Patch/AdminSettings.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// What admin mode remembers between runs, so Jacob doesn't type the client
// folder in every time: the folder, the version he gave it, and where the
// manifest was written.  One small file beside Remember Me in the player
// folder (PlayerFiles.cs).  Nothing secret in it.

using System;
using System.IO;
using System.Text.Json;
using Opus.Soundcheck;

namespace Opus.Patch
{
    public class AdminSettings
    {
        public const string FileName = "soundcheck_admin.json";

        public string ClientFolder { get; set; }
        public string ClientVersion { get; set; }
        public string WriteTo { get; set; }

        static readonly JsonSerializerOptions Pretty = new JsonSerializerOptions { WriteIndented = true };

        public static string FilePath
        {
            get { return PlayerFiles.PathOf(FileName); }
        }

        // The remembered settings, or a blank set when there's no file or
        // it doesn't read.
        public static AdminSettings Load()
        {
            string path = FilePath;
            if (!File.Exists(path))
                return new AdminSettings();
            try
            {
                return JsonSerializer.Deserialize<AdminSettings>(File.ReadAllText(path)) ?? new AdminSettings();
            }
            catch (Exception e)
            {
                Log.Warn("Admin: " + path + " couldn't be read (" + e.Message + ").  Starting blank.");
                return new AdminSettings();
            }
        }

        public void Save()
        {
            string path = FilePath;
            string temp = path + ".tmp";
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(path));
                File.WriteAllText(temp, JsonSerializer.Serialize(this, Pretty));
                File.Move(temp, path, true);
            }
            catch (Exception e)
            {
                Log.Warn("Admin: " + path + " couldn't be written (" + e.Message + ").");
            }
        }
    }
}
