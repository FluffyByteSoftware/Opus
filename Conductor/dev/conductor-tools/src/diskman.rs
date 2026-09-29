//! File:       Opus/Conductor/dev/conductor-tools/src/diskman.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! DiskMan, the disk manager.  The layer between the server and the disk:
//! every file Conductor writes goes through here, and so does every file
//! it reads.  It's the first thing Conductor starts and the last thing it
//! stops, since everything else leans on it -- Scribe's log included.
//!
//! The idea is that nobody but DiskMan ever waits on the disk.  `write()`,
//! `append()` and `read()` hand back a `Pending` straight away, the same
//! as Archivist's jobs, and one worker thread keeps streaming whatever is
//! waiting out to the disk as fast as the disk will take it.  Whatever
//! comes in while the worker is busy is held in memory:
//!
//! - A second `write()` to a file that hasn't gone out yet replaces the
//!   first in memory, so only the newest ever reaches the disk.
//! - A file that's been read stays loaded, so the next read of it never
//!   touches the disk.  A write to a loaded file changes the copy in
//!   memory and marks it dirty, and the worker writes it out.
//! - A clean file (one the disk already has) can be unloaded when memory
//!   gets tight.  A dirty one is the only copy, so it stays however big
//!   it is -- a few gigabytes of world terrain included -- until it's on
//!   disk.
//!
//! A whole-file write never leaves half a file.  It goes to a temp file,
//! gets flushed all the way to the disk, and is renamed over the old one.
//! A big one (world terrain, say) goes into its temp file a chunk at a
//! time, and other files get their turn in between, so it never holds up
//! a config write or a log line.  `stream()` reads a big file back the
//! same way, a chunk at a time, without loading all of it.
//!
//! Two more jobs, for the config files.  `swap()` renames one file over
//! another -- now, or held in a list until the server stops or Conductor
//! shuts down, which is how a change saved from the web admin waits for
//! its reboot (Constellations writes `name.cfg.wait4server` and asks for
//! the swap).  `remove()` deletes a file.  Both drop whatever DiskMan held
//! for the files, so the next read goes to the disk.
//!
//! A crash is still the worst case.  Whatever was in memory and not yet
//! on disk is gone, and no amount of care gets around that.  What DiskMan
//! makes sure of is that a crash never leaves a file half written.
//!
//! The rest is in `diskman/`: `cache.rs` holds the files and the rules for
//! changing them, and `worker.rs` is the thread that does the disk work.

mod cache;
mod worker;

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cache::{Limits, ReadJob, RemoveJob, State, StreamJob, Swap};

use crate::clock::Utc;
use crate::pending::NotRunning;

/// The sizes DiskMan works to.
const LIMITS: Limits = Limits {
    // 1 MB.  Big enough that the disk isn't asked for crumbs, small enough
    // that the other files waiting behind a big one never wait long.
    chunk_bytes: 1024 * 1024,
    // A file over 8 MB goes out in chunks.  Every config and save we have
    // today is a few KB, so this is for the world terrain.
    big_above: 8 * 1024 * 1024,
    // 256 MB of files already on disk can stay loaded.  Dirty files don't
    // count toward this and are never unloaded.
    clean_cache_bytes: 256 * 1024 * 1024,
};

/// How many chunks a stream reads ahead of its reader.  4 MB at 1 MB a
/// chunk, so a slow reader doesn't end up with the whole file in memory.
const STREAM_AHEAD: usize = 4;

/// A file's bytes, shared.  A file DiskMan has loaded goes out to every
/// reader as this, so nobody pays for a copy of it.
// Rust note: `Arc` is a shared, read-only handle to data that stays alive
// until the last holder lets go of it.  Handing out another one is just a
// count going up, never a copy of the bytes.
pub type Contents = Arc<Vec<u8>>;

/// Why a read or write didn't happen.
// Rust note: an `io::Error` can't be copied, and one failed write can have
// several callers waiting on it, so the kind and the words are kept
// instead.
#[derive(Debug, Clone)]
pub enum DiskError {
    /// DiskMan isn't running: `start()` hasn't happened, it has finished
    /// shutting down, or its worker died.
    NotRunning,
    /// The disk said no.  `kind` says what sort of no (`NotFound`,
    /// `PermissionDenied`, ...).
    Failed { kind: io::ErrorKind, why: String },
}

impl DiskError {
    fn from_io(e: &io::Error) -> DiskError {
        DiskError::Failed { kind: e.kind(), why: e.to_string() }
    }

    /// The file isn't there.  Usually the one worth handling on its own,
    /// by writing the file with its defaults.
    pub fn is_not_found(&self) -> bool {
        matches!(self, DiskError::Failed { kind: io::ErrorKind::NotFound, .. })
    }
}

impl fmt::Display for DiskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiskError::NotRunning => write!(f, "DiskMan isn't running"),
            DiskError::Failed { why, .. } => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for DiskError {}

impl NotRunning for DiskError {
    fn not_running() -> DiskError {
        DiskError::NotRunning
    }
}

/// An answer that is on its way: `check()` never waits, `wait()` does.
/// The workings are in `pending.rs`, shared with Archivist.
pub type Pending<T> = crate::pending::Pending<T, DiskError>;

/// One piece of a stream.
#[derive(Debug)]
pub enum Piece {
    /// The next chunk of the file, up to 1 MB.
    Bytes(Vec<u8>),
    /// That was all of it.
    Done,
    /// It stopped part way, and why.  Nothing more comes after this.
    Failed(DiskError),
}

/// A file coming back a chunk at a time.  Ask it for the next piece until
/// it says `Done` (or `Failed`).  Dropping it part way tells the worker to
/// stop reading.
pub struct Stream {
    pieces: Receiver<Piece>,
}

impl Stream {
    /// The next piece if it's here, `None` if the worker hasn't read it
    /// yet.  Never waits.
    pub fn check(&self) -> Option<Piece> {
        match self.pieces.try_recv() {
            Ok(piece) => Some(piece),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Piece::Failed(DiskError::NotRunning)),
        }
    }

    /// Waits for the next piece.  Not for the game loop.
    pub fn wait(&self) -> Piece {
        self.pieces.recv().unwrap_or(Piece::Failed(DiskError::NotRunning))
    }
}

/// When a `swap()` happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapAt {
    /// As soon as the worker gets to it.
    Now,
    /// When the launcher says the server pieces have stopped
    /// (`run_swaps(SwapAt::ServerStop)`).  A soft config file.
    ServerStop,
    /// As DiskMan's last act at Conductor's shutdown, after everything
    /// else is written.  A hard config file.
    Shutdown,
}

/// How DiskMan is doing, for the web admin's Storage tab.
#[derive(Debug, Clone)]
pub struct Status {
    /// Taking jobs.
    pub running: bool,
    /// Shutting down: finishing what it holds, then ending.
    pub stopping: bool,
    /// Files with something in memory that hasn't reached the disk.
    pub files_waiting: usize,
    pub bytes_waiting: u64,
    /// Files held whole in memory, dirty or clean.
    pub files_loaded: usize,
    pub bytes_loaded: u64,
    /// Files whose last write failed and are waiting to try again.
    pub files_failing: usize,
    /// Reads waiting on the disk.
    pub reads_waiting: usize,
    pub streams_open: usize,
    /// Swaps held for a server stop or Conductor's shutdown.
    pub swaps_waiting: usize,
    /// The big write under way: the file, how many bytes are out, and of
    /// how many.
    pub big_write: Option<(PathBuf, u64, u64)>,
    /// Whole files written since Conductor started.
    pub writes_done: u64,
    /// Batches of appends written.  Many `append()` calls can go out as one.
    pub appends_done: u64,
    /// Reads that went to the disk.
    pub reads_done: u64,
    /// Reads answered from memory.
    pub cache_hits: u64,
    pub bytes_written: u64,
    pub bytes_read: u64,
    /// Tries that failed, counting retries.
    pub failures: u64,
    /// Files DiskMan gave up on after its tries ran out.
    pub given_up: u64,
    /// When the last failure was, and the file and what the disk said.
    pub last_failure: Option<(Utc, String)>,
    pub slowest_write: Duration,
}

/// DiskMan's state and the worker's thread.  There's one of these for the
/// server; the tests make their own.
struct Shared {
    state: Mutex<State>,
    /// Rings when a job comes in or `stop()` is called, so a sleeping
    /// worker wakes up.
    // Rust note: a `Condvar` is a bell that one thread can wait on while
    // it's let go of a lock, and another can ring.
    wake: Condvar,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Shared {
    const fn new(limits: Limits) -> Shared {
        Shared {
            state: Mutex::new(State::new(limits)),
            wake: Condvar::new(),
            worker: Mutex::new(None),
        }
    }

    fn write(&self, path: &Path, bytes: Vec<u8>) -> Pending<()> {
        let mut state = self.lock();
        if !state.running {
            return Pending::ready(Err(DiskError::NotRunning));
        }
        let (reply, pending) = Pending::new();
        state.write(path, bytes, reply);
        drop(state);
        self.wake.notify_one();
        pending
    }

    fn append(&self, path: &Path, bytes: &[u8], for_scribe: bool) -> Pending<()> {
        let mut state = self.lock();
        if !state.running {
            return Pending::ready(Err(DiskError::NotRunning));
        }
        let (reply, pending) = Pending::new();
        state.append(path, bytes, reply, for_scribe);
        drop(state);
        self.wake.notify_one();
        pending
    }

    fn read(&self, path: &Path) -> Pending<Contents> {
        let mut state = self.lock();
        if !state.running {
            return Pending::ready(Err(DiskError::NotRunning));
        }
        if let Some(content) = state.read_hit(path) {
            return Pending::ready(Ok(content));
        }
        let (reply, pending) = Pending::new();
        state.reads.push_back(ReadJob { path: path.to_path_buf(), reply });
        drop(state);
        self.wake.notify_one();
        pending
    }

    fn stream(&self, path: &Path) -> Stream {
        let (reply, pieces) = mpsc::sync_channel(STREAM_AHEAD);
        let mut state = self.lock();
        if !state.running {
            // Dropping `reply` here is what tells the stream.
            return Stream { pieces };
        }
        state.new_streams.push(StreamJob { path: path.to_path_buf(), reply });
        drop(state);
        self.wake.notify_one();
        Stream { pieces }
    }

    fn swap(&self, original: &Path, replacement: &Path, when: SwapAt) -> Pending<()> {
        let mut state = self.lock();
        if !state.running {
            return Pending::ready(Err(DiskError::NotRunning));
        }
        let (reply, pending) = Pending::new();
        state.add_swap(Swap {
            original: original.to_path_buf(),
            replacement: replacement.to_path_buf(),
            when,
            reply: Some(reply),
        });
        drop(state);
        self.wake.notify_one();
        pending
    }

    fn run_swaps(&self, when: SwapAt) -> Pending<()> {
        let mut state = self.lock();
        if !state.running {
            return Pending::ready(Err(DiskError::NotRunning));
        }
        state.release_swaps(when);
        let (reply, pending) = Pending::new();
        state.swap_fences.push(reply);
        drop(state);
        self.wake.notify_one();
        pending
    }

    fn forget_swap(&self, original: &Path) {
        self.lock().forget_swap(original);
    }

    fn remove(&self, path: &Path) -> Pending<()> {
        let mut state = self.lock();
        if !state.running {
            return Pending::ready(Err(DiskError::NotRunning));
        }
        let (reply, pending) = Pending::new();
        state.removes.push_back(RemoveJob { path: path.to_path_buf(), reply });
        drop(state);
        self.wake.notify_one();
        pending
    }

    fn stop(&self) {
        self.lock().stopping = true;
        self.wake.notify_one();
    }

    fn finished(&self) -> bool {
        let worker = self.worker.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        worker.as_ref().is_none_or(|handle| handle.is_finished())
    }
}

static DISKMAN: Shared = Shared::new(LIMITS);

// ---------------------------------------------------------------------------
// Starting and stopping
// ---------------------------------------------------------------------------

/// Starts the worker.  main calls this first of all, before Scribe, since
/// Scribe's log goes through DiskMan.  Until it runs, every job comes back
/// "not running".
pub fn start() {
    worker::start(&DISKMAN);
}

/// Asks DiskMan to finish up: write out everything it's holding, then end.
/// It comes back straight away, and DiskMan keeps taking jobs (Scribe's
/// lines, mostly) until it's done.  Check `finished()` to know when.  main
/// calls this last of all, after Archivist has stopped.
pub fn stop() {
    DISKMAN.stop();
}

/// True once the worker has ended, which after `stop()` means everything
/// it held is on disk (or given up on, and the log says so).  Also true if
/// it never started.
pub fn finished() -> bool {
    DISKMAN.finished()
}

/// How DiskMan is doing right now.
pub fn status() -> Status {
    DISKMAN.lock().status()
}

/// The files that still have something waiting to reach the disk, for the
/// shutdown countdown.
pub fn waiting_files() -> Vec<PathBuf> {
    DISKMAN.lock().waiting_files()
}

// ---------------------------------------------------------------------------
// Jobs
// ---------------------------------------------------------------------------

// Every path handed in should be a full one, from
// `constellations::content_dir()`.  DiskMan goes by the path as given, so
// the same file under two spellings would be two files to it.

/// Replaces the whole file with `bytes`, making it and its folder if they
/// aren't there.  The `Pending` says when it's on disk.  If another write
/// to the same file replaces this one before it goes out, the `Pending`
/// hears back when the newer one lands.
///
/// An error means the first try failed.  DiskMan keeps what it was handed
/// and tries twice more before giving up on it, and the log says which.
///
/// ```text
/// diskman::write(&path, text.into_bytes());          // and carry on
/// diskman::write(&path, text.into_bytes()).wait()?;  // at startup, to know
/// ```
pub fn write(path: &Path, bytes: Vec<u8>) -> Pending<()> {
    DISKMAN.write(path, bytes)
}

/// Adds `bytes` to the end of the file, making it if it isn't there.
/// Appends to one file go out in the order they came in, and many of them
/// can go out as one write.
pub fn append(path: &Path, bytes: &[u8]) -> Pending<()> {
    DISKMAN.append(path, bytes, false)
}

/// Scribe's own appends.  The same as `append()`, except that trouble
/// writing them goes to the console, not the log.
pub(crate) fn append_for_scribe(path: &Path, bytes: &[u8]) {
    // Scribe has no use for the answer: a line that doesn't make it to
    // the file has already reached the console.
    let _ = DISKMAN.append(path, bytes, true);
}

/// The whole file.  From memory, straight away, if DiskMan holds it,
/// including a write that hasn't gone out yet.  Otherwise the worker reads
/// it from the disk, and it stays loaded for next time.
///
/// ```text
/// match diskman::read(&path).wait() {
///     Ok(bytes) => ...,
///     Err(e) if e.is_not_found() => ...,
///     Err(e) => ...,
/// }
/// ```
pub fn read(path: &Path) -> Pending<Contents> {
    DISKMAN.read(path)
}

/// The file a chunk at a time, for one too big to load (world terrain).
/// Nothing streamed is kept in memory.  What it hands back is the newest
/// the file is, the same as `read()`.
pub fn stream(path: &Path) -> Stream {
    DISKMAN.stream(path)
}

/// Renames `replacement` over `original`, so the file at `original` is
/// what `replacement` held and `replacement` is gone.  `when` says when:
/// now, when the server stops, or at Conductor's shutdown.  Whatever
/// DiskMan held for either file is dropped when it happens, so the next
/// read sees the new file.  A second swap for the same `original`
/// replaces the first in the list.  If `replacement` isn't there when the
/// time comes (it was discarded), nothing happens and that counts as done.
///
/// The `Pending` answers when the swap has happened.  For a held one that
/// can be a long time, and the caller needn't wait on it: DiskMan says so
/// in the log if a swap fails.  A read asked for straight after a swap
/// should wait on the swap first, or it may still see the old file.
///
/// ```text
/// diskman::swap(&live, &waiting, SwapAt::Now).wait()?;   // then read `live`
/// diskman::swap(&live, &waiting, SwapAt::Shutdown);       // and forget about it
/// ```
pub fn swap(original: &Path, replacement: &Path, when: SwapAt) -> Pending<()> {
    DISKMAN.swap(original, replacement, when)
}

/// Runs every swap held for `when` now.  The launcher calls this with
/// `SwapAt::ServerStop` once the server pieces are down, and waits on the
/// answer, so the next START SERVER reads the swapped files.  The
/// `SwapAt::Shutdown` ones run on their own as DiskMan stops, and so does
/// anything else still in the list then: a hard reboot applies the lot.
/// The `Pending` answers once every swap that was due has happened.
pub fn run_swaps(when: SwapAt) -> Pending<()> {
    DISKMAN.run_swaps(when)
}

/// Takes the swap for `original` out of the list, if there is one.  For
/// a change that was discarded.
pub fn forget_swap(original: &Path) {
    DISKMAN.forget_swap(original);
}

/// Deletes the file, and drops whatever DiskMan held for it.  A file
/// that isn't there counts as removed.
pub fn remove(path: &Path) -> Pending<()> {
    DISKMAN.remove(path)
}

/// The bytes as text, for the files that are text (configs, SQL).  Bytes
/// that aren't text come back as an error, the same as a read that failed.
///
/// ```text
/// let bytes = diskman::read(&path).wait()?;
/// let text = diskman::as_text(&bytes)?;
/// ```
pub fn as_text(bytes: &[u8]) -> Result<String, DiskError> {
    String::from_utf8(bytes.to_vec()).map_err(|e| DiskError::Failed {
        kind: io::ErrorKind::InvalidData,
        why: format!("not text: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A DiskMan of the test's own, with tiny sizes so a few bytes is a
    /// "big" file.  Leaked on purpose: the worker needs it to outlive the
    /// test, and a few of them is nothing.
    fn own_diskman() -> &'static Shared {
        let limits = Limits { chunk_bytes: 4, big_above: 16, clean_cache_bytes: 64 };
        Box::leak(Box::new(Shared::new(limits)))
    }

    /// A clean folder for one test, under the system's temp folder.
    fn folder(test: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("opus-diskman-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        folder
    }

    fn stop_and_wait(diskman: &Shared) {
        diskman.stop();
        while !diskman.finished() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn nothing_is_taken_before_start() {
        let diskman = own_diskman();
        let path = folder("not-started").join("a.txt");
        assert!(matches!(diskman.write(&path, b"x".to_vec()).wait(), Err(DiskError::NotRunning)));
        assert!(matches!(diskman.read(&path).wait(), Err(DiskError::NotRunning)));
        assert!(matches!(diskman.stream(&path).wait(), Piece::Failed(DiskError::NotRunning)));
    }

    #[test]
    fn a_write_lands_and_reads_back() {
        let diskman = own_diskman();
        assert!(worker::start(diskman));
        let path = folder("write").join("deeper").join("a.cfg");

        diskman.write(&path, b"first".to_vec());
        diskman.write(&path, b"second".to_vec()).wait().expect("the write should land");
        assert_eq!(fs::read(&path).expect("the file should be there"), b"second");
        assert!(!path.with_file_name("a.cfg.diskman-tmp").exists());

        let read = diskman.read(&path).wait().expect("the read should work");
        assert_eq!(read.as_slice(), b"second");
        stop_and_wait(diskman);
    }

    #[test]
    fn appends_land_in_order() {
        let diskman = own_diskman();
        assert!(worker::start(diskman));
        let path = folder("append").join("x.log");

        for n in 0..50 {
            diskman.append(&path, format!("{n}\n").as_bytes(), false);
        }
        diskman.append(&path, b"end\n", false).wait().expect("the appends should land");

        let expected: String = (0..50).map(|n| format!("{n}\n")).collect::<String>() + "end\n";
        assert_eq!(fs::read_to_string(&path).expect("the file should be there"), expected);
        stop_and_wait(diskman);
    }

    #[test]
    fn a_missing_file_says_so() {
        let diskman = own_diskman();
        assert!(worker::start(diskman));
        let answer = diskman.read(&folder("missing").join("nope.cfg")).wait();
        assert!(answer.is_err_and(|e| e.is_not_found()));
        stop_and_wait(diskman);
    }

    #[test]
    fn a_big_file_goes_out_in_chunks_and_streams_back() {
        let diskman = own_diskman();
        assert!(worker::start(diskman));
        let path = folder("big").join("terrain.bin");
        let bytes: Vec<u8> = (0..100).collect();

        diskman.write(&path, bytes.clone()).wait().expect("the big write should land");
        assert_eq!(fs::read(&path).expect("the file should be there"), bytes);

        let stream = diskman.stream(&path);
        let mut back = Vec::new();
        loop {
            match stream.wait() {
                Piece::Bytes(chunk) => {
                    assert!(chunk.len() <= 4);
                    back.extend(chunk);
                }
                Piece::Done => break,
                Piece::Failed(e) => panic!("the stream failed: {e}"),
            }
        }
        assert_eq!(back, bytes);
        stop_and_wait(diskman);
    }

    #[test]
    fn a_swap_now_puts_the_replacement_in_place() {
        let diskman = own_diskman();
        assert!(worker::start(diskman));
        let folder = folder("swap-now");
        let live = folder.join("a.cfg");
        let waiting = folder.join("a.cfg.wait4server");
        diskman.write(&live, b"old".to_vec()).wait().expect("the live file should land");
        diskman.write(&waiting, b"new".to_vec()).wait().expect("the waiting file should land");
        // Both are held in memory now.  The swap has to drop them.
        assert_eq!(diskman.read(&live).wait().expect("a read").as_slice(), b"old");

        diskman.swap(&live, &waiting, SwapAt::Now).wait().expect("the swap should happen");
        assert_eq!(diskman.read(&live).wait().expect("a read").as_slice(), b"new");
        assert!(!waiting.exists());
        assert_eq!(diskman.status().swaps_waiting, 0);

        // Nothing waiting is fine: the swap counts as done.
        diskman.swap(&live, &waiting, SwapAt::Now).wait().expect("nothing to swap is fine");
        assert_eq!(fs::read(&live).expect("still there"), b"new");
        stop_and_wait(diskman);
    }

    #[test]
    fn a_held_swap_waits_for_its_moment() {
        let diskman = own_diskman();
        assert!(worker::start(diskman));
        let folder = folder("swap-held");
        let soft = folder.join("soft.cfg");
        let hard = folder.join("hard.cfg");
        for (path, text) in [(&soft, "old soft"), (&hard, "old hard")] {
            diskman.write(path, text.as_bytes().to_vec()).wait().expect("the file should land");
            diskman.write(&path.with_extension("cfg.wait4server"), format!("new {text}").into_bytes())
                .wait()
                .expect("the waiting file should land");
        }
        diskman.swap(&soft, &soft.with_extension("cfg.wait4server"), SwapAt::ServerStop);
        diskman.swap(&hard, &hard.with_extension("cfg.wait4server"), SwapAt::Shutdown);
        // A second swap for the same file replaces the first.
        diskman.swap(&hard, &hard.with_extension("cfg.wait4server"), SwapAt::Shutdown);
        assert_eq!(diskman.status().swaps_waiting, 2);
        assert_eq!(fs::read(&soft).expect("there"), b"old soft");

        diskman.run_swaps(SwapAt::ServerStop).wait().expect("the server-stop swaps should run");
        assert_eq!(diskman.read(&soft).wait().expect("a read").as_slice(), b"new old soft");
        assert_eq!(fs::read(&hard).expect("there"), b"old hard");
        assert_eq!(diskman.status().swaps_waiting, 1);

        stop_and_wait(diskman);
        assert_eq!(fs::read(&hard).expect("there"), b"new old hard");
        assert!(!hard.with_extension("cfg.wait4server").exists());
    }

    #[test]
    fn a_forgotten_swap_never_happens_and_a_remove_deletes() {
        let diskman = own_diskman();
        assert!(worker::start(diskman));
        let folder = folder("swap-forget");
        let live = folder.join("a.cfg");
        let waiting = folder.join("a.cfg.wait4server");
        diskman.write(&live, b"old".to_vec()).wait().expect("the live file should land");
        diskman.write(&waiting, b"new".to_vec()).wait().expect("the waiting file should land");
        diskman.swap(&live, &waiting, SwapAt::Shutdown);
        diskman.forget_swap(&live);
        assert_eq!(diskman.status().swaps_waiting, 0);

        diskman.remove(&waiting).wait().expect("the remove should happen");
        assert!(!waiting.exists());
        assert!(diskman.read(&waiting).wait().is_err_and(|e| e.is_not_found()));
        diskman.remove(&waiting).wait().expect("removing what isn't there is fine");

        stop_and_wait(diskman);
        assert_eq!(fs::read(&live).expect("there"), b"old");
    }

    #[test]
    fn stopping_writes_everything_first() {
        let diskman = own_diskman();
        assert!(worker::start(diskman));
        let folder = folder("stop");
        for n in 0..20 {
            diskman.write(&folder.join(format!("{n}.txt")), vec![b'a'; n]);
        }
        stop_and_wait(diskman);

        for n in 0..20 {
            assert_eq!(fs::read(folder.join(format!("{n}.txt"))).expect("every file should be there"),
                       vec![b'a'; n]);
        }
        assert!(matches!(diskman.write(&folder.join("late.txt"), Vec::new()).wait(),
                         Err(DiskError::NotRunning)));
    }
}
