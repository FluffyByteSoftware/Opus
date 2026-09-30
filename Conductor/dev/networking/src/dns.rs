//! File:       Opus/Conductor/dev/networking/src/dns.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The names behind the addresses on the ledger, from a reverse DNS
//! lookup ("DNS if known", Jacob's words for the TCP tab, 2026-09-29).
//! A lookup asks the OS's resolver, which can take seconds when the
//! network is slow or the address has no name, so none of it runs on the
//! acceptor or a login thread: `ask()` puts the address on a queue and
//! comes straight back, and one thread, `net-dns`, works through the
//! queue and writes what it finds into a cache.  `name_of()` reads the
//! cache and never waits.  An address is looked up once per START SERVER;
//! the cache is wiped on the next.
//!
//! Every OS asks its resolver a different way, so `dns/linux.rs` and
//! `dns/windows.rs` each hold a `reverse()` around the OS's own
//! `getnameinfo`, and `dns/other.rs` says it doesn't know.  Only the one
//! for the OS being built for is compiled.  No name is never a failure:
//! the tab shows the address on its own.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{LazyLock, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use conductor_tools::scribe::{self, Channel};
use conductor_tools::threads;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::reverse;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::reverse;

#[cfg(not(any(target_os = "linux", windows)))]
mod other;
#[cfg(not(any(target_os = "linux", windows)))]
use other::reverse;

/// The most addresses the cache holds.  Past this, new addresses aren't
/// looked up until the next START SERVER.  A flood from thousands of
/// addresses would otherwise be thousands of resolver calls.
const MOST_CACHED: usize = 4096;

/// How long `stop()` waits for the thread.  A lookup under way can take
/// longer; the thread ends on its own once it comes back.
const STOP_WAIT: Duration = Duration::from_secs(1);

/// What the cache knows about an address.
enum Lookup {
    /// On the queue, or being looked up right now.
    Asked,
    /// Looked up: its name, or `None` for an address without one.
    Known(Option<String>),
}

/// The running lookup thread and the way to feed it.
struct DnsSide {
    asks: Sender<IpAddr>,
    worker: JoinHandle<()>,
}

// Rust note: `None` means the thread isn't running, the same shape the
// TCP and UDP sides use.
static DNS: Mutex<Option<DnsSide>> = Mutex::new(None);

static NAMES: LazyLock<Mutex<HashMap<IpAddr, Lookup>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn names() -> std::sync::MutexGuard<'static, HashMap<IpAddr, Lookup>> {
    NAMES.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Starts the lookup thread with an empty cache.  If the thread won't
/// start, the log says so and the tab goes without names; nothing else
/// minds.
pub fn start() {
    let mut guard = DNS.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.is_some() {
        return;
    }
    names().clear();

    let (asks, queue) = mpsc::channel::<IpAddr>();
    match threads::spawn("net-dns", move || work(queue)) {
        Ok(worker) => *guard = Some(DnsSide { asks, worker }),
        Err(e) => scribe::warn(Channel::Network, &format!("Couldn't start the reverse DNS thread: {e}.  The TCP \
            tab shows addresses without names until the next START SERVER.")),
    }
}

/// Stops the thread and waits for it, briefly.  Safe to call when it
/// isn't running.
pub fn stop() {
    let side = {
        let mut guard = DNS.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.take()
    };
    let Some(side) = side else {
        return;
    };

    // Dropping the sending end is what ends the thread: its recv() comes
    // back with nothing more coming.
    drop(side.asks);
    let started = Instant::now();
    while !side.worker.is_finished() && started.elapsed() < STOP_WAIT {
        thread::sleep(Duration::from_millis(10));
    }
    if side.worker.is_finished() {
        let _ = side.worker.join();
    } else {
        scribe::debug(Channel::Network, "The reverse DNS thread is still in a lookup.  It ends when that does.");
    }
}

/// Asks for an address's name, once.  Comes straight back; the answer
/// turns up in `name_of()` later.
pub fn ask(address: IpAddr) {
    let asks = {
        let guard = DNS.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.as_ref() {
            Some(side) => side.asks.clone(),
            None => return,
        }
    };
    {
        let mut names = names();
        if names.contains_key(&address) || names.len() >= MOST_CACHED {
            return;
        }
        names.insert(address, Lookup::Asked);
    }
    // The thread is gone (a stop() between the two locks).  The entry
    // would say "asked" forever, so it goes.
    if asks.send(address).is_err() {
        names().remove(&address);
    }
}

/// The address's name, if the lookup is back and it has one.  Never
/// waits.
pub fn name_of(address: IpAddr) -> Option<String> {
    match names().get(&address) {
        Some(Lookup::Known(name)) => name.clone(),
        _ => None,
    }
}

/// The lookup thread: one address at a time, until the queue closes.
fn work(queue: Receiver<IpAddr>) {
    while let Ok(address) = queue.recv() {
        let name = reverse(address);
        match &name {
            Some(name) => scribe::debug(Channel::Network, &format!("Reverse DNS: {address} is {name}.")),
            None => scribe::debug(Channel::Network, &format!("Reverse DNS: {address} has no name.")),
        }
        names().insert(address, Lookup::Known(name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_nothing_started_an_ask_is_dropped_and_no_name_is_known() {
        let address: IpAddr = "203.0.113.9".parse().unwrap();
        ask(address);
        assert_eq!(name_of(address), None);
        assert!(!names().contains_key(&address));
        stop();
    }

    #[test]
    fn a_known_name_comes_back_and_an_asked_one_doesnt_yet() {
        let named: IpAddr = "203.0.113.10".parse().unwrap();
        let asked: IpAddr = "203.0.113.11".parse().unwrap();
        {
            let mut names = names();
            names.insert(named, Lookup::Known(Some("desk.lan".to_string())));
            names.insert(asked, Lookup::Asked);
        }
        assert_eq!(name_of(named), Some("desk.lan".to_string()));
        assert_eq!(name_of(asked), None);
        {
            let mut names = names();
            names.remove(&named);
            names.remove(&asked);
        }
    }

    /// The loopback address is the one name every machine knows.
    #[test]
    fn the_loopback_address_has_a_name_or_the_os_says_it_doesnt() {
        // Either answer is fine; what matters is that the call comes back
        // and doesn't hand over junk.
        if let Some(name) = reverse("127.0.0.1".parse().unwrap()) {
            assert!(!name.is_empty());
            assert!(name.chars().all(|c| !c.is_control()));
        }
    }
}
