//! File:       Opus/Conductor/dev/networking/src/commands.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! What a player types.  The client sends the line as it was typed, in a
//! PlayerCommand (protocol version 8), and the server goes "oh hey that
//! started with / that means look for a command" (Jacob, 2026-10-02).
//! There are two so far:
//!
//! ```text
//! /chat Yo yo yo!
//! /who            /who list
//! ```
//!
//! `/chat` is here: everybody in the world sees `[Chat] Jacob: Yo yo yo!`,
//! the one who said it too, one fixed channel, "like the way the old shit
//! muds did it".  The line is made here, on the UDP thread, and left in
//! the GameClock's chat mailbox; the GameClock's broadcast check hands
//! each cycle's lines back to `send_out()`, which sends them.  `/who` is
//! in `who.rs`.
//!
//! A chat message is plain English (Jacob: "letters numbers special
//! characters, spaces"), printable ASCII, and anything past its 300th
//! character is dropped without a word ("The server will just ignore
//! everything after 300").  The client stops at 300 itself.  A line
//! without a `/` will be said out loud, nearby, once there's somewhere to
//! be near; until then it's refused.

use std::net::SocketAddr;

use conductor_tools::scribe::{self, Channel};

use crate::protocol;
use crate::sessions;
use crate::udp;
use crate::who;

/// The most characters of a chat message that are kept.  Jacob's 300.
pub const MOST_CHARACTERS: usize = 300;

const NOT_A_COMMAND: &str = "Saying things without a command isn't in yet.  Use /chat.";
const NO_SUCH_COMMAND: &str = "There's no command by that name.  For now there's /chat and /who.";
const NOTHING_SAID: &str = "Say something after /chat.";
const NOT_PLAIN: &str = "Chat is plain English: letters, numbers, punctuation and spaces.";
const UNAVAILABLE: &str = "Chat Unavailable";

/// What became of a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The answer, to keep in the book and send.
    Answer(Vec<u8>),
    /// The GameClock answers it, a cycle from now (`/who list`).  The ask
    /// stays open in the book until then.
    Later,
}

/// A command, read off a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command<'a> {
    /// `/chat`, and what came after it.
    Chat(&'a str),
    /// `/who`, and what came after it.
    Who(&'a str),
}

/// A line typed by a player whose character is in the world (the caller
/// turns away the rest), as the ask numbered `ask`.
pub fn command(from: SocketAddr, account: &str, character: &str, ask: u32, line: &str) -> Outcome {
    match read(line) {
        Ok(Command::Chat(message)) => match chat(account, character, message) {
            Ok(()) => Outcome::Answer(protocol::command_accepted(ask)),
            Err(why) => Outcome::Answer(protocol::command_refused(ask, why)),
        },
        Ok(Command::Who(rest)) => who::ask(from, account, ask, rest),
        Err(why) => Outcome::Answer(protocol::command_refused(ask, why)),
    }
}

/// Sends a cycle's chat to everybody in the world.  The GameClock calls
/// this from its broadcast check, on its own thread; networking hands it
/// over as it starts (`conductor_gameclock::set_chat_sender()`).
pub fn send_out(lines: &[String]) {
    let packets = protocol::chat_deliveries(lines);
    udp::tell_all(&sessions::in_world(), &packets);
}

/// Which command a line is, and what came after its word.
fn read(line: &str) -> Result<Command<'_>, &'static str> {
    let Some(command) = line.strip_prefix('/') else {
        return Err(NOT_A_COMMAND);
    };
    // Rust note: `split_once` cuts at the first space, if there is one:
    // the word before it is the command, the rest is what it says.
    let (word, rest) = command.split_once(' ').unwrap_or((command, ""));
    if word.eq_ignore_ascii_case("chat") {
        Ok(Command::Chat(rest))
    } else if word.eq_ignore_ascii_case("who") {
        Ok(Command::Who(rest))
    } else {
        Err(NO_SUCH_COMMAND)
    }
}

/// A `/chat`: the finished line into the GameClock's chat mailbox.
fn chat(account: &str, character: &str, message: &str) -> Result<(), &'static str> {
    let finished = chat_line(character, message)?;
    conductor_gameclock::chat(finished.clone()).map_err(|why| {
        scribe::debug(Channel::Game, &format!("{account}'s chat as {character} didn't go out: {why}."));
        UNAVAILABLE
    })?;
    scribe::debug(Channel::Game, &format!("Chat, from {account}: {finished}"));
    Ok(())
}

/// The finished line a chat message makes, or the words for why it
/// doesn't make one.
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
    fn a_line_is_read_as_its_command() {
        assert_eq!(read("/chat Yo yo yo!"), Ok(Command::Chat("Yo yo yo!")));
        assert_eq!(read("/CHAT   Yo"), Ok(Command::Chat("  Yo")));
        assert_eq!(read("/chat"), Ok(Command::Chat("")));
        assert_eq!(read("/who"), Ok(Command::Who("")));
        assert_eq!(read("/Who list"), Ok(Command::Who("list")));
        assert_eq!(read("Yo yo yo!"), Err(NOT_A_COMMAND));
        assert_eq!(read(""), Err(NOT_A_COMMAND));
        assert_eq!(read("/shout Yo"), Err(NO_SUCH_COMMAND));
        assert_eq!(read("/chatter Yo"), Err(NO_SUCH_COMMAND));
        assert_eq!(read("/"), Err(NO_SUCH_COMMAND));
    }

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
