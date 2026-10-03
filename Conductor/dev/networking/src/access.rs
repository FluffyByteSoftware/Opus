//! File:       Opus/Conductor/dev/networking/src/access.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The whitelist and the blacklist: which addresses may reach the login
//! door.  Two files, `Content/cfg/whitelist.cfg` and `blacklist.cfg`
//! (`whitelist_file` and `blacklist_file` in `networking.cfg` say where,
//! like the TLS files), one address or range a line, and a switch in
//! `networking.cfg` (`access_list`) that says which one is looked at, if
//! either.  They're `.cfg` files by Jacob's call, but not Constellations'
//! kind: a list that grows from the page doesn't fit `key = value`, so
//! they don't show on the Settings tab, and they have tabs of their own.
//! With `whitelist` on, only a listed address gets past the acceptor;
//! with `blacklist` on, a listed address is closed at the door; `off`
//! looks at nobody.  Jacob's ask, 2026-09-29, the session after the TCP
//! tab.
//!
//! The switch is read on START SERVER like the rest of networking.cfg,
//! and so are the files: every START SERVER builds both lists in memory
//! again from what's on disk.  A change from the web admin's Whitelist
//! or Blacklist tab (or the Connections tab's menu) takes at once, on
//! the next connection, and the file is written back the same moment,
//! so the file on disk always says what's running.  That's the first
//! thing in Conductor that changes without a reboot; a ban that waited
//! for a STOP SERVER wouldn't be much of a ban.  While the server is
//! stopped there's nothing loaded to change, so the page can't; the
//! files can be edited by hand then, and the next START SERVER reads
//! them.  The next change from the page rewrites the file, comments
//! included, so notes belong in the log, not the file.  Jacob's rules,
//! 2026-09-29.
//!
//! An entry is one address (`1.2.3.4`, `::1`) or a range in the usual
//! slash form (`1.2.3.0/24`, `2001:db8::/32`), since a ban on one home
//! address is easy to step around.  A range's host bits are cleared on
//! the way in, so `1.2.3.4/24` is kept as `1.2.3.0/24`.  A check is one
//! lock and a walk down a short list: the acceptor pays it once per
//! connection, never per byte.
//!
//! The work is done on a `Lists` handed in, so the tests run on lists of
//! their own and never touch the real one or the disk.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use conductor_tools::diskman;
use conductor_tools::scribe::{self, Channel};

/// What `access_list` in networking.cfg says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Nobody is checked.
    Off,
    /// Only an address on the whitelist gets in.
    Whitelist,
    /// An address on the blacklist is closed at the door.
    Blacklist,
}

impl Mode {
    /// The setting's text, any case.  `None` for anything else.
    pub fn parse(text: &str) -> Option<Mode> {
        match text.trim().to_ascii_lowercase().as_str() {
            "off" => Some(Mode::Off),
            "whitelist" => Some(Mode::Whitelist),
            "blacklist" => Some(Mode::Blacklist),
            _ => None,
        }
    }

    /// As it's written in the file and in the JSON.
    pub fn word(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Whitelist => "whitelist",
            Mode::Blacklist => "blacklist",
        }
    }
}

/// Which of the two lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    Whitelist,
    Blacklist,
}

impl List {
    /// `whitelist` or `blacklist`, any case; the web admin's routes take
    /// it in the query.
    pub fn parse(text: &str) -> Option<List> {
        match text.trim().to_ascii_lowercase().as_str() {
            "whitelist" => Some(List::Whitelist),
            "blacklist" => Some(List::Blacklist),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            List::Whitelist => "whitelist",
            List::Blacklist => "blacklist",
        }
    }

    /// The comment at the top of the file, written whenever the file is.
    fn heading(self) -> &'static str {
        match self {
            List::Whitelist => "# The addresses let in at the login door, one a line: an address\n\
                                # (1.2.3.4) or a range (1.2.3.0/24).  \"#\" starts a comment.  Only looked\n\
                                # at while access_list in networking.cfg says whitelist; then nobody\n\
                                # else gets in, and an empty list keeps everybody out.\n\
                                #\n\
                                # The web admin's Whitelist tab writes this file on every change, and\n\
                                # takes the change at once.  A hand edit is read on the next START\n\
                                # SERVER, and the next change from the page rewrites the file, so\n\
                                # comments here don't last.\n",
            List::Blacklist => "# The addresses turned away at the login door, one a line: an address\n\
                                # (1.2.3.4) or a range (1.2.3.0/24).  \"#\" starts a comment.  Only looked\n\
                                # at while access_list in networking.cfg says blacklist.\n\
                                #\n\
                                # The web admin's Blacklist tab writes this file on every change, and\n\
                                # takes the change at once.  A hand edit is read on the next START\n\
                                # SERVER, and the next change from the page rewrites the file, so\n\
                                # comments here don't last.\n",
        }
    }
}

/// One line of a list: an address, or a range of them.  A single address
/// is a range with every bit fixed (a prefix of 32, or 128 for IPv6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// With the host bits cleared, so two spellings of one range compare
    /// equal.
    address: IpAddr,
    prefix: u8,
}

impl Entry {
    /// `1.2.3.4`, `1.2.3.0/24`, `::1` or `2001:db8::/32`, with any
    /// spaces around it.  The `Err` says what's wrong with it, in words
    /// for the page.
    pub fn parse(text: &str) -> Result<Entry, String> {
        let text = text.trim();
        let (address_text, prefix_text) = match text.split_once('/') {
            Some((address, prefix)) => (address.trim(), Some(prefix.trim())),
            None => (text, None),
        };
        let address: IpAddr = address_text.parse().map_err(|_| {
            format!("\"{text}\" isn't an address or a range.  An address is 1.2.3.4 or an IPv6 one; a \
                     range is 1.2.3.0/24.")
        })?;
        let longest = match address {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        let prefix = match prefix_text {
            None => longest,
            Some(prefix) => match prefix.parse::<u8>() {
                Ok(prefix) if prefix <= longest => prefix,
                _ => return Err(format!("\"{text}\": the part after the slash has to be 0 to {longest}.")),
            },
        };
        Ok(Entry { address: masked(address, prefix), prefix })
    }

    /// Whether `address` is this address, or inside this range.  An IPv4
    /// address that arrived wrapped in IPv6 (`::ffff:1.2.3.4`, which is
    /// what a listener on `::` sees) is checked as the IPv4 one too.
    pub fn contains(&self, address: IpAddr) -> bool {
        if masked(address, self.prefix) == self.address && same_family(address, self.address) {
            return true;
        }
        match address {
            IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
                Some(v4) => self.contains(IpAddr::V4(v4)),
                None => false,
            },
            IpAddr::V4(_) => false,
        }
    }

    fn is_single(&self) -> bool {
        match self.address {
            IpAddr::V4(_) => self.prefix == 32,
            IpAddr::V6(_) => self.prefix == 128,
        }
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_single() {
            write!(f, "{}", self.address)
        } else {
            write!(f, "{}/{}", self.address, self.prefix)
        }
    }
}

fn same_family(one: IpAddr, other: IpAddr) -> bool {
    one.is_ipv4() == other.is_ipv4()
}

/// The address with everything past the first `prefix` bits cleared.
fn masked(address: IpAddr, prefix: u8) -> IpAddr {
    match address {
        IpAddr::V4(v4) => {
            let bits = u32::from(v4);
            let kept = if prefix == 0 { 0 } else { bits & (u32::MAX << (32 - u32::from(prefix.min(32)))) };
            IpAddr::V4(Ipv4Addr::from(kept))
        }
        IpAddr::V6(v6) => {
            let bits = u128::from(v6);
            let kept = if prefix == 0 { 0 } else { bits & (u128::MAX << (128 - u32::from(prefix.min(128)))) };
            IpAddr::V6(Ipv6Addr::from(kept))
        }
    }
}

/// What the door decides about an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allowed,
    /// The blacklist is on and the address is on it.
    Blacklisted,
    /// The whitelist is on and the address isn't on it.
    NotWhitelisted,
}

/// Both lists as the web admin sees them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub mode: Mode,
    /// Each entry as it's written in its file, in the file's order.
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
}

/// The lists as loaded.
struct Lists {
    /// True between `start()` and `stop()`.  Nothing can be changed
    /// while it's false: there's no running server to change it on.
    loaded: bool,
    mode: Mode,
    whitelist: Vec<Entry>,
    blacklist: Vec<Entry>,
    /// Where each list's file is, from networking.cfg at the start.
    whitelist_path: PathBuf,
    blacklist_path: PathBuf,
}

impl Lists {
    fn new() -> Lists {
        Lists { loaded: false, mode: Mode::Off, whitelist: Vec::new(), blacklist: Vec::new(),
                whitelist_path: PathBuf::new(), blacklist_path: PathBuf::new() }
    }

    fn path(&self, which: List) -> &Path {
        match which {
            List::Whitelist => &self.whitelist_path,
            List::Blacklist => &self.blacklist_path,
        }
    }

    fn list(&self, which: List) -> &Vec<Entry> {
        match which {
            List::Whitelist => &self.whitelist,
            List::Blacklist => &self.blacklist,
        }
    }

    fn list_mut(&mut self, which: List) -> &mut Vec<Entry> {
        match which {
            List::Whitelist => &mut self.whitelist,
            List::Blacklist => &mut self.blacklist,
        }
    }
}

static LISTS: LazyLock<Mutex<Lists>> = LazyLock::new(|| Mutex::new(Lists::new()));

fn lists() -> std::sync::MutexGuard<'static, Lists> {
    LISTS.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------
// Start and stop
// ---------------------------------------------------------------------------

/// Reads both files (`whitelist_file` and `blacklist_file` in
/// networking.cfg, as full paths), or writes an empty one where there
/// isn't one, and switches the check to `mode`.  Called on every START
/// SERVER before the TCP side listens, so the lists in memory are always
/// built from what's on disk at the start.  A file that can't be read is
/// a Warn and an empty list; a line that isn't an entry is a Warn and
/// skipped; nothing here stops the server.
pub fn start(mode: Mode, whitelist_path: &Path, blacklist_path: &Path) {
    let whitelist = load(List::Whitelist, whitelist_path);
    let blacklist = load(List::Blacklist, blacklist_path);
    {
        let mut lists = lists();
        lists.loaded = true;
        lists.mode = mode;
        lists.whitelist = whitelist;
        lists.blacklist = blacklist;
        lists.whitelist_path = whitelist_path.to_path_buf();
        lists.blacklist_path = blacklist_path.to_path_buf();
    }
    let (whitelisted, blacklisted) = counts();
    match mode {
        Mode::Off => scribe::debug(Channel::Network, &format!("Access lists: off.  {whitelisted} on the \
            whitelist and {blacklisted} on the blacklist, neither looked at.")),
        Mode::Whitelist if whitelisted == 0 => scribe::warn(Channel::Network, "access_list is whitelist and \
            the whitelist is empty: NOBODY CAN LOG IN.  Add an address on the web admin's Whitelist tab, or set \
            access_list = off in networking.cfg."),
        Mode::Whitelist => scribe::info(Channel::Network, &format!("Access lists: whitelist, {whitelisted} \
            entr{} let in.", if whitelisted == 1 { "y" } else { "ies" })),
        Mode::Blacklist => scribe::info(Channel::Network, &format!("Access lists: blacklist, {blacklisted} \
            entr{} turned away.", if blacklisted == 1 { "y" } else { "ies" })),
    }
}

/// Forgets the lists on STOP SERVER.  The files stay as they are and
/// are read again on the next start.
pub fn stop() {
    *lists() = Lists::new();
}

/// One file's entries.  A missing file is written with its heading and
/// no entries, so it's there to edit by hand.
fn load(list: List, path: &Path) -> Vec<Entry> {
    match diskman::read(path).wait().and_then(|bytes| diskman::as_text(&bytes)) {
        Ok(contents) => {
            let (entries, problems) = parse(&contents);
            for problem in problems {
                scribe::warn(Channel::Network, &format!("{}, {problem}", path.display()));
            }
            scribe::debug(Channel::Network, &format!("Read {} entr{} from {}.", entries.len(),
                                                     if entries.len() == 1 { "y" } else { "ies" }, path.display()));
            entries
        }
        Err(e) if e.is_not_found() => {
            if let Err(e) = diskman::write(path, write_out(list, &[]).into_bytes()).wait() {
                scribe::warn(Channel::Network, &format!("Couldn't write an empty {}: {e}.  Going on without \
                    it; the list is empty until it's there.", path.display()));
            } else {
                scribe::debug(Channel::Network, &format!("Wrote an empty {}.", path.display()));
            }
            Vec::new()
        }
        Err(e) => {
            scribe::warn(Channel::Network, &format!("Couldn't read {}: {e}.  The {} is empty until it can be.",
                                                    path.display(), list.word()));
            Vec::new()
        }
    }
}

/// The entries in a file's text, and a complaint for each line that
/// isn't one.  Blank lines and `#` comments are skipped, and so is a
/// comment after an entry.  An entry written twice is kept once.
fn parse(contents: &str) -> (Vec<Entry>, Vec<String>) {
    let mut entries: Vec<Entry> = Vec::new();
    let mut problems = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        match Entry::parse(line) {
            Ok(entry) => {
                if !entries.contains(&entry) {
                    entries.push(entry);
                }
            }
            Err(why) => problems.push(format!("line {}: {why}  Skipped.", index + 1)),
        }
    }
    (entries, problems)
}

/// The file as it's written: the heading, then one entry a line.
fn write_out(list: List, entries: &[Entry]) -> String {
    let mut text = String::from(list.heading());
    text.push('\n');
    for entry in entries {
        text.push_str(&entry.to_string());
        text.push('\n');
    }
    text
}

/// Writes `list` back to its file, from the entries handed in.  Not
/// waited on: the change took in memory already, and DiskMan says so in
/// the log (and on the bell) if the file can't be written.
fn save(list: List, path: &Path, entries: &[Entry]) {
    let _ = diskman::write(path, write_out(list, entries).into_bytes());
}

// ---------------------------------------------------------------------------
// The check and the changes
// ---------------------------------------------------------------------------

/// What the door does with `address`, by the mode.
pub fn verdict(address: IpAddr) -> Verdict {
    verdict_in(&lists(), address)
}

/// The mode the server started with.
pub fn mode() -> Mode {
    lists().mode
}

/// How many entries each list has, for the status.
pub fn counts() -> (usize, usize) {
    let lists = lists();
    (lists.whitelist.len(), lists.blacklist.len())
}

/// Both lists, for the Whitelist and Blacklist tabs.  `None` while the server isn't
/// running, since the lists only load with it.
pub fn snapshot() -> Option<Snapshot> {
    let lists = lists();
    if !lists.loaded {
        return None;
    }
    Some(Snapshot {
        mode: lists.mode,
        whitelist: lists.whitelist.iter().map(Entry::to_string).collect(),
        blacklist: lists.blacklist.iter().map(Entry::to_string).collect(),
    })
}

/// Puts `entry` on `list`, in memory now and on disk right behind.  True
/// if it wasn't there already.  An `Err` while the server isn't running.
pub fn add(list: List, entry: Entry) -> Result<bool, String> {
    let (path, entries) = {
        let mut lists = lists();
        if !lists.loaded {
            return Err("Networking isn't running, so there are no lists to change.".to_string());
        }
        if !add_in(&mut lists, list, entry) {
            return Ok(false);
        }
        (lists.path(list).to_path_buf(), lists.list(list).clone())
    };
    save(list, &path, &entries);
    scribe::info(Channel::Network, &format!("The admin added {entry} to the {}.", list.word()));
    Ok(true)
}

/// Takes `entry` off `list`, the same way.  True if it was there.
pub fn remove(list: List, entry: Entry) -> Result<bool, String> {
    let (path, entries) = {
        let mut lists = lists();
        if !lists.loaded {
            return Err("Networking isn't running, so there are no lists to change.".to_string());
        }
        if !remove_in(&mut lists, list, entry) {
            return Ok(false);
        }
        (lists.path(list).to_path_buf(), lists.list(list).clone())
    };
    save(list, &path, &entries);
    scribe::info(Channel::Network, &format!("The admin took {entry} off the {}.", list.word()));
    Ok(true)
}

// ---------------------------------------------------------------------------
// The work, on whatever lists are handed in
// ---------------------------------------------------------------------------

fn verdict_in(lists: &Lists, address: IpAddr) -> Verdict {
    match lists.mode {
        Mode::Off => Verdict::Allowed,
        Mode::Whitelist => {
            if lists.whitelist.iter().any(|entry| entry.contains(address)) {
                Verdict::Allowed
            } else {
                Verdict::NotWhitelisted
            }
        }
        Mode::Blacklist => {
            if lists.blacklist.iter().any(|entry| entry.contains(address)) {
                Verdict::Blacklisted
            } else {
                Verdict::Allowed
            }
        }
    }
}

fn add_in(lists: &mut Lists, list: List, entry: Entry) -> bool {
    let entries = lists.list_mut(list);
    if entries.contains(&entry) {
        return false;
    }
    entries.push(entry);
    true
}

fn remove_in(lists: &mut Lists, list: List, entry: Entry) -> bool {
    let entries = lists.list_mut(list);
    let before = entries.len();
    entries.retain(|kept| *kept != entry);
    entries.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(text: &str) -> Entry {
        Entry::parse(text).unwrap_or_else(|why| panic!("{text}: {why}"))
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn an_entry_is_an_address_or_a_range() {
        assert_eq!(entry("1.2.3.4").to_string(), "1.2.3.4");
        assert_eq!(entry(" 1.2.3.4 ").to_string(), "1.2.3.4");
        assert_eq!(entry("1.2.3.4/32").to_string(), "1.2.3.4");
        assert_eq!(entry("1.2.3.0/24").to_string(), "1.2.3.0/24");
        // The host bits go, so two spellings of one range are one entry.
        assert_eq!(entry("1.2.3.4/24").to_string(), "1.2.3.0/24");
        assert_eq!(entry("1.2.3.4/24"), entry("1.2.3.0/24"));
        assert_eq!(entry("10.0.0.0/8").to_string(), "10.0.0.0/8");
        assert_eq!(entry("0.0.0.0/0").to_string(), "0.0.0.0/0");
        assert_eq!(entry("::1").to_string(), "::1");
        assert_eq!(entry("2001:db8::1/32").to_string(), "2001:db8::/32");
        assert_eq!(entry("::1/128").to_string(), "::1");
    }

    #[test]
    fn what_isnt_an_entry_says_why() {
        for bad in ["", "potato", "1.2.3", "1.2.3.4.5", "1.2.3.4/33", "1.2.3.4/-1", "1.2.3.4/x", "::1/129",
                    "1.2.3.4/24/8", "1.2.3.4 5.6.7.8"] {
            let why = Entry::parse(bad).expect_err(bad);
            assert!(why.contains(bad.trim()), "{bad}: {why}");
        }
    }

    #[test]
    fn a_range_holds_its_addresses_and_no_others() {
        let home = entry("1.2.3.4");
        assert!(home.contains(ip("1.2.3.4")));
        assert!(!home.contains(ip("1.2.3.5")));
        assert!(!home.contains(ip("::1")));

        let street = entry("1.2.3.0/24");
        assert!(street.contains(ip("1.2.3.0")));
        assert!(street.contains(ip("1.2.3.4")));
        assert!(street.contains(ip("1.2.3.255")));
        assert!(!street.contains(ip("1.2.4.0")));
        assert!(!street.contains(ip("1.2.2.255")));

        let everyone = entry("0.0.0.0/0");
        assert!(everyone.contains(ip("8.8.8.8")));
        // Every IPv4 address, not every address.
        assert!(!everyone.contains(ip("2001:db8::1")));

        let block = entry("2001:db8::/32");
        assert!(block.contains(ip("2001:db8::1")));
        assert!(block.contains(ip("2001:db8:ffff::")));
        assert!(!block.contains(ip("2001:db9::")));
        assert!(!block.contains(ip("1.2.3.4")));
    }

    #[test]
    fn an_ipv4_address_wrapped_in_ipv6_is_checked_as_ipv4() {
        // A listener on :: sees 1.2.3.4 as ::ffff:1.2.3.4.
        assert!(entry("1.2.3.4").contains(ip("::ffff:1.2.3.4")));
        assert!(entry("1.2.3.0/24").contains(ip("::ffff:1.2.3.9")));
        assert!(!entry("1.2.3.4").contains(ip("::ffff:1.2.3.5")));
    }

    #[test]
    fn the_mode_and_the_list_names_read_any_case() {
        assert_eq!(Mode::parse("off"), Some(Mode::Off));
        assert_eq!(Mode::parse(" Whitelist "), Some(Mode::Whitelist));
        assert_eq!(Mode::parse("BLACKLIST"), Some(Mode::Blacklist));
        assert_eq!(Mode::parse("on"), None);
        assert_eq!(Mode::parse(""), None);
        assert_eq!(List::parse("whitelist"), Some(List::Whitelist));
        assert_eq!(List::parse("Blacklist"), Some(List::Blacklist));
        assert_eq!(List::parse("greylist"), None);
        for mode in [Mode::Off, Mode::Whitelist, Mode::Blacklist] {
            assert_eq!(Mode::parse(mode.word()), Some(mode));
        }
    }

    #[test]
    fn a_file_is_entries_comments_and_complaints() {
        let text = "# heading\n\n1.2.3.4\n1.2.3.0/24  # the street\npotato\n1.2.3.4\n  ::1\n1.2.3.4/40\n";
        let (entries, problems) = parse(text);
        assert_eq!(entries, vec![entry("1.2.3.4"), entry("1.2.3.0/24"), entry("::1")]);
        assert_eq!(problems.len(), 2);
        assert!(problems[0].starts_with("line 5: \"potato\" isn't an address"), "{}", problems[0]);
        assert!(problems[1].starts_with("line 8: \"1.2.3.4/40\": the part after the slash"), "{}", problems[1]);
        assert!(problems[1].ends_with("Skipped."));

        let (entries, problems) = parse("");
        assert!(entries.is_empty());
        assert!(problems.is_empty());
    }

    #[test]
    fn a_file_written_out_reads_back_the_same() {
        let entries = vec![entry("1.2.3.4"), entry("10.0.0.0/8"), entry("2001:db8::/32")];
        let text = write_out(List::Blacklist, &entries);
        assert!(text.starts_with("# The addresses turned away"));
        assert!(text.ends_with("\n1.2.3.4\n10.0.0.0/8\n2001:db8::/32\n"));
        let (read_back, problems) = parse(&text);
        assert_eq!(read_back, entries);
        assert!(problems.is_empty());
        assert!(write_out(List::Whitelist, &[]).starts_with("# The addresses let in"));
    }

    #[test]
    fn the_verdict_follows_the_mode() {
        let mut lists = Lists::new();
        lists.whitelist = vec![entry("10.0.0.0/8")];
        lists.blacklist = vec![entry("1.2.3.4")];

        lists.mode = Mode::Off;
        assert_eq!(verdict_in(&lists, ip("1.2.3.4")), Verdict::Allowed);
        assert_eq!(verdict_in(&lists, ip("8.8.8.8")), Verdict::Allowed);

        lists.mode = Mode::Whitelist;
        assert_eq!(verdict_in(&lists, ip("10.1.2.3")), Verdict::Allowed);
        assert_eq!(verdict_in(&lists, ip("8.8.8.8")), Verdict::NotWhitelisted);
        // The blacklist isn't looked at in whitelist mode.
        assert_eq!(verdict_in(&lists, ip("1.2.3.4")), Verdict::NotWhitelisted);

        lists.mode = Mode::Blacklist;
        assert_eq!(verdict_in(&lists, ip("1.2.3.4")), Verdict::Blacklisted);
        assert_eq!(verdict_in(&lists, ip("8.8.8.8")), Verdict::Allowed);
        assert_eq!(verdict_in(&lists, ip("10.1.2.3")), Verdict::Allowed);

        // An empty whitelist lets nobody in.
        lists.mode = Mode::Whitelist;
        lists.whitelist.clear();
        assert_eq!(verdict_in(&lists, ip("10.1.2.3")), Verdict::NotWhitelisted);
    }

    #[test]
    fn adding_and_removing_say_whether_anything_changed() {
        let mut lists = Lists::new();
        assert!(add_in(&mut lists, List::Blacklist, entry("1.2.3.4")));
        assert!(!add_in(&mut lists, List::Blacklist, entry("1.2.3.4")));
        assert!(!add_in(&mut lists, List::Blacklist, entry("1.2.3.4/32")));
        assert!(add_in(&mut lists, List::Whitelist, entry("1.2.3.4")));
        assert_eq!(lists.blacklist, vec![entry("1.2.3.4")]);
        assert_eq!(lists.whitelist, vec![entry("1.2.3.4")]);

        assert!(remove_in(&mut lists, List::Blacklist, entry("1.2.3.4")));
        assert!(!remove_in(&mut lists, List::Blacklist, entry("1.2.3.4")));
        assert!(lists.blacklist.is_empty());
        assert_eq!(lists.whitelist, vec![entry("1.2.3.4")]);
    }

    #[test]
    fn nothing_can_change_while_the_server_is_stopped() {
        // The real lists, unloaded: the tests never call start().
        assert_eq!(snapshot(), None);
        assert!(add(List::Blacklist, entry("1.2.3.4")).is_err());
        assert!(remove(List::Blacklist, entry("1.2.3.4")).is_err());
        assert_eq!(verdict(ip("1.2.3.4")), Verdict::Allowed);
        assert_eq!(counts(), (0, 0));
    }
}
