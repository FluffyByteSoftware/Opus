// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/Session.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The player's session with the server, from SUBMIT to the login screen
// again.  It runs the two connections in turn (the login over TCP, then
// the game over UDP), keeps where the player is, makes the asks at
// character select (the list, CREATE, DELETE, RESET HOME, PLAY) and sends
// what's typed in the chat box once the character is in the world, and
// tells the screens through its events.  Main thread only: the connections' threads reach it
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
        InWorld,
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

        // The ask at character select waiting on its answer (its packet
        // type), or 0.  The server takes one at a time, so the screen greys
        // its buttons while there's one.
        public static byte Asking { get; private set; }

        public static bool Waiting
        {
            get { return Asking != 0; }
        }

        // What character select's status line says, and whether it's
        // something gone wrong.
        public static string Said { get; private set; }
        public static bool SaidTrouble { get; private set; }

        // The character's name once it's in the world, or null.
        public static string InWorldAs { get; private set; }

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

        // Anything character select shows changed: the list, its trouble,
        // an ask started or answered, the status line, the world.
        public static event Action CharacterSelectChanged;

        // An ask was answered (its packet type, and whether the server did
        // it), rung before CharacterSelectChanged.  The cards close on a yes.
        public static event Action<byte, bool> AskAnswered;

        // The session is over and the player is back at the login: why,
        // and whether it's something gone wrong.
        public static event Action<string, bool> BackAtLogin;

        // PLAY's answer came: the character is in the world, and the HUD
        // takes over from character select.
        public static event Action ReachedWorld;

        // A line for the chat box that isn't the player's own: the chat,
        // a command refused and why, the server not answering.
        public static event Action<string> ChatLine;

        // /who's answer, for the chat box to draw.
        public static event Action<WhoAnswer> WhoAnswered;

        static LoginConnection login;
        static GameConnection game;

        // The name of the character RESET HOME was asked for, for its
        // answer (a CommandAccepted, which carries no name).
        static string sentHome;

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
            ForgetCharacterSelect();
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

        // ---------------------------------------------------------------
        // Character select's asks.  Each one returns false, with the reason
        // on the status line, when it can't be sent; the screen's buttons
        // are greyed in the same cases, so that's rare.
        // ---------------------------------------------------------------

        // A character's name is 4 to 20 letters, a to z, and only the first
        // can be a capital: the server's own rule and its own words, checked
        // here first so a name that can't work never costs the server an
        // ask.  The server checks again either way.
        public const string NameRule = "A character's name is 4 to 20 letters, a to z, and only the first can be "
                                       + "a capital.";

        public static bool NameFollowsRule(string name)
        {
            if (name == null || name.Length < 4 || name.Length > 20)
                return false;
            for (int i = 0; i < name.Length; i++)
            {
                char c = name[i];
                bool small = c >= 'a' && c <= 'z';
                bool capital = c >= 'A' && c <= 'Z';
                if (!small && !(capital && i == 0))
                    return false;
            }
            return true;
        }

        // The word that deletes a character, any capitals.  The server
        // checks it again.
        public const string DeleteWord = "DELETE";

        public static bool CreateCharacter(string name)
        {
            if (!CanAsk())
                return false;
            if (!NameFollowsRule(name))
            {
                SayHere(NameRule, true);
                return false;
            }
            if (Characters != null && Characters.Length >= 3)
            {
                SayHere("All three character slots are full.", true);
                return false;
            }
            Debug.Log("Game: asking to make \"" + name + "\".");
            AskServer(Protocol.CreateCharacter, name);
            return true;
        }

        public static bool DeleteCharacter(string uuid, string typed)
        {
            if (!CanAsk())
                return false;
            if (typed == null || typed.Trim().ToUpperInvariant() != DeleteWord)
            {
                SayHere("Type DELETE to delete a character.", true);
                return false;
            }
            Debug.Log("Game: asking to delete " + uuid + ".");
            AskServer(Protocol.DeleteCharacter, uuid, typed.Trim());
            return true;
        }

        public static bool ResetHome(string uuid)
        {
            if (!CanAsk())
                return false;
            Debug.Log("Game: asking to send " + uuid + " home.");
            CharacterEntry character = Find(uuid);
            sentHome = character != null ? character.Name : "Your character";
            AskServer(Protocol.CharacterRequestResetHome, uuid);
            return true;
        }

        public static bool Play(string uuid)
        {
            if (!CanAsk())
                return false;
            Debug.Log("Game: asking to play " + uuid + ".");
            AskServer(Protocol.UserPressPlay, uuid);
            SayHere("Entering the world...", false);
            return true;
        }

        // The character in this slot, or null.
        public static CharacterEntry InSlot(int slot)
        {
            if (Characters == null)
                return null;
            foreach (CharacterEntry character in Characters)
            {
                if (character.Slot == slot)
                    return character;
            }
            return null;
        }

        public static CharacterEntry Find(string uuid)
        {
            if (Characters == null || uuid == null)
                return null;
            foreach (CharacterEntry character in Characters)
            {
                if (character.Uuid == uuid)
                    return character;
            }
            return null;
        }

        static bool CanAsk()
        {
            if (Stage != SessionStage.AtCharacterSelect || game == null)
                return false;
            if (Waiting)
            {
                SayHere("Still waiting on the server.", false);
                return false;
            }
            return true;
        }

        static void AskServer(byte kind, params string[] fields)
        {
            Asking = kind;
            game.Ask(kind, fields);
            Changed();
        }

        // ---------------------------------------------------------------
        // The chat box
        // ---------------------------------------------------------------

        // A line typed in the chat box, as it was typed.  /camp is the
        // client's own and never goes to the server.  Anything else goes as
        // a PlayerCommand, and replaces a line still waiting on its answer,
        // since the server takes one at a time.  The server's answer comes
        // back through ChatLine (a refusal; a line that went out comes back
        // as chat) or WhoAnswered.
        public static void SendLine(string line)
        {
            if (Camped(line))
                return;
            if (Stage != SessionStage.InWorld || game == null)
            {
                Chat("You're not in the world.");
                return;
            }
            Asking = Protocol.PlayerCommand;
            game.Ask(Protocol.PlayerCommand, line);
        }

        // /camp, any capitals, logs out to the login; /camp desktop logs out
        // and closes the game (Jacob, 2026-10-02).  True when the line was a
        // /camp, whatever came after it.
        static bool Camped(string line)
        {
            string typed = line.Trim();
            int space = typed.IndexOf(' ');
            string word = space < 0 ? typed : typed.Substring(0, space);
            if (!string.Equals(word, "/camp", StringComparison.OrdinalIgnoreCase))
                return false;

            string rest = space < 0 ? "" : typed.Substring(space + 1).Trim();
            if (rest == "")
            {
                Debug.Log("Game: camped to the login.");
                LogOut();
            }
            else if (string.Equals(rest, "desktop", StringComparison.OrdinalIgnoreCase))
            {
                Debug.Log("Game: camped to the desktop.");
                LogOut();
                CloseTheGame();
            }
            else
            {
                Chat("Try /camp, or /camp desktop.");
            }
            return true;
        }

        // The game closing itself.  In the editor Application.Quit() does
        // nothing, so Play mode stops instead.
        static void CloseTheGame()
        {
#if UNITY_EDITOR
            UnityEditor.EditorApplication.isPlaying = false;
#else
            Application.Quit();
#endif
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
            ForgetCharacterSelect();
            Asking = Protocol.CharacterListRequest;
            game.Ask(Protocol.CharacterListRequest);
            if (ReachedCharacterSelect != null)
                ReachedCharacterSelect();
        }

        internal static void CharactersCame(GameConnection from, CharacterEntry[] characters)
        {
            if (from != game)
                return;
            Asking = 0;
            Characters = characters;
            CharactersTrouble = null;
            Answered(Protocol.CharacterListRequest, true);
        }

        // CREATE's answer.  Made: the list is asked for again, since the
        // answer doesn't say which slot it went in.
        internal static void CreateAnswered(GameConnection from, byte answer, string message)
        {
            if (from != game)
                return;
            Asking = 0;
            bool made = answer == 0;
            SayHere(message, !made);
            Answered(Protocol.CreateCharacter, made);
            if (made)
                AskForList();
        }

        // DELETE's answer.  Gone: the list again.
        internal static void DeleteAnswered(GameConnection from, byte answer, string message)
        {
            if (from != game)
                return;
            Asking = 0;
            bool deleted = answer == 0;
            SayHere(message, !deleted);
            Answered(Protocol.DeleteCharacter, deleted);
            if (deleted)
                AskForList();
        }

        // A CommandAccepted.  At character select only RESET HOME gets one.
        // In the world it's a line that went out, and the line itself comes
        // back as chat, so there's nothing to show for it.
        internal static void AskAccepted(GameConnection from)
        {
            if (from != game)
                return;
            byte kind = Asking;
            Asking = 0;
            if (kind == Protocol.PlayerCommand)
                return;
            if (kind == Protocol.CharacterRequestResetHome)
                SayHere(sentHome + " is back at 0, 0, 0.", false);
            Answered(kind, true);
        }

        // A CommandRefused, for whichever ask was waiting: the list's goes
        // in the list, a typed line's in the chat box, anything else on the
        // status line.
        internal static void AskRefused(GameConnection from, string why)
        {
            if (from != game)
                return;
            byte kind = Asking;
            Asking = 0;
            if (kind == Protocol.PlayerCommand)
            {
                Chat(why);
                return;
            }
            if (kind == Protocol.CharacterListRequest)
                CharactersTrouble = why;
            else
                SayHere(why, true);
            Answered(kind, false);
        }

        internal static void AskUnanswered(GameConnection from, byte kind)
        {
            if (from != game)
                return;
            Asking = 0;
            if (kind == Protocol.PlayerCommand)
            {
                Chat("The server didn't answer.");
                return;
            }
            if (kind == Protocol.CharacterListRequest)
                CharactersTrouble = "The server didn't send the list of characters.";
            else
                SayHere("The server didn't answer.", true);
            Answered(kind, false);
        }

        // A PleaseWait for the ask that's waiting: the server is on it and
        // says it'll be a moment, so the words go on the status line (a
        // character still being saved from its last session, say).  The
        // buttons stay grey, since the ask is still out.
        internal static void AskWaiting(GameConnection from, string words)
        {
            if (from != game || Asking == 0)
                return;
            if (Asking == Protocol.PlayerCommand)
            {
                Chat(words);
                return;
            }
            SayHere(words, false);
        }

        // PLAY's answer: the character is in the world.  Character select is
        // behind the player now; the way out is LOG OUT, to the login.
        internal static void EnteredWorld(GameConnection from, string name)
        {
            if (from != game)
                return;
            Asking = 0;
            Stage = SessionStage.InWorld;
            InWorldAs = name;
            SayHere("", false);
            Answered(Protocol.UserPressPlay, true);
            if (ReachedWorld != null)
                ReachedWorld();
        }

        // The chat, as it went out to everybody in the world.
        internal static void ChatCame(GameConnection from, string[] lines)
        {
            if (from != game)
                return;
            foreach (string line in lines)
                Chat(line);
        }

        internal static void WhoCame(GameConnection from, WhoAnswer who)
        {
            if (from != game)
                return;
            Asking = 0;
            if (WhoAnswered != null)
                WhoAnswered(who);
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

        static void Chat(string line)
        {
            if (ChatLine != null)
                ChatLine(line);
        }

        // Character select's status line.
        static void SayHere(string words, bool trouble)
        {
            Said = words;
            SaidTrouble = trouble;
            Changed();
        }

        static void AskForList()
        {
            Asking = Protocol.CharacterListRequest;
            game.Ask(Protocol.CharacterListRequest);
            Changed();
        }

        static void Answered(byte kind, bool done)
        {
            if (AskAnswered != null)
                AskAnswered(kind, done);
            Changed();
        }

        static void Changed()
        {
            if (CharacterSelectChanged != null)
                CharacterSelectChanged();
        }

        static void ForgetCharacterSelect()
        {
            Characters = null;
            CharactersTrouble = null;
            Asking = 0;
            Said = "";
            SaidTrouble = false;
            InWorldAs = null;
        }

        static void Finish(string why, bool trouble)
        {
            Stage = SessionStage.LoggedOut;
            ForgetCharacterSelect();
            if (BackAtLogin != null)
                BackAtLogin(why, trouble);
        }
    }
}
