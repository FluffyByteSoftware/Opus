//! File:       Opus/Conductor/dev/conductor-wgui/src/json.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Turns the server's switch, the monitor's snapshot, DiskMan's numbers,
//! the open notices and Scribe's recent lines into the JSON the page asks
//! for once a second.  Written by hand rather than with a crate: it's one
//! shape, it only ever goes out, and JSON is simple enough to write as
//! long as the text is escaped properly.
//!
//! The shape, trimmed (the page's script is the other half of this, so a
//! change here needs one there):
//!
//! ```text
//! { "server": { "state": "stopped", "note": "...", "since": "...Z" },
//!   "login": { "name": "admin", "can_change": true },
//!   "monitor": { "taken_at": "...Z", "uptime_seconds": 61, "os": "...", "process_id": 4092,
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
//!   "notices": { "open": 12, "newest": [ { "id", "when", "level", "source", "text" } ] },
//!   "log": { "file": "...", "lines": [ { "number": 12, "priority": "Info", "text": "..." } ] } }
//! ```
//!
//! `server.state` is stopped, starting, running or stopping; `since` is
//! `null` until the launcher has said anything.  While the server isn't
//! running, `monitor` is `null` and the page shows the Control Panel and
//! the log and nothing else.
//!
//! `login` is who this browser is logged in as: `user` or `admin`, and
//! whether they can change anything (`user` can't, and the page greys
//! every button that would).  `/Opus/login` answers with the same object
//! on its own.
//!
//! Anything that couldn't be measured is `null`, and `monitor` itself is
//! `null` for the second before its first look and while the server is
//! stopped.  `services` comes straight
//! from the services list, not from the monitor, so it still tells the
//! truth if the monitor has died.  `state` is expected, starting, running,
//! trouble or stopped; `since` is `null` while a service is still expected,
//! and `seen_seconds_ago` is `null` for one that doesn't check in.
//! `processes` is busiest first, and a process the OS won't let us read
//! has `null` for its numbers.  `diskman` also comes straight from DiskMan;
//! its `big_write` is `null` when there isn't one under way, and so is
//! `last_failure` when nothing has failed.
//!
//! `notices.newest` is the newest five open notices, newest first, for the
//! bell.  `level` is Notice, Warn or Error.
//!
//! `/Opus/notices` has an answer of its own, every open notice for the
//! Notifications History tab, newest first:
//!
//! ```text
//! { "open": [ { "id", "when", "level", "source", "text" } ] }
//! ```
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
//!
//! `/Opus/settings` has an answer of its own, every config file for the
//! Settings tab, straight from Constellations' table:
//!
//! ```text
//! { "files": [ { "name": "conductor_globals.cfg", "reboot": "hard",
//!                "reboot_text": "a hard reboot (Conductor shut down and run again)",
//!                "about": "...", "loaded": true, "waiting": false,
//!                "settings": [ { "key": "wgui_port", "kind": "port", "low": 1, "high": 65535,
//!                                "about": "...", "default": "9996", "running": "9996",
//!                                "waiting": null } ] } ] }
//! ```
//!
//! `kind` is text, secret, folder, port or number; `low` and `high` are
//! the range for a port or a number and `null` for the rest.  `running`
//! is what Conductor is running on, or, for a file that isn't `loaded`
//! yet this run (`postgres.cfg` before the first START SERVER), what the
//! file says, which is what the next start reads.  `waiting` on a setting
//! is the value saved to the file's `.wait4server` and not yet applied,
//! `null` when there's no such file; `waiting` on the file says whether
//! there is one.  A secret goes out as it is: the page shows it (Jacob's
//! call; nothing leaves the machine).

use std::path::Path;

use conductor_monitor::probe::{MachineMemory, ThreadReading};
use conductor_monitor::{Disk, ProcessInUse, Snapshot, ThreadInUse};
use conductor_tools::archivist::{SlowJob, Status};
use conductor_tools::constellations::{ConfigFile, Kind, Reboot, Setting, Values};
use conductor_tools::diskman::Status as DiskStatus;
use conductor_tools::notices::Notice;
use conductor_tools::scribe::RecentLine;
use conductor_tools::server::Status as ServerStatus;
use conductor_tools::services::Service;
use conductor_tools::threads::ThreadRecord;

use crate::login::Role;

/// The whole answer to `/Opus/status`.
pub(crate) fn status(switch: &ServerStatus,
                     role: Role,
                     snapshot: Option<&Snapshot>,
                     services: &[Service],
                     disk: &DiskStatus,
                     open_notices: usize,
                     newest_notices: &[Notice],
                     lines: &[RecentLine],
                     log_file: Option<&Path>) -> String {
    let log = Object::new()
        .raw("file", log_file.map_or_else(null, |path| text(&path.display().to_string())))
        .raw("lines", array(lines.iter().map(line)))
        .done();

    Object::new()
        .raw("server", server(switch))
        .raw("login", login(role))
        .raw("monitor", snapshot.map_or_else(null, monitor))
        .raw("services", array(services.iter().map(service)))
        .raw("diskman", diskman(disk))
        .raw("notices", Object::new()
            .whole("open", open_notices as u64)
            .raw("newest", array(newest_notices.iter().map(notice)))
            .done())
        .raw("log", log)
        .done()
}

/// Who's logged in, for the status and for `/Opus/login`'s own answer.
pub(crate) fn login(role: Role) -> String {
    Object::new()
        .text("name", role.name())
        .flag("can_change", role.can_change())
        .done()
}

fn server(switch: &ServerStatus) -> String {
    Object::new()
        .text("state", &switch.state.to_string())
        .text("note", &switch.note)
        .raw("since", switch.since.map_or_else(null, |since| text(&since.line_stamp())))
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

/// The whole answer to `/Opus/notices`.
pub(crate) fn notices(open: &[Notice]) -> String {
    Object::new()
        .raw("open", array(open.iter().map(notice)))
        .done()
}

fn notice(notice: &Notice) -> String {
    Object::new()
        .whole("id", notice.id)
        .text("when", &notice.when.line_stamp())
        .text("level", &notice.level.to_string())
        .text("source", &notice.source)
        .text("text", &notice.text)
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
// The settings
// ---------------------------------------------------------------------------

/// One config file as the Settings tab sees it: the table's entry, plus
/// what's running and what's waiting.  `lib.rs` fills these in from
/// Constellations.
pub(crate) struct FileState {
    pub(crate) file: &'static ConfigFile,
    /// Whether `load()` has run for it this run.
    pub(crate) loaded: bool,
    /// Every setting's value: loaded, or from the file on disk when it
    /// isn't loaded, or the defaults when there's no file either.
    pub(crate) running: Values,
    /// Every setting's value out of the `.wait4server`, if there is one.
    pub(crate) waiting: Option<Values>,
}

/// The whole answer to `/Opus/settings`.
pub(crate) fn settings(files: &[FileState]) -> String {
    Object::new()
        .raw("files", array(files.iter().map(file_state)))
        .done()
}

fn file_state(state: &FileState) -> String {
    let file = state.file;
    let reboot = match file.reboot {
        Reboot::Soft => "soft",
        Reboot::Hard => "hard",
    };
    Object::new()
        .text("name", file.name)
        .text("reboot", reboot)
        .text("reboot_text", file.reboot.describe())
        .text("about", file.about)
        .flag("loaded", state.loaded)
        .flag("waiting", state.waiting.is_some())
        .raw("settings", array(file.settings.iter().map(|setting| setting_state(setting, state))))
        .done()
}

fn setting_state(setting: &Setting, state: &FileState) -> String {
    let (kind, range) = match setting.kind {
        Kind::Text => ("text", None),
        Kind::Secret => ("secret", None),
        Kind::Folder => ("folder", None),
        Kind::Port => ("port", Some((1, u64::from(u16::MAX)))),
        Kind::Number { low, high } => ("number", Some((low, high))),
    };
    let running = state.running.get(setting.key).map_or(setting.default, String::as_str);
    let waiting = state.waiting.as_ref().and_then(|values| values.get(setting.key));
    Object::new()
        .text("key", setting.key)
        .text("kind", kind)
        .raw("low", range.map_or_else(null, |(low, _)| low.to_string()))
        .raw("high", range.map_or_else(null, |(_, high)| high.to_string()))
        .text("about", setting.about)
        .text("default", setting.default)
        .text("running", running)
        .raw("waiting", waiting.map_or_else(null, |value| text(value)))
        .done()
}

/// A failed save's complaints, for the page to put beside the fields:
///
/// ```text
/// { "problems": [ "line 2: wgui_port is ...", ... ] }
/// ```
pub(crate) fn problems(problems: &[String]) -> String {
    Object::new()
        .raw("problems", array(problems.iter().map(|problem| text(problem))))
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
        let switch = ServerStatus { state: conductor_tools::server::State::Stopped, note: "x".to_string(),
                                    since: None };
        let answer = status(&switch, Role::User, None, &[], &conductor_tools::diskman::status(), 0, &[], &[], None);
        assert!(answer.starts_with("{\"server\":{\"state\":\"stopped\",\"note\":\"x\",\"since\":null},\
            \"login\":{\"name\":\"user\",\"can_change\":false},\
            \"monitor\":null,\"services\":[],\"diskman\":{\"running\":false,"));
        assert!(answer.ends_with("\"notices\":{\"open\":0,\"newest\":[]},\"log\":{\"file\":null,\"lines\":[]}}"));
    }

    #[test]
    fn a_config_file_goes_out_with_what_runs_and_what_waits() {
        use conductor_tools::constellations::{self, GLOBALS};

        let mut waiting = constellations::values(&GLOBALS);
        waiting.insert("wgui_port", "9997".to_string());
        let state = FileState { file: &GLOBALS, loaded: false, running: constellations::values(&GLOBALS),
                                waiting: Some(waiting) };
        let answer = settings(&[state]);
        assert!(answer.starts_with("{\"files\":[{\"name\":\"conductor_globals.cfg\",\"reboot\":\"hard\",\
            \"reboot_text\":\"a hard reboot (Conductor shut down and run again)\",\"about\":\""));
        assert!(answer.contains("\"loaded\":false,\"waiting\":true,\"settings\":[{\"key\":\"scribe_log_dir\",\
            \"kind\":\"folder\",\"low\":null,\"high\":null,"));
        assert!(answer.contains("\"default\":\"logs\",\"running\":\"logs\",\"waiting\":\"logs\"}"));
        assert!(answer.contains("{\"key\":\"wgui_port\",\"kind\":\"port\",\"low\":1,\"high\":65535,"));
        assert!(answer.ends_with("\"default\":\"9996\",\"running\":\"9996\",\"waiting\":\"9997\"}]}]}"));

        let state = FileState { file: &GLOBALS, loaded: true, running: constellations::values(&GLOBALS),
                                waiting: None };
        let answer = settings(&[state]);
        assert!(answer.contains("\"loaded\":true,\"waiting\":false,"));
        assert!(answer.ends_with("\"running\":\"9996\",\"waiting\":null}]}]}"));
    }

    #[test]
    fn diskman_with_nothing_going_on_has_nulls() {
        let answer = diskman(&conductor_tools::diskman::status());
        assert!(answer.contains("\"big_write\":null"));
        assert!(answer.contains("\"last_failure\":null"));
    }
}
