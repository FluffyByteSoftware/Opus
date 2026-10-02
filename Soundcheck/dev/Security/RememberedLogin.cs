// File:       Opus/Soundcheck/dev/Security/RememberedLogin.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// What Remember Me keeps between runs: the server, the username and the
// password's key (PasswordKey.cs), never the password.  One small file in
// the folder every player file goes in (PlayerFiles.cs), in the user's own
// folder on that computer.  The same file, with the same names in it, as
// Ensemble wrote, so a login remembered there is remembered here.
// The key logs in as well as the password would, so the file is worth as
// much as a saved password to whoever copies it.  What it doesn't give them
// is the password itself, to try anywhere else.

using System;
using System.IO;
using System.Text.Json;
using Opus.Soundcheck;

namespace Opus.Security
{
    public class RememberedLogin
    {
        public const string FileName = "remembered_login.json";

        // Written out as they're named here, the way Unity's JsonUtility
        // wrote them from Ensemble.
        public string ServerIp { get; set; }
        public string ServerPort { get; set; }
        public string Username { get; set; }
        public string Key { get; set; }

        static readonly JsonSerializerOptions Pretty = new JsonSerializerOptions { WriteIndented = true };

        public static string FilePath
        {
            get { return PlayerFiles.PathOf(FileName); }
        }

        // The remembered login, or null when there isn't one or the file
        // doesn't hold up.  A bad file is left where it is and said so; the
        // next SUBMIT with Remember Me ticked writes over it.
        public static RememberedLogin Load()
        {
            string path = FilePath;
            if (!File.Exists(path))
                return null;

            RememberedLogin login;
            try
            {
                login = JsonSerializer.Deserialize<RememberedLogin>(File.ReadAllText(path));
            }
            catch (Exception e)
            {
                Log.Warn("Remember Me: " + path + " couldn't be read (" + e.Message + ").  Ignored.");
                return null;
            }

            if (login == null || string.IsNullOrEmpty(login.Username) || !PasswordKey.LooksLikeKey(login.Key))
            {
                Log.Warn("Remember Me: " + path + " isn't a remembered login.  Ignored.");
                return null;
            }
            return login;
        }

        // Writes the file whole: to a temp file first, then over the old
        // one, so a crash halfway leaves the old file or the new, never half.
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
                Log.Warn("Remember Me: " + path + " couldn't be written (" + e.Message + ").");
            }
        }

        // Forgets the login: the file goes.  Nothing to do when there isn't
        // one.
        public static void Forget()
        {
            string path = FilePath;
            try
            {
                if (File.Exists(path))
                    File.Delete(path);
            }
            catch (Exception e)
            {
                Log.Warn("Remember Me: " + path + " couldn't be deleted (" + e.Message + ").");
            }
        }
    }
}
