// File:       Opus/Soundcheck/dev/App.axaml.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Opens the window once Avalonia is up.  --admin on the command line opens
// it in admin mode (the manifest writer, for the server's machine); anything
// else is a player logging in.

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
                Log.Say("Soundcheck " + ClientVersion.Text + (admin ? ", admin mode." : "."));
                desktop.MainWindow = new MainWindow(admin);
            }
            base.OnFrameworkInitializationCompleted();
        }
    }
}
