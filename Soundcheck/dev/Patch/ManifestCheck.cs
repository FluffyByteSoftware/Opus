// File:       Opus/Soundcheck/dev/Patch/ManifestCheck.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// The check in user mode: the install folder hashed and held against the
// manifest fetched from the web address.  A file the manifest lists that
// isn't there, or isn't the same bytes, fails the check.  A file that's
// there and not listed is said but doesn't fail it: a patcher mends files
// and doesn't delete them, and the server will have its own look at the
// report one day.  Downloading what's off isn't built yet (TODO.md), so a
// fail is words the player reads.

using System;
using System.Collections.Generic;
using System.Text;
using System.Threading;

namespace Opus.Patch
{
    public class ManifestCheck
    {
        // How many files the manifest lists, and what the install came to.
        public int Listed;
        public long InstallBytes;

        // Relative paths, sorted.
        public List<string> Missing = new List<string>();
        public List<string> Changed = new List<string>();
        public List<string> Extra = new List<string>();

        public bool Passed
        {
            get { return Missing.Count == 0 && Changed.Count == 0; }
        }

        // Hashes the install (slow: call it off the window's thread, with
        // progress told after each file, on that thread) and compares.
        public static ManifestCheck Run(string install, Manifest stamp, Action<int, int, string> progress,
                                        CancellationToken cancel)
        {
            Manifest local = Manifest.Of(install, stamp.Platform, stamp.ClientVersion, progress, cancel);

            var have = new Dictionary<string, ManifestFile>(StringComparer.Ordinal);
            foreach (ManifestFile file in local.Files)
                have[file.Path] = file;

            var check = new ManifestCheck { Listed = stamp.Files.Count, InstallBytes = local.TotalBytes };
            foreach (ManifestFile want in stamp.Files)
            {
                ManifestFile got;
                if (!have.TryGetValue(want.Path, out got))
                    check.Missing.Add(want.Path);
                else if (got.Size != want.Size || got.Sha256 != want.Sha256)
                    check.Changed.Add(want.Path);
                have.Remove(want.Path);
            }
            foreach (string left in have.Keys)
                check.Extra.Add(left);
            check.Missing.Sort(StringComparer.Ordinal);
            check.Changed.Sort(StringComparer.Ordinal);
            check.Extra.Sort(StringComparer.Ordinal);
            return check;
        }

        // What's wrong, in a sentence for the status box: the counts, and
        // the first few names.
        public string Trouble()
        {
            var words = new StringBuilder();
            if (Missing.Count > 0)
                words.Append(Missing.Count).Append(Missing.Count == 1 ? " file missing" : " files missing");
            if (Changed.Count > 0)
            {
                if (words.Length > 0)
                    words.Append(", ");
                words.Append(Changed.Count).Append(Changed.Count == 1 ? " file changed" : " files changed");
            }
            var names = new List<string>();
            names.AddRange(Missing);
            names.AddRange(Changed);
            words.Append(" (").Append(FirstFew(names)).Append(")");
            return words.ToString();
        }

        // Up to three names, then "and N more".
        public static string FirstFew(List<string> names)
        {
            const int shown = 3;
            var words = new StringBuilder();
            for (int i = 0; i < names.Count && i < shown; i++)
            {
                if (i > 0)
                    words.Append(", ");
                words.Append(names[i]);
            }
            if (names.Count > shown)
                words.Append(" and ").Append(names.Count - shown).Append(" more");
            return words.ToString();
        }
    }
}
