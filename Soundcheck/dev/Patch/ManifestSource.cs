// File:       Opus/Soundcheck/dev/Patch/ManifestSource.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Where user mode gets the manifest: a web address, not Conductor.  The
// two manifests sit at http://opusensemble.com:8553/manifest_lin.json and
// manifest_win.json (Jacob, 2026-10-02), and Soundcheck fetches the one
// for the OS it's running on.  --manifest and a URL on the command line
// points at another copy, for a test against a web server on this
// machine.  Plain HTTP is fine here: the manifest holds nothing secret,
// and a tampered one can only make the check fail, never pass something
// the server would turn away.

using System;
using System.IO;
using System.Net;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;

namespace Opus.Patch
{
    public static class ManifestSource
    {
        public const string Address = "http://opusensemble.com:8553/";

        // How long a fetch may take before it's given up on.  The file is
        // tens of kilobytes, so this is generous.
        public const int TimeoutSeconds = 15;

        // The manifest's address for a platform.
        public static string UrlFor(string platform)
        {
            return Address + Manifest.FileNameFor(platform);
        }

        // Fetches the manifest at the address and checks it's the one for
        // this platform.  Throws with plain words when it can't; the
        // screen shows them.
        public static async Task<Manifest> FetchAsync(string url, string platform, CancellationToken cancel)
        {
            string text;
            using (var client = new HttpClient { Timeout = TimeSpan.FromSeconds(TimeoutSeconds) })
            {
                HttpResponseMessage answer;
                try
                {
                    answer = await client.GetAsync(url, cancel);
                }
                catch (HttpRequestException e)
                {
                    throw new IOException("couldn't reach " + url + " (" + e.Message + ")");
                }
                catch (TaskCanceledException) when (!cancel.IsCancellationRequested)
                {
                    // HttpClient says a timeout as a cancel; ours is the
                    // other kind, and goes up as it is.
                    throw new IOException(url + " didn't answer in " + TimeoutSeconds + " s");
                }
                using (answer)
                {
                    if (answer.StatusCode != HttpStatusCode.OK)
                        throw new IOException(url + " answered " + (int)answer.StatusCode + " " + answer.ReasonPhrase);
                    text = await answer.Content.ReadAsStringAsync(cancel);
                }
            }

            Manifest manifest = Manifest.Parse(text, url);
            if (manifest.Platform != platform)
                throw new InvalidDataException(url + " is the " + manifest.Platform + " manifest, and this machine is "
                                               + platform);
            return manifest;
        }
    }
}
