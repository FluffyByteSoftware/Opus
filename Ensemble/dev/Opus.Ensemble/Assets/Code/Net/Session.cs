// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/Session.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The player's session with the server, from the ticket to the launcher
// again.  The launcher (Soundcheck) did the login; the game starts with
// the ticket it earned (Ticket.cs), opens UDP to the server, keeps where
// the player is, makes the asks at character select (the list, CREATE,
// DELETE, RESET HOME, PLAY) and sends what's typed in the chat box once
// the character is in the world, and tells the screens through its
// events.  PLAY fetches the simple overworld map first (protocol version
// 11): the loading bar fills while it comes, it's checked against the
// server's SHA-256 and kept in the player's folder, written over every
// time.  Then the chunks around the character (version 12) go into the
// Ground as they come, and once the nearest 99 are in PlayerReady puts
// the character in the world; the rest keep coming after.  A map that
// can't be had sends the player back to the launcher, told to delete the
// file (or the game) and try again; chunks that stop coming send them back
// too.  Main thread only: the connection's threads reach it through
// MainThread.Post, and every message from a connection that's already
// been dropped is ignored.
//
// There's no reconnect.  However the session ends (a Kicked, the server
// going quiet, LOG OUT, /camp), the player is gone from the server, and
// the game goes back to the launcher: it starts Soundcheck again and
// closes, so the player is looking at the login.  A game that wasn't
// started by the launcher (the editor, or one started by hand) stays
// open on the start screen, which says why.

using System;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using Opus.World;
using UnityEngine;

namespace Opus.Net
{
    public enum SessionStage
    {
        LoggedOut,
        Connecting,
        AtCharacterSelect,
        LoadingWorld,
        InWorld,
    }

    public static class Session
    {
        public static SessionStage Stage { get; private set; }

        public static bool Busy
        {
            get { return Stage != SessionStage.LoggedOut; }
        }

        // What the start screen says: what's happening before character
        // select ("Joining the world..."), or why the last session ended,
        // and whether it's something gone wrong.  Null when there's nothing
        // to say yet.
        public static string Notice { get; private set; }
        public static bool NoticeTrouble { get; private set; }

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

        // The map at PLAY, while it comes: how many bytes are in, of how
        // many.  0 of 0 before the offer.
        public static long MapReceived { get; private set; }
        public static long MapSize { get; private set; }

        // Then the nearest chunks: how many are in (or refused for good),
        // of how many PlayerReady waits on.  0 of 0 before the map is in.
        public static int GroundHave { get; private set; }
        public static int GroundNeed { get; private set; }

        // Notice changed: the start screen's card redraws.
        public static event Action NoticeChanged;

        // The UDP side said Welcome: the player is at character select.
        public static event Action ReachedCharacterSelect;

        // Anything character select shows changed: the list, its trouble,
        // an ask started or answered, the status line, the world.
        public static event Action CharacterSelectChanged;

        // An ask was answered (its packet type, and whether the server did
        // it), rung before CharacterSelectChanged.  The cards close on a yes.
        public static event Action<byte, bool> AskAnswered;

        // The session is over and the game is staying open: why, and
        // whether it's something gone wrong.  Not rung when the game goes
        // back to the launcher instead.
        public static event Action<string, bool> SessionOver;

        // More of the map, or of the nearest chunks, came in: the loading
        // bar fills.
        public static event Action LoadingProgressed;

        // PlayerReady's answer came: the character is in the world, and the
        // HUD takes over from character select.
        public static event Action ReachedWorld;

        // A line for the chat box that isn't the player's own: the chat,
        // a command refused and why, the server not answering.
        public static event Action<string> ChatLine;

        // /who's answer, for the chat box to draw.
        public static event Action<WhoAnswer> WhoAnswered;

        static GameConnection game;

        // The name of the character RESET HOME was asked for, for its
        // answer (a CommandAccepted, which carries no name).
        static string sentHome;

        // The map's SHA-256, as the offer said it.
        static string mapHash;

        // ---------------------------------------------------------------
        // In: the ticket
        // ---------------------------------------------------------------

        // The ticket from the launcher (or, in the editor, from Soundcheck's
        // debug file): straight to the server's UDP port, and character
        // select once it says Welcome.  False if it couldn't start (the
        // start screen says why).
        public static bool Enter(Ticket ticket)
        {
            if (Busy)
                return false;

            ForgetCharacterSelect();
            try
            {
                game = GameConnection.Start(ticket.Host, ticket.UdpPort, ticket.Token);
            }
            catch (Exception e)
            {
                Debug.LogWarning("Game: couldn't open UDP to " + ticket.Host + ":" + ticket.UdpPort + " ("
                                 + e.Message + ").");
                Finish("Couldn't reach the server's game port.", true);
                return false;
            }
            Stage = SessionStage.Connecting;
            Tell("Joining the world...", false);
            return true;
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

        // /camp, any capitals, logs out, back to the launcher; /camp desktop
        // logs out and closes the game without it (Jacob, 2026-10-02).
        // True when the line was a /camp, whatever came after it.
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
                Debug.Log("Game: camped to the launcher.");
                LogOut();
            }
            else if (string.Equals(rest, "desktop", StringComparison.OrdinalIgnoreCase))
            {
                Debug.Log("Game: camped to the desktop.");
                CloseTheGame();
            }
            else
            {
                Chat("Try /camp, or /camp desktop.");
            }
            return true;
        }

        // ---------------------------------------------------------------
        // Out
        // ---------------------------------------------------------------

        // LOG OUT, or /camp: a Goodbye to the server, and back to the
        // launcher.
        public static void LogOut()
        {
            if (!Busy)
                return;
            Debug.Log("Game: logged out.");
            Drop();
            Finish("Logged out.", false);
        }

        // QUIT on the start screen, or /camp desktop: a Goodbye to the
        // server if there's a session, and the game closes without going
        // back to the launcher.
        public static void CloseTheGame()
        {
            Drop();
            Stage = SessionStage.LoggedOut;
            StopRunning();
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
        }

        // The program ending.  In the editor Application.Quit() does
        // nothing, so Play mode stops instead.
        static void StopRunning()
        {
#if UNITY_EDITOR
            UnityEditor.EditorApplication.isPlaying = false;
#else
            Application.Quit();
#endif
        }

        // Starts Soundcheck again, from the path it left in the environment
        // (Ticket.cs), so a player who's out is looking at the login, with
        // why the session ended in the launcher's status box (the same
        // way, in its environment).  The used-up ticket doesn't go along.
        // False when the game wasn't started by the launcher, or the
        // launcher can't be started (the log says why); never in the
        // editor, where there's no launcher to go back to.
        static bool BackToTheLauncher(string why, bool trouble)
        {
#if UNITY_EDITOR
            return false;
#else
            string path = Ticket.LauncherPath();
            if (path == null)
                return false;
            try
            {
                var start = new System.Diagnostics.ProcessStartInfo(path);
                start.UseShellExecute = false;
                start.WorkingDirectory = System.IO.Path.GetDirectoryName(path);
                start.EnvironmentVariables.Remove(Ticket.ServerVariable);
                start.EnvironmentVariables.Remove(Ticket.UdpPortVariable);
                start.EnvironmentVariables.Remove(Ticket.TokenVariable);
                start.EnvironmentVariables.Remove(Ticket.LauncherVariable);
                start.EnvironmentVariables[Ticket.SessionOverVariable] = why ?? "";
                start.EnvironmentVariables[Ticket.SessionTroubleVariable] = trouble ? "1" : "0";
                System.Diagnostics.Process.Start(start);
                Debug.Log("Game: back to the launcher, " + path + ".");
                return true;
            }
            catch (Exception e)
            {
                Debug.LogError("Game: couldn't start the launcher, " + path + " (" + e.Message + ").");
                return false;
            }
#endif
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
            if (kind == Protocol.PlayerReady)
            {
                MapTrouble("the server said \"" + why + "\"");
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
            if (kind == Protocol.PlayerReady)
            {
                MapTrouble("the server didn't answer PlayerReady");
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

        // PLAY's answer (version 11): the character is loaded, and the map
        // is coming.  Character select's buttons go, and the loading bar
        // shows over the list.
        internal static void MapOffered(GameConnection from, string hash, uint size)
        {
            if (from != game)
                return;
            Asking = 0;
            Stage = SessionStage.LoadingWorld;
            mapHash = hash;
            MapSize = size;
            MapReceived = 0;
            SayHere("", false);
        }

        internal static void MapProgress(GameConnection from, long received, uint size)
        {
            if (from != game || Stage != SessionStage.LoadingWorld)
                return;
            MapReceived = received;
            MapSize = size;
            if (LoadingProgressed != null)
                LoadingProgressed();
        }

        // Every piece is in.  The map is checked against the offer's hash,
        // written over the player's copy, and read once to be sure of it;
        // then the chunks around the character are asked for.  Here on the
        // main thread: a hash, a write and a read of a few MB, once a PLAY,
        // behind a full loading bar.
        internal static void MapArrived(GameConnection from, byte[] bytes)
        {
            if (from != game || Stage != SessionStage.LoadingWorld)
                return;
            string hash = Sha256Hex(bytes);
            if (hash != mapHash)
            {
                MapTrouble("what came has SHA-256 " + hash + ", and the server said " + mapHash);
                return;
            }

            SimpleOverworldMap map;
            try
            {
                map = SimpleOverworldMap.FromBytes(bytes);
                KeepMap(bytes);
            }
            catch (Exception e)
            {
                MapTrouble(e.Message);
                return;
            }
            SimpleOverworldMap.Current = map;
            Debug.Log("Game: the world's map is kept, " + map.Width + " by " + map.Depth + " patches, in "
                      + MapPath() + ".");

            MapReceived = bytes.Length;
            Ground.Clear();
            GroundHave = 0;
            GroundNeed = game.FetchChunks();
            if (LoadingProgressed != null)
                LoadingProgressed();
        }

        // The map couldn't be had: no new piece in the wait, or an offer
        // that didn't add up.
        internal static void MapFailed(GameConnection from, string why)
        {
            if (from != game)
                return;
            MapTrouble(why);
        }

        // A chunk, unsqueezed, for the Ground.  They keep coming once the
        // character is in the world.
        internal static void ChunkArrived(GameConnection from, Chunk chunk)
        {
            if (from != game)
                return;
            Ground.Put(chunk);
        }

        internal static void GroundProgress(GameConnection from, int have, int need)
        {
            if (from != game || Stage != SessionStage.LoadingWorld)
                return;
            GroundHave = have;
            GroundNeed = need;
            if (LoadingProgressed != null)
                LoadingProgressed();
        }

        // The nearest chunks are all in (or refused for good): PlayerReady,
        // with the map's hash, puts the character in the world.  The rest
        // keep coming.
        internal static void NearGroundIn(GameConnection from)
        {
            if (from != game || Stage != SessionStage.LoadingWorld || Asking == Protocol.PlayerReady)
                return;
            Debug.Log("Game: the nearest " + GroundNeed + " chunks are in.  PlayerReady.");
            Asking = Protocol.PlayerReady;
            game.Ask(Protocol.PlayerReady, mapHash);
            Changed();
        }

        // Every chunk in the view is in, or refused for good: the summary
        // in the Console, with what the Ground holds.
        internal static void GroundAllIn(GameConnection from, string summary)
        {
            if (from != game)
                return;
            int asBlocks = Ground.HeldAsBlocks();
            ushort? gold = Ground.BlockAt(0, 0, 0);
            Debug.Log(summary + "  Held: " + asBlocks + " as blocks ("
                      + (asBlocks * (double)Chunk.BlockCount * 2 / (1024 * 1024)).ToString("0.0") + " MB), "
                      + (Ground.Count - asBlocks) + " all one kind.  The block at 0,0,0: "
                      + (gold.HasValue ? Blocks.NameOf(gold.Value) : "not here") + ".");
        }

        // The chunks stopped coming, while loading or in the world.
        internal static void GroundFailed(GameConnection from, string why)
        {
            if (from != game)
                return;
            Debug.LogWarning("Game: couldn't get the ground around the character: " + why + ".");
            Drop();
            Finish("Couldn't get the ground around you.", true);
        }

        // PlayerReady's answer: the character is in the world.  Character
        // select is behind the player now; the way out is LOG OUT, to the
        // launcher.
        internal static void EnteredWorld(GameConnection from, string name)
        {
            if (from != game)
                return;
            Asking = 0;
            Stage = SessionStage.InWorld;
            InWorldAs = name;
            SayHere("", false);
            Answered(Protocol.PlayerReady, true);
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

        // The start screen's line.
        static void Tell(string words, bool trouble)
        {
            Notice = words;
            NoticeTrouble = trouble;
            if (NoticeChanged != null)
                NoticeChanged();
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
            MapReceived = 0;
            MapSize = 0;
            mapHash = null;
            GroundHave = 0;
            GroundNeed = 0;
            Ground.Clear();
        }

        // ---------------------------------------------------------------
        // The map at PLAY
        // ---------------------------------------------------------------

        // Where the map is kept: the player's folder, simple_overworld.map.
        static string MapPath()
        {
            return PlayerFiles.PathOf(SimpleOverworldMap.FileName);
        }

        // Writes the map over the player's copy: to a file beside it first,
        // then moved into its place, so a game that dies part way never
        // leaves half a map under the real name.
        static void KeepMap(byte[] bytes)
        {
            string path = MapPath();
            string part = path + ".part";
            Directory.CreateDirectory(PlayerFiles.Folder);
            File.WriteAllBytes(part, bytes);
            if (File.Exists(path))
                File.Delete(path);
            File.Move(part, path);
        }

        static string Sha256Hex(byte[] bytes)
        {
            byte[] hash;
            using (SHA256 sha = SHA256.Create())
                hash = sha.ComputeHash(bytes);
            var hex = new StringBuilder(64);
            foreach (byte b in hash)
                hex.Append(b.ToString("x2"));
            return hex.ToString();
        }

        // The player couldn't get the map.  The session ends, and they're
        // told to delete the file, or the game, and try again (Jacob,
        // 2026-10-03: "notify the person playing the game to delete the
        // local map file or client and try again"), back at the launcher.
        // The why goes in the log, not on the screen.
        static void MapTrouble(string why)
        {
            Debug.LogWarning("Game: couldn't get the world's map: " + why + ".");
            Drop();
            Finish("Couldn't get the world's map. Delete " + MapPath() + " (or reinstall the game) and try again.",
                   true);
        }

        // The session is over, however it ended.  A game the launcher
        // started goes back to it and closes; any other stays open on the
        // start screen, which says why.
        static void Finish(string why, bool trouble)
        {
            Stage = SessionStage.LoggedOut;
            ForgetCharacterSelect();
            Tell(why, trouble);
            if (BackToTheLauncher(why, trouble))
            {
                StopRunning();
                return;
            }
            if (SessionOver != null)
                SessionOver(why, trouble);
        }
    }
}
