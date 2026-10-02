// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/PlayerFiles.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Where every file the game keeps for a player goes (the HUD's layout; in
// the editor, dev mode's ticket from Soundcheck): one folder in the user's
// own folder on that computer, the same one Soundcheck uses,
//   Linux:   ~/.config/unity3d/FluffyByte/Opus.Ensemble/
//   Windows: %USERPROFILE%\AppData\LocalLow\FluffyByte\Opus.Ensemble\
// Unity's own folder for this (persistentDataPath) sits beside it, but
// Unity names that one after the Product Name and changes the dot to an
// underscore (Opus_Ensemble), so we take the company's folder above it and
// name ours ourselves.

using System.IO;
using UnityEngine;

namespace Opus
{
    public static class PlayerFiles
    {
        public const string FolderName = "Opus.Ensemble";

        public static string Folder
        {
            get
            {
                string company = Path.GetDirectoryName(Application.persistentDataPath);
                return Path.Combine(company, FolderName);
            }
        }

        // The full path of one of the player's files, by its name.
        public static string PathOf(string fileName)
        {
            return Path.Combine(Folder, fileName);
        }
    }
}
