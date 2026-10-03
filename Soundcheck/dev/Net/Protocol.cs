// File:       Opus/Soundcheck/dev/Net/Protocol.cs
// Component:  Soundcheck
// Author:     Jacob Chacko
// The numbers both sides agree on: the protocol's version, the packet
// types, the answers inside them.  Documentation/LLM/PROTOCOL.md is the
// contract, byte for byte; when this file and that one disagree, this file
// is what gets fixed.  Soundcheck only ever speaks the login's half (0x1_);
// the rest is listed so a packet that shouldn't come can be named.

namespace Opus.Net
{
    public static class Protocol
    {
        // The version this client speaks.  The server says its own in the
        // Hello, and a client built against another stops right there.
        public const byte Version = 13;

        // What a client has to say with its login.  Not a secret from
        // anybody with a copy of the client; it turns port scanners away
        // before they cost the server a hash.  networking.cfg's secret_word.
        public const string SecretWord = "potato";

        // The biggest TCP frame either side takes.
        public const int LargestFrame = 4096;

        // ---------------------------------------------------------------
        // Packet types.  The high four bits are the group: 0x1_ the login
        // over TCP, 0x2_ character select over UDP, 0x3_ the game over UDP,
        // 0x4_ the ground over UDP.
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
        public const byte ChunkRequest = 0x43;
        public const byte ChunkPiece = 0x44;
        public const byte ChunkRefused = 0x45;

        // ---------------------------------------------------------------
        // What's inside them
        // ---------------------------------------------------------------

        // LoginResult's answer.
        public const byte LoginFailed = 1;
        public const byte LoginAlreadyLoggedIn = 2;
        public const byte LoginOutdated = 3;
        public const byte LoginUnavailable = 4;

        // SessionChoice, when the account is already in the world.
        public const byte LogTheOtherOut = 0;
        public const byte HangUp = 1;

        // A packet type as a word, for the log.
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
                case ChunkRequest: return "ChunkRequest";
                case ChunkPiece: return "ChunkPiece";
                case ChunkRefused: return "ChunkRefused";
                default: return "0x" + kind.ToString("X2");
            }
        }
    }
}
