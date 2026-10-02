//! File:       Opus/Conductor/dev/networking/src/commands/chat.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! `/chat <message>`, EverQuest's (protocol version 8): everybody in the
//! world sees `[Chat] Jacob: Yo yo yo!`, the one who said it too, one
//! fixed channel, "like the way the old shit muds did it" (Jacob,
//! 2026-10-02).  The line is made here, on the UDP thread, and left in
//! the GameClock's chat mailbox; the GameClock's broadcast check hands
//! each cycle's lines back to `send_out()`, which sends them.
//!
//! A message is plain English (Jacob: "letters numbers special
//! characters, spaces"), printable ASCII, and anything past its 300th
//! character is dropped without a word ("The server will just ignore
//! everything after 300").  The client stops at 300 itself.

use conductor_tools::scribe::{self, Channel};

use super::{Asker, Outcome};
use crate::protocol;
use crate::sessions;
use crate::udp;

/// The most characters of a message that are kept.  Jacob's 300.
pub const MOST_CHARACTERS: usize = 300;

const NOTHING_SAID: &str = "Say something after /chat.";
const NOT_PLAIN: &str = "Chat is plain English: letters, numbers, punctuation and spaces.";
const UNAVAILABLE: &str = "Chat Unavailable";

/// A `/chat`, with whatever came after the word: the finished line into
/// the GameClock's chat mailbox, and a CommandAccepted, or a
/// CommandRefused saying why not.
pub fn run(asker: &Asker, message: &str) -> Outcome {
    let answer = match chat(asker.account, asker.character, message) {
        Ok(()) => protocol::command_accepted(asker.ask),
        Err(why) => protocol::command_refused(asker.ask, why),
    };
    Outcome::Answer(answer)
}

/// Sends a cycle's chat to everybody in the world.  The GameClock calls
/// this from its broadcast check, on its own thread; networking hands it
/// over as it starts (`conductor_gameclock::set_chat_sender()`).
pub fn send_out(lines: &[String]) {
    let packets = protocol::chat_deliveries(lines);
    udp::tell_all(&sessions::in_world(), &packets);
}

fn chat(account: &str, character: &str, message: &str) -> Result<(), &'static str> {
    let finished = chat_line(character, message)?;
    conductor_gameclock::chat(finished.clone()).map_err(|why| {
        scribe::debug(Channel::Game, &format!("{account}'s chat as {character} didn't go out: {why}."));
        UNAVAILABLE
    })?;
    scribe::debug(Channel::Game, &format!("Chat, from {account}: {finished}"));
    Ok(())
}

/// The finished line a message makes, or the words for why it doesn't
/// make one.
fn chat_line(character: &str, message: &str) -> Result<String, &'static str> {
    let message: String = message.trim_start_matches(' ').chars().take(MOST_CHARACTERS).collect();
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
        assert_eq!(chat_line("Jacob", "Yo yo yo!"), Ok("[Chat] Jacob: Yo yo yo!".to_string()));
        // The spaces around the message don't count.
        assert_eq!(chat_line("Jacob", "   Yo yo yo!  "), Ok("[Chat] Jacob: Yo yo yo!".to_string()));
        // Every printable thing on an English keyboard.
        let keyboard = "~`!@#$%^&*()_+-={}[]|\\:;\"'<>,.?/ 0123456789 AZaz";
        assert_eq!(chat_line("Jacob", keyboard), Ok(format!("[Chat] Jacob: {keyboard}")));
    }

    #[test]
    fn everything_after_300_characters_is_dropped() {
        let long = "x".repeat(MOST_CHARACTERS + 50);
        let line = chat_line("Jacob", &long).unwrap();
        assert_eq!(line, format!("[Chat] Jacob: {}", "x".repeat(MOST_CHARACTERS)));
        // What's past the 300th isn't looked at, so it can't refuse the
        // line either.
        let line = chat_line("Jacob", &format!("{}é", "x".repeat(MOST_CHARACTERS))).unwrap();
        assert_eq!(line, format!("[Chat] Jacob: {}", "x".repeat(MOST_CHARACTERS)));
    }

    #[test]
    fn what_chat_turns_away() {
        assert_eq!(chat_line("Jacob", ""), Err(NOTHING_SAID));
        assert_eq!(chat_line("Jacob", "     "), Err(NOTHING_SAID));
        assert_eq!(chat_line("Jacob", "héllo"), Err(NOT_PLAIN));
        assert_eq!(chat_line("Jacob", "a\tb"), Err(NOT_PLAIN));
        assert_eq!(chat_line("Jacob", "new\nline"), Err(NOT_PLAIN));
    }
}
