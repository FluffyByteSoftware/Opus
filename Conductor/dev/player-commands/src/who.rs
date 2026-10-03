//! File:       Opus/Conductor/dev/player-commands/src/who.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! `/who` (protocol version 13): every character in the world, one line
//! each, EverQuest's way, for the one who asked.  Jacob, 2026-10-02: "a
//! /who that shows all connected players", characters in the world only.
//! 2026-10-03, since the chat window is scaleable: one line a character,
//! with where it stands and how long it's been in the world, the one in
//! longest at the top ("Chatter is at [0, 0, 0] [16 days, 12 minutes
//! online]").  `/who list` went into `/who` ("just mutate them into who").
//!
//! The answer is a WhoDelivery: the names, their blocks and their seconds
//! online, and the time it ran as seconds since midnight UTC.  The client
//! writes the lines, the count and the stamp under them in the player's
//! own time zone.  So nothing here is laid out for the eye.
//!
//! Where each character stands and when it came in are the GameClock's,
//! which only its thread may read, so the ask is left in the GameClock's
//! mailbox and answered from its next broadcast check through `send()`,
//! a cycle later.

use conductor_gameclock::{Standing, WhoAsked};
use conductor_networking::protocol::{self, WhoEntry};
use conductor_networking::typed::{Asker, Outcome};
use conductor_tools::clock::Utc;
use conductor_tools::scribe::{self, Channel};

const ONLY_WHO: &str = "Try /who.";
const UNAVAILABLE: &str = "Who Unavailable";

/// A `/who`, with whatever came after the word.  `/who` alone is answered
/// a cycle from now, anything after it refused.
pub fn run(asker: &Asker, rest: &str) -> Outcome {
    let (account, ask) = (asker.account, asker.ask);
    if !rest.trim_matches(' ').is_empty() {
        return Outcome::Answer(protocol::command_refused(ask, ONLY_WHO));
    }
    let asked = WhoAsked { from: asker.from, account: account.to_string(), ask };
    match conductor_gameclock::who(asked) {
        Ok(()) => Outcome::Later,
        Err(why) => {
            scribe::debug(Channel::Game, &format!("{account}'s /who wasn't asked: {why}."));
            Outcome::Answer(protocol::command_refused(ask, UNAVAILABLE))
        }
    }
}

/// Answers a `/who` with everybody in the world, in the GameClock's order
/// (the one in longest first).  The GameClock calls this from its
/// broadcast check, on its own thread; `wire()` hands it over
/// (`conductor_gameclock::set_who_sender()`).  Nothing goes out if the
/// one who asked has left since.
pub fn send(asked: &WhoAsked, standing: &[Standing]) {
    let entries: Vec<WhoEntry> = standing.iter().map(entry).collect();
    let answer = protocol::who_delivery(asked.ask, seconds_since_midnight(), &entries);
    if conductor_networking::finish_ask(asked.from, &asked.account, asked.ask, &answer) {
        conductor_networking::tell_answer(asked.from, asked.ask, &answer);
    }
}

/// One character as the packet carries it: its time online in whole
/// seconds, which a u32 holds for 136 years.
fn entry(character: &Standing) -> WhoEntry {
    let online = u32::try_from(character.online.as_secs()).unwrap_or(u32::MAX);
    WhoEntry { name: character.name.clone(), block: character.block, online }
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
    use std::time::Duration;

    #[test]
    fn the_time_is_inside_one_day() {
        assert!(seconds_since_midnight() < 86_400);
    }

    #[test]
    fn anything_after_who_is_refused() {
        let asker = Asker { from: "10.0.0.5:50000".parse().unwrap(), account: "jacob", character: "Jacob", ask: 4 };
        let refused = protocol::command_refused(4, ONLY_WHO);
        assert_eq!(run(&asker, "everybody"), Outcome::Answer(refused.clone()));
        assert_eq!(run(&asker, "list"), Outcome::Answer(refused));
    }

    #[test]
    fn time_online_goes_as_whole_seconds() {
        let chatter = Standing { name: "Chatter".to_string(), block: [0, 0, 0],
                                 online: Duration::from_millis(1_383_120_900) };
        assert_eq!(entry(&chatter), WhoEntry { name: "Chatter".to_string(), block: [0, 0, 0], online: 1_383_120 });
    }
}
