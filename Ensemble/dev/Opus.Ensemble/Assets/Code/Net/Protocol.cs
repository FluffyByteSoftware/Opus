// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Code/Net/Protocol.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// The numbers both sides agree on: the protocol's version, the packet
// types, the answers inside them.  Documentation/LLM/PROTOCOL.md is the
// contract, byte for byte; when this file and that one disagree, this file
// is what gets fixed.

namespace Opus.Net
{
    public static class Protocol
    {
        // The version this client speaks.  The server says its own in the
        // Hello, and a client built against another stops right there.
        public const byte Version = 11;

        // The biggest UDP packet the server takes.  (The TCP frame's cap is
        // the launcher's business.)
        public const int LargestDatagram = 1200;

        // The most pieces of the map one OverworldMapRequest may ask for.
        public const int MapPiecesAtOnce = 64;

        // ---------------------------------------------------------------
        // Packet types.  The high four bits are the group: 0x1_ the login
        // over TCP, 0x2_ character select over UDP, 0x3_ the game over UDP,
        // 0x4_ the ground over UDP (the simple overworld map at PLAY).
        // The login is the launcher's (Soundcheck), and the game never
        // sends or reads a 0x1_ packet; they're listed so this file is the
        // whole table, as PROTOCOL.md has it.
        // ---------------------------------------------------------------

        public const byte Hello = 0x10;
        public const byte Login = 0x11;
        public const byte InLine = 0x12;
        public const byte LoginResult = 0x13;
        public const byte SessionChoice = 0x14;
        public const byte Ticket = 0x15;

        public const byte CharacterListRequest = 0x20;
        public const byte CharacterListDelivery = 0x21;
        public const byte CreateCharacter = 0x22;
        public const byte CharacterCreateResult = 0x23;
        public const byte DeleteCharacter = 0x24;
        public const byte CharacterDeleteResult = 0x25;
        public const byte CharacterRequestResetHome = 0x26;
        public const byte UserPressPlay = 0x27;
        public const byte CharacterEnteredWorld = 0x28;
        public const byte PlayerReady = 0x29;

        public const byte Connect = 0x30;
        public const byte ConnectResult = 0x31;
        public const byte KeepAlive = 0x32;
        public const byte Goodbye = 0x33;
        public const byte Kicked = 0x34;
        public const byte CommandAccepted = 0x35;
        public const byte CommandRefused = 0x36;
        public const byte PlayerCommand = 0x37;
        public const byte ChatDelivery = 0x38;
        public const byte WhoDelivery = 0x39;
        public const byte Span = 0x3A;
        public const byte PleaseWait = 0x3B;

        public const byte OverworldMapOffer = 0x40;
        public const byte OverworldMapRequest = 0x41;
        public const byte OverworldMapPiece = 0x42;

        // ---------------------------------------------------------------
        // What's inside them
        // ---------------------------------------------------------------

        // ConnectResult's answer.
        public const byte Welcome = 0;

        // Every answer that carries an ask number first, so it can be
        // matched to the ask it's for.  A Span carries one too, but it's a
        // piece of an answer, not an answer.
        public static bool CarriesAsk(byte kind)
        {
            return kind == CharacterListDelivery || kind == CharacterCreateResult || kind == CharacterDeleteResult
                || kind == CharacterEnteredWorld || kind == CommandAccepted || kind == CommandRefused
                || kind == WhoDelivery || kind == OverworldMapOffer;
        }

        // What the player is told for each Kicked reason.  The server sends
        // only the number, so the words are the client's.
        public static string KickedSays(uint reason)
        {
            switch (reason)
            {
                case 1: return "Your account logged in from somewhere else.";
                case 2: return "The server is stopping.";
                case 3: return "You've been banned from this server.";
                case 4: return "You were kicked by the admin.";
                case 5: return "ACCOUNT TERMINATED";
                case 6: return "That character is locked for a moment. Log in again.";
                default: return "The server ended the session (reason " + reason + ").";
            }
        }

        // A packet type as a word, for the Console.
        public static string NameOf(byte kind)
        {
            switch (kind)
            {
                case Hello: return "Hello";
                case Login: return "Login";
                case InLine: return "InLine";
                case LoginResult: return "LoginResult";
                case SessionChoice: return "SessionChoice";
                case Ticket: return "Ticket";
                case CharacterListRequest: return "CharacterListRequest";
                case CharacterListDelivery: return "CharacterListDelivery";
                case CreateCharacter: return "CreateCharacter";
                case CharacterCreateResult: return "CharacterCreateResult";
                case DeleteCharacter: return "DeleteCharacter";
                case CharacterDeleteResult: return "CharacterDeleteResult";
                case CharacterRequestResetHome: return "CharacterRequestResetHome";
                case UserPressPlay: return "UserPressPlay";
                case CharacterEnteredWorld: return "CharacterEnteredWorld";
                case PlayerReady: return "PlayerReady";
                case Connect: return "Connect";
                case ConnectResult: return "ConnectResult";
                case KeepAlive: return "KeepAlive";
                case Goodbye: return "Goodbye";
                case Kicked: return "Kicked";
                case CommandAccepted: return "CommandAccepted";
                case CommandRefused: return "CommandRefused";
                case PlayerCommand: return "PlayerCommand";
                case ChatDelivery: return "ChatDelivery";
                case WhoDelivery: return "WhoDelivery";
                case Span: return "Span";
                case PleaseWait: return "PleaseWait";
                case OverworldMapOffer: return "OverworldMapOffer";
                case OverworldMapRequest: return "OverworldMapRequest";
                case OverworldMapPiece: return "OverworldMapPiece";
                default: return "0x" + kind.ToString("X2");
            }
        }
    }
}
