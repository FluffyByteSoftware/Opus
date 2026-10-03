//! File:       Opus/Conductor/dev/networking/src/protocol.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The packets Conductor and its clients agree on, down to the byte.  This
//! is the Rust half; `Documentation/LLM/PROTOCOL.md` is what both halves
//! are written from, and when a half disagrees with the document, the half
//! is what gets fixed.
//!
//! Every TCP packet goes in the same frame:
//!
//! ```text
//! [length: u32][type: u8][payload]
//! ```
//!
//! The length counts the type byte and the payload, not itself.  TCP is
//! one long stream of bytes with no gaps in it, so without the length the
//! reader couldn't tell where one packet stops and the next starts.
//!
//! UDP keeps every packet whole and apart from the next, so a UDP packet
//! carries no length, just the type and the payload:
//!
//! ```text
//! [type: u8][payload]
//! ```
//!
//! Numbers are little-endian (lowest byte first), because that's what C#'s
//! BinaryWriter and BinaryReader do, and the client, Ensemble, is C#.
//! A string is a u32 byte count and then that many bytes of UTF-8.
//!
//! Nothing in here logs or touches the network.  It turns packets into
//! bytes and bytes back into packets, which is what lets the tests run.
//! The answers that never change (`hello()`, `keep_alive()`, the results)
//! are built once by the callers and sent as they are.
//!
//! Character select (0x2_, over UDP, version 5) and the two general
//! answers, CommandAccepted and CommandRefused, carry an ask number: a u32
//! the client picks, one higher for every new ask.  The same number again
//! means the client didn't hear the answer, and it gets the one it missed
//! rather than the ask done twice (a second CreateCharacter would find
//! its own name taken).
//!
//! Version 6 (2026-10-01) is the spawn: UserPressPlay picks a character
//! at character select, and CharacterEnteredWorld says it's in the world
//! and where.  Jacob's names.
//!
//! Version 7 (2026-10-02) is the password's key: the Login's fourth string
//! is no longer the password as typed but the key the client makes from
//! it (Security's `password_key()` has the recipe), 64 lowercase hex.
//!
//! Version 8 (2026-10-02) is chat: PlayerCommand carries a line the
//! player typed, as it was typed (`/chat Yo yo yo!`), and ChatDelivery
//! carries the finished lines out to everybody in the world, once a
//! GameClock cycle.
//!
//! Version 9 (2026-10-02) is `/who` and the span.  WhoDelivery answers a
//! `/who` with the names of the characters in the world (and with `/who
//! list`, where each stands), and the time it ran; the client draws the
//! rest.  Span carries an answer too big for one packet in pieces, each
//! saying which piece it is, and the client puts them back together.
//!
//! Version 10 (2026-10-02) is PleaseWait: the ask will take a moment, and
//! these are the words to show meanwhile.
//!
//! Version 11 (2026-10-03) is the simple overworld map at PLAY, the new
//! group 0x4_, the ground.  UserPressPlay is answered with an
//! OverworldMapOffer (the map's size, its pieces and its SHA-256) instead
//! of the character going straight into the world; the client asks for
//! the pieces with OverworldMapRequests and gets OverworldMapPieces; then
//! PlayerReady, with the hash of what it got, puts the character in the
//! world, answered with the CharacterEnteredWorld that used to answer
//! UserPressPlay.  Jacob: "it doesn't show them or spawn them in the
//! physical world until they're ready".
//!
//! Version 12 (2026-10-03) is the chunks around the player, pulled by the
//! client (Jacob: "Pull"): ChunkRequest asks for up to 64 chunks by place,
//! and each comes back squeezed as ChunkPieces, or as a ChunkRefused
//! saying why not.  The OverworldMapOffer now says where the character
//! will stand and how far it sees (`view_chunks`), so the client knows
//! which chunks to ask for before PlayerReady.
//!
//! Version 13 (2026-10-03) is `/who` one line a character, EverQuest's
//! way: WhoDelivery loses its list byte (`/who list` is gone into `/who`),
//! and every character carries where it stands and its seconds online,
//! the one in the world longest first.
//!
//! Version 14 (2026-10-03) is the world's objects, the new group 0x5_:
//! what a player sees of the world around them, the server deciding
//! where everything is and the client only drawing it (Jacob: "The
//! client is just a dumb renderer").  A Hydrate sends an object whole
//! when it comes into view, ObjectsMoved where known ones are now,
//! ObjectsGone the ones that left, and a RollCall once a second
//! everything the player is believed to know; the client asks about a
//! number it doesn't know with an ObjectAsk.  CharacterEnteredWorld now
//! ends with the number of the player's own character.
//!
//! Version 15 (2026-10-03) is movement, EverQuest's way: a player's client
//! walks its own character and says where it went with a PlayerMoved
//! (0x55), and the server takes each move or pulls the character back to
//! its last good spot with a MoveCorrection (0x56).  CharacterEnteredWorld
//! ends with how fast a character walks and turns, the Hydrate carries
//! the room an object takes up (its collider), and a player is no longer
//! sent their own character's moves.

use conductor_gameclock::{Hydrate, Motion, PullBack};
use conductor_gameworld::ChunkPos;

/// Which protocol this is.  The Hello says it, so a client built against
/// a different one can stop right there.  Goes up when a packet changes.
pub const PROTOCOL_VERSION: u8 = 15;

/// The biggest length a TCP frame may claim.  Plenty for a login, and it
/// stops somebody claiming a 4 GB packet and making us wait for it.
pub const MAX_PACKET_BYTES: usize = 4096;

/// The biggest UDP packet we take.  Anything much over 1200 bytes risks
/// being split up somewhere along the internet, and a lost piece loses
/// the whole packet.  So ours stay under it, and one that isn't is
/// dropped.
pub const MAX_UDP_BYTES: usize = 1200;

/// How long a ticket's token is: 32 random bytes as 64 hex characters,
/// from Fingerprinter.  A Connect whose token isn't this long is junk.
pub const TOKEN_LENGTH: usize = 64;

/// How much of the simple overworld map one OverworldMapPiece carries,
/// the last piece less (version 11).  With the piece's type and number in
/// front it's 1,029 bytes, under MAX_UDP_BYTES.  4,097 pieces at
/// `world_size` 16, 16,385 at 32.
pub const MAP_PIECE_BYTES: usize = 1024;

/// The most pieces one OverworldMapRequest may ask for.  One more than
/// this is a request we can't read.  64 pieces is 64 KB, which a client's
/// receive buffer holds with room to spare.
pub const MAP_PIECES_AT_ONCE: u8 = 64;

/// The most chunks one ChunkRequest may ask for (version 12).  Five bytes
/// a chunk, so a full one is 322 bytes.
pub const CHUNKS_AT_ONCE: u8 = 64;

/// What goes in front of a ChunkPiece's bytes: its type, the chunk's x and
/// z (i16 each), its row, the piece's number and how many pieces.
const CHUNK_PIECE_HEADER: usize = 1 + 2 + 2 + 1 + 1 + 1;

/// How much of a squeezed chunk one ChunkPiece carries, the last piece
/// less: whatever MAX_UDP_BYTES leaves, 1,192.  Most chunks are one piece.
pub const CHUNK_PIECE_BYTES: usize = MAX_UDP_BYTES - CHUNK_PIECE_HEADER;

/// The bytes one object's motion takes (version 14): its number, then
/// nine f32s, its position, rotation and velocity.
const MOTION_BYTES: usize = 4 + 9 * 4;

/// The most motions one ObjectsMoved carries: its type and count, then as
/// many as fit under MAX_UDP_BYTES, 29.
pub const MOVED_AT_ONCE: usize = (MAX_UDP_BYTES - 2) / MOTION_BYTES;

/// The most numbers one ObjectsGone carries: its type and a u16 count,
/// then as many as fit, 299.
pub const GONE_AT_ONCE: usize = (MAX_UDP_BYTES - 3) / 4;

/// What goes in front of a RollCall's motions: its type, the roll's
/// number, which piece and how many, and the count.
const ROLL_CALL_HEADER: usize = 1 + 4 + 1 + 1 + 1;

/// The most motions one piece of a RollCall carries, 29.
pub const ROLL_CALL_AT_ONCE: usize = (MAX_UDP_BYTES - ROLL_CALL_HEADER) / MOTION_BYTES;

/// The most numbers one ObjectAsk may carry.
pub const ASKS_AT_ONCE: u8 = 64;

/// A PlayerMoved's payload (version 15): the move's number, the last
/// pull-back the client had, then nine f32s, its position, rotation and
/// velocity.
const PLAYER_MOVED_BYTES: usize = 4 + 4 + 9 * 4;

/// Every packet type there is.  The high four bits say the group and the
/// low four which one in it: 0x1_ is the login, over TCP; 0x2_ is
/// character select, between the login and the world, over UDP (version
/// 5, 2026-09-30); 0x3_ is the game, over UDP; and 0x4_ is the ground,
/// over UDP (version 11, 2026-10-03), the simple overworld map and, since
/// version 12, the chunks; and 0x5_ is the world's objects, over UDP
/// (version 14, 2026-10-03), and movement since version 15.
// Rust note: `repr(u8)` stores the enum as one byte, and `as u8` turns a
// value back into its number, the same as a C# `enum : byte`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketType {
    /// Server to client, right after TLS.  One byte: PROTOCOL_VERSION.
    Hello = 0x10,
    /// Client to server.  Four strings: the client's version, the secret
    /// word, the username, the password's key.
    Login = 0x11,
    /// Server to client, once a second while the login waits in
    /// Security's line.  A u32 for how many are ahead, and a u32 for
    /// about how many milliseconds that is.
    InLine = 0x12,
    /// Server to client, when the login didn't end in a ticket.  A
    /// LoginAnswer byte, then a string for the player.
    LoginResult = 0x13,
    /// Client to server, after a LoginResult of "already logged in".  One
    /// byte: 0 logs the other session out, 1 hangs this one up and leaves
    /// the other alone.  (So a shared account doesn't kick your brother
    /// off because you wanted to play.)
    SessionChoice = 0x14,
    /// Server to client: the login worked.  The token (a string), then
    /// the UDP port to take it to (a u16).  The server closes the
    /// connection right after.
    Ticket = 0x15,
    /// Client to server, over UDP, at character select.  The ask number.
    /// The server answers with a CharacterListDelivery.  Version 5.
    CharacterListRequest = 0x20,
    /// Server to client.  The ask number, a u8 count, then for each of the
    /// account's characters, in slot order: its uuid and name (strings),
    /// its slot (u8, 1 to 3), and whether it can be played (u8, 1 yes, 0
    /// no; the client greys it out).  Only the account's own characters.
    CharacterListDelivery = 0x21,
    /// Client to server.  The ask number, then the name (a string).
    CreateCharacter = 0x22,
    /// Server to client.  The ask number, a CreateAnswer byte, then a
    /// string for the player.
    CharacterCreateResult = 0x23,
    /// Client to server.  The ask number, the character's uuid, then the
    /// word the player typed to say they mean it (strings).  Only DELETE
    /// deletes.
    DeleteCharacter = 0x24,
    /// Server to client.  The ask number, a DeleteAnswer byte, then a
    /// string for the player.
    CharacterDeleteResult = 0x25,
    /// Client to server.  The ask number, then the character's uuid.  Puts
    /// the character back at 0, 0, 0.  Answered with a CommandAccepted or
    /// a CommandRefused.
    CharacterRequestResetHome = 0x26,
    /// Client to server.  The ask number, then the uuid of the character
    /// to play.  Answered with a CharacterEnteredWorld, or a
    /// CommandRefused saying why not.  Version 6, Jacob's name.
    UserPressPlay = 0x27,
    /// Server to client: the character is in the world.  The ask number,
    /// its uuid and name (strings), then where it stands, x, y and z, an
    /// f32 each, y up, then the number its Hydrate and the rest of the
    /// world's objects know it by (u32, version 14).  Version 6.  Since
    /// version 11 it answers PlayerReady, not UserPressPlay.
    CharacterEnteredWorld = 0x28,
    /// Client to server: the simple overworld map is in and good to
    /// draw.  The ask number, then the map's SHA-256 as the client has it
    /// (a string, 64 lowercase hex).  Answered with a
    /// CharacterEnteredWorld, or a CommandRefused saying why not.  Version
    /// 11, Jacob's name: "verification from client it streamed the
    /// terrain and is good to display".
    PlayerReady = 0x29,
    /// Client to server, over UDP.  One string: the token from the
    /// Ticket.  The first UDP packet a client sends, and it sends it again
    /// every half second until it hears back.
    Connect = 0x30,
    /// Server to client, over UDP.  A ConnectAnswer byte, then a string
    /// for the player.
    ConnectResult = 0x31,
    /// Both ways, over UDP, no payload.  The client sends one every second
    /// while it's in the world, and the server sends one straight back, so
    /// each side knows the other is still there.
    KeepAlive = 0x32,
    /// Client to server, over UDP, no payload.  The player is logging out.
    /// No answer; the session is gone.
    Goodbye = 0x33,
    /// Server to client, over UDP.  A KickReason as a u32.  The session is
    /// gone, and the client goes back to the login screen.
    Kicked = 0x34,
    /// Server to client, over UDP.  The ask number: the command was done.
    /// A general answer, for any command that needs no more said (Jacob,
    /// 2026-09-30: "This can be reused elsewhere").  Version 5.
    CommandAccepted = 0x35,
    /// Server to client, over UDP.  The ask number, then a string saying
    /// why the command wasn't done, for the player.  Version 5.
    CommandRefused = 0x36,
    /// Client to server, over UDP.  The ask number, then a line the
    /// player typed, as it was typed (a string).  One starting with `/`
    /// is a command (`/chat Yo yo yo!`).  Answered with a CommandAccepted
    /// or a CommandRefused.  Version 8, 2026-10-02.
    PlayerCommand = 0x37,
    /// Server to client, over UDP.  A u8 count, then that many finished
    /// lines of chat (strings), the way the player sees them: `[Chat]
    /// Jacob: Yo yo yo!`.  Sent once a GameClock cycle that has chat, to
    /// everybody in the world, the one who said it too.  Version 8.
    ChatDelivery = 0x38,
    /// Server to client, the answer to a `/who`.  The ask number, the
    /// seconds since midnight UTC when it ran (a u32), and a u16 count,
    /// then for each character in the world, the one in longest first:
    /// its name (a string), its x, y and z in whole blocks (an i32 each)
    /// and its seconds online (a u32).  The client draws the lines.
    /// Version 9, one line a character since version 13.
    WhoDelivery = 0x39,
    /// Server to client: one piece of an answer too big for one packet.
    /// The ask number, which piece (u8, from 1), how many pieces (u8),
    /// then the piece's bytes.  The pieces' bytes, in order, make the
    /// answer, type byte and all.  An answer that fits in one packet goes
    /// as it is.  Version 9, Jacob's "span packet".
    Span = 0x3A,
    /// The ask is being worked on and will take a moment (version 10):
    /// the words to show while it is.  Not the answer: that follows under
    /// the same ask number.
    PleaseWait = 0x3B,
    /// Server to client, the answer to a UserPressPlay (version 11): the
    /// character is loaded and waiting, and here's the map to fetch
    /// first.  The ask number, the map's size in bytes (u32), how much a
    /// piece carries (u16, MAP_PIECE_BYTES), how many pieces (u32), its
    /// SHA-256 (a string, 64 lowercase hex), then where the character will
    /// stand, x, y and z (f32 each), and how many chunks it sees each way
    /// (u8, `view_chunks`; both version 12).
    OverworldMapOffer = 0x40,
    /// Client to server: send me these pieces of the map.  The first
    /// piece's number (u32, from 0), then how many (u8, 1 to
    /// MAP_PIECES_AT_ONCE).  No ask number: asking twice is harmless, so
    /// it doesn't take the player's one ask.  Only a player who's been
    /// offered the map gets an answer.  Version 11.
    OverworldMapRequest = 0x41,
    /// Server to client: one piece of the map.  Its number (u32), then its
    /// bytes, MAP_PIECE_BYTES of them but for the last.  Version 11.
    OverworldMapPiece = 0x42,
    /// Client to server: send me these chunks.  How many (u8, 1 to
    /// CHUNKS_AT_ONCE), then each one's place: x and z (i16, counted in
    /// chunks), and its row (u8, 0 to 10).  No ask number, like the
    /// map's: asking twice is harmless.  Only a player whose character is
    /// waiting on the map or in the world gets an answer.  Version 12.
    ChunkRequest = 0x43,
    /// Server to client: a piece of a squeezed chunk.  Its x, z and row,
    /// which piece (u8, from 1) and how many (u8), then the bytes,
    /// CHUNK_PIECE_BYTES of them but for the last.  The pieces' bytes put
    /// together in order are the chunk, squeezed.  Version 12.
    ChunkPiece = 0x44,
    /// Server to client: a chunk asked for that isn't sent.  Its x, z and
    /// row, then a ChunkRefusal byte saying why.  Version 12.
    ChunkRefused = 0x45,
    /// Server to client: an object in the player's view, whole, to draw
    /// (Jacob's word: the client hydrates an Actor or a plain object with
    /// it).  Its number (u32), its uuid (a string), whether it's Living
    /// (u8, 1 yes), its short name (a string, empty for none), then its
    /// motion's nine f32s (position, rotation in degrees, velocity), its
    /// scale (three f32s), its model's uuid (a string, empty for none),
    /// the shape to draw without one (u8: 0 cube, 1 sphere, 2 capsule, 3
    /// cylinder, 4 plane, 5 quad) and what it's doing (a string, empty
    /// for nothing).  Sent when it comes into view, and when the client
    /// asks about it.  Version 14.
    Hydrate = 0x50,
    /// Server to client: objects the player knows that moved.  A u8
    /// count, then each one's motion: its number (u32), position,
    /// rotation and velocity (f32 each).  Version 14.
    ObjectsMoved = 0x51,
    /// Server to client: objects gone from the player's view, or from the
    /// world.  A u16 count, then each one's number (u32).  Version 14.
    ObjectsGone = 0x52,
    /// Server to client, once a second: every object the player is
    /// believed to know, and where it is now.  The roll call's number
    /// (u32), which piece (u8, from 1) and how many (u8), a u8 count,
    /// then that many motions.  Once a roll call's pieces are all in, the
    /// client drops what isn't on it and asks about what it doesn't
    /// know.  Version 14.
    RollCall = 0x53,
    /// Client to server: tell me about these objects.  A u8 count (1 to
    /// ASKS_AT_ONCE), then each one's number (u32).  No ask number, like
    /// the chunks': asking twice is harmless.  Answered by the next
    /// broadcast, a Hydrate for each one in view and an ObjectsGone for
    /// the rest.  Version 14.
    ObjectAsk = 0x54,
    /// Client to server: where the player walked their own character.
    /// The move's number (u32, one higher each time), the number of the
    /// last MoveCorrection the client had (u32, 0 for none), then nine
    /// f32s: position, rotation in degrees and velocity in blocks a
    /// second.  Sent whenever which way it walks or faces changes, and
    /// every half second while it walks.  No answer unless it's pulled
    /// back.  Version 15.
    PlayerMoved = 0x55,
    /// Server to client: your character is pulled back to its last good
    /// spot, standing still.  The pull-back's number (u32, from 1), then
    /// six f32s, its position and rotation.  The client's moves after
    /// this carry its number.  Version 15.
    MoveCorrection = 0x56,
}

impl PacketType {
    /// The packet type for a byte off the wire, or `None` for a byte that
    /// isn't one.
    pub fn from_byte(byte: u8) -> Option<PacketType> {
        match byte {
            0x10 => Some(PacketType::Hello),
            0x11 => Some(PacketType::Login),
            0x12 => Some(PacketType::InLine),
            0x13 => Some(PacketType::LoginResult),
            0x14 => Some(PacketType::SessionChoice),
            0x15 => Some(PacketType::Ticket),
            0x20 => Some(PacketType::CharacterListRequest),
            0x21 => Some(PacketType::CharacterListDelivery),
            0x22 => Some(PacketType::CreateCharacter),
            0x23 => Some(PacketType::CharacterCreateResult),
            0x24 => Some(PacketType::DeleteCharacter),
            0x25 => Some(PacketType::CharacterDeleteResult),
            0x26 => Some(PacketType::CharacterRequestResetHome),
            0x27 => Some(PacketType::UserPressPlay),
            0x28 => Some(PacketType::CharacterEnteredWorld),
            0x29 => Some(PacketType::PlayerReady),
            0x30 => Some(PacketType::Connect),
            0x31 => Some(PacketType::ConnectResult),
            0x32 => Some(PacketType::KeepAlive),
            0x33 => Some(PacketType::Goodbye),
            0x34 => Some(PacketType::Kicked),
            0x35 => Some(PacketType::CommandAccepted),
            0x36 => Some(PacketType::CommandRefused),
            0x37 => Some(PacketType::PlayerCommand),
            0x38 => Some(PacketType::ChatDelivery),
            0x39 => Some(PacketType::WhoDelivery),
            0x3A => Some(PacketType::Span),
            0x3B => Some(PacketType::PleaseWait),
            0x40 => Some(PacketType::OverworldMapOffer),
            0x41 => Some(PacketType::OverworldMapRequest),
            0x42 => Some(PacketType::OverworldMapPiece),
            0x43 => Some(PacketType::ChunkRequest),
            0x44 => Some(PacketType::ChunkPiece),
            0x45 => Some(PacketType::ChunkRefused),
            0x50 => Some(PacketType::Hydrate),
            0x51 => Some(PacketType::ObjectsMoved),
            0x52 => Some(PacketType::ObjectsGone),
            0x53 => Some(PacketType::RollCall),
            0x54 => Some(PacketType::ObjectAsk),
            0x55 => Some(PacketType::PlayerMoved),
            0x56 => Some(PacketType::MoveCorrection),
            _ => None,
        }
    }
}

/// Why a login didn't end in a ticket: the first byte of a LoginResult.
/// A login that worked gets a Ticket instead, so there's no "success"
/// here.  One failure for a wrong secret word, a wrong name and a wrong
/// password alike, so all three look the same from outside.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginAnswer {
    Failed = 1,
    /// The password was right, and the account is already in the world.
    /// The client asks the player and answers with a SessionChoice.  Only
    /// ever sent after the right password, so it tells a stranger nothing.
    AlreadyLoggedIn = 2,
    /// The client's version isn't on the list.  Checked before the
    /// password, so it costs no hash.
    Outdated = 3,
    /// The server can't check logins right now: the database or Security
    /// is down, or Fingerprinter couldn't make a token.  Nothing the player
    /// did.
    Unavailable = 4,
}

impl LoginAnswer {
    /// What the player sees.  Failures they can act on are sentences;
    /// ones they can't are short labels, Jacob's rule.
    pub fn message(self) -> &'static str {
        match self {
            LoginAnswer::Failed => "Invalid Credentials",
            LoginAnswer::AlreadyLoggedIn => "This account is already logged in.",
            LoginAnswer::Outdated => "Outdated Client Failure",
            LoginAnswer::Unavailable => "Login Unavailable",
        }
    }
}

/// The server's answer to a Connect: the first byte of a ConnectResult.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectAnswer {
    /// The token is good.  The player is in.
    Accepted = 0,
    /// The token isn't one we handed out, it has expired, or it's already
    /// in use from somewhere else.  The same words as a bad login.
    Refused = 1,
}

impl ConnectAnswer {
    pub fn message(self) -> &'static str {
        match self {
            ConnectAnswer::Accepted => "Welcome to the world.",
            ConnectAnswer::Refused => "Invalid Credentials",
        }
    }
}

/// Why the server is dropping a player, which is the whole payload of a
/// Kicked.  A number, not a sentence, so the client can act on it without
/// matching our wording.  A player who went quiet gets no Kicked: they
/// wouldn't hear it.
// Rust note: `repr(u32)` stores this as four bytes, the same as a C#
// `enum : uint`.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KickReason {
    /// The account logged in from somewhere else and chose to log this
    /// session out.
    LoggedInElsewhere = 1,
    /// STOP SERVER on the web admin's Server tab.
    ServerStopping = 2,
    /// The admin changed the access lists and this address is no longer
    /// let in: put on the blacklist, or taken off the whitelist.  Version
    /// 2 of the protocol, 2026-09-29.
    Banned = 3,
    /// The admin kicked them from the web admin's Connections tab.
    /// Version 3 of the protocol, 2026-09-29.
    KickedByAdmin = 4,
    /// The admin deleted their account from the web admin's Accounts
    /// tab.  The client says ACCOUNT TERMINATED (Jacob's words).  Version
    /// 4 of the protocol, 2026-09-29.
    AccountTerminated = 5,
    /// They picked a character that's locked for a moment: it started
    /// loading or left the world less than a second ago, or its last save
    /// is still on its way to the database (Jacob: "a temporary 'load'
    /// lock on a character ... locked for 1 second and then released").
    /// Nothing is wrong: they log in again.  Version 6 of the protocol,
    /// 2026-10-01.
    CharacterLocked = 6,
}

/// Why a chunk asked for isn't sent: the last byte of a ChunkRefused.
/// Version 12.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkRefusal {
    /// Outside the player's view (`view_chunks` around where their
    /// character stands), or outside the world.  Asking again won't help.
    OutOfView = 1,
    /// Being read or built: ask again in a moment.
    NotYet = 2,
    /// Not to be had this run: its file doesn't read right, or the world
    /// couldn't be read.  Asking again won't help.
    Unavailable = 3,
}

/// What became of a CreateCharacter: the first byte of a
/// CharacterCreateResult.  Version 5.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreateAnswer {
    /// Made, in the account's first empty slot.
    Made = 0,
    /// The name breaks the rule: 4 to 20 letters, a to z, and only the
    /// first can be a capital.
    NameNotAllowed = 1,
    /// Another character has that name, whatever the capitals.
    NameTaken = 2,
    /// All three of the account's slots are full.  The client shouldn't
    /// offer it then (Jacob: "we refuse to even allow them to create"),
    /// and the server turns it away anyway.
    SlotsFull = 3,
    /// The server can't make one right now: the database is down, say.
    Unavailable = 4,
}

impl CreateAnswer {
    /// What the player sees.
    pub fn message(self) -> &'static str {
        match self {
            CreateAnswer::Made => "Your character has been made.",
            CreateAnswer::NameNotAllowed => "A character's name is 4 to 20 letters, a to z, and only the first \
                can be a capital.",
            CreateAnswer::NameTaken => "That name is taken.",
            CreateAnswer::SlotsFull => "All three character slots are full.",
            CreateAnswer::Unavailable => "Character Creation Unavailable",
        }
    }
}

/// What became of a DeleteCharacter: the first byte of a
/// CharacterDeleteResult.  The string after it says why a denial was one.
/// Version 5.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteAnswer {
    /// The word was DELETE, and the character is gone.
    Approved = 0,
    /// The word wasn't DELETE, there's no such character on the account,
    /// or the server couldn't do it right now.
    Denied = 1,
}

/// One character in a CharacterListDelivery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedCharacter {
    pub uuid: String,
    pub name: String,
    /// 1 to 3.
    pub slot: u8,
    /// False when its save wouldn't load this run: the client greys it
    /// out, and the server won't let it in.
    pub playable: bool,
}

/// A character that has come into the world, for a CharacterEnteredWorld.
#[derive(Clone, Debug, PartialEq)]
pub struct EnteredCharacter {
    pub uuid: String,
    pub name: String,
    /// Where it stands: x, y and z, y up.
    pub position: [f32; 3],
    /// The number its client knows it by among the world's objects
    /// (version 14), so the camera knows which one to follow.
    pub object: u32,
    /// How fast it walks, in blocks a second, and turns, in degrees a
    /// second (version 15): the server's numbers, which its moves are
    /// held to.
    pub walk: f32,
    pub turn: f32,
}

/// One character in a WhoDelivery: its name, the block it stands in, and
/// how long it's been in the world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhoEntry {
    pub name: String,
    /// x, y and z in whole blocks, y up.
    pub block: [i32; 3],
    /// Whole seconds since it came into the world.
    pub online: u32,
}

/// What the player picked when their account was already logged in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Log the other session out and carry on with this one.
    LogTheOtherOut,
    /// Leave the other session alone and hang this one up.
    HangUp,
}

/// One whole TCP packet, taken off the wire.  The type is kept as the raw
/// byte, so one we don't know can still be logged.
#[derive(Debug, PartialEq, Eq)]
pub struct Packet {
    pub kind: u8,
    pub payload: Vec<u8>,
}

/// What a Login says.  No `Debug` on purpose: it holds the password's
/// key, which logs in as well as the password does, and a `{:?}` in a log
/// line would print it.
pub struct LoginRequest {
    pub client_version: String,
    pub secret_word: String,
    pub username: String,
    /// The password's key, as the client made it: 64 lowercase hex if
    /// it's a real one.  Not checked here; the login does that.
    pub key: String,
}

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

/// A TCP packet as the bytes that go on the wire.
pub fn frame(kind: PacketType, payload: &[u8]) -> Vec<u8> {
    let length = (1 + payload.len()) as u32;
    let mut bytes = Vec::with_capacity(4 + length as usize);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.push(kind as u8);
    bytes.extend_from_slice(payload);
    bytes
}

/// Takes one whole packet off the front of `buffer`, if there is one.
/// `Ok(None)` means there isn't a whole one yet, and `buffer` is left alone
/// until more bytes arrive.  An `Err` means the frame is one we won't take,
/// and the connection should close.
///
/// The length is checked as soon as its four bytes are in, so somebody
/// claiming a huge packet is turned away before we wait for any of it.
pub fn take_packet(buffer: &mut Vec<u8>) -> Result<Option<Packet>, String> {
    if buffer.len() < 4 {
        return Ok(None);
    }
    let length = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
    if length == 0 {
        return Err("a packet with no type byte".to_string());
    }
    if length > MAX_PACKET_BYTES {
        return Err(format!("a packet of {length} bytes, and the most we take is {MAX_PACKET_BYTES}"));
    }
    if buffer.len() < 4 + length {
        return Ok(None);
    }

    let kind = buffer[4];
    let payload = buffer[5..4 + length].to_vec();
    // Rust note: `drain` takes those bytes out of the front of the Vec and
    // slides whatever is left down to the start.
    buffer.drain(..4 + length);
    Ok(Some(Packet { kind, payload }))
}

/// Splits a UDP packet into its type and its payload, borrowed from the
/// buffer it came in, so a keep-alive costs no allocation.  `None` means
/// the packet is empty or too big, and it gets dropped without an answer.
pub fn take_datagram(bytes: &[u8]) -> Option<(u8, &[u8])> {
    if bytes.is_empty() || bytes.len() > MAX_UDP_BYTES {
        return None;
    }
    Some((bytes[0], &bytes[1..]))
}

// ---------------------------------------------------------------------------
// Strings
// ---------------------------------------------------------------------------

fn put_string(bytes: &mut Vec<u8>, text: &str) {
    bytes.extend_from_slice(&(text.len() as u32).to_le_bytes());
    bytes.extend_from_slice(text.as_bytes());
}

/// Reads the string that starts at `*at` in `payload`, and moves `*at` past
/// it, so the next read picks up where this one stopped.
fn take_string(payload: &[u8], at: &mut usize) -> Result<String, String> {
    let rest = &payload[*at..];
    if rest.len() < 4 {
        return Err("a string cut off before its length".to_string());
    }
    let length = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
    if rest.len() - 4 < length {
        return Err("a string cut off partway".to_string());
    }
    let text = String::from_utf8(rest[4..4 + length].to_vec())
        .map_err(|_| "a string that isn't UTF-8".to_string())?;
    *at += 4 + length;
    Ok(text)
}

/// Reads the u32 that starts at `*at` in `payload`, and moves `*at` past
/// it.
fn take_u32(payload: &[u8], at: &mut usize) -> Result<u32, String> {
    let rest = &payload[*at..];
    if rest.len() < 4 {
        return Err("a number cut off partway".to_string());
    }
    *at += 4;
    Ok(u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]))
}

/// Says no if anything is left over after the last field.  A packet with
/// extra bytes on the end was built by something that doesn't agree with
/// us about the protocol, and we'd rather hear about it than guess.
fn finished(payload: &[u8], at: usize) -> Result<(), String> {
    if at == payload.len() {
        Ok(())
    } else {
        Err(format!("{} byte(s) left over at the end", payload.len() - at))
    }
}

// ---------------------------------------------------------------------------
// The packets the server sends
// ---------------------------------------------------------------------------

pub fn hello() -> Vec<u8> {
    frame(PacketType::Hello, &[PROTOCOL_VERSION])
}

/// Where a login stands in Security's line.  The wait is capped at what a
/// u32 holds, which is 49 days; nobody waits that long.
pub fn in_line(ahead: u64, wait_millis: u128) -> Vec<u8> {
    let mut payload = Vec::with_capacity(8);
    payload.extend_from_slice(&(ahead.min(u32::MAX as u64) as u32).to_le_bytes());
    payload.extend_from_slice(&(wait_millis.min(u32::MAX as u128) as u32).to_le_bytes());
    frame(PacketType::InLine, &payload)
}

pub fn login_result(answer: LoginAnswer) -> Vec<u8> {
    let mut payload = vec![answer as u8];
    put_string(&mut payload, answer.message());
    frame(PacketType::LoginResult, &payload)
}

/// The token and where to take it.  The port rides along, so the client
/// never has to be told it separately.
pub fn ticket(token: &str, udp_port: u16) -> Vec<u8> {
    let mut payload = Vec::with_capacity(4 + token.len() + 2);
    put_string(&mut payload, token);
    payload.extend_from_slice(&udp_port.to_le_bytes());
    frame(PacketType::Ticket, &payload)
}

/// The answer to a Connect, as a UDP packet (no length in front).  Always
/// smaller than any Connect we answer, since those carry a whole token.
/// That way somebody who fakes the sender's address can't use us to flood
/// a stranger with more bytes than they sent.
pub fn connect_result(answer: ConnectAnswer) -> Vec<u8> {
    let mut bytes = vec![PacketType::ConnectResult as u8, answer as u8];
    put_string(&mut bytes, answer.message());
    bytes
}

/// The server's half of a keep-alive: the one byte.
pub fn keep_alive() -> Vec<u8> {
    vec![PacketType::KeepAlive as u8]
}

/// The server is dropping the player, and this says why.
pub fn kicked(reason: KickReason) -> Vec<u8> {
    let mut bytes = vec![PacketType::Kicked as u8];
    bytes.extend_from_slice(&(reason as u32).to_le_bytes());
    bytes
}

/// The general "done" for an ask.
pub fn command_accepted(ask: u32) -> Vec<u8> {
    let mut bytes = vec![PacketType::CommandAccepted as u8];
    bytes.extend_from_slice(&ask.to_le_bytes());
    bytes
}

/// The general "not done", and why, for the player.
pub fn command_refused(ask: u32, why: &str) -> Vec<u8> {
    let mut bytes = vec![PacketType::CommandRefused as u8];
    bytes.extend_from_slice(&ask.to_le_bytes());
    put_string(&mut bytes, why);
    bytes
}

/// The general "one moment": the ask is being worked on and will take a
/// while, and these are the words to show meanwhile.  Not the answer, so
/// the book doesn't keep it; the answer follows under the same ask
/// number.  Protogame sends it when a picked character is locked and it
/// waits the lock out (version 10).
pub fn please_wait(ask: u32, words: &str) -> Vec<u8> {
    let mut bytes = vec![PacketType::PleaseWait as u8];
    bytes.extend_from_slice(&ask.to_le_bytes());
    put_string(&mut bytes, words);
    bytes
}

/// An account's characters, for character select.  Three at most, so it
/// stays far under MAX_UDP_BYTES.
pub fn character_list(ask: u32, characters: &[ListedCharacter]) -> Vec<u8> {
    let mut bytes = vec![PacketType::CharacterListDelivery as u8];
    bytes.extend_from_slice(&ask.to_le_bytes());
    bytes.push(characters.len().min(u8::MAX as usize) as u8);
    for character in characters.iter().take(u8::MAX as usize) {
        put_string(&mut bytes, &character.uuid);
        put_string(&mut bytes, &character.name);
        bytes.push(character.slot);
        bytes.push(u8::from(character.playable));
    }
    bytes
}

pub fn create_result(ask: u32, answer: CreateAnswer) -> Vec<u8> {
    let mut bytes = vec![PacketType::CharacterCreateResult as u8];
    bytes.extend_from_slice(&ask.to_le_bytes());
    bytes.push(answer as u8);
    put_string(&mut bytes, answer.message());
    bytes
}

pub fn delete_result(ask: u32, answer: DeleteAnswer, message: &str) -> Vec<u8> {
    let mut bytes = vec![PacketType::CharacterDeleteResult as u8];
    bytes.extend_from_slice(&ask.to_le_bytes());
    bytes.push(answer as u8);
    put_string(&mut bytes, message);
    bytes
}

/// The answer to a UserPressPlay: the character is loaded, and this is
/// the map to fetch before it comes in, where it will stand, and how many
/// chunks each way it sees.  The map's numbers are worked out once a
/// START SERVER (`overworld.rs`).
pub fn overworld_map_offer(ask: u32, size: u32, pieces: u32, hash: &str, position: [f32; 3], view: u8)
    -> Vec<u8> {
    let mut bytes = vec![PacketType::OverworldMapOffer as u8];
    bytes.extend_from_slice(&ask.to_le_bytes());
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&(MAP_PIECE_BYTES as u16).to_le_bytes());
    bytes.extend_from_slice(&pieces.to_le_bytes());
    put_string(&mut bytes, hash);
    for axis in position {
        bytes.extend_from_slice(&axis.to_le_bytes());
    }
    bytes.push(view);
    bytes
}

/// One piece of the map, numbered from 0.  `bytes` is MAP_PIECE_BYTES or
/// fewer; the caller cuts them.
pub fn overworld_map_piece(index: u32, bytes: &[u8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(1 + 4 + bytes.len());
    packet.push(PacketType::OverworldMapPiece as u8);
    packet.extend_from_slice(&index.to_le_bytes());
    packet.extend_from_slice(bytes);
    packet
}

/// A chunk's place as it goes on the wire: x and z as i16, then the row.
/// An i16 holds every chunk there is: at `world_size` 32, the most, the
/// world is 1024 chunks across, -512 to 511.
fn put_chunk_pos(bytes: &mut Vec<u8>, pos: ChunkPos) {
    bytes.extend_from_slice(&(pos.x as i16).to_le_bytes());
    bytes.extend_from_slice(&(pos.z as i16).to_le_bytes());
    bytes.push(pos.row);
}

/// A squeezed chunk as its ChunkPieces, CHUNK_PIECE_BYTES at a time.
/// Empty for no bytes, or for more than 255 pieces' worth (304,960
/// bytes), which no chunk squeezes to: its worst is about 164 KB.
pub fn chunk_pieces(pos: ChunkPos, squeezed: &[u8]) -> Vec<Vec<u8>> {
    let count = squeezed.len().div_ceil(CHUNK_PIECE_BYTES);
    if count == 0 || count > u8::MAX as usize {
        return Vec::new();
    }
    squeezed.chunks(CHUNK_PIECE_BYTES).enumerate().map(|(index, piece)| {
        let mut bytes = Vec::with_capacity(CHUNK_PIECE_HEADER + piece.len());
        bytes.push(PacketType::ChunkPiece as u8);
        put_chunk_pos(&mut bytes, pos);
        bytes.push(index as u8 + 1);
        bytes.push(count as u8);
        bytes.extend_from_slice(piece);
        bytes
    }).collect()
}

/// A chunk asked for that isn't sent, and why.
pub fn chunk_refused(pos: ChunkPos, why: ChunkRefusal) -> Vec<u8> {
    let mut bytes = vec![PacketType::ChunkRefused as u8];
    put_chunk_pos(&mut bytes, pos);
    bytes.push(why as u8);
    bytes
}

/// The character is in the world, and where: the answer to a PlayerReady
/// (a UserPressPlay's, before version 11).
pub fn entered_world(ask: u32, character: &EnteredCharacter) -> Vec<u8> {
    let mut bytes = vec![PacketType::CharacterEnteredWorld as u8];
    bytes.extend_from_slice(&ask.to_le_bytes());
    put_string(&mut bytes, &character.uuid);
    put_string(&mut bytes, &character.name);
    put_floats(&mut bytes, &character.position);
    bytes.extend_from_slice(&character.object.to_le_bytes());
    put_floats(&mut bytes, &[character.walk, character.turn]);
    bytes
}

/// A cycle's chat, as few ChatDeliveries as it fits in: each line in
/// order, and a new packet started whenever the next line would take
/// this one past MAX_UDP_BYTES (or past 255 lines, what the count
/// holds).  No lines, no packets.  A line too big for a packet on its own
/// can't happen (chat cuts a message at 300 characters), and one would
/// go out alone and be too big.
pub fn chat_deliveries(lines: &[String]) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    let mut packet: Vec<u8> = Vec::new();
    let mut count: u8 = 0;
    for line in lines {
        let room = 4 + line.len();
        if count > 0 && (packet.len() + room > MAX_UDP_BYTES || count == u8::MAX) {
            packet[1] = count;
            packets.push(std::mem::take(&mut packet));
            count = 0;
        }
        if count == 0 {
            packet = vec![PacketType::ChatDelivery as u8, 0];
        }
        put_string(&mut packet, line);
        count += 1;
    }
    if count > 0 {
        packet[1] = count;
        packets.push(packet);
    }
    packets
}

/// The answer to a `/who`: the time it ran, in seconds since midnight
/// UTC, and the characters, in the order given (the caller sorts them),
/// each with its block and its seconds online.  No limit on its size
/// here: one too big for a packet goes out in pieces (`spans()`).
pub fn who_delivery(ask: u32, seconds_since_midnight: u32, entries: &[WhoEntry]) -> Vec<u8> {
    let mut bytes = vec![PacketType::WhoDelivery as u8];
    bytes.extend_from_slice(&ask.to_le_bytes());
    bytes.extend_from_slice(&seconds_since_midnight.to_le_bytes());
    let count = entries.len().min(u16::MAX as usize);
    bytes.extend_from_slice(&(count as u16).to_le_bytes());
    for entry in entries.iter().take(count) {
        put_string(&mut bytes, &entry.name);
        for axis in entry.block {
            bytes.extend_from_slice(&axis.to_le_bytes());
        }
        bytes.extend_from_slice(&entry.online.to_le_bytes());
    }
    bytes
}

/// What goes in each Span besides its piece: the type, the ask number,
/// which piece and how many.
const SPAN_HEADER: usize = 1 + 4 + 1 + 1;

/// The biggest piece of an answer one Span carries.
const SPAN_PIECE: usize = MAX_UDP_BYTES - SPAN_HEADER;

/// An answer as the packets that carry it: itself alone if it fits in
/// MAX_UDP_BYTES, or else Spans, each a piece of it in order.  Empty if
/// it's too big for even 255 Spans (about 300 KB), which nothing sends:
/// a `/who list` of eight thousand characters would just fit.
pub fn spans(ask: u32, answer: &[u8]) -> Vec<Vec<u8>> {
    if answer.len() <= MAX_UDP_BYTES {
        return vec![answer.to_vec()];
    }
    let pieces: Vec<&[u8]> = answer.chunks(SPAN_PIECE).collect();
    if pieces.len() > u8::MAX as usize {
        return Vec::new();
    }
    let count = pieces.len() as u8;
    pieces.iter().enumerate().map(|(index, piece)| {
        let mut bytes = Vec::with_capacity(SPAN_HEADER + piece.len());
        bytes.push(PacketType::Span as u8);
        bytes.extend_from_slice(&ask.to_le_bytes());
        bytes.push(index as u8 + 1);
        bytes.push(count);
        bytes.extend_from_slice(piece);
        bytes
    }).collect()
}

/// f32s, one after another, lowest byte first.
fn put_floats(bytes: &mut Vec<u8>, floats: &[f32]) {
    for float in floats {
        bytes.extend_from_slice(&float.to_le_bytes());
    }
}

/// One object's motion, MOTION_BYTES of it.
fn put_motion(bytes: &mut Vec<u8>, motion: &Motion) {
    bytes.extend_from_slice(&motion.object.to_le_bytes());
    put_floats(bytes, &motion.position);
    put_floats(bytes, &motion.rotation);
    put_floats(bytes, &motion.velocity);
}

/// An object whole, for the player who's to draw it.  About 100 bytes
/// for a character: a uuid, a name and nothing in the strings past that.
pub fn hydrate(object: &Hydrate) -> Vec<u8> {
    let mut bytes = vec![PacketType::Hydrate as u8];
    bytes.extend_from_slice(&object.motion.object.to_le_bytes());
    put_string(&mut bytes, &object.uuid);
    bytes.push(object.living as u8);
    put_string(&mut bytes, &object.short_name);
    put_floats(&mut bytes, &object.motion.position);
    put_floats(&mut bytes, &object.motion.rotation);
    put_floats(&mut bytes, &object.motion.velocity);
    put_floats(&mut bytes, &object.scale);
    put_string(&mut bytes, &object.model);
    bytes.push(object.shape);
    bytes.push(object.collider);
    put_floats(&mut bytes, &object.collider_size);
    put_string(&mut bytes, &object.track);
    bytes
}

/// A player's character pulled back to its last good spot (version 15).
pub fn move_correction(pull_back: &PullBack) -> Vec<u8> {
    let mut bytes = vec![PacketType::MoveCorrection as u8];
    bytes.extend_from_slice(&pull_back.number.to_le_bytes());
    put_floats(&mut bytes, &pull_back.position);
    put_floats(&mut bytes, &pull_back.rotation);
    bytes
}

/// Objects that moved, as few ObjectsMoved as they fit in, MOVED_AT_ONCE
/// to a packet.  None, no packets.
pub fn objects_moved(motions: &[Motion]) -> Vec<Vec<u8>> {
    motions.chunks(MOVED_AT_ONCE).map(|some| {
        let mut bytes = Vec::with_capacity(2 + some.len() * MOTION_BYTES);
        bytes.push(PacketType::ObjectsMoved as u8);
        bytes.push(some.len() as u8);
        for motion in some {
            put_motion(&mut bytes, motion);
        }
        bytes
    }).collect()
}

/// Objects gone, as few ObjectsGone as they fit in, GONE_AT_ONCE to a
/// packet.  None, no packets.
pub fn objects_gone(objects: &[u32]) -> Vec<Vec<u8>> {
    objects.chunks(GONE_AT_ONCE).map(|some| {
        let mut bytes = Vec::with_capacity(3 + some.len() * 4);
        bytes.push(PacketType::ObjectsGone as u8);
        bytes.extend_from_slice(&(some.len() as u16).to_le_bytes());
        for object in some {
            bytes.extend_from_slice(&object.to_le_bytes());
        }
        bytes
    }).collect()
}

/// A roll call, in as many pieces as it takes, ROLL_CALL_AT_ONCE motions
/// to a piece.  Knowing nothing is one piece with a count of 0, so the
/// client still hears it.  At most 255 pieces (7,395 objects); one that
/// knows more is told about the first 7,395, and the rest come and go on
/// their own.
pub fn roll_call(roll: u32, motions: &[Motion]) -> Vec<Vec<u8>> {
    let mut pieces: Vec<&[Motion]> = motions.chunks(ROLL_CALL_AT_ONCE).take(u8::MAX as usize).collect();
    if pieces.is_empty() {
        pieces.push(&[]);
    }
    let count = pieces.len() as u8;
    pieces.iter().enumerate().map(|(index, some)| {
        let mut bytes = Vec::with_capacity(ROLL_CALL_HEADER + some.len() * MOTION_BYTES);
        bytes.push(PacketType::RollCall as u8);
        bytes.extend_from_slice(&roll.to_le_bytes());
        bytes.push(index as u8 + 1);
        bytes.push(count);
        bytes.push(some.len() as u8);
        for motion in some.iter() {
            put_motion(&mut bytes, motion);
        }
        bytes
    }).collect()
}

// ---------------------------------------------------------------------------
// The packets the server reads
// ---------------------------------------------------------------------------

/// The payload of a Login: the client's version, the secret word, the
/// username, then the password's key.
pub fn read_login(payload: &[u8]) -> Result<LoginRequest, String> {
    let mut at = 0;
    let client_version = take_string(payload, &mut at)?;
    let secret_word = take_string(payload, &mut at)?;
    let username = take_string(payload, &mut at)?;
    let key = take_string(payload, &mut at)?;
    finished(payload, at)?;
    Ok(LoginRequest { client_version, secret_word, username, key })
}

/// The payload of a SessionChoice: exactly one byte, 0 or 1.
pub fn read_session_choice(payload: &[u8]) -> Result<Choice, String> {
    match payload {
        [0] => Ok(Choice::LogTheOtherOut),
        [1] => Ok(Choice::HangUp),
        [other] => Err(format!("a session choice of {other}, and only 0 and 1 mean anything")),
        _ => Err(format!("a session choice of {} bytes, and it should be 1", payload.len())),
    }
}

/// The payload of a Connect: the token.  One that isn't TOKEN_LENGTH long
/// is refused here, so a Connect too small to be real never gets an
/// answer (see `connect_result()`).
pub fn read_connect(payload: &[u8]) -> Result<String, String> {
    let mut at = 0;
    let token = take_string(payload, &mut at)?;
    finished(payload, at)?;
    if token.len() != TOKEN_LENGTH {
        return Err(format!("a token of {} bytes, and it should be {}", token.len(), TOKEN_LENGTH));
    }
    Ok(token)
}

/// The payload of a CharacterListRequest: the ask number.
pub fn read_list_request(payload: &[u8]) -> Result<u32, String> {
    let mut at = 0;
    let ask = take_u32(payload, &mut at)?;
    finished(payload, at)?;
    Ok(ask)
}

/// The payload of a CreateCharacter: the ask number and the name.
pub fn read_create(payload: &[u8]) -> Result<(u32, String), String> {
    let mut at = 0;
    let ask = take_u32(payload, &mut at)?;
    let name = take_string(payload, &mut at)?;
    finished(payload, at)?;
    Ok((ask, name))
}

/// The payload of a DeleteCharacter: the ask number, the uuid, and the
/// word the player typed.
pub fn read_delete(payload: &[u8]) -> Result<(u32, String, String), String> {
    let mut at = 0;
    let ask = take_u32(payload, &mut at)?;
    let uuid = take_string(payload, &mut at)?;
    let typed = take_string(payload, &mut at)?;
    finished(payload, at)?;
    Ok((ask, uuid, typed))
}

/// The payload of a CharacterRequestResetHome: the ask number and the
/// uuid.
pub fn read_reset_home(payload: &[u8]) -> Result<(u32, String), String> {
    let mut at = 0;
    let ask = take_u32(payload, &mut at)?;
    let uuid = take_string(payload, &mut at)?;
    finished(payload, at)?;
    Ok((ask, uuid))
}

/// The payload of a UserPressPlay: the ask number and the uuid.
pub fn read_user_press_play(payload: &[u8]) -> Result<(u32, String), String> {
    let mut at = 0;
    let ask = take_u32(payload, &mut at)?;
    let uuid = take_string(payload, &mut at)?;
    finished(payload, at)?;
    Ok((ask, uuid))
}

/// The payload of a PlayerReady: the ask number and the map's SHA-256 as
/// the client has it.  Whether that's the right hash is the caller's to
/// say.
pub fn read_player_ready(payload: &[u8]) -> Result<(u32, String), String> {
    let mut at = 0;
    let ask = take_u32(payload, &mut at)?;
    let hash = take_string(payload, &mut at)?;
    finished(payload, at)?;
    Ok((ask, hash))
}

/// The payload of an OverworldMapRequest: the first piece's number and how
/// many, 1 to MAP_PIECES_AT_ONCE.  Pieces past the end of the map are the
/// caller's to leave out.
pub fn read_overworld_map_request(payload: &[u8]) -> Result<(u32, u8), String> {
    let mut at = 0;
    let first = take_u32(payload, &mut at)?;
    let Some(&count) = payload.get(at) else {
        return Err("a map request cut off before its count".to_string());
    };
    at += 1;
    finished(payload, at)?;
    if count == 0 || count > MAP_PIECES_AT_ONCE {
        return Err(format!("a map request for {count} pieces, and it's 1 to {MAP_PIECES_AT_ONCE}"));
    }
    Ok((first, count))
}

/// The payload of a ChunkRequest: the chunks asked for, 1 to
/// CHUNKS_AT_ONCE, in the order asked.  Whether the player may have them
/// is the caller's to say.  A row past the top is left for the caller
/// too: it's outside the world, like an x or z past the edge.
pub fn read_chunk_request(payload: &[u8]) -> Result<Vec<ChunkPos>, String> {
    let Some((&count, places)) = payload.split_first() else {
        return Err("an empty chunk request".to_string());
    };
    if count == 0 || count > CHUNKS_AT_ONCE {
        return Err(format!("a chunk request for {count} chunks, and it's 1 to {CHUNKS_AT_ONCE}"));
    }
    if places.len() != count as usize * 5 {
        return Err(format!("a chunk request for {count} chunks with {} bytes of places", places.len()));
    }
    Ok(places.chunks(5).map(|place| ChunkPos {
        x: i16::from_le_bytes([place[0], place[1]]) as i32,
        z: i16::from_le_bytes([place[2], place[3]]) as i32,
        row: place[4],
    }).collect())
}

/// The payload of a PlayerCommand: the ask number and the line as typed.
pub fn read_player_command(payload: &[u8]) -> Result<(u32, String), String> {
    let mut at = 0;
    let ask = take_u32(payload, &mut at)?;
    let line = take_string(payload, &mut at)?;
    finished(payload, at)?;
    Ok((ask, line))
}

/// What a PlayerMoved says: the move's number, the last pull-back its
/// client had, and the character's position, rotation and velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerMove {
    pub number: u32,
    pub pull_backs_had: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub velocity: [f32; 3],
}

/// The payload of a PlayerMoved (version 15).  Whether the numbers make
/// sense (a NaN, a place too far) is the GameClock's to say.
pub fn read_player_moved(payload: &[u8]) -> Result<PlayerMove, String> {
    if payload.len() != PLAYER_MOVED_BYTES {
        return Err(format!("a move of {} bytes, and it should be {PLAYER_MOVED_BYTES}", payload.len()));
    }
    let number = |at: usize| u32::from_le_bytes([payload[at], payload[at + 1], payload[at + 2], payload[at + 3]]);
    let float = |at: usize| f32::from_le_bytes([payload[at], payload[at + 1], payload[at + 2], payload[at + 3]]);
    let three = |at: usize| [float(at), float(at + 4), float(at + 8)];
    Ok(PlayerMove { number: number(0), pull_backs_had: number(4), position: three(8), rotation: three(20),
                    velocity: three(32) })
}

/// The payload of an ObjectAsk: the numbers asked about, 1 to
/// ASKS_AT_ONCE, in the order asked.  Whether the player may see them is
/// the GameClock's to say.
pub fn read_object_ask(payload: &[u8]) -> Result<Vec<u32>, String> {
    let Some((&count, numbers)) = payload.split_first() else {
        return Err("an empty object ask".to_string());
    };
    if count == 0 || count > ASKS_AT_ONCE {
        return Err(format!("an object ask for {count} objects, and it's 1 to {ASKS_AT_ONCE}"));
    }
    if numbers.len() != count as usize * 4 {
        return Err(format!("an object ask for {count} objects with {} bytes of numbers", numbers.len()));
    }
    Ok(numbers.chunks(4).map(|number| u32::from_le_bytes([number[0], number[1], number[2], number[3]])).collect())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A payload of strings, built the long way, the way a client would.
    fn strings(parts: &[&str]) -> Vec<u8> {
        let mut payload = Vec::new();
        for part in parts {
            put_string(&mut payload, part);
        }
        payload
    }

    #[test]
    fn a_hello_is_six_bytes() {
        assert_eq!(hello(), vec![2, 0, 0, 0, 0x10, PROTOCOL_VERSION]);
    }

    #[test]
    fn numbers_go_lowest_byte_first() {
        // Spelled out byte by byte, because this is exactly what the
        // client has to match.  "Invalid Credentials" is 19 bytes, so the
        // length is 1 + 1 + 4 + 19 = 25.
        let mut expected = vec![25, 0, 0, 0, 0x13, 1, 19, 0, 0, 0];
        expected.extend_from_slice(b"Invalid Credentials");
        assert_eq!(login_result(LoginAnswer::Failed), expected);
    }

    #[test]
    fn every_login_answer_carries_its_byte_and_its_words() {
        for answer in [LoginAnswer::Failed, LoginAnswer::AlreadyLoggedIn, LoginAnswer::Outdated,
                       LoginAnswer::Unavailable] {
            let mut buffer = login_result(answer);
            let packet = take_packet(&mut buffer).unwrap().unwrap();
            assert_eq!(packet.kind, PacketType::LoginResult as u8);
            assert_eq!(packet.payload[0], answer as u8);
            let mut at = 1;
            assert_eq!(take_string(&packet.payload, &mut at), Ok(answer.message().to_string()));
            assert_eq!(finished(&packet.payload, at), Ok(()));
        }
    }

    #[test]
    fn a_place_in_line_in_bytes() {
        // 3 ahead, 120 ms.  The length is 1 + 4 + 4 = 9.
        assert_eq!(in_line(3, 120), vec![9, 0, 0, 0, 0x12, 3, 0, 0, 0, 120, 0, 0, 0]);
        // Too big to fit is capped, not wrapped.
        assert_eq!(&in_line(u64::MAX, u128::MAX)[5..], &[255, 255, 255, 255, 255, 255, 255, 255]);
    }

    #[test]
    fn a_ticket_in_bytes() {
        // 9998 is 0x270E, so the port goes out as 0E 27.
        assert_eq!(ticket("ab", 9998), vec![9, 0, 0, 0, 0x15, 2, 0, 0, 0, b'a', b'b', 0x0E, 0x27]);
    }

    #[test]
    fn udp_answers_in_bytes() {
        // No length in front, because they go over UDP.
        let mut expected = vec![0x31, 1, 19, 0, 0, 0];
        expected.extend_from_slice(b"Invalid Credentials");
        assert_eq!(connect_result(ConnectAnswer::Refused), expected);
        assert_eq!(connect_result(ConnectAnswer::Accepted)[1], 0);

        assert_eq!(keep_alive(), vec![0x32]);
        assert_eq!(kicked(KickReason::LoggedInElsewhere), vec![0x34, 1, 0, 0, 0]);
        assert_eq!(kicked(KickReason::ServerStopping), vec![0x34, 2, 0, 0, 0]);
        assert_eq!(kicked(KickReason::Banned), vec![0x34, 3, 0, 0, 0]);
        assert_eq!(kicked(KickReason::KickedByAdmin), vec![0x34, 4, 0, 0, 0]);
        assert_eq!(kicked(KickReason::AccountTerminated), vec![0x34, 5, 0, 0, 0]);
        assert_eq!(kicked(KickReason::CharacterLocked), vec![0x34, 6, 0, 0, 0]);
    }

    /// The key for `jacob_01` / `Correct horse 1!`, PROTOCOL.md's example.
    const KEY: &str = "fc71f0c94665dfd6ff4e217891cd7ff81c8fd8ed7b20cf498c122c1bbd1f8855";

    #[test]
    fn a_login_reads_back() {
        let payload = strings(&["0.0.1", "potato", "jacob_01", KEY]);
        let login = read_login(&payload).unwrap();
        assert_eq!(login.client_version, "0.0.1");
        assert_eq!(login.secret_word, "potato");
        assert_eq!(login.username, "jacob_01");
        assert_eq!(login.key, KEY);
    }

    #[test]
    fn broken_logins_are_refused() {
        let payload = strings(&["0.0.1", "potato", "jacob_01", KEY]);

        // Cut off anywhere short of the end.
        for cut in 0..payload.len() {
            assert!(read_login(&payload[..cut]).is_err(), "cut at {cut}");
        }

        // Something extra on the end.
        let mut longer = payload.clone();
        longer.push(0);
        assert!(read_login(&longer).is_err());

        // A string that isn't UTF-8.
        let mut not_text = Vec::new();
        not_text.extend_from_slice(&2u32.to_le_bytes());
        not_text.extend_from_slice(&[0xFF, 0xFE]);
        assert!(take_string(&not_text, &mut 0).is_err());

        // A string that claims to be longer than the packet.
        let mut liar = Vec::new();
        liar.extend_from_slice(&1000u32.to_le_bytes());
        liar.extend_from_slice(b"short");
        assert!(take_string(&liar, &mut 0).is_err());
    }

    #[test]
    fn a_connect_reads_back_and_a_short_token_does_not() {
        let token = "0123456789abcdef".repeat(4);
        assert_eq!(read_connect(&strings(&[&token])), Ok(token.clone()));

        assert!(read_connect(&strings(&[""])).is_err());
        assert!(read_connect(&strings(&[&"a".repeat(63)])).is_err());
        assert!(read_connect(&strings(&[&"a".repeat(65)])).is_err());
        assert!(read_connect(&strings(&[&token, "extra"])).is_err());

        // Every Connect we answer is bigger than every answer we send.
        let smallest = 1 + strings(&[&token]).len();
        for answer in [ConnectAnswer::Accepted, ConnectAnswer::Refused] {
            assert!(connect_result(answer).len() < smallest);
        }
    }

    #[test]
    fn session_choices() {
        assert_eq!(read_session_choice(&[0]), Ok(Choice::LogTheOtherOut));
        assert_eq!(read_session_choice(&[1]), Ok(Choice::HangUp));
        assert!(read_session_choice(&[2]).is_err());
        assert!(read_session_choice(&[]).is_err());
        assert!(read_session_choice(&[0, 0]).is_err());
    }

    #[test]
    fn udp_packets_have_no_length_and_borrow_their_payload() {
        assert_eq!(take_datagram(&[0x30, 7, 8]), Some((0x30, &[7u8, 8][..])));
        assert_eq!(take_datagram(&[0x32]), Some((0x32, &b""[..])));
        assert_eq!(take_datagram(&[]), None);
        assert!(take_datagram(&vec![0x30; MAX_UDP_BYTES]).is_some());
        assert_eq!(take_datagram(&vec![0x30; MAX_UDP_BYTES + 1]), None);
    }

    #[test]
    fn a_packet_comes_back_out_of_its_frame() {
        let mut buffer = ticket("hello", 1);
        let packet = take_packet(&mut buffer).unwrap().unwrap();
        assert_eq!(packet.kind, PacketType::Ticket as u8);
        assert_eq!(&packet.payload[..9], &[5, 0, 0, 0, b'h', b'e', b'l', b'l', b'o']);
        assert!(buffer.is_empty());
    }

    #[test]
    fn half_a_packet_waits_for_the_rest() {
        let whole = ticket("hello", 1);
        for cut in 0..whole.len() {
            let mut buffer = whole[..cut].to_vec();
            assert_eq!(take_packet(&mut buffer), Ok(None));
            assert_eq!(buffer.len(), cut);
        }
    }

    #[test]
    fn two_packets_in_one_read_come_out_one_at_a_time() {
        let mut buffer = hello();
        buffer.extend(login_result(LoginAnswer::Failed));

        assert_eq!(take_packet(&mut buffer).unwrap().unwrap().kind, 0x10);
        assert_eq!(take_packet(&mut buffer).unwrap().unwrap().kind, 0x13);
        assert_eq!(take_packet(&mut buffer), Ok(None));
    }

    #[test]
    fn bad_lengths_are_refused() {
        let mut empty = vec![0, 0, 0, 0];
        assert!(take_packet(&mut empty).is_err());

        // 4097, one more than we take.  Refused on the length alone.
        let mut huge = vec![0x01, 0x10, 0, 0];
        assert!(take_packet(&mut huge).is_err());

        // Exactly the most we take is fine, once it has all arrived.
        let mut biggest = vec![0x00, 0x10, 0, 0];
        biggest.extend(vec![0xF0; MAX_PACKET_BYTES]);
        assert!(take_packet(&mut biggest).unwrap().is_some());
    }

    #[test]
    fn every_type_survives_its_byte() {
        let every = [PacketType::Hello, PacketType::Login, PacketType::InLine, PacketType::LoginResult,
                     PacketType::SessionChoice, PacketType::Ticket, PacketType::Connect, PacketType::ConnectResult,
                     PacketType::KeepAlive, PacketType::Goodbye, PacketType::Kicked, PacketType::CommandAccepted,
                     PacketType::CommandRefused, PacketType::CharacterListRequest, PacketType::CharacterListDelivery,
                     PacketType::CreateCharacter, PacketType::CharacterCreateResult, PacketType::DeleteCharacter,
                     PacketType::CharacterDeleteResult, PacketType::CharacterRequestResetHome,
                     PacketType::UserPressPlay, PacketType::CharacterEnteredWorld, PacketType::PlayerCommand,
                     PacketType::ChatDelivery, PacketType::WhoDelivery, PacketType::Span, PacketType::PleaseWait,
                     PacketType::PlayerReady, PacketType::OverworldMapOffer, PacketType::OverworldMapRequest,
                     PacketType::OverworldMapPiece, PacketType::ChunkRequest, PacketType::ChunkPiece,
                     PacketType::ChunkRefused, PacketType::Hydrate, PacketType::ObjectsMoved,
                     PacketType::ObjectsGone, PacketType::RollCall, PacketType::ObjectAsk,
                     PacketType::PlayerMoved, PacketType::MoveCorrection];
        for kind in every {
            assert_eq!(PacketType::from_byte(kind as u8), Some(kind));
        }
        assert_eq!(PacketType::from_byte(0x00), None);
        assert_eq!(PacketType::from_byte(0x16), None);
        assert_eq!(PacketType::from_byte(0x2A), None);
        assert_eq!(PacketType::from_byte(0x3C), None);
        assert_eq!(PacketType::from_byte(0x46), None);
        assert_eq!(PacketType::from_byte(0x57), None);
    }

    #[test]
    fn the_map_at_play_in_bytes() {
        // PROTOCOL.md's example: a map of 4,194,332 bytes (world_size 16),
        // 0x0040001C, in 4,097 pieces, 0x1001, of 1,024, 0x0400, for a
        // character at 1.5, 0, -2 that sees 4 chunks each way.
        let hash = "ab".repeat(32);
        let mut expected = vec![0x40, 3, 0, 0, 0, 0x1C, 0x00, 0x40, 0x00, 0x00, 0x04, 0x01, 0x10, 0, 0, 64, 0, 0, 0];
        expected.extend_from_slice(hash.as_bytes());
        expected.extend_from_slice(&[0x00, 0x00, 0xC0, 0x3F, 0, 0, 0, 0, 0x00, 0x00, 0x00, 0xC0, 4]);
        assert_eq!(overworld_map_offer(3, 4_194_332, 4_097, &hash, [1.5, 0.0, -2.0], 4), expected);

        // The last piece: number 4,096, 0x1000, the 28 bytes left.
        let mut expected = vec![0x42, 0x00, 0x10, 0, 0];
        expected.extend_from_slice(&[7; 28]);
        assert_eq!(overworld_map_piece(4_096, &[7; 28]), expected);
        // A whole piece fits under the cap.
        assert!(overworld_map_piece(0, &[0; MAP_PIECE_BYTES]).len() <= MAX_UDP_BYTES);

        let mut ready = vec![4, 0, 0, 0];
        ready.extend_from_slice(&strings(&[&hash]));
        assert_eq!(read_player_ready(&ready), Ok((4, hash.clone())));
        ready.push(0);
        assert!(read_player_ready(&ready).is_err());
    }

    #[test]
    fn a_map_request_is_one_to_sixty_four_pieces() {
        assert_eq!(read_overworld_map_request(&[64, 0, 0, 0, 64]), Ok((64, 64)));
        assert_eq!(read_overworld_map_request(&[0, 0x10, 0, 0, 1]), Ok((4_096, 1)));
        assert!(read_overworld_map_request(&[0, 0, 0, 0, 0]).is_err());
        assert!(read_overworld_map_request(&[0, 0, 0, 0, 65]).is_err());
        assert!(read_overworld_map_request(&[0, 0, 0, 0]).is_err());
        assert!(read_overworld_map_request(&[0, 0, 0, 0, 1, 1]).is_err());
    }

    #[test]
    fn the_chunks_in_bytes() {
        // PROTOCOL.md's example: chunk -1,0 row 1, Alpha's ground just
        // west of 0,0, squeezed to 13 bytes.
        let pos = ChunkPos { x: -1, z: 0, row: 1 };
        let squeezed = [1, 2, 0, 1, 0, 0, 0, 0xFF, 0x07, 0, 0xFF, 0xF7, 1];
        let pieces = chunk_pieces(pos, &squeezed);
        let mut expected = vec![0x44, 0xFF, 0xFF, 0x00, 0x00, 0x01, 1, 1];
        expected.extend_from_slice(&squeezed);
        assert_eq!(pieces, vec![expected]);

        assert_eq!(chunk_refused(ChunkPos { x: 300, z: -2, row: 10 }, ChunkRefusal::NotYet),
                   vec![0x45, 0x2C, 0x01, 0xFE, 0xFF, 10, 2]);

        // Two whole pieces and 5 bytes: three pieces, each under the cap,
        // that put together are the chunk.
        let big: Vec<u8> = (0..CHUNK_PIECE_BYTES * 2 + 5).map(|n| n as u8).collect();
        let pieces = chunk_pieces(pos, &big);
        assert_eq!(pieces.len(), 3);
        let mut joined = Vec::new();
        for (index, piece) in pieces.iter().enumerate() {
            assert!(piece.len() <= MAX_UDP_BYTES);
            assert_eq!(piece[6], index as u8 + 1);
            assert_eq!(piece[7], 3);
            joined.extend_from_slice(&piece[CHUNK_PIECE_HEADER..]);
        }
        assert_eq!(joined, big);
        assert_eq!(pieces[0].len(), MAX_UDP_BYTES);

        // Nothing, or more than 255 pieces, can't be sent.
        assert!(chunk_pieces(pos, &[]).is_empty());
        assert!(chunk_pieces(pos, &vec![0; CHUNK_PIECE_BYTES * 255]).len() == 255);
        assert!(chunk_pieces(pos, &vec![0; CHUNK_PIECE_BYTES * 255 + 1]).is_empty());
    }

    #[test]
    fn a_chunk_request_is_one_to_sixty_four_places() {
        // Chunk -1,0 row 1, and 300,-2 row 10.
        assert_eq!(read_chunk_request(&[2, 0xFF, 0xFF, 0, 0, 1, 0x2C, 0x01, 0xFE, 0xFF, 10]),
                   Ok(vec![ChunkPos { x: -1, z: 0, row: 1 }, ChunkPos { x: 300, z: -2, row: 10 }]));
        let mut full = vec![CHUNKS_AT_ONCE];
        full.extend(vec![0; CHUNKS_AT_ONCE as usize * 5]);
        assert_eq!(read_chunk_request(&full).map(|places| places.len()), Ok(64));

        assert!(read_chunk_request(&[]).is_err());
        assert!(read_chunk_request(&[0]).is_err());
        let mut too_many = vec![CHUNKS_AT_ONCE + 1];
        too_many.extend(vec![0; (CHUNKS_AT_ONCE as usize + 1) * 5]);
        assert!(read_chunk_request(&too_many).is_err());
        // A place cut short, and a byte too many.
        assert!(read_chunk_request(&[1, 0, 0, 0, 0]).is_err());
        assert!(read_chunk_request(&[1, 0, 0, 0, 0, 1, 0]).is_err());
    }

    #[test]
    fn the_general_answers_in_bytes() {
        assert_eq!(command_accepted(7), vec![0x35, 7, 0, 0, 0]);
        let mut expected = vec![0x36, 7, 0, 0, 0, 3, 0, 0, 0];
        expected.extend_from_slice(b"No.");
        assert_eq!(command_refused(7, "No."), expected);
        let mut expected = vec![0x3B, 7, 0, 0, 0, 10, 0, 0, 0];
        expected.extend_from_slice(b"One moment");
        assert_eq!(please_wait(7, "One moment"), expected);
    }

    #[test]
    fn a_character_list_in_bytes() {
        let jacob = ListedCharacter { uuid: "u-1".to_string(), name: "Jacob".to_string(), slot: 1, playable: true };
        let mckay = ListedCharacter { uuid: "u-2".to_string(), name: "Mckay".to_string(), slot: 3, playable: false };
        let mut expected = vec![0x21, 9, 0, 0, 0, 2];
        expected.extend_from_slice(&[3, 0, 0, 0]);
        expected.extend_from_slice(b"u-1");
        expected.extend_from_slice(&[5, 0, 0, 0]);
        expected.extend_from_slice(b"Jacob");
        expected.extend_from_slice(&[1, 1]);
        expected.extend_from_slice(&[3, 0, 0, 0]);
        expected.extend_from_slice(b"u-2");
        expected.extend_from_slice(&[5, 0, 0, 0]);
        expected.extend_from_slice(b"Mckay");
        expected.extend_from_slice(&[3, 0]);
        assert_eq!(character_list(9, &[jacob, mckay]), expected);
        assert_eq!(character_list(1, &[]), vec![0x21, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn create_and_delete_answers_in_bytes() {
        let made = create_result(4, CreateAnswer::Made);
        assert_eq!(&made[..6], &[0x23, 4, 0, 0, 0, 0]);
        let mut at = 6;
        assert_eq!(take_string(&made, &mut at), Ok(CreateAnswer::Made.message().to_string()));
        assert_eq!(finished(&made, at), Ok(()));
        for (answer, byte) in [(CreateAnswer::NameNotAllowed, 1), (CreateAnswer::NameTaken, 2),
                               (CreateAnswer::SlotsFull, 3), (CreateAnswer::Unavailable, 4)] {
            assert_eq!(create_result(1, answer)[5], byte);
        }

        let denied = delete_result(5, DeleteAnswer::Denied, "Type DELETE.");
        assert_eq!(&denied[..6], &[0x25, 5, 0, 0, 0, 1]);
        assert_eq!(delete_result(5, DeleteAnswer::Approved, "Gone.")[5], 0);
    }

    #[test]
    fn character_select_asks_read_back() {
        assert_eq!(read_list_request(&[3, 0, 0, 0]), Ok(3));
        assert!(read_list_request(&[3, 0, 0]).is_err());
        assert!(read_list_request(&[3, 0, 0, 0, 0]).is_err());

        let mut create = 8u32.to_le_bytes().to_vec();
        put_string(&mut create, "Jacob");
        assert_eq!(read_create(&create), Ok((8, "Jacob".to_string())));
        assert!(read_create(&create[..6]).is_err());

        let mut delete = 9u32.to_le_bytes().to_vec();
        put_string(&mut delete, "u-1");
        put_string(&mut delete, "DELETE");
        assert_eq!(read_delete(&delete), Ok((9, "u-1".to_string(), "DELETE".to_string())));
        delete.push(0);
        assert!(read_delete(&delete).is_err());

        let mut home = 10u32.to_le_bytes().to_vec();
        put_string(&mut home, "u-1");
        assert_eq!(read_reset_home(&home), Ok((10, "u-1".to_string())));
        assert!(read_reset_home(&[]).is_err());

        let mut play = 11u32.to_le_bytes().to_vec();
        put_string(&mut play, "u-1");
        assert_eq!(read_user_press_play(&play), Ok((11, "u-1".to_string())));
        assert!(read_user_press_play(&play[..7]).is_err());
        play.push(0);
        assert!(read_user_press_play(&play).is_err());
    }

    #[test]
    fn a_character_entering_the_world_in_bytes() {
        // 1.5 is 0x3FC00000 as an f32, and -2.0 is 0xC0000000, lowest
        // byte first.
        let jacob = EnteredCharacter { uuid: "u-1".to_string(), name: "Jacob".to_string(),
                                       position: [1.5, 0.0, -2.0], object: 7, walk: 4.0, turn: 450.0 };
        let mut expected = vec![0x28, 12, 0, 0, 0];
        expected.extend_from_slice(&[3, 0, 0, 0]);
        expected.extend_from_slice(b"u-1");
        expected.extend_from_slice(&[5, 0, 0, 0]);
        expected.extend_from_slice(b"Jacob");
        expected.extend_from_slice(&[0x00, 0x00, 0xC0, 0x3F]);
        expected.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        expected.extend_from_slice(&[0x00, 0x00, 0x00, 0xC0]);
        expected.extend_from_slice(&[7, 0, 0, 0]);
        // 4.0 is 0x40800000 and 450.0 0x43E10000.
        expected.extend_from_slice(&[0x00, 0x00, 0x80, 0x40]);
        expected.extend_from_slice(&[0x00, 0x00, 0xE1, 0x43]);
        assert_eq!(entered_world(12, &jacob), expected);
    }

    #[test]
    fn a_player_command_reads_back() {
        let mut command = 13u32.to_le_bytes().to_vec();
        put_string(&mut command, "/chat Yo yo yo!");
        assert_eq!(read_player_command(&command), Ok((13, "/chat Yo yo yo!".to_string())));
        assert!(read_player_command(&command[..10]).is_err());
        command.push(0);
        assert!(read_player_command(&command).is_err());
    }

    #[test]
    fn a_chat_delivery_in_bytes() {
        let lines = vec!["[Chat] Jacob: Yo".to_string(), "[Chat] Mckay: Hi".to_string()];
        let mut expected = vec![0x38, 2];
        expected.extend_from_slice(&[16, 0, 0, 0]);
        expected.extend_from_slice(b"[Chat] Jacob: Yo");
        expected.extend_from_slice(&[16, 0, 0, 0]);
        expected.extend_from_slice(b"[Chat] Mckay: Hi");
        assert_eq!(chat_deliveries(&lines), vec![expected]);
        assert!(chat_deliveries(&[]).is_empty());
    }

    #[test]
    fn a_busy_cycle_of_chat_is_split_under_the_udp_limit() {
        // The longest line there is: a 20-letter name and 300 characters.
        let longest = format!("[Chat] {}: {}", "A".repeat(20), "x".repeat(300));
        let lines = vec![longest; 10];
        let packets = chat_deliveries(&lines);
        assert!(packets.len() > 1);
        let mut total = 0;
        for packet in &packets {
            assert!(packet.len() <= MAX_UDP_BYTES);
            assert_eq!(packet[0], 0x38);
            total += packet[1] as usize;
        }
        assert_eq!(total, 10);

        // Empty lines, 4 bytes each: the count's 255 is the limit, not
        // the bytes.
        let empty = vec![String::new(); 300];
        let packets = chat_deliveries(&empty);
        assert_eq!(packets.iter().map(|packet| packet[1] as usize).collect::<Vec<_>>(), vec![255, 45]);
        assert!(packets.iter().all(|packet| packet.len() <= MAX_UDP_BYTES));
    }

    #[test]
    fn a_who_in_bytes() {
        // 03:53:24 is 14,004 seconds after midnight, 0x36B4.  Aldric at
        // 0, 0, 0 for 16 days and 12 minutes (1,383,120 seconds,
        // 0x151AD0); Jacob at 1, 0, -2 (-2 being FE FF FF FF) for 90
        // seconds.
        let who = [WhoEntry { name: "Aldric".to_string(), block: [0, 0, 0], online: 1_383_120 },
                   WhoEntry { name: "Jacob".to_string(), block: [1, 0, -2], online: 90 }];
        let mut expected = vec![0x39, 14, 0, 0, 0, 0xB4, 0x36, 0, 0, 2, 0];
        expected.extend_from_slice(&[6, 0, 0, 0]);
        expected.extend_from_slice(b"Aldric");
        expected.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        expected.extend_from_slice(&[0xD0, 0x1A, 0x15, 0]);
        expected.extend_from_slice(&[5, 0, 0, 0]);
        expected.extend_from_slice(b"Jacob");
        expected.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0, 0xFE, 0xFF, 0xFF, 0xFF]);
        expected.extend_from_slice(&[90, 0, 0, 0]);
        assert_eq!(who_delivery(14, 14_004, &who), expected);

        // Nobody, which a `/who` can't really get: the one asking is in
        // the world.
        assert_eq!(who_delivery(16, 0, &[]), vec![0x39, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn an_answer_that_fits_goes_as_it_is() {
        let small = who_delivery(1, 0, &[]);
        assert_eq!(spans(1, &small), vec![small.clone()]);
        let just_fits = vec![0x39; MAX_UDP_BYTES];
        assert_eq!(spans(1, &just_fits), vec![just_fits.clone()]);
    }

    #[test]
    fn a_big_answer_goes_in_spans_that_put_it_back_together() {
        // Fifty characters with 20-letter names: 50 * 40 bytes and the
        // front, a little over 2,000 bytes, so two Spans.
        let many: Vec<WhoEntry> = (0..50)
            .map(|n| WhoEntry { name: format!("{:A>20}", n), block: [n, 0, -n], online: n as u32 })
            .collect();
        let answer = who_delivery(7, 100, &many);
        let pieces = spans(7, &answer);
        assert_eq!(pieces.len(), 2);

        let mut joined = Vec::new();
        for (index, piece) in pieces.iter().enumerate() {
            assert!(piece.len() <= MAX_UDP_BYTES);
            assert_eq!(&piece[..7], &[0x3A, 7, 0, 0, 0, index as u8 + 1, 2]);
            joined.extend_from_slice(&piece[7..]);
        }
        assert_eq!(joined, answer);

        // Past 255 pieces, nothing.
        assert!(spans(7, &vec![0u8; SPAN_PIECE * 255 + 1]).is_empty());
        assert_eq!(spans(7, &vec![0u8; SPAN_PIECE * 255]).len(), 255);
    }

    /// PROTOCOL.md's worked example: Jacob, object 7, standing at 0.5, 1,
    /// 0.5 facing 90 degrees round, a capsule with no model.
    fn jacob_whole() -> Hydrate {
        Hydrate {
            motion: Motion { object: 7, position: [0.5, 1.0, 0.5], rotation: [0.0, 90.0, 0.0], velocity: [0.0; 3] },
            uuid: "u-1".to_string(),
            living: true,
            short_name: "Jacob".to_string(),
            scale: [1.0; 3],
            model: String::new(),
            shape: 2,
            collider: 1,
            collider_size: [0.5, 2.0, 0.0],
            track: String::new(),
        }
    }

    #[test]
    fn a_hydrate_in_bytes() {
        // 0.5 is 0x3F000000, 1.0 0x3F800000 and 90.0 0x42B40000, lowest
        // byte first.
        let mut expected = vec![0x50, 7, 0, 0, 0];
        expected.extend_from_slice(&[3, 0, 0, 0]);
        expected.extend_from_slice(b"u-1");
        expected.push(1);
        expected.extend_from_slice(&[5, 0, 0, 0]);
        expected.extend_from_slice(b"Jacob");
        expected.extend_from_slice(&[0, 0, 0, 0x3F, 0, 0, 0x80, 0x3F, 0, 0, 0, 0x3F]);
        expected.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0xB4, 0x42, 0, 0, 0, 0]);
        expected.extend_from_slice(&[0; 12]);
        expected.extend_from_slice(&[0, 0, 0x80, 0x3F, 0, 0, 0x80, 0x3F, 0, 0, 0x80, 0x3F]);
        expected.extend_from_slice(&[0, 0, 0, 0]);
        expected.push(2);
        // A capsule collider, 0.5 round (0x3F000000) and 2 tall
        // (0x40000000).
        expected.push(1);
        expected.extend_from_slice(&[0, 0, 0, 0x3F, 0, 0, 0, 0x40, 0, 0, 0, 0]);
        expected.extend_from_slice(&[0, 0, 0, 0]);
        assert_eq!(hydrate(&jacob_whole()), expected);
    }

    #[test]
    fn a_move_reads_back_and_a_pull_back_in_bytes() {
        let mut payload = 12u32.to_le_bytes().to_vec();
        payload.extend_from_slice(&1u32.to_le_bytes());
        put_floats(&mut payload, &[0.5, 1.0, 0.5, 0.0, 90.0, 0.0, 4.0, 0.0, 0.0]);
        assert_eq!(read_player_moved(&payload), Ok(PlayerMove { number: 12, pull_backs_had: 1,
            position: [0.5, 1.0, 0.5], rotation: [0.0, 90.0, 0.0], velocity: [4.0, 0.0, 0.0] }));
        assert!(read_player_moved(&payload[..43]).is_err());
        payload.push(0);
        assert!(read_player_moved(&payload).is_err());

        let pull_back = PullBack { number: 2, position: [0.5, 1.0, 0.5], rotation: [0.0, 90.0, 0.0] };
        let mut expected = vec![0x56, 2, 0, 0, 0];
        expected.extend_from_slice(&[0, 0, 0, 0x3F, 0, 0, 0x80, 0x3F, 0, 0, 0, 0x3F]);
        expected.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0xB4, 0x42, 0, 0, 0, 0]);
        assert_eq!(move_correction(&pull_back), expected);
    }

    #[test]
    fn moves_and_goings_in_bytes_and_split_under_the_udp_limit() {
        let jacob = jacob_whole().motion;
        let moved = objects_moved(&[jacob]);
        assert_eq!(moved.len(), 1);
        assert_eq!(&moved[0][..6], &[0x51, 1, 7, 0, 0, 0]);
        assert_eq!(moved[0].len(), 2 + MOTION_BYTES);
        assert_eq!(&moved[0][6..18], &[0, 0, 0, 0x3F, 0, 0, 0x80, 0x3F, 0, 0, 0, 0x3F]);
        assert!(objects_moved(&[]).is_empty());

        let many = vec![jacob; MOVED_AT_ONCE * 2 + 1];
        let packets = objects_moved(&many);
        assert_eq!(packets.iter().map(|packet| packet[1] as usize).collect::<Vec<_>>(),
                   vec![MOVED_AT_ONCE, MOVED_AT_ONCE, 1]);
        assert!(packets.iter().all(|packet| packet.len() <= MAX_UDP_BYTES));

        assert_eq!(objects_gone(&[7, 300]), vec![vec![0x52, 2, 0, 7, 0, 0, 0, 0x2C, 0x01, 0, 0]]);
        assert!(objects_gone(&[]).is_empty());
        let packets = objects_gone(&vec![1; GONE_AT_ONCE + 1]);
        assert_eq!(packets.len(), 2);
        assert!(packets.iter().all(|packet| packet.len() <= MAX_UDP_BYTES));
    }

    #[test]
    fn a_roll_call_in_pieces_and_knowing_nothing_is_still_said() {
        let jacob = jacob_whole().motion;
        let one = roll_call(3, &[jacob]);
        assert_eq!(one.len(), 1);
        assert_eq!(&one[0][..12], &[0x53, 3, 0, 0, 0, 1, 1, 1, 7, 0, 0, 0]);
        assert_eq!(one[0].len(), ROLL_CALL_HEADER + MOTION_BYTES);

        assert_eq!(roll_call(4, &[]), vec![vec![0x53, 4, 0, 0, 0, 1, 1, 0]]);

        let pieces = roll_call(5, &vec![jacob; ROLL_CALL_AT_ONCE + 1]);
        assert_eq!(pieces.len(), 2);
        assert_eq!((pieces[0][5], pieces[0][6], pieces[0][7]), (1, 2, ROLL_CALL_AT_ONCE as u8));
        assert_eq!((pieces[1][5], pieces[1][6], pieces[1][7]), (2, 2, 1));
        assert!(pieces.iter().all(|piece| piece.len() <= MAX_UDP_BYTES));
    }

    #[test]
    fn an_object_ask_is_one_to_sixty_four_numbers() {
        assert_eq!(read_object_ask(&[2, 7, 0, 0, 0, 0x2C, 0x01, 0, 0]), Ok(vec![7, 300]));
        assert!(read_object_ask(&[]).is_err());
        assert!(read_object_ask(&[0]).is_err());
        assert!(read_object_ask(&[1, 7, 0, 0]).is_err());
        assert!(read_object_ask(&[1, 7, 0, 0, 0, 0]).is_err());
        let mut too_many = vec![ASKS_AT_ONCE + 1];
        too_many.extend(vec![0; (ASKS_AT_ONCE as usize + 1) * 4]);
        assert!(read_object_ask(&too_many).is_err());
    }
}
