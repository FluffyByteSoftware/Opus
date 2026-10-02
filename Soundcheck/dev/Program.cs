// File:       Opus/Soundcheck/dev/Program.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Where the program starts.  Avalonia wants it this way: build the app,
// hand it the command line, and it runs the window until the window closes.
// The one thing on the command line we look at is --admin (App.cs).

using Avalonia;

namespace Opus.Soundcheck
{
    public static class Program
    {
        public static void Main(string[] args)
        {
            BuildAvaloniaApp().StartWithClassicDesktopLifetime(args);
        }

        // Avalonia's designer looks for this by name, so it stays public
        // and keeps its name.
        public static AppBuilder BuildAvaloniaApp()
        {
            return AppBuilder.Configure<App>()
                .UsePlatformDetect()
                .With(new X11PlatformOptions
                {
                    // Soundcheck has no menu bar, so there's nothing to
                    // register with the desktop's global menu over DBus.
                    UseDBusMenu = false,
                })
                .LogToTrace();
        }
    }
}
