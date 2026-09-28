//! File:       Opus/Conductor/dev/conductor-tools/src/threads.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Every thread Conductor starts goes through `spawn()` here, so there is a
//! list of them: its name, the file and line that asked for it, when it
//! started, and whether it's still going.  conductor-monitor puts that list
//! next to what the OS says is running, and the web admin shows both.
//!
//! The OS knows every thread by a number of its own.  Each thread writes
//! that number down as the first thing it does, which is how the monitor
//! matches a thread the OS reports to a name from this list.  A thread the
//! OS reports that isn't on the list was started by somebody else -- the
//! postgres crate starts a few of its own, for one.

use std::io;
use std::panic::Location;
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use crate::clock::Utc;

/// One thread our code asked for.
#[derive(Debug, Clone)]
pub struct ThreadRecord {
    /// The name it was given.  The OS gets the same name, where it keeps
    /// one.
    pub name: String,
    /// The file and line that called `spawn()`.
    pub started_by: String,
    /// When `spawn()` was called.
    pub started_at: Utc,
    /// The OS's own number for it.  `None` for the moment between starting
    /// and the thread writing it down, and on an OS we can't ask yet.
    pub os_id: Option<u64>,
    /// False once the thread has ended, or if it never started.
    pub running: bool,
}

// Finished threads stay on the list, so the page can show that something
// ended.  Conductor starts a handful of threads, not thousands, so the list
// staying small isn't something we have to work at.
static THREADS: Mutex<Vec<ThreadRecord>> = Mutex::new(Vec::new());

/// Starts a thread called `name` running `work`, and puts it on the list.
/// It's `std::thread::Builder` with the bookkeeping added, and hands back
/// the same thing Builder does.
///
/// ```text
/// let handle = threads::spawn("archivist", move || run(receiver))?;
/// ```
// Rust note: `F` is whatever closure the caller writes, and `T` is what it
// hands back when it's done, which `join()` on the handle gets.
#[track_caller]
pub fn spawn<F, T>(name: &str, work: F) -> io::Result<JoinHandle<T>>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let caller = Location::caller();
    let index = {
        let mut guard = THREADS.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.push(ThreadRecord {
            name: name.to_string(),
            started_by: format!("{}, Line: {}", caller.file(), caller.line()),
            started_at: Utc::now(),
            os_id: None,
            running: true,
        });
        guard.len() - 1
    };

    let spawned = thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            set_os_id(index, current_os_id());
            // Rust note: this is dropped when the closure ends, however it
            // ends -- a panic included -- and dropping it marks the thread
            // finished.  A `_name` binding lives to the end of the block,
            // where a plain `_` would be dropped on the spot.
            let _finished = MarkFinished { index };
            work()
        });

    if spawned.is_err() {
        set_finished(index);
    }
    spawned
}

/// A copy of the list, oldest first.
pub fn list() -> Vec<ThreadRecord> {
    let guard = THREADS.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.clone()
}

/// Marks its thread finished when it's dropped.
struct MarkFinished {
    index: usize,
}

impl Drop for MarkFinished {
    fn drop(&mut self) {
        set_finished(self.index);
    }
}

fn set_os_id(index: usize, os_id: Option<u64>) {
    let mut guard = THREADS.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(record) = guard.get_mut(index) {
        record.os_id = os_id;
    }
}

fn set_finished(index: usize) {
    let mut guard = THREADS.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(record) = guard.get_mut(index) {
        record.running = false;
    }
}

// ---------------------------------------------------------------------------
// The OS's number for the thread we're on
// ---------------------------------------------------------------------------

// Rust note: `#[cfg(...)]` keeps a piece of code in or out of the build
// depending on what it's being built for.  Only one of the three
// `current_os_id()` below is ever compiled, so each OS gets its own.

/// Linux.  `/proc/thread-self` is a link to `<pid>/task/<thread id>`, so
/// the last part of where it points is the number.
#[cfg(target_os = "linux")]
fn current_os_id() -> Option<u64> {
    let link = std::fs::read_link("/proc/thread-self").ok()?;
    link.file_name()?.to_str()?.parse().ok()
}

/// Windows asks kernel32 for it.
#[cfg(windows)]
fn current_os_id() -> Option<u64> {
    Some(u64::from(windows::GetCurrentThreadId()))
}

/// Anywhere else (macOS, for now) we don't ask, and the monitor can't
/// match threads up there anyway.
#[cfg(not(any(target_os = "linux", windows)))]
fn current_os_id() -> Option<u64> {
    None
}

#[cfg(windows)]
mod windows {
    // Rust note: an `extern` block lists functions that live outside Rust,
    // here in Windows' kernel32.dll.  Calling one is normally `unsafe`,
    // since Rust can't check what the other side does.  Marking it `safe
    // fn` is us promising it can't go wrong, which holds for a function
    // that takes nothing and hands back a number.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub(super) safe fn GetCurrentThreadId() -> u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thread_goes_on_the_list_and_comes_off_running() {
        let handle = spawn("threads-test", || 7).expect("the test thread should start");
        assert_eq!(handle.join().ok(), Some(7));

        let record = list().into_iter().find(|record| record.name == "threads-test")
            .expect("the test thread should be on the list");
        assert!(!record.running);
        assert!(record.started_by.contains("threads.rs"));
        if cfg!(any(target_os = "linux", windows)) {
            assert!(record.os_id.is_some());
        }
    }
}
