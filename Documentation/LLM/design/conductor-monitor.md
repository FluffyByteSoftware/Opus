<!--
File:       Opus/Documentation/LLM/design/conductor-monitor.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-monitor

A lib crate.  Conductor's probe into itself.  Once a second, on a thread of its own, it looks at the process
(and every other process on the machine) and keeps the latest look for whoever asks.  Jacob wanted it
separate from the web admin: the monitor measures, the web admin shows.

## Skeleton

```
conductor-monitor/
├── Cargo.toml         depends on conductor-tools, nothing else
└── src/
    ├── lib.rs         start(), stop(), latest() -> Option<Snapshot>
    │                    the thread: look, keep, check in with the services list, wait 1 s
    ├── snapshot.rs    struct Snapshot, struct Disk, struct ThreadInUse, struct ProcessInUse
    │                    build(): two readings a second apart -> percents and speeds
    ├── probe.rs       struct Reading { cpu_time, memory_bytes, disk, threads, cores, machine_memory,
    │                    processes }, struct ProcessReading; picks the file for the OS
    └── probe/
        ├── linux.rs   /proc/self/stat, status, io, task/*/stat; /proc/stat, /proc/meminfo;
        │                /proc/<pid>/stat for every process; threads_of(pid) from /proc/<pid>/task;
        │                os_name() from /etc/os-release
        ├── windows.rs kernel32: GetProcessTimes, K32GetProcessMemoryInfo, GetProcessIoCounters,
        │                Toolhelp32 (threads and processes) + GetThreadTimes, OpenProcess,
        │                GlobalMemoryStatusEx; ntdll: NtQuerySystemInformation for each core,
        │                RtlGetVersion for the version
        └── other.rs   everything else (macOS, for now): read() and threads_of() are None
```

## What a snapshot holds

- When it was taken (with the `Z`), uptime, the OS name ("Nobara Linux 42 (KDE Plasma), kernel 6.14.5",
  "Windows 11 (10.0, build 22631)"), the process id, the core count.
- CPU: Conductor's share of the whole machine over the last second, 0 to 100.  And every core's load over
  the last second, for the whole machine (`/proc/stat` on Linux, ntdll's `NtQuerySystemInformation` on
  Windows).
- Memory: what's actually in RAM, for the whole process.  And the machine's RAM, total and available
  (`/proc/meminfo`, `GlobalMemoryStatusEx`), so the page can put Conductor's use against it.
- Disk: bytes read and written so far, and per second over the last second.
- **Threads in use**: every thread the OS says Conductor has, with its share of one core over the last second
  and its CPU time so far.  A thread started through `threads::spawn()` shows our name and counts as ours;
  anything else (the postgres crate's own threads, say) shows the OS's name for it, if there is one.
- **Threads asked for**: `threads::list()`, our threads with who started each and when.
- Archivist's `status()`, reads and writes included.
- **Processes**: every process on the machine it's allowed to see, Conductor included (`ours`): name, pid,
  share of the whole machine's CPU over the last second, memory in RAM, thread count.  Busiest first, then
  biggest.  A process the OS won't let us read has no numbers, and sorts last.

One more thing is asked for on its own rather than every second: `probe::threads_of(pid)`, one other
process's threads with their CPU time.  The web admin calls it straight from its own thread when the admin
picks a process on the System tab.  Reading a few `/proc` files takes microseconds, so it doesn't need a
thread of its own.

## What we decided

- Separate from the web admin.  The monitor knows nothing about HTTP or JSON; it hands out plain structs.
- **Memory per thread can't be shown.**  Every thread in a process shares the same memory, and no OS keeps
  count of which thread uses what.  Memory is for the process, CPU is per thread.
- Per-thread CPU is "of one core", since one thread can only run on one core at a time.  The process's CPU is
  "of the whole machine".  The page labels both.
- No OS setting in the config.  The compiler knows what it's building for, and `#[cfg]` picks the probe file.
  Jacob asked for a config switch first; it would have had nothing to switch, since a Linux build can't run
  Windows code anyway.
- No crate.  Linux keeps it all in `/proc`, and Windows has it in kernel32, which every Windows program has
  loaded already.  `sysconf` (for ticks per second) is the one C call on Linux.
- The first look has no percents or speeds, since there's nothing to compare it to.  They start with the
  second.
- On an OS it can't measure, the snapshot still comes, with `measured: false`, and the database and our
  thread list still show.  One Info line says so.
- It checks in with `services::seen("Monitor")` after every look, so a monitor that gets stuck shows on the
  Services tab after 5 seconds.  It says "running" from its own thread, not from `start()`, so that can't
  land after "can't measure here" and hide it.
- **Every process** (2026-09-28), for the System tab.  A process's CPU % is its share of the whole machine,
  the same measure as Conductor's own.  Last second's process is matched by pid *and* name, since the OS
  hands a finished process's number to the next one.  Linux leaves out kernel threads (the `PF_KTHREAD` flag
  in `stat`): they're the kernel's own, there are hundreds, and they aren't programs.  Windows leaves out
  process 0, the "System Idle Process", which is idle time, not a program.

## Differences between the OSes

- Disk on Linux is `read_bytes` / `write_bytes`: what actually reached the disk.  A read the OS answered from
  its cache doesn't count.  On Windows, `GetProcessIoCounters` counts every read and write the process makes,
  network included.  So the same work shows bigger numbers on Windows.
- Linux thread names are cut to 15 characters.  Windows threads have no name from the probe, so only ours
  (matched by id) get one.
- Other processes: Linux lets anyone read every process's `stat`, so every one has numbers.  Windows won't
  open its own protected processes, or other users' unless Conductor runs as an administrator; those show
  with a thread count and nothing else, and their threads mostly won't open either.

## What's open

- macOS isn't measured.  It would be `proc_pidinfo` and friends from libproc.  Waiting on a Mac to test it.
- The Windows probe was written without a Windows machine to build it on, and the process list and
  `threads_of` were added the same way.  First build there may need a fix.
- Uptime counts from when the monitor started, a moment after Conductor did.
