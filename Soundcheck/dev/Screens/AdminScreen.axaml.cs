// File:       Opus/Soundcheck/dev/Screens/AdminScreen.axaml.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// What admin mode does: reads every file of the correct client folder,
// hashes it, and writes patch_manifest.json (Patch/Manifest.cs), on a
// worker thread with the progress bar following it.  The folder, the
// version and the output path are remembered between runs
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

        // Set while a manifest is being written, so a second click waits.
        bool writing;

        public AdminScreen()
        {
            InitializeComponent();
            settings = AdminSettings.Load();
            FolderBox.Text = settings.ClientFolder ?? "";
            VersionBox.Text = string.IsNullOrEmpty(settings.ClientVersion)
                ? ClientVersion.Text
                : settings.ClientVersion;
            WriteToBox.Text = settings.WriteTo ?? "";
        }

        // ---------------------------------------------------------------
        // The two BROWSE buttons
        // ---------------------------------------------------------------

        async void BrowseFolderClicked(object sender, RoutedEventArgs e)
        {
            IStorageProvider storage = TopLevel.GetTopLevel(this)?.StorageProvider;
            if (storage == null)
                return;
            IReadOnlyList<IStorageFolder> picked = await storage.OpenFolderPickerAsync(new FolderPickerOpenOptions
            {
                Title = "The client folder",
                AllowMultiple = false,
            });
            if (picked.Count == 0)
                return;
            string path = picked[0].TryGetLocalPath();
            if (path == null)
                return;
            FolderBox.Text = path;
            // A first manifest goes beside the folder, not in it.
            if (string.IsNullOrEmpty(WriteToBox.Text))
            {
                string parent = Path.GetDirectoryName(Path.GetFullPath(path)) ?? path;
                WriteToBox.Text = Path.Combine(parent, Manifest.FileName);
            }
        }

        async void BrowseWriteToClicked(object sender, RoutedEventArgs e)
        {
            IStorageProvider storage = TopLevel.GetTopLevel(this)?.StorageProvider;
            if (storage == null)
                return;
            IStorageFile picked = await storage.SaveFilePickerAsync(new FilePickerSaveOptions
            {
                Title = "Write the manifest to",
                SuggestedFileName = Manifest.FileName,
                DefaultExtension = "json",
                ShowOverwritePrompt = true,
            });
            if (picked == null)
                return;
            string path = picked.TryGetLocalPath();
            if (path != null)
                WriteToBox.Text = path;
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
            if (folder == "" || !Directory.Exists(folder))
            {
                ShowStatus("Pick the client folder first; there's no folder at \"" + folder + "\".", true);
                return;
            }
            if (version == "")
            {
                ShowStatus("Type the client's version.", true);
                return;
            }
            if (writeTo == "")
            {
                ShowStatus("Say where to write the manifest.", true);
                return;
            }
            string fullFolder = Path.GetFullPath(folder).TrimEnd(Path.DirectorySeparatorChar)
                                + Path.DirectorySeparatorChar;
            string fullWriteTo = Path.GetFullPath(writeTo);
            if (fullWriteTo.StartsWith(fullFolder, StringComparison.Ordinal))
            {
                ShowStatus("Write the manifest outside the client folder, or it would have to list itself.", true);
                return;
            }

            settings.ClientFolder = folder;
            settings.ClientVersion = version;
            settings.WriteTo = writeTo;
            settings.Save();

            writing = true;
            SetEnabled(false);
            Progress.Value = 0;
            Progress.IsVisible = true;
            ShowStatus("Reading " + folder + "...", false);
            Log.Say("Admin: writing a manifest of " + folder + " (version " + version + ") to " + writeTo + ".");

            var clock = Stopwatch.StartNew();
            try
            {
                Manifest manifest = await Task.Run(() => Manifest.Of(folder, version, Report, CancellationToken.None));
                manifest.Save(writeTo);
                int count = manifest.Files.Count;
                string words = "Wrote " + count + (count == 1 ? " file" : " files") + ", "
                               + Megabytes(manifest.TotalBytes) + " MB, in "
                               + clock.Elapsed.TotalSeconds.ToString("0.0") + " s, to " + writeTo + ".";
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
