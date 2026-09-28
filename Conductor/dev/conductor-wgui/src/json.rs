//! File:       Opus/Conductor/dev/conductor-wgui/src/json.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Turns the monitor's snapshot, DiskMan's numbers and Scribe's recent
//! lines into the JSON the page asks for once a second.  Written by hand rather than with a crate:
//! it's one shape, it only ever goes out, and JSON is simple enough to
//! write as long as the text is escaped properly.
//!
//! The shape, trimmed (the page's script is the other half of this, so a
//! change here needs one there):
//!
//! ```text
//! { "monitor": { "taken_at": "...Z", "uptime_seconds": 61, "os": "...", "process_id": 4092,
//!                "cores": 16, "measured": true, "cpu_percent": 1.25, "memory_bytes": 9437184,
//!                "core_percents": [ 3.00, 12.50, ... ],
//!                "machine_memory": { "total_bytes", "available_bytes" },
//!                "disk": { "read_bytes", "written_bytes", "read_per_second", "written_per_second" },
//!                "threads_in_use": [ { "os_id", "name", "ours", "core_percent", "cpu_ms" } ],
//!                "threads_asked_for": [ { "name", "started_by", "started_at", "os_id", "running" } ],
//!                "database": { "running", "connected", "waiting", "jobs_done", "reads", "writes",
//!                              "other", "slow_jobs", "slowest_ms", "recent_slow": [ ... ] },
//!                "processes": [ { "pid", "name", "ours", "cpu_percent", "memory_bytes", "threads" } ] },
//!   "services": [ { "name": "Archivist", "state": "running", "note": "...", "since": "...Z",
//!                   "seen_seconds_ago": 0.42, "healthy": true } ],
//!   "diskman": { "running", "stopping", "files_waiting", "bytes_waiting", "files_loaded", "bytes_loaded",
//!                "files_failing", "reads_waiting", "streams_open",
//!                "big_write": { "file", "done_bytes", "total_bytes" },
//!                "writes_done", "appends_done", "reads_done", "cache_hits", "bytes_written", "bytes_read",
//!                "failures", "given_up", "last_failure": { "when", "what" }, "slowest_write_ms" },
//!   "log": { "file": "...", "lines": [ { "number": 12, "priority": "Info", "text": "..." } ] } }
//! ```
//!
//! Anything that couldn't be measured is `null`, and `monitor` itself is
//! `null` for the second before its first look.  `services` comes straight
//! from the services list, not from the monitor, so it still tells the
//! truth if the monitor has died.  `state` is expected, starting, running,
//! trouble or stopped; `since` is `null` while a service is still expected,
//! and `seen_seconds_ago` is `null` for one that doesn't check in.
//! `processes` is busiest first, and a process the OS won't let us read
//! has `null` for its numbers.  `diskman` also comes straight from DiskMan;
//! its `big_write` is `null` when there isn't one under way, and so is
//! `last_failure` when nothing has failed.
//!
//! `/Opus/threads?pid=N` has an answer of its own, one process's threads:
//!
//! ```text
//! { "pid": 1234, "visible": true, "threads": [ { "os_id", "name", "cpu_ms" } ] }
//! ```
//!
//! `visible` is false, with no threads, when the process is gone or the OS
//! won't let us look.  There's no percent in it: the page asks once a
//! second and works that out from two answers.

use std::path::Path;

use conductor_monitor::probe::{MachineMemory, ThreadReading};
use conductor_monitor::{Disk, ProcessInUse, Snapshot, ThreadInUse};
use conductor_tools::archivist::{SlowJob, Status};
use conductor_tools::diskman::Status as DiskStatus;
use conductor_tools::scribe::RecentLine;
use conductor_tools::services::Service;
use conductor_tools::threads::ThreadRecord;

/// The whole answer to `/Opus/status`.
pub(crate) fn status(snapshot: Option<&Snapshot>,
                     services: &[Service],
                     disk: &DiskStatus,
                     lines: &[RecentLine],
                     log_file: Option<&Path>) -> String {
    let log = Object::new()
        .raw("file", log_file.map_or_else(null, |path| text(&path.display().to_string())))
        .raw("lines", array(lines.iter().map(line)))
        .done();

    Object::new()
        .raw("monitor", snapshot.map_or_else(null, monitor))
        .raw("services", array(services.iter().map(service)))
        .raw("diskman", diskman(disk))
        .raw("log", log)
        .done()
}

fn monitor(snapshot: &Snapshot) -> String {
    Object::new()
        .text("taken_at", &snapshot.taken_at.line_stamp())
        .whole("uptime_seconds", snapshot.uptime.as_secs())
        .text("os", &snapshot.os)
        .whole("process_id", u64::from(snapshot.process_id))
        .whole("cores", snapshot.cores as u64)
        .flag("measured", snapshot.measured)
        .raw("cpu_percent", decimal(snapshot.cpu_percent))
        .raw("memory_bytes", snapshot.memory_bytes.map_or_else(null, |bytes| bytes.to_string()))
        .raw("core_percents", array(snapshot.core_percents.iter().map(|&percent| decimal(Some(percent)))))
        .raw("machine_memory", snapshot.machine_memory.as_ref().map_or_else(null, machine_memory))
        .raw("disk", snapshot.disk.as_ref().map_or_else(null, disk))
        .raw("threads_in_use", array(snapshot.threads_in_use.iter().map(thread_in_use)))
        .raw("threads_asked_for", array(snapshot.threads_asked_for.iter().map(thread_asked_for)))
        .raw("database", database(&snapshot.database))
        .raw("processes", array(snapshot.processes.iter().map(process)))
        .done()
}

/// The whole answer to `/Opus/threads?pid=N`.  `None` for a process we
/// can't see.
pub(crate) fn threads_of(pid: u32, threads: Option<&[ThreadReading]>) -> String {
    Object::new()
        .whole("pid", u64::from(pid))
        .flag("visible", threads.is_some())
        .raw("threads", array(threads.unwrap_or_default().iter().map(thread_reading)))
        .done()
}

fn process(process: &ProcessInUse) -> String {
    Object::new()
        .whole("pid", u64::from(process.pid))
        .text("name", &process.name)
        .flag("ours", process.ours)
        .raw("cpu_percent", decimal(process.cpu_percent))
        .raw("memory_bytes", process.memory_bytes.map_or_else(null, |bytes| bytes.to_string()))
        .raw("threads", process.threads.map_or_else(null, |count| count.to_string()))
        .done()
}

fn thread_reading(thread: &ThreadReading) -> String {
    Object::new()
        .whole("os_id", thread.os_id)
        .raw("name", thread.name.as_deref().map_or_else(null, text))
        .whole("cpu_ms", thread.cpu_time.as_millis() as u64)
        .done()
}

fn machine_memory(memory: &MachineMemory) -> String {
    Object::new()
        .whole("total_bytes", memory.total_bytes)
        .whole("available_bytes", memory.available_bytes)
        .done()
}

fn disk(disk: &Disk) -> String {
    Object::new()
        .whole("read_bytes", disk.read_bytes)
        .whole("written_bytes", disk.written_bytes)
        .raw("read_per_second", decimal(disk.read_per_second))
        .raw("written_per_second", decimal(disk.written_per_second))
        .done()
}

fn thread_in_use(thread: &ThreadInUse) -> String {
    Object::new()
        .whole("os_id", thread.os_id)
        .text("name", &thread.name)
        .flag("ours", thread.ours)
        .raw("core_percent", decimal(thread.core_percent))
        .whole("cpu_ms", thread.cpu_time.as_millis() as u64)
        .done()
}

fn thread_asked_for(record: &ThreadRecord) -> String {
    Object::new()
        .text("name", &record.name)
        .text("started_by", &record.started_by)
        .text("started_at", &record.started_at.line_stamp())
        .raw("os_id", record.os_id.map_or_else(null, |id| id.to_string()))
        .flag("running", record.running)
        .done()
}

fn database(status: &Status) -> String {
    Object::new()
        .flag("running", status.running)
        .flag("connected", status.connected)
        .whole("waiting", status.waiting as u64)
        .whole("jobs_done", status.jobs_done)
        .whole("reads", status.reads)
        .whole("writes", status.writes)
        .whole("other", status.other)
        .whole("slow_jobs", status.slow_jobs)
        .whole("slowest_ms", status.slowest.as_millis() as u64)
        .raw("recent_slow", array(status.recent_slow.iter().map(slow_job)))
        .done()
}

fn slow_job(job: &SlowJob) -> String {
    Object::new()
        .text("when", &job.when.line_stamp())
        .text("label", &job.label)
        .whole("ran_ms", job.ran_for.as_millis() as u64)
        .whole("waited_ms", job.waited.as_millis() as u64)
        .done()
}

fn service(service: &Service) -> String {
    Object::new()
        .text("name", service.name)
        .text("state", &service.state.to_string())
        .text("note", &service.note)
        .raw("since", service.since.map_or_else(null, |since| text(&since.line_stamp())))
        .raw("seen_seconds_ago", decimal(service.seen_ago.map(|ago| ago.as_secs_f64())))
        .flag("healthy", service.healthy())
        .done()
}

fn diskman(status: &DiskStatus) -> String {
    let big_write = status.big_write.as_ref().map_or_else(null, |(file, done, total)| {
        Object::new()
            .text("file", &file.display().to_string())
            .whole("done_bytes", *done)
            .whole("total_bytes", *total)
            .done()
    });
    let last_failure = status.last_failure.as_ref().map_or_else(null, |(when, what)| {
        Object::new().text("when", &when.line_stamp()).text("what", what).done()
    });

    Object::new()
        .flag("running", status.running)
        .flag("stopping", status.stopping)
        .whole("files_waiting", status.files_waiting as u64)
        .whole("bytes_waiting", status.bytes_waiting)
        .whole("files_loaded", status.files_loaded as u64)
        .whole("bytes_loaded", status.bytes_loaded)
        .whole("files_failing", status.files_failing as u64)
        .whole("reads_waiting", status.reads_waiting as u64)
        .whole("streams_open", status.streams_open as u64)
        .raw("big_write", big_write)
        .whole("writes_done", status.writes_done)
        .whole("appends_done", status.appends_done)
        .whole("reads_done", status.reads_done)
        .whole("cache_hits", status.cache_hits)
        .whole("bytes_written", status.bytes_written)
        .whole("bytes_read", status.bytes_read)
        .whole("failures", status.failures)
        .whole("given_up", status.given_up)
        .raw("last_failure", last_failure)
        .whole("slowest_write_ms", status.slowest_write.as_millis() as u64)
        .done()
}

fn line(line: &RecentLine) -> String {
    Object::new()
        .whole("number", line.number)
        .text("priority", &line.priority.to_string())
        .text("text", &line.text)
        .done()
}

// ---------------------------------------------------------------------------
// Writing JSON
// ---------------------------------------------------------------------------

/// A JSON object, one field at a time.
///
/// ```text
/// Object::new().text("name", "archivist").flag("running", true).done()
///     -> {"name":"archivist","running":true}
/// ```
struct Object {
    fields: Vec<String>,
}

impl Object {
    fn new() -> Object {
        Object { fields: Vec::new() }
    }

    /// A value that's already JSON: an object, an array, `null`.
    fn raw(mut self, key: &str, value: String) -> Object {
        self.fields.push(format!("{}:{value}", text(key)));
        self
    }

    fn text(self, key: &str, value: &str) -> Object {
        self.raw(key, text(value))
    }

    fn whole(self, key: &str, value: u64) -> Object {
        self.raw(key, value.to_string())
    }

    fn flag(self, key: &str, value: bool) -> Object {
        self.raw(key, value.to_string())
    }

    fn done(self) -> String {
        format!("{{{}}}", self.fields.join(","))
    }
}

/// A JSON array of values that are already JSON.
fn array(items: impl Iterator<Item = String>) -> String {
    format!("[{}]", items.collect::<Vec<_>>().join(","))
}

fn null() -> String {
    "null".to_string()
}

/// A number with two places, or `null`.  JSON has no way to write "not a
/// number" or infinity, so those are `null` too.
fn decimal(value: Option<f64>) -> String {
    match value {
        Some(value) if value.is_finite() => format!("{value:.2}"),
        _ => null(),
    }
}

/// Text in quotes, with anything that would break out of the quotes
/// escaped.  A log line can hold anything, a quote or a stray control
/// character included, so everything below a space gets the `\u` form.
fn text(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_escaped() {
        assert_eq!(text("plain"), "\"plain\"");
        assert_eq!(text("a \"quote\" and a \\"), "\"a \\\"quote\\\" and a \\\\\"");
        assert_eq!(text("two\nlines\ttab\u{1}"), "\"two\\nlines\\ttab\\u0001\"");
    }

    #[test]
    fn numbers_that_json_cant_hold_are_null() {
        assert_eq!(decimal(Some(1.0 / 3.0)), "0.33");
        assert_eq!(decimal(Some(f64::NAN)), "null");
        assert_eq!(decimal(None), "null");
    }

    #[test]
    fn objects_and_arrays_put_together() {
        let inner = Object::new().whole("n", 1).flag("yes", false).done();
        let outer = Object::new().text("name", "x").raw("list", array([inner].into_iter())).done();
        assert_eq!(outer, "{\"name\":\"x\",\"list\":[{\"n\":1,\"yes\":false}]}");
    }

    #[test]
    fn a_process_we_cant_see_has_no_threads() {
        assert_eq!(threads_of(7, None), "{\"pid\":7,\"visible\":false,\"threads\":[]}");
    }

    #[test]
    fn before_the_first_look_the_monitor_is_null() {
        let answer = status(None, &[], &conductor_tools::diskman::status(), &[], None);
        assert!(answer.starts_with("{\"monitor\":null,\"services\":[],\"diskman\":{\"running\":false,"));
        assert!(answer.ends_with("\"log\":{\"file\":null,\"lines\":[]}}"));
    }

    #[test]
    fn diskman_with_nothing_going_on_has_nulls() {
        let answer = diskman(&conductor_tools::diskman::status());
        assert!(answer.contains("\"big_write\":null"));
        assert!(answer.contains("\"last_failure\":null"));
    }
}
