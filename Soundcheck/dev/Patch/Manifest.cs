// File:       Opus/Soundcheck/dev/Patch/Manifest.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// The manifest: every file of the installed client, with its size and its
// SHA-256, the client's version, and which players' machines it's for.
// There's one a platform, linux_manifest.json and windows_manifest.json,
// since a Linux build and a Windows build are different files.  Admin mode
// writes one from the mirror of the client in the web folder (Mirror.cs);
// user mode fetches the one for the OS it's running on from the web folder
// (ManifestSource.cs) and makes one of its own install to hold against it
// (ManifestCheck.cs).
// Documentation/LLM/PATCH_MANIFEST.md is the contract: the JSON's shape,
// byte for byte, and this file is written from it.

using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;

namespace Opus.Patch
{
    // The two kinds of machine a client is built for, as the manifest
    // names them.
    public static class Platforms
    {
        public const string Linux = "linux";
        public const string Windows = "windows";

        // The one this program is running on.
        public static string Here
        {
            get { return RuntimeInfo.IsWindows ? Windows : Linux; }
        }

        public static bool IsKnown(string platform)
        {
            return platform == Linux || platform == Windows;
        }
    }

    public class ManifestFile
    {
        // Relative to the install folder, with forward slashes whatever
        // the OS, so the same file has the same name on Linux and Windows.
        [JsonPropertyName("path")]
        public string Path { get; set; }

        [JsonPropertyName("size")]
        public long Size { get; set; }

        // 64 lowercase hex characters.
        [JsonPropertyName("sha256")]
        public string Sha256 { get; set; }

        // True when the file is a program on Linux (its owner's execute
        // bit is set), so a fetched copy is made runnable again; a download
        // comes with no permissions.  Left out of the JSON when false, and
        // never set by a manifest written on Windows.
        [JsonPropertyName("executable")]
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)]
        public bool Executable { get; set; }
    }

    public class Manifest
    {
        // The shape of the JSON.  Bumps when the shape changes, with
        // PATCH_MANIFEST.md.
        public const int FormatVersion = 3;

        // The file's name, by the platform it's for: at the web folder's
        // root, beside download/.
        public static string FileNameFor(string platform)
        {
            return platform + "_manifest.json";
        }

        // Every name a manifest has had, so one sitting at the folder's
        // root is skipped whichever it is (a manifest can't list itself).
        static readonly string[] FileNames =
        {
            "linux_manifest.json", "windows_manifest.json", "manifest_lin.json", "manifest_win.json",
            "patch_manifest.json",
        };

        // Unity leaves this folder beside a build: the IL2CPP symbols,
        // hundreds of megabytes a player never needs.  It doesn't ship, so
        // it isn't in the manifest and isn't checked.
        public const string UnityBackupFolder = "Ensemble_BackUpThisFolder_ButDontShipItWithYourGame";

        [JsonPropertyName("format")]
        public int Format { get; set; } = FormatVersion;

        // "linux" or "windows" (Platforms).
        [JsonPropertyName("platform")]
        public string Platform { get; set; }

        // What the Login carries; networking.cfg's client_versions lists it.
        [JsonPropertyName("client_version")]
        public string ClientVersion { get; set; }

        // When it was written, UTC, ending in Z like every time we show.
        [JsonPropertyName("written")]
        public string Written { get; set; }

        // Sorted by path, byte by byte, so two manifests of the same folder
        // are the same text.
        [JsonPropertyName("files")]
        public List<ManifestFile> Files { get; set; } = new List<ManifestFile>();

        static readonly JsonSerializerOptions Pretty = new JsonSerializerOptions { WriteIndented = true };

        // Every file's size added up.
        public long TotalBytes
        {
            get
            {
                long total = 0;
                foreach (ManifestFile file in Files)
                    total += file.Size;
                return total;
            }
        }

        // What the walk leaves out: a manifest at the folder's root, Unity's
        // backup folder, and the patcher's own leftovers (a download on its
        // way in, a launcher file renamed aside on Windows).  `relative` has
        // forward slashes.
        public static bool Skipped(string relative)
        {
            if (Array.IndexOf(FileNames, relative) >= 0)
                return true;
            if (relative.EndsWith(Patcher.TempSuffix, StringComparison.Ordinal)
                || relative.EndsWith(Patcher.AsideSuffix, StringComparison.Ordinal))
                return true;
            return relative.StartsWith(UnityBackupFolder + "/", StringComparison.Ordinal);
        }

        // Whether a file is a program here: Linux's owner-execute bit.
        // Windows has no such bit, so there it's never.
        public static bool IsExecutable(string path)
        {
            if (RuntimeInfo.IsWindows)
                return false;
            return (File.GetUnixFileMode(path) & UnixFileMode.UserExecute) != 0;
        }

        // Walks a folder and hashes every file in it.  Slow for a whole
        // client (a Unity build is hundreds of megabytes), so call it off
        // the window's thread; progress is told (done, total, the file)
        // after each file, on that same thread.
        public static Manifest Of(string folder, string platform, string clientVersion,
                                  Action<int, int, string> progress, CancellationToken cancel)
        {
            folder = System.IO.Path.GetFullPath(folder);
            var paths = new List<string>(Directory.EnumerateFiles(folder, "*", SearchOption.AllDirectories));

            var manifest = new Manifest
            {
                Platform = platform,
                ClientVersion = clientVersion,
                Written = DateTime.UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss'Z'"),
            };

            // The relative paths are what's sorted, not the full ones, so
            // the order doesn't depend on where the folder sits.
            var entries = new List<KeyValuePair<string, string>>(paths.Count);
            foreach (string full in paths)
            {
                string relative = System.IO.Path.GetRelativePath(folder, full).Replace('\\', '/');
                if (Skipped(relative))
                    continue;
                entries.Add(new KeyValuePair<string, string>(relative, full));
            }
            entries.Sort((a, b) => string.CompareOrdinal(a.Key, b.Key));

            int done = 0;
            foreach (KeyValuePair<string, string> entry in entries)
            {
                cancel.ThrowIfCancellationRequested();
                string hash;
                long size;
                using (FileStream stream = File.OpenRead(entry.Value))
                {
                    size = stream.Length;
                    hash = Convert.ToHexStringLower(SHA256.HashData(stream));
                }
                manifest.Files.Add(new ManifestFile
                {
                    Path = entry.Key,
                    Size = size,
                    Sha256 = hash,
                    Executable = IsExecutable(entry.Value),
                });
                done++;
                if (progress != null)
                    progress(done, entries.Count, entry.Key);
            }
            return manifest;
        }

        // Writes the file whole: to a temp file first, then over the old
        // one, so a crash halfway leaves the old file or the new, never
        // half.  Throws if it can't; the screen says so.
        public void Save(string path)
        {
            string temp = path + ".tmp";
            string folder = System.IO.Path.GetDirectoryName(path);
            if (!string.IsNullOrEmpty(folder))
                Directory.CreateDirectory(folder);
            File.WriteAllText(temp, JsonSerializer.Serialize(this, Pretty));
            File.Move(temp, path, true);
        }

        // Reads one off the disk.  Throws when the file isn't there or
        // isn't a manifest; the caller says so.
        public static Manifest Load(string path)
        {
            return Parse(File.ReadAllText(path), path);
        }

        // A manifest out of its JSON text.  `from` is where the text came
        // from (a path, a URL), for the words when it isn't one.  Throws
        // when it isn't a manifest this launcher reads.
        public static Manifest Parse(string json, string from)
        {
            Manifest manifest;
            try
            {
                manifest = JsonSerializer.Deserialize<Manifest>(json);
            }
            catch (JsonException e)
            {
                throw new InvalidDataException(from + " isn't a manifest (" + e.Message + ")");
            }
            if (manifest == null || manifest.Files == null)
                throw new InvalidDataException(from + " isn't a manifest");
            if (manifest.Format != FormatVersion)
                throw new InvalidDataException(from + " is manifest format " + manifest.Format + "; this launcher "
                                               + "reads format " + FormatVersion);
            if (!Platforms.IsKnown(manifest.Platform))
                throw new InvalidDataException(from + " is for \"" + manifest.Platform + "\", which isn't a platform "
                                               + "this launcher knows (linux or windows)");
            return manifest;
        }
    }
}
