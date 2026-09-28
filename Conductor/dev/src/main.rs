//! File:       Opus/Conductor/dev/src/main.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Entry point.  Loads the globals, opens the log, says hello, and for now
//! stops there.  The tick loop and the network come later.

mod tools;

use tools::constellations;
use tools::scribe::{self, Channel};

fn main() {
    // Constellations has to come up before Scribe, because Scribe gets its
    // log folder from the globals.  Until Scribe is open, a failure can only
    // go to stderr.
    if let Err(e) = constellations::load() {
        eprintln!("Could not load the globals: {e}");
        std::process::exit(1);
    }
    let globals = constellations::globals();

    if let Err(e) = scribe::start(&globals.scribe_log_dir) {
        eprintln!("Could not open the log in {}: {e}", globals.scribe_log_dir.display());
        std::process::exit(1);
    }

    scribe::info(Channel::System, "Conductor is starting.");
    scribe::info(Channel::System, &format!("Content folder: {}", constellations::content_dir().display()));
    scribe::info(Channel::System, &format!("Globals loaded from {}", constellations::config_path().display()));
    for key in &globals.unknown_keys {
        scribe::warn(Channel::System, &format!("Unknown key in the globals file, ignored: {key}"));
    }

    scribe::info(Channel::System, "Nothing else to do yet, so this is where Conductor stops.");
}
