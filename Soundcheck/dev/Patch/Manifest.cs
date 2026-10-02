// File:       Opus/Soundcheck/dev/Patch/Manifest.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// The manifest: every file of the installed client, with its size and its
// SHA-256, and the client's version.  Admin mode writes one from the
// correct client folder (patch_manifest.json, which goes to
// Content/patch/); user mode will make one of its own install and compare.
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
    }

    public class Manifest
    {
        public const string FileName = "patch_manifest.json";

        // The shape of the JSON.  Bumps when the shape changes, with
        // PATCH_MANIFEST.md.
        public const int FormatVersion = 1;

        [JsonPropertyName("format")]
        public int Format { get; set; } = FormatVersion;

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

        // Walks a folder and hashes every file in it.  Slow for a whole
        // client (a Unity build is hundreds of megabytes), so call it off
        // the window's thread; progress is told (done, total, the file)
        // after each file, on that same thread.  A patch_manifest.json
        // sitting at the folder's root is skipped, since a manifest can't
        // list itself.
        public static Manifest Of(string folder, string clientVersion, Action<int, int, string> progress,
                                  CancellationToken cancel)
        {
            folder = System.IO.Path.GetFullPath(folder);
            var paths = new List<string>(Directory.EnumerateFiles(folder, "*", SearchOption.AllDirectories));
            paths.Sort(StringComparer.Ordinal);

            var manifest = new Manifest
            {
                ClientVersion = clientVersion,
                Written = DateTime.UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss'Z'"),
            };

            // The relative paths are what's sorted, not the full ones, so
            // the order doesn't depend on where the folder sits.
            var entries = new List<KeyValuePair<string, string>>(paths.Count);
            foreach (string full in paths)
            {
                string relative = System.IO.Path.GetRelativePath(folder, full).Replace('\\', '/');
                if (relative == FileName)
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
                manifest.Files.Add(new ManifestFile { Path = entry.Key, Size = size, Sha256 = hash });
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

        // Reads one back.  Throws when the file isn't there or isn't a
        // manifest; the caller says so.
        public static Manifest Load(string path)
        {
            Manifest manifest = JsonSerializer.Deserialize<Manifest>(File.ReadAllText(path));
            if (manifest == null || manifest.Files == null)
                throw new InvalidDataException(path + " isn't a manifest");
            if (manifest.Format != FormatVersion)
                throw new InvalidDataException(path + " is manifest format " + manifest.Format + "; this "
                                               + "launcher reads format " + FormatVersion);
            return manifest;
        }
    }
}
