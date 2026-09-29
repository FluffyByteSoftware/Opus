//! File:       Opus/Conductor/dev/conductor-launcher/src/main.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Entry point.  Brings everything up in order -- DiskMan, Scribe,
//! Constellations, Fingerprinter, Archivist, the monitor, then the web
//! admin -- and waits.  The console is only Scribe's output from here on,
//! and typing in it does nothing.  The admin works through the web page,
//! and when they press Shut Down there, the web admin stops, main wakes
//! up, and Conductor shuts down.  DiskMan goes last, and shutdown waits
//! on it to write out everything it's holding.

// Rust note: the tools, the monitor and the web admin live in their own
// crates, and the `use` lines reach into them.  The crates are called
// conductor-tools and so on in Cargo.toml, and Rust spells them with an
// underscore in code, since a `-` would read as a minus sign.
use std::thread;
use std::time::{Duration, Instant};

use conductor_tools::{archivist, constellations, diskman, fingerprinter, threads};
use conductor_tools::scribe::{self, Channel};

/// How long shutdown gives DiskMan before telling the admin to force quit.
/// It keeps waiting after that; it just stops counting.
const DISKMAN_GRACE: Duration = Duration::from_secs(60);

/// How often the countdown says where it's at.
const COUNTDOWN_EVERY: Duration = Duration::from_secs(5);

fn main() {
    // On the thread list as "main", like every thread we start.
    threads::name_this_thread("main");

    // DiskMan first.  Every file goes through it, Scribe's log included.
    diskman::start();

    // Then Scribe, so everything after it has somewhere to complain.  The
    // config isn't loaded yet, so it starts on the default log folder.
    scribe::start(&constellations::log_dir());

    // Then the settings.  Any complaints about the file go to the log.  If
    // the file points the logs somewhere else, Scribe follows.
    constellations::load();
    scribe::move_to(&constellations::log_dir());

    scribe::info(Channel::System, "Conductor is starting.");
    scribe::info(Channel::System, &format!("Content folder: {}", constellations::content_dir().display()));
    scribe::info(Channel::System, &format!("Settings from {}", constellations::config_path().display()));

    // Fingerprinter, the UUID maker, checks the OS will give it random
    // bytes before anything needs a UUID.
    fingerprinter::start();

    // Then the database.  This comes straight back, and Archivist connects
    // on its own thread.  The log says how that went.
    archivist::start();

    // The monitor next, so it has a first look ready by the time the page
    // asks for one.
    conductor_monitor::start();

    // Last, the web admin, and then we wait on it.  If it can't start,
    // there'd be no way to shut Conductor down short of killing it, so we
    // don't run without it.
    if conductor_wgui::start(constellations::settings().wgui_port) {
        conductor_wgui::wait();
    } else {
        scribe::error(Channel::System, "CONDUCTOR CAN'T RUN WITHOUT ITS WEB ADMIN.  \
            The line above says why.");
    }

    scribe::info(Channel::System, "Conductor is shutting down.");

    conductor_monitor::stop();
    // After the rest, so the jobs already in Archivist's mailbox get done
    // first.
    archivist::stop();

    // DiskMan is last, since Archivist and everything before it may have
    // handed it files on the way out.
    scribe::info(Channel::System, "Everything else has stopped.  DiskMan is writing out what it holds.");
    wait_on_diskman();

    // DiskMan has finished, so this one only reaches the console.
    scribe::info(Channel::System, "Conductor has shut down.");
}

/// Tells DiskMan to finish up and waits until it has.  If it takes more
/// than a second, the console counts down from a minute.  At zero it says
/// Conductor should be closed and to force quit it if it isn't, with what
/// would be lost, and says so again every 30 seconds for as long as it's
/// still going.  It keeps waiting on DiskMan the whole time.  The countdown
/// lines go through DiskMan too, like every line.
fn wait_on_diskman() {
    diskman::stop();
    let started = Instant::now();
    let mut next_note = Duration::from_secs(1);

    while !diskman::finished() {
        thread::sleep(Duration::from_millis(50));
        let waited = started.elapsed();
        if waited < next_note {
            continue;
        }

        let status = diskman::status();
        let holding = format!("{} file(s), {}", status.files_waiting, megabytes(status.bytes_waiting));
        if waited < DISKMAN_GRACE {
            let left = (DISKMAN_GRACE - waited).as_secs();
            scribe::info(Channel::System, &format!("Waiting on DiskMan to write {holding}.  {left} s left."));
            next_note += COUNTDOWN_EVERY;
        } else {
            scribe::error(Channel::System, &format!("SHOULD BE CLOSED, IF STILL RUNNING PLEASE FORCE QUIT.  \
                DiskMan still has {holding} to write, and force quitting loses it:"));
            for path in diskman::waiting_files() {
                scribe::error(Channel::System, &format!("Not written yet: {}", path.display()));
            }
            next_note = waited + DISKMAN_GRACE / 2;
        }
    }
}

/// Bytes as megabytes, one place after the point.
fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}
