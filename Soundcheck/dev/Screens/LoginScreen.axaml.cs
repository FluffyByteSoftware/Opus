// File:       Opus/Soundcheck/dev/Screens/LoginScreen.axaml.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// What the login screen does: Ensemble's LoginForm.cs, moved here.  A
// remembered login fills the boxes; SUBMIT turns the password into its key
// at once and starts the login (LoginConnection.cs) while the key is still
// being made; the status box says how it's going; once a Ticket comes,
// Remember Me keeps the key.  The login's thread talks to this screen
// through ILoginListener, and every call is put back on the window's
// thread first.  The key is never logged.
//
// Debug mode (--debug) is for Jacob testing a fix in Unity's editor without
// a patch round: the file check (when it exists) is skipped, and the ticket
// goes to debug_ticket.json in the player folder (Net/DebugTicket.cs) for
// the editor's Ensemble, instead of into a started Ensemble's environment.

using System;
using System.Diagnostics;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Threading;
using Opus.Net;
using Opus.Security;

namespace Opus.Soundcheck.Screens
{
    public partial class LoginScreen : UserControl, ILoginListener
    {
        // What the Password box shows, as a hint, while there's a key to use
        // instead of a password: a remembered one, or the one SUBMIT just
        // made.  The box itself stays empty, so the key is never in a box.
        const string RememberedHint = "(remembered)";
        const string PasswordHint = "";

        // Debug mode: skip the file check, leave the ticket in a file.
        readonly bool debug;

        // The login under way, if one is.  Null between logins.
        LoginConnection login;

        // The key SUBMIT uses instead of hashing the box, and the username
        // (lowercase) it was made for.  Null once the player types a
        // password or changes the username.
        string key;
        string keyFor;

        // What SUBMIT sent, and what Remember Me ticked will keep once the
        // login works.
        Task<string> sentKey;
        RememberedLogin toKeep;

        // The countdown while the player decides about the other session.
        DispatcherTimer countdown;
        DateTime choiceDeadline;

        // For Avalonia's XAML loader and the designer, which want a
        // constructor with nothing in it.  User mode.
        public LoginScreen() : this(false)
        {
        }

        public LoginScreen(bool debug)
        {
            InitializeComponent();
            this.debug = debug;

            RememberedLogin remembered = RememberedLogin.Load();
            if (remembered != null)
            {
                ServerBox.Text = remembered.ServerIp ?? "";
                PortBox.Text = remembered.ServerPort ?? "";
                UsernameBox.Text = remembered.Username;
                key = remembered.Key;
                keyFor = PasswordKey.AsciiLower(remembered.Username);
                PasswordBox.Watermark = RememberedHint;
                RememberMeBox.IsChecked = true;
                Log.Say("Login: a remembered login for " + remembered.Username + " filled the boxes.");
            }

            // Typing a password drops the remembered key, and so does
            // changing the username, since the key was made from it.
            PasswordBox.TextChanged += (sender, e) =>
            {
                if (key != null && !string.IsNullOrEmpty(PasswordBox.Text))
                    DropKey();
            };
            UsernameBox.TextChanged += (sender, e) =>
            {
                if (key != null && PasswordKey.AsciiLower(UsernameBox.Text ?? "") != keyFor)
                    DropKey();
            };

            // Enter anywhere on the screen is SUBMIT.
            KeyDown += (sender, e) =>
            {
                if (e.Key == Key.Enter && SubmitButton.IsEnabled)
                {
                    e.Handled = true;
                    Submit();
                }
            };
        }

        // The window is closing: stop a login in the middle without a word.
        public void WindowClosing()
        {
            LoginConnection open = login;
            if (open != null)
                open.Cancel();
        }

        // ---------------------------------------------------------------
        // SUBMIT
        // ---------------------------------------------------------------

        void SubmitClicked(object sender, RoutedEventArgs e)
        {
            Submit();
        }

        // Starts the login.  `async void` because it's a button's handler:
        // it returns at the first await, so the window keeps drawing while
        // the key is made, and comes back to the window's thread to finish.
        async void Submit()
        {
            if (login != null)
                return;

            string host = (ServerBox.Text ?? "").Trim();
            string portText = (PortBox.Text ?? "").Trim();
            string name = UsernameBox.Text ?? "";
            string typed = PasswordBox.Text ?? "";
            ushort port;
            if (host == "")
            {
                ShowStatus("Type the server's address.", true);
                return;
            }
            if (!ushort.TryParse(portText, out port) || port == 0)
            {
                ShowStatus("The server port is a number from 1 to 65535.", true);
                return;
            }
            if (name == "")
            {
                ShowStatus("Type your username.", true);
                return;
            }
            if (key == null && typed == "")
            {
                ShowStatus("Type your password.", true);
                return;
            }

            byte[] certificate = ServerCertificate.Load();
            if (certificate == null)
            {
                Log.Error("Login: there's no server certificate to check the server against.  Copy "
                          + "Content/certs/conductor.crt to " + ServerCertificate.FilePath + ".");
                ShowStatus("This launcher has no copy of the server's certificate, so it can't log in.", true);
                return;
            }

            bool keep = RememberMeBox.IsChecked == true;
            toKeep = keep ? new RememberedLogin { ServerIp = host, ServerPort = portText, Username = name } : null;
            if (!keep)
                RememberedLogin.Forget();

            bool madeNow = key == null;
            if (madeNow)
            {
                // Out of the box at once; the hint stands in while the key
                // is made.
                PasswordBox.Text = "";
                sentKey = PasswordKey.MakeAsync(name, typed);
                typed = null;
            }
            else
            {
                Log.Say("Login: using the remembered key.");
                sentKey = Task.FromResult(key);
            }

            ShowChoice(false);
            SetBoxesEnabled(false);
            PlayButton.IsEnabled = false;
            login = LoginConnection.Start(host, port, certificate, ClientVersion.Text, name, sentKey, this);

            if (!madeNow)
                return;

            // The key is kept even when the login fails for another reason,
            // so trying again doesn't mean typing the password again.
            var clock = Stopwatch.StartNew();
            try
            {
                string made = await sentKey;
                Log.Say("Login: the password became its key in " + clock.ElapsedMilliseconds + " ms ("
                        + PasswordKey.Rounds + " rounds).");
                key = made;
                keyFor = PasswordKey.AsciiLower(name);
                PasswordBox.Watermark = RememberedHint;
            }
            catch (Exception e)
            {
                Log.Error("Login: the password's key couldn't be made (" + e.Message + ").");
                DropKey();
            }
        }

        // The key we have is no good (or the player started typing), so
        // the password has to be typed.  A remembered file stays until the
        // next login that works writes over it or forgets it.
        void DropKey()
        {
            key = null;
            keyFor = null;
            PasswordBox.Watermark = PasswordHint;
        }

        void SetBoxesEnabled(bool enabled)
        {
            ServerBox.IsEnabled = enabled;
            PortBox.IsEnabled = enabled;
            UsernameBox.IsEnabled = enabled;
            PasswordBox.IsEnabled = enabled;
            RememberMeBox.IsEnabled = enabled;
            SubmitButton.IsEnabled = enabled;
        }

        // ---------------------------------------------------------------
        // The other session
        // ---------------------------------------------------------------

        void LogOtherOutClicked(object sender, RoutedEventArgs e)
        {
            Choose(true);
        }

        void LogOffClicked(object sender, RoutedEventArgs e)
        {
            Choose(false);
        }

        void Choose(bool logOtherOut)
        {
            StopCountdown();
            ShowChoice(false);
            ShowStatus(logOtherOut ? "Logging the other session out..." : "Logging off...", false);
            LoginConnection open = login;
            if (open != null)
                open.Choose(logOtherOut);
        }

        void ShowChoice(bool shown)
        {
            ChoiceRow.IsVisible = shown;
        }

        void Countdown(object sender, EventArgs e)
        {
            double left = Math.Ceiling((choiceDeadline - DateTime.UtcNow).TotalSeconds);
            ShowStatus("This account is already playing somewhere else. Log that session out, or log off? "
                       + Math.Max(0, left) + " s", false);
        }

        void StopCountdown()
        {
            if (countdown != null)
            {
                countdown.Stop();
                countdown = null;
            }
        }

        // ---------------------------------------------------------------
        // The status box
        // ---------------------------------------------------------------

        // Trouble shows as a dark red box behind the words.
        void ShowStatus(string words, bool trouble)
        {
            StatusLine.Text = words ?? "";
            StatusBox.IsVisible = StatusLine.Text != "";
            if (trouble)
                StatusBox.Classes.Add("trouble");
            else
                StatusBox.Classes.Remove("trouble");
        }

        // ---------------------------------------------------------------
        // ILoginListener: from the login's thread, put back on ours
        // ---------------------------------------------------------------

        public void LoginStatus(LoginConnection from, string words)
        {
            Dispatcher.UIThread.Post(() =>
            {
                if (from == login)
                    ShowStatus(words, false);
            });
        }

        public void AskedAboutOtherSession(LoginConnection from)
        {
            Dispatcher.UIThread.Post(() =>
            {
                if (from != login)
                    return;
                choiceDeadline = DateTime.UtcNow.AddSeconds(LoginConnection.ChoiceSeconds);
                ShowChoice(true);
                StopCountdown();
                countdown = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(250) };
                countdown.Tick += Countdown;
                countdown.Start();
                Countdown(null, EventArgs.Empty);
            });
        }

        public void LoginEnded(LoginConnection from, string why, bool wrongPassword)
        {
            Dispatcher.UIThread.Post(() =>
            {
                if (from != login)
                    return;
                login = null;
                StopCountdown();
                ShowChoice(false);
                if (wrongPassword)
                    DropKey();
                SetBoxesEnabled(true);
                ShowStatus(why, true);
            });
        }

        public void TicketCame(LoginConnection from, string host, ushort udpPort, string token)
        {
            Dispatcher.UIThread.Post(() =>
            {
                if (from != login)
                    return;
                login = null;
                StopCountdown();

                if (toKeep != null && sentKey != null && sentKey.Status == TaskStatus.RanToCompletion)
                {
                    toKeep.Key = sentKey.Result;
                    toKeep.Save();
                    toKeep = null;
                    Log.Say("Login: remembered for next time, in " + RememberedLogin.FilePath + ".");
                }

                SetBoxesEnabled(true);
                if (debug)
                {
                    // For an Ensemble already running in Unity's editor.
                    try
                    {
                        DebugTicket.Save(host, udpPort, token);
                        Log.Say("Login: debug mode, so the ticket went to " + DebugTicket.FilePath + ".");
                        ShowStatus("Logged in (debug mode). The ticket is in " + DebugTicket.FilePath + " for an "
                                   + "Ensemble running in the editor. It's good once, for 30 seconds.", false);
                    }
                    catch (Exception e)
                    {
                        Log.Error("Login: the debug ticket couldn't be written (" + e.Message + ").");
                        ShowStatus("Logged in, but the debug ticket couldn't be written: " + e.Message, true);
                    }
                    return;
                }

                // The token is good once, for 30 seconds, and nothing can
                // use it yet: Ensemble doesn't take a ticket from Soundcheck
                // until its next step.  So it's dropped here, unlogged, and
                // the screen says the login worked.
                ShowStatus("Logged in. The server's game port is UDP " + udpPort + ". PLAY comes with Ensemble's "
                           + "next step; the ticket was dropped.", false);
            });
        }
    }
}
