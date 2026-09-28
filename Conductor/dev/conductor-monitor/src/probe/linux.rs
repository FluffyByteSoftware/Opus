//! File:       Opus/Conductor/dev/conductor-monitor/src/probe/linux.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Linux keeps everything we want as text files under `/proc/self`, so
//! this is reading files and picking numbers out of them:
//!
//! - `/proc/self/stat`: CPU time, in "ticks" (usually 100 a second).
//! - `/proc/self/status`: `VmRSS`, the memory actually in RAM.
//! - `/proc/self/io`: bytes that actually went to or came from the disk.
//!   A read the OS answered from its cache doesn't count.
//! - `/proc/self/task/<id>/stat`: the same as the first, one per thread,
//!   with the thread's name.
//!
//! Written with Nobara and Fedora in mind, but any Linux kernel from the
//! last ten years lays these files out the same way.

use std::ffi::{c_int, c_long};
use std::fs;
use std::time::Duration;

use super::{DiskTotals, Reading, ThreadReading};

// Rust note: `sysconf` is a C function every Linux program already has
// linked in.  Calling one is normally `unsafe`, since Rust can't check what
// the other side does.  `safe fn` is us promising this one can't go wrong:
// it takes a number and hands back a number.
unsafe extern "C" {
    safe fn sysconf(name: c_int) -> c_long;
}

/// The name `sysconf` knows "ticks per second" by, on Linux.
const SC_CLK_TCK: c_int = 2;

/// Reads the process.  `None` only if `/proc/self/stat` or `status` can't
/// be read, which would mean `/proc` isn't there at all.
pub fn read() -> Option<Reading> {
    let ticks = ticks_per_second();
    let cpu_time = cpu_from_stat(&fs::read_to_string("/proc/self/stat").ok()?, ticks)?;
    let memory_bytes = rss_from_status(&fs::read_to_string("/proc/self/status").ok()?)?;
    // Some kernels are built without the disk counts, so a missing file
    // just means no disk numbers.
    let disk = fs::read_to_string("/proc/self/io").ok().and_then(|text| disk_from_io(&text));

    Some(Reading { cpu_time, memory_bytes, disk, threads: read_threads(ticks) })
}

/// Something like "Nobara Linux 42 (KDE Plasma), kernel 6.14.5".
pub fn os_name() -> String {
    let pretty = fs::read_to_string("/etc/os-release").ok()
        .and_then(|text| pretty_name(&text))
        .unwrap_or_else(|| "Linux".to_string());
    match fs::read_to_string("/proc/sys/kernel/osrelease") {
        Ok(kernel) => format!("{pretty}, kernel {}", kernel.trim()),
        Err(_) => pretty,
    }
}

/// Every thread in `/proc/self/task`.  A thread that ends between listing
/// the folder and reading its file is skipped.
fn read_threads(ticks: u64) -> Vec<ThreadReading> {
    let Ok(entries) = fs::read_dir("/proc/self/task") else {
        return Vec::new();
    };

    let mut threads = Vec::new();
    for entry in entries.flatten() {
        let Some(os_id) = entry.file_name().to_str().and_then(|name| name.parse::<u64>().ok()) else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        if let Some(cpu_time) = cpu_from_stat(&stat, ticks) {
            threads.push(ThreadReading { os_id, name: name_from_stat(&stat), cpu_time });
        }
    }
    threads.sort_by_key(|thread| thread.os_id);
    threads
}

fn ticks_per_second() -> u64 {
    match sysconf(SC_CLK_TCK) {
        ticks if ticks > 0 => ticks as u64,
        // It has been 100 on every Linux for decades.
        _ => 100,
    }
}

/// CPU time from a `stat` file: user time plus system time, fields 14 and
/// 15.  The name in field 2 sits in brackets and can have spaces in it,
/// so we count the fields from the last `)`.  After it, field 3 is first.
fn cpu_from_stat(stat: &str, ticks: u64) -> Option<Duration> {
    let (_, after_name) = stat.rsplit_once(')')?;
    let fields: Vec<&str> = after_name.split_whitespace().collect();
    let user: u64 = fields.get(14 - 3)?.parse().ok()?;
    let system: u64 = fields.get(15 - 3)?.parse().ok()?;
    Some(Duration::from_millis((user + system) * 1000 / ticks))
}

/// The name in brackets in a `stat` file.
fn name_from_stat(stat: &str) -> Option<String> {
    let start = stat.find('(')?;
    let end = stat.rfind(')')?;
    (end > start).then(|| stat[start + 1..end].to_string())
}

/// `VmRSS:     12345 kB` from `status`, in bytes.
fn rss_from_status(status: &str) -> Option<u64> {
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

/// `read_bytes` and `write_bytes` from `io`.
fn disk_from_io(io: &str) -> Option<DiskTotals> {
    let value = |key: &str| -> Option<u64> {
        let line = io.lines().find(|line| line.starts_with(key))?;
        line.split_once(':')?.1.trim().parse().ok()
    };
    Some(DiskTotals { read: value("read_bytes:")?, written: value("write_bytes:")? })
}

/// `PRETTY_NAME="Nobara Linux 42"` from `/etc/os-release`, without the
/// quotes.
fn pretty_name(os_release: &str) -> Option<String> {
    let line = os_release.lines().find(|line| line.starts_with("PRETTY_NAME="))?;
    let value = line.trim_start_matches("PRETTY_NAME=").trim().trim_matches('"');
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // A real line from a thread named "archivist", with a space and a ")"
    // put in the name to make sure they can't throw the count off.
    const STAT: &str = "4105 (arch vist)) S 4092 4092 4092 0 -1 4194368 120 0 0 0 250 30 0 0 20 0 5 0 \
        1234 5678 90 18446744073709551615 1 1 0 0 0 0 0 0 0 0 0 0 -1 3 0 0 0 0 0";

    #[test]
    fn cpu_time_is_user_plus_system() {
        assert_eq!(cpu_from_stat(STAT, 100), Some(Duration::from_millis(2_800)));
    }

    #[test]
    fn the_name_can_hold_brackets_and_spaces() {
        assert_eq!(name_from_stat(STAT).as_deref(), Some("arch vist)"));
    }

    #[test]
    fn memory_comes_from_vm_rss() {
        let status = "Name:\tconductor\nVmPeak:\t  900 kB\nVmRSS:\t   12345 kB\nThreads:\t5\n";
        assert_eq!(rss_from_status(status), Some(12_345 * 1024));
    }

    #[test]
    fn disk_comes_from_read_and_write_bytes() {
        let io = "rchar: 100\nwchar: 200\nsyscr: 3\nsyscw: 4\nread_bytes: 4096\nwrite_bytes: 8192\n\
            cancelled_write_bytes: 0\n";
        let disk = disk_from_io(io).expect("both numbers are there");
        assert_eq!((disk.read, disk.written), (4096, 8192));
    }

    #[test]
    fn the_os_name_loses_its_quotes() {
        let os_release = "NAME=\"Nobara Linux\"\nPRETTY_NAME=\"Nobara Linux 42 (KDE Plasma)\"\nID=nobara\n";
        assert_eq!(pretty_name(os_release).as_deref(), Some("Nobara Linux 42 (KDE Plasma)"));
    }

    #[test]
    fn this_process_can_be_read() {
        let reading = read().expect("/proc should be there on Linux");
        assert!(reading.memory_bytes > 0);
        assert!(!reading.threads.is_empty());
    }
}
