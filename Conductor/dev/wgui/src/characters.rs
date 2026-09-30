//! File:       Opus/Conductor/dev/wgui/src/characters.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The Characters tab's route, under GAME MANAGEMENT: every player's
//! character on the server, with its name, UUID, where it last stood and
//! the account it belongs to.  Look only (Jacob, 2026-09-30); editing
//! them is its own piece of work.
//!
//! - `GET /Opus/Content/characters` -- every character, by name.
//!
//! `admin` and `user` both see it (Jacob's pick).  It only works while
//! the server is running, since the database is a server piece.  The
//! list is one quick database job, so the web admin waits for it, up to
//! DATABASE_WAIT.

use std::time::Duration;

use conductor_accounts::characters;
use conductor_tools::server;

use crate::{json, Answer, Next};

/// How long the web admin waits on Archivist for the list.  It's one
/// quick statement; this is only so a database that's stuck can't hold
/// the page up for good.
const DATABASE_WAIT: Duration = Duration::from_secs(5);

/// The list, for the Characters tab.
pub(crate) fn list() -> (Answer, Next) {
    if !matches!(server::status().state, server::State::Running) {
        return (Answer::plain("409 Conflict", "Characters can only be looked at while the server is running.  \
            START SERVER on the Server tab."), Next::KeepGoing);
    }
    let answer = match characters::list_all().wait_for(DATABASE_WAIT) {
        Some(Ok(list)) => Answer::new("200 OK", "application/json", json::characters(&list)),
        Some(Err(e)) => Answer::plain("503 Service Unavailable", &format!("The characters couldn't be read: {e}.")),
        None => Answer::plain("503 Service Unavailable", &format!("The database didn't answer within {} seconds.  \
            Try again in a moment.", DATABASE_WAIT.as_secs())),
    };
    (answer, Next::KeepGoing)
}
