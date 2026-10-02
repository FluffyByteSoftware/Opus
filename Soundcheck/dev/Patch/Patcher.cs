// File:       Opus/Soundcheck/dev/Patch/Patcher.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Mends the install: each file the check found missing or changed is
// fetched from the web folder's copy of the client (ManifestSource.cs),
// written to a temp beside the real one, checked against the manifest's
// hash, and swapped in, so a download that dies halfway leaves the old
// file whole.  A fetched copy has no permissions, so a Linux program gets
// its execute bit back.  When one of the launcher's own files was
// replaced, Soundcheck starts itself again (Restart) and the new one
// checks from the top.  On Windows a running program can't be written
// over but can be renamed, so a file that won't take the swap is renamed
// aside first, and the leftovers go on the next start (CleanUp).

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using Opus.Soundcheck;

namespace Opus.Patch
{
    public class PatchResult
    {
        // How many files landed, and their bytes.
        public int Fetched;
        public long Bytes;

        // Set when a file directly in the launcher's own folder was
        // replaced: the launcher starts itself again then.
        public bool LauncherTouched;
    }

    public static class Patcher
    {
        // A download on its way in, beside the file it will replace.
        public const string TempSuffix = ".patch";

        // A launcher file renamed aside on Windows, since it was running.
        public const string AsideSuffix = ".old";

        // The command-line word a restarted launcher carries, so a second
        // fail in a row is an error and not another download.
        public const string PatchedFlag = "--patched";

        // Fetches every path in `paths` (relative, forward slashes) into
        // `install`.  Progress is told (done, total, the file, its bytes so
        // far, its size) on the caller's thread.  Throws at the first file
        // that can't be fetched or won't go in; the screen says so.
        public static async Task<PatchResult> MendAsync(string install, string platform, string www, Manifest stamp,
                                                        List<string> paths,
                                                        Action<int, int, string, long, long> progress,
                                                        CancellationToken cancel)
        {
            var wanted = new Dictionary<string, ManifestFile>(StringComparer.Ordinal);
            foreach (ManifestFile file in stamp.Files)
                wanted[file.Path] = file;

            string launcherFolder = Path.GetFullPath(AppContext.BaseDirectory)
                                        .TrimEnd(Path.DirectorySeparatorChar);
            var result = new PatchResult();
            int done = 0;
            foreach (string relative in paths)
            {
                cancel.ThrowIfCancellationRequested();
                ManifestFile want = wanted[relative];
                string full = Path.Combine(install, relative.Replace('/', Path.DirectorySeparatorChar));
                string temp = full + TempSuffix;
                string url = ManifestSource.FileUrl(www, platform, relative);
                Directory.CreateDirectory(Path.GetDirectoryName(full));

                if (progress != null)
                    progress(done, paths.Count, relative, 0, want.Size);
                string hash;
                try
                {
                    hash = await ManifestSource.FetchFileAsync(url, temp, bytes =>
                    {
                        if (progress != null)
                            progress(done, paths.Count, relative, bytes, want.Size);
                    }, cancel);
                }
                catch (Exception)
                {
                    TryDelete(temp);
                    throw;
                }

                long size = new FileInfo(temp).Length;
                if (size != want.Size || hash != want.Sha256)
                {
                    TryDelete(temp);
                    throw new InvalidDataException("the copy at " + url + " isn't what the manifest says (" + size
                                                   + " bytes, hash " + hash.Substring(0, 12) + "...)");
                }
                if (want.Executable && !RuntimeInfo.IsWindows)
                {
                    File.SetUnixFileMode(temp, UnixFileMode.UserRead | UnixFileMode.UserWrite | UnixFileMode.UserExecute
                                               | UnixFileMode.GroupRead | UnixFileMode.GroupExecute
                                               | UnixFileMode.OtherRead | UnixFileMode.OtherExecute);
                }

                SwapIn(temp, full);
                done++;
                result.Fetched = done;
                result.Bytes += size;
                string folder = Path.GetFullPath(Path.GetDirectoryName(full)).TrimEnd(Path.DirectorySeparatorChar);
                if (string.Equals(folder, launcherFolder, StringComparison.Ordinal))
                    result.LauncherTouched = true;
                Log.Say("Patch: " + relative + " (" + size + " bytes) is in.");
            }
            return result;
        }

        // The new file over the old one.  On Windows the old one may be a
        // program that's running (the launcher's own), which can't be
        // written over but can be renamed: aside it goes, and CleanUp takes
        // it on the next start.
        static void SwapIn(string temp, string full)
        {
            try
            {
                File.Move(temp, full, true);
                return;
            }
            catch (IOException) when (RuntimeInfo.IsWindows && File.Exists(full))
            {
                // Fall through to the rename.
            }
            catch (UnauthorizedAccessException) when (RuntimeInfo.IsWindows && File.Exists(full))
            {
                // The same.
            }
            string aside = full + AsideSuffix;
            File.Move(full, aside, true);
            File.Move(temp, full);
            Log.Say("Patch: " + Path.GetFileName(full) + " was in use, so the old one is " + aside + " until the next "
                    + "start.");
        }

        // The leftovers of an earlier patch, under the install: a temp a
        // download didn't finish, a file renamed aside.  Run at the start,
        // before the check, which would otherwise count them as extras.
        public static void CleanUp(string install)
        {
            int gone = 0;
            foreach (string path in Directory.EnumerateFiles(install, "*", SearchOption.AllDirectories))
            {
                if (!path.EndsWith(TempSuffix, StringComparison.Ordinal)
                    && !path.EndsWith(AsideSuffix, StringComparison.Ordinal))
                    continue;
                if (TryDelete(path))
                    gone++;
            }
            if (gone > 0)
                Log.Say("Patch: " + gone + " leftover file" + (gone == 1 ? "" : "s") + " from an earlier patch cleaned "
                        + "up.");
        }

        // Starts this launcher again with the same command line and
        // --patched on the end.  The caller closes its window after.  False,
        // with why, when it couldn't.
        public static bool Restart(out string why)
        {
            try
            {
                var start = new ProcessStartInfo(GameLauncher.OwnPath());
                start.UseShellExecute = false;
                start.WorkingDirectory = AppContext.BaseDirectory;
                string[] args = Environment.GetCommandLineArgs();
                for (int i = 1; i < args.Length; i++)
                {
                    if (args[i] != PatchedFlag)
                        start.ArgumentList.Add(args[i]);
                }
                start.ArgumentList.Add(PatchedFlag);
                Process.Start(start);
                why = null;
                return true;
            }
            catch (Exception e)
            {
                why = "the launcher couldn't be started again (" + e.Message + ")";
                return false;
            }
        }

        static bool TryDelete(string path)
        {
            try
            {
                File.Delete(path);
                return true;
            }
            catch (Exception e)
            {
                Log.Warn("Patch: " + path + " couldn't be deleted (" + e.Message + ").");
                return false;
            }
        }
    }
}
