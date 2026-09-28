//! File:       Opus/Conductor/dev/conductor-tools/src/archivist.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Archivist, the database.  Its worker owns the connection to Postgres and
//! runs on a thread of its own, so a slow query or a database that has gone
//! away never holds up the rest of the server.
//!
//! The idea is a mailbox.  `execute()`, `query()`, `batch()` and
//! `transaction()` drop a job in Archivist's mailbox and hand back a
//! `Pending` straight away.  The caller carries on with its work and checks
//! the `Pending` when it wants the answer: `check()` never waits, `wait()`
//! does.  The worker takes the jobs one at a time, in the order they were
//! sent.
//!
//! This file is the front door.  The rest is in `archivist/`:
//! `settings.rs` reads `postgres.cfg`, `worker.rs` runs the worker and its
//! connection, `schemas.rs` gets the database into shape when it connects,
//! and `status.rs` keeps the running totals.
//!
//! Nothing in here can stop the server either.  If Postgres isn't there,
//! the jobs come back as errors, and the worker tries again on the next job.

mod schemas;
mod settings;
mod status;
mod worker;

use std::fmt;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Instant;

use status::JobKind;
use worker::{Job, Link};

// Rust note: `pub use` hands these on to whoever uses Archivist, so the
// launcher and the game can name a Row or a ToSql without adding the
// postgres crate to their own Cargo.toml.
pub use postgres::types::ToSql;
pub use postgres::{Error as PostgresError, Row, Transaction};
pub use settings::config_path;
pub use status::{SlowJob, Status};

/// One value for a `$1`, `$2` placeholder in the SQL.  Boxed, because the
/// values have to travel over to the worker's thread with the job.
///
/// ```text
/// let params: Vec<Param> = vec![Box::new(name.to_string()), Box::new(42_i32)];
/// ```
// Rust note: `dyn ToSql` means "any type Postgres knows how to send".
// `Send + Sync` are the compiler's promises that it is safe to hand to
// another thread.
pub type Param = Box<dyn ToSql + Send + Sync>;

// ---------------------------------------------------------------------------
// What comes back
// ---------------------------------------------------------------------------

/// Why a job didn't get done.
#[derive(Debug)]
pub enum ArchivistError {
    /// Archivist isn't running: `start()` hasn't happened, `stop()` already
    /// has, or its worker died.
    NotRunning,
    /// There's no connection to Postgres, and the reason why.
    NotConnected(String),
    /// Postgres got the job and said no.  Bad SQL, a missing table, a
    /// broken rule, a query that ran past the time limit, that kind of
    /// thing.
    Postgres(PostgresError),
}

impl fmt::Display for ArchivistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivistError::NotRunning => write!(f, "Archivist isn't running"),
            ArchivistError::NotConnected(why) => write!(f, "Archivist isn't connected to Postgres: {why}"),
            ArchivistError::Postgres(e) => write!(f, "Postgres: {e}"),
        }
    }
}

impl std::error::Error for ArchivistError {}

/// An answer that is on its way.  Every job hands one of these back
/// straight away, and the answer turns up in it once the worker gets to
/// the job.
///
/// Once `check()` has handed back an answer, the `Pending` is used up.
/// Ask it again and it says `NotRunning`, because there is nobody left on
/// the other end.
pub struct Pending<T> {
    reply: Receiver<Result<T, ArchivistError>>,
}

impl<T> Pending<T> {
    /// The answer if it's here, `None` if the worker hasn't got to the job
    /// yet.  Never waits, so this is the one the game loop uses.
    pub fn check(&self) -> Option<Result<T, ArchivistError>> {
        match self.reply.try_recv() {
            Ok(answer) => Some(answer),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(ArchivistError::NotRunning)),
        }
    }

    /// Waits for the answer.  Fine at startup and for the web admin, but
    /// never in the game loop, because this is exactly the blocking the
    /// worker was made to avoid.
    pub fn wait(self) -> Result<T, ArchivistError> {
        // Rust note: `recv()` fails only when the other end is gone
        // without answering, which means the worker isn't there.
        self.reply.recv().unwrap_or(Err(ArchivistError::NotRunning))
    }
}

// ---------------------------------------------------------------------------
// Starting and stopping
// ---------------------------------------------------------------------------

/// Reads `postgres.cfg` and starts the worker.  It comes back right away.
/// The first connect happens on the worker's thread, and the log says how
/// it went.  main calls this once, after Constellations has loaded.
pub fn start() {
    worker::start(settings::load(&config_path()));
}

/// Stops taking jobs, finishes the ones already in the mailbox, closes
/// the connection, and waits for the worker to end.  main calls this on
/// the way out.  A long query holds up shutdown until it's done, or until
/// the time limit in `postgres.cfg` cancels it.
pub fn stop() {
    worker::stop();
}

/// How Archivist is doing right now: running, connected, jobs waiting,
/// reads and writes, and the slow ones.
pub fn status() -> Status {
    worker::status()
}

// ---------------------------------------------------------------------------
// Jobs
// ---------------------------------------------------------------------------

/// Runs a statement that doesn't hand back rows (INSERT, UPDATE, DELETE,
/// CREATE) and gets back how many rows it changed.  The values for `$1`,
/// `$2` and so on go in `params`, never pasted into the SQL itself, so
/// what a player types can't turn into SQL.
pub fn execute(sql: &str, params: Vec<Param>) -> Pending<u64> {
    let owned = sql.to_string();
    post(sql, JobKind::Write, move |link| link.execute(&owned, &params))
}

/// Runs a SELECT (or anything else with RETURNING) and gets back the rows.
/// `params` works the same as in `execute()`.
///
/// ```text
/// let pending = archivist::query("SELECT account_username FROM accounts WHERE id = $1",
///                                vec![Box::new(7_i64)]);
/// // ... later ...
/// if let Some(Ok(rows)) = pending.check() {
///     let name: String = rows[0].get("account_username");
/// }
/// ```
pub fn query(sql: &str, params: Vec<Param>) -> Pending<Vec<Row>> {
    let owned = sql.to_string();
    post(sql, JobKind::Read, move |link| link.query(&owned, &params))
}

/// Runs several statements in one go, separated by `;`, with no params.
/// It's for SQL we wrote ourselves, never for anything with a player's
/// input in it.
pub fn batch(sql: &str) -> Pending<()> {
    let owned = sql.to_string();
    post(sql, JobKind::Other, move |link| link.batch(&owned))
}

/// Runs `work` inside a transaction, on the worker's thread.  Everything it
/// does to the database happens, or none of it does: if `work` hands back
/// an error, or anything in it fails, it all gets rolled back.  `name` is
/// what the slow-job log calls it.
///
/// Inside, `work` can query, look at what came back, and use it in the
/// next statement.  The one rule: never wait on anything else in there
/// (another `Pending`, a lock the game holds), because the worker is stuck
/// until `work` is done, and a `Pending` sent from inside would be waiting
/// on the very worker that's waiting on it.
///
/// ```text
/// let pending = archivist::transaction("make account", move |tx| {
///     let id: i64 = tx.query_one("INSERT INTO accounts (...) VALUES (...) RETURNING id",
///                                &[&name, &first, &last, &email, &hash])?.get(0);
///     tx.execute("INSERT INTO characters (account_id, ...) VALUES ($1, ...)", &[&id, ...])?;
///     Ok(id)
/// });
/// ```
// Rust note: `F` is whatever closure the caller writes.  `Send + 'static`
// means it can move to the worker's thread and borrows nothing that might
// be gone by the time it runs, so it has to own what it uses (`move`).
pub fn transaction<T, F>(name: &str, work: F) -> Pending<T>
where
    T: Send + 'static,
    F: FnOnce(&mut Transaction<'_>) -> Result<T, PostgresError> + Send + 'static,
{
    post(name, JobKind::Other, move |link| {
        let mut transaction = link.client()?.transaction().map_err(ArchivistError::Postgres)?;
        // If `work` fails, `?` returns before commit(), and dropping the
        // transaction rolls it back.
        let answer = work(&mut transaction).map_err(ArchivistError::Postgres)?;
        transaction.commit().map_err(ArchivistError::Postgres)?;
        Ok(answer)
    })
}

/// Packs a job up with a channel for its answer and puts it in the
/// mailbox.  If Archivist isn't running, the job is thrown away and the
/// channel with it, so the `Pending` hears `NotRunning` instead of waiting
/// forever.
fn post<T, W>(label: &str, kind: JobKind, work: W) -> Pending<T>
where
    T: Send + 'static,
    W: FnOnce(&mut Link) -> Result<T, ArchivistError> + Send + 'static,
{
    let (reply, answer) = mpsc::channel();
    worker::send(Job {
        label: label.to_string(),
        kind,
        posted_at: Instant::now(),
        // A dropped Pending means nobody wants the answer, so a failed
        // `send` is fine to ignore.
        work: Box::new(move |link: &mut Link| {
            let _ = reply.send(work(link));
        }),
    });
    Pending { reply: answer }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_with_no_archivist_says_so() {
        // The tests never call start(), so there is no worker to answer.
        let answer = execute("SELECT 1", Vec::new()).wait();
        assert!(matches!(answer, Err(ArchivistError::NotRunning)));

        let pending = query("SELECT 1", Vec::new());
        assert!(matches!(pending.check(), Some(Err(ArchivistError::NotRunning))));

        let pending = transaction("nothing", |_tx| Ok(()));
        assert!(matches!(pending.wait(), Err(ArchivistError::NotRunning)));
    }
}
