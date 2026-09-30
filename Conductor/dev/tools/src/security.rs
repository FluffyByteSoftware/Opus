//! File:       Opus/Conductor/dev/tools/src/security.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Security, the password hasher.  A password gets hashed with Argon2id
//! and a random salt, and only the hash is ever kept.  Hashing is one-way
//! on purpose: we can check a password, and we can never get one back.
//! It is slow on purpose too, because a slow hash is what makes a stolen
//! accounts table expensive to crack.  Every login pays that cost.
//!
//! What goes in the accounts table is one line of text in the standard
//! "PHC string" format, and it carries its own settings:
//!
//! ```text
//! $argon2id$v=19$m=65536,t=1,p=1$<salt>$<hash>
//! ```
//!
//! So when we change the settings in a year, old accounts still check
//! against the settings they were made with, and nobody gets locked out.
//!
//! # Where the time goes, and how we spend less of it
//!
//! Argon2 fills a stretch of memory (`m`, in KiB) with hash output, then
//! makes `t` passes over it, each block mixed from two others that the
//! data picks.  The CPU time is close to `m` times `t`: every pass
//! touches every KiB once.  The lanes (`p`) only spread the work over
//! threads when the crate is built with its rayon feature, which we don't
//! use, so `p` stays 1.  What an attacker with a graphics card pays for is
//! the memory, not the passes: a card has thousands of cores and not much
//! RAM for each.  So the trade we make, wherever we can, is RAM for CPU:
//!
//! - **One pass (`t=1`) instead of two, at the same memory.**  Half the
//!   CPU for the same memory an attacker has to find for every guess.
//!   RFC 9106's first recommendation for Argon2id is one pass over as
//!   much memory as can be spared.  To make a hash harder from here, raise
//!   the memory, not the passes.
//! - **The memory is handed out once and kept.**  The crate's own
//!   `hash_password_into()` asks the OS for a fresh 64 MiB on every hash
//!   and gives it back after.  A fresh 64 MiB is 16,384 pages the kernel
//!   has to find, zero and map the first time each one is touched, which
//!   is CPU spent on nothing that is ours.  The worker allots its arena
//!   once when it starts, touches every page then, and every hash after
//!   that runs in the same arena.
//! - **On Linux, the arena is marked for huge pages** (`madvise`).
//!   Argon2 jumps around its memory at random, and each jump into a page
//!   the CPU hasn't seen lately costs a walk through the page tables.
//!   With 2 MiB pages instead of 4 KiB ones, 64 MiB is 32 pages instead of
//!   16,384, and the CPU's own list of recent pages (the TLB) covers all
//!   of it.  Whether the kernel goes along with the hint is its call.  On
//!   the dev machine it's set to `always`, so the arena had huge pages
//!   before we asked and the hint bought nothing there; on a kernel set to
//!   `madvise` it's the difference.
//! - **One thread, one hash at a time, everybody else in line.**  This is
//!   the hard limit: one login is hashed at a time, and the rest wait in
//!   the queue in the order they arrived.  Stratum's tick-sim showed 50
//!   logins hashing on 50 threads took 1.2 seconds each and put the tick
//!   over budget for as long as they ran; one at a time, each took 72 ms
//!   and all 50 were done in 3.6 seconds.  One at a time is also what
//!   makes one arena enough.
//!
//! Measured on the dev machine (`cargo test --release`, 2026-09-29), one
//! pass at 64 MiB: 37.5 ms with fresh memory each hash, 29.7 ms in the
//! arena.  The arena saves about 20%, and that share holds from 19 MiB to
//! 256.  Two passes at 64 MiB, Stratum's setting, is 64 ms.  Time is
//! linear in memory: 128 MiB one pass is 62 ms, 256 is 127.
//!
//! What's not here: rayon lanes (a crate, and threads started outside
//! `threads::spawn()`), and not hashing at all when a player reconnects
//! (a login token from Fingerprinter instead; that's accounts' job once
//! there are accounts).
//!
//! # How it's used
//!
//! Nothing waits on the worker.  `hash_password()` and `verify_password()`
//! hand back a `Ticket` the moment the job is in line: the `Pending` the
//! answer arrives in, the same as Archivist and DiskMan, so the game loop
//! never waits on a hash, plus `place()`, which says how many are ahead
//! and about how long that is.  The worker numbers every job, counts the
//! ones it has finished, and keeps a running average of how long one
//! takes; the place is worked out from those three numbers, so a client
//! in line can be told "3 ahead of you, about 120 ms" while it waits.  A
//! login that fails always says the same thing, whether the name or the
//! password was wrong; that's the caller's job.  Ours is the other half:
//! a name with no account still costs a hash, in the same line as
//! everybody else (`verify_no_account()`), so nobody can tell a real name
//! from a made-up one with a stopwatch, and `pad_login_time()` evens out
//! the rest.
//!
//! This file hands back strings and checks strings.  It never touches the
//! accounts table (that's accounts' job, through Archivist), and it never
//! logs a password, not even a wrong one.  Standard library plus argon2,
//! our second crate.  Nobody should write their own password hash, and
//! that includes us.

use std::fmt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use argon2::password_hash::phc::{Output, ParamsString, Salt};
use argon2::{Algorithm, Argon2, Block, Params, PasswordHash, Version};

use crate::fingerprinter;
use crate::pending::NotRunning;
use crate::scribe::{self, Channel};
use crate::services::{self, State};
use crate::threads;

// Rust note: `#[cfg(...)]` keeps a line in or out of the build depending on
// what it's being built for.  On Linux, `advise_huge_pages()` is
// `linux::advise_huge_pages()`, and the other two files aren't compiled.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::advise_huge_pages;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::advise_huge_pages;

#[cfg(not(any(target_os = "linux", windows)))]
mod other;
#[cfg(not(any(target_os = "linux", windows)))]
use other::advise_huge_pages;

// ---------------------------------------------------------------------------
// The numbers
// ---------------------------------------------------------------------------

/// How much memory one hash uses, in KiB.  This is the knob that matters:
/// memory is what makes a graphics card's job expensive, and it's the
/// one we raise when we want a harder hash.  The arena is this big, and
/// it's held for as long as Conductor runs.  64 MiB is what RFC 9106
/// suggests for a machine short on memory.  One pass over it measured
/// 30 ms on the dev machine (the benchmark at the bottom); Stratum's two
/// passes measured 64 ms on the same machine.
const HASH_MEMORY_KIB: u32 = 64 * 1024;

/// How many passes over that memory.  One, on purpose: see the top of the
/// file.  More passes cost more CPU for the same memory.
const HASH_PASSES: u32 = 1;

/// Lanes.  The crate only runs them in parallel with a feature that pulls
/// in another crate, so more than one buys nothing here.
const HASH_LANES: u32 = 1;

/// How long the hash itself is, in bytes.  The crate's default, and
/// plenty: nobody guesses 256 bits.
const HASH_BYTES: usize = 32;

/// How long the salt is, in bytes.  16 is what the PHC format recommends
/// (a UUID is 16 bytes, and those are "very good salts").
const SALT_BYTES: usize = 16;

/// How long every login attempt takes, at least, in milliseconds.  It has
/// to sit above one hash, or a slow hash pokes out over the top and the
/// timing leak is back.  One hash measured 30 ms on the dev machine, so
/// 150 leaves room for a slower machine and a busy moment.  Time spent
/// waiting in line doesn't come
/// into it: a made-up name waits in the same line, so a long wait says
/// nothing about which kind of name it was.  It also caps every
/// connection at a few guesses a second.
const MIN_LOGIN_MILLIS: u64 = 150;

/// The password rules.  Jacob's.  Between the two lengths, printable ASCII
/// only (anything on a US keyboard, spaces included), and at least one
/// digit, one capital letter and one symbol.
///
/// The maximum isn't about the hash: ten million characters hash in the
/// same time as eight, because Argon2 boils the password down with a
/// quick hash first.  It's about how big a "password" we let a client
/// send us.
const MIN_PASSWORD_CHARS: usize = 8;
const MAX_PASSWORD_CHARS: usize = 128;

/// What one hash is assumed to take until the worker has done one and
/// measured it: the benchmark's 30 ms.  After that the running average
/// takes over, so a slower machine tells the truth about itself.
const HASH_ESTIMATE_MILLIS: u64 = 30;

/// How much of the running average one hash moves: a new time counts for
/// an eighth, the old average for the rest.  Enough to follow a machine
/// that's warming up or under load, without one slow hash swinging it.
const AVERAGE_WEIGHT: u64 = 8;

/// How long the worker waits for a job before checking in with the
/// services list, so a worker with nothing to do still shows as alive.
const CHECK_IN: Duration = Duration::from_secs(1);

// ---------------------------------------------------------------------------
// What a caller gets back
// ---------------------------------------------------------------------------

/// Why a job didn't get an answer.  A wrong password isn't one of these;
/// that's `Ok(false)` from `verify_password()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// Security isn't running: `start()` hasn't happened, it has stopped,
    /// or its worker died.
    NotRunning,
    /// The hash couldn't be made or checked.  The words say why: the OS
    /// wouldn't give random bytes for the salt, or the stored line isn't
    /// one of our hashes.
    Failed(String),
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::NotRunning => write!(f, "Security isn't running"),
            SecurityError::Failed(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for SecurityError {}

impl NotRunning for SecurityError {
    fn not_running() -> SecurityError {
        SecurityError::NotRunning
    }
}

/// An answer that is on its way: `check()` never waits, `wait()` does.
/// The workings are in `pending.rs`, shared with Archivist and DiskMan.
pub type Pending<T> = crate::pending::Pending<T, SecurityError>;

/// A job in line: the answer on its way, and where the job stands.
pub struct Ticket<T> {
    /// The job's number in the line, counting from 1 since Conductor
    /// started.  0 means it never got in line (the worker wasn't running)
    /// and the answer is already "not running".
    number: u64,
    pending: Pending<T>,
}

impl<T> Ticket<T> {
    /// The answer if it's here, `None` if the worker hasn't got to the job
    /// yet.  Never waits, so this is the one the game loop uses.
    pub fn check(&self) -> Option<Result<T, SecurityError>> {
        self.pending.check()
    }

    /// Waits up to `wait` for the answer; `None` if it hasn't come by
    /// then, and the ticket is still good.  Networking waits a second at
    /// a time and tells the client its `place()` in between.
    pub fn wait_for(&self, wait: Duration) -> Option<Result<T, SecurityError>> {
        self.pending.wait_for(wait)
    }

    /// Waits for the answer.  Fine on a connection's own thread, never in
    /// the game loop.
    pub fn wait(self) -> Result<T, SecurityError> {
        self.pending.wait()
    }

    /// Where the job stands right now: how many are ahead of it, and about
    /// how long until its answer.  For telling a waiting client.
    pub fn place(&self) -> Place {
        place(self.number, DONE.load(Ordering::Relaxed), average_hash())
    }
}

/// Where a job stands in the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    /// Jobs still to finish before this one, the one being hashed right
    /// now included.  0 means this one is next, or already done.
    pub ahead: u64,
    /// About how long until this job's answer: the jobs ahead plus this
    /// one, at the running average.  Zero once it's done.  "About" because
    /// the average is of whatever the machine has been doing lately.
    pub wait: Duration,
}

/// How many jobs have been given a number.  The next job gets this plus
/// one, under WORKER's lock, so numbers go in the same order as the line.
static NUMBERED: AtomicU64 = AtomicU64::new(0);

/// How many jobs the worker has finished, in order, so a job's number
/// minus this is how many are still ahead of it.
static DONE: AtomicU64 = AtomicU64::new(0);

/// The running average of one job, in nanoseconds.  Starts at the
/// benchmark's number and follows what the machine actually does.
static AVERAGE_NANOS: AtomicU64 = AtomicU64::new(HASH_ESTIMATE_MILLIS * 1_000_000);

/// The running average, as a Duration.
fn average_hash() -> Duration {
    Duration::from_nanos(AVERAGE_NANOS.load(Ordering::Relaxed))
}

/// Where job `number` stands when `done` jobs have finished and one takes
/// `average`.  Kept apart from the statics so the tests can pin it.
fn place(number: u64, done: u64, average: Duration) -> Place {
    if number == 0 || number <= done {
        return Place { ahead: 0, wait: Duration::ZERO };
    }
    let ahead = number - done - 1;
    let wait = Duration::from_nanos((average.as_nanos() as u64).saturating_mul(ahead + 1));
    Place { ahead, wait }
}

// ---------------------------------------------------------------------------
// The worker
// ---------------------------------------------------------------------------

/// One piece of work in the line, and where its answer goes.
// Rust note: each job carries the sending half of a channel of its own.
// The worker puts the answer in it, and the `Pending` the caller holds is
// the other half.
enum Job {
    /// Hash a new password.
    Hash {
        password: String,
        reply: Sender<Result<String, SecurityError>>,
    },
    /// Check a typed password against the line from the accounts table.
    Verify {
        password: String,
        stored: String,
        reply: Sender<Result<bool, SecurityError>>,
    },
    /// Spend one hash's worth of time on a name with no account.  The
    /// answer only says it's done.
    NoAccount {
        password: String,
        reply: Sender<Result<(), SecurityError>>,
    },
}

/// The running worker.  Held in WORKER below between `start()` and
/// `stop()`.
struct Worker {
    /// The back of the line.  Letting go of it is what tells the worker
    /// to finish up.
    queue: Sender<Job>,
    handle: JoinHandle<()>,
}

// Rust note: the same shape as Archivist's mailbox.  `None` means the
// worker isn't running.
static WORKER: Mutex<Option<Worker>> = Mutex::new(None);

/// Starts the worker.  It comes straight back; the worker allots its
/// arena on its own thread and says on the Services tab when it's ready.
/// The launcher calls this after Fingerprinter (the salts come from
/// there) and before Archivist.
pub fn start() {
    let mut worker = WORKER.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if worker.is_some() {
        scribe::warn(Channel::Security, "Security was asked to start twice.  The first one stands.");
        return;
    }

    services::set(services::SECURITY, State::Starting, "Allotting the hash arena.");
    let (queue, jobs) = mpsc::channel();
    match threads::spawn("security", move || run(jobs)) {
        Ok(handle) => *worker = Some(Worker { queue, handle }),
        Err(e) => {
            services::set(services::SECURITY, State::Trouble, &format!("Its thread wouldn't start: {e}"));
            scribe::error_with(Channel::Security, &e, "SECURITY'S WORKER THREAD WOULDN'T START.  \
                No password can be hashed or checked, so nobody can make an account or log in.");
        }
    }
}

/// Stops the worker once it has finished everything already in line, and
/// waits for it.  Anything queued after this hears "not running".  Does
/// nothing if it isn't running.  The launcher calls this before Archivist
/// stops, so a hash on its way to the accounts table still gets there.
pub fn stop() {
    let taken = {
        let mut worker = WORKER.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        worker.take()
    };
    let Some(Worker { queue, handle }) = taken else {
        return;
    };

    // Dropping the back of the line is the signal.  The worker finishes
    // what's already queued, then ends.
    drop(queue);
    let _ = handle.join();
    services::set(services::SECURITY, State::Stopped, "Stopped.");
    scribe::info(Channel::Security, "Security has stopped.");
}

/// The worker's thread: allot the arena, say we're up, then serve.
fn run(jobs: Receiver<Job>) {
    let mut arena = Arena::allot(current_params().block_count(), true);
    let note = arena.describe();
    services::set(services::SECURITY, State::Running, &note);
    scribe::info(Channel::Security, &format!("Security is running.  {note}"));

    serve(jobs, &mut arena);
}

/// One job at a time, in the order they arrived, until the line closes.
/// Checks in with the services list between jobs, and once a second when
/// there are none.
fn serve(jobs: Receiver<Job>, arena: &mut Arena) {
    // Rust note: recv_timeout() sleeps until a job arrives or the time is
    // up.  Once stop() has let go of the back of the line and the line is
    // empty, it says Disconnected instead, and that ends the loop.
    loop {
        match jobs.recv_timeout(CHECK_IN) {
            Ok(job) => {
                let started = Instant::now();
                do_job(job, arena);
                note_finished(started.elapsed());
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        services::seen(services::SECURITY);
    }
}

/// One more job done, and the running average moved toward how long it
/// took.  The count goes up after the average, so a place read between
/// the two is off by one job's time, never by a job.
fn note_finished(took: Duration) {
    let took = took.as_nanos() as u64;
    // Rust note: fetch_update reads the value, runs the closure on it and
    // writes what comes back, trying again if another thread got in
    // between.  Nobody else writes this one, so it never has to.
    let _ = AVERAGE_NANOS.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |average| {
        Some(average - average / AVERAGE_WEIGHT + took / AVERAGE_WEIGHT)
    });
    DONE.fetch_add(1, Ordering::Relaxed);
}

/// Does one job and sends back the answer.  A failure is logged here, on
/// the worker, since the caller has long since moved on.
fn do_job(job: Job, arena: &mut Arena) {
    // Rust note: the sends use `let _ =` because a caller that has stopped
    // waiting for its answer is no reason to stop the worker.
    match job {
        Job::Hash { password, reply } => {
            let answer = make_hash(&current_params(), &password, arena);
            if let Err(why) = &answer {
                scribe::error(Channel::Security, &format!("SECURITY COULDN'T HASH A PASSWORD ({why}).  \
                    No account should be made until this is fixed."));
            }
            let _ = reply.send(answer.map_err(SecurityError::Failed));
        }
        Job::Verify { password, stored, reply } => {
            let answer = check_hash(&password, &stored, arena);
            if let Err(why) = &answer {
                scribe::error(Channel::Security, &format!("Security was handed a stored password hash it \
                    can't read ({why}).  That account's row is damaged.  Treating the password as wrong."));
            }
            let _ = reply.send(answer.map_err(SecurityError::Failed));
        }
        Job::NoAccount { password, reply } => {
            // Hashing the typed password at our settings costs the same as
            // checking it against an account made at our settings.  The
            // hash goes straight in the bin.
            let _ = make_hash(&current_params(), &password, arena);
            let _ = reply.send(Ok(()));
        }
    }
}

/// Puts a job in line and hands back its number.  If the worker isn't
/// running, or has died, the job is dropped here, and dropping it drops
/// its reply end, which is what tells the `Pending` that nobody is
/// coming; the number is 0.
fn queue(job: Job) -> u64 {
    let worker = WORKER.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(running) = worker.as_ref() else {
        return 0;
    };
    match running.queue.send(job) {
        Ok(()) => NUMBERED.fetch_add(1, Ordering::Relaxed) + 1,
        Err(_) => 0,
    }
}

// ---------------------------------------------------------------------------
// The arena
// ---------------------------------------------------------------------------

/// The memory every hash runs in.  Allotted once, when the worker starts,
/// and kept for good.
struct Arena {
    blocks: Vec<Block>,
    /// What the OS said when asked for huge pages.  `Ok` means it took the
    /// hint, not that they landed; only the kernel knows that.
    huge_pages: Result<(), String>,
}

impl Arena {
    /// Allots `block_count` KiB.  With `want_huge_pages`, the OS gets the
    /// huge page hint first, then every page is touched, so the kernel
    /// hands them out as huge pages where it's willing to.  The order
    /// matters: a hint after the pages are already in is mostly ignored.
    fn allot(block_count: usize, want_huge_pages: bool) -> Arena {
        // Rust note: with_capacity() reserves the room without touching
        // it.  resize() is what writes every block, and that's when the
        // kernel actually hands the pages over.
        let mut blocks: Vec<Block> = Vec::with_capacity(block_count);
        let huge_pages = if want_huge_pages {
            advise_huge_pages(blocks.as_mut_ptr().cast::<u8>(), block_count * Block::SIZE)
                .map_err(|e| e.to_string())
        } else {
            Err("not asked for".to_string())
        };
        blocks.resize(block_count, Block::new());

        Arena { blocks, huge_pages }
    }

    /// How big it is, in whole MiB.
    fn mib(&self) -> usize {
        self.blocks.len() * Block::SIZE / (1024 * 1024)
    }

    /// One line for the Services tab and the log.
    fn describe(&self) -> String {
        match &self.huge_pages {
            Ok(()) => format!("{} MiB arena, huge pages asked for.", self.mib()),
            Err(why) => format!("{} MiB arena on ordinary pages ({why}).", self.mib()),
        }
    }
}

// ---------------------------------------------------------------------------
// What the rest of the server calls
// ---------------------------------------------------------------------------

/// The settings every new hash is made with.
fn current_params() -> Params {
    // Rust note: Params::new() says no to settings Argon2 doesn't allow.
    // Ours are constants that the tests check, so `expect` can't actually
    // trip here, and if it somehow does, "can't make a hash" is worth
    // stopping the server over.
    Params::new(HASH_MEMORY_KIB, HASH_PASSES, HASH_LANES, Some(HASH_BYTES))
        .expect("Argon2 turned down Security's hash settings")
}

/// Hashes a new password, with a fresh random salt, and hands back the
/// line that goes in the accounts table, when the worker gets to it.
///
/// The salt comes from the OS's random source, through Fingerprinter.  If
/// that fails (it doesn't, on Linux) there's no safe way to make a hash,
/// so the worker says so on the Security channel and the answer is the
/// error.  The caller should not create the account.
///
/// This doesn't check the password rules.  Call `check_password_rules()`
/// first, and tell the player what they got wrong.
pub fn hash_password(password: &str) -> Ticket<String> {
    let (reply, pending) = Pending::new();
    let number = queue(Job::Hash { password: password.to_string(), reply });
    Ticket { number, pending }
}

/// Checks a typed password against the line from the accounts table.
/// True if it matches, when the worker gets to it.  It takes about as
/// long whether it matches or not.
///
/// The settings come out of the stored line, not out of the constants
/// above, so an account hashed under old settings still checks.
///
/// A stored line that can't be read as a hash is a damaged row, and the
/// worker logs an Error on the Security channel (without the line itself
/// in it: a salt and a hash have no business in a log).  The answer is
/// the error, and the password is wrong as far as the caller is
/// concerned.
pub fn verify_password(password: &str, stored: &str) -> Ticket<bool> {
    let (reply, pending) = Pending::new();
    let number = queue(Job::Verify { password: password.to_string(), stored: stored.to_string(), reply });
    Ticket { number, pending }
}

/// For a login whose name has no account.  It waits in the same line as
/// `verify_password()` and takes as long, so nobody can tell the two
/// apart by how long the answer took.  There's nothing to check the
/// password against, so the answer is always no, and the caller knows
/// that already.  That's why it hands back nothing.
pub fn verify_no_account(password: &str) -> Ticket<()> {
    let (reply, pending) = Pending::new();
    let number = queue(Job::NoAccount { password: password.to_string(), reply });
    Ticket { number, pending }
}

/// Says whether a new password is allowed, and if not, why, in words
/// meant for the player.  Doesn't log, doesn't hash.
pub fn check_password_rules(password: &str) -> Result<(), String> {
    // Rust note: `chars().count()` counts characters and `len()` counts
    // bytes.  They're the same for ASCII, which is all we allow, but the
    // length is checked first and the ASCII rule second, so it has to be
    // the one that's right either way.
    let length = password.chars().count();
    if length < MIN_PASSWORD_CHARS {
        return Err(format!("A password needs at least {} characters.", MIN_PASSWORD_CHARS));
    }
    if length > MAX_PASSWORD_CHARS {
        return Err(format!("A password can't be longer than {} characters.", MAX_PASSWORD_CHARS));
    }

    // Printable ASCII is space (32) through tilde (126).  Nothing outside
    // that, because the same accented letter can arrive as two different
    // byte sequences, and then a password that looks right doesn't match.
    if !password.chars().all(|c| (' '..='~').contains(&c)) {
        return Err("A password can only use letters, digits, spaces and the symbols on a US keyboard."
            .to_string());
    }

    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err("A password needs at least one digit.".to_string());
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err("A password needs at least one capital letter.".to_string());
    }
    if !password.chars().any(|c| c.is_ascii_punctuation()) {
        return Err("A password needs at least one symbol, like ! or #.".to_string());
    }

    Ok(())
}

/// Makes a login attempt take at least MIN_LOGIN_MILLIS from `started`,
/// however much or little work it did.  The caller notes the clock when
/// the attempt arrives, does the work (or skips it, for a name that
/// doesn't exist), and calls this before answering.
///
/// Both kinds of name cost a hash, but the paths still differ a little (a
/// real account gets read from the database, and a made-up one doesn't),
/// and a packet we couldn't read costs nothing at all.  This evens all of
/// that out, so every attempt answers at the same moment.  The waiting
/// happens on the connection's own thread, so it only ever holds up the
/// one client who is logging in.
pub fn pad_login_time(started: Instant) {
    let floor = Duration::from_millis(MIN_LOGIN_MILLIS);
    let spent = started.elapsed();
    if spent < floor {
        thread::sleep(floor - spent);
    }
}

// ---------------------------------------------------------------------------
// The work
// ---------------------------------------------------------------------------

// These don't log, which is what lets the tests run them.  The worker
// above is the same thing plus the complaint.

/// A new salt from the OS, then the PHC string for `password` at `params`.
fn make_hash(params: &Params, password: &str, arena: &mut Arena) -> Result<String, String> {
    let mut salt = [0u8; SALT_BYTES];
    fingerprinter::random_bytes(&mut salt)
        .map_err(|e| format!("the OS wouldn't give random bytes for the salt: {e}"))?;
    hash_with(params, password.as_bytes(), &salt, arena)
}

/// The PHC string for `password` under `params` and `salt`, run in the
/// arena.  The error is the crate's reason, as text.
fn hash_with(params: &Params, password: &[u8], salt: &[u8], arena: &mut Arena) -> Result<String, String> {
    let algorithm = Algorithm::Argon2id;
    let version = Version::V0x13;
    let salt = Salt::new(salt).map_err(|e| e.to_string())?;
    let argon2 = Argon2::new(algorithm, version, params.clone());

    let mut hash = [0u8; HASH_BYTES];
    run_in_arena(&argon2, password, salt.as_ref(), &mut hash, arena)?;

    // The same five pieces the crate's own hasher writes out, in the same
    // order, so the line reads back with PasswordHash::new().
    let line = PasswordHash {
        algorithm: algorithm.ident(),
        version: Some(version.into()),
        params: ParamsString::try_from(params).map_err(|e| e.to_string())?,
        salt: Some(salt),
        hash: Some(Output::new(&hash).map_err(|e| e.to_string())?),
    };
    Ok(line.to_string())
}

/// Whether `password` matches `stored`, run in the arena.  The error only
/// means the stored line couldn't be read as one of our hashes; a wrong
/// password is `Ok(false)`.
fn check_hash(password: &str, stored: &str, arena: &mut Arena) -> Result<bool, String> {
    // Rust note: `PasswordHash::new` pulls the algorithm, the settings,
    // the salt and the hash out of the line.  The `?` hands its error to
    // whoever called us if the line is garbage.
    let parsed = PasswordHash::new(stored).map_err(|e| e.to_string())?;
    let algorithm = Algorithm::try_from(parsed.algorithm.as_str()).map_err(|e| e.to_string())?;
    let version = match parsed.version {
        Some(number) => Version::try_from(number).map_err(|e| e.to_string())?,
        None => Version::default(),
    };
    // This also takes the hash's length as the output length, so the
    // hash we make below is the same size as the one stored.
    let params = Params::try_from(&parsed).map_err(|e| e.to_string())?;

    // The crate is happy to parse a line with the salt or the hash missing
    // off the end, and then it calls every password wrong.  That's a
    // damaged row, not a wrong password, and somebody should hear about
    // it.
    let (Some(salt), Some(expected)) = (parsed.salt, parsed.hash) else {
        return Err("the salt or the hash is missing off the end".to_string());
    };

    let argon2 = Argon2::new(algorithm, version, params);
    let mut hash = [0u8; Output::MAX_LENGTH];
    let hash = &mut hash[..expected.len()];
    run_in_arena(&argon2, password.as_bytes(), salt.as_ref(), hash, arena)?;

    // Rust note: `==` on two Outputs takes the same time whether they
    // differ at the first byte or the last, so nobody can feel their way
    // to a hash a byte at a time with a stopwatch.  A plain `==` on two
    // slices stops at the first difference.
    let computed = Output::new(hash).map_err(|e| e.to_string())?;
    Ok(computed == expected)
}

/// Runs one hash in the arena.  A stored line made with more memory than
/// the arena holds (a setting we've since lowered, say) gets a one-off
/// allocation the slow way instead, rather than no answer.
fn run_in_arena(argon2: &Argon2<'_>, password: &[u8], salt: &[u8], hash: &mut [u8], arena: &mut Arena)
                -> Result<(), String> {
    let fits = argon2.params().block_count() <= arena.blocks.len();
    let result = if fits {
        argon2.hash_password_into_with_memory(password, salt, hash, arena.blocks.as_mut_slice())
    } else {
        argon2.hash_password_into(password, salt, hash)
    };
    result.map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// None of these go through the real worker or down a path that logs.  The
// real hash costs tens of milliseconds, so most of these run at the
// cheapest setting Argon2 allows, in an arena of the test's own, and one
// runs at the real one.

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest, fastest settings Argon2 accepts.  A hash at this cost
    /// is worthless as protection and takes well under a millisecond.
    fn cheap_params() -> Params {
        Params::new(Params::MIN_M_COST, 1, 1, Some(HASH_BYTES)).unwrap()
    }

    /// An arena just big enough for the cheap settings, on ordinary pages.
    fn cheap_arena() -> Arena {
        Arena::allot(cheap_params().block_count(), false)
    }

    /// A hash at the cheap settings, with a fixed salt.
    fn cheap_hash(password: &str, arena: &mut Arena) -> String {
        hash_with(&cheap_params(), password.as_bytes(), b"sixteen sevens 7", arena).unwrap()
    }

    #[test]
    fn the_right_password_matches_and_the_wrong_one_does_not() {
        let mut arena = cheap_arena();
        let stored = cheap_hash("Correct horse 1!", &mut arena);

        assert_eq!(check_hash("Correct horse 1!", &stored, &mut arena), Ok(true));
        assert_eq!(check_hash("Correct horse 1?", &stored, &mut arena), Ok(false));
        assert_eq!(check_hash("", &stored, &mut arena), Ok(false));
    }

    #[test]
    fn two_hashes_of_one_password_differ() {
        // A fresh salt every time, so a stolen table can't say which two
        // accounts share a password.  make_hash() is the one that asks the
        // OS for the salt.
        let mut arena = cheap_arena();
        let first = make_hash(&cheap_params(), "Same password 1!", &mut arena).unwrap();
        let second = make_hash(&cheap_params(), "Same password 1!", &mut arena).unwrap();

        assert_ne!(first, second);
        assert_eq!(check_hash("Same password 1!", &first, &mut arena), Ok(true));
        assert_eq!(check_hash("Same password 1!", &second, &mut arena), Ok(true));
    }

    #[test]
    fn the_stored_line_carries_the_real_settings() {
        // The one test at the real cost, in an arena like the worker's.
        // It checks what a new row will actually hold, and that the
        // settings in it are ours.
        let params = current_params();
        let mut arena = Arena::allot(params.block_count(), true);
        let stored = make_hash(&params, "Real cost 1!", &mut arena).unwrap();

        assert!(stored.starts_with("$argon2id$v=19$m=65536,t=1,p=1$"), "the stored line was {}", stored);
        assert_eq!(stored.matches('$').count(), 5);
        // One `=` in `v=19` and three in the settings.  The salt and the
        // hash are unpadded base64, so they never have one.
        assert_eq!(stored.matches('=').count(), 4);
        assert_eq!(check_hash("Real cost 1!", &stored, &mut arena), Ok(true));
        assert_eq!(check_hash("Real cost 2!", &stored, &mut arena), Ok(false));
    }

    #[test]
    fn a_hash_checks_under_its_own_settings_not_ours() {
        // What happens to an old account after we change the cost: the
        // stored line still says what it was hashed with, and that's what
        // gets used.  This one was made at the cheap setting, and
        // current_params() never enters into checking it.
        let mut arena = cheap_arena();
        let stored = cheap_hash("Old account 1!", &mut arena);
        assert!(stored.contains("m=8,t=1,p=1"), "the stored line was {}", stored);
        assert_eq!(check_hash("Old account 1!", &stored, &mut arena), Ok(true));
    }

    #[test]
    fn a_hash_bigger_than_the_arena_still_gets_an_answer() {
        // A line made with more memory than the arena holds falls back to
        // a one-off allocation.  Here the arena is the cheap 8 KiB and the
        // hash wants 64.
        let bigger = Params::new(64, 1, 1, Some(HASH_BYTES)).unwrap();
        let mut arena = cheap_arena();
        assert!(bigger.block_count() > arena.blocks.len());

        let stored = hash_with(&bigger, b"Too big 1!", b"sixteen sevens 7", &mut arena).unwrap();
        assert!(stored.contains("m=64,"), "the stored line was {}", stored);
        assert_eq!(check_hash("Too big 1!", &stored, &mut arena), Ok(true));
        assert_eq!(check_hash("Too big 2!", &stored, &mut arena), Ok(false));
    }

    #[test]
    fn the_crates_own_hasher_reads_our_line_and_we_read_its() {
        // Our line and the crate's line for the same password and salt are
        // the same text, so the two can never drift apart without this
        // test saying so.
        use argon2::PasswordHasher;

        let mut arena = cheap_arena();
        let ours = cheap_hash("Both ways 1!", &mut arena);
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, cheap_params());
        let theirs = argon2.hash_password_with_salt(b"Both ways 1!", b"sixteen sevens 7").unwrap().to_string();

        assert_eq!(ours, theirs);
    }

    #[test]
    fn a_damaged_stored_line_is_an_error_not_a_match() {
        let mut arena = cheap_arena();
        assert!(check_hash("Anything 1!", "", &mut arena).is_err());
        assert!(check_hash("Anything 1!", "not a hash at all", &mut arena).is_err());
        assert!(check_hash("Anything 1!", "$argon2id$v=19$m=8,t=1,p=1", &mut arena).is_err());
        assert!(check_hash("Anything 1!", "$argon2id$v=19$m=8,t=1,p=1$c2l4dGVlbiBzZXZlbnMgNw", &mut arena)
            .is_err());
    }

    #[test]
    fn the_password_rules() {
        assert_eq!(check_password_rules("Abcdef1!"), Ok(()));
        assert_eq!(check_password_rules("correct Horse battery staple 1#"), Ok(()));

        assert!(check_password_rules("Abcde1!").is_err());
        assert!(check_password_rules(&format!("Abcdef1!{}", "x".repeat(121))).is_err());
        assert_eq!(check_password_rules(&format!("Abcdef1!{}", "x".repeat(120))), Ok(()));

        assert!(check_password_rules("abcdefg1!").is_err());
        assert!(check_password_rules("Abcdefgh!").is_err());
        assert!(check_password_rules("Abcdefgh1").is_err());

        // A tab, an accented letter and a newline are all outside
        // printable ASCII.
        assert!(check_password_rules("Abcdef1!\t").is_err());
        assert!(check_password_rules("Abcd\u{e9}f1!").is_err());
        assert!(check_password_rules("Abcdef1!\n").is_err());
    }

    #[test]
    fn our_settings_are_ones_argon2_accepts() {
        // current_params() uses `expect`, so this is the test that keeps
        // that from ever firing on a real server.
        let params = current_params();
        assert_eq!(params.m_cost(), HASH_MEMORY_KIB);
        assert_eq!(params.t_cost(), HASH_PASSES);
        assert_eq!(params.p_cost(), HASH_LANES);
        // One block is one KiB, so the arena is exactly the memory cost.
        assert_eq!(params.block_count(), HASH_MEMORY_KIB as usize);
    }

    #[test]
    fn the_huge_page_hint_leaves_the_arena_working() {
        // The hint trims the span to whole pages, and getting that wrong
        // would corrupt what's next to the arena.  Whether the OS took the
        // hint is its business; that a hash still comes out right is ours.
        let mut plain = cheap_arena();
        let mut hinted = Arena::allot(cheap_params().block_count(), true);
        let stored = cheap_hash("Hinted 1!", &mut hinted);

        assert_eq!(stored, cheap_hash("Hinted 1!", &mut plain));
        assert_eq!(check_hash("Hinted 1!", &stored, &mut hinted), Ok(true));
        assert!(plain.describe().contains("ordinary pages"));
    }

    #[test]
    fn the_worker_answers_each_job_in_order_and_stops_when_the_line_closes() {
        // A worker of the test's own, on a line of the test's own, with an
        // arena of the test's own, so the real one never gets touched.
        // It does check in with the real services list, which is harmless.
        let mut arena = cheap_arena();
        let stored = cheap_hash("In line 1!", &mut arena);
        let (queue, jobs) = mpsc::channel();
        let worker = threads::spawn("security-test", move || serve(jobs, &mut arena)).unwrap();

        let (first_reply, first) = Pending::new();
        let (second_reply, second) = Pending::new();
        let (third_reply, third) = Pending::new();
        queue.send(Job::Verify { password: "In line 1!".to_string(), stored: stored.clone(), reply: first_reply })
            .unwrap();
        queue.send(Job::Verify { password: "In line 2!".to_string(), stored, reply: second_reply }).unwrap();
        queue.send(Job::NoAccount { password: "Nobody 1!".to_string(), reply: third_reply }).unwrap();

        assert_eq!(first.wait(), Ok(true));
        assert_eq!(second.wait(), Ok(false));
        assert_eq!(third.wait(), Ok(()));
        // Three more finished, and the average is a real number.
        assert!(DONE.load(Ordering::Relaxed) >= 3);
        assert!(average_hash() > Duration::ZERO);

        // Letting go of the line ends the thread.  If it didn't, join()
        // would never come back and the test would hang.
        drop(queue);
        worker.join().unwrap();
    }

    #[test]
    fn a_job_with_no_worker_hears_not_running() {
        // The real worker isn't started in the tests, so this goes through
        // queue() and finds nobody there.
        assert_eq!(hash_password("Nobody home 1!").wait(), Err(SecurityError::NotRunning));
        assert_eq!(verify_password("Nobody home 1!", "$argon2id$v=19$m=8,t=1,p=1$x$y").wait(),
                   Err(SecurityError::NotRunning));
        let ticket = verify_no_account("Nobody home 1!");
        assert_eq!(ticket.place(), Place { ahead: 0, wait: Duration::ZERO });
        assert_eq!(ticket.wait(), Err(SecurityError::NotRunning));
    }

    #[test]
    fn a_place_counts_the_jobs_ahead_and_the_time_they_take() {
        let hash = Duration::from_millis(30);

        // Job 5 with 2 done: jobs 3 and 4 are ahead, and 3 hashes to go.
        assert_eq!(place(5, 2, hash), Place { ahead: 2, wait: Duration::from_millis(90) });
        // Job 3 with 2 done: it's being hashed now.
        assert_eq!(place(3, 2, hash), Place { ahead: 0, wait: hash });
        // Done, and never queued, are both nothing to wait for.
        assert_eq!(place(2, 2, hash), Place { ahead: 0, wait: Duration::ZERO });
        assert_eq!(place(0, 2, hash), Place { ahead: 0, wait: Duration::ZERO });
    }

    #[test]
    fn a_login_never_answers_early() {
        // No work at all, which is the "no such name" path.
        let started = Instant::now();
        pad_login_time(started);
        assert!(started.elapsed() >= Duration::from_millis(MIN_LOGIN_MILLIS));

        // Work that already took longer than the floor doesn't wait again.
        let long_ago = Instant::now() - Duration::from_millis(MIN_LOGIN_MILLIS * 2);
        let started = Instant::now();
        pad_login_time(long_ago);
        assert!(started.elapsed() < Duration::from_millis(MIN_LOGIN_MILLIS / 2));
    }

    // -----------------------------------------------------------------------
    // The benchmark
    // -----------------------------------------------------------------------

    /// How many hashes we time at each setting.  The middle one is the
    /// number that counts.
    const BENCH_RUNS: usize = 5;

    /// The memory settings to try, in MiB.  19 is OWASP's floor for
    /// Argon2id and the crate's default; 64 is what RFC 9106 suggests for
    /// a machine short on memory; the rest is what raising it would cost.
    const BENCH_MEMORY_MIB: [u32; 5] = [19, 32, 64, 128, 256];

    /// One pass is ours.  Two is Stratum's, for the comparison.
    const BENCH_PASSES: [u32; 2] = [1, 2];

    /// Times Argon2id at a range of settings, three ways each: the crate
    /// allotting fresh memory for every hash, a kept arena, and a kept
    /// arena with the huge page hint.  Not part of a normal `cargo test`.
    /// Run it by hand, from `Conductor/dev`, and with `--release`: a test
    /// build is unoptimized, and unoptimized Argon2 ran six times slower
    /// on the dev machine, which drowns out what the columns measure.
    ///
    /// ```text
    /// cargo test -p conductor-tools --release argon2_cost -- --ignored --nocapture
    /// ```
    ///
    /// The "fresh" column is what a hash costs without the arena.  The gap
    /// to "arena" is the page faults; the gap from there to "huge" is the
    /// TLB.  A "huge" that matches "arena" means the kernel didn't go
    /// along with the hint; `/sys/kernel/mm/transparent_hugepage/enabled`
    /// says whether it ever would.
    #[test]
    #[ignore]
    fn argon2_cost_benchmark() {
        let biggest = BENCH_MEMORY_MIB[BENCH_MEMORY_MIB.len() - 1] as usize * 1024;
        let mut plain = Arena::allot(biggest, false);
        let mut hinted = Arena::allot(biggest, true);

        println!();
        println!("Argon2id, 1 lane, {} hashes per cell, the middle time shown.  The arena: {}",
                 BENCH_RUNS, hinted.describe());
        println!();
        println!("  memory  passes      fresh      arena       huge");

        for memory_mib in BENCH_MEMORY_MIB {
            for passes in BENCH_PASSES {
                let params = Params::new(memory_mib * 1024, passes, 1, Some(HASH_BYTES))
                    .expect("Argon2 turned down the benchmark's settings");
                let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

                let fresh = middle(time_hashes(&argon2, None));
                let arena = middle(time_hashes(&argon2, Some(&mut plain)));
                let huge = middle(time_hashes(&argon2, Some(&mut hinted)));
                println!("  {:>3} MiB  {:>4}   {:>6.1} ms  {:>6.1} ms  {:>6.1} ms", memory_mib, passes,
                         millis(fresh), millis(arena), millis(huge));
            }
        }
        println!();
    }

    /// Hashes a password BENCH_RUNS times and hands back how long each one
    /// took: in `arena` if given, or with fresh memory each time if not.
    /// One untimed hash goes first, so nothing is measured cold.
    fn time_hashes(argon2: &Argon2<'_>, mut arena: Option<&mut Arena>) -> Vec<Duration> {
        // The salt would be random for real.  It makes no difference to
        // the time.
        let salt = b"sixteen sevens 7";
        let mut hash = [0u8; HASH_BYTES];
        let mut one_hash = |password: &[u8]| {
            let result = match arena.as_deref_mut() {
                Some(arena) => argon2.hash_password_into_with_memory(password, salt, &mut hash,
                                                                     arena.blocks.as_mut_slice()),
                None => argon2.hash_password_into(password, salt, &mut hash),
            };
            result.expect("Argon2 couldn't hash the benchmark's password");
        };

        one_hash(b"warming up");
        let mut times = Vec::new();
        for _ in 0..BENCH_RUNS {
            let started = Instant::now();
            one_hash(b"correct horse battery staple");
            times.push(started.elapsed());
        }
        times
    }

    fn middle(mut times: Vec<Duration>) -> Duration {
        times.sort();
        times[times.len() / 2]
    }

    fn millis(duration: Duration) -> f64 {
        duration.as_secs_f64() * 1000.0
    }
}
