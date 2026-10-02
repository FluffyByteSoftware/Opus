// File:       Opus/Soundcheck/dev/MainWindow.axaml.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Puts the right screen in the window, says so in the corner when it's
// admin or debug mode, and tells the login to stop if the window closes in
// the middle of one.

using Avalonia.Controls;
using Opus.Soundcheck.Screens;

namespace Opus.Soundcheck
{
    public partial class MainWindow : Window
    {
        // Avalonia's XAML loader and the designer look for a constructor
        // with nothing in it, and warn when there isn't one.  Plain user
        // mode; the program itself uses the one below.
        public MainWindow() : this(false, false, null, null)
        {
        }

        // `game` is --game's path, or null for the game beside the
        // launcher; `manifest` is --manifest's URL, or null for the web
        // address.
        public MainWindow(bool admin, bool debug, string game, string manifest)
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
                var login = new LoginScreen(debug, game, manifest);
                ScreenHost.Content = login;
                Closing += (sender, e) => login.WindowClosing();
            }
        }
    }
}
