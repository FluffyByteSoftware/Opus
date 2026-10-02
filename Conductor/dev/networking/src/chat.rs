//! File:       Opus/Conductor/dev/networking/src/chat.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! What a player types, and the chat it makes.  The client sends the line
//! as it was typed, in a PlayerCommand (protocol version 8), and the
//! server goes "oh hey that started with / that means look for a command"
//! (Jacob, 2026-10-02).  There's one command so far, EverQuest's:
//!
//! ```text
//! /chat Yo yo yo!
//! ```
//!
//! which everybody in the world sees as `[Chat] Jacob: Yo yo yo!`, the one
//! who said it too.  One fixed channel, "like the way the old shit muds
//! did it".  The line is made here, on the UDP thread, and left in the
//! GameClock's chat mailbox; the GameClock's broadcast check hands each
//! cycle's lines back to `send_out()`, which sends them.
//!
//! A message is plain English (Jacob: "letters numbers special
//! characters, spaces"), printable ASCII, and anything past its 300th
//! character is dropped without a word ("The server will just ignore
//! everything after 300").  The client stops at 300 itself.  A line
//! without a `/` will be said out loud, nearby, once there's somewhere to
//! be near; until then it's refused.

use conductor_tools::scribe::{self, Channel};

use crate::protocol;
use crate::sessions;
use crate::udp;

/// The most characters of a message that are kept.  Jacob's 300.
pub const MOST_CHARACTERS: usize = 300;

const NOT_A_COMMAND: &str = "Saying things without a command isn't in yet.  Use /chat.";
const NO_SUCH_COMMAND: &str = "There's no command by that name.  For now there's only /chat.";
const NOTHING_SAID: &str = "Say something after /chat.";
const NOT_PLAIN: &str = "Chat is plain English: letters, numbers, punctuation and spaces.";
const UNAVAILABLE: &str = "Chat Unavailable";

/// What a player's character said.  For players in the world only; the
/// caller turns away the rest.  Ok when it went in the GameClock's
/// mailbox; an Err is the words for the player, for a CommandRefused.
pub fn command(account: &str, character: &str, line: &str) -> Result<(), &'static str> {
    let finished = read(character, line)?;
    conductor_gameclock::chat(finished.clone()).map_err(|why| {
        scribe::debug(Channel::Game, &format!("{account}'s chat as {character} didn't go out: {why}."));
        UNAVAILABLE
    })?;
    scribe::debug(Channel::Game, &format!("Chat, from {account}: {finished}"));
    Ok(())
}

/// Sends a cycle's lines to everybody in the world.  The GameClock calls
/// this from its broadcast check, on its own thread; networking hands it
/// over as it starts (`conductor_gameclock::set_chat_sender()`).
pub fn send_out(lines: &[String]) {
    let packets = protocol::chat_deliveries(lines);
    udp::tell_all(&sessions::in_world(), &packets);
}

/// The finished line a player's typed line makes, or the words for why
/// it doesn't make one.
fn read(character: &str, line: &str) -> Result<String, &'static str> {
    let Some(command) = line.strip_prefix('/') else {
        return Err(NOT_A_COMMAND);
    };
    // Rust note: `split_once` cuts at the first space, if there is one:
    // the word before it is the command, the rest is what it says.
    let (word, rest) = command.split_once(' ').unwrap_or((command, ""));
    if !word.eq_ignore_ascii_case("chat") {
        return Err(NO_SUCH_COMMAND);
    }

    let message: String = rest.trim_start_matches(' ').chars().take(MOST_CHARACTERS).collect();
    if !message.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
        return Err(NOT_PLAIN);
    }
    let message = message.trim_end_matches(' ');
    if message.is_empty() {
        return Err(NOTHING_SAID);
    }
    Ok(format!("[Chat] {character}: {message}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chat_makes_jacobs_line() {
        assert_eq!(read("Jacob", "/chat Yo yo yo!"), Ok("[Chat] Jacob: Yo yo yo!".to_string()));
        // Any capitals, and the spaces around the message don't count.
        assert_eq!(read("Jacob", "/CHAT   Yo yo yo!  "), Ok("[Chat] Jacob: Yo yo yo!".to_string()));
        // Every printable thing on an English keyboard.
        let keyboard = "~`!@#$%^&*()_+-={}[]|\\:;\"'<>,.?/ 0123456789 AZaz";
        assert_eq!(read("Jacob", &format!("/chat {keyboard}")), Ok(format!("[Chat] Jacob: {keyboard}")));
    }

    #[test]
    fn everything_after_300_characters_is_dropped() {
        let long = "x".repeat(MOST_CHARACTERS + 50);
        let line = read("Jacob", &format!("/chat {long}")).unwrap();
        assert_eq!(line, format!("[Chat] Jacob: {}", "x".repeat(MOST_CHARACTERS)));
        // What's past the 300th isn't looked at, so it can't refuse the
        // line either.
        let line = read("Jacob", &format!("/chat {}é", "x".repeat(MOST_CHARACTERS))).unwrap();
        assert_eq!(line, format!("[Chat] Jacob: {}", "x".repeat(MOST_CHARACTERS)));
    }

    #[test]
    fn what_chat_turns_away() {
        assert_eq!(read("Jacob", "Yo yo yo!"), Err(NOT_A_COMMAND));
        assert_eq!(read("Jacob", ""), Err(NOT_A_COMMAND));
        assert_eq!(read("Jacob", "/shout Yo"), Err(NO_SUCH_COMMAND));
        assert_eq!(read("Jacob", "/chatter Yo"), Err(NO_SUCH_COMMAND));
        assert_eq!(read("Jacob", "/"), Err(NO_SUCH_COMMAND));
        assert_eq!(read("Jacob", "/chat"), Err(NOTHING_SAID));
        assert_eq!(read("Jacob", "/chat     "), Err(NOTHING_SAID));
        assert_eq!(read("Jacob", "/chat héllo"), Err(NOT_PLAIN));
        assert_eq!(read("Jacob", "/chat a\tb"), Err(NOT_PLAIN));
        assert_eq!(read("Jacob", "/chat new\nline"), Err(NOT_PLAIN));
    }
}
