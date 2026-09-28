//! File:       Opus/Conductor/dev/conductor-monitor/src/probe/windows.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Windows doesn't keep these as files the way Linux does.  We ask
//! kernel32 for them, the same DLL every Windows program already has
//! loaded, so there is no crate to add:
//!
//! - `GetProcessTimes`: CPU time, in 100-nanosecond steps.
//! - `K32GetProcessMemoryInfo`: the working set, the memory actually in
//!   RAM.
//! - `GetProcessIoCounters`: bytes read and written.  Unlike Linux, this
//!   counts every read and write the process makes, network included, not
//!   just the disk.
//! - `CreateToolhelp32Snapshot`: a list of every thread on the machine,
//!   which we cut down to ours.  Then `GetThreadTimes` for each one.  It's
//!   a few thousand entries a second, which Windows does in well under a
//!   millisecond.
//! - `GlobalMemoryStatusEx`: the whole machine's RAM, total and available.
//!
//! Two come from ntdll, a layer under kernel32: `NtQuerySystemInformation`
//! for how busy each core has been, which kernel32 has no call for, and
//! `RtlGetVersion` for the version, because the kernel32 one lies to
//! programs that don't declare which Windows they were built for.
//! Microsoft says `NtQuerySystemInformation` may change, but it has been
//! there for over twenty years and plenty of tools lean on it.
//!
//! The structs below copy Windows' own, field for field in the same order,
//! which is what `#[repr(C)]` makes Rust keep.  Their fields are renamed in
//! Rust's style, since only the order matters.  A field with a leading `_`
//! is one we fill in or skip over but never read, and the `_` tells the
//! compiler that's on purpose.

use std::ffi::c_void;
use std::time::Duration;

use super::{CoreTimes, DiskTotals, MachineMemory, Reading, ThreadReading};

/// Windows' HANDLE: a number standing for something the OS has open.
type Handle = *mut c_void;

/// What CreateToolhelp32Snapshot hands back when it fails.
const INVALID_HANDLE: Handle = -1_isize as Handle;

/// Ask CreateToolhelp32Snapshot for threads.
const TH32CS_SNAPTHREAD: u32 = 0x0000_0004;

/// Ask OpenThread for just enough to read its times.
const THREAD_QUERY_LIMITED_INFORMATION: u32 = 0x0000_0800;

/// Ask NtQuerySystemInformation for each core's times.
const SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION: u32 = 8;

/// Room for this many cores' times.  Windows hands out 64 at most per
/// "processor group", so 256 is plenty.
const MOST_CORES: usize = 256;

/// Windows' FILETIME: a count of 100-nanosecond steps, split in two
/// halves.
#[repr(C)]
#[derive(Default)]
struct FileTime {
    low: u32,
    high: u32,
}

impl FileTime {
    fn duration(&self) -> Duration {
        let steps = (u64::from(self.high) << 32) | u64::from(self.low);
        Duration::from_nanos(steps.saturating_mul(100))
    }
}

/// Windows' PROCESS_MEMORY_COUNTERS.
#[repr(C)]
#[derive(Default)]
struct MemoryCounters {
    _size: u32,
    _page_faults: u32,
    _peak_working_set: usize,
    working_set: usize,
    _quota_peak_paged_pool: usize,
    _quota_paged_pool: usize,
    _quota_peak_non_paged_pool: usize,
    _quota_non_paged_pool: usize,
    _pagefile: usize,
    _peak_pagefile: usize,
}

/// Windows' IO_COUNTERS.
#[repr(C)]
#[derive(Default)]
struct IoCounters {
    _read_operations: u64,
    _write_operations: u64,
    _other_operations: u64,
    read_bytes: u64,
    written_bytes: u64,
    _other_bytes: u64,
}

/// Windows' THREADENTRY32.
#[repr(C)]
#[derive(Default)]
struct ThreadEntry {
    _size: u32,
    _usage: u32,
    thread_id: u32,
    owner_process_id: u32,
    _base_priority: i32,
    _delta_priority: i32,
    _flags: u32,
}

/// Windows' MEMORYSTATUSEX.
#[repr(C)]
struct MemoryStatus {
    _size: u32,
    _load: u32,
    total_physical: u64,
    available_physical: u64,
    _total_page_file: u64,
    _available_page_file: u64,
    _total_virtual: u64,
    _available_virtual: u64,
    _available_extended_virtual: u64,
}

/// Windows' SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION, in 100 ns steps.
/// Kernel time includes the idle time.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ProcessorTimes {
    idle: i64,
    kernel: i64,
    user: i64,
    _dpc: i64,
    _interrupt: i64,
    _interrupt_count: u32,
}

/// Windows' RTL_OSVERSIONINFOW.
#[repr(C)]
struct VersionInfo {
    _size: u32,
    major: u32,
    minor: u32,
    build: u32,
    _platform: u32,
    _service_pack: [u16; 128],
}

// Rust note: an `extern` block lists functions that live outside Rust.
// Calling one is `unsafe`, since Rust can't check what the other side does
// with the pointers we hand it.  The two marked `safe fn` take nothing and
// hand back a number, so there's nothing for them to get wrong.
#[link(name = "kernel32")]
unsafe extern "system" {
    safe fn GetCurrentProcess() -> Handle;
    safe fn GetCurrentProcessId() -> u32;
    fn GetProcessTimes(process: Handle,
                       creation: *mut FileTime,
                       exit: *mut FileTime,
                       kernel: *mut FileTime,
                       user: *mut FileTime) -> i32;
    fn K32GetProcessMemoryInfo(process: Handle, counters: *mut MemoryCounters, size: u32) -> i32;
    fn GetProcessIoCounters(process: Handle, counters: *mut IoCounters) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> Handle;
    fn Thread32First(snapshot: Handle, entry: *mut ThreadEntry) -> i32;
    fn Thread32Next(snapshot: Handle, entry: *mut ThreadEntry) -> i32;
    fn OpenThread(access: u32, inherit: i32, thread_id: u32) -> Handle;
    fn GetThreadTimes(thread: Handle,
                      creation: *mut FileTime,
                      exit: *mut FileTime,
                      kernel: *mut FileTime,
                      user: *mut FileTime) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
    fn GlobalMemoryStatusEx(status: *mut MemoryStatus) -> i32;
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlGetVersion(info: *mut VersionInfo) -> i32;
    fn NtQuerySystemInformation(class: u32, buffer: *mut c_void, length: u32, returned: *mut u32) -> i32;
}

// Every `unsafe` block below is the same deal: we hand Windows pointers to
// structs we own, sized the way it expects, and they stay put until it's
// done with them.  Every handle we open, we close.

/// Reads the process.  `None` if Windows won't give us the CPU time or the
/// memory, which shouldn't happen for our own process.
pub fn read() -> Option<Reading> {
    let process = GetCurrentProcess();

    let mut creation = FileTime::default();
    let mut exit = FileTime::default();
    let mut kernel = FileTime::default();
    let mut user = FileTime::default();
    let ok = unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) };
    if ok == 0 {
        return None;
    }

    let mut memory = MemoryCounters { _size: size_of::<MemoryCounters>() as u32, ..Default::default() };
    let ok = unsafe { K32GetProcessMemoryInfo(process, &mut memory, size_of::<MemoryCounters>() as u32) };
    if ok == 0 {
        return None;
    }

    let mut io = IoCounters::default();
    let disk = match unsafe { GetProcessIoCounters(process, &mut io) } {
        0 => None,
        _ => Some(DiskTotals { read: io.read_bytes, written: io.written_bytes }),
    };

    Some(Reading {
        cpu_time: kernel.duration() + user.duration(),
        memory_bytes: memory.working_set as u64,
        disk,
        threads: read_threads(),
        cores: read_cores(),
        machine_memory: read_machine_memory(),
    })
}

/// Something like "Windows 11 (10.0, build 22631)".
pub fn os_name() -> String {
    let mut info = VersionInfo {
        _size: size_of::<VersionInfo>() as u32,
        major: 0,
        minor: 0,
        build: 0,
        _platform: 0,
        _service_pack: [0; 128],
    };
    if unsafe { RtlGetVersion(&mut info) } != 0 {
        return "Windows".to_string();
    }

    // Windows 11 still calls itself 10.0.  The build number is the only
    // way to tell them apart.
    let name = match (info.major, info.build) {
        (10, build) if build >= 22_000 => "Windows 11",
        (10, _) => "Windows 10",
        _ => "Windows",
    };
    format!("{name} ({}.{}, build {})", info.major, info.minor, info.build)
}

/// Our threads, each with its CPU time.  A thread that ends or won't open
/// between the list and the asking is skipped.
fn read_threads() -> Vec<ThreadReading> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE || snapshot.is_null() {
        return Vec::new();
    }

    let our_id = GetCurrentProcessId();
    let mut threads = Vec::new();
    let mut entry = ThreadEntry { _size: size_of::<ThreadEntry>() as u32, ..Default::default() };
    let mut more = unsafe { Thread32First(snapshot, &mut entry) } != 0;
    while more {
        if entry.owner_process_id == our_id {
            if let Some(cpu_time) = thread_cpu_time(entry.thread_id) {
                threads.push(ThreadReading { os_id: u64::from(entry.thread_id), name: None, cpu_time });
            }
        }
        more = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snapshot) };

    threads.sort_by_key(|thread| thread.os_id);
    threads
}

/// Each core's busy and total time.  Empty if Windows won't say.
fn read_cores() -> Vec<CoreTimes> {
    let mut times = vec![ProcessorTimes::default(); MOST_CORES];
    let mut returned: u32 = 0;
    let status = unsafe {
        NtQuerySystemInformation(SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION,
                                 times.as_mut_ptr().cast(),
                                 (times.len() * size_of::<ProcessorTimes>()) as u32,
                                 &mut returned)
    };
    if status != 0 {
        return Vec::new();
    }

    let count = returned as usize / size_of::<ProcessorTimes>();
    times.iter().take(count).map(|core| {
        let total = (core.kernel + core.user).max(0) as u64;
        CoreTimes { busy: total.saturating_sub(core.idle.max(0) as u64), total }
    }).collect()
}

/// The machine's RAM.  `None` if Windows won't say.
fn read_machine_memory() -> Option<MachineMemory> {
    let mut status = MemoryStatus {
        _size: size_of::<MemoryStatus>() as u32,
        _load: 0,
        total_physical: 0,
        available_physical: 0,
        _total_page_file: 0,
        _available_page_file: 0,
        _total_virtual: 0,
        _available_virtual: 0,
        _available_extended_virtual: 0,
    };
    match unsafe { GlobalMemoryStatusEx(&mut status) } {
        0 => None,
        _ => Some(MachineMemory { total_bytes: status.total_physical, available_bytes: status.available_physical }),
    }
}

fn thread_cpu_time(thread_id: u32) -> Option<Duration> {
    let thread = unsafe { OpenThread(THREAD_QUERY_LIMITED_INFORMATION, 0, thread_id) };
    if thread.is_null() {
        return None;
    }

    let mut creation = FileTime::default();
    let mut exit = FileTime::default();
    let mut kernel = FileTime::default();
    let mut user = FileTime::default();
    let ok = unsafe { GetThreadTimes(thread, &mut creation, &mut exit, &mut kernel, &mut user) };
    unsafe { CloseHandle(thread) };

    (ok != 0).then(|| kernel.duration() + user.duration())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_time_is_hundreds_of_nanoseconds() {
        let one_second = FileTime { low: 10_000_000, high: 0 };
        assert_eq!(one_second.duration(), Duration::from_secs(1));
    }

    #[test]
    fn this_process_can_be_read() {
        let reading = read().expect("Windows should tell us about our own process");
        assert!(reading.memory_bytes > 0);
        assert!(!reading.threads.is_empty());
        assert!(!reading.cores.is_empty());
        assert!(reading.machine_memory.is_some());
    }
}
