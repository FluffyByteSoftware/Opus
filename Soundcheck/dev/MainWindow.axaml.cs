// File:       Opus/Soundcheck/dev/MainWindow.axaml.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// Puts the right screen in the window, and tells the login to stop if the
// window closes in the middle of one.

using Avalonia.Controls;
using Opus.Soundcheck.Screens;

namespace Opus.Soundcheck
{
    public partial class MainWindow : Window
    {
        public MainWindow(bool admin)
        {
            InitializeComponent();
            VersionLine.Text = "Soundcheck " + ClientVersion.Text;
            if (admin)
            {
                Title = "Forgotten Legends - Soundcheck admin";
                ScreenHost.Content = new AdminScreen();
            }
            else
            {
                var login = new LoginScreen();
                ScreenHost.Content = login;
                Closing += (sender, e) => login.WindowClosing();
            }
        }
    }
}
