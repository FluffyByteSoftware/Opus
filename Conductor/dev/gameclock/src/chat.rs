//! File:       Opus/Conductor/dev/gameclock/src/chat.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The chat's mailbox.  Networking reads a player's `/chat`, makes the
//! finished line (`[Chat] Jacob: Yo yo yo!`) and leaves it here with
//! `chat()`, which comes straight back.  The broadcast check takes
//! everything left since the cycle before and hands it, in the order it
//! came, to the function networking gave us at its start, which sends it
//! to everybody in the world, the one who said it too.  Jacob,
//! 2026-10-02: "we send a packet to all users including the person who
//! sent the message on the next "chat" GameClock tick that carries chat
//! (which should be every beat)".
//!
//! The GameClock never knows who's in the world over UDP, or where they
//! are: networking does.  And networking already leans on the GameClock
//! (it calls `enter()` and `leave()`), so the GameClock can't lean back
//! on networking; Rust won't build two crates that each need the other.
//! So networking hands us its sender as a plain function, once, and we
//! call it.  The positions will go out the same way when there's
//! movement.

use std::mem;
use std::sync::Mutex;

/// The lines waiting for the next broadcast.  `None` while the GameClock
/// is stopped, so a line said then is turned away instead of waiting for
/// a GameClock that isn't coming.
static WAITING: Mutex<Option<Vec<String>>> = Mutex::new(None);

/// What sends a cycle's lines to everybody in the world.  Networking's,
/// handed in with `set_chat_sender()`.
// Rust note: `fn(&[String])` is the type of a plain function that takes
// a list of lines, the same as the checks' `fn(&mut Game)`.  A plain
// function lives as long as the program, so it's never out of date.
static SENDER: Mutex<Option<fn(&[String])>> = Mutex::new(None);

/// Opens the mailbox, empty, for a fresh start of the GameClock.
pub(crate) fn open() {
    *crate::lock(&WAITING) = Some(Vec::new());
}

/// Turns lines away from here on, and drops any still waiting.  For
/// `stop()`: nobody is in the world to hear them.
pub(crate) fn close() {
    crate::lock(&WAITING).take();
}

/// Leaves a finished line of chat for the next broadcast.  Comes straight
/// back.  An error means the GameClock isn't running.
pub fn chat(line: String) -> Result<(), String> {
    match crate::lock(&WAITING).as_mut() {
        Some(waiting) => {
            waiting.push(line);
            Ok(())
        }
        None => Err("the GameClock isn't running".to_string()),
    }
}

/// Networking's function for sending a cycle's lines to everybody in the
/// world.  Networking calls this as it starts.  Lines with nobody to send
/// them before then are dropped.
pub fn set_chat_sender(send: fn(&[String])) {
    *crate::lock(&SENDER) = Some(send);
}

/// The broadcast check's part: everything said since the last cycle, out
/// to everybody.  Costs a lock and nothing else in a cycle with no chat.
pub(crate) fn broadcast() {
    let lines = take_waiting();
    if lines.is_empty() {
        return;
    }
    // Rust note: `*` copies the function out of the lock, so the lock is
    // let go before the sending starts.
    let send = *crate::lock(&SENDER);
    if let Some(send) = send {
        send(&lines);
    }
}

/// Everything waiting, taken out, leaving the mailbox empty.
fn take_waiting() -> Vec<String> {
    match crate::lock(&WAITING).as_mut() {
        Some(waiting) => mem::take(waiting),
        None => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // One test, not several: the mailbox is a static, and Rust runs tests
    // side by side, so two tests opening and closing it would trip each
    // other up.
    #[test]
    fn lines_wait_in_order_until_taken_and_a_stopped_gameclock_turns_them_away() {
        close();
        assert!(chat("[Chat] Jacob: Hello?".to_string()).is_err());

        open();
        chat("[Chat] Jacob: Yo yo yo!".to_string()).unwrap();
        chat("[Chat] Mckay: Hi".to_string()).unwrap();
        assert_eq!(take_waiting(), vec!["[Chat] Jacob: Yo yo yo!".to_string(), "[Chat] Mckay: Hi".to_string()]);
        assert!(take_waiting().is_empty());

        chat("[Chat] Jacob: Anybody?".to_string()).unwrap();
        close();
        assert!(take_waiting().is_empty());
    }
}
