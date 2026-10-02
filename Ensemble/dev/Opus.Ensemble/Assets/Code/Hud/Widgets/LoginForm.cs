// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/LoginForm.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Where the login's widgets meet.  Each one hands its box over here as it's
// built, so SUBMIT can read the others, and a remembered login
// (RememberedLogin.cs) fills them in.  What SUBMIT does is here too: the
// password becomes its key straight away, the login starts at once
// (Session.cs), and once it works Remember Me keeps the key.  The status
// line under SUBMIT says how it's going.

using System;
using System.Threading.Tasks;
using Opus.Net;
using Opus.Security;
using UnityEngine;
using UnityEngine.UIElements;

namespace Opus.Hud
{
    public static class LoginForm
    {
        // What sits in the Password box, as dots, while there's a key to use
        // instead of a password: a remembered one, or the one SUBMIT just
        // made.  It starts with a character no keyboard types, so it can
        // never be somebody's real password.
        const string StandIn = "\u0001remembered";

        static TextField serverIp;
        static TextField serverPort;
        static TextField username;
        static TextField password;
        static Toggle rememberMe;
        static Button submit;

        // The status line and its two buttons, shown when the account is
        // already playing somewhere else.
        static Label statusLine;
        static VisualElement statusBox;
        static VisualElement choiceRow;
        static IVisualElementScheduledItem countdown;

        // What the status line says, kept so a login screen built again
        // (after a kick, say) shows why it's back.
        static string status = "";
        static bool statusTrouble;

        // The file, read the first time a box asks for it.  Null when
        // there's no remembered login.
        static RememberedLogin remembered;
        static bool readFile;

        // The key SUBMIT uses instead of hashing the box, and the username
        // (lowercase) it was made for.  Null once the player types a
        // password or changes the username.
        static string key;
        static string keyFor;

        // What SUBMIT sent, and whether Remember Me was ticked, for when the
        // login works and the key is kept.
        static Task<string> sentKey;
        static RememberedLogin toKeep;

        static RememberedLogin Remembered
        {
            get
            {
                if (!readFile)
                {
                    remembered = RememberedLogin.Load();
                    readFile = true;
                }
                return remembered;
            }
        }

        // ---------------------------------------------------------------
        // The widgets hand their boxes over as they're built
        // ---------------------------------------------------------------

        public static void ServerIpBuilt(TextField field)
        {
            serverIp = field;
            if (Remembered != null && !string.IsNullOrEmpty(Remembered.ServerIp))
                field.SetValueWithoutNotify(Remembered.ServerIp);
        }

        public static void ServerPortBuilt(TextField field)
        {
            serverPort = field;
            if (Remembered != null && !string.IsNullOrEmpty(Remembered.ServerPort))
                field.SetValueWithoutNotify(Remembered.ServerPort);
        }

        public static void UsernameBuilt(TextField field)
        {
            username = field;
            if (Remembered != null)
                field.SetValueWithoutNotify(Remembered.Username);

            // The key is made from the username, so a different name means
            // the key is no good and the password has to be typed.
            field.RegisterValueChangedCallback(changed =>
            {
                if (key != null && PasswordKey.AsciiLower(changed.newValue) != keyFor)
                    DropKey();
            });
        }

        public static void PasswordBuilt(TextField field)
        {
            password = field;

            // A freshly built screen always shows what the file says.
            key = null;
            keyFor = null;
            if (Remembered != null)
            {
                key = Remembered.Key;
                keyFor = PasswordKey.AsciiLower(Remembered.Username);
                field.SetValueWithoutNotify(StandIn);
            }

            // Clicking in clears the dots so typing starts fresh; clicking
            // out of an empty box puts them back if the key's still good.
            field.RegisterCallback<FocusInEvent>(_ =>
            {
                if (field.value == StandIn)
                    field.SetValueWithoutNotify("");
            });
            field.RegisterCallback<FocusOutEvent>(_ =>
            {
                if (field.value == "" && key != null)
                    field.SetValueWithoutNotify(StandIn);
            });

            // Anything typed is a new password, and the key is dropped.
            field.RegisterValueChangedCallback(changed =>
            {
                if (changed.newValue != "" && changed.newValue != StandIn)
                {
                    key = null;
                    keyFor = null;
                }
            });
        }

        public static void RememberMeBuilt(Toggle toggle)
        {
            rememberMe = toggle;
            toggle.SetValueWithoutNotify(Remembered != null);
        }

        public static void SubmitBuilt(Button button)
        {
            submit = button;
        }

        public static void StatusBuilt(VisualElement box, Label line, VisualElement choices)
        {
            statusBox = box;
            statusLine = line;
            choiceRow = choices;
            countdown = null;
            ShowStatus(status, statusTrouble);
            ShowChoice(false);
        }

        static void DropKey()
        {
            key = null;
            keyFor = null;
            if (password != null && password.value == StandIn)
                password.SetValueWithoutNotify("");
        }

        // ---------------------------------------------------------------
        // SUBMIT
        // ---------------------------------------------------------------

        // Starts the login straight away, while the password's key is made
        // on a worker thread (unless there's a key already); the login sends
        // it the moment it's ready.  Remember Me unticked forgets the kept
        // login at once; ticked, the login is kept once the server lets us
        // in, so a wrong password is never remembered.  The key itself is
        // never logged.
        //
        // `async void` because it's a button's handler: it returns at the
        // first await, so the screen keeps drawing while the key is made,
        // and Unity brings it back to the main thread to finish.
        public static async void Submit()
        {
            if (Session.Busy)
                return;
            if (username == null || password == null || serverIp == null || serverPort == null)
            {
                Debug.LogWarning("Login: the login screen is missing one of its boxes (Server IP, Server Port, "
                                 + "Username, Password), so SUBMIT can't run.");
                return;
            }

            string host = serverIp.value.Trim();
            string portText = serverPort.value.Trim();
            string name = username.value;
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
            if (key == null && (password.value == "" || password.value == StandIn))
            {
                ShowStatus("Type your password.", true);
                return;
            }

            bool keep = rememberMe != null && rememberMe.value;
            toKeep = keep ? new RememberedLogin { ServerIp = host, ServerPort = portText, Username = name } : null;
            if (!keep)
            {
                if (Remembered != null)
                    Debug.Log("Login: Remember Me is off, so the remembered login was forgotten.");
                RememberedLogin.Forget();
                remembered = null;
                readFile = true;
            }

            string typed = null;
            if (key == null)
            {
                // Out of the box at once; the dots stand in while the key
                // is made.
                typed = password.value;
                password.SetValueWithoutNotify(StandIn);
                sentKey = PasswordKey.MakeAsync(name, typed);
            }
            else
            {
                Debug.Log("Login: using the remembered key.");
                sentKey = Task.FromResult(key);
            }

            if (Session.LogIn(host, port, name, sentKey))
                SetBoxesEnabled(false);

            if (typed == null)
                return;

            // The key is kept even when the login couldn't start, so trying
            // again doesn't mean typing the password again.
            var clock = System.Diagnostics.Stopwatch.StartNew();
            try
            {
                string made = await sentKey;
                Debug.Log("Login: the password became its key in " + clock.ElapsedMilliseconds + " ms ("
                          + PasswordKey.Rounds + " rounds).");
                key = made;
                keyFor = PasswordKey.AsciiLower(name);
            }
            catch (Exception e)
            {
                Debug.LogError("Login: the password's key couldn't be made (" + e.Message + ").");
                DropKey();
            }
        }

        // The player's answer when the account is already playing somewhere
        // else: true kicks that session, false logs this one off.
        public static void Choose(bool kickOther)
        {
            StopCountdown();
            ShowChoice(false);
            ShowStatus(kickOther ? "Kicking the other session..." : "Logging off...", false);
            Session.Choose(kickOther);
        }

        // ---------------------------------------------------------------
        // What the session says
        // ---------------------------------------------------------------

        // ScreenRoot starts and stops the listening, as it comes and goes.
        // Taking each off before putting it on means a second Listen() never
        // hears everything twice.
        public static void Listen()
        {
            StopListening();
            Session.StatusChanged += ShowStatus;
            Session.OtherSessionAsked += OtherSessionAsked;
            Session.LoggedIn += LoggedIn;
            Session.WrongPassword += WrongPassword;
            Session.BackAtLogin += BackAtLogin;
        }

        public static void StopListening()
        {
            Session.StatusChanged -= ShowStatus;
            Session.OtherSessionAsked -= OtherSessionAsked;
            Session.LoggedIn -= LoggedIn;
            Session.WrongPassword -= WrongPassword;
            Session.BackAtLogin -= BackAtLogin;
        }

        static void LoggedIn()
        {
            if (toKeep == null || sentKey == null || sentKey.Status != TaskStatus.RanToCompletion)
                return;
            toKeep.Key = sentKey.Result;
            toKeep.Save();
            remembered = toKeep;
            readFile = true;
            toKeep = null;
            Debug.Log("Login: remembered for next time, in " + RememberedLogin.FilePath + ".");
        }

        // The key we sent is no good, so the password has to be typed.  A
        // remembered file stays until the next login that works writes over
        // it or forgets it.
        static void WrongPassword()
        {
            DropKey();
        }

        static void BackAtLogin(string why, bool trouble)
        {
            StopCountdown();
            ShowChoice(false);
            ShowStatus(why, trouble);
            SetBoxesEnabled(true);
        }

        // The account is already in the world from somewhere else.  The
        // two buttons show, and the line counts down the time left to pick.
        static void OtherSessionAsked()
        {
            ShowChoice(true);
            Countdown();
            if (statusLine != null)
                countdown = statusLine.schedule.Execute(Countdown).Every(250);
        }

        static void Countdown()
        {
            if (Session.Stage != SessionStage.AskedAboutOtherSession)
            {
                StopCountdown();
                return;
            }
            double left = Math.Ceiling((Session.ChoiceDeadline - DateTime.UtcNow).TotalSeconds);
            ShowStatus("This account is already playing somewhere else. Kick that session, or log off? "
                       + Math.Max(0, left) + " s", false);
        }

        static void StopCountdown()
        {
            if (countdown != null)
            {
                countdown.Pause();
                countdown = null;
            }
        }

        // ---------------------------------------------------------------
        // The screen
        // ---------------------------------------------------------------

        static void ShowStatus(string words, bool trouble)
        {
            status = words ?? "";
            statusTrouble = trouble;
            if (statusLine != null)
                statusLine.text = status;
            // Trouble shows as a dark red band behind the words.  Not the
            // words' colour: ScreenRoot's Login Text Color is set on every
            // piece of text, and would win.
            if (statusBox != null)
                statusBox.EnableInClassList("login-status--trouble", trouble && status != "");
        }

        static void ShowChoice(bool show)
        {
            if (choiceRow != null)
                choiceRow.style.display = show ? DisplayStyle.Flex : DisplayStyle.None;
        }

        // Everything the player could change, off while the login runs.
        static void SetBoxesEnabled(bool enabled)
        {
            VisualElement[] all = { serverIp, serverPort, username, password, rememberMe, submit };
            foreach (VisualElement element in all)
            {
                if (element != null)
                    element.SetEnabled(enabled);
            }
        }
    }
}
