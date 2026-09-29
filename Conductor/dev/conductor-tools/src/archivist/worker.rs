//! File:       Opus/Conductor/dev/conductor-tools/src/archivist/worker.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Archivist's worker: one thread with one connection to Postgres, taking
//! jobs out of the mailbox one at a time, in the order they were sent.
//!
//! One worker is on purpose.  Nobody waits on it anyway: sending a job
//! comes straight back with a `Pending`, and the game carries on while the
//! worker gets to it.  With two workers, two jobs could run at once and the
//! second one sent could finish first, so a SELECT could miss the UPDATE
//! sent just before it.  One worker means that can't happen.
//!
//! What we do for speed instead is keep every statement the worker has
//! prepared.  Postgres works out how to run a piece of SQL the first time
//! it sees it, and after that the worker hands it the prepared version, so
//! the same query sent 10,000 times gets planned once.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use postgres::{Client, NoTls, Statement};

use super::schemas;
use super::settings::DbSettings;
use super::status::{self, JobKind, Status};
use super::{ArchivistError, Param, ToSql};
use crate::scribe::{self, Channel};
use crate::services::{self, State};
use crate::threads;

/// After a failed connect, jobs fail straight away for this long instead
/// of each one trying again.  Otherwise a queue of 100 jobs with the
/// database down could mean 100 connect timeouts back to back.
const RETRY_WAIT: Duration = Duration::from_secs(5);

/// The most prepared statements the worker keeps.  The game sends the
/// same few dozen queries over and over, so this is plenty.  If something
/// ever builds SQL on the fly and fills it, the list is emptied and starts
/// again, which costs a little planning and nothing else.
const MOST_PREPARED: usize = 500;

/// One job in the mailbox.  `work` is the job itself, packed up with the
/// channel its answer goes back on, so the worker doesn't need to know
/// what kind of job it is.  It just runs it.
pub(super) struct Job {
    /// The start of the SQL, or a transaction's name.  For the slow-job
    /// log.
    pub(super) label: String,
    pub(super) kind: JobKind,
    pub(super) posted_at: Instant,
    // Rust note: `Box<dyn FnOnce(...)>` is a function packed up to be
    // called later, exactly once, along with anything it captured.  Like
    // a C# Action, and `Send` means it may move to another thread.
    pub(super) work: Box<dyn FnOnce(&mut Link) + Send>,
}

// Rust note: a channel is a queue between threads.  The `Sender` end can
// be used from anywhere; the `Receiver` end belongs to the worker.  When
// `stop()` drops the Sender, the worker gets the jobs still queued and
// then hears that nothing more is coming.
static MAILBOX: Mutex<Option<Sender<Job>>> = Mutex::new(None);
static WORKER: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// How many jobs are in the mailbox that the worker hasn't picked up yet.
static WAITING: AtomicUsize = AtomicUsize::new(0);

/// Opens the mailbox and starts the worker.  The settings go with the
/// worker onto its thread, and are read again from the file on every
/// start, so a STOP and a START from the web admin pick up a changed
/// `postgres.cfg`.
pub(super) fn start(settings: DbSettings) {
    if lock(&WORKER).as_ref().is_some_and(|handle| !handle.is_finished()) {
        scribe::warn(Channel::Database, "Archivist was asked to start while it's already running.  \
            The running one stands.");
        return;
    }

    services::set(services::ARCHIVIST, State::Starting, "Connecting to Postgres.");
    let (sender, receiver) = mpsc::channel();
    let spawned = threads::spawn("archivist", move || run(receiver, settings));

    match spawned {
        Ok(handle) => {
            *lock(&MAILBOX) = Some(sender);
            *lock(&WORKER) = Some(handle);
        }
        // Every job from here on comes back NotRunning.  The server keeps
        // going, it just has no database.
        Err(e) => {
            scribe::error_with(Channel::Database, &e, "Archivist couldn't start its thread.  \
                There is no database this run.");
            services::set(services::ARCHIVIST, State::Stopped, &format!("Couldn't start its thread: {e}"));
        }
    }
}

/// Closes the mailbox, lets the worker finish what's already in it, and
/// waits for it to end.
pub(super) fn stop() {
    // Dropping the Sender is what closes the mailbox.
    lock(&MAILBOX).take();

    let handle = lock(&WORKER).take();
    if let Some(handle) = handle {
        if handle.join().is_err() {
            scribe::error(Channel::Database, "Archivist's thread had already died.");
        }
    }
}

/// Puts a job in the mailbox.  If the mailbox is closed, the job is
/// dropped, and the answer channel with it, so whoever sent it hears
/// `NotRunning`.
pub(super) fn send(job: Job) {
    let mailbox = lock(&MAILBOX);
    let Some(sender) = mailbox.as_ref() else {
        return;
    };
    WAITING.fetch_add(1, Ordering::SeqCst);
    if sender.send(job).is_err() {
        WAITING.fetch_sub(1, Ordering::SeqCst);
    }
}

/// How the worker and the mailbox are doing, with the totals.
pub(super) fn status() -> Status {
    let running = lock(&WORKER).as_ref().is_some_and(|handle| !handle.is_finished());
    status::snapshot(running, WAITING.load(Ordering::SeqCst))
}

/// The lock idiom, for the statics above.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------
// The worker's thread
// ---------------------------------------------------------------------------

/// Everything from here down runs on the worker's own thread.
fn run(mailbox: Receiver<Job>, settings: DbSettings) {
    let slow_limit = Duration::from_millis(settings.slow_job_ms);
    let mut link = Link { settings, client: None, failed_at: None, prepared: HashMap::new() };

    // Connect right away, so the log says at startup whether Postgres is
    // there instead of waiting for the first job to find out.  Whatever
    // went wrong is already in the log.
    let _ = link.client();

    // Rust note: `for job in mailbox` waits for the next job each time
    // around, and ends once the Sender is gone and the queue is empty.
    for job in mailbox {
        WAITING.fetch_sub(1, Ordering::SeqCst);
        let waited = job.posted_at.elapsed();
        let started = Instant::now();
        (job.work)(&mut link);
        status::record(&job.label, job.kind, waited, started.elapsed(), slow_limit);
    }

    if link.client.is_some() {
        link.disconnect();
        scribe::info(Channel::Database, "Archivist closed its connection to Postgres.");
    }
    services::set(services::ARCHIVIST, State::Stopped, "Shut down.");
}

/// The connection, and what we need to make it again.
pub(super) struct Link {
    settings: DbSettings,
    client: Option<Client>,
    /// When the last connect failed.  `None` while things are fine.
    failed_at: Option<Instant>,
    /// Every statement prepared on this connection, by its SQL.  They
    /// belong to the connection, so a new connection starts with none.
    prepared: HashMap<String, Statement>,
}

impl Link {
    /// The connection, making it first if there isn't one or the old one
    /// has dropped.
    pub(super) fn client(&mut self) -> Result<&mut Client, ArchivistError> {
        if self.client.as_ref().is_some_and(|client| client.is_closed()) {
            self.disconnect();
            scribe::warn(Channel::Database, "Archivist lost its connection to Postgres.  \
                It will reconnect on the next job.");
            services::set(services::ARCHIVIST, State::Trouble, "Lost its connection to Postgres.  \
                It reconnects on the next job.");
        }
        if self.client.is_none() {
            self.connect()?;
        }
        self.client
            .as_mut()
            .ok_or_else(|| ArchivistError::NotConnected("the connection vanished".to_string()))
    }

    fn connect(&mut self) -> Result<(), ArchivistError> {
        // Already in the log once, from settings::load().
        if self.settings.password.is_empty() {
            let why = format!("there's no password in {}", super::config_path().display());
            services::set(services::ARCHIVIST, State::Trouble, &format!("Can't connect: {why}."));
            return Err(ArchivistError::NotConnected(why));
        }
        if self.failed_at.is_some_and(|failed_at| failed_at.elapsed() < RETRY_WAIT) {
            return Err(ArchivistError::NotConnected("the last try failed a moment ago".to_string()));
        }

        let where_to = self.settings.where_to();
        match self.settings.to_config().connect(NoTls) {
            Ok(mut client) => {
                // The first thing we pull from the database is its own
                // version.  It proves the connection works end to end.
                let version: String = match client.query_one("SELECT version()", &[]) {
                    Ok(row) => row.try_get(0).unwrap_or_default(),
                    Err(_) => String::new(),
                };
                scribe::info(Channel::Database, &format!("Archivist connected to {where_to}.  {version}"));
                schemas::get_in_shape(&mut client);

                self.client = Some(client);
                self.failed_at = None;
                status::set_connected(true);
                services::set(services::ARCHIVIST, State::Running, &format!("Connected to {where_to}"));
                Ok(())
            }
            Err(e) => {
                // Log the first failure only.  With the database down,
                // every job would add the same line otherwise.
                if self.failed_at.is_none() {
                    scribe::error_with(Channel::Database, &e, &format!("Archivist can't connect to {where_to}.  \
                        Jobs will fail until it can.  It tries again on the next job."));
                }
                self.failed_at = Some(Instant::now());
                services::set(services::ARCHIVIST, State::Trouble, &format!("Can't connect to {where_to}: {e}"));
                Err(ArchivistError::NotConnected(e.to_string()))
            }
        }
    }

    /// Lets go of the connection, and the statements prepared on it.
    /// Dropping a Client is what closes it.
    fn disconnect(&mut self) {
        self.client = None;
        self.prepared.clear();
        status::set_connected(false);
    }

    /// The prepared version of `sql`, preparing it first if this is the
    /// first time we've seen it on this connection.
    fn prepare(&mut self, sql: &str) -> Result<Statement, ArchivistError> {
        // Rust note: cloning a Statement is cheap.  It's a shared handle
        // to the one Postgres holds, not a copy of it.
        if let Some(statement) = self.prepared.get(sql) {
            return Ok(statement.clone());
        }

        let statement = self.client()?.prepare(sql).map_err(ArchivistError::Postgres)?;
        if self.prepared.len() >= MOST_PREPARED {
            self.prepared.clear();
        }
        self.prepared.insert(sql.to_string(), statement.clone());
        Ok(statement)
    }

    pub(super) fn execute(&mut self, sql: &str, params: &[Param]) -> Result<u64, ArchivistError> {
        let statement = self.prepare(sql)?;
        let params = borrow_params(params);
        self.client()?.execute(&statement, &params).map_err(ArchivistError::Postgres)
    }

    pub(super) fn query(&mut self, sql: &str, params: &[Param]) -> Result<Vec<postgres::Row>, ArchivistError> {
        let statement = self.prepare(sql)?;
        let params = borrow_params(params);
        self.client()?.query(&statement, &params).map_err(ArchivistError::Postgres)
    }

    /// Batches aren't prepared, since there can be several statements in
    /// one.  A batch is also how the game would change a table, and a
    /// prepared statement made before a table changed can refuse to run
    /// after.  So the list is emptied, and the next query gets prepared
    /// fresh.
    pub(super) fn batch(&mut self, sql: &str) -> Result<(), ArchivistError> {
        let answer = self.client()?.batch_execute(sql).map_err(ArchivistError::Postgres);
        self.prepared.clear();
        answer
    }
}

/// The postgres crate wants the params as a list of borrowed values, and
/// ours are boxed.  This makes the list.
// Rust note: `&**param` goes through the reference and then the Box to
// the value inside, and borrows that.  Dropping `Send` from the type along
// the way is allowed; the crate just doesn't ask for it.
fn borrow_params(params: &[Param]) -> Vec<&(dyn ToSql + Sync)> {
    params.iter().map(|param| &**param as &(dyn ToSql + Sync)).collect()
}
