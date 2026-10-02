//! File:       Opus/Conductor/dev/networking/src/commands/who.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! `/who` and `/who list` (protocol version 9): every character in the
//! world, A to Z, for the one who asked.  Jacob, 2026-10-02: "a /who that
//! shows all connected players", characters in the world only.  `/who` is
//! the names; `/who list` is each with the block it stands in ("if they
//! do /who list It shows [Aldric] is currently at [0, 0, 0]").
//!
//! The answer is a WhoDelivery: the names (and blocks), and the time it
//! ran as seconds since midnight UTC.  The client draws the box around
//! it, his old MUD's, in the player's own time zone and to the width of
//! its own chat box, and writes the count out in words.  So nothing here
//! is laid out for the eye.
//!
//! `/who` is names, which the book has, so it's answered on the spot.
//! `/who list` needs where each character stands, which only the
//! GameClock's thread may read, so it's left in the GameClock's mailbox
//! and answered from its next broadcast check through `send_list()`, a
//! cycle later.

use conductor_gameclock::{Standing, WhoAsked};
use conductor_tools::clock::Utc;
use conductor_tools::scribe::{self, Channel};

use super::{Asker, Outcome};
use crate::protocol::{self, WhoEntry};
use crate::sessions;
use crate::udp;

const ONLY_WHO: &str = "Try /who, or /who list.";
const UNAVAILABLE: &str = "Who Unavailable";

/// A `/who`, with whatever came after the word.  `/who` is answered now,
/// `/who list` a cycle from now, anything else refused.
pub fn run(asker: &Asker, rest: &str) -> Outcome {
    let (account, ask) = (asker.account, asker.ask);
    let rest = rest.trim_matches(' ');
    if rest.is_empty() {
        let entries: Vec<WhoEntry> = sorted(sessions::names_in_world()).into_iter()
            .map(|name| WhoEntry { name, block: None })
            .collect();
        return Outcome::Answer(protocol::who_delivery(ask, seconds_since_midnight(), false, &entries));
    }
    if rest.eq_ignore_ascii_case("list") {
        let asked = WhoAsked { from: asker.from, account: account.to_string(), ask };
        return match conductor_gameclock::who_list(asked) {
            Ok(()) => Outcome::Later,
            Err(why) => {
                scribe::debug(Channel::Game, &format!("{account}'s /who list wasn't asked: {why}."));
                Outcome::Answer(protocol::command_refused(ask, UNAVAILABLE))
            }
        };
    }
    Outcome::Answer(protocol::command_refused(ask, ONLY_WHO))
}

/// Answers a `/who list` with where everybody stands.  The GameClock calls
/// this from its broadcast check, on its own thread; networking hands it
/// over as it starts (`conductor_gameclock::set_who_sender()`).  Nothing
/// goes out if the one who asked has left since.
pub fn send_list(asked: &WhoAsked, standing: &[Standing]) {
    let mut entries: Vec<WhoEntry> = standing.iter()
        .map(|character| WhoEntry { name: character.name.clone(), block: Some(character.block) })
        .collect();
    entries.sort_by_key(|entry| entry.name.to_ascii_lowercase());
    let answer = protocol::who_delivery(asked.ask, seconds_since_midnight(), true, &entries);
    if sessions::finish_ask(asked.from, &asked.account, asked.ask, &answer) {
        udp::tell_answer(asked.from, asked.ask, &answer);
    }
}

/// Names A to Z, whatever the capitals.
fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort_by_key(|name| name.to_ascii_lowercase());
    names
}

/// The seconds since midnight UTC, right now: 0 to 86,399.
fn seconds_since_midnight() -> u32 {
    let now = Utc::now();
    now.hour * 3_600 + now.minute * 60 + now.second
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_go_a_to_z_whatever_the_capitals() {
        let names = vec!["Zeleya".to_string(), "aldric".to_string(), "Bujin".to_string()];
        assert_eq!(sorted(names), vec!["aldric".to_string(), "Bujin".to_string(), "Zeleya".to_string()]);
    }

    #[test]
    fn the_time_is_inside_one_day() {
        assert!(seconds_since_midnight() < 86_400);
    }

    #[test]
    fn anything_but_who_or_who_list_is_refused() {
        let asker = Asker { from: "10.0.0.5:50000".parse().unwrap(), account: "jacob", character: "Jacob", ask: 4 };
        let refused = protocol::command_refused(4, ONLY_WHO);
        assert_eq!(run(&asker, "everybody"), Outcome::Answer(refused.clone()));
        assert_eq!(run(&asker, "list please"), Outcome::Answer(refused));
    }
}
