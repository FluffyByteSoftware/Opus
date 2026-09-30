//! File:       Opus/Conductor/dev/launcher/src/main.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Entry point.  Brings up the program -- DiskMan, Scribe, Constellations
//! and the web admin -- and then waits on the web admin's Control Panel.
//! The server itself (Fingerprinter, Security, Archivist, the account desk,
//! Lua, networking, the monitor, and whatever comes later) doesn't start until
//! the admin presses START SERVER there,
//! and STOP SERVER takes it back down while the program keeps running.
//! The console is only Scribe's output, and typing in it does nothing.
//! When the admin presses SHUT DOWN, the web admin stops, main wakes up,
//! stops the server if it's running, and Conductor shuts down.  DiskMan
//! goes last, and shutdown waits on it to write out everything it's
//! holding.
//!
//! `start_server()` and `stop_server()` below are the list of what the
//! server is.  A new piece goes in both.

// Rust note: the tools, the monitor and the web admin live in their own
// crates, and the `use` lines reach into them.  The crates are called
// conductor-tools and so on in Cargo.toml (their folders are tools and so
// on), and Rust spells them with an underscore in code, since a `-` would
// read as a minus sign.
use std::thread;
use std::time::{Duration, Instant};

use conductor_tools::{archivist, constellations, diskman, fingerprinter, security, threads};
use conductor_tools::scribe::{self, Channel};
use conductor_tools::server::{self, Command, State};

/// How long shutdown gives DiskMan before telling the admin to force quit.
/// It keeps waiting after that; it just stops counting.
const DISKMAN_GRACE: Duration = Duration::from_secs(60);

/// How often the countdown says where it's at.
const COUNTDOWN_EVERY: Duration = Duration::from_secs(5);

/// How long main waits for a command from the Control Panel before
/// checking that the web admin is still there.
const COMMAND_WAIT: Duration = Duration::from_millis(250);

fn main() {
    // On the thread list as "main", like every thread we start.
    threads::name_this_thread("main");

    // DiskMan first.  Every file goes through it, Scribe's log included.
    diskman::start();

    // Then Scribe, so everything after it has somewhere to complain.  The
    // config isn't loaded yet, so it starts on the default log folder.
    scribe::start(&constellations::log_dir());

    // Then the program's own settings.  Any complaints about the file go
    // to the log.  If the file points the logs somewhere else, Scribe
    // follows.  The server pieces load their own files when they start.
    constellations::load(&constellations::GLOBALS);
    scribe::move_to(&constellations::log_dir());

    // The web admin's own file: its two accounts.  Hard, like globals, so
    // it's read here and never again.
    constellations::load(&constellations::WGUI);

    scribe::info(Channel::System, "Conductor is starting.");
    scribe::info(Channel::System, &format!("Content folder: {}", constellations::content_dir().display()));
    scribe::info(Channel::System, &format!("Settings from {}", constellations::config_path().display()));
    scribe::info(Channel::System, "A changed conductor_globals.cfg or wgui.cfg needs Conductor run again; a \
        changed postgres.cfg or networking.cfg needs STOP SERVER and START SERVER.");

    server::set(State::Stopped, "Not started yet.  START SERVER on the Control Panel starts it.");

    // Last, the web admin, and then we wait on it.  If it can't start,
    // there'd be no way to shut Conductor down short of killing it, so we
    // don't run without it.
    if conductor_wgui::start(constellations::settings().wgui_port) {
        scribe::info(Channel::System, "Conductor is up.  The server waits on START SERVER from the Control Panel.");
        take_commands();
    } else {
        scribe::error(Channel::System, "CONDUCTOR CAN'T RUN WITHOUT ITS WEB ADMIN.  \
            The line above says why.");
    }

    scribe::info(Channel::System, "Conductor is shutting down.");
    if server::status().state != State::Stopped {
        stop_server();
    }

    // DiskMan is last, since Archivist and everything before it may have
    // handed it files on the way out.
    scribe::info(Channel::System, "Everything else has stopped.  DiskMan is writing out what it holds.");
    wait_on_diskman();

    // DiskMan has finished, so this one only reaches the console.
    scribe::info(Channel::System, "Conductor has shut down.");
}

/// Sits on the Control Panel's mailbox for as long as Conductor runs,
/// doing what it asks: start, stop or restart the server.  Comes back
/// once the web admin has ended, which is SHUT DOWN, or its thread dying.
fn take_commands() {
    loop {
        if conductor_wgui::has_ended() {
            return;
        }
        match server::next_command(COMMAND_WAIT) {
            Some(Command::Start) => start_server(),
            Some(Command::Stop) => stop_server(),
            Some(Command::Restart) => {
                stop_server();
                start_server();
            }
            None => {}
        }
    }
}

/// Brings up everything that is the server, in order.  Fingerprinter
/// checks the OS will give it random bytes before anything needs a UUID;
/// Security allots its hash arena on its own thread, and takes its salts
/// from Fingerprinter, so it comes after it; Archivist comes straight back
/// and connects on its own thread; the account desk, which leans on
/// Security and Archivist, comes after both; Lua runs the scripts before
/// the door opens, since the world will be made of them one day;
/// networking opens the door once the three a login leans on are up; the
/// monitor starts looking once a second.  None of them can fail to the
/// point of stopping this: each says how it went in the log and on the
/// Services tab.
fn start_server() {
    server::set(State::Starting, "Starting Fingerprinter, Security, Archivist, the account desk, Lua, networking \
        and the monitor.");
    scribe::info(Channel::System, "The server is starting.");

    fingerprinter::start();
    security::start();
    archivist::start();
    conductor_accounts::desk::start();
    conductor_lua_parser::start();
    conductor_networking::start();
    conductor_monitor::start();

    server::set(State::Running, "Fingerprinter, Security, Archivist, the account desk, Lua, networking and the \
        monitor were started.  The Services tab says how each one is doing.");
    scribe::info(Channel::System, "The server is running.");
}

/// Takes the server back down, in the opposite order.  Networking goes
/// first, so the door is shut and every player told before the pieces a
/// login leans on go; Lua goes once nobody is left in the world its
/// scripts will run; the account desk finishes the jobs the web admin
/// handed it while Security and Archivist are still there to do them;
/// Security goes before Archivist, so a hash on its way to the accounts
/// table still gets there; Archivist finishes the
/// jobs already in its mailbox, and anything it hands DiskMan on the way
/// out is written by the DiskMan that's still running.  Last, with every
/// server piece down, any config file saved from the web admin while they
/// ran is swapped in, so the next START SERVER reads the new one.
fn stop_server() {
    server::set(State::Stopping, "Stopping the monitor, networking, Lua, the account desk, Security, Archivist \
        and Fingerprinter.");
    scribe::info(Channel::System, "The server is stopping.");

    conductor_monitor::stop();
    conductor_networking::stop();
    conductor_lua_parser::stop();
    conductor_accounts::desk::stop();
    security::stop();
    archivist::stop();
    fingerprinter::stop();
    constellations::server_stopped();

    server::set(State::Stopped, "Stopped.  START SERVER on the Control Panel starts it again.");
    scribe::info(Channel::System, "The server has stopped.");
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
