//! File:       Opus/Conductor/dev/conductor-tools/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The tools: the pieces the rest of the server leans on but that know
//! nothing about the game.  The disk, logging, config, the database, the
//! UUIDs, the clock, the list of threads we started, the list of services
//! we expect, the notices the admin has to acknowledge, and the switch
//! that starts and stops the server.  A lib, so the launcher and anything
//! that comes after it can all use them.

pub mod archivist;
pub mod clock;
pub mod constellations;
pub mod diskman;
pub mod fingerprinter;
pub mod notices;
pub mod pending;
pub mod scribe;
pub mod server;
pub mod services;
pub mod threads;
