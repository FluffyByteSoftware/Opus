// File:       Opus/Soundcheck/dev/App.axaml.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Opens the window once Avalonia is up.  --admin on the command line opens
// it in admin mode (the manifest writer, for the server's machine); --debug
// is debug mode (a login that skips the file check and leaves the ticket in
// a file for an Ensemble running in Unity's editor); anything else is a
// player logging in.

using System;
using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;

namespace Opus.Soundcheck
{
    public partial class App : Application
    {
        public override void Initialize()
        {
            AvaloniaXamlLoader.Load(this);
        }

        public override void OnFrameworkInitializationCompleted()
        {
            if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
            {
                bool admin = desktop.Args != null && Array.IndexOf(desktop.Args, "--admin") >= 0;
                bool debug = desktop.Args != null && Array.IndexOf(desktop.Args, "--debug") >= 0;
                Log.Say("Soundcheck " + ClientVersion.Text + (admin ? ", admin mode." : debug ? ", debug mode." : "."));
                desktop.MainWindow = new MainWindow(admin, debug);

                // The window has closed and Avalonia is about to shut its
                // own thread down.  We end the program here instead of
                // letting it: on KDE's Wayland session, Avalonia's tidy-up
                // after that point hands a late DBus message to the thread
                // it just stopped and dies with an unhandled
                // TaskCanceledException (Avalonia issue 19523, open at
                // 2026-10-02; an X11 session doesn't show it).  Nothing of
                // ours is left to do by then: Remember Me is written when
                // the Ticket comes, and the log goes straight to the
                // terminal.
                desktop.Exit += (sender, e) =>
                {
                    Log.Say("Soundcheck closing.");
                    Console.Out.Flush();
                    Environment.Exit(e.ApplicationExitCode);
                };
            }
            base.OnFrameworkInitializationCompleted();
        }
    }
}
