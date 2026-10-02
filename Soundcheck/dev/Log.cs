// File:       Opus/Soundcheck/dev/Log.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Where Soundcheck says what it's doing, for whoever started it from a
// terminal: one line a message with the time in UTC.  Ensemble has Unity's
// Console for this; we have the terminal.  On Windows a windowed program
// has no terminal, so nothing shows there until there's a log file
// (TODO.md).  Nothing here ever prints a password, a key or a token.

using System;

namespace Opus.Soundcheck
{
    public static class Log
    {
        public static void Say(string words)
        {
            Write("     ", words);
        }

        public static void Warn(string words)
        {
            Write("WARN ", words);
        }

        public static void Error(string words)
        {
            Write("ERROR", words);
        }

        static void Write(string level, string words)
        {
            Console.WriteLine(DateTime.UtcNow.ToString("yyyy-MM-dd HH:mm:ss") + "Z " + level + " " + words);
        }
    }
}
