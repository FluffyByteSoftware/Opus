//! File:       Opus/Conductor/dev/conductor-monitor/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The monitor is Conductor's probe into itself.  Once a second, on a
//! thread of its own, it looks at how much memory and CPU the process is
//! using, how much it has read and written, every thread the OS says it
//! has, the threads our code asked for, and how Archivist is doing.  It
//! keeps the latest look, and `latest()` hands a copy to whoever asks
//! (the web admin, today) without waiting on anything.
//!
//! Where the numbers come from depends on the OS, and that's all in
//! `probe.rs`.  The math that turns two looks into a percent or a speed
//! is in `snapshot.rs`.
//!
//! What it can't show is memory per thread.  Every thread in a process
//! shares the same memory, and no OS keeps count of which thread is using
//! which part of it.  So memory is for the process as a whole, and CPU is
//! per thread.

pub mod probe;
mod snapshot;

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use conductor_tools::scribe::{self, Channel};
use conductor_tools::threads;

use snapshot::{Fixed, Previous};

// Rust note: `pub use` hands these on, so whoever uses the monitor can
// name a Snapshot without knowing it lives in snapshot.rs.
pub use snapshot::{Disk, Snapshot, ThreadInUse};

/// How often the monitor looks.  Once a second is often enough for a
/// person watching a page, and reading a few small files that often
/// costs next to nothing.
const EVERY: Duration = Duration::from_secs(1);

static LATEST: Mutex<Option<Snapshot>> = Mutex::new(None);

// Nothing is ever sent on this.  Dropping the Sender is the signal to
// stop, the same way Archivist's mailbox closes.
static STOP: Mutex<Option<Sender<()>>> = Mutex::new(None);
static MONITOR: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Starts the monitor's thread.  It comes straight back, and the first
/// look is ready a moment later.
pub fn start() {
    let (stop, stopped) = mpsc::channel();
    match threads::spawn("monitor", move || run(stopped)) {
        Ok(handle) => {
            *lock(&STOP) = Some(stop);
            *lock(&MONITOR) = Some(handle);
            scribe::info(Channel::System, "The monitor is up, looking once a second.");
        }
        Err(e) => scribe::error_with(Channel::System, &e, "The monitor couldn't start its thread.  \
            The web admin has no numbers this run."),
    }
}

/// Stops the monitor and waits for its thread to end, which is at most
/// the time it takes to finish the look it's on.
pub fn stop() {
    lock(&STOP).take();

    let handle = lock(&MONITOR).take();
    if let Some(handle) = handle {
        if handle.join().is_err() {
            scribe::error(Channel::System, "The monitor's thread had already died.");
        }
    }
}

/// A copy of the latest look.  `None` before the first one.
pub fn latest() -> Option<Snapshot> {
    lock(&LATEST).clone()
}

/// The lock idiom, for the statics above.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The monitor's thread: look, keep it, wait a second, and again, until
/// `stop()` drops the Sender.
fn run(stopped: Receiver<()>) {
    let fixed = Fixed {
        os: probe::os_name(),
        cores: thread::available_parallelism().map_or(1, |cores| cores.get()),
        started: Instant::now(),
    };
    scribe::debug(Channel::System, &format!("The monitor is on {}, with {} core(s).", fixed.os, fixed.cores));

    let mut previous: Option<Previous> = None;
    let mut said_it_cant = false;
    loop {
        let now = Instant::now();
        let reading = probe::read();

        if reading.is_none() && !said_it_cant {
            scribe::info(Channel::System, &format!("The monitor can't measure anything on {} yet.  \
                The web admin shows the database and the threads, and nothing else.", fixed.os));
            said_it_cant = true;
        }

        let snapshot = snapshot::build(&fixed, previous.as_ref(), now, reading.as_ref());
        *lock(&LATEST) = Some(snapshot);
        previous = reading.map(|reading| Previous { at: now, reading });

        // Rust note: `recv_timeout` waits up to a second for a message.
        // None ever comes, so it either times out (look again) or hears
        // the Sender is gone (stop).
        match stopped.recv_timeout(EVERY) {
            Err(RecvTimeoutError::Timeout) => {}
            Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    scribe::info(Channel::System, "The monitor has stopped.");
}
