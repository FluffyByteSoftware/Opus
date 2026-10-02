// File:       Opus/Soundcheck/dev/Screens/AdminScreen.axaml.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// What admin mode does: reads every file of the correct client folder for
// the platform picked, hashes it, and writes that platform's manifest
// (manifest_lin.json or manifest_win.json, Patch/Manifest.cs) in the
// folder given, on a worker thread with the progress bar following it.
// The two files then go up to the web address user mode fetches them
// from (Patch/ManifestSource.cs).  A folder per platform, the version,
// the platform picked and the output folder are remembered between runs
// (Patch/AdminSettings.cs).  The manifest can't be written inside the
// folder it describes, since it would have to list itself.

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

        // Which platform's manifest the screen is on.
        string platform;

        // Set while a manifest is being written, so a second click waits.
        bool writing;

        public AdminScreen()
        {
            InitializeComponent();
            settings = AdminSettings.Load();
            VersionBox.Text = string.IsNullOrEmpty(settings.ClientVersion)
                ? ClientVersion.Text
                : settings.ClientVersion;
            WriteToBox.Text = settings.WriteTo ?? "";

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
            FileNameLine.Text = "The file is " + Manifest.FileNameFor(platform) + ", for "
                                + ManifestSource.UrlFor(platform) + ".";
        }

        // ---------------------------------------------------------------
        // The two BROWSE buttons
        // ---------------------------------------------------------------

        async void BrowseFolderClicked(object sender, RoutedEventArgs e)
        {
            string path = await PickFolder("The " + platform + " client folder");
            if (path == null)
                return;
            FolderBox.Text = path;
            // A first manifest goes beside the folder, not in it.
            if (string.IsNullOrEmpty(WriteToBox.Text))
                WriteToBox.Text = Path.GetDirectoryName(Path.GetFullPath(path)) ?? path;
        }

        async void BrowseWriteToClicked(object sender, RoutedEventArgs e)
        {
            string path = await PickFolder("Write the manifest in");
            if (path != null)
                WriteToBox.Text = path;
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
        // WRITE MANIFEST
        // ---------------------------------------------------------------

        async void WriteClicked(object sender, RoutedEventArgs e)
        {
            if (writing)
                return;

            string folder = (FolderBox.Text ?? "").Trim();
            string version = (VersionBox.Text ?? "").Trim();
            string writeTo = (WriteToBox.Text ?? "").Trim();
            string fileName = Manifest.FileNameFor(platform);
            if (folder == "" || !Directory.Exists(folder))
            {
                ShowStatus("Pick the " + platform + " client folder first; there's no folder at \"" + folder + "\".",
                           true);
                return;
            }
            if (version == "")
            {
                ShowStatus("Type the client's version.", true);
                return;
            }
            if (writeTo == "")
            {
                ShowStatus("Say which folder to write " + fileName + " in.", true);
                return;
            }
            string fullFolder = Path.GetFullPath(folder).TrimEnd(Path.DirectorySeparatorChar)
                                + Path.DirectorySeparatorChar;
            string fullWriteTo = Path.GetFullPath(writeTo).TrimEnd(Path.DirectorySeparatorChar)
                                 + Path.DirectorySeparatorChar;
            if (fullWriteTo.StartsWith(fullFolder, StringComparison.Ordinal))
            {
                ShowStatus("Write the manifest outside the client folder, or it would have to list itself.", true);
                return;
            }
            string path = Path.Combine(fullWriteTo, fileName);

            settings.SetFolder(platform, folder);
            settings.ClientVersion = version;
            settings.Platform = platform;
            settings.WriteTo = writeTo;
            settings.Save();

            writing = true;
            SetEnabled(false);
            Progress.Value = 0;
            Progress.IsVisible = true;
            ShowStatus("Reading " + folder + "...", false);
            Log.Say("Admin: writing the " + platform + " manifest of " + folder + " (version " + version + ") to "
                    + path + ".");

            var clock = Stopwatch.StartNew();
            try
            {
                Manifest manifest = await Task.Run(
                    () => Manifest.Of(folder, platform, version, Report, CancellationToken.None));
                manifest.Save(path);
                int count = manifest.Files.Count;
                string words = "Wrote " + fileName + ": " + count + (count == 1 ? " file" : " files") + ", "
                               + Megabytes(manifest.TotalBytes) + " MB, in "
                               + clock.Elapsed.TotalSeconds.ToString("0.0") + " s, to " + path + ".  Put it at "
                               + ManifestSource.UrlFor(platform) + ".";
                Log.Say("Admin: " + words);
                ShowStatus(words, false);
                Progress.Value = 1;
            }
            catch (Exception ex)
            {
                Log.Error("Admin: the manifest couldn't be written (" + ex.Message + ").");
                ShowStatus("Couldn't write the manifest: " + ex.Message, true);
                Progress.IsVisible = false;
            }
            finally
            {
                writing = false;
                SetEnabled(true);
            }
        }

        // From the worker thread, after each file.
        void Report(int done, int total, string path)
        {
            Dispatcher.UIThread.Post(() =>
            {
                Progress.Value = total == 0 ? 1 : (double)done / total;
                ShowStatus(done + " of " + total + ": " + path, false);
            });
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
            WriteToBox.IsEnabled = enabled;
            BrowseFolderButton.IsEnabled = enabled;
            BrowseWriteToButton.IsEnabled = enabled;
            WriteButton.IsEnabled = enabled;
        }

        void ShowStatus(string words, bool trouble)
        {
            StatusLine.Text = words ?? "";
            StatusBox.IsVisible = StatusLine.Text != "";
            if (trouble)
                StatusBox.Classes.Add("trouble");
            else
                StatusBox.Classes.Remove("trouble");
        }
    }
}
