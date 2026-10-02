// File:       Opus/Soundcheck/dev/Patch/Mirror.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Admin mode's copy: the build folder mirrored into download/<platform>/
// in the web folder, so a player's launcher can fetch any one file by its
// own path (ManifestSource.cs).  What's new or changed is copied, what the
// build no longer has is taken out, and what the manifest skips (Unity's
// backup folder, a manifest at the root) isn't copied.  A copy keeps the
// file's times and, on Linux, its permissions, so the manifest written
// from the mirror sees the execute bit.

using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;

namespace Opus.Patch
{
    public class MirrorResult
    {
        public int Copied;
        public int Kept;
        public int Removed;
    }

    public static class Mirror
    {
        // Where a platform's copy sits under the web folder.
        public static string FolderFor(string www, string platform)
        {
            return Path.Combine(www, ManifestSource.DownloadFolder, platform);
        }

        // Mirrors `build` into `mirror`.  Progress is told (done, total,
        // the file) after each file, on the caller's thread.  Throws when
        // a file can't be copied; the screen says so.
        public static MirrorResult Run(string build, string mirror, Action<int, int, string> progress,
                                       CancellationToken cancel)
        {
            build = Path.GetFullPath(build);
            mirror = Path.GetFullPath(mirror);
            Directory.CreateDirectory(mirror);

            // The build's files, by relative path, without what's skipped.
            var sources = new List<KeyValuePair<string, string>>();
            foreach (string full in Directory.EnumerateFiles(build, "*", SearchOption.AllDirectories))
            {
                string relative = Path.GetRelativePath(build, full).Replace('\\', '/');
                if (Manifest.Skipped(relative))
                    continue;
                sources.Add(new KeyValuePair<string, string>(relative, full));
            }
            sources.Sort((a, b) => string.CompareOrdinal(a.Key, b.Key));

            var result = new MirrorResult();
            var keep = new HashSet<string>(StringComparer.Ordinal);
            int done = 0;
            foreach (KeyValuePair<string, string> source in sources)
            {
                cancel.ThrowIfCancellationRequested();
                string target = Path.Combine(mirror, source.Key.Replace('/', Path.DirectorySeparatorChar));
                keep.Add(target);
                if (Same(source.Value, target))
                {
                    result.Kept++;
                }
                else
                {
                    Directory.CreateDirectory(Path.GetDirectoryName(target));
                    // To a temp first, then over, so a copy that dies
                    // halfway never leaves a half file under the name.
                    string temp = target + Patcher.TempSuffix;
                    File.Copy(source.Value, temp, true);
                    File.Move(temp, target, true);
                    result.Copied++;
                }
                done++;
                if (progress != null)
                    progress(done, sources.Count, source.Key);
            }

            // What the build no longer has goes, and the folders left empty.
            foreach (string path in Directory.GetFiles(mirror, "*", SearchOption.AllDirectories))
            {
                if (keep.Contains(path))
                    continue;
                File.Delete(path);
                result.Removed++;
            }
            RemoveEmptyFolders(mirror);
            return result;
        }

        // Whether the copy is already the file: the same size and the same
        // last write time (File.Copy keeps the time).
        static bool Same(string source, string target)
        {
            if (!File.Exists(target))
                return false;
            var a = new FileInfo(source);
            var b = new FileInfo(target);
            return a.Length == b.Length && a.LastWriteTimeUtc == b.LastWriteTimeUtc;
        }

        static void RemoveEmptyFolders(string folder)
        {
            foreach (string sub in Directory.GetDirectories(folder))
            {
                RemoveEmptyFolders(sub);
                if (Directory.GetFileSystemEntries(sub).Length == 0)
                    Directory.Delete(sub);
            }
        }
    }
}
