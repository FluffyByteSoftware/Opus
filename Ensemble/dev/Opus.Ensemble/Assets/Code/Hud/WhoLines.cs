// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/WhoLines.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Writes /who's answer into the chat as plain lines, EverQuest's way: a
// line a character, the one in the world longest at the top, then a blank
// line, the count, and the time it ran in the player's own time zone.
// Nothing is laid out to a width, so it reads at any size of chat box
// and a long line just wraps (Jacob, 2026-10-03, "since the chat window
// is scaleable"):
//
//   Chatter is at [0, 0, 0] [16 days, 12 minutes online]
//   Seliris is at [15, 1, 20] [3 hours, 4 minutes online]
//
//   There are 2 Legends online.
//   Sat Oct  3 14:05:09 2026

using System;
using System.Collections.Generic;
using System.Globalization;
using Opus.Net;

namespace Opus.Hud
{
    public static class WhoLines
    {
        public static List<string> Draw(WhoAnswer who)
        {
            CultureInfo plain = CultureInfo.InvariantCulture;
            var lines = new List<string>();
            foreach (WhoEntry c in who.Characters)
            {
                lines.Add(c.Name + " is at [" + c.X.ToString(plain) + ", " + c.Y.ToString(plain) + ", "
                          + c.Z.ToString(plain) + "] [" + TimeOnline(c.Online) + " online]");
            }

            int count = who.Characters.Length;
            lines.Add("");
            lines.Add(count == 1
                ? "There is 1 Legend online."
                : "There are " + count.ToString(plain) + " Legends online.");

            // The date is this computer's, the time the server's.
            DateTime midnight = DateTime.SpecifyKind(DateTime.UtcNow.Date, DateTimeKind.Utc);
            DateTime ran = midnight.AddSeconds(who.Seconds).ToLocalTime();
            lines.Add(ran.ToString("ddd MMM ", plain) + ran.Day.ToString(plain).PadLeft(2)
                      + ran.ToString(" HH:mm:ss yyyy", plain));
            return lines;
        }

        // How long a character's been in the world: days, hours and
        // minutes, the ones that are 0 left out ("16 days, 12 minutes"),
        // and "under a minute" before the first.  Seconds never show.
        static string TimeOnline(uint seconds)
        {
            uint days = seconds / 86400;
            uint hours = seconds % 86400 / 3600;
            uint minutes = seconds % 3600 / 60;

            var parts = new List<string>();
            AddPart(parts, days, "day");
            AddPart(parts, hours, "hour");
            AddPart(parts, minutes, "minute");
            return parts.Count == 0 ? "under a minute" : string.Join(", ", parts);
        }

        static void AddPart(List<string> parts, uint amount, string unit)
        {
            if (amount == 0)
                return;
            parts.Add(amount.ToString(CultureInfo.InvariantCulture) + " " + unit + (amount == 1 ? "" : "s"));
        }
    }
}
