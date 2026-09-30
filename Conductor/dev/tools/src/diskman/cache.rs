//! File:       Opus/Conductor/dev/tools/src/diskman/cache.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! What DiskMan holds in memory, and the rules for changing it.  Every file
//! DiskMan knows about is an `Entry` here, keyed by its path.  No disk work
//! happens in this file.  The worker does that, and comes back here to say
//! how it went.
//!
//! An entry is two things:
//!
//! - **The whole file**, when we know it: after a read, or after `write()`
//!   handed us all of it.  `dirty` means it's newer than what's on disk.
//! - **A tail**: bytes `append()` wants added to the end of the file on
//!   disk, for a file we don't hold whole (Scribe's log, say).
//!
//! The idea is that the worker writes as fast as the disk lets it, and
//! anything that comes in while it's busy lands here instead.  A second
//! write to a file that hasn't gone out yet just replaces the first in
//! memory, so the disk only ever sees the newest.  Once an entry is on
//! disk and clean, it can be unloaded whenever memory gets tight.  A dirty
//! one is never unloaded, however big it is, because it's the only copy.
//!
//! A clean copy is only trusted while the disk agrees with it.  Each entry
//! keeps the file's modified time and size as they were when the copy last
//! matched the disk, and a read of a clean copy goes to the worker first
//! to ask the disk for those two again.  If either has changed, somebody
//! edited the file outside Conductor, and the copy is dropped for the
//! file as it is now.

use std::collections::{BTreeMap, VecDeque};
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Sender, SyncSender};
use std::time::{Duration, Instant, SystemTime};

use super::{Contents, DiskError, Piece, Status, SwapAt};
use crate::clock::Utc;

/// Someone waiting to hear that their write or append is on disk.
pub(super) type Reply = Sender<Result<(), DiskError>>;

/// How long a file that failed to write waits before the next try.
pub(super) const RETRY_AFTER: Duration = Duration::from_secs(5);

/// How many tries a file gets before DiskMan gives up on what it's holding
/// for it.  Three tries over about 10 seconds.  A disk that's still saying
/// no after that isn't going to change its mind on its own, and holding on
/// forever would hang shutdown behind it.
pub(super) const MOST_TRIES: u32 = 3;

/// The sizes DiskMan works to.  The real ones are in `diskman.rs`; the
/// tests use tiny ones so they don't need gigabytes to see a chunk.
#[derive(Debug, Clone, Copy)]
pub(super) struct Limits {
    /// How much of a big file goes to disk (or comes back in a stream) at
    /// a time.
    pub(super) chunk_bytes: usize,
    /// A whole-file write bigger than this is written a chunk at a time,
    /// with other files getting their turn in between.
    pub(super) big_above: usize,
    /// How much clean data (already on disk) can stay loaded.  Past this,
    /// the one used longest ago is unloaded first.  Dirty data doesn't
    /// count and is never unloaded.
    pub(super) clean_cache_bytes: usize,
}

/// A file's modified time and size, as the disk has them.  Two of these
/// that differ mean the file changed in between.  The size is there as
/// well as the time because some file systems only keep the time to the
/// second, or coarser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Stamp {
    pub(super) modified: SystemTime,
    pub(super) len: u64,
}

/// What a read can have from memory, straight away.
pub(super) enum Held {
    /// A copy with something in it the disk hasn't got yet: a write or
    /// appends still on their way.  Newer than the disk, whatever the disk
    /// says, so it's the answer.
    Newest(Contents),
    /// A clean copy.  The worker asks the disk whether the file has changed
    /// since, and answers with the copy or the file as it is now.
    Check,
    /// Nothing held.  The worker reads the disk.
    Nothing,
}

/// One file DiskMan knows about.
pub(super) struct Entry {
    /// The whole file, as far as we know.  `None` when all we've been
    /// handed is appends.
    pub(super) content: Option<Contents>,
    /// `content` is newer than the disk and has to go out.
    pub(super) dirty: bool,
    /// Counts up on every `write()`, so the worker can tell whether what
    /// it's in the middle of writing has already been replaced.
    pub(super) version: u64,
    /// Bytes to add to the end of the file on disk.  Only used while
    /// `content` is `None` or clean; a dirty `content` already has them.
    pub(super) tail: Vec<u8>,
    /// Waiting on `content` reaching the disk.
    pub(super) write_waiters: Vec<Reply>,
    /// Waiting on `tail` reaching the disk.
    pub(super) tail_waiters: Vec<Reply>,
    /// The worker is writing this file right now, with the lock let go.
    pub(super) in_flight: bool,
    /// When this entry was last touched, as a count, for unloading the one
    /// used longest ago.
    pub(super) last_used: u64,
    /// Scribe's log.  Problems with it go to the console only, since
    /// logging them would be another line for the same broken file.
    pub(super) for_scribe: bool,
    /// Tries that have failed in a row.  Back to 0 on a good write.
    pub(super) failed_tries: u32,
    /// Not before this, after a failure.
    pub(super) retry_at: Option<Instant>,
    /// The file's time and size on disk as of when `content` last matched
    /// it.  `None` when we can't say, and then a clean copy is read again
    /// rather than trusted.
    pub(super) on_disk: Option<Stamp>,
}

impl Entry {
    fn new() -> Entry {
        Entry {
            content: None,
            dirty: false,
            version: 0,
            tail: Vec::new(),
            write_waiters: Vec::new(),
            tail_waiters: Vec::new(),
            in_flight: false,
            last_used: 0,
            for_scribe: false,
            failed_tries: 0,
            retry_at: None,
            on_disk: None,
        }
    }

    /// Nothing to write, nobody waiting, and nothing held: it can go.
    fn empty(&self) -> bool {
        self.content.is_none() && !self.dirty && self.tail.is_empty() && !self.in_flight
            && self.write_waiters.is_empty() && self.tail_waiters.is_empty()
    }

    /// Held only as a copy of what's on disk, so it can be unloaded.
    fn clean_copy(&self) -> bool {
        self.content.is_some() && !self.dirty && self.tail.is_empty() && !self.in_flight
    }

    /// Something in here still has to reach the disk.
    pub(super) fn waiting(&self) -> bool {
        self.dirty || !self.tail.is_empty() || self.in_flight
    }

    fn ready_to_try(&self, now: Instant) -> bool {
        !self.in_flight && self.retry_at.is_none_or(|at| now >= at)
    }
}

/// A read the worker has to go to the disk for.
pub(super) struct ReadJob {
    pub(super) path: PathBuf,
    pub(super) reply: Sender<Result<Contents, DiskError>>,
}

/// A stream the worker hasn't opened yet.
pub(super) struct StreamJob {
    pub(super) path: PathBuf,
    pub(super) reply: SyncSender<Piece>,
}

/// A file to rename over another, and when.
pub(super) struct Swap {
    pub(super) original: PathBuf,
    pub(super) replacement: PathBuf,
    pub(super) when: SwapAt,
    /// Whoever asked, if they're still listening.
    pub(super) reply: Option<Reply>,
}

/// A file to delete.
pub(super) struct RemoveJob {
    pub(super) path: PathBuf,
    pub(super) reply: Reply,
}

/// What the worker should do next for one file.
pub(super) enum Flush {
    /// Write the whole file.  `version` is the one being written.
    Whole { content: Contents, version: u64, waiters: Vec<Reply> },
    /// Add these bytes to the end of the file.
    Tail { bytes: Vec<u8>, waiters: Vec<Reply> },
}

/// The running totals for the web admin.
pub(super) struct Totals {
    pub(super) writes_done: u64,
    pub(super) appends_done: u64,
    pub(super) reads_done: u64,
    pub(super) cache_hits: u64,
    pub(super) bytes_written: u64,
    pub(super) bytes_read: u64,
    pub(super) failures: u64,
    pub(super) given_up: u64,
    pub(super) last_failure: Option<(Utc, String)>,
    pub(super) slowest_write: Duration,
}

/// Everything behind DiskMan's lock.
pub(super) struct State {
    /// Taking jobs.  True from `start()` until the worker has finished.
    pub(super) running: bool,
    /// `stop()` has been called: finish what's here, then end.
    pub(super) stopping: bool,
    pub(super) files: BTreeMap<PathBuf, Entry>,
    pub(super) reads: VecDeque<ReadJob>,
    /// Reads of a clean copy, waiting on the worker to ask the disk whether
    /// the file has changed since.
    pub(super) checks: VecDeque<ReadJob>,
    pub(super) new_streams: Vec<StreamJob>,
    /// Every swap asked for and not yet done, whatever its `when`.
    pub(super) swaps: Vec<Swap>,
    pub(super) removes: VecDeque<RemoveJob>,
    /// Waiting to hear that every swap due now has happened.
    pub(super) swap_fences: Vec<Reply>,
    /// For the page: streams the worker has open.
    pub(super) streams_open: usize,
    /// For the page: the big write under way, how far along, out of how
    /// much.
    pub(super) big_write: Option<(PathBuf, u64, u64)>,
    /// Counts up on every touch, for `Entry::last_used`.
    uses: u64,
    pub(super) totals: Totals,
    pub(super) limits: Limits,
}

impl State {
    pub(super) const fn new(limits: Limits) -> State {
        State {
            running: false,
            stopping: false,
            files: BTreeMap::new(),
            reads: VecDeque::new(),
            checks: VecDeque::new(),
            new_streams: Vec::new(),
            swaps: Vec::new(),
            removes: VecDeque::new(),
            swap_fences: Vec::new(),
            streams_open: 0,
            big_write: None,
            uses: 0,
            totals: Totals {
                writes_done: 0,
                appends_done: 0,
                reads_done: 0,
                cache_hits: 0,
                bytes_written: 0,
                bytes_read: 0,
                failures: 0,
                given_up: 0,
                last_failure: None,
                slowest_write: Duration::ZERO,
            },
            limits,
        }
    }

    /// The entry for `path`, made if there isn't one, marked as just used.
    fn entry(&mut self, path: &Path) -> &mut Entry {
        self.uses += 1;
        let uses = self.uses;
        let entry = self.files.entry(path.to_path_buf()).or_insert_with(Entry::new);
        entry.last_used = uses;
        entry
    }

    // -----------------------------------------------------------------------
    // What callers ask for
    // -----------------------------------------------------------------------

    /// The whole file is now `bytes`.  Anything that hadn't gone out yet
    /// for it, a write or appends, is replaced, and whoever was waiting on
    /// those hears back when this goes out instead.
    pub(super) fn write(&mut self, path: &Path, bytes: Vec<u8>, reply: Reply) {
        let entry = self.entry(path);
        entry.content = Some(Arc::new(bytes));
        entry.dirty = true;
        entry.version += 1;
        entry.tail.clear();
        let from_tail = mem::take(&mut entry.tail_waiters);
        entry.write_waiters.extend(from_tail);
        entry.write_waiters.push(reply);
        // New data, so it gets a fresh try straight away.
        entry.failed_tries = 0;
        entry.retry_at = None;
    }

    /// `bytes` go on the end of the file.
    pub(super) fn append(&mut self, path: &Path, bytes: &[u8], reply: Reply, for_scribe: bool) {
        let entry = self.entry(path);
        entry.for_scribe |= for_scribe;
        match entry.content.as_mut() {
            Some(content) => {
                // We hold the whole file, so the copy in memory gets the
                // bytes too.
                // Rust note: `Arc::make_mut` changes the shared data in
                // place if nobody else holds it, or makes this entry its
                // own copy first if a reader or the worker still does.
                Arc::make_mut(content).extend_from_slice(bytes);
                if entry.dirty || entry.in_flight {
                    // A whole write is waiting or under way.  The next one
                    // carries these bytes, and an append on top of it could
                    // land before it, so the file has to go out whole again.
                    entry.dirty = true;
                    entry.write_waiters.push(reply);
                } else {
                    entry.tail.extend_from_slice(bytes);
                    entry.tail_waiters.push(reply);
                }
            }
            None => {
                entry.tail.extend_from_slice(bytes);
                entry.tail_waiters.push(reply);
            }
        }
    }

    /// What a read of `path` can have straight away, if anything.
    pub(super) fn held(&mut self, path: &Path) -> Held {
        let Some(entry) = self.files.get(path) else {
            return Held::Nothing;
        };
        let waiting = entry.waiting();
        match entry.content.clone() {
            Some(content) if waiting => {
                self.totals.cache_hits += 1;
                self.entry(path);
                Held::Newest(content)
            }
            Some(_) => Held::Check,
            None => Held::Nothing,
        }
    }

    /// The copy in memory, if it's still the newest there is: it has
    /// something the disk hasn't got yet, or the disk says the same time
    /// and size (`stamp`) as when the copy last matched it.
    pub(super) fn current(&self, path: &Path, stamp: Option<Stamp>) -> Option<Contents> {
        let entry = self.files.get(path)?;
        let content = entry.content.as_ref()?;
        let unchanged = stamp.is_some() && entry.on_disk == stamp;
        (entry.waiting() || unchanged).then(|| Arc::clone(content))
    }

    /// The worker asked the disk about a file we hold a clean copy of, and
    /// it said `stamp`.  The copy if it's still right; otherwise `None`, and
    /// the copy is dropped so the worker reads the file as it is now.
    pub(super) fn after_check(&mut self, path: &Path, stamp: Option<Stamp>) -> Option<Contents> {
        if let Some(content) = self.current(path, stamp) {
            self.totals.cache_hits += 1;
            self.entry(path);
            return Some(content);
        }
        // Edited outside Conductor, gone, or the disk won't say.  Either
        // way the copy can't be trusted.
        if let Some(entry) = self.files.get_mut(path) {
            entry.content = None;
            entry.on_disk = None;
        }
        None
    }

    /// The worker went to the disk for a read.  What the caller gets is the
    /// file on disk plus any tail that hasn't gone out yet, and that's kept
    /// as the entry's content if it isn't too big to hold.  `stamp` is the
    /// file's time and size, asked for just before the bytes.
    pub(super) fn after_disk_read(&mut self, path: &Path, from_disk: std::io::Result<Vec<u8>>,
                                  stamp: Option<Stamp>) -> Result<Contents, DiskError> {
        // A write may have come in while the worker was reading.  It's
        // newer than the disk.
        if let Some(content) = self.files.get(path).and_then(|entry| entry.content.clone()) {
            return Ok(content);
        }
        let tail = self.files.get(path).map(|entry| entry.tail.clone()).unwrap_or_default();
        let matches_disk = tail.is_empty();

        let mut bytes = match from_disk {
            Ok(bytes) => bytes,
            // Not on disk yet, but appends for it are on their way.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !tail.is_empty() => Vec::new(),
            Err(e) => return Err(DiskError::from_io(&e)),
        };
        self.totals.reads_done += 1;
        self.totals.bytes_read += bytes.len() as u64;
        bytes.extend_from_slice(&tail);
        let content: Contents = Arc::new(bytes);

        if content.len() <= self.limits.clean_cache_bytes {
            let entry = self.entry(path);
            entry.content = Some(Arc::clone(&content));
            entry.dirty = false;
            // With a tail on the end, the copy isn't what the disk has yet.
            // It's stamped again once the tail is written.
            entry.on_disk = if matches_disk { stamp } else { None };
        }
        Ok(content)
    }

    /// Puts a swap in the list.  One already there for the same original
    /// is replaced, and whoever asked for it hears that it's done, since
    /// the newer one carries what they wanted.
    pub(super) fn add_swap(&mut self, swap: Swap) {
        if let Some(at) = self.swaps.iter().position(|held| held.original == swap.original) {
            let old = self.swaps.remove(at);
            if let Some(reply) = old.reply {
                let _ = reply.send(Ok(()));
            }
        }
        self.swaps.push(swap);
    }

    /// Every swap held for `when` is due now.
    pub(super) fn release_swaps(&mut self, when: SwapAt) {
        for swap in self.swaps.iter_mut().filter(|swap| swap.when == when) {
            swap.when = SwapAt::Now;
        }
    }

    /// Takes the swap for `original` out of the list.  Whoever asked hears
    /// that it's done, in the sense that nothing more will happen to it.
    pub(super) fn forget_swap(&mut self, original: &Path) {
        if let Some(at) = self.swaps.iter().position(|held| held.original == original) {
            if let Some(reply) = self.swaps.remove(at).reply {
                let _ = reply.send(Ok(()));
            }
        }
    }

    /// True while a swap is due now and not yet done.
    pub(super) fn swaps_due(&self) -> bool {
        self.swaps.iter().any(|swap| swap.when == SwapAt::Now)
    }

    /// Takes every swap due now whose two files have nothing waiting to
    /// go out, so the rename never races a write.  One with a write still
    /// waiting stays for the next round.
    pub(super) fn take_due_swaps(&mut self) -> Vec<Swap> {
        let quiet = |files: &BTreeMap<PathBuf, Entry>, path: &Path| {
            files.get(path).is_none_or(|entry| !entry.waiting())
        };
        let mut due = Vec::new();
        let mut kept = Vec::new();
        for swap in self.swaps.drain(..) {
            let files_quiet = quiet(&self.files, &swap.original) && quiet(&self.files, &swap.replacement);
            if swap.when == SwapAt::Now && files_quiet {
                due.push(swap);
            } else {
                kept.push(swap);
            }
        }
        self.swaps = kept;
        due
    }

    /// Takes every remove whose file has nothing in flight.  Whatever was
    /// held for the file goes with it, and anyone waiting on a write to it
    /// hears that it's done: the file was removed on purpose after it.
    pub(super) fn take_removes(&mut self) -> Vec<RemoveJob> {
        let mut due = Vec::new();
        let mut kept = VecDeque::new();
        for job in self.removes.drain(..) {
            if self.files.get(&job.path).is_some_and(|entry| entry.in_flight) {
                kept.push_back(job);
                continue;
            }
            if let Some(entry) = self.files.remove(&job.path) {
                for waiter in entry.write_waiters.into_iter().chain(entry.tail_waiters) {
                    let _ = waiter.send(Ok(()));
                }
            }
            due.push(job);
        }
        self.removes = kept;
        due
    }

    /// The files a swap touched are whatever the disk has now, so nothing
    /// held for them is right any more.  Nothing was waiting on them
    /// (`take_due_swaps()` saw to that), so there's nobody to tell.
    pub(super) fn forget_files(&mut self, original: &Path, replacement: &Path) {
        self.files.remove(original);
        self.files.remove(replacement);
    }

    // -----------------------------------------------------------------------
    // What the worker asks for
    // -----------------------------------------------------------------------

    /// Every file with something ready to go out, the ones touched longest
    /// ago first.
    pub(super) fn ready_to_flush(&self, now: Instant) -> Vec<PathBuf> {
        let mut ready: Vec<(&PathBuf, u64)> = self.files.iter()
            .filter(|(_, entry)| (entry.dirty || !entry.tail.is_empty()) && entry.ready_to_try(now))
            .map(|(path, entry)| (path, entry.last_used))
            .collect();
        ready.sort_by_key(|(_, used)| *used);
        ready.into_iter().map(|(path, _)| path.clone()).collect()
    }

    /// Takes the next thing to write for `path`, and marks it in flight.
    /// A whole file bigger than `big_above` is only taken if `big_allowed`,
    /// since only one big write runs at a time.
    pub(super) fn take_flush(&mut self, path: &Path, big_allowed: bool) -> Option<Flush> {
        let big_above = self.limits.big_above;
        let entry = self.files.get_mut(path)?;
        if !entry.ready_to_try(Instant::now()) {
            return None;
        }
        if entry.dirty {
            let content = Arc::clone(entry.content.as_ref()?);
            if content.len() > big_above && !big_allowed {
                return None;
            }
            // A dirty whole file already carries every append.
            entry.tail.clear();
            let from_tail = mem::take(&mut entry.tail_waiters);
            entry.write_waiters.extend(from_tail);

            entry.dirty = false;
            entry.in_flight = true;
            return Some(Flush::Whole {
                content,
                version: entry.version,
                waiters: mem::take(&mut entry.write_waiters),
            });
        }
        if !entry.tail.is_empty() {
            entry.in_flight = true;
            return Some(Flush::Tail {
                bytes: mem::take(&mut entry.tail),
                waiters: mem::take(&mut entry.tail_waiters),
            });
        }
        None
    }

    /// True if a newer `write()` has come in for `path` since `version`.
    pub(super) fn replaced(&self, path: &Path, version: u64) -> bool {
        self.files.get(path).is_none_or(|entry| entry.version != version)
    }

    /// A big write was dropped part way because a newer write replaced it.
    /// Its waiters go back on the entry, to hear when the newer one lands.
    pub(super) fn abandon(&mut self, path: &Path, waiters: Vec<Reply>) {
        if let Some(entry) = self.files.get_mut(path) {
            entry.in_flight = false;
            entry.write_waiters.extend(waiters);
        }
    }

    /// How a write went.  Everyone waiting on it hears the answer.  A
    /// failure puts the data back to be tried again after `RETRY_AFTER`,
    /// up to `MOST_TRIES`.
    pub(super) fn finish(&mut self, path: &Path, flush: Flush, result: Result<Duration, DiskError>)
                         -> Outcome {
        let Some(entry) = self.files.get_mut(path) else {
            return Outcome::Written { recovered: false, for_scribe: false };
        };
        entry.in_flight = false;
        let for_scribe = entry.for_scribe;

        let (waiters, put_back) = match flush {
            Flush::Whole { waiters, .. } => (waiters, None),
            Flush::Tail { bytes, waiters } => (waiters, Some(bytes)),
        };
        let written = result.is_ok();

        for waiter in waiters {
            let _ = waiter.send(result.clone().map(|_| ()));
        }

        let outcome = if written {
            let recovered = entry.failed_tries > 0;
            entry.failed_tries = 0;
            entry.retry_at = None;
            Outcome::Written { recovered, for_scribe }
        } else {
            // An append may have gone part way, so what's on disk is
            // anybody's guess until the next good write.
            entry.on_disk = None;
            entry.failed_tries += 1;
            if entry.failed_tries >= MOST_TRIES {
                // Given up.  What's on disk is whatever the last good write
                // left there, so the copy in memory isn't it any more.
                entry.failed_tries = 0;
                entry.retry_at = None;
                entry.content = None;
                entry.dirty = false;
                entry.tail.clear();
                for waiter in mem::take(&mut entry.write_waiters).into_iter()
                    .chain(mem::take(&mut entry.tail_waiters)) {
                    let _ = waiter.send(result.clone().map(|_| ()));
                }
                Outcome::GaveUp { for_scribe }
            } else {
                entry.retry_at = Some(Instant::now() + RETRY_AFTER);
                if let Some(bytes) = put_back {
                    if entry.dirty {
                        // A whole write came in behind it and carries
                        // everything, so the old bytes aren't needed.
                    } else {
                        let mut tail = bytes;
                        tail.extend_from_slice(&entry.tail);
                        entry.tail = tail;
                    }
                } else if !entry.dirty {
                    entry.dirty = true;
                }
                Outcome::Failed { first: entry.failed_tries == 1, for_scribe }
            }
        };

        match (&outcome, result) {
            (Outcome::Written { .. }, Ok(took)) => self.totals.slowest_write = self.totals.slowest_write.max(took),
            (_, Err(e)) => {
                self.totals.failures += 1;
                self.totals.last_failure = Some((Utc::now(), format!("{}: {e}", path.display())));
                if matches!(outcome, Outcome::GaveUp { .. }) {
                    self.totals.given_up += 1;
                }
            }
            _ => {}
        }

        if self.files.get(path).is_some_and(Entry::empty) {
            self.files.remove(path);
        }
        outcome
    }

    /// A write of ours has landed, and the disk says `stamp` for the file
    /// now.  Noted so the next read doesn't take our own write for somebody
    /// else's edit.
    pub(super) fn note_on_disk(&mut self, path: &Path, stamp: Option<Stamp>) {
        if let Some(entry) = self.files.get_mut(path) {
            entry.on_disk = stamp;
        }
    }

    /// Unloads clean files, the one used longest ago first, until what's
    /// held is under `clean_cache_bytes`.  Also drops entries with nothing
    /// left in them.
    pub(super) fn unload_extra(&mut self) {
        self.files.retain(|_, entry| !entry.empty());
        loop {
            let held: usize = self.files.values()
                .filter(|entry| entry.clean_copy())
                .map(|entry| entry.content.as_ref().map_or(0, |content| content.len()))
                .sum();
            if held <= self.limits.clean_cache_bytes {
                return;
            }
            let oldest = self.files.iter()
                .filter(|(_, entry)| entry.clean_copy())
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(path, _)| path.clone());
            match oldest {
                Some(path) => {
                    self.files.remove(&path);
                }
                None => return,
            }
        }
    }

    /// Nothing left to write or read.  Streams don't count: they're closed
    /// at shutdown rather than finished.
    pub(super) fn all_done(&self) -> bool {
        self.reads.is_empty() && self.checks.is_empty() && self.removes.is_empty()
            && self.files.values().all(|entry| !entry.waiting())
    }

    /// Files that still have something to go out, for the shutdown notes.
    pub(super) fn waiting_files(&self) -> Vec<PathBuf> {
        self.files.iter().filter(|(_, entry)| entry.waiting()).map(|(path, _)| path.clone()).collect()
    }

    pub(super) fn status(&self) -> Status {
        let mut status = Status {
            running: self.running,
            stopping: self.stopping,
            files_waiting: 0,
            bytes_waiting: 0,
            files_loaded: 0,
            bytes_loaded: 0,
            files_failing: 0,
            reads_waiting: self.reads.len() + self.checks.len(),
            streams_open: self.streams_open + self.new_streams.len(),
            swaps_waiting: self.swaps.len(),
            big_write: self.big_write.clone(),
            writes_done: self.totals.writes_done,
            appends_done: self.totals.appends_done,
            reads_done: self.totals.reads_done,
            cache_hits: self.totals.cache_hits,
            bytes_written: self.totals.bytes_written,
            bytes_read: self.totals.bytes_read,
            failures: self.totals.failures,
            given_up: self.totals.given_up,
            last_failure: self.totals.last_failure.clone(),
            slowest_write: self.totals.slowest_write,
        };
        for entry in self.files.values() {
            let content_len = entry.content.as_ref().map_or(0, |content| content.len()) as u64;
            if entry.waiting() {
                status.files_waiting += 1;
                status.bytes_waiting += if entry.dirty { content_len } else { entry.tail.len() as u64 };
            }
            if entry.content.is_some() {
                status.files_loaded += 1;
                status.bytes_loaded += content_len;
            }
            if entry.failed_tries > 0 {
                status.files_failing += 1;
            }
        }
        status
    }
}

/// What came of a write, for the worker to report once the lock is let go.
pub(super) enum Outcome {
    /// On disk.  `recovered` if the try before this one failed.
    Written { recovered: bool, for_scribe: bool },
    /// Didn't make it, and it'll be tried again.  `first` for the first
    /// failure in a row, which is the one that gets reported.
    Failed { first: bool, for_scribe: bool },
    /// Didn't make it after `MOST_TRIES`, and what we held for it is gone.
    GaveUp { for_scribe: bool },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    const TINY: Limits = Limits { chunk_bytes: 4, big_above: 16, clean_cache_bytes: 10 };

    fn reply() -> (Reply, mpsc::Receiver<Result<(), DiskError>>) {
        mpsc::channel()
    }

    fn path(name: &str) -> PathBuf {
        PathBuf::from("/nowhere").join(name)
    }

    #[test]
    fn a_second_write_replaces_the_first_before_it_goes_out() {
        let mut state = State::new(TINY);
        let (first, first_hears) = reply();
        let (second, second_hears) = reply();
        state.write(&path("a"), b"old".to_vec(), first);
        state.write(&path("a"), b"new".to_vec(), second);

        let Some(Flush::Whole { content, version, waiters }) = state.take_flush(&path("a"), true) else {
            panic!("there should be a whole write to do");
        };
        assert_eq!(content.as_slice(), b"new");
        assert_eq!(version, 2);
        // Both callers hear back from the one write.
        assert_eq!(waiters.len(), 2);

        state.finish(&path("a"), Flush::Whole { content, version, waiters }, Ok(Duration::ZERO));
        assert!(first_hears.try_recv().is_ok_and(|answer| answer.is_ok()));
        assert!(second_hears.try_recv().is_ok_and(|answer| answer.is_ok()));
        assert!(state.take_flush(&path("a"), true).is_none());
    }

    #[test]
    fn appends_pile_up_in_the_tail_and_go_out_together() {
        let mut state = State::new(TINY);
        state.append(&path("log"), b"one\n", reply().0, false);
        state.append(&path("log"), b"two\n", reply().0, false);

        let Some(Flush::Tail { bytes, .. }) = state.take_flush(&path("log"), true) else {
            panic!("there should be a tail to write");
        };
        assert_eq!(bytes, b"one\ntwo\n");
    }

    #[test]
    fn an_append_while_a_whole_write_is_out_means_writing_it_whole_again() {
        let mut state = State::new(TINY);
        state.write(&path("a"), b"abc".to_vec(), reply().0);
        let flush = state.take_flush(&path("a"), true).expect("a whole write");
        state.append(&path("a"), b"d", reply().0, false);
        state.finish(&path("a"), flush, Ok(Duration::ZERO));

        let Some(Flush::Whole { content, .. }) = state.take_flush(&path("a"), true) else {
            panic!("the file should go out whole again");
        };
        assert_eq!(content.as_slice(), b"abcd");
    }

    #[test]
    fn a_read_sees_the_tail_that_hasnt_gone_out() {
        let mut state = State::new(TINY);
        state.append(&path("log"), b"new", reply().0, false);
        let read = state.after_disk_read(&path("log"), Ok(b"old ".to_vec()), None).expect("a good read");
        assert_eq!(read.as_slice(), b"old new");
        // The tail hasn't gone out, so the copy is newer than the disk.
        assert!(matches!(state.held(&path("log")), Held::Newest(_)));
    }

    #[test]
    fn a_failed_tail_goes_back_in_front_and_three_failures_give_up() {
        let mut state = State::new(TINY);
        state.append(&path("x"), b"first", reply().0, false);
        let broken = || Err(DiskError::Failed { kind: std::io::ErrorKind::Other, why: "broken".to_string() });

        let flush = state.take_flush(&path("x"), true).expect("a tail");
        state.append(&path("x"), b"-second", reply().0, false);
        assert!(matches!(state.finish(&path("x"), flush, broken()), Outcome::Failed { first: true, .. }));
        assert_eq!(state.files[&path("x")].tail, b"first-second");

        // Too soon for another try.
        assert!(state.take_flush(&path("x"), true).is_none());
        for tries_left in (1..MOST_TRIES).rev() {
            state.files.get_mut(&path("x")).expect("still there").retry_at = None;
            let flush = state.take_flush(&path("x"), true).expect("a retry");
            let outcome = state.finish(&path("x"), flush, broken());
            if tries_left == 1 {
                assert!(matches!(outcome, Outcome::GaveUp { .. }));
            }
        }
        assert!(state.all_done());
    }

    #[test]
    fn clean_files_unload_oldest_first_and_dirty_ones_stay() {
        let mut state = State::new(TINY);
        state.after_disk_read(&path("old"), Ok(b"123456".to_vec()), None).expect("read");
        state.after_disk_read(&path("new"), Ok(b"abcdef".to_vec()), None).expect("read");
        state.write(&path("dirty"), b"0123456789ABCDEF".to_vec(), reply().0);
        state.unload_extra();

        assert!(!state.files.contains_key(&path("old")));
        assert!(state.files.contains_key(&path("new")));
        assert!(state.files.contains_key(&path("dirty")));
    }

    #[test]
    fn a_swap_waits_for_its_files_to_be_quiet() {
        let mut state = State::new(TINY);
        state.write(&path("live"), b"old".to_vec(), reply().0);
        let (asked, hears) = reply();
        state.add_swap(Swap {
            original: path("live"),
            replacement: path("live.wait4server"),
            when: SwapAt::ServerStop,
            reply: Some(asked),
        });

        // Not due yet, and then due but the live file is still waiting.
        assert!(state.take_due_swaps().is_empty());
        state.release_swaps(SwapAt::ServerStop);
        assert!(state.swaps_due());
        assert!(state.take_due_swaps().is_empty());

        let flush = state.take_flush(&path("live"), true).expect("the write");
        state.finish(&path("live"), flush, Ok(Duration::ZERO));
        let due = state.take_due_swaps();
        assert_eq!(due.len(), 1);
        assert!(!state.swaps_due());
        assert!(hears.try_recv().is_err(), "nobody has answered yet: the worker does that");
    }

    #[test]
    fn a_second_swap_for_the_same_file_replaces_the_first() {
        let mut state = State::new(TINY);
        let (first, first_hears) = reply();
        let shutdown = SwapAt::Shutdown;
        state.add_swap(Swap { original: path("a"), replacement: path("a.1"), when: shutdown, reply: Some(first) });
        state.add_swap(Swap { original: path("a"), replacement: path("a.2"), when: shutdown, reply: None });
        assert_eq!(state.swaps.len(), 1);
        assert_eq!(state.swaps[0].replacement, path("a.2"));
        assert!(first_hears.try_recv().is_ok_and(|answer| answer.is_ok()));

        state.forget_swap(&path("a"));
        assert!(state.swaps.is_empty());
    }

    #[test]
    fn a_clean_copy_is_checked_against_the_disk_and_ours_wins_while_it_waits() {
        let mut state = State::new(TINY);
        let then = Stamp { modified: SystemTime::UNIX_EPOCH, len: 3 };
        state.after_disk_read(&path("a"), Ok(b"old".to_vec()), Some(then)).expect("a good read");
        assert!(matches!(state.held(&path("a")), Held::Check));

        // The disk says the same: the copy is the answer.
        let same = state.after_check(&path("a"), Some(then)).expect("the copy");
        assert_eq!(same.as_slice(), b"old");

        // The disk says otherwise: somebody edited it, so the copy goes.
        let later = Stamp { modified: SystemTime::UNIX_EPOCH + Duration::from_secs(1), len: 3 };
        assert!(state.after_check(&path("a"), Some(later)).is_none());
        assert!(matches!(state.held(&path("a")), Held::Nothing));

        // The disk won't say (the file is gone, say): not trusted either.
        state.after_disk_read(&path("b"), Ok(b"bee".to_vec()), Some(then)).expect("a good read");
        assert!(state.after_check(&path("b"), None).is_none());

        // A write of ours that hasn't gone out is newer than any disk.
        state.write(&path("a"), b"ours".to_vec(), reply().0);
        assert!(matches!(state.held(&path("a")), Held::Newest(_)));
        assert!(state.current(&path("a"), Some(later)).is_some());
    }

    #[test]
    fn a_big_write_waits_its_turn() {
        let mut state = State::new(TINY);
        state.write(&path("big"), vec![7; 20], reply().0);
        assert!(state.take_flush(&path("big"), false).is_none());
        assert!(matches!(state.take_flush(&path("big"), true), Some(Flush::Whole { .. })));
    }
}
