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
//! BinaryWriter and BinaryReader do and the client will most likely be C#.
//! A string is a u32 byte count and then that many bytes of UTF-8.
//!
//! Nothing in here logs or touches the network.  It turns packets into
//! bytes and bytes back into packets, which is what lets the tests run.
//! The answers that never change (`hello()`, `keep_alive()`, the results)
//! are built once by the callers and sent as they are.

/// Which protocol this is.  The Hello says it, so a client built against
/// a different one can stop right there.  Goes up when a packet changes.
pub const PROTOCOL_VERSION: u8 = 4;

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

/// Every packet type there is.  The high four bits say the group and the
/// low four which one in it: 0x1_ is the login, over TCP, and 0x3_ is the
/// game, over UDP.  0x2_ is kept free for whatever goes between them one
/// day (a character select, say).
// Rust note: `repr(u8)` stores the enum as one byte, and `as u8` turns a
// value back into its number, the same as a C# `enum : byte`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketType {
    /// Server to client, right after TLS.  One byte: PROTOCOL_VERSION.
    Hello = 0x10,
    /// Client to server.  Four strings: the client's version, the secret
    /// word, the username, the password.
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
            0x30 => Some(PacketType::Connect),
            0x31 => Some(PacketType::ConnectResult),
            0x32 => Some(PacketType::KeepAlive),
            0x33 => Some(PacketType::Goodbye),
            0x34 => Some(PacketType::Kicked),
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
    /// is down.  Nothing the player did.
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
    /// STOP SERVER on the Control Panel.
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

/// What a Login says.  No `Debug` on purpose: it holds a password, and a
/// `{:?}` in a log line would print it.
pub struct LoginRequest {
    pub client_version: String,
    pub secret_word: String,
    pub username: String,
    pub password: String,
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

// ---------------------------------------------------------------------------
// The packets the server reads
// ---------------------------------------------------------------------------

/// The payload of a Login: the client's version, the secret word, the
/// username, then the password.
pub fn read_login(payload: &[u8]) -> Result<LoginRequest, String> {
    let mut at = 0;
    let client_version = take_string(payload, &mut at)?;
    let secret_word = take_string(payload, &mut at)?;
    let username = take_string(payload, &mut at)?;
    let password = take_string(payload, &mut at)?;
    finished(payload, at)?;
    Ok(LoginRequest { client_version, secret_word, username, password })
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
    }

    #[test]
    fn a_login_reads_back() {
        let payload = strings(&["0.0.1", "potato", "jacob", "Correct horse 1!"]);
        let login = read_login(&payload).unwrap();
        assert_eq!(login.client_version, "0.0.1");
        assert_eq!(login.secret_word, "potato");
        assert_eq!(login.username, "jacob");
        assert_eq!(login.password, "Correct horse 1!");
    }

    #[test]
    fn broken_logins_are_refused() {
        let payload = strings(&["0.0.1", "potato", "jacob", "Correct horse 1!"]);

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
                     PacketType::KeepAlive, PacketType::Goodbye, PacketType::Kicked];
        for kind in every {
            assert_eq!(PacketType::from_byte(kind as u8), Some(kind));
        }
        assert_eq!(PacketType::from_byte(0x00), None);
        assert_eq!(PacketType::from_byte(0x16), None);
        assert_eq!(PacketType::from_byte(0x20), None);
        assert_eq!(PacketType::from_byte(0x35), None);
    }
}
