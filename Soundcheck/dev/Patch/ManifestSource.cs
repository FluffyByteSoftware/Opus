// File:       Opus/Soundcheck/dev/Patch/ManifestSource.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Where user mode gets the manifest and the files: the web folder, not
// Conductor.  It's served at http://opusensemble.duckdns.org:8553/ (Jacob,
// 2026-10-02), with linux_manifest.json and windows_manifest.json at its
// root and download/linux/ and download/windows/ under it, each an exact
// copy of that platform's client folder, so a file's address is the web
// folder, download/, the platform, and the file's own path.  Soundcheck
// fetches the manifest for the OS it's running on, and then only the files
// that are off.  --www and a URL on the command line points at another web
// folder, for a test against a web server on this machine.  Plain HTTP is
// fine here: nothing in the folder is secret, and a tampered file fails
// its hash.

using System;
using System.IO;
using System.Net;
using System.Net.Http;
using System.Security.Cryptography;
using System.Threading;
using System.Threading.Tasks;

namespace Opus.Patch
{
    public static class ManifestSource
    {
        public const string Address = "http://opusensemble.duckdns.org:8553/";

        // Under the web folder: the copies of the clients.
        public const string DownloadFolder = "download";

        // How long the manifest may take: it's tens of kilobytes, so this is
        // generous.  A file's download has no limit but the window closing.
        public const int ManifestTimeoutSeconds = 15;

        // One client for the run.  Its own timeout is off; each fetch brings
        // its own cancel.
        static readonly HttpClient Client = new HttpClient { Timeout = Timeout.InfiniteTimeSpan };

        // The web folder's address with its slash on the end.
        public static string Base(string www)
        {
            string address = string.IsNullOrEmpty(www) ? Address : www;
            return address.EndsWith("/", StringComparison.Ordinal) ? address : address + "/";
        }

        // The manifest's address for a platform.
        public static string ManifestUrl(string www, string platform)
        {
            return Base(www) + Manifest.FileNameFor(platform);
        }

        // A file's address: the web folder, download/, the platform, then
        // the file's path with each piece escaped (a space is %20 to a web
        // server, and a slash stays a slash).
        public static string FileUrl(string www, string platform, string relative)
        {
            string[] pieces = relative.Split('/');
            for (int i = 0; i < pieces.Length; i++)
                pieces[i] = Uri.EscapeDataString(pieces[i]);
            return Base(www) + DownloadFolder + "/" + platform + "/" + string.Join("/", pieces);
        }

        // Fetches the manifest and checks it's the one for this platform.
        // Throws with plain words when it can't; the screen shows them.
        public static async Task<Manifest> FetchManifestAsync(string url, string platform, CancellationToken cancel)
        {
            string text;
            using (var limit = CancellationTokenSource.CreateLinkedTokenSource(cancel))
            {
                limit.CancelAfter(TimeSpan.FromSeconds(ManifestTimeoutSeconds));
                using (HttpResponseMessage answer = await GetAsync(url, limit.Token, cancel))
                    text = await answer.Content.ReadAsStringAsync(limit.Token);
            }

            Manifest manifest = Manifest.Parse(text, url);
            if (manifest.Platform != platform)
                throw new InvalidDataException(url + " is the " + manifest.Platform + " manifest, and this machine is "
                                               + platform);
            return manifest;
        }

        // Fetches one file to `path` (a temp beside the real one), telling
        // the bytes so far as it goes, and hands back its SHA-256 as 64
        // lowercase hex for the caller to check.  Throws when it can't.
        public static async Task<string> FetchFileAsync(string url, string path, Action<long> progress,
                                                        CancellationToken cancel)
        {
            using (HttpResponseMessage answer = await GetAsync(url, cancel, cancel))
            using (Stream body = await answer.Content.ReadAsStreamAsync(cancel))
            using (FileStream file = File.Create(path))
            using (IncrementalHash hash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256))
            {
                byte[] buffer = new byte[64 * 1024];
                long total = 0;
                while (true)
                {
                    int got = await body.ReadAsync(buffer, 0, buffer.Length, cancel);
                    if (got == 0)
                        break;
                    await file.WriteAsync(buffer, 0, got, cancel);
                    hash.AppendData(buffer, 0, got);
                    total += got;
                    if (progress != null)
                        progress(total);
                }
                return Convert.ToHexStringLower(hash.GetHashAndReset());
            }
        }

        // A GET with the headers read and the body still to come.  `limit`
        // is the cancel with the timeout in it; `asked` is the caller's own,
        // so a timeout and a window closing read as two different things.
        static async Task<HttpResponseMessage> GetAsync(string url, CancellationToken limit, CancellationToken asked)
        {
            HttpResponseMessage answer;
            try
            {
                answer = await Client.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, limit);
            }
            catch (HttpRequestException e)
            {
                throw new IOException("couldn't reach " + url + " (" + e.Message + ")");
            }
            catch (OperationCanceledException) when (!asked.IsCancellationRequested)
            {
                throw new IOException(url + " didn't answer in " + ManifestTimeoutSeconds + " s");
            }
            if (answer.StatusCode != HttpStatusCode.OK)
            {
                string words = url + " answered " + (int)answer.StatusCode + " " + answer.ReasonPhrase;
                answer.Dispose();
                throw new IOException(words);
            }
            return answer;
        }
    }
}
