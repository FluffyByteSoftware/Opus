// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/Session.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The player's session with the server, from SUBMIT to the login screen
// again.  It runs the two connections in turn (the login over TCP, then
// the game over UDP), keeps where the player is, and tells the screens
// through its events.  Main thread only: the connections' threads reach it
// through MainThread.Post, and every message from a connection that's
// already been dropped is ignored.
//
// There's no reconnect.  However the session ends (a Kicked, the server
// going quiet, LOG OUT), the player is back at the login and starts over.

using System;
using System.Threading.Tasks;
using UnityEngine;

namespace Opus.Net
{
    public enum SessionStage
    {
        LoggedOut,
        LoggingIn,
        AskedAboutOtherSession,
        Connecting,
        AtCharacterSelect,
    }

    public static class Session
    {
        // The server's certificate as PEM text, from ScreenRoot's Server
        // Certificate slot.  The client trusts that one and no other.
        public static string Certificate;

        public static SessionStage Stage { get; private set; }

        public static bool Busy
        {
            get { return Stage != SessionStage.LoggedOut; }
        }

        // When the player's time to answer about the other session runs
        // out, UTC.
        public static DateTime ChoiceDeadline { get; private set; }

        // The account's characters, in slot order, or null while they're
        // still being asked for.
        public static CharacterEntry[] Characters { get; private set; }

        // Why there's no list of characters, or null.
        public static string CharactersTrouble { get; private set; }

        // What's happening, for the login's status line, and whether it's
        // something gone wrong.
        public static event Action<string, bool> StatusChanged;

        // The account is already in the world from somewhere else: the
        // player picks, before ChoiceDeadline, with Choose().
        public static event Action OtherSessionAsked;

        // The password was right and the Ticket came.
        public static event Action LoggedIn;

        // The server said Invalid Credentials, so whatever key was sent is
        // no good (or the username isn't an account).
        public static event Action WrongPassword;

        // The UDP side said Welcome: the player is at character select.
        public static event Action ReachedCharacterSelect;

        // Characters or CharactersTrouble changed.
        public static event Action CharactersChanged;

        // The session is over and the player is back at the login: why,
        // and whether it's something gone wrong.
        public static event Action<string, bool> BackAtLogin;

        static LoginConnection login;
        static GameConnection game;

        // ---------------------------------------------------------------
        // From the screens
        // ---------------------------------------------------------------

        // SUBMIT.  Starts connecting at once; the key can still be being
        // made.  False if it couldn't start (it says why on the status line).
        public static bool LogIn(string host, ushort port, string username, Task<string> key)
        {
            if (Busy)
                return false;

            byte[] carried = ServerCertificate.FromPem(Certificate);
            if (carried == null)
            {
                Debug.LogError("Login: there's no server certificate to check the server against.  Drag "
                               + "conductor_crt.txt, from Assets/Data/Certs, onto ScreenRoot's Server Certificate.");
                Say("This client has no copy of the server's certificate, so it can't log in.", true);
                return false;
            }

            Stage = SessionStage.LoggingIn;
            Characters = null;
            CharactersTrouble = null;
            // Player Settings' Version.  networking.cfg's client_versions has
            // to list it, or the server says Outdated Client Failure.
            login = LoginConnection.Start(host, port, carried, Application.version, username, key);
            return true;
        }

        // The player's answer about the other session: true logs it out,
        // false logs this one off.
        public static void Choose(bool logOtherOut)
        {
            if (Stage != SessionStage.AskedAboutOtherSession || login == null)
                return;
            Stage = SessionStage.LoggingIn;
            login.Choose(logOtherOut);
        }

        // LOG OUT: a Goodbye to the server, and back to the login.
        public static void LogOut()
        {
            if (!Busy)
                return;
            Debug.Log("Game: logged out.");
            Drop();
            Finish("Logged out.", false);
        }

        // The game is closing (or Play mode stopping).  The same as LOG OUT,
        // with nobody left to tell.
        public static void Quit()
        {
            Drop();
            Stage = SessionStage.LoggedOut;
        }

        static void Drop()
        {
            if (game != null)
            {
                game.Close(true);
                game = null;
            }
            if (login != null)
            {
                login.Cancel();
                login = null;
            }
        }

        // ---------------------------------------------------------------
        // From the login's thread, through MainThread
        // ---------------------------------------------------------------

        internal static void LoginStatus(LoginConnection from, string words)
        {
            if (from == login)
                Say(words, false);
        }

        internal static void AskedAboutOtherSession(LoginConnection from)
        {
            if (from != login)
                return;
            Stage = SessionStage.AskedAboutOtherSession;
            ChoiceDeadline = DateTime.UtcNow.AddSeconds(LoginConnection.ChoiceSeconds);
            if (OtherSessionAsked != null)
                OtherSessionAsked();
        }

        internal static void LoginEnded(LoginConnection from, string why, bool wrongPassword)
        {
            if (from != login)
                return;
            login = null;
            if (wrongPassword && WrongPassword != null)
                WrongPassword();
            Finish(why, true);
        }

        internal static void TicketCame(LoginConnection from, string host, ushort udpPort, string token)
        {
            if (from != login)
                return;
            login = null;
            if (LoggedIn != null)
                LoggedIn();

            try
            {
                game = GameConnection.Start(host, udpPort, token);
            }
            catch (Exception e)
            {
                Debug.LogWarning("Game: couldn't open UDP to " + host + ":" + udpPort + " (" + e.Message + ").");
                Finish("Couldn't reach the server's game port.", true);
                return;
            }
            Stage = SessionStage.Connecting;
            Say("Joining the world...", false);
        }

        // ---------------------------------------------------------------
        // From the game's threads, through MainThread
        // ---------------------------------------------------------------

        internal static void Welcomed(GameConnection from)
        {
            if (from != game)
                return;
            Stage = SessionStage.AtCharacterSelect;
            Characters = null;
            CharactersTrouble = null;
            game.Ask(Protocol.CharacterListRequest);
            if (ReachedCharacterSelect != null)
                ReachedCharacterSelect();
        }

        internal static void CharactersCame(GameConnection from, CharacterEntry[] characters)
        {
            if (from != game)
                return;
            Characters = characters;
            CharactersTrouble = null;
            if (CharactersChanged != null)
                CharactersChanged();
        }

        // Character select only asks for the list so far, so a refusal or
        // no answer at all is about the list.
        internal static void AskRefused(GameConnection from, string why)
        {
            if (from != game)
                return;
            CharactersTrouble = why;
            if (CharactersChanged != null)
                CharactersChanged();
        }

        internal static void AskUnanswered(GameConnection from, byte kind)
        {
            if (from != game)
                return;
            CharactersTrouble = "The server didn't send the list of characters.";
            if (CharactersChanged != null)
                CharactersChanged();
        }

        internal static void GameEnded(GameConnection from, string why)
        {
            if (from != game)
                return;
            game = null;
            Finish(why, true);
        }

        // ---------------------------------------------------------------

        static void Say(string words, bool trouble)
        {
            if (StatusChanged != null)
                StatusChanged(words, trouble);
        }

        static void Finish(string why, bool trouble)
        {
            Stage = SessionStage.LoggedOut;
            Characters = null;
            CharactersTrouble = null;
            if (BackAtLogin != null)
                BackAtLogin(why, trouble);
        }
    }
}
