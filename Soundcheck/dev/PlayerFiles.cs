// File:       Opus/Soundcheck/dev/PlayerFiles.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Where every file the game keeps for a player goes, the same folder
// Ensemble uses (its PlayerFiles.cs), so Remember Me is one file for both:
//   Linux:   ~/.config/unity3d/FluffyByte/Opus.Ensemble/
//   Windows: %USERPROFILE%\AppData\LocalLow\FluffyByte\Opus.Ensemble\
// Ensemble gets there from Unity's persistentDataPath; we have no Unity, so
// we build the same path from the user's folders ourselves.

using System;
using System.IO;
using System.Runtime.InteropServices;

namespace Opus
{
    public static class PlayerFiles
    {
        public const string Company = "FluffyByte";
        public const string FolderName = "Opus.Ensemble";

        public static string Folder
        {
            get
            {
                if (RuntimeInfo.IsWindows)
                {
                    // LocalApplicationData is AppData\Local; LocalLow sits
                    // beside it, and that's where Unity puts a game's files.
                    string local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
                    string appData = Path.GetDirectoryName(local);
                    return Path.Combine(appData, "LocalLow", Company, FolderName);
                }
                // ApplicationData is ~/.config on Linux.
                string config = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
                return Path.Combine(config, "unity3d", Company, FolderName);
            }
        }

        // The full path of one of the player's files, by its name.
        public static string PathOf(string fileName)
        {
            return Path.Combine(Folder, fileName);
        }
    }

    // Which OS this is, asked once.
    public static class RuntimeInfo
    {
        public static readonly bool IsWindows = RuntimeInformation.IsOSPlatform(OSPlatform.Windows);
    }
}
