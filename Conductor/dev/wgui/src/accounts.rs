//! File:       Opus/Conductor/dev/wgui/src/accounts.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The Accounts tab's routes: the game's accounts, the ones players log
//! in with, not the web admin's own two.  Jacob's ask, 2026-09-29: make
//! one, list them, look at one as a card, change its owner's details or
//! its password, delete it.
//!
//! - `GET /Opus/Content/accounts` -- every account, by name.
//! - `GET /Opus/Content/accounts/job?id=N` -- where a job on the account
//!   desk is.
//! - `POST /Opus/wwwhook/accounts/create` -- a new account.
//! - `POST /Opus/wwwhook/accounts/edit?name=<account>` -- the owner's
//!   names and email.
//! - `POST /Opus/wwwhook/accounts/password?name=<account>` -- a new
//!   password.
//! - `POST /Opus/wwwhook/accounts/delete?name=<account>` -- deletes it,
//!   and its player, if in the world, is told ACCOUNT TERMINATED and
//!   taken out.
//!
//! Every one of them is `admin`'s only: `user` can't even see the list
//! (Jacob's call).  They only work while the server is running, since
//! the database and Security are server pieces.  The four that change
//! something carry `X-Opus: accounts`, the page's header, and their
//! fields come in the body as `key = value` lines.
//!
//! The list, an edit and a delete are quick database jobs, so the web
//! admin waits for them, up to DATABASE_WAIT.  Making an account and
//! changing a password need a hash, which waits in Security's line with
//! the logins, so those go to the account desk (conductor-accounts'
//! `desk.rs`) and the answer is the job's number at once; the page asks
//! after the job until it's done.  The web admin's one thread never waits
//! on a hash.

use std::collections::HashMap;
use std::time::Duration;

use conductor_accounts::desk::{self, Job};
use conductor_accounts::{self as accounts, Account, Edited};
use conductor_tools::scribe::{self, Channel};
use conductor_tools::server;

use crate::http::Request;
use crate::login::Role;
use crate::{json, only_admin, unescape, Answer, Next};

/// How long the web admin waits on Archivist for the list, an edit or a
/// delete.  Each is one quick statement; this is only so a database
/// that's stuck can't hold the page up for good.
const DATABASE_WAIT: Duration = Duration::from_secs(5);

/// The list, for the Accounts tab.
pub(crate) fn list(role: Role) -> (Answer, Next) {
    if let Err(answer) = allowed(role) {
        return (answer, Next::KeepGoing);
    }
    let answer = match accounts::list().wait_for(DATABASE_WAIT) {
        Some(Ok(list)) => Answer::new("200 OK", "application/json", json::accounts(&list)),
        Some(Err(e)) => Answer::plain("503 Service Unavailable", &format!("The accounts couldn't be read: {e}.")),
        None => Answer::plain("503 Service Unavailable", &format!("The database didn't answer within {} seconds.  \
            Try again in a moment.", DATABASE_WAIT.as_secs())),
    };
    (answer, Next::KeepGoing)
}

/// Where a job on the account desk is.
pub(crate) fn job(request: &Request, role: Role) -> (Answer, Next) {
    if let Err(answer) = allowed(role) {
        return (answer, Next::KeepGoing);
    }
    let Some(number) = request.query_value("id").and_then(|id| id.parse::<u64>().ok()) else {
        return (Answer::plain("400 Bad Request", "Which job?  /Opus/Content/accounts/job?id=N"), Next::KeepGoing);
    };
    let answer = match desk::outcome(number) {
        Some(outcome) => Answer::new("200 OK", "application/json", json::account_job(number, &outcome)),
        None => Answer::plain("404 Not Found", "The account desk doesn't know that job."),
    };
    (answer, Next::KeepGoing)
}

/// NEW ACCOUNT: every field checked here first, so a mistake comes back
/// as words beside its field, then the account desk hashes the password
/// and writes the row.
pub(crate) fn create(request: &Request, role: Role) -> (Answer, Next) {
    if let Err(answer) = changing(request, role) {
        return (answer, Next::KeepGoing);
    }
    let fields = fields(&request.body);
    let username = field(&fields, "username").trim().to_string();
    let first_name = field(&fields, "first_name").trim().to_string();
    let last_name = field(&fields, "last_name").trim().to_string();
    let email = field(&fields, "email").trim().to_string();
    let password = field(&fields, "password").to_string();
    let problems = accounts::check_new(&username, &first_name, &last_name, &email, &password,
                                       field(&fields, "password_again"));
    if !problems.is_empty() {
        return (Answer::new("400 Bad Request", "application/json", json::problems(&problems)), Next::KeepGoing);
    }

    let account = match Account::new(&username, &first_name, &last_name, &email) {
        Ok(account) => account,
        Err(e) => {
            return (Answer::plain("500 Internal Server Error", &format!("Fingerprinter couldn't make the account a \
                UUID: {e}.")), Next::KeepGoing);
        }
    };
    handed_in(desk::hand_in(Job::Create { account, password }))
}

/// SAVE on an account's card: the owner's names and email, straight in
/// the row.  An account whose player is in the world is changed the same
/// way; nothing about it is held anywhere else.
pub(crate) fn edit(request: &Request, role: Role) -> (Answer, Next) {
    if let Err(answer) = changing(request, role) {
        return (answer, Next::KeepGoing);
    }
    let name = match account_named(request) {
        Ok(name) => name,
        Err(answer) => return (answer, Next::KeepGoing),
    };
    let fields = fields(&request.body);
    let first_name = field(&fields, "first_name").trim();
    let last_name = field(&fields, "last_name").trim();
    let email = field(&fields, "email").trim();
    let problems = accounts::check_owner(first_name, last_name, email);
    if !problems.is_empty() {
        return (Answer::new("400 Bad Request", "application/json", json::problems(&problems)), Next::KeepGoing);
    }

    let answer = match accounts::edit(&name, first_name, last_name, email).wait_for(DATABASE_WAIT) {
        Some(Ok(Edited::Saved)) => {
            scribe::info(Channel::Security, &format!("The admin changed the owner's details on {name}."));
            Answer::new("200 OK", "application/json", "{\"saved\":true}")
        }
        Some(Ok(Edited::NoSuchAccount)) => no_such_account(&name),
        Some(Ok(Edited::EmailTaken)) => {
            let problems = ["email: Another account already has that email.".to_string()];
            Answer::new("400 Bad Request", "application/json", json::problems(&problems))
        }
        Some(Err(e)) => Answer::plain("503 Service Unavailable", &format!("The change couldn't be written: {e}.")),
        None => database_late(),
    };
    (answer, Next::KeepGoing)
}

/// CHANGE PASSWORD on an account's card: the new one, typed twice, goes
/// to the account desk to be hashed and written.  It takes at the
/// account's next login.
pub(crate) fn password(request: &Request, role: Role) -> (Answer, Next) {
    if let Err(answer) = changing(request, role) {
        return (answer, Next::KeepGoing);
    }
    let name = match account_named(request) {
        Ok(name) => name,
        Err(answer) => return (answer, Next::KeepGoing),
    };
    let fields = fields(&request.body);
    let password = field(&fields, "password").to_string();
    let problems = accounts::check_new_password(&password, field(&fields, "password_again"));
    if !problems.is_empty() {
        return (Answer::new("400 Bad Request", "application/json", json::problems(&problems)), Next::KeepGoing);
    }
    handed_in(desk::hand_in(Job::Password { username: name, password }))
}

/// DELETE on an account's card.  The row goes first; only once it's gone
/// is its player, if there's one in the world, told ACCOUNT TERMINATED
/// and taken out (Jacob's ask), and any unused ticket for it killed.
pub(crate) fn delete(request: &Request, role: Role) -> (Answer, Next) {
    if let Err(answer) = changing(request, role) {
        return (answer, Next::KeepGoing);
    }
    let name = match account_named(request) {
        Ok(name) => name,
        Err(answer) => return (answer, Next::KeepGoing),
    };
    let answer = match accounts::delete(&name).wait_for(DATABASE_WAIT) {
        Some(Ok(0)) => no_such_account(&name),
        Some(Ok(_)) => {
            let kicked = conductor_networking::terminate(&name);
            scribe::info(Channel::Security, &format!("The admin deleted the account {name}."));
            Answer::new("200 OK", "application/json", format!("{{\"deleted\":true,\"kicked\":{kicked}}}"))
        }
        Some(Err(e)) => Answer::plain("503 Service Unavailable", &format!("The account couldn't be deleted: {e}.")),
        // The database is late, not failed: the delete is in Archivist's
        // mailbox and the row goes when it gets there.  The player is taken
        // out now all the same, or they'd go on playing on a deleted
        // account until they left (the 0.0.1 review's B4).
        None => {
            let kicked = conductor_networking::terminate(&name);
            scribe::info(Channel::Security, &format!("The admin deleted the account {name}; the database is late \
                with the row, and its player was taken out ({kicked})."));
            database_late()
        }
    };
    (answer, Next::KeepGoing)
}

// ---------------------------------------------------------------------------
// What every route checks
// ---------------------------------------------------------------------------

/// `admin`, and the server running.  The answer to send if not.
fn allowed(role: Role) -> Result<(), Answer> {
    if let Some(turned_away) = only_admin(role) {
        return Err(turned_away);
    }
    if !matches!(server::status().state, server::State::Running) {
        return Err(Answer::plain("409 Conflict", "Accounts can only be looked at and changed while the server is \
            running.  START SERVER on the Control Panel."));
    }
    Ok(())
}

/// What `allowed()` checks, and the page's header on top, for the routes
/// that change something.  One without the header is a Warn, like a
/// server button or a kick without it.
fn changing(request: &Request, role: Role) -> Result<(), Answer> {
    if let Some(turned_away) = only_admin(role) {
        return Err(turned_away);
    }
    if request.header("x-opus") != Some("accounts") {
        scribe::warn(Channel::System, "The web admin turned away an account change that didn't come from its own \
            page.");
        return Err(Answer::plain("403 Forbidden", "Change accounts from the page."));
    }
    allowed(role)
}

/// The account `?name=` names, checked against the name rule.
fn account_named(request: &Request) -> Result<String, Answer> {
    let name = unescape(request.query_value("name").unwrap_or(""));
    if name.is_empty() {
        return Err(Answer::plain("400 Bad Request", "Which account?  ?name=<account>"));
    }
    if !accounts::username_allowed(&name) {
        return Err(no_such_account(&name));
    }
    Ok(name)
}

fn no_such_account(name: &str) -> Answer {
    Answer::plain("404 Not Found", &format!("There's no account called {name}."))
}

fn database_late() -> Answer {
    Answer::plain("503 Service Unavailable", &format!("The database didn't answer within {} seconds.  It may still \
        happen: look at the list again in a moment.", DATABASE_WAIT.as_secs()))
}

/// The answer to a job handed to the account desk: its number, for the
/// page to ask after.  202, since it isn't done yet.
fn handed_in(handed: Result<u64, String>) -> (Answer, Next) {
    match handed {
        Ok(number) => (Answer::new("202 Accepted", "application/json", format!("{{\"job\":{number}}}")),
                       Next::KeepGoing),
        Err(why) => (Answer::plain("409 Conflict", &why), Next::KeepGoing),
    }
}

// ---------------------------------------------------------------------------
// The body
// ---------------------------------------------------------------------------

/// The form's fields from the body, one `key = value` a line, the way
/// the page sends them.  A value is taken exactly as typed after the
/// `= `, spaces and all, since a password can start or end with one; the
/// route trims the fields that shouldn't.  A key that comes twice keeps
/// its first value.
fn fields(body: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    for line in body.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.strip_prefix(' ').unwrap_or(value);
        fields.entry(key.trim().to_ascii_lowercase()).or_insert_with(|| value.to_string());
    }
    fields
}

/// A field's value, or empty if it wasn't sent.
fn field<'a>(fields: &'a HashMap<String, String>, key: &str) -> &'a str {
    fields.get(key).map_or("", String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_is_taken_as_typed() {
        let fields = fields("username = jacob_01\r\npassword =  Two spaces in front 1! \nemail=\nnope\n\
            username = second\n");
        assert_eq!(field(&fields, "username"), "jacob_01");
        // One space after the = is the form's own; the rest are typed.
        assert_eq!(field(&fields, "password"), " Two spaces in front 1! ");
        assert_eq!(field(&fields, "email"), "");
        assert_eq!(field(&fields, "first_name"), "");
    }
}
