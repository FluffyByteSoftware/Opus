//! File:       Opus/Conductor/dev/conductor-tools/src/archivist/worker.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Archivist's workers.  Each one is a thread with its own connection to
//! Postgres, and they all take jobs from the one mailbox.
//!
//! There is always one.  When more than `busy_queue` jobs are waiting,
//! another one starts, up to `max_workers`.  An extra one that has had
//! nothing to do for `idle_worker_seconds` closes its connection and ends.
//! So a quiet server holds one connection, and a busy one holds a few.
//!
//! The catch with more than one worker: two jobs can run at the same time,
//! so the second one sent can finish first.  When the order matters, put
//! the steps in one transaction, or wait for the first answer before
//! sending the next job.  `max_workers = 1` puts everything back in order.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use postgres::{Client, NoTls};

use super::schemas;
use super::settings::DbSettings;
use super::status::{self, Status};
use super::{ArchivistError, Param, ToSql};
use crate::scribe::{self, Channel};

/// After a failed connect, this worker's jobs fail straight away for this
/// long instead of each one trying again.  Otherwise a queue of 100 jobs
/// with the database down could mean 100 connect timeouts back to back.
const RETRY_WAIT: Duration = Duration::from_secs(5);

/// The least time between starting one extra worker and the next.  A new
/// worker needs a moment to connect, and the jobs keep piling up while it
/// does, so without this one busy second could start every worker there is.
const GROW_WAIT: Duration = Duration::from_secs(2);

/// One job in the mailbox.  `work` is the job itself, packed up with the
/// channel its answer goes back on, so the worker doesn't need to know
/// what kind of job it is.  It just runs it.
pub(super) struct Job {
    /// The start of the SQL, or a transaction's name.  For the slow-job
    /// log.
    pub(super) label: String,
    pub(super) posted_at: Instant,
    // Rust note: `Box<dyn FnOnce(...)>` is a function packed up to be
    // called later, exactly once, along with anything it captured.  Like
    // a C# Action, and `Send` means it may move to another thread.
    pub(super) work: Box<dyn FnOnce(&mut Link) + Send>,
}

/// The mailbox.
struct Mailbox {
    jobs: VecDeque<Job>,
    /// Set by `stop()`.  No new jobs get in, and the workers end once the
    /// mailbox is empty.  It starts out true, so jobs sent before `start()`
    /// are turned away.
    stopping: bool,
}

static MAILBOX: Mutex<Mailbox> = Mutex::new(Mailbox { jobs: VecDeque::new(), stopping: true });

// Rust note: a `Condvar` lets a thread sleep until another one says
// something changed.  The workers sleep on it while the mailbox is empty,
// and `send()` wakes one up.
static JOB_POSTED: Condvar = Condvar::new();

/// The settings, set once by `start()`.  Every worker reads them.
static SETTINGS: OnceLock<DbSettings> = OnceLock::new();

/// Every worker thread, so `stop()` can wait for them all.
static HANDLES: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());

/// How many workers are running, counting ones still starting up.
static WORKERS: AtomicUsize = AtomicUsize::new(0);

/// Gives every worker its own number for the log.
static NEXT_NUMBER: AtomicUsize = AtomicUsize::new(1);

/// When the last extra worker was started.
static LAST_GROWTH: Mutex<Option<Instant>> = Mutex::new(None);

fn mailbox() -> MutexGuard<'static, Mailbox> {
    MAILBOX.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Opens the mailbox and starts the first worker.
pub(super) fn start(settings: DbSettings) {
    if SETTINGS.set(settings).is_err() {
        scribe::warn(Channel::Database, "Archivist was asked to start twice.  The first one stands.");
        return;
    }

    mailbox().stopping = false;
    if take_worker_slot(1) {
        spawn_worker(false);
    }
}

/// Closes the mailbox, lets the workers finish what's already in it, and
/// waits for every one of them to end.
pub(super) fn stop() {
    mailbox().stopping = true;
    JOB_POSTED.notify_all();

    let handles: Vec<JoinHandle<()>> = {
        let mut guard = HANDLES.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.drain(..).collect()
    };
    for handle in handles {
        if handle.join().is_err() {
            scribe::error(Channel::Database, "An Archivist worker had already died.");
        }
    }
}

/// Puts a job in the mailbox and wakes a worker.  If the mailbox is
/// closed, the job is dropped, and the answer channel with it, so whoever
/// sent it hears `NotRunning`.  Starts another worker if it's getting busy.
pub(super) fn send(job: Job) {
    let waiting = {
        let mut mailbox = mailbox();
        if mailbox.stopping {
            return;
        }
        mailbox.jobs.push_back(job);
        mailbox.jobs.len()
    };
    JOB_POSTED.notify_one();

    if let Some(settings) = SETTINGS.get() {
        if waiting > settings.busy_queue {
            grow(settings, waiting);
        }
    }
}

/// How the workers and the mailbox are doing, with the totals.
pub(super) fn status() -> Status {
    let waiting = mailbox().jobs.len();
    status::snapshot(WORKERS.load(Ordering::SeqCst), waiting)
}

/// Starts one more worker, unless there are already `max_workers` or one
/// was started a moment ago.
fn grow(settings: &DbSettings, waiting: usize) {
    {
        let mut last = LAST_GROWTH.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if last.is_some_and(|when| when.elapsed() < GROW_WAIT) {
            return;
        }
        if !take_worker_slot(settings.max_workers) {
            return;
        }
        *last = Some(Instant::now());
    }

    scribe::info(Channel::Database, &format!("Archivist has {waiting} jobs waiting, so it's starting \
        another worker."));
    spawn_worker(true);
}

/// Counts one more worker if there's room under `max`.  Checking and
/// counting happen in one step, so two threads can't both take the last
/// slot.
fn take_worker_slot(max: usize) -> bool {
    WORKERS
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| if count < max { Some(count + 1) } else { None })
        .is_ok()
}

/// Starts a worker thread.  Its slot has already been taken.  `extra`
/// workers end when they've been idle a while; the first one never does.
fn spawn_worker(extra: bool) {
    let number = NEXT_NUMBER.fetch_add(1, Ordering::SeqCst);
    let spawned = thread::Builder::new()
        .name(format!("archivist-{number}"))
        .spawn(move || run(number, extra));

    match spawned {
        Ok(handle) => {
            let mut guard = HANDLES.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            // Workers that have already ended don't need waiting for.
            guard.retain(|handle| !handle.is_finished());
            guard.push(handle);
        }
        Err(e) => {
            WORKERS.fetch_sub(1, Ordering::SeqCst);
            scribe::error_with(Channel::Database, &e, "Archivist couldn't start a worker thread.");
        }
    }
}

// ---------------------------------------------------------------------------
// A worker's thread
// ---------------------------------------------------------------------------

/// Everything from here down runs on a worker's own thread.
fn run(number: usize, extra: bool) {
    let Some(settings) = SETTINGS.get() else {
        WORKERS.fetch_sub(1, Ordering::SeqCst);
        return;
    };
    let idle_limit = Duration::from_secs(settings.idle_worker_seconds);
    let slow_limit = Duration::from_millis(settings.slow_job_ms);
    let mut link = Link { settings, number, client: None, failed_at: None };

    // Connect right away, so the log says at startup whether Postgres is
    // there instead of waiting for the first job to find out.  Whatever
    // went wrong is already in the log.
    let _ = link.client();

    while let Some(job) = next_job(extra, idle_limit) {
        let waited = job.posted_at.elapsed();
        let started = Instant::now();
        (job.work)(&mut link);
        status::record(&job.label, waited, started.elapsed(), slow_limit);
    }

    link.disconnect();
    WORKERS.fetch_sub(1, Ordering::SeqCst);
    if extra {
        scribe::info(Channel::Database, &format!("Archivist worker {number} had nothing to do, so it closed."));
    }
}

/// Waits for the next job.  `None` means this worker is done: the mailbox
/// is closed and empty, or this is an extra worker that sat idle too long.
fn next_job(extra: bool, idle_limit: Duration) -> Option<Job> {
    let mut mailbox = mailbox();
    loop {
        if let Some(job) = mailbox.jobs.pop_front() {
            return Some(job);
        }
        if mailbox.stopping {
            return None;
        }

        // Rust note: `wait_timeout` lets go of the mailbox while it sleeps
        // and takes it back when it wakes, so the other workers and
        // `send()` can get in meanwhile.
        let (guard, timeout) = JOB_POSTED.wait_timeout(mailbox, idle_limit)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        mailbox = guard;

        if timeout.timed_out() && extra && mailbox.jobs.is_empty() {
            return None;
        }
    }
}

/// One worker's connection, and what we need to make it again.
pub(super) struct Link {
    settings: &'static DbSettings,
    number: usize,
    client: Option<Client>,
    /// When the last connect failed.  `None` while things are fine.
    failed_at: Option<Instant>,
}

impl Link {
    /// The connection, making it first if there isn't one or the old one
    /// has dropped.
    pub(super) fn client(&mut self) -> Result<&mut Client, ArchivistError> {
        if self.client.as_ref().is_some_and(|client| client.is_closed()) {
            self.disconnect();
            scribe::warn(Channel::Database, &format!("Archivist worker {} lost its connection to Postgres.  \
                It will reconnect on the next job.", self.number));
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
            return Err(ArchivistError::NotConnected(format!("there's no password in {}",
                                                            super::config_path().display())));
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
                scribe::info(Channel::Database, &format!("Archivist worker {} connected to {where_to}.  {version}",
                                                         self.number));
                schemas::get_in_shape(&mut client);

                self.client = Some(client);
                self.failed_at = None;
                status::CONNECTED.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            Err(e) => {
                // Log the first failure only.  With the database down,
                // every job would add the same line otherwise.
                if self.failed_at.is_none() {
                    scribe::error_with(Channel::Database, &e, &format!("Archivist worker {} can't connect to \
                        {where_to}.  Its jobs will fail until it can.  It tries again on the next job.",
                        self.number));
                }
                self.failed_at = Some(Instant::now());
                Err(ArchivistError::NotConnected(e.to_string()))
            }
        }
    }

    /// Lets go of the connection, if there is one.  Dropping a Client is
    /// what closes it.
    fn disconnect(&mut self) {
        if self.client.take().is_some() {
            status::CONNECTED.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub(super) fn execute(&mut self, sql: &str, params: &[Param]) -> Result<u64, ArchivistError> {
        let params = borrow_params(params);
        self.client()?.execute(sql, &params).map_err(ArchivistError::Postgres)
    }

    pub(super) fn query(&mut self, sql: &str, params: &[Param]) -> Result<Vec<postgres::Row>, ArchivistError> {
        let params = borrow_params(params);
        self.client()?.query(sql, &params).map_err(ArchivistError::Postgres)
    }

    pub(super) fn batch(&mut self, sql: &str) -> Result<(), ArchivistError> {
        self.client()?.batch_execute(sql).map_err(ArchivistError::Postgres)
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
