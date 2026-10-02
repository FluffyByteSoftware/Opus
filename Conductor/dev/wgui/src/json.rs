//! File:       Opus/Conductor/dev/wgui/src/json.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Turns the server's switch, the monitor's snapshot, DiskMan's numbers,
//! networking's door, the open notices and Scribe's recent lines into the
//! JSON the page asks for once a second.  Written by hand rather than with a crate: it's one
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
//!   "networking": { "tcp": "0.0.0.0:9997", "udp": "0.0.0.0:9998", "players": 1, "tickets": 0,
//!                   "connections": [ { "id": 7, "address": "192.168.1.20:51234", "host": "desk.lan",
//!                                      "arrived": "...Z", "seconds_ago": 12, "stage": "in_line",
//!                                      "text": "In Security's line: 2 ahead, about 400 ms",
//!                                      "queued_ahead": 0, "done": false, "logged_in": false } ],
//!                   "in_world": [ { "address": "192.168.1.20:51235", "account": "jacob_01",
//!                                   "character": "Jacob", "connected": "...Z", "playing_seconds": 61,
//!                                   "quiet_seconds": 0 } ],
//!                   "access": "off", "whitelisted": 0, "blacklisted": 2 },
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
//! `networking` comes straight from the networking crate.  `tcp` and
//! `udp` are where each side listens, `null` while it doesn't, and the
//! Network Admin tabs are locked until both are there.  `connections` is
//! every connection that reached the TCP listener since START SERVER,
//! newest first, and where each one is: `stage` is queued, handshake,
//! login, checking, in_line, asked or done, and `text` says it in words
//! (for in_line, how many are ahead and about how long; for done, how it
//! ended, or "LINKDEAD: ..." and how for a login whose player has since
//! left the world or never came).  `logged_in` is true only while that
//! player is still in it, which is what the page colours green.
//! `queued_ahead` is how many queued connections arrived before a queued
//! one.  `host` is the address's name from reverse DNS, `null`
//! until the lookup is back or when it has none.  `id` is what
//! `/Opus/wwwhook/tcp/kick?id=N` takes.  No account name is in it, on
//! purpose: the tab is about the door.
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
//! `/Opus/networking` has an answer of its own, both access lists for
//! the Whitelist and Blacklist tabs.  `running` is false, with empty
//! lists, while the server isn't running (the lists only load with it);
//! each entry is as it's kept, an address or a range:
//!
//! ```text
//! { "running": true, "mode": "blacklist", "whitelist": [ "10.0.0.0/8" ], "blacklist": [ "1.2.3.4" ] }
//! ```
//!
//! `/Opus/wwwhook/networking/addip` and `/removeip` answer with what the
//! change did: `changed` is false when there was nothing to do (listed
//! already, or not there to take off), `enforced` says whether the door
//! is checking that list, and the two counts are who was dropped for it
//! (a blacklisting, or a whitelist entry taken away, with that list on):
//!
//! ```text
//! { "entry": "1.2.3.0/24", "changed": true, "enforced": true, "tcp_closed": 1, "players_dropped": 0 }
//! ```
//!
//! `/Opus/Content/accounts` has an answer of its own, every game account
//! for the Accounts tab, by name.  Every column but the password hash;
//! `created` and `last_login` are UTC, and `last_login` is `null` for an
//! account nobody has played on yet:
//!
//! ```text
//! { "accounts": [ { "username": "jacob_01", "uuid": "0199...", "first_name": "Jacob",
//!                   "last_name": "Chacko", "email": "jacob@example.com", "created": "...Z",
//!                   "last_login": null } ] }
//! ```
//!
//! Whether an account's player is in the world isn't in it: the page
//! reads that from the status's `networking.in_world`, which it has every
//! second anyway.
//!
//! `/Opus/Content/characters` is every player's character for the
//! Characters tab, by name: its name, UUID, where it last stood (x, y, z,
//! with y up, as of its last save) and the account it belongs to:
//!
//! ```text
//! { "characters": [ { "name": "Jacob", "uuid": "0199...", "account": "jacob_01",
//!                     "x": 0.00, "y": 0.00, "z": 0.00 } ] }
//! ```
//!
//! `/Opus/wwwhook/accounts/create` and `/password` answer `{ "job": 7 }`,
//! the account desk's number for the job, and `/Opus/Content/accounts/job`
//! says where it is.  `state` is working, done or failed, and `text` says
//! what happened in words (why, for a failed one):
//!
//! ```text
//! { "job": 7, "state": "done", "text": "Made the account jacob_01." }
//! ```
//!
//! `/edit` answers `{ "saved": true }` and `/delete`
//! `{ "deleted": true, "kicked": false }` (`kicked` is true when its player
//! was in the world, or had a ticket, and was taken out).  A field that's
//! wrong comes back as a 400 with `{ "problems": [ "email: ..." ] }`, each
//! with the field's name in front for the page to put it beside.
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
//! `kind` is text, secret, password, folder, port or number; `low` and `high` are
//! the range for a port or a number and `null` for the rest.  `running`
//! is what Conductor is running on, or, for a file that isn't `loaded`
//! yet this run (`postgres.cfg` before the first START SERVER), what the
//! file says, which is what the next start reads.  `waiting` on a setting
//! is the value saved to the file's `.wait4server` and not yet applied,
//! `null` when there's no such file; `waiting` on the file says whether
//! there is one.  A secret goes out as it is to `admin`, who may change
//! it, and as `""` to `user`, who may not (and shouldn't be able to read
//! the admin's password off the page).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use conductor_accounts::Account;
use conductor_accounts::characters::CharacterSnapshot;
use conductor_accounts::desk::Outcome;
use conductor_monitor::probe::{MachineMemory, ThreadReading};
use conductor_monitor::{Disk, ProcessInUse, Snapshot, ThreadInUse};
use conductor_networking::{AccessLists, Changed, Connection, End, Player, Stage, Status as NetStatus};
use conductor_tools::archivist::{SlowJob, Status};
use conductor_tools::clock::Utc;
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
                     networking: &NetStatus,
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
        .raw("networking", net(networking))
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

fn net(status: &NetStatus) -> String {
    Object::new()
        .raw("tcp", status.tcp.map_or_else(null, |address| text(&address.to_string())))
        .raw("udp", status.udp.map_or_else(null, |address| text(&address.to_string())))
        .whole("players", status.players as u64)
        .whole("tickets", status.tickets as u64)
        .raw("connections", array(status.connections.iter().map(connection)))
        .raw("in_world", array(status.in_world.iter().map(player)))
        .text("access", status.access.word())
        .whole("whitelisted", status.whitelisted as u64)
        .whole("blacklisted", status.blacklisted as u64)
        .done()
}

fn player(player: &Player) -> String {
    Object::new()
        .text("address", &player.address.to_string())
        .text("account", &player.account)
        .raw("character", player.character.as_deref().map_or_else(null, text))
        .text("connected", &player.connected.line_stamp())
        .whole("playing_seconds", player.playing_for.as_secs())
        .whole("quiet_seconds", player.quiet_for.as_secs())
        .done()
}

/// `/Opus/networking`: both access lists, or none while the server
/// isn't running.
pub(crate) fn access(lists: Option<&AccessLists>) -> String {
    match lists {
        Some(lists) => Object::new()
            .flag("running", true)
            .text("mode", lists.mode.word())
            .raw("whitelist", array(lists.whitelist.iter().map(|entry| text(entry))))
            .raw("blacklist", array(lists.blacklist.iter().map(|entry| text(entry))))
            .done(),
        None => Object::new()
            .flag("running", false)
            .text("mode", "off")
            .raw("whitelist", array(std::iter::empty()))
            .raw("blacklist", array(std::iter::empty()))
            .done(),
    }
}

/// `/Opus/wwwhook/networking/addip`'s and `/removeip`'s answer.
pub(crate) fn changed(changed: &Changed) -> String {
    Object::new()
        .text("entry", &changed.entry)
        .flag("changed", changed.changed)
        .flag("enforced", changed.enforced)
        .whole("tcp_closed", changed.tcp_closed as u64)
        .whole("players_dropped", changed.players_dropped as u64)
        .done()
}

fn connection(connection: &Connection) -> String {
    Object::new()
        .whole("id", connection.id)
        .text("address", &connection.address.to_string())
        .raw("host", connection.host.as_deref().map_or_else(null, text))
        .text("arrived", &connection.arrived.line_stamp())
        .whole("seconds_ago", connection.ago.as_secs())
        .text("stage", connection.stage.word())
        .text("text", &connection.describe())
        .whole("queued_ahead", connection.queued_ahead as u64)
        .flag("done", connection.stage.is_done())
        .flag("logged_in", connection.stage == Stage::Done(End::LoggedIn) && connection.gone.is_none())
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

/// The whole answer to `/Opus/settings`.  A secret (a password) goes out
/// as it is to `admin` and as nothing to `user`, who can't change it and
/// shouldn't be able to read the admin's password off the page and
/// become admin (the 0.0.1 review's R1).
pub(crate) fn settings(files: &[FileState], role: Role) -> String {
    let secrets_shown = role.can_change();
    Object::new()
        .raw("files", array(files.iter().map(|state| file_state(state, secrets_shown))))
        .done()
}

fn file_state(state: &FileState, secrets_shown: bool) -> String {
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
        .raw("settings", array(file.settings.iter().map(|setting| setting_state(setting, state, secrets_shown))))
        .done()
}

fn setting_state(setting: &Setting, state: &FileState, secrets_shown: bool) -> String {
    let (kind, range) = match setting.kind {
        Kind::Text => ("text", None),
        Kind::Secret => ("secret", None),
        Kind::Password => ("password", None),
        Kind::Folder => ("folder", None),
        Kind::Port => ("port", Some((1, u64::from(u16::MAX)))),
        Kind::Number { low, high } => ("number", Some((low, high))),
    };
    let hidden = matches!(setting.kind, Kind::Secret | Kind::Password) && !secrets_shown;
    let running = if hidden { "" } else { state.running.get(setting.key).map_or(setting.default, String::as_str) };
    let default = if hidden { "" } else { setting.default };
    let waiting = state.waiting.as_ref().and_then(|values| values.get(setting.key))
        .map(|value| if hidden { "" } else { value.as_str() });
    Object::new()
        .text("key", setting.key)
        .text("kind", kind)
        .raw("low", range.map_or_else(null, |(low, _)| low.to_string()))
        .raw("high", range.map_or_else(null, |(_, high)| high.to_string()))
        .text("about", setting.about)
        .text("default", default)
        .text("running", running)
        .raw("waiting", waiting.map_or_else(null, text))
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

/// The whole answer to `/Opus/Content/accounts`.
pub(crate) fn accounts(list: &[Account]) -> String {
    Object::new()
        .raw("accounts", array(list.iter().map(account)))
        .done()
}

fn account(account: &Account) -> String {
    Object::new()
        .text("username", account.username())
        .text("uuid", account.uuid())
        .text("first_name", &account.first_name)
        .text("last_name", &account.last_name)
        .text("email", &account.email)
        .raw("created", when(account.created_at()))
        .raw("last_login", when(account.last_login()))
        .done()
}

/// A time from the database, the way a person reads it (UTC, ending in
/// `Z`), or `null`.
fn when(time: Option<SystemTime>) -> String {
    let Some(time) = time else {
        return null();
    };
    // A time before 1970 can't come out of the accounts table, but if one
    // did, it would show as 1970 rather than stop the answer.
    let seconds = time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs() as i64);
    text(&Utc::from_unix(seconds).line_stamp())
}

/// The whole answer to `/Opus/Content/characters`.
pub(crate) fn characters(list: &[CharacterSnapshot]) -> String {
    Object::new()
        .raw("characters", array(list.iter().map(character)))
        .done()
}

fn character(character: &CharacterSnapshot) -> String {
    let [x, y, z] = character.position();
    Object::new()
        .text("name", character.name())
        .text("uuid", character.uuid())
        .text("account", character.account_username())
        .raw("x", decimal(Some(f64::from(x))))
        .raw("y", decimal(Some(f64::from(y))))
        .raw("z", decimal(Some(f64::from(z))))
        .done()
}

/// The whole answer to `/Opus/Content/accounts/job?id=N`.
pub(crate) fn account_job(number: u64, outcome: &Outcome) -> String {
    Object::new()
        .whole("job", number)
        .text("state", outcome.progress.word())
        .text("text", &outcome.text)
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
        let quiet = NetStatus { tcp: None, udp: None, players: 0, tickets: 0, connections: Vec::new(),
                                in_world: Vec::new(), access: conductor_networking::AccessMode::Off, whitelisted: 0,
                                blacklisted: 0 };
        let answer = status(&switch, Role::User, None, &[], &conductor_tools::diskman::status(), &quiet, 0, &[], &[],
                            None);
        assert!(answer.starts_with("{\"server\":{\"state\":\"stopped\",\"note\":\"x\",\"since\":null},\
            \"login\":{\"name\":\"user\",\"can_change\":false},\
            \"monitor\":null,\"services\":[],\"diskman\":{\"running\":false,"));
        assert!(answer.contains("\"networking\":{\"tcp\":null,\"udp\":null,\"players\":0,\"tickets\":0,\
            \"connections\":[],\"in_world\":[],\"access\":\"off\",\"whitelisted\":0,\"blacklisted\":0},"));
        assert!(answer.ends_with("\"notices\":{\"open\":0,\"newest\":[]},\"log\":{\"file\":null,\"lines\":[]}}"));
    }

    #[test]
    fn a_connection_goes_out_with_its_stage_in_a_word_and_in_words() {
        use conductor_networking::Gone;
        use conductor_tools::clock::Utc;
        use std::time::Duration;

        let waiting = Connection { id: 7, address: "192.168.1.20:51234".parse().unwrap(),
                                   host: Some("desk.lan".to_string()), arrived: Utc::from_unix(1_790_000_000),
                                   ago: Duration::from_millis(12_400),
                                   stage: Stage::InLine { ahead: 2, wait: Duration::from_millis(400) },
                                   queued_ahead: 0, gone: None };
        assert_eq!(connection(&waiting), "{\"id\":7,\"address\":\"192.168.1.20:51234\",\"host\":\"desk.lan\",\
            \"arrived\":\"02:13:20 PM - 09-21-26 Z\",\"seconds_ago\":12,\"stage\":\"in_line\",\
            \"text\":\"In Security's line: 2 ahead, about 400 ms\",\"queued_ahead\":0,\"done\":false,\
            \"logged_in\":false}");

        let done = Connection { id: 8, address: "[::1]:40000".parse().unwrap(), host: None,
                                arrived: Utc::from_unix(1_790_000_000), ago: Duration::from_secs(1),
                                stage: Stage::Done(End::LoggedIn), queued_ahead: 0, gone: None };
        let answer = connection(&done);
        assert!(answer.contains("\"host\":null,"));
        assert!(answer.ends_with("\"stage\":\"done\",\"text\":\"Logged in and handed a ticket for UDP\",\
            \"queued_ahead\":0,\"done\":true,\"logged_in\":true}"));

        // The same login once a second one has logged its player out: no
        // longer green, and it says why.
        let replaced = Connection { gone: Some(Gone::Replaced { by: "10.0.0.84:44194".parse().unwrap() }), ..done };
        assert!(connection(&replaced).ends_with("\"stage\":\"done\",\
            \"text\":\"LINKDEAD: logged out by a second login from 10.0.0.84:44194\",\
            \"queued_ahead\":0,\"done\":true,\"logged_in\":false}"));
    }

    #[test]
    fn a_player_goes_out_with_their_account_and_times() {
        use conductor_tools::clock::Utc;
        use std::time::Duration;

        let jacob = Player { address: "192.168.1.20:51235".parse().unwrap(), account: "jacob_01".to_string(),
                             character: Some("Jacob".to_string()), connected: Utc::from_unix(1_790_000_000),
                             playing_for: Duration::from_millis(61_900), quiet_for: Duration::from_millis(400) };
        assert_eq!(player(&jacob), "{\"address\":\"192.168.1.20:51235\",\"account\":\"jacob_01\",\
            \"character\":\"Jacob\",\"connected\":\"02:13:20 PM - 09-21-26 Z\",\"playing_seconds\":61,\
            \"quiet_seconds\":0}");

        // At character select, there's no character yet.
        let selecting = Player { character: None, ..jacob };
        assert!(player(&selecting).contains("\"character\":null,"));
    }

    #[test]
    fn the_lists_go_out_as_they_are_or_as_not_running() {
        use conductor_networking::AccessMode;

        assert_eq!(access(None), "{\"running\":false,\"mode\":\"off\",\"whitelist\":[],\"blacklist\":[]}");
        let lists = AccessLists { mode: AccessMode::Blacklist, whitelist: vec!["10.0.0.0/8".to_string()],
                                  blacklist: vec!["1.2.3.4".to_string(), "2001:db8::/32".to_string()] };
        assert_eq!(access(Some(&lists)), "{\"running\":true,\"mode\":\"blacklist\",\"whitelist\":[\"10.0.0.0/8\"],\
            \"blacklist\":[\"1.2.3.4\",\"2001:db8::/32\"]}");

        let done = Changed { entry: "1.2.3.0/24".to_string(), changed: true, enforced: true, tcp_closed: 1,
                             players_dropped: 0 };
        assert_eq!(changed(&done), "{\"entry\":\"1.2.3.0/24\",\"changed\":true,\"enforced\":true,\"tcp_closed\":1,\
            \"players_dropped\":0}");
    }

    #[test]
    fn a_config_file_goes_out_with_what_runs_and_what_waits() {
        use conductor_tools::constellations::{self, GLOBALS};

        let mut waiting = constellations::values(&GLOBALS);
        waiting.insert("wgui_port", "9997".to_string());
        let state = FileState { file: &GLOBALS, loaded: false, running: constellations::values(&GLOBALS),
                                waiting: Some(waiting) };
        let answer = settings(&[state], Role::Admin);
        assert!(answer.starts_with("{\"files\":[{\"name\":\"conductor_globals.cfg\",\"reboot\":\"hard\",\
            \"reboot_text\":\"a hard reboot (Conductor shut down and run again)\",\"about\":\""));
        assert!(answer.contains("\"loaded\":false,\"waiting\":true,\"settings\":[{\"key\":\"scribe_log_dir\",\
            \"kind\":\"folder\",\"low\":null,\"high\":null,"));
        assert!(answer.contains("\"default\":\"logs\",\"running\":\"logs\",\"waiting\":\"logs\"}"));
        assert!(answer.contains("{\"key\":\"wgui_port\",\"kind\":\"port\",\"low\":1,\"high\":65535,"));
        assert!(answer.ends_with("\"default\":\"9996\",\"running\":\"9996\",\"waiting\":\"9997\"}]}]}"));

        let state = FileState { file: &GLOBALS, loaded: true, running: constellations::values(&GLOBALS),
                                waiting: None };
        let answer = settings(&[state], Role::Admin);
        assert!(answer.contains("\"loaded\":true,\"waiting\":false,"));
        assert!(answer.ends_with("\"running\":\"9996\",\"waiting\":null}]}]}"));
    }

    #[test]
    fn a_secret_goes_out_to_admin_and_not_to_user() {
        use conductor_tools::constellations::{self, WGUI};

        let mut waiting = constellations::values(&WGUI);
        waiting.insert("admin_password", "hunter2".to_string());
        let state = FileState { file: &WGUI, loaded: true, running: constellations::values(&WGUI),
                                waiting: Some(waiting) };
        let admin = settings(&[state], Role::Admin);
        assert!(admin.contains("{\"key\":\"admin_password\",\"kind\":\"password\",\"low\":null,\"high\":null,"));
        assert!(admin.contains("\"default\":\"admin\",\"running\":\"admin\",\"waiting\":\"hunter2\"}"), "{admin}");

        let mut waiting = constellations::values(&WGUI);
        waiting.insert("admin_password", "hunter2".to_string());
        let state = FileState { file: &WGUI, loaded: true, running: constellations::values(&WGUI),
                                waiting: Some(waiting) };
        let user = settings(&[state], Role::User);
        assert!(user.contains("{\"key\":\"admin_password\",\"kind\":\"password\",\"low\":null,\"high\":null,"));
        assert!(user.contains("\"default\":\"\",\"running\":\"\",\"waiting\":\"\"}"), "{user}");
        assert!(!user.contains("hunter2") && !user.contains("\"admin\",\"running\""), "{user}");
    }

    #[test]
    fn an_account_goes_out_with_every_column_but_the_hash() {
        let account = Account::new("jacob_01", "Jacob", "Chacko", "jacob@example.com").unwrap();
        let answer = accounts(&[account.clone()]);
        assert_eq!(answer, format!("{{\"accounts\":[{{\"username\":\"jacob_01\",\"uuid\":\"{}\",\
            \"first_name\":\"Jacob\",\"last_name\":\"Chacko\",\"email\":\"jacob@example.com\",\
            \"created\":null,\"last_login\":null}}]}}", account.uuid()));
        assert_eq!(accounts(&[]), "{\"accounts\":[]}");
        assert_eq!(when(Some(UNIX_EPOCH + std::time::Duration::from_secs(1_790_000_000))),
                   "\"02:13:20 PM - 09-21-26 Z\"");
    }

    #[test]
    fn a_job_goes_out_with_its_state_in_a_word() {
        use conductor_accounts::desk::Progress;
        let outcome = Outcome { progress: Progress::Failed, text: "There's already an account called x.".to_string() };
        assert_eq!(account_job(7, &outcome),
                   "{\"job\":7,\"state\":\"failed\",\"text\":\"There's already an account called x.\"}");
    }

    #[test]
    fn diskman_with_nothing_going_on_has_nulls() {
        let answer = diskman(&conductor_tools::diskman::status());
        assert!(answer.contains("\"big_write\":null"));
        assert!(answer.contains("\"last_failure\":null"));
    }
}
