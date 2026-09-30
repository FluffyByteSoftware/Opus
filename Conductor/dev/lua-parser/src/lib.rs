//! File:       Opus/Conductor/dev/lua-parser/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Lua, the language the game's content will be written in.  On START
//! SERVER this finds every `.lua` file under `Content/scripts/` (folders
//! inside it included), reads each one through DiskMan, and runs it in a
//! locked-down Lua of its own (`sandbox.rs` says what's locked).  STOP
//! SERVER stops it, and RESTART SERVER runs them all again, so a changed
//! script takes on a soft reboot.
//!
//! For now that's all it does.  The only thing a script can call is the
//! log, so `hello.lua` saying hello on the Log tab is the proof that Lua
//! runs inside Conductor and can't hurt it.  Templates, blueprints and
//! behaviour scripts on objects come with the ECS.
//!
//! A script with an error is a Warn naming its file and line, and the rest
//! still run.  Nothing a script does is ever an Error or takes the server
//! down.
//!
//! It all happens on one thread, `lua`, since running a folder of scripts
//! can be slow (a second each, at worst) and START SERVER shouldn't wait
//! on it.  The thread stays up after the scripts have run, checking in
//! with the Services tab, because that's where the game's scripts will
//! live once there are objects for them to drive.

mod sandbox;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

use conductor_tools::constellations;
use conductor_tools::diskman;
use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};
use conductor_tools::threads;

/// The scripts' folder, under `Content/`.  Jacob's pick, 2026-09-30.
const SCRIPT_FOLDER: &str = "scripts";

/// How often the thread checks in with the Services tab once the scripts
/// have run.
const CHECK_IN_EVERY: Duration = Duration::from_secs(1);

// Nothing is ever sent on this.  Dropping the Sender is the signal to
// stop, the same way the monitor does it.
static STOP: Mutex<Option<Sender<()>>> = Mutex::new(None);
static LUA: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Starts the `lua` thread, which runs the scripts.  It comes straight
/// back; the Services tab and the log say how the scripts went.  The
/// launcher calls this every time the server starts, and `stop()` every
/// time it stops.
pub fn start() {
    if lock(&LUA).as_ref().is_some_and(|handle| !handle.is_finished()) {
        scribe::warn(Channel::Script, "Lua was asked to start while it's already running.  \
            The running one stands.");
        return;
    }

    services::set(services::LUA, State::Starting, "Running the scripts.");
    let (stop, stopped) = mpsc::channel();
    match threads::spawn("lua", move || run(stopped)) {
        Ok(handle) => {
            *lock(&STOP) = Some(stop);
            *lock(&LUA) = Some(handle);
        }
        Err(e) => {
            scribe::error_with(Channel::Script, &e, "Lua couldn't start its thread.  \
                No scripts run this time.");
            services::set(services::LUA, State::Stopped, &format!("Couldn't start its thread: {e}"));
        }
    }
}

/// Stops the `lua` thread and waits for it to end.  If it's in the middle
/// of the scripts, it finishes the one it's on (a second at most) and
/// skips the rest.
pub fn stop() {
    lock(&STOP).take();

    let handle = lock(&LUA).take();
    if let Some(handle) = handle {
        if handle.join().is_err() {
            scribe::error(Channel::Script, "Lua's thread had already died.");
        }
    }
}

/// The lock idiom, for the statics above.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The `lua` thread: run every script once, then check in once a second
/// until `stop()` drops the Sender.
fn run(stopped: Receiver<()>) {
    let content = constellations::content_dir();
    let folder = content.join(SCRIPT_FOLDER);
    // DiskMan does files, not folders.  Making an empty folder and listing
    // what's in one stay out here.
    let _ = fs::create_dir_all(&folder);

    let mut scripts = Vec::new();
    if let Err(e) = find_scripts(&folder, &mut scripts) {
        scribe::warn_with(Channel::Script, &e, &format!("Lua can't read the script folder {}.  \
            No scripts run this time.", folder.display()));
    }
    // Sorted, so they run in the same order every time and the log reads
    // the same from one run to the next.
    scripts.sort();

    let mut ran = 0;
    let mut failed = 0;
    for path in &scripts {
        // Rust note: `try_recv` never waits.  Nothing is ever sent, so
        // "Disconnected" is the only answer that means anything: `stop()`
        // has dropped the Sender.
        if let Err(TryRecvError::Disconnected) = stopped.try_recv() {
            scribe::debug(Channel::Script, "The server is stopping, so the rest of the scripts are skipped.");
            break;
        }
        services::seen(services::LUA);

        let name = name_of(content, path);
        match read_and_run(path, &name) {
            Ok(()) => {
                scribe::debug(Channel::Script, &format!("Ran {name}."));
                ran += 1;
            }
            Err(why) => {
                scribe::warn(Channel::Script, &why);
                failed += 1;
            }
        }
    }

    let summary = format!("Ran {ran} script(s) from {}, {failed} with an error.", folder.display());
    scribe::info(Channel::Script, &format!("Lua is up.  {summary}"));
    if failed > 0 {
        services::set(services::LUA, State::Trouble, &format!("{summary}  The log says which."));
    } else {
        services::set(services::LUA, State::Running, &summary);
    }

    loop {
        services::seen(services::LUA);
        // Rust note: `recv_timeout` waits up to a second for a message.
        // None ever comes, so it either times out (check in again) or
        // hears the Sender is gone (stop).
        match stopped.recv_timeout(CHECK_IN_EVERY) {
            Err(RecvTimeoutError::Timeout) => {}
            Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    scribe::info(Channel::Script, "Lua has stopped.");
    services::set(services::LUA, State::Stopped, "Shut down.");
}

/// Every `.lua` file in `folder` and the folders inside it, added to
/// `found`.  A folder inside that can't be read is a Warn, and the rest
/// are still looked at.  A link to a folder isn't followed, so a link
/// that points back up can't send this round in circles.
fn find_scripts(folder: &Path, found: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        let is_folder = entry.file_type().is_ok_and(|kind| kind.is_dir());

        if is_folder {
            if let Err(e) = find_scripts(&path, found) {
                scribe::warn_with(Channel::Script, &e, &format!("Lua can't read the folder {}.  \
                    The scripts in it don't run this time.", path.display()));
            }
        } else if path.extension().is_some_and(|ext| ext == "lua") {
            found.push(path);
        }
    }
    Ok(())
}

/// The script's path from `Content/`, with `/` between the folders on
/// every OS: `scripts/npcs/goblin.lua`.  That's the name the log uses.
fn name_of(content: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(content).unwrap_or(path);
    let parts: Vec<String> = relative.components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    parts.join("/")
}

/// Reads one script through DiskMan and runs it.  An error is the Warn to
/// log.
fn read_and_run(path: &Path, name: &str) -> Result<(), String> {
    let bytes = diskman::read(path).wait()
        .map_err(|e| format!("Lua couldn't read {name}: {e}"))?;
    let source = diskman::as_text(&bytes)
        .map_err(|e| format!("{name} isn't a text file Lua can read: {e}"))?;
    sandbox::run(name, &source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_is_named_from_content() {
        let content = Path::new("opus").join("Content");
        let path = content.join("scripts").join("npcs").join("goblin.lua");
        assert_eq!(name_of(&content, &path), "scripts/npcs/goblin.lua");
    }
}
