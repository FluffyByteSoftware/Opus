// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/WhoBox.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Draws /who's answer as my old MUD's box, to the chat box's width in
// characters: the banner, the time it ran in the player's own time zone,
// the names in columns (or a line each with where it stands, for /who
// list), and the count written out.  At 79 wide:
//
//   -----------------------======] Forgotten Legends [======-----------------------
//                             Fri Oct  2 03:53:24 2026
//   ----------------------------------] Players [----------------------------------
//   Aldric   Bujin    Eetius   Guesty   Kriket   Malachy  Trzk     Zeleya
//   -----------------> There are eight legends currently online. <-----------------
//
// It only lines up in a monospaced font, which is why the chat box has one.

using System;
using System.Collections.Generic;
using System.Globalization;
using Opus.Net;

namespace Opus.Hud
{
    public static class WhoBox
    {
        public static List<string> Draw(WhoAnswer who, int width)
        {
            // The date is this computer's, the time the server's.
            DateTime midnight = DateTime.SpecifyKind(DateTime.UtcNow.Date, DateTimeKind.Utc);
            DateTime ran = midnight.AddSeconds(who.Seconds).ToLocalTime();
            CultureInfo plain = CultureInfo.InvariantCulture;
            string stamp = ran.ToString("ddd MMM ", plain) + ran.Day.ToString(plain).PadLeft(2)
                           + ran.ToString(" HH:mm:ss yyyy", plain);

            var lines = new List<string>
            {
                Centred("======] Forgotten Legends [======", '-', width),
                Centred(stamp, ' ', width).TrimEnd(),
                Centred("] Players [", '-', width),
            };

            WhoEntry[] characters = who.Characters;
            if (who.Listed)
            {
                foreach (WhoEntry c in characters)
                    lines.Add("[" + c.Name + "] is currently at [" + c.X + ", " + c.Y + ", " + c.Z + "]");
            }
            else if (characters.Length > 0)
            {
                // Columns as wide as the longest name and two spaces, as
                // many to a row as fit.  The last column's two spaces can
                // hang off the end, since they're trimmed.
                int longest = 0;
                foreach (WhoEntry c in characters)
                    longest = Math.Max(longest, c.Name.Length);
                int column = longest + 2;
                int across = Math.Max(1, (width + 2) / column);

                for (int start = 0; start < characters.Length; start += across)
                {
                    string row = "";
                    for (int i = start; i < Math.Min(start + across, characters.Length); i++)
                        row += characters[i].Name.PadRight(column);
                    lines.Add(row.TrimEnd());
                }
            }

            int count = characters.Length;
            string footer = count == 1
                ? "There is one legend currently online."
                : "There are " + Translator.NumberToWords(count) + " legends currently online.";
            lines.Add(Centred("> " + footer + " <", '-', width));
            return lines;
        }

        // The text in the middle of a line this wide, filled out either
        // side.  Text longer than the line is left as it is.
        static string Centred(string text, char fill, int width)
        {
            int pad = width - text.Length;
            if (pad <= 0)
                return text;
            int left = pad / 2;
            return new string(fill, left) + text + new string(fill, pad - left);
        }
    }
}
