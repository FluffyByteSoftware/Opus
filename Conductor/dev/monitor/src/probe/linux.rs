//! File:       Opus/Conductor/dev/monitor/src/probe/linux.rs
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
//! - `/proc/stat`: the whole machine's time, one `cpuN` line per core.
//! - `/proc/meminfo`: the whole machine's RAM, total and available.
//! - `/proc/<pid>/stat`: the same as the first, for every process on the
//!   machine, with its name, thread count and memory.  Linux lets anyone
//!   read these, whoever's process it is.  Kernel threads (`kworker` and
//!   friends) are left out: they're the kernel's own, not programs.
//! - `/proc/<pid>/task/<id>/stat`: one other process's threads, when the
//!   admin asks for them.
//!
//! Written with Nobara and Fedora in mind, but any Linux kernel from the
//! last ten years lays these files out the same way.

use std::ffi::{c_int, c_long};
use std::fs;
use std::path::Path;
use std::time::Duration;

use super::{CoreTimes, DiskTotals, MachineMemory, ProcessReading, Reading, ThreadReading};

// Rust note: `sysconf` is a C function every Linux program already has
// linked in.  Calling one is normally `unsafe`, since Rust can't check what
// the other side does.  `safe fn` is us promising this one can't go wrong:
// it takes a number and hands back a number.
unsafe extern "C" {
    safe fn sysconf(name: c_int) -> c_long;
}

/// The names `sysconf` knows "ticks per second" and "bytes in a page of
/// memory" by, on Linux.
const SC_CLK_TCK: c_int = 2;
const SC_PAGESIZE: c_int = 30;

/// The flag in a `stat` file that marks a kernel thread.
const PF_KTHREAD: u64 = 0x0020_0000;

/// Reads the process.  `None` only if `/proc/self/stat` or `status` can't
/// be read, which would mean `/proc` isn't there at all.
pub fn read() -> Option<Reading> {
    let ticks = ticks_per_second();
    let cpu_time = cpu_from_stat(&fs::read_to_string("/proc/self/stat").ok()?, ticks)?;
    let memory_bytes = rss_from_status(&fs::read_to_string("/proc/self/status").ok()?)?;
    // Some kernels are built without the disk counts, so a missing file
    // just means no disk numbers.
    let disk = fs::read_to_string("/proc/self/io").ok().and_then(|text| disk_from_io(&text));

    let cores = fs::read_to_string("/proc/stat").map(|text| cores_from_stat(&text)).unwrap_or_default();
    let machine_memory = fs::read_to_string("/proc/meminfo").ok().and_then(|text| machine_from_meminfo(&text));

    Some(Reading {
        cpu_time,
        memory_bytes,
        disk,
        threads: threads_in(Path::new("/proc/self/task"), ticks).unwrap_or_default(),
        cores,
        machine_memory,
        processes: read_processes(ticks),
    })
}

/// The threads of process `pid`.  `None` if it's gone, or we aren't
/// allowed to look.
pub fn threads_of(pid: u32) -> Option<Vec<ThreadReading>> {
    threads_in(&Path::new("/proc").join(pid.to_string()).join("task"), ticks_per_second())
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

/// Every thread in a `task` folder.  A thread that ends between listing
/// the folder and reading its file is skipped.  `None` if the folder can't
/// be read at all.
fn threads_in(task_dir: &Path, ticks: u64) -> Option<Vec<ThreadReading>> {
    let entries = fs::read_dir(task_dir).ok()?;

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
    Some(threads)
}

/// Every process in `/proc` except the kernel's own threads.  One that
/// ends between listing the folder and reading its file is skipped.
fn read_processes(ticks: u64) -> Vec<ProcessReading> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };

    let page_size = page_size();
    let mut processes = Vec::new();
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        if let Some(process) = process_from_stat(pid, &stat, ticks, page_size) {
            processes.push(process);
        }
    }
    processes
}

/// A process from its `stat` file: the name, CPU time, thread count (field
/// 20) and memory in RAM (field 24, counted in pages).  `None` for a
/// kernel thread, which has the kernel-thread flag in field 9.
fn process_from_stat(pid: u32, stat: &str, ticks: u64, page_size: u64) -> Option<ProcessReading> {
    let (_, after_name) = stat.rsplit_once(')')?;
    let fields: Vec<&str> = after_name.split_whitespace().collect();
    let field = |number: usize| -> Option<u64> { fields.get(number - 3)?.parse().ok() };

    if field(9)? & PF_KTHREAD != 0 {
        return None;
    }
    Some(ProcessReading {
        pid,
        name: name_from_stat(stat).unwrap_or_else(|| "(no name)".to_string()),
        cpu_time: cpu_from_stat(stat, ticks),
        memory_bytes: field(24).map(|pages| pages * page_size),
        threads: field(20).map(|count| count as u32),
    })
}

fn page_size() -> u64 {
    match sysconf(SC_PAGESIZE) {
        bytes if bytes > 0 => bytes as u64,
        // 4 KB on every PC Linux runs on.
        _ => 4096,
    }
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

/// One `CoreTimes` per `cpuN` line in `/proc/stat` (the plain `cpu` line is
/// all of them added up, and gets skipped).  The numbers after the name are
/// ticks spent on user, nice, system, idle, iowait, irq, softirq and steal,
/// and idle plus iowait is the time it wasn't doing anything.
fn cores_from_stat(stat: &str) -> Vec<CoreTimes> {
    stat.lines()
        .filter(|line| line.starts_with("cpu") && line.as_bytes().get(3).is_some_and(u8::is_ascii_digit))
        .filter_map(|line| {
            let numbers: Vec<u64> = line.split_whitespace()
                .skip(1)
                .take(8)
                .map(|field| field.parse().ok())
                .collect::<Option<Vec<u64>>>()?;
            let idle = numbers.get(3)? + numbers.get(4).copied().unwrap_or(0);
            let total: u64 = numbers.iter().sum();
            Some(CoreTimes { busy: total.saturating_sub(idle), total })
        })
        .collect()
}

/// `MemTotal` and `MemAvailable` from `/proc/meminfo`, in bytes.
fn machine_from_meminfo(meminfo: &str) -> Option<MachineMemory> {
    let kb = |key: &str| -> Option<u64> {
        let line = meminfo.lines().find(|line| line.starts_with(key))?;
        line.split_whitespace().nth(1)?.parse().ok()
    };
    Some(MachineMemory { total_bytes: kb("MemTotal:")? * 1024, available_bytes: kb("MemAvailable:")? * 1024 })
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
    fn a_process_comes_from_its_stat_file() {
        let process = process_from_stat(4105, STAT, 100, 4096).expect("that's a process, not a kernel thread");
        assert_eq!(process.pid, 4105);
        assert_eq!(process.name, "arch vist)");
        assert_eq!(process.cpu_time, Some(Duration::from_millis(2_800)));
        assert_eq!(process.threads, Some(5));
        assert_eq!(process.memory_bytes, Some(90 * 4096));
    }

    #[test]
    fn kernel_threads_are_left_out() {
        let kworker = "57 (kworker/3:1) I 2 0 0 0 -1 69238880 0 0 0 0 0 12 0 0 20 0 1 0 150 0 0 \
            18446744073709551615 0 0 0 0 0 0 0 2147483647 0 0 0 0 17 3 0 0 0 0 0";
        assert!(process_from_stat(57, kworker, 100, 4096).is_none());
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
    fn each_core_gets_its_own_line() {
        let stat = "cpu  400 0 100 1500 0 0 0 0 0 0\n\
            cpu0 100 0 50 800 50 0 0 0 0 0\n\
            cpu1 300 0 50 700 0 0 0 0 0 0\n\
            intr 12345\n";
        let cores = cores_from_stat(stat);
        assert_eq!(cores.len(), 2);
        assert_eq!((cores[0].busy, cores[0].total), (150, 1000));
        assert_eq!((cores[1].busy, cores[1].total), (350, 1050));
    }

    #[test]
    fn machine_memory_comes_from_meminfo() {
        let meminfo = "MemTotal:       65536000 kB\nMemFree:  1000 kB\nMemAvailable:   32768000 kB\n";
        let memory = machine_from_meminfo(meminfo).expect("both numbers are there");
        assert_eq!(memory.total_bytes, 65_536_000 * 1024);
        assert_eq!(memory.available_bytes, 32_768_000 * 1024);
    }

    #[test]
    fn this_process_can_be_read() {
        let reading = read().expect("/proc should be there on Linux");
        assert!(reading.memory_bytes > 0);
        assert!(!reading.threads.is_empty());
        assert!(!reading.cores.is_empty());
        assert!(reading.machine_memory.is_some());
        assert!(reading.processes.iter().any(|process| process.pid == std::process::id()));
    }

    #[test]
    fn another_process_can_be_asked_for_its_threads() {
        let threads = threads_of(std::process::id()).expect("our own process is always there");
        assert!(!threads.is_empty());
        assert!(threads_of(u32::MAX).is_none());
    }
}
