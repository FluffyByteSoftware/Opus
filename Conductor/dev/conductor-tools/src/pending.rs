//! File:       Opus/Conductor/dev/conductor-tools/src/pending.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! `Pending`, the answer that is on its way.  Archivist and DiskMan both
//! hand one back the moment a job goes in their mailbox, so the caller can
//! carry on and pick the answer up later.  It started in Archivist and
//! moved here when DiskMan needed the same thing.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::time::Duration;

/// What a tool's error says when there's nobody left to answer: the tool
/// isn't running, or it stopped before it got to the job.
pub trait NotRunning {
    fn not_running() -> Self;
}

/// An answer that is on its way.  The answer turns up in it once the
/// worker gets to the job.
///
/// Once `check()` has handed back an answer, the `Pending` is used up.
/// Ask it again and it says "not running", because there is nobody left
/// on the other end.
// Rust note: `T` is what a good answer holds and `E` is the tool's own
// error.  Archivist and DiskMan each give this a shorter name of their
// own, so callers write `Pending<u64>` and never see the `E`.
pub struct Pending<T, E> {
    reply: Receiver<Result<T, E>>,
}

impl<T, E: NotRunning> Pending<T, E> {
    /// A `Pending` and the other end of it, which the worker answers on.
    /// If the worker drops its end without answering, the `Pending` hears
    /// "not running" instead of waiting forever.
    pub(crate) fn new() -> (Sender<Result<T, E>>, Pending<T, E>) {
        let (reply, answer) = mpsc::channel();
        (reply, Pending { reply: answer })
    }

    /// A `Pending` that already has its answer, for a job that could be
    /// answered on the spot.
    pub(crate) fn ready(answer: Result<T, E>) -> Pending<T, E> {
        let (reply, pending) = Pending::new();
        // The receiving end is right here, so this can't fail.
        let _ = reply.send(answer);
        pending
    }

    /// The answer if it's here, `None` if the worker hasn't got to the job
    /// yet.  Never waits, so this is the one the game loop uses.
    pub fn check(&self) -> Option<Result<T, E>> {
        match self.reply.try_recv() {
            Ok(answer) => Some(answer),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(E::not_running())),
        }
    }

    /// Waits up to `wait` for the answer, and hands back `None` if it
    /// hasn't come by then.  The `Pending` is still good after a `None`,
    /// so a thread can wait a second at a time and do something in
    /// between (tell a client where it stands in Security's line, say).
    /// For a connection's own thread, never the game loop.
    pub fn wait_for(&self, wait: Duration) -> Option<Result<T, E>> {
        match self.reply.recv_timeout(wait) {
            Ok(answer) => Some(answer),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => Some(Err(E::not_running())),
        }
    }

    /// Waits for the answer.  Fine at startup and for the web admin, but
    /// never in the game loop, because this is exactly the blocking the
    /// workers were made to avoid.
    pub fn wait(self) -> Result<T, E> {
        // Rust note: `recv()` fails only when the other end is gone
        // without answering, which means the worker isn't there.
        self.reply.recv().unwrap_or_else(|_| Err(E::not_running()))
    }
}
