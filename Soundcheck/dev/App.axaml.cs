// File:       Opus/Soundcheck/dev/App.axaml.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Opens the window once Avalonia is up.  --admin on the command line opens
// it in admin mode (the manifest writer, for the server's machine); --debug
// is debug mode (a login that skips the file check and leaves the ticket in
// a file for an Ensemble running in Unity's editor); --game and a path says
// where the game is when it isn't beside the launcher (GameLauncher.cs);
// anything else is a player logging in.

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
                string[] args = desktop.Args ?? new string[0];
                bool admin = Array.IndexOf(args, "--admin") >= 0;
                bool debug = Array.IndexOf(args, "--debug") >= 0;
                string game = null;
                int at = Array.IndexOf(args, "--game");
                if (at >= 0 && at + 1 < args.Length)
                    game = args[at + 1];
                else if (at >= 0)
                    Log.Warn("--game needs the game's path after it.  Ignored.");
                Log.Say("Soundcheck " + ClientVersion.Text + (admin ? ", admin mode." : debug ? ", debug mode." : ".")
                        + (game != null ? "  The game is " + game + "." : ""));
                desktop.MainWindow = new MainWindow(admin, debug, game);

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
