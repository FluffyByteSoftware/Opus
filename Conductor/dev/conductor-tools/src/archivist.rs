//! File:       Opus/Conductor/dev/conductor-tools/src/archivist.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Archivist, the database.  It owns the one connection to Postgres and
//! runs on a thread of its own, so a slow query or a database that has
//! gone away never holds up the rest of the server.
//!
//! The idea is a mailbox.  `execute()`, `query()` and `batch()` drop a job
//! in Archivist's mailbox and hand back a `Pending` straight away.  The
//! caller carries on with its work and checks the `Pending` when it wants
//! the answer: `check()` never waits, `wait()` does.  Archivist works
//! through the jobs one at a time, in the order they came in.
//!
//! Where to connect comes from `Content/cfg/postgres.cfg`, which holds the
//! password, so it never goes in the source.  A missing file gets written
//! with everything but the password filled in, and Archivist won't try to
//! connect until somebody puts one in.
//!
//! Nothing in here can stop the server either.  If Postgres isn't there,
//! the jobs come back as errors, and Archivist tries again on the next job.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use postgres::{Client, Config, NoTls};

use crate::constellations;
use crate::scribe::{self, Channel};

// Rust note: `pub use` hands these on to whoever uses Archivist, so the
// launcher and the game can name a Row or a ToSql without adding the
// postgres crate to their own Cargo.toml.
pub use postgres::Row;
pub use postgres::types::ToSql;

/// Where Archivist's settings live, under the Content folder.
const CONFIG_FILE: &str = "cfg/postgres.cfg";

/// How long a connect gets before we call it a failure.  Postgres is on
/// the same machine, so if it hasn't answered in 5 seconds it isn't going
/// to.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// After a failed connect, jobs fail straight away for this long instead
/// of each one trying again.  Otherwise a queue of 100 jobs with the
/// database down could mean 100 connect timeouts back to back.
const RETRY_WAIT: Duration = Duration::from_secs(5);

/// One value for a `$1`, `$2` placeholder in the SQL.  Boxed, because the
/// values have to travel over to Archivist's thread with the job.
///
/// ```text
/// let params: Vec<Param> = vec![Box::new(name.to_string()), Box::new(42_i32)];
/// ```
// Rust note: `dyn ToSql` means "any type Postgres knows how to send".
// `Send + Sync` are the compiler's promises that it is safe to hand to
// another thread.
pub type Param = Box<dyn ToSql + Send + Sync>;

// ---------------------------------------------------------------------------
// The settings
// ---------------------------------------------------------------------------

/// Everything `postgres.cfg` can hold.  The field names are the keys in the
/// file.
// Rust note: no `#[derive(Debug)]` here.  A derived Debug prints every
// field, and the password would end up in the log the first time somebody
// printed the settings.  There is a hand-written one further down.
#[derive(Clone, PartialEq)]
struct DbSettings {
    address: String,
    port: u16,
    database: String,
    username: String,
    password: String,
}

/// The built-in values.  Everything but the password, which only the admin
/// knows.
fn default_settings() -> DbSettings {
    DbSettings {
        address: "localhost".to_string(),
        port: 5432,
        database: "opusdb".to_string(),
        username: "opus_game".to_string(),
        password: String::new(),
    }
}

impl fmt::Debug for DbSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let password = if self.password.is_empty() { "(none)" } else { "(hidden)" };
        write!(f,
               "{}@{}:{}/{} password {password}",
               self.username,
               self.address,
               self.port,
               self.database)
    }
}

impl DbSettings {
    /// The settings in the shape the postgres crate wants.
    fn to_config(&self) -> Config {
        let mut config = Config::new();
        config.host(&self.address)
              .port(self.port)
              .dbname(&self.database)
              .user(&self.username)
              .password(&self.password)
              .application_name("Conductor")
              .connect_timeout(CONNECT_TIMEOUT);
        config
    }
}

// ---------------------------------------------------------------------------
// What comes back
// ---------------------------------------------------------------------------

/// Why a job didn't get done.
#[derive(Debug)]
pub enum ArchivistError {
    /// Archivist isn't running: `start()` hasn't happened, `stop()` already
    /// has, or its thread died.
    NotRunning,
    /// There's no connection to Postgres, and the reason why.
    NotConnected(String),
    /// Postgres got the job and said no.  Bad SQL, a missing table, a
    /// broken rule, that kind of thing.
    Postgres(postgres::Error),
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
/// straight away, and the answer turns up in it once Archivist gets to the
/// job.
///
/// Once `check()` has handed back an answer, the `Pending` is used up.
/// Ask it again and it says `NotRunning`, because there is nobody left on
/// the other end.
pub struct Pending<T> {
    reply: Receiver<Result<T, ArchivistError>>,
}

impl<T> Pending<T> {
    /// The answer if it's here, `None` if Archivist hasn't got to the job
    /// yet.  Never waits, so this is the one the game loop uses.
    pub fn check(&self) -> Option<Result<T, ArchivistError>> {
        match self.reply.try_recv() {
            Ok(answer) => Some(answer),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(ArchivistError::NotRunning)),
        }
    }

    /// Waits for the answer.  Fine at startup and in the admin's menu, but
    /// never in the game loop, because this is exactly the blocking the
    /// thread was made to avoid.
    pub fn wait(self) -> Result<T, ArchivistError> {
        // Rust note: `recv()` fails only when the other end is gone
        // without answering, which means Archivist's thread isn't there.
        self.reply.recv().unwrap_or(Err(ArchivistError::NotRunning))
    }
}

// ---------------------------------------------------------------------------
// The mailbox
// ---------------------------------------------------------------------------

/// One job in Archivist's mailbox.  Each carries its own `reply` channel,
/// which is where the answer goes and what the `Pending` listens on.
enum Job {
    Execute { sql: String, params: Vec<Param>, reply: Sender<Result<u64, ArchivistError>> },
    Query { sql: String, params: Vec<Param>, reply: Sender<Result<Vec<Row>, ArchivistError>> },
    Batch { sql: String, reply: Sender<Result<(), ArchivistError>> },
    Stop,
}

// Rust note: a channel is a queue between threads.  The `Sender` end can
// be copied and used from anywhere; the `Receiver` end belongs to one
// thread, which is Archivist's.  It replaces the lock-and-queue a C#
// version would build by hand.
static JOBS: OnceLock<Sender<Job>> = OnceLock::new();
static WORKER: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Reads `postgres.cfg` and starts Archivist's thread.  It comes back right
/// away.  The first connect happens on the thread, and the log says how it
/// went.  main calls this once, after Constellations has loaded.
pub fn start() {
    let path = config_path();
    let settings = load_settings(&path);

    let (jobs, mailbox) = mpsc::channel();
    if JOBS.set(jobs).is_err() {
        scribe::warn(Channel::Database, "Archivist was asked to start twice.  The first one stands.");
        return;
    }

    let spawned = thread::Builder::new()
        .name("archivist".to_string())
        .spawn(move || run(settings, mailbox));

    match spawned {
        Ok(handle) => {
            let mut guard = WORKER.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = Some(handle);
        }
        // Every job from here on comes back NotRunning.  The server keeps
        // going, it just has no database.
        Err(e) => scribe::error_with(Channel::Database, &e, "Archivist couldn't start its thread.  \
            There is no database this run."),
    }
}

/// Finishes the jobs already in the mailbox, closes the connection, and
/// ends Archivist's thread.  main calls this on the way out.  It waits for
/// the last job, so a long query holds up shutdown until it's done.
pub fn stop() {
    if let Some(jobs) = JOBS.get() {
        let _ = jobs.send(Job::Stop);
    }

    let handle = {
        let mut guard = WORKER.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.take()
    };
    if let Some(handle) = handle {
        if handle.join().is_err() {
            scribe::error(Channel::Database, "Archivist's thread had already died.");
        }
    }
}

/// Runs a statement that doesn't hand back rows (INSERT, UPDATE, DELETE,
/// CREATE) and gets back how many rows it changed.  The values for `$1`,
/// `$2` and so on go in `params`, never pasted into the SQL itself, so
/// what a player types can't turn into SQL.
pub fn execute(sql: &str, params: Vec<Param>) -> Pending<u64> {
    post(|reply| Job::Execute { sql: sql.to_string(), params, reply })
}

/// Runs a SELECT (or anything else with RETURNING) and gets back the rows.
/// `params` works the same as in `execute()`.
///
/// ```text
/// let pending = archivist::query("SELECT name FROM accounts WHERE id = $1", vec![Box::new(7_i64)]);
/// // ... later ...
/// if let Some(Ok(rows)) = pending.check() {
///     let name: String = rows[0].get("name");
/// }
/// ```
pub fn query(sql: &str, params: Vec<Param>) -> Pending<Vec<Row>> {
    post(|reply| Job::Query { sql: sql.to_string(), params, reply })
}

/// Runs several statements in one go, separated by `;`, with no params.
/// It's for SQL we wrote ourselves, like the schema, never for anything
/// with a player's input in it.
pub fn batch(sql: &str) -> Pending<()> {
    post(|reply| Job::Batch { sql: sql.to_string(), reply })
}

/// Makes the reply channel, builds the job around it, and drops it in the
/// mailbox.
///
/// If Archivist isn't running there's no mailbox, or nobody is reading it,
/// and the job is thrown away.  The reply channel goes with it, so the
/// `Pending` hears `NotRunning` instead of waiting forever.
fn post<T>(make_job: impl FnOnce(Sender<Result<T, ArchivistError>>) -> Job) -> Pending<T> {
    let (reply, answer) = mpsc::channel();
    if let Some(jobs) = JOBS.get() {
        let _ = jobs.send(make_job(reply));
    }
    Pending { reply: answer }
}

// ---------------------------------------------------------------------------
// Archivist's thread
// ---------------------------------------------------------------------------

/// Everything below runs on Archivist's own thread and nowhere else.
fn run(settings: DbSettings, mailbox: Receiver<Job>) {
    let mut link = Link { settings, client: None, failed_at: None };

    // Try once right away, so the log says at startup whether Postgres is
    // there instead of waiting for the first job to find out.  Whatever
    // went wrong is already in the log.
    let _ = link.client();

    // Rust note: `for job in mailbox` waits for the next job each time
    // around, and ends if every Sender is gone.  Ours lives in a static,
    // so really it ends on Stop.
    for job in mailbox {
        // A dropped Pending means nobody wants the answer, so a failed
        // `send` on a reply is fine to ignore.
        match job {
            Job::Execute { sql, params, reply } => {
                let _ = reply.send(link.execute(&sql, &params));
            }
            Job::Query { sql, params, reply } => {
                let _ = reply.send(link.query(&sql, &params));
            }
            Job::Batch { sql, reply } => {
                let _ = reply.send(link.batch(&sql));
            }
            Job::Stop => break,
        }
    }

    if link.client.is_some() {
        scribe::info(Channel::Database, "Archivist closed its connection to Postgres.");
    }
}

/// The connection, and what we need to make it again.
struct Link {
    settings: DbSettings,
    client: Option<Client>,
    /// When the last connect failed.  `None` while things are fine.
    failed_at: Option<Instant>,
}

impl Link {
    /// The connection, making it first if there isn't one or the old one
    /// has dropped.
    fn client(&mut self) -> Result<&mut Client, ArchivistError> {
        if self.client.as_ref().is_some_and(|client| client.is_closed()) {
            self.client = None;
            scribe::warn(Channel::Database, "Archivist lost its connection to Postgres.  \
                It will reconnect on the next job.");
        }
        if self.client.is_none() {
            self.connect()?;
        }
        self.client
            .as_mut()
            .ok_or_else(|| ArchivistError::NotConnected("the connection vanished".to_string()))
    }

    fn connect(&mut self) -> Result<(), ArchivistError> {
        // Already in the log once, from load_settings().
        if self.settings.password.is_empty() {
            return Err(ArchivistError::NotConnected(format!("there's no password in {}",
                                                            config_path().display())));
        }
        if self.failed_at.is_some_and(|failed_at| failed_at.elapsed() < RETRY_WAIT) {
            return Err(ArchivistError::NotConnected("the last try failed a moment ago".to_string()));
        }

        let where_to = format!("{} at {}:{} as {}",
                               self.settings.database,
                               self.settings.address,
                               self.settings.port,
                               self.settings.username);

        match self.settings.to_config().connect(NoTls) {
            Ok(mut client) => {
                // The first thing we pull from the database is its own
                // version.  It proves the connection works end to end.
                let version: String = match client.query_one("SELECT version()", &[]) {
                    Ok(row) => row.try_get(0).unwrap_or_default(),
                    Err(_) => String::new(),
                };
                scribe::info(Channel::Database, &format!("Archivist connected to {where_to}.  {version}"));
                self.client = Some(client);
                self.failed_at = None;
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
                Err(ArchivistError::NotConnected(e.to_string()))
            }
        }
    }

    fn execute(&mut self, sql: &str, params: &[Param]) -> Result<u64, ArchivistError> {
        let params = borrow_params(params);
        self.client()?.execute(sql, &params).map_err(ArchivistError::Postgres)
    }

    fn query(&mut self, sql: &str, params: &[Param]) -> Result<Vec<Row>, ArchivistError> {
        let params = borrow_params(params);
        self.client()?.query(sql, &params).map_err(ArchivistError::Postgres)
    }

    fn batch(&mut self, sql: &str) -> Result<(), ArchivistError> {
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

// ---------------------------------------------------------------------------
// postgres.cfg
// ---------------------------------------------------------------------------

/// The full path of `postgres.cfg`.
pub fn config_path() -> PathBuf {
    constellations::content_dir().join(CONFIG_FILE)
}

/// Reads `postgres.cfg`, or writes one if it's missing.  Complaints go to
/// the log, and anything we couldn't read keeps its default.
fn load_settings(path: &Path) -> DbSettings {
    let mut settings = default_settings();

    match fs::read_to_string(path) {
        Ok(text) => {
            for problem in parse_text(&text, &mut settings) {
                scribe::warn(Channel::Database, &format!("{}, {problem}", path.display()));
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => write_default_file(path),
        Err(e) => {
            scribe::error_with(Channel::Database, &e, &format!("Archivist can't read {}.", path.display()));
        }
    }

    if settings.password.is_empty() {
        scribe::error(Channel::Database, &format!("NO POSTGRES PASSWORD, FILL IT IN BY HAND: {}.  \
            THERE IS NO DATABASE UNTIL IT IS, AND CONDUCTOR NEEDS A RESTART AFTER.",
            path.display()));
    }

    settings
}

/// Goes through the file a line at a time, the same rules as
/// `conductor_globals.cfg`: `#` comments, `key = value`, keys in any case,
/// the later of two wins.  Every complaint comes back, one per line.
fn parse_text(text: &str, settings: &mut DbSettings) -> Vec<String> {
    let mut problems = Vec::new();
    let mut set_on: HashMap<String, usize> = HashMap::new();

    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // The line itself isn't in this complaint, because it might be the
        // password line with the `=` left out.
        let Some((key, value)) = line.split_once('=') else {
            problems.push(format!("line {number} isn't a \"key = value\" line.  Ignored."));
            continue;
        };
        let key = key.trim().to_ascii_lowercase();

        match apply_setting(settings, &key, value.trim()) {
            Ok(()) => {
                if let Some(earlier) = set_on.get(&key) {
                    problems.push(format!("line {number}: {key} was already set on line {earlier}.  \
                        This one wins."));
                }
                set_on.insert(key, number);
            }
            Err(problem) => problems.push(format!("line {number}: {problem}  Ignored.")),
        }
    }

    problems
}

/// Puts one value into the settings, if the key is one we know and the
/// value makes sense for it.
fn apply_setting(settings: &mut DbSettings, key: &str, value: &str) -> Result<(), String> {
    match key {
        "address" => settings.address = parse_word(key, value)?,
        "port" => settings.port = parse_port(value)?,
        "database" => settings.database = parse_word(key, value)?,
        "username" => settings.username = parse_word(key, value)?,
        // An empty password is allowed here.  It's what a fresh file has,
        // and load_settings() shouts about it.
        "password" => settings.password = value.to_string(),
        _ => return Err(format!("There is no setting called {key}.")),
    }
    Ok(())
}

/// A name or an address.  Anything but nothing.
fn parse_word(key: &str, value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err(format!("{key} is empty."));
    }
    Ok(value.to_string())
}

fn parse_port(value: &str) -> Result<u16, String> {
    match value.parse::<u16>() {
        Ok(port) if port > 0 => Ok(port),
        _ => Err(format!("port has to be a number from 1 to 65535, and \"{value}\" isn't.")),
    }
}

/// The whole text of a `postgres.cfg` holding these settings.  Whatever
/// this writes, parse_text() has to read back without a complaint.
fn file_text(settings: &DbSettings) -> String {
    format!("\
# Where Archivist finds Postgres.  One \"key = value\" a line, and \"#\"
# starts a comment.  Conductor wrote this file because there wasn't one.
# It holds the database password, so it never gets committed (all of
# Content/ is ignored) and never gets pasted anywhere.
# Stop the server before editing it; it is only read at startup.

# The machine Postgres is on, and its port.
address = {}
port = {}

# The database, and the role Conductor logs in as.
database = {}
username = {}

# The role's password, as it is, no quotes.
password = {}
", settings.address, settings.port, settings.database, settings.username, settings.password)
}

/// There was no `postgres.cfg`, so we write one with the defaults and an
/// empty password for the admin to fill in.
fn write_default_file(path: &Path) {
    if let Some(folder) = path.parent() {
        let _ = fs::create_dir_all(folder);
    }

    match fs::write(path, file_text(&default_settings())) {
        Ok(()) => scribe::info(Channel::Database, &format!("No postgres.cfg, so Archivist wrote one: {}",
                                                          path.display())),
        Err(e) => scribe::error_with(Channel::Database, &e, &format!("No postgres.cfg, and Archivist \
            can't write one at {}.", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled_in() -> DbSettings {
        DbSettings {
            address: "db.example".to_string(),
            port: 6543,
            database: "otherdb".to_string(),
            username: "someone".to_string(),
            password: "p@ss = word".to_string(),
        }
    }

    #[test]
    fn what_we_write_we_can_read() {
        let mut read_back = default_settings();
        let problems = parse_text(&file_text(&filled_in()), &mut read_back);

        assert!(problems.is_empty(), "complaints: {problems:?}");
        assert_eq!(read_back, filled_in());
    }

    #[test]
    fn the_default_file_reads_clean() {
        let mut settings = default_settings();
        assert!(parse_text(&file_text(&default_settings()), &mut settings).is_empty());
        assert_eq!(settings, default_settings());
    }

    #[test]
    fn a_bad_port_keeps_the_default() {
        for bad in ["0", "65536", "five", ""] {
            let mut settings = default_settings();
            let problems = parse_text(&format!("port = {bad}\n"), &mut settings);

            assert_eq!(problems.len(), 1, "port = {bad}");
            assert_eq!(settings.port, 5432);
        }
    }

    #[test]
    fn an_unknown_key_is_a_complaint() {
        let mut settings = default_settings();
        let problems = parse_text("# a comment\n\npasword = typo\n", &mut settings);

        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("line 3:"));
        assert_eq!(settings, default_settings());
    }

    #[test]
    fn a_broken_line_does_not_show_itself() {
        // A password line with the `=` forgotten must not end up in the log.
        let mut settings = default_settings();
        let problems = parse_text("password hunter2\n", &mut settings);

        assert_eq!(problems.len(), 1);
        assert!(!problems[0].contains("hunter2"));
    }

    #[test]
    fn debug_never_shows_the_password() {
        let shown = format!("{:?}", filled_in());
        assert!(!shown.contains("p@ss"));
        assert!(shown.contains("(hidden)"));
    }

    #[test]
    fn a_job_with_no_archivist_says_so() {
        // The tests never call start(), so there is no thread to answer.
        let answer = execute("SELECT 1", Vec::new()).wait();
        assert!(matches!(answer, Err(ArchivistError::NotRunning)));

        let pending = query("SELECT 1", Vec::new());
        assert!(matches!(pending.check(), Some(Err(ArchivistError::NotRunning))));
    }
}
