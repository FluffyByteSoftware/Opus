//! File:       Opus/Conductor/dev/networking/src/view.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! What each player sees of the world, sent (protocol version 14).  The
//! GameClock works out, once a cycle, what each player is to be told:
//! objects come into view whole, objects moved, objects gone, and once a
//! second a roll call (`conductor_gameclock`'s `view.rs` has why).  It
//! hands that here as plain data, through the slot `start()` fills with
//! `send_out()`, since the GameClock can't call networking itself; this
//! makes the packets and sends them, on the GameClock's thread.
//!
//! An ObjectAsk (the client doesn't know a number on a roll call) comes
//! in on the UDP thread and goes to the GameClock's mailbox
//! (`conductor_gameclock::ask_about()`); the next broadcast answers it.
//!
//! Since movement (protocol version 15) a PlayerMoved, the client saying
//! where it walked its own character, comes in here too and goes to the
//! GameClock's mailbox (`conductor_gameclock::moved()`), stamped with
//! when it came; the next input check judges it.  A pull-back goes out
//! with the player's news as a MoveCorrection, and the column their
//! character is in, when it changes, goes into the book for the chunks
//! they may have.

use std::net::SocketAddr;
use std::time::Instant;

use conductor_gameclock::{Moved, News};
use conductor_tools::scribe::{self, Channel};

use crate::protocol::{self, PlayerMove};
use crate::sessions;
use crate::udp;

/// Hands the GameClock the function that sends each player their news.
/// Networking's `start()` calls this, every START SERVER.
pub fn wire() {
    conductor_gameclock::set_view_sender(send_out);
}

/// A cycle's news, sent: each player's packets to their address, and
/// their column into the book when it's changed.  A player not in the
/// book any more (they left this very cycle) is skipped; the GameClock
/// forgets them next cycle.
fn send_out(news: &[News]) {
    let addresses = sessions::in_world_by_character();
    for one in news {
        let Some(&address) = addresses.get(&one.viewer) else {
            continue;
        };
        if let Some(column) = one.standing {
            sessions::stands_in(address, column);
        }
        udp::tell_all(&[address], &packets(one, address));
    }
}

/// One player's news as the packets that carry it: a pull-back first, so
/// the client is back where it should be before anything else lands; the
/// objects gone, then the ones come whole, the moves, and the roll call
/// last, so it lands after everything else this cycle said.
fn packets(news: &News, to: SocketAddr) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    if let Some(pull_back) = &news.pull_back {
        packets.push(protocol::move_correction(pull_back));
    }
    packets.extend(protocol::objects_gone(&news.gone));
    for object in &news.hydrates {
        let packet = protocol::hydrate(object);
        // Only a uuid, a name and three short strings: nothing we make
        // comes near the limit.  One that somehow did would be dropped on
        // the way, so it isn't sent at all.
        if packet.len() > protocol::MAX_UDP_BYTES {
            scribe::debug(Channel::Network, &format!("Object {} is {} bytes whole, too big for one packet.  \
                Not sent to {to}.", object.motion.object, packet.len()));
            continue;
        }
        packets.push(packet);
    }
    packets.extend(protocol::objects_moved(&news.moved));
    if let Some((roll, present)) = &news.roll_call {
        packets.extend(protocol::roll_call(*roll, present));
    }
    packets
}

/// An ObjectAsk from `from`: the numbers go to the GameClock, if their
/// character is in the world.  A stranger, or a player at character
/// select, hears nothing.
pub fn asked(from: SocketAddr, objects: Vec<u32>) {
    let Some(character_id) = sessions::character_in_world(from) else {
        return;
    };
    if let Err(why) = conductor_gameclock::ask_about(character_id, objects) {
        scribe::debug(Channel::Network, &format!("An object ask from {from} went nowhere: {why}."));
    }
}

/// A PlayerMoved from `from`: the move goes to the GameClock, stamped with
/// now, if their character is in the world.  A stranger, or a player at
/// character select, hears nothing.
pub fn moved(from: SocketAddr, walked: PlayerMove) {
    let heard = Instant::now();
    let Some(character_id) = sessions::character_in_world(from) else {
        return;
    };
    let moved = Moved { character_id, number: walked.number, pull_backs_had: walked.pull_backs_had,
                        position: walked.position, rotation: walked.rotation, velocity: walked.velocity, heard };
    if let Err(why) = conductor_gameclock::moved(moved) {
        scribe::debug(Channel::Network, &format!("A move from {from} went nowhere: {why}."));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use conductor_gameclock::{Motion, PullBack};

    #[test]
    fn a_pull_back_first_and_the_roll_call_last() {
        let motion = Motion { object: 3, position: [0.5, 1.0, 0.5], rotation: [0.0; 3], velocity: [0.0; 3] };
        let pull_back = PullBack { number: 1, position: [0.5, 1.0, 0.5], rotation: [0.0; 3] };
        let news = News { viewer: 42, moved: vec![motion], gone: vec![9], roll_call: Some((1, vec![motion])),
                          pull_back: Some(pull_back), ..News::default() };
        let to = "10.0.0.5:50000".parse().unwrap();
        let kinds: Vec<u8> = packets(&news, to).iter().map(|packet| packet[0]).collect();
        assert_eq!(kinds, vec![0x56, 0x52, 0x51, 0x53]);
    }
}
