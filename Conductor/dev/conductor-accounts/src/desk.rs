//! File:       Opus/Conductor/dev/conductor-accounts/src/desk.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The account desk: the web admin's account jobs that need a password
//! hashed, done one at a time on a thread of their own, `account-desk`.
//! Making an account and changing its password both wait in Security's
//! line with the logins, which can take a while with players logging in,
//! and the web admin answers one request at a time.  So the web admin
//! hands the job in here, answers at once with the job's number, and the
//! page asks after it until it's done.  Nothing on the web admin's thread
//! ever waits on a hash.
//!
//! A server piece, like Security and Archivist that it leans on: the
//! launcher starts it after them and stops it before them, so a job it's
//! in the middle of still gets its hash and its write.  A job handed in
//! while it's stopped is turned away on the spot.
//!
//! What each job came to is kept for the page to ask about, the newest
//! KEEP_RESULTS of them, in memory only.  Never the password.

use std::collections::BTreeMap;
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;

use conductor_tools::scribe::{self, Channel};
use conductor_tools::security;
use conductor_tools::services::{self, State};
use conductor_tools::threads;

use crate::{Account, Created};

/// How many finished jobs are remembered for the page to ask after.  One
/// admin, one page: a handful is plenty.
const KEEP_RESULTS: usize = 50;

/// A job for the desk.  No `Debug` on purpose: it holds a password, and a
/// `{:?}` in a log line would print it.
pub enum Job {
    /// Make this account, with this password.  The fields have been
    /// checked already (`check_new()`).
    Create { account: Account, password: String },
    /// Give this account this password.  Checked already
    /// (`check_new_password()`).
    Password { username: String, password: String },
}

/// Where a job is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    Working,
    Done,
    Failed,
}

impl Progress {
    /// One word for the page.
    pub fn word(&self) -> &'static str {
        match self {
            Progress::Working => "working",
            Progress::Done => "done",
            Progress::Failed => "failed",
        }
    }
}

/// What a job came to, for the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub progress: Progress,
    /// What happened, in words for the admin.  For a failed job, why.
    pub text: String,
}

/// The desk while it runs: its mailbox and its thread.
struct Desk {
    mailbox: Sender<(u64, Job)>,
    handle: JoinHandle<()>,
}

static DESK: Mutex<Option<Desk>> = Mutex::new(None);

/// Every job's outcome by its number, and the next number.  A BTreeMap
/// keeps them in number order, so the oldest is the first to go.
static RESULTS: Mutex<(BTreeMap<u64, Outcome>, u64)> = Mutex::new((BTreeMap::new(), 1));

fn results() -> std::sync::MutexGuard<'static, (BTreeMap<u64, Outcome>, u64)> {
    RESULTS.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn desk() -> std::sync::MutexGuard<'static, Option<Desk>> {
    DESK.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Starts the desk's thread.  The launcher calls it on START SERVER,
/// after Security and Archivist.
pub fn start() {
    let mut guard = desk();
    if guard.is_some() {
        return;
    }
    let (mailbox, jobs) = mpsc::channel::<(u64, Job)>();
    match threads::spawn("account-desk", move || {
        // Runs until stop() drops the mailbox's other end, and finishes
        // every job already in it first.
        for (number, job) in jobs {
            let outcome = work(job);
            finish(number, outcome);
        }
    }) {
        Ok(handle) => {
            *guard = Some(Desk { mailbox, handle });
            services::set(services::ACCOUNT_DESK, State::Running, "Waiting for account jobs from the web admin.");
        }
        Err(e) => {
            scribe::error_with(Channel::System, &e, "The account desk couldn't start its thread.  Accounts can't be \
                made or have their passwords changed until the next START SERVER.");
            services::set(services::ACCOUNT_DESK, State::Stopped, &format!("Couldn't start its thread: {e}"));
        }
    }
}

/// Stops the desk once it has done every job already handed in.  The
/// launcher calls it on STOP SERVER, before Security and Archivist, so
/// those jobs still get their hash and their write.
pub fn stop() {
    let Some(desk) = desk().take() else {
        return;
    };
    services::set(services::ACCOUNT_DESK, State::Running, "Stopping: finishing the jobs already handed in.");
    drop(desk.mailbox);
    if desk.handle.join().is_err() {
        scribe::error(Channel::System, "The account desk's thread died.");
    }
    services::set(services::ACCOUNT_DESK, State::Stopped, "Stopped with the server.");
}

/// Hands a job in.  Its number, for `outcome()`; an `Err` in words when
/// the desk isn't running.
pub fn hand_in(job: Job) -> Result<u64, String> {
    let guard = desk();
    let Some(desk) = guard.as_ref() else {
        return Err("The account desk isn't running.  Accounts can only be changed while the server is running."
            .to_string());
    };
    let number = {
        let mut results = results();
        let number = results.1;
        results.1 += 1;
        results.0.insert(number, Outcome { progress: Progress::Working, text: "Waiting its turn.".to_string() });
        number
    };
    if desk.mailbox.send((number, job)).is_err() {
        finish(number, failed("The account desk has stopped."));
        return Err("The account desk has stopped.".to_string());
    }
    Ok(number)
}

/// What job `number` came to, or `None` for a number the desk doesn't
/// know (never handed in, or so old it's been forgotten).
pub fn outcome(number: u64) -> Option<Outcome> {
    results().0.get(&number).cloned()
}

/// Notes a job's outcome, and forgets the oldest finished ones past
/// KEEP_RESULTS.
fn finish(number: u64, outcome: Outcome) {
    let mut results = results();
    results.0.insert(number, outcome);
    while results.0.len() > KEEP_RESULTS {
        let Some(oldest) = results.0.keys().next().copied() else {
            break;
        };
        results.0.remove(&oldest);
    }
}

fn failed(why: &str) -> Outcome {
    Outcome { progress: Progress::Failed, text: why.to_string() }
}

fn done(what: &str) -> Outcome {
    Outcome { progress: Progress::Done, text: what.to_string() }
}

/// One job, start to finish, on the desk's thread.  It waits on
/// Archivist and Security as long as they take: nobody is waiting on this
/// thread but the page, which asks after the job.
fn work(job: Job) -> Outcome {
    match job {
        Job::Create { account, password } => create(account, password),
        Job::Password { username, password } => password_change(&username, &password),
    }
}

fn create(account: Account, password: String) -> Outcome {
    let name = account.username().to_string();

    // A name or email in use is found before the password costs a hash.
    // The write checks again, in case one was taken in between.
    match crate::taken(&name, &account.email).wait() {
        Ok((true, _)) => return failed(&format!("There's already an account called {name}.")),
        Ok((_, true)) => return failed(&format!("Another account already has the email {}.", account.email)),
        Ok(_) => {}
        Err(e) => return failed(&format!("The accounts table couldn't be read: {e}.")),
    }

    let hash = match security::hash_password(&password).wait() {
        Ok(hash) => hash,
        Err(e) => return failed(&format!("The password couldn't be hashed: {e}.  Nothing was written.")),
    };

    match crate::create(account, hash).wait() {
        Ok(Created::Made(_)) => {
            scribe::info(Channel::Security, &format!("The admin made the account {name}."));
            done(&format!("Made the account {name}."))
        }
        Ok(Created::NameTaken) => failed(&format!("There's already an account called {name}.")),
        Ok(Created::EmailTaken) => failed("Another account already has that email."),
        Err(e) => failed(&format!("The account couldn't be written: {e}.")),
    }
}

fn password_change(username: &str, password: &str) -> Outcome {
    let hash = match security::hash_password(password).wait() {
        Ok(hash) => hash,
        Err(e) => return failed(&format!("The password couldn't be hashed: {e}.  Nothing was written.")),
    };
    match crate::set_password(username, hash).wait() {
        Ok(0) => failed(&format!("There's no account called {username} any more.")),
        Ok(_) => {
            scribe::info(Channel::Security, &format!("The admin changed the password on {username}."));
            done(&format!("Changed the password on {username}.  It takes at their next login."))
        }
        Err(e) => failed(&format!("The password couldn't be written: {e}.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_is_turned_away_while_the_desk_is_stopped() {
        let job = Job::Password { username: "jacob_01".to_string(), password: "Abcdef1!".to_string() };
        assert!(hand_in(job).is_err());
    }

    #[test]
    fn only_the_newest_outcomes_are_kept() {
        // Numbers far past anything the other test could hand out.
        for number in 1_000_000..1_000_000 + KEEP_RESULTS as u64 + 5 {
            finish(number, done("x"));
        }
        assert_eq!(results().0.len(), KEEP_RESULTS);
        assert_eq!(outcome(1_000_000), None);
        assert_eq!(outcome(1_000_000 + KEEP_RESULTS as u64 + 4), Some(done("x")));
    }
}
