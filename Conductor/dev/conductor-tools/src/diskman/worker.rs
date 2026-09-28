//! File:       Opus/Conductor/dev/conductor-tools/src/diskman/worker.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! DiskMan's one worker thread, and the only code in Conductor that
//! touches the disk to write.  It goes round and round: answer the reads,
//! write out every file that has something waiting, move the big write on
//! by one chunk, hand each open stream its next chunk, unload what memory
//! can spare.  When there's nothing to do it sleeps until a caller wakes
//! it, or a second passes and it checks in with the services list.
//!
//! The lock is never held while the disk is being touched, or while
//! anything is logged.  That's what lets callers keep handing DiskMan
//! work while it writes, and it's what stops a deadlock with Scribe, who
//! hands DiskMan every log line.
//!
//! **Nothing routine is logged in here.**  Every log line is a write to
//! the log file, and a line saying that write happened would be another
//! write, forever.  Only failures, and the file coming good again, get a
//! line.  Problems with Scribe's own file go to the console only, since a
//! log line about the log file not working would never make it.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::TrySendError;
use std::sync::{Arc, MutexGuard};
use std::time::{Duration, Instant};

use super::cache::{Flush, Outcome, Reply, State};
use super::{Contents, DiskError, Piece, Shared};
use crate::scribe::{self, Channel};
use crate::services::{self, State as ServiceState};
use crate::threads;

/// How long the worker sleeps when there's nothing to do.  It wakes early
/// the moment anything comes in.
const IDLE_WAIT: Duration = Duration::from_secs(1);

/// How long it sleeps while a stream's reader hasn't taken its last chunk
/// yet.  Short, so a reader that's keeping up isn't held back.
const STREAM_WAIT: Duration = Duration::from_millis(5);

/// Starts the worker on its own thread.  False if the thread wouldn't
/// start, and then nothing gets written.
pub(super) fn start(shared: &'static Shared) -> bool {
    {
        let mut state = shared.lock();
        if state.running {
            return true;
        }
        state.running = true;
        state.stopping = false;
    }
    match threads::spawn("diskman", move || run(shared)) {
        Ok(handle) => {
            let mut worker = shared.worker.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *worker = Some(handle);
            true
        }
        Err(e) => {
            shared.lock().running = false;
            // Scribe comes after DiskMan and has nowhere to write without
            // it, so this one goes to the console.
            eprintln!("DiskMan couldn't start its thread: {e}.  Nothing will be written to disk.");
            false
        }
    }
}

/// A big whole-file write, going out a chunk at a time into its temp file.
struct BigWrite {
    path: PathBuf,
    temp: PathBuf,
    file: File,
    content: Contents,
    version: u64,
    waiters: Vec<Reply>,
    done: usize,
    started: Instant,
}

/// A stream the worker has open.
struct OpenStream {
    reply: std::sync::mpsc::SyncSender<Piece>,
    source: Source,
    /// A piece the reader hasn't made room for yet.
    held: Option<Piece>,
    /// `Done` or `Failed` has been handed over, so it can be closed.
    finished: bool,
}

/// Where a stream's chunks come from.
enum Source {
    /// A file we hold whole, and how far along it we are.
    Memory(Contents, usize),
    /// The file on disk, then the tail that hasn't gone out yet, which was
    /// copied when the stream opened.  Only as many bytes as the file had
    /// when it opened are read from it, since that tail may be appended
    /// to it while the stream is going.
    Disk { file: File, left: u64, tail: Option<Vec<u8>> },
}

/// Sets `running` back to false when the worker ends, however it ends --
/// a panic included -- and throws away anyone still waiting, so their
/// `Pending` hears "not running" instead of waiting forever.
struct Closing(&'static Shared);

impl Drop for Closing {
    fn drop(&mut self) {
        let mut state = self.0.lock();
        state.running = false;
        state.reads.clear();
        state.new_streams.clear();
        state.streams_open = 0;
        state.big_write = None;
        for entry in state.files.values_mut() {
            entry.write_waiters.clear();
            entry.tail_waiters.clear();
        }
    }
}

fn run(shared: &'static Shared) {
    let _closing = Closing(shared);
    services::set(services::DISKMAN, ServiceState::Running, "Nothing waiting to be written.");

    let mut big: Option<BigWrite> = None;
    let mut streams: Vec<OpenStream> = Vec::new();
    let mut was_failing = false;

    loop {
        services::seen(services::DISKMAN);

        let mut busy = do_reads(shared);
        busy |= flush_files(shared, &mut big);
        busy |= step_big_write(shared, &mut big);
        open_streams(shared, &mut streams);
        if shared.lock().stopping {
            // Shutting down closes the streams rather than finishing them,
            // or a reader working through a huge file would hold it up.
            // Their readers hear "not running".
            streams.clear();
        }
        busy |= step_streams(shared, &mut streams);

        let mut state = shared.lock();
        state.streams_open = streams.len();
        state.unload_extra();
        let failing = state.status().files_failing;
        if (failing > 0) != was_failing {
            was_failing = failing > 0;
            let note = if was_failing {
                format!("{failing} file(s) failing to write.  See the log.")
            } else {
                "Nothing failing.".to_string()
            };
            let service_state = if was_failing { ServiceState::Trouble } else { ServiceState::Running };
            drop(state);
            services::set(services::DISKMAN, service_state, &note);
            state = shared.lock();
        }

        if busy {
            continue;
        }
        if state.stopping && state.all_done() && big.is_none() {
            // Turned away from here on, under the same lock that saw
            // nothing left, so no log line can slip in behind the check
            // and be lost.
            state.running = false;
            break;
        }
        let wait = if streams.is_empty() { IDLE_WAIT } else { STREAM_WAIT };
        // Rust note: `wait_timeout` lets go of the lock while it sleeps and
        // takes it back when it wakes, which is how a caller can hand in
        // new work in the meantime.
        let _ = shared.wake.wait_timeout(state, wait);
    }

    drop(streams);
    services::set(services::DISKMAN, ServiceState::Stopped, "Everything it was holding is written.");
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

/// Answers every read waiting on the disk.  True if there were any.
fn do_reads(shared: &Shared) -> bool {
    let jobs: Vec<_> = shared.lock().reads.drain(..).collect();
    let any = !jobs.is_empty();
    for job in jobs {
        let from_disk = fs::read(&job.path);
        let answer = shared.lock().after_disk_read(&job.path, from_disk);
        let _ = job.reply.send(answer);
    }
    any
}

// ---------------------------------------------------------------------------
// Writes
// ---------------------------------------------------------------------------

/// Writes out every file with something ready.  A big whole file starts
/// the big write if none is under way, and otherwise waits its turn.  True
/// if anything was written.
fn flush_files(shared: &Shared, big: &mut Option<BigWrite>) -> bool {
    let (ready, big_above) = {
        let state = shared.lock();
        (state.ready_to_flush(Instant::now()), state.limits.big_above)
    };
    let mut any = false;

    for path in ready {
        let Some(flush) = shared.lock().take_flush(&path, big.is_none()) else {
            continue;
        };
        any = true;

        match flush {
            Flush::Whole { content, version, waiters } if content.len() > big_above => {
                match start_big_write(&path, Arc::clone(&content), version, waiters) {
                    Ok(started) => {
                        shared.lock().big_write = Some((path.clone(), 0, content.len() as u64));
                        *big = Some(started);
                    }
                    Err((e, waiters)) => {
                        let flush = Flush::Whole { content, version, waiters };
                        let outcome = shared.lock().finish(&path, flush, Err(DiskError::from_io(&e)));
                        report(&path, &outcome, Some(&e));
                    }
                }
            }
            flush => {
                let started = Instant::now();
                let result = match &flush {
                    Flush::Whole { content, .. } => write_whole(&path, content),
                    Flush::Tail { bytes, .. } => append_bytes(&path, bytes),
                };
                finish_write(shared, &path, flush, result, started);
            }
        }
    }
    any
}

/// Counts a finished write, tells the waiters, and reports how it went.
fn finish_write(shared: &Shared, path: &Path, flush: Flush, result: io::Result<()>, started: Instant) {
    let took = started.elapsed();
    let (bytes, whole) = match &flush {
        Flush::Whole { content, .. } => (content.len() as u64, true),
        Flush::Tail { bytes, .. } => (bytes.len() as u64, false),
    };
    let outcome = {
        let mut state = shared.lock();
        if result.is_ok() {
            state.totals.bytes_written += bytes;
            if whole {
                state.totals.writes_done += 1;
            } else {
                state.totals.appends_done += 1;
            }
        }
        state.finish(path, flush, result.as_ref().map(|_| took).map_err(DiskError::from_io))
    };
    report(path, &outcome, result.as_ref().err());
}

/// Says what went wrong, or that a file is fine again.  Scribe's own file
/// only goes to the console and the services list.  Called with the lock
/// let go.
fn report(path: &Path, outcome: &Outcome, error: Option<&io::Error>) {
    let why = error.map_or_else(String::new, |e| e.to_string());
    match *outcome {
        Outcome::Written { recovered: true, for_scribe: true } => {
            eprintln!("DiskMan can write the log to {} again.", path.display());
            services::set(services::SCRIBE, ServiceState::Running, &format!("Writing to {}", path.display()));
        }
        Outcome::Written { recovered: true, for_scribe: false } => {
            scribe::info(Channel::System, &format!("DiskMan can write {} again.", path.display()));
        }
        Outcome::Written { .. } => {}
        Outcome::Failed { first: true, for_scribe: true } => {
            eprintln!("DiskMan can't write the log to {}: {why}.  It'll keep trying, and the lines still \
                reach the console.", path.display());
            services::set(services::SCRIBE, ServiceState::Trouble, &format!("Can't write to {}: {why}.  \
                Lines still reach the console.", path.display()));
        }
        Outcome::Failed { first: true, for_scribe: false } => {
            scribe::error(Channel::System, &format!("DiskMan can't write {}: {why}.  It'll try again in \
                a few seconds.", path.display()));
        }
        Outcome::Failed { .. } => {}
        Outcome::GaveUp { for_scribe: true } => {
            eprintln!("DiskMan gave up on the log lines for {}: {why}.  They only reached the console.",
                      path.display());
        }
        Outcome::GaveUp { for_scribe: false } => {
            scribe::error(Channel::System, &format!("DISKMAN GAVE UP ON {} AFTER {} TRIES: {why}.  What it \
                was holding for it is lost, and the file on disk is whatever was there before.",
                path.display(), super::cache::MOST_TRIES));
        }
    }
}

// ---------------------------------------------------------------------------
// The big write
// ---------------------------------------------------------------------------

// Rust note: the error side carries the waiters back out, since the
// function took them and the caller still has to answer them.
fn start_big_write(path: &Path, content: Contents, version: u64, waiters: Vec<Reply>)
                   -> Result<BigWrite, (io::Error, Vec<Reply>)> {
    let temp = match temp_path(path) {
        Ok(temp) => temp,
        Err(e) => return Err((e, waiters)),
    };
    if let Err(e) = make_folder(path) {
        return Err((e, waiters));
    }
    match File::create(&temp) {
        Ok(file) => Ok(BigWrite {
            path: path.to_path_buf(),
            temp,
            file,
            content,
            version,
            waiters,
            done: 0,
            started: Instant::now(),
        }),
        Err(e) => Err((e, waiters)),
    }
}

/// Writes one more chunk of the big write, or finishes it.  If a newer
/// write has replaced the file in the meantime, this one is dropped and
/// the newer one goes out instead.  True while there's a big write.
fn step_big_write(shared: &Shared, big: &mut Option<BigWrite>) -> bool {
    let Some(job) = big.as_mut() else {
        return false;
    };

    let (replaced, chunk_bytes) = {
        let state = shared.lock();
        (state.replaced(&job.path, job.version), state.limits.chunk_bytes)
    };
    if replaced {
        let Some(job) = big.take() else {
            return false;
        };
        drop(job.file);
        let _ = fs::remove_file(&job.temp);
        let mut state = shared.lock();
        state.abandon(&job.path, job.waiters);
        state.big_write = None;
        return true;
    }

    let end = (job.done + chunk_bytes).min(job.content.len());
    let mut result = job.file.write_all(&job.content[job.done..end]);
    if result.is_ok() {
        job.done = end;
        if job.done < job.content.len() {
            shared.lock().big_write = Some((job.path.clone(), job.done as u64, job.content.len() as u64));
            return true;
        }
    }

    // All of it is in the temp file, or a chunk failed.  Either way this
    // big write is over.
    let Some(job) = big.take() else {
        return false;
    };
    if result.is_ok() {
        result = job.file.sync_all();
    }
    // The file has to be closed before the rename, or Windows says no.
    drop(job.file);
    if result.is_ok() {
        result = fs::rename(&job.temp, &job.path);
    }
    if result.is_ok() {
        sync_folder(&job.path);
    } else {
        let _ = fs::remove_file(&job.temp);
    }
    shared.lock().big_write = None;
    let flush = Flush::Whole { content: job.content, version: job.version, waiters: job.waiters };
    finish_write(shared, &job.path, flush, result, job.started);
    true
}

// ---------------------------------------------------------------------------
// Streams
// ---------------------------------------------------------------------------

/// Opens the streams callers have asked for since the last round.
fn open_streams(shared: &Shared, streams: &mut Vec<OpenStream>) {
    let jobs: Vec<_> = shared.lock().new_streams.drain(..).collect();
    for job in jobs {
        // What we hold in memory is newer than the disk, so it's used when
        // we have the whole file.  Otherwise it's the disk, plus the tail
        // that hasn't gone out.
        let (held, tail) = {
            let state = shared.lock();
            let entry = state.files.get(&job.path);
            (entry.and_then(|entry| entry.content.clone()), entry.map(|entry| entry.tail.clone()))
        };
        let tail = tail.filter(|tail| !tail.is_empty());
        let source = match (held, File::open(&job.path)) {
            (Some(content), _) => Ok(Source::Memory(content, 0)),
            (None, Ok(file)) => match file.metadata() {
                Ok(about) => Ok(Source::Disk { file, left: about.len(), tail }),
                Err(e) => Err(DiskError::from_io(&e)),
            },
            (None, Err(e)) if e.kind() == io::ErrorKind::NotFound && tail.is_some() => {
                Ok(Source::Memory(Arc::new(tail.unwrap_or_default()), 0))
            }
            (None, Err(e)) => Err(DiskError::from_io(&e)),
        };
        match source {
            Ok(source) => streams.push(OpenStream { reply: job.reply, source, held: None, finished: false }),
            Err(e) => {
                // A reader too slow to take even this gets nothing.
                let _ = job.reply.try_send(Piece::Failed(e));
            }
        }
    }
}

/// Hands each open stream its next chunk, if its reader has room.  True if
/// any chunk was handed over.
fn step_streams(shared: &Shared, streams: &mut Vec<OpenStream>) -> bool {
    let chunk_bytes = shared.lock().limits.chunk_bytes;
    let mut any = false;
    let mut read_bytes = 0u64;

    for stream in streams.iter_mut() {
        let piece = match stream.held.take() {
            Some(piece) => piece,
            None => next_piece(&mut stream.source, chunk_bytes),
        };
        let ends = !matches!(piece, Piece::Bytes(_));
        let size = match &piece {
            Piece::Bytes(bytes) => bytes.len() as u64,
            _ => 0,
        };
        match stream.reply.try_send(piece) {
            Ok(()) => {
                any = true;
                read_bytes += size;
                stream.finished = ends;
            }
            Err(TrySendError::Full(piece)) => stream.held = Some(piece),
            // The reader dropped its end, so nobody wants the rest.
            Err(TrySendError::Disconnected(_)) => stream.finished = true,
        }
    }

    streams.retain(|stream| !stream.finished);
    if read_bytes > 0 {
        shared.lock().totals.bytes_read += read_bytes;
    }
    any
}

fn next_piece(source: &mut Source, chunk_bytes: usize) -> Piece {
    match source {
        Source::Memory(content, at) => {
            if *at >= content.len() {
                return Piece::Done;
            }
            let end = (*at + chunk_bytes).min(content.len());
            let piece = content[*at..end].to_vec();
            *at = end;
            Piece::Bytes(piece)
        }
        Source::Disk { file, left, tail } => {
            if *left == 0 {
                return match tail.take() {
                    Some(tail) => Piece::Bytes(tail),
                    None => Piece::Done,
                };
            }
            // Rust note: `as usize` on a number that doesn't fit would cut
            // it short, which is why the smaller one is picked as a u64
            // first.
            let size = (*left).min(chunk_bytes as u64) as usize;
            let mut buffer = vec![0u8; size];
            match file.read(&mut buffer) {
                // Shorter than it was when it opened.  Somebody cut it down
                // outside Conductor, so what's there is all there is.
                Ok(0) => {
                    *left = 0;
                    match tail.take() {
                        Some(tail) => Piece::Bytes(tail),
                        None => Piece::Done,
                    }
                }
                Ok(count) => {
                    buffer.truncate(count);
                    *left -= count as u64;
                    Piece::Bytes(buffer)
                }
                Err(e) => Piece::Failed(DiskError::from_io(&e)),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The disk itself
// ---------------------------------------------------------------------------

/// Replaces a whole file without ever leaving half of one.  The bytes go
/// into a temp file next to it, are flushed all the way to the disk, and
/// only then is the temp file renamed over the real one.  A rename within
/// one folder happens all at once, so if the power goes out, the file is
/// either the old one or the new one.
fn write_whole(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = temp_path(path)?;
    make_folder(path)?;
    let written = File::create(&temp)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            file.sync_all()
        })
        .and_then(|()| fs::rename(&temp, path));
    if written.is_err() {
        let _ = fs::remove_file(&temp);
    }
    written.map(|()| sync_folder(path))
}

/// Adds bytes to the end of a file, making it if it isn't there.  Not all
/// at once like `write_whole()`: a crash part way can leave part of them.
/// For a log, that's a cut-off last line, which I can live with.
fn append_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    make_folder(path)?;
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_data()
}

/// `name.cfg` -> `name.cfg.diskman-tmp`, in the same folder, since a
/// rename is only all-at-once within one drive.  One left behind by a
/// crash is written over the next time.
fn temp_path(path: &Path) -> io::Result<PathBuf> {
    let Some(name) = path.file_name() else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "that path has no file name"));
    };
    let mut temp = name.to_os_string();
    temp.push(".diskman-tmp");
    Ok(path.with_file_name(temp))
}

fn make_folder(path: &Path) -> io::Result<()> {
    match path.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => fs::create_dir_all(folder),
        _ => Ok(()),
    }
}

/// Linux (and macOS) keep the list of what's in a folder in the folder
/// itself, so after a rename the folder gets flushed too, or a crash could
/// lose the rename.  If it fails, the file is written and the rename will
/// most likely stick anyway, so it's let go.
#[cfg(unix)]
fn sync_folder(path: &Path) {
    if let Some(folder) = path.parent() {
        let _ = File::open(folder).and_then(|folder| folder.sync_all());
    }
}

/// Windows won't open a folder like a file, and flushing the file before
/// the rename is as far as it lets us go.
#[cfg(not(unix))]
fn sync_folder(_path: &Path) {}

impl Shared {
    /// The lock, taken back if a thread panicked while holding it.  What
    /// it guards is still whole: nothing in here leaves it half changed.
    pub(super) fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
