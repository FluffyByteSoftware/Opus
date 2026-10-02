//! File:       Opus/Conductor/dev/networking/src/commands.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! What a player types.  The client sends the line as it was typed, in a
//! PlayerCommand (protocol version 8), and the server goes "oh hey that
//! started with / that means look for a command" (Jacob, 2026-10-02).
//!
//! Every command is one line in `COMMANDS`: its name, how long the player
//! waits after it before the next, and the function that runs it, in a
//! file of its own under `commands/`.  A new command is a new file and a
//! new line there, and nothing else changes.  The same shape as the
//! GameClock's checks, and Jacob's ask: "make a command interface and
//! then make it so we could easily stuff new commands in".
//!
//! **Anti-flood** (Jacob, 2026-10-02: "anti flood prevention on the
//! server for any chat commands"): after a command goes through, the
//! player waits that command's `wait` before the next one, whichever it
//! is.  The default is 500 ms, two game cycles ("maybe two full game
//! ticks?  So 500 ms?"); one that costs more takes longer ("if we make a
//! command that hits the database a bunch maybe that needs longer").  A
//! line too soon is refused before it's looked at any further, so a flood
//! never builds a list, reaches the GameClock or goes out on the wire,
//! and the refusal doesn't restart the wait.  A line without a `/`, or a
//! command there's no such thing as, waits the default too.  A resend of
//! the same ask never gets here: the book answers it with the answer it
//! kept.
//!
//! A line without a `/` will be said out loud, nearby, once there's
//! somewhere to be near; until then it's refused.

mod chat;
mod who;

use std::net::SocketAddr;
use std::time::Duration;

use crate::protocol;
use crate::sessions;

// Rust note: these hand the two functions the GameClock calls on to
// lib.rs, which gives them to it as networking starts.
pub use chat::send_out;
pub use who::send_list;

/// How long a player waits after a command unless the command says
/// otherwise.  Jacob's two game cycles.
pub const DEFAULT_WAIT: Duration = Duration::from_millis(500);

const NOT_A_COMMAND: &str = "Saying things without a command isn't in yet.  Use /chat.";
const NO_SUCH_COMMAND: &str = "There's no command by that name.  For now there's /chat and /who.";
const TOO_SOON: &str = "You can't do that again so soon.";

/// Who typed the line: where from, their account and character, and the
/// ask number it came in as.
pub struct Asker<'a> {
    pub from: SocketAddr,
    pub account: &'a str,
    pub character: &'a str,
    pub ask: u32,
}

/// What became of a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The answer, to keep in the book and send.
    Answer(Vec<u8>),
    /// The GameClock answers it, a cycle from now (`/who list`).  The ask
    /// stays open in the book until then.
    Later,
}

/// One command.
struct Command {
    /// The word after the `/`, lowercase.  Matched whatever the capitals.
    name: &'static str,
    /// How long the player waits after it before the next command.
    wait: Duration,
    // Rust note: a plain function, like the checks' `run`.  It gets who
    // asked and whatever came after the word.
    run: fn(&Asker<'_>, &str) -> Outcome,
}

/// Every command there is.
static COMMANDS: [Command; 2] = [
    Command { name: "chat", wait: DEFAULT_WAIT, run: chat::run },
    // Jacob's second, from before the anti-flood was for every command:
    // "a temporary cooldown of like 1 second".
    Command { name: "who", wait: Duration::from_secs(1), run: who::run },
];

/// A line typed by a player whose character is in the world (the caller
/// turns away the rest).
pub fn command(asker: &Asker, line: &str) -> Outcome {
    let (word, rest) = split(line);
    let found = word.and_then(find);
    let wait = found.map_or(DEFAULT_WAIT, |command| command.wait);
    if !sessions::may_command(asker.from, wait) {
        return Outcome::Answer(protocol::command_refused(asker.ask, TOO_SOON));
    }
    match (word, found) {
        (None, _) => Outcome::Answer(protocol::command_refused(asker.ask, NOT_A_COMMAND)),
        (Some(_), None) => Outcome::Answer(protocol::command_refused(asker.ask, NO_SUCH_COMMAND)),
        (Some(_), Some(command)) => (command.run)(asker, rest),
    }
}

/// A line's command word, if it starts with a `/`, and whatever came after
/// the first space.
fn split(line: &str) -> (Option<&str>, &str) {
    let Some(command) = line.strip_prefix('/') else {
        return (None, line);
    };
    // Rust note: `split_once` cuts at the first space, if there is one:
    // the word before it is the command, the rest is what it says.
    let (word, rest) = command.split_once(' ').unwrap_or((command, ""));
    (Some(word), rest)
}

/// The command a word names, whatever the capitals.
fn find(word: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.name.eq_ignore_ascii_case(word))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_splits_into_its_word_and_the_rest() {
        assert_eq!(split("/chat Yo yo yo!"), (Some("chat"), "Yo yo yo!"));
        assert_eq!(split("/CHAT   Yo"), (Some("CHAT"), "  Yo"));
        assert_eq!(split("/chat"), (Some("chat"), ""));
        assert_eq!(split("/Who list"), (Some("Who"), "list"));
        assert_eq!(split("/"), (Some(""), ""));
        assert_eq!(split("Yo yo yo!"), (None, "Yo yo yo!"));
        assert_eq!(split(""), (None, ""));
    }

    #[test]
    fn a_word_finds_its_command_whatever_the_capitals() {
        assert_eq!(find("chat").map(|command| command.name), Some("chat"));
        assert_eq!(find("WHO").map(|command| command.name), Some("who"));
        assert!(find("shout").is_none());
        assert!(find("chatter").is_none());
        assert!(find("").is_none());
    }

    #[test]
    fn every_command_has_its_own_lowercase_name_and_waits_at_least_the_default() {
        for (number, command) in COMMANDS.iter().enumerate() {
            assert_eq!(command.name, command.name.to_ascii_lowercase());
            assert!(command.wait >= DEFAULT_WAIT, "{} waits less than the default", command.name);
            assert!(COMMANDS[number + 1..].iter().all(|other| other.name != command.name));
        }
    }
}
