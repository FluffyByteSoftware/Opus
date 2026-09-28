//! File:       Opus/Conductor/dev/conductor-launcher/src/main.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Entry point.  Brings everything up in order -- Scribe, Constellations,
//! Archivist, the monitor, then the web admin -- and waits.  The console
//! is only Scribe's output from here on, and typing in it does nothing.
//! The admin works through the web page, and when they press Shut Down
//! there, the web admin stops, main wakes up, and Conductor shuts down.

// Rust note: the tools, the monitor and the web admin live in their own
// crates, and the `use` lines reach into them.  The crates are called
// conductor-tools and so on in Cargo.toml, and Rust spells them with an
// underscore in code, since a `-` would read as a minus sign.
use conductor_tools::{archivist, constellations, threads};
use conductor_tools::scribe::{self, Channel};

fn main() {
    // On the thread list as "main", like every thread we start.
    threads::name_this_thread("main");

    // Scribe first, so everything after it has somewhere to complain.  The
    // config isn't loaded yet, so it starts on the default log folder.
    scribe::start(&constellations::log_dir());

    // Then the settings.  Any complaints about the file go to the log.  If
    // the file points the logs somewhere else, Scribe follows.
    constellations::load();
    scribe::move_to(&constellations::log_dir());

    scribe::info(Channel::System, "Conductor is starting.");
    scribe::info(Channel::System, &format!("Content folder: {}", constellations::content_dir().display()));
    scribe::info(Channel::System, &format!("Settings from {}", constellations::config_path().display()));

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
    // Last, so the jobs already in Archivist's mailbox get done first.
    archivist::stop();

    scribe::info(Channel::System, "Conductor has shut down.");
}
