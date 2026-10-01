// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Security/RememberedLogin.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// What Remember Me keeps between runs: the server, the username and the
// password's key (PasswordKey.cs), never the password.  One small file in
// the player's own folder for the game:
//   Linux:   ~/.config/unity3d/FluffyByte Studios/Forgotten Legends/
//   Windows: %USERPROFILE%\AppData\LocalLow\FluffyByte Studios\Forgotten Legends\
// The key logs in as well as the password would, so the file is worth as
// much as a saved password to whoever copies it.  What it doesn't give them
// is the password itself, to try anywhere else.

using System;
using System.IO;
using UnityEngine;

namespace Opus.Security
{
    [Serializable]
    public class RememberedLogin
    {
        public const string FileName = "remembered_login.json";

        // Public fields because JsonUtility only reads and writes those.
        public string ServerIp;
        public string ServerPort;
        public string Username;
        public string Key;

        public static string FilePath
        {
            get { return Path.Combine(Application.persistentDataPath, FileName); }
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
                login = JsonUtility.FromJson<RememberedLogin>(File.ReadAllText(path));
            }
            catch (Exception e)
            {
                Debug.LogWarning("Remember Me: " + path + " couldn't be read (" + e.Message + ").  Ignored.");
                return null;
            }

            if (login == null || string.IsNullOrEmpty(login.Username) || !PasswordKey.LooksLikeKey(login.Key))
            {
                Debug.LogWarning("Remember Me: " + path + " isn't a remembered login.  Ignored.");
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
                File.WriteAllText(temp, JsonUtility.ToJson(this, true));
                if (File.Exists(path))
                    File.Replace(temp, path, null);
                else
                    File.Move(temp, path);
            }
            catch (Exception e)
            {
                Debug.LogWarning("Remember Me: " + path + " couldn't be written (" + e.Message + ").");
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
                Debug.LogWarning("Remember Me: " + path + " couldn't be deleted (" + e.Message + ").");
            }
        }
    }
}
