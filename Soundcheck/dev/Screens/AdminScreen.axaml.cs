// File:       Opus/Soundcheck/dev/Screens/AdminScreen.axaml.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// What admin mode does: PUBLISH mirrors the build folder for the platform
// picked into download/<platform>/ in the web folder (Patch/Mirror.cs),
// then hashes the mirror and writes that platform's manifest at the web
// folder's root (Patch/Manifest.cs), on a worker thread with the progress
// bar following it.  The web server serves the folder as it is, and a
// player's launcher fetches the manifest and then any file by its path.
// A folder per platform, the version, the platform picked and the web
// folder are remembered between runs (Patch/AdminSettings.cs).

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Avalonia.Threading;
using Opus.Patch;

namespace Opus.Soundcheck.Screens
{
    public partial class AdminScreen : UserControl
    {
        readonly AdminSettings settings;

        // Which platform the screen is on.
        string platform;

        // Set while a publish runs, so a second click waits.
        bool publishing;

        // Counts the phases (copying, hashing, done).  A progress message
        // carries the phase it was sent in, and one that reaches the
        // window's thread after its phase ended is dropped: the worker's
        // last "193 of 193" and the "Published" line both come through the
        // dispatcher, and the progress one can land second.
        int phase;

        public AdminScreen()
        {
            InitializeComponent();
            settings = AdminSettings.Load();
            VersionBox.Text = string.IsNullOrEmpty(settings.ClientVersion)
                ? ClientVersion.Text
                : settings.ClientVersion;
            WebFolderBox.Text = string.IsNullOrEmpty(settings.WebFolder)
                ? AdminSettings.DefaultWebFolder
                : settings.WebFolder;
            WebFolderBox.TextChanged += (sender, e) => SayWhere();

            // The platform picked last time, else the one this machine is;
            // ticking the button fills the folder box (PlatformChanged).
            string picked = Platforms.IsKnown(settings.Platform) ? settings.Platform : Platforms.Here;
            if (picked == Platforms.Windows)
                WindowsButton.IsChecked = true;
            else
                LinuxButton.IsChecked = true;
        }

        // ---------------------------------------------------------------
        // Linux or Windows
        // ---------------------------------------------------------------

        // One of the two buttons changed.  Avalonia fires this for the one
        // going off as well as the one going on, and while the screen is
        // still being built (before `settings` is set), so both are
        // checked for.
        void PlatformChanged(object sender, RoutedEventArgs e)
        {
            RadioButton button = sender as RadioButton;
            if (settings == null || button == null || button.IsChecked != true)
                return;
            string picked = button == WindowsButton ? Platforms.Windows : Platforms.Linux;
            if (picked == platform)
                return;
            if (platform != null)
                settings.SetFolder(platform, (FolderBox.Text ?? "").Trim());
            platform = picked;
            FolderBox.Text = settings.FolderFor(platform) ?? "";
            SayWhere();
        }

        // The line under the web folder: where the copy and the manifest go.
        void SayWhere()
        {
            if (platform == null)
                return;
            string www = (WebFolderBox.Text ?? "").Trim();
            if (www == "")
            {
                WhereLine.Text = "";
                return;
            }
            WhereLine.Text = "The copy goes to " + Mirror.FolderFor(www, platform) + ", the manifest to "
                             + Path.Combine(www, Manifest.FileNameFor(platform)) + ".  Players fetch both from "
                             + ManifestSource.Base(null) + ".";
        }

        // ---------------------------------------------------------------
        // The two BROWSE buttons
        // ---------------------------------------------------------------

        async void BrowseFolderClicked(object sender, RoutedEventArgs e)
        {
            string path = await PickFolder("The " + platform + " build folder");
            if (path != null)
                FolderBox.Text = path;
        }

        async void BrowseWebFolderClicked(object sender, RoutedEventArgs e)
        {
            string path = await PickFolder("The web folder");
            if (path != null)
                WebFolderBox.Text = path;
        }

        async Task<string> PickFolder(string title)
        {
            IStorageProvider storage = TopLevel.GetTopLevel(this)?.StorageProvider;
            if (storage == null)
                return null;
            IReadOnlyList<IStorageFolder> picked = await storage.OpenFolderPickerAsync(new FolderPickerOpenOptions
            {
                Title = title,
                AllowMultiple = false,
            });
            if (picked.Count == 0)
                return null;
            return picked[0].TryGetLocalPath();
        }

        // ---------------------------------------------------------------
        // PUBLISH
        // ---------------------------------------------------------------

        async void PublishClicked(object sender, RoutedEventArgs e)
        {
            if (publishing)
                return;

            string folder = (FolderBox.Text ?? "").Trim();
            string version = (VersionBox.Text ?? "").Trim();
            string www = (WebFolderBox.Text ?? "").Trim();
            if (folder == "" || !Directory.Exists(folder))
            {
                ShowStatus("Pick the " + platform + " build folder first; there's no folder at \"" + folder + "\".",
                           true);
                return;
            }
            if (version == "")
            {
                ShowStatus("Type the client's version.", true);
                return;
            }
            if (www == "")
            {
                ShowStatus("Say which folder the web server serves.", true);
                return;
            }
            string fullFolder = WithSlash(folder);
            string fullWww = WithSlash(www);
            if (fullWww.StartsWith(fullFolder, StringComparison.Ordinal)
                || fullFolder.StartsWith(fullWww, StringComparison.Ordinal))
            {
                ShowStatus("The build folder and the web folder can't be inside each other: the copy would be copying "
                           + "itself.", true);
                return;
            }
            string mirror = Mirror.FolderFor(www, platform);
            string manifestPath = Path.Combine(www, Manifest.FileNameFor(platform));

            settings.SetFolder(platform, folder);
            settings.ClientVersion = version;
            settings.Platform = platform;
            settings.WebFolder = www;
            settings.Save();

            publishing = true;
            SetEnabled(false);
            Progress.Value = 0;
            Reserved.Show(Progress, true);
            ShowStatus("Copying " + folder + " to " + mirror + "...", false);
            Log.Say("Admin: publishing the " + platform + " build at " + folder + " (version " + version + ") into "
                    + www + ".");

            var clock = Stopwatch.StartNew();
            try
            {
                int copying = ++phase;
                MirrorResult copied = await Task.Run(
                    () => Mirror.Run(folder, mirror,
                                     (done, total, path) => Report(copying, "Copying", done, total, path),
                                     CancellationToken.None));
                Log.Say("Admin: the copy is at " + mirror + ": " + copied.Copied + " copied, " + copied.Kept
                        + " already there, " + copied.Removed + " removed.");
                int hashing = ++phase;
                Manifest manifest = await Task.Run(
                    () => Manifest.Of(mirror, platform, version,
                                      (done, total, path) => Report(hashing, "Hashing", done, total, path),
                                      CancellationToken.None));
                phase++;
                manifest.Save(manifestPath);
                int count = manifest.Files.Count;
                string words = "Published " + platform + " in " + clock.Elapsed.TotalSeconds.ToString("0.0") + " s: "
                               + copied.Copied + " copied, " + copied.Kept + " already there, " + copied.Removed
                               + " removed; the manifest lists " + count + (count == 1 ? " file, " : " files, ")
                               + Megabytes(manifest.TotalBytes) + " MB.  The copy is at " + mirror
                               + ", the manifest at " + manifestPath + ".";
                Log.Say("Admin: " + words);
                ShowStatus(words, false);
                Progress.Value = 1;
            }
            catch (Exception ex)
            {
                Log.Error("Admin: the publish failed (" + ex.Message + ").");
                ShowStatus("Couldn't publish: " + ex.Message, true);
                Reserved.Show(Progress, false);
            }
            finally
            {
                phase++;
                publishing = false;
                SetEnabled(true);
            }
        }

        // From the worker thread, after each file.  Dropped once its phase
        // is over.
        void Report(int sentIn, string doing, int done, int total, string path)
        {
            Dispatcher.UIThread.Post(() =>
            {
                if (sentIn != phase)
                    return;
                Progress.Value = total == 0 ? 1 : (double)done / total;
                ShowStatus(doing + " " + done + " of " + total + ": " + path, false);
            });
        }

        // A folder's full path with the separator on the end, so "inside"
        // is a plain StartsWith.
        static string WithSlash(string folder)
        {
            return Path.GetFullPath(folder).TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;
        }

        static string Megabytes(long bytes)
        {
            return (bytes / (1024.0 * 1024.0)).ToString("0.0");
        }

        void SetEnabled(bool enabled)
        {
            LinuxButton.IsEnabled = enabled;
            WindowsButton.IsEnabled = enabled;
            FolderBox.IsEnabled = enabled;
            VersionBox.IsEnabled = enabled;
            WebFolderBox.IsEnabled = enabled;
            BrowseFolderButton.IsEnabled = enabled;
            BrowseWebFolderButton.IsEnabled = enabled;
            PublishButton.IsEnabled = enabled;
        }

        void ShowStatus(string words, bool trouble)
        {
            StatusLine.Text = words ?? "";
            Reserved.Show(StatusBox, StatusLine.Text != "");
            if (trouble)
                StatusBox.Classes.Add("trouble");
            else
                StatusBox.Classes.Remove("trouble");
        }
    }
}
