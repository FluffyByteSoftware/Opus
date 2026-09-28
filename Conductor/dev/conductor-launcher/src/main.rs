//! File:       Opus/Conductor/dev/conductor-launcher/src/main.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Entry point.  Brings the tools up in order, hands the terminal to the
//! admin's menu, and when the menu returns, Conductor shuts down.  Right
//! now the tools are Scribe, Constellations and Archivist, in that order.

// Rust note: `mod launcher;` tells the compiler that src/launcher.rs is part
// of this program.  The tools live in their own crate, and the `use` lines
// reach into it.  The crate is called conductor-tools in Cargo.toml, and
// Rust spells it with an underscore in code, since a `-` would read as a
// minus sign.
mod launcher;

use conductor_tools::{archivist, constellations};
use conductor_tools::scribe::{self, Channel};

fn main() {
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

    // The admin's menu.  It runs until they pick Q.
    launcher::run();

    scribe::info(Channel::System, "Conductor is shutting down.");

    // Last, so the jobs already in Archivist's mailbox get done first.
    archivist::stop();
}
