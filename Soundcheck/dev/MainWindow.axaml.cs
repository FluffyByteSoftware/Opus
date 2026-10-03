// File:       Opus/Soundcheck/dev/MainWindow.axaml.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Puts the right screen in the window, says so in the corner when it's
// admin or debug mode, tells the login to stop if the window closes in the
// middle of one, and sizes the window once, to what's in it.

using System;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Platform;
using Opus.Soundcheck.Screens;

namespace Opus.Soundcheck
{
    public partial class MainWindow : Window
    {
        // How much bigger than its contents the window is: 5 % of the
        // fitted size, half of it on each edge (Jacob, 2026-10-03: "if the
        // window is 1000 pixels wide by the time its 'scaled' to fit its
        // content we want to add 5% of the 1000 as padding").
        const double Room = 0.05;

        // Avalonia's XAML loader and the designer look for a constructor
        // with nothing in it, and warn when there isn't one.  Plain user
        // mode; the program itself uses the one below.
        public MainWindow() : this(false, false, null, null, false)
        {
        }

        // `game` is --game's path, or null for the game beside the
        // launcher; `www` is --www's URL, or null for the real web folder;
        // `patched` says this launcher was started by one that had just
        // patched itself.
        public MainWindow(bool admin, bool debug, string game, string www, bool patched)
        {
            InitializeComponent();
            VersionLine.Text = "Soundcheck " + ClientVersion.Text;
            if (admin)
            {
                Title = "Forgotten Legends - Soundcheck admin";
                VersionLine.Text += " - ADMIN MODE";
                ScreenHost.Content = new AdminScreen();
            }
            else
            {
                if (debug)
                {
                    Title = "Forgotten Legends - DEBUG MODE";
                    VersionLine.Text += " - DEBUG MODE";
                }
                var login = new LoginScreen(debug, game, www, patched);
                ScreenHost.Content = login;
                Closing += (sender, e) => login.WindowClosing();
            }
            Opened += FitOnce;
        }

        // Avalonia has already fitted the window to the screen showing in it
        // by the time it opens (SizeToContent, in the .axaml), at the screen's
        // own scaling.  This adds the room round the edge and fixes the size
        // there for good: nothing on a screen changes its height afterwards
        // (Screens/Reserved.cs), so nothing needs the window to grow.
        void FitOnce(object sender, EventArgs e)
        {
            Opened -= FitOnce;
            Size fitted = ClientSize;
            double across = fitted.Width * Room / 2;
            double down = fitted.Height * Room / 2;
            Thickness edge = Frame.Margin;
            Frame.Margin = new Thickness(edge.Left + across, edge.Top + down, edge.Right + across, edge.Bottom + down);
            SizeToContent = SizeToContent.Manual;
            Width = fitted.Width * (1 + Room);
            Height = fitted.Height * (1 + Room);
            Centre();
        }

        // In the middle of the screen again, at the new size.  The screen's
        // corners are in its own pixels, the window's size in Avalonia's, so
        // the size is scaled to match.
        void Centre()
        {
            Screen screen = Screens.ScreenFromWindow(this);
            if (screen == null)
                return;
            PixelRect area = screen.WorkingArea;
            double scale = screen.Scaling;
            Position = new PixelPoint(area.X + (int)((area.Width - Width * scale) / 2),
                                      area.Y + (int)((area.Height - Height * scale) / 2));
        }
    }
}
