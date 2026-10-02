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
        public MainWindow(bool admin, bool debug)
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
                var login = new LoginScreen(debug);
                ScreenHost.Content = login;
                Closing += (sender, e) => login.WindowClosing();
            }
        }
    }
}
