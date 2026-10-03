// File:       Opus/Soundcheck/dev/Screens/Reserved.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Shows or hides a piece of a screen without giving up its room.  The
// window is sized once, to what's in it at start (MainWindow's FitOnce),
// so nothing that comes and goes may change how tall a screen is.

using Avalonia.Controls;

namespace Opus.Soundcheck.Screens
{
    static class Reserved
    {
        // Hidden is see-through, can't be clicked and can't be tabbed to,
        // and still takes its space.  In the .axaml, a piece that starts
        // hidden says Opacity="0" IsHitTestVisible="False" IsEnabled="False".
        public static void Show(Control control, bool shown)
        {
            control.Opacity = shown ? 1 : 0;
            control.IsHitTestVisible = shown;
            control.IsEnabled = shown;
        }
    }
}
