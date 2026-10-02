//! File:       Opus/Conductor/dev/gameclock/src/who.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! `/who list`'s mailbox.  A plain `/who` is only names, which networking
//! has, so networking answers it itself.  `/who list` says where each
//! character stands, and only the GameClock's thread may look in the
//! world, so networking leaves the ask here with `who_list()`, which
//! comes straight back.  The broadcast check reads every player's
//! character's name and the block it stands in, once for however many
//! asked that cycle, and hands them with each ask to the function
//! networking gave us at its start, which builds the answer and sends it
//! to the one who asked.  The same shape as the chat (`chat.rs`), and for
//! the same reason: the GameClock can't call networking itself.

use std::mem;
use std::net::SocketAddr;
use std::sync::Mutex;

use crate::Game;

/// Who asked for a `/who list`, so the answer finds its way back.  The
/// GameClock only carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhoAsked {
    pub from: SocketAddr,
    pub account: String,
    pub ask: u32,
}

/// One player's character as `/who list` shows it: its name and the block
/// it stands in, x, y and z, y up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub name: String,
    pub block: [i32; 3],
}

/// The asks waiting for the next broadcast.  `None` while the GameClock is
/// stopped.
static WAITING: Mutex<Option<Vec<WhoAsked>>> = Mutex::new(None);

/// What answers one ask with where everybody stands.  Networking's, handed
/// in with `set_who_sender()`.
static SENDER: Mutex<Option<fn(&WhoAsked, &[Standing])>> = Mutex::new(None);

/// Opens the mailbox, empty, for a fresh start of the GameClock.
pub(crate) fn open() {
    *crate::lock(&WAITING) = Some(Vec::new());
}

/// Turns asks away from here on, and drops any still waiting.  For
/// `stop()`: networking has stopped already, and nobody is left to answer.
pub(crate) fn close() {
    crate::lock(&WAITING).take();
}

/// Leaves a `/who list` for the next broadcast.  Comes straight back.  An
/// error means the GameClock isn't running.
pub fn who_list(asked: WhoAsked) -> Result<(), String> {
    match crate::lock(&WAITING).as_mut() {
        Some(waiting) => {
            waiting.push(asked);
            Ok(())
        }
        None => Err("the GameClock isn't running".to_string()),
    }
}

/// Networking's function for answering a `/who list`.  Networking calls
/// this as it starts.
pub fn set_who_sender(send: fn(&WhoAsked, &[Standing])) {
    *crate::lock(&SENDER) = Some(send);
}

/// The broadcast check's part: every `/who list` asked since the last
/// cycle, answered.  Costs a lock and nothing else in a cycle nobody asked.
pub(crate) fn answer(game: &Game) {
    let asks = take_waiting();
    if asks.is_empty() {
        return;
    }
    let standing = game.players.standing(&game.world);
    let send = *crate::lock(&SENDER);
    if let Some(send) = send {
        for asked in &asks {
            send(asked, &standing);
        }
    }
}

/// Every ask waiting, taken out, leaving the mailbox empty.
fn take_waiting() -> Vec<WhoAsked> {
    match crate::lock(&WAITING).as_mut() {
        Some(waiting) => mem::take(waiting),
        None => Vec::new(),
    }
}

/// The block a position is in: each axis rounded down, so 1.5 is in block
/// 1 and -1.5 in block -2.  Jacob: "a block is the width of a player so
/// they can only really fit on one".
pub(crate) fn block_of(position: [f32; 3]) -> [i32; 3] {
    // Rust note: `as i32` on a float too big for an i32 gives the biggest
    // i32 rather than garbage, so a wild position can't break this.
    position.map(|axis| axis.floor() as i32)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(ask: u32) -> WhoAsked {
        WhoAsked { from: "10.0.0.5:50000".parse().unwrap(), account: "jacob".to_string(), ask }
    }

    // One test for the mailbox, since it's a static (see chat.rs).
    #[test]
    fn asks_wait_in_order_until_taken_and_a_stopped_gameclock_turns_them_away() {
        close();
        assert!(who_list(asked(1)).is_err());

        open();
        who_list(asked(2)).unwrap();
        who_list(asked(3)).unwrap();
        assert_eq!(take_waiting(), vec![asked(2), asked(3)]);
        assert!(take_waiting().is_empty());

        who_list(asked(4)).unwrap();
        close();
        assert!(take_waiting().is_empty());
    }

    #[test]
    fn a_position_is_in_the_block_below_it() {
        assert_eq!(block_of([1.5, 0.0, -2.0]), [1, 0, -2]);
        assert_eq!(block_of([-1.5, 0.99, -0.01]), [-2, 0, -1]);
        assert_eq!(block_of([1e20, -1e20, 0.0]), [i32::MAX, i32::MIN, 0]);
    }
}
