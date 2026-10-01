// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/LayoutLoader.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Finds the HUD's layout: the player's own file if there's a good one, the
// default that ships with the game if not.  Whatever it turns away, it says
// why in the log.

using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

namespace Opus.Hud
{
    public static class LayoutLoader
    {
        public const string LayoutFormat = "opus-hud-layout";
        public const int LayoutVersion = 1;
        public const string HudScreen = "hud";

        // The biggest and smallest reference a layout can have.  8K at the
        // top; anything outside this is a broken file, not a screen.
        const float SmallestWidth = 320f, SmallestHeight = 240f;
        const float BiggestWidth = 7680f, BiggestHeight = 4320f;

        // The player's layout, in Unity's folder for this player.  On Linux
        // that's ~/.config/unity3d/<company>/<product>/, on Windows
        // AppData\LocalLow\<company>\<product>\.
        public static string PlayerFilePath
        {
            get { return Path.Combine(Application.persistentDataPath, "hud_layout.json"); }
        }

        // The HUD's layout.  The player's file wins when it's there and good;
        // the default otherwise.  Null only when the default itself is
        // missing or broken, which is ours to fix, so that's an error.
        public static LayoutFile LoadHud(TextAsset defaultLayout)
        {
            string path = PlayerFilePath;
            if (File.Exists(path))
            {
                string why;
                LayoutFile mine = ReadFile(path, out why);
                if (mine != null)
                {
                    Debug.Log("HUD: using your layout, " + path);
                    return mine;
                }
                Debug.LogWarning("HUD: not using " + path + ", since " + why + ".  Using the default instead.");
            }
            else
            {
                Debug.Log("HUD: no layout of yours at " + path + ", so it's the default.");
            }

            if (defaultLayout == null)
            {
                Debug.LogError("HUD: there's no default layout.  Drag hud_default.json from Assets/Data/Layouts "
                    + "onto HudRoot's Default Layout.");
                return null;
            }

            string whyNot;
            LayoutFile layout = Read(defaultLayout.text, HudScreen, out whyNot);
            if (layout == null)
                Debug.LogError("HUD: the default layout (" + defaultLayout.name + ") can't be used, since " + whyNot
                    + ".");
            return layout;
        }

        static LayoutFile ReadFile(string path, out string why)
        {
            string text;
            try
            {
                text = File.ReadAllText(path);
            }
            catch (Exception e)
            {
                why = "it can't be read (" + e.Message + ")";
                return null;
            }
            return Read(text, HudScreen, out why);
        }

        // A layout for this screen, or null with the reason the whole file is
        // turned away.  This only checks the file as a whole; the widgets in
        // it are LayoutChecker's.
        public static LayoutFile Read(string json, string screen, out string why)
        {
            LayoutFile layout;
            try
            {
                layout = JsonUtility.FromJson<LayoutFile>(json);
            }
            catch (Exception e)
            {
                why = "it isn't JSON the game can read (" + e.Message + ")";
                return null;
            }

            if (layout == null)
            {
                why = "it's empty";
                return null;
            }
            if (layout.format != LayoutFormat)
            {
                why = "its format is \"" + layout.format + "\", and a layout's is \"" + LayoutFormat + "\"";
                return null;
            }
            if (layout.version < 1 || layout.version > LayoutVersion)
            {
                why = "it's version " + layout.version + ", and this game reads up to version " + LayoutVersion;
                return null;
            }
            if (layout.screen != screen)
            {
                why = "it's a layout for \"" + layout.screen + "\", not \"" + screen + "\"";
                return null;
            }
            if (layout.reference == null
                || layout.reference.width < SmallestWidth || layout.reference.width > BiggestWidth
                || layout.reference.height < SmallestHeight || layout.reference.height > BiggestHeight)
            {
                why = "its reference resolution is missing or isn't a screen (" + SmallestWidth + " x "
                    + SmallestHeight + " to " + BiggestWidth + " x " + BiggestHeight + ")";
                return null;
            }

            if (layout.widgets == null)
                layout.widgets = new List<LayoutEntry>();
            why = null;
            return layout;
        }

        // Back to the default: the player's file goes, so the next build
        // finds none.
        public static void ForgetPlayerLayout()
        {
            string path = PlayerFilePath;
            if (!File.Exists(path))
                return;
            try
            {
                File.Delete(path);
                Debug.Log("HUD: deleted " + path + ", back to the default.");
            }
            catch (Exception e)
            {
                Debug.LogWarning("HUD: couldn't delete " + path + " (" + e.Message + ").");
            }
        }
    }
}
