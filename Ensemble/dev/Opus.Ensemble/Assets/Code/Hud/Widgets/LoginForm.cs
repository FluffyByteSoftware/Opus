// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Hud/Widgets/LoginForm.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Where the login's widgets meet.  Each one hands its box over here as it's
// built, so SUBMIT can read the others, and a remembered login
// (RememberedLogin.cs) fills them in.  What SUBMIT does is here too: the
// password becomes its key straight away, and Remember Me keeps the key.

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

        // The file, read the first time a box asks for it.  Null when
        // there's no remembered login.
        static RememberedLogin remembered;
        static bool readFile;

        // The key SUBMIT uses instead of hashing the box, and the username
        // (lowercase) it was made for.  Null once the player types a
        // password or changes the username.
        static string key;
        static string keyFor;

        // True while SUBMIT is making a key, so a second press waits.
        static bool busy;

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

        // Makes the password's key (unless there's one already), then keeps
        // it or forgets it as Remember Me says.  It doesn't log in yet: that
        // comes with the network client, and sends the key where the
        // password went.  The key itself is never logged.
        //
        // `async void` because it's a button's handler: it returns at the
        // first await, so the screen keeps drawing while the key is made,
        // and Unity brings it back to the main thread to finish.
        public static async void Submit()
        {
            if (busy)
                return;
            if (username == null || password == null)
            {
                Debug.LogWarning("Login: the login screen has no Username or Password box, so SUBMIT can't run.");
                return;
            }

            string name = username.value;
            if (name == "")
            {
                Debug.Log("Login: no username.");
                return;
            }
            if (key == null && (password.value == "" || password.value == StandIn))
            {
                Debug.Log("Login: no password.");
                return;
            }

            busy = true;
            username.SetEnabled(false);
            password.SetEnabled(false);
            try
            {
                if (key == null)
                {
                    // Out of the box at once; the dots stand in while the
                    // key is made.
                    string typed = password.value;
                    password.SetValueWithoutNotify(StandIn);

                    var clock = System.Diagnostics.Stopwatch.StartNew();
                    string made = await PasswordKey.MakeAsync(name, typed);
                    Debug.Log("Login: the password became its key in " + clock.ElapsedMilliseconds + " ms ("
                              + PasswordKey.Rounds + " rounds).");

                    key = made;
                    keyFor = PasswordKey.AsciiLower(name);
                }
                else
                {
                    Debug.Log("Login: using the remembered key.");
                }

                bool keep = rememberMe != null && rememberMe.value;
                if (keep)
                {
                    var login = new RememberedLogin
                    {
                        ServerIp = serverIp != null ? serverIp.value : "",
                        ServerPort = serverPort != null ? serverPort.value : "",
                        Username = name,
                        Key = key,
                    };
                    login.Save();
                    remembered = login;
                    readFile = true;
                    Debug.Log("Login: remembered for next time, in " + RememberedLogin.FilePath + ".");
                }
                else
                {
                    if (Remembered != null)
                        Debug.Log("Login: Remember Me is off, so the remembered login was forgotten.");
                    RememberedLogin.Forget();
                    remembered = null;
                    readFile = true;
                }

                Debug.Log("Login: the key is ready.  It doesn't log in yet; that comes with the network client.");
            }
            catch (System.Exception e)
            {
                Debug.LogError("Login: the password's key couldn't be made (" + e.Message + ").");
                DropKey();
            }
            finally
            {
                username.SetEnabled(true);
                password.SetEnabled(true);
                busy = false;
            }
        }
    }
}
