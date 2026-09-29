//! File:       Opus/Conductor/dev/conductor-wgui/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The web admin.  A small web server on a thread of its own that shows
//! how Conductor is doing, and is how the admin starts and stops the
//! server and shuts Conductor down.  The console is nothing but Scribe's
//! output now, so this is the only way in.
//!
//! It listens on 127.0.0.1 and nowhere else, so it can't be reached from
//! another machine, and it's plain HTTP.  HTTPS would mean a certificate,
//! and with nothing leaving the machine it would buy us nothing yet.  When
//! Security brings in TLS for the game, this can use it too.
//!
//! What it answers:
//!
//! - `GET /Opus` -- the page (`page.html`, baked in).  `/` sends you there.
//! - `POST /Opus/login` -- the login card's ask, `name = ...` and
//!   `password = ...` lines in the body.  Two accounts, `user` and
//!   `admin`, with their passwords in `wgui.cfg`.  A good one answers
//!   with a cookie; see `login.rs`.  Everything below needs that cookie,
//!   and answers 401 without it.  The routes that change something need
//!   `admin`, and answer 403 to `user`.
//! - `POST /Opus/logout` -- forgets the cookie's login.
//! - `GET /Opus/status?after=N` -- where the server is at, who's logged
//!   in, the monitor's latest look, the services, DiskMan's numbers,
//!   networking's door (every TCP connection of the last five minutes,
//!   every player in the world, and which access list is on), and
//!   Scribe's lines after line N, as JSON.  The page asks once a second.
//! - `GET /Opus/threads?pid=N` -- one process's threads, for when the admin
//!   clicks it on the System tab.  It only reads, like the status.
//! - `GET /Opus/notices` -- every open notice, for the Notifications
//!   History tab.  It only reads.
//! - `POST /Opus/notices/ack?id=N` -- clears one notice.
//! - `POST /Opus/notices/ack-all` -- clears every notice.
//! - `POST /Opus/notices/test` -- raises a test notice, to see the bell
//!   work.
//! - `POST /Opus/wwwhook/start`, `/stop`, `/restart` -- the Control Panel's
//!   buttons.  Each drops an ask in the server's mailbox (`server.rs` in
//!   conductor-tools) for the launcher to act on, and answers straight
//!   away.  Turned away with a 409 when it doesn't fit where the server is
//!   (a start while it's running, say).
//! - `GET /Opus/settings` -- every config file and every setting in it,
//!   for the Settings tab: what's running, what's waiting.  It only reads.
//! - `POST /Opus/wwwhook/settings/save?file=<name>` -- the Settings tab's
//!   SAVE.  The body is `key = value` lines in the file's own format, and
//!   Constellations checks every one before writing any: a bad line means
//!   nothing is written and the complaints come back, one per line, as
//!   JSON for the page to show beside the fields.  A good save goes to the
//!   file's `.wait4server` and takes at the file's reboot; nothing changes
//!   before then.
//! - `POST /Opus/wwwhook/settings/discard?file=<name>` -- throws that
//!   waiting file away.
//! - `POST /Opus/wwwhook/tcp/kick?id=N` -- the Connections tab's KICK on
//!   one connection, by its number in the status.  The connection is
//!   closed where it stands.  404 for a number that isn't open, 409 while
//!   the TCP side isn't listening.
//! - `GET /Opus/networking` -- both access lists, for the Whitelist and
//!   Blacklist tabs.  It only reads.
//! - `POST /Opus/wwwhook/networking/addip?list=<whitelist|blacklist>&entry=<address or range>`
//!   -- puts an address (`1.2.3.4`) or a range (`1.2.3.0/24`) on a list.
//!   It takes at once and the file is written behind it; a blacklisting
//!   while the blacklist is on kicks every connection and player from
//!   that address then and there.  400 for an entry that isn't one (the
//!   text says what's wrong), 409 while networking isn't running.
//! - `POST /Opus/wwwhook/networking/removeip?list=...&entry=...` -- takes
//!   one off, the same way.
//! - `POST /Opus/shutdown` -- shuts Conductor down.
//!
//! One request at a time, one per connection.  It's one admin with one
//! page, asking once a second, so there is nothing to gain from more.
//!
//! Listening on 127.0.0.1 keeps other machines out, but not other web
//! pages open in the admin's own browser.  Any site could have the browser
//! send a POST to 127.0.0.1:9996/Opus/shutdown.  Three checks stop that.
//! The `Host` header has to be this server's own address; the login's
//! cookie has to be there, and it's `SameSite=Strict`, which the browser
//! won't send from another site's page; and the login, the shutdown, the
//! server buttons and the ACKs have to carry an `X-Opus` header, which a
//! browser won't let another site's page add.

mod http;
mod json;
mod login;

use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

use conductor_tools::{constellations, diskman};
use conductor_tools::notices::{self, Level};
use conductor_tools::scribe::{self, Channel};
use conductor_tools::server::{self, Command};
use conductor_tools::services::{self, State};
use conductor_tools::threads;

use http::Request;
use login::{Login, Role};

/// The page, baked into Conductor so there's no file to lose.
const PAGE: &str = include_str!("page.html");

/// How long a connection gets to send its request, and to take the
/// answer.  A browser needs a few milliseconds.  This is so a connection
/// that sends nothing can't hold the server up forever.
const TIME_LIMIT: Duration = Duration::from_secs(2);

/// How many notices the bell shows at once.
const NEWEST_NOTICES: usize = 5;

static SERVER: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Starts listening on 127.0.0.1 at `port`, on a thread of its own.  False
/// if that didn't work (most likely something else already has the port),
/// and the log says why.
pub fn start(port: u16) -> bool {
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
        Ok(listener) => listener,
        Err(e) => {
            scribe::error_with(Channel::System, &e, &format!("THE WEB ADMIN CAN'T LISTEN ON 127.0.0.1:{port}.  \
                Something else probably has the port.  Stop it, or change wgui_port in conductor_globals.cfg."));
            services::set(services::WEB_ADMIN, State::Stopped, &format!("Can't listen on 127.0.0.1:{port}: {e}"));
            return false;
        }
    };

    match threads::spawn("wgui", move || serve(listener, port)) {
        Ok(handle) => {
            let mut guard = SERVER.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = Some(handle);
            scribe::info(Channel::System, &format!("The web admin is up at http://127.0.0.1:{port}/Opus"));
            services::set(services::WEB_ADMIN, State::Running, &format!("At http://127.0.0.1:{port}/Opus"));
            true
        }
        Err(e) => {
            scribe::error_with(Channel::System, &e, "The web admin couldn't start its thread.");
            services::set(services::WEB_ADMIN, State::Stopped, &format!("Couldn't start its thread: {e}"));
            false
        }
    }
}

/// True once the web admin has stopped: somebody pressed SHUT DOWN on the
/// page, or its thread died.  main asks this between commands from the
/// Control Panel, and shuts Conductor down when it says so.  The first
/// time it's true the thread is joined, and a death gets a line in the
/// log.
pub fn has_ended() -> bool {
    let mut guard = SERVER.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.as_ref().is_some_and(|handle| !handle.is_finished()) {
        return false;
    }
    if let Some(handle) = guard.take() {
        if handle.join().is_err() {
            scribe::error(Channel::System, "The web admin's thread died.  Conductor is shutting down, \
                since there is no other way in.");
        }
    }
    true
}

/// What to do after answering a request.
enum Next {
    KeepGoing,
    ShutDown,
}

/// The web admin's thread.  Takes connections one at a time until one of
/// them asks for a shutdown.
fn serve(listener: TcpListener, port: u16) {
    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(e) => {
                scribe::debug_with(Channel::System, &e, "The web admin couldn't take a connection.");
                continue;
            }
        };
        // If these fail, the connection just has no time limit.
        let _ = stream.set_read_timeout(Some(TIME_LIMIT));
        let _ = stream.set_write_timeout(Some(TIME_LIMIT));

        if let Next::ShutDown = handle(&mut stream, port) {
            break;
        }
    }
    scribe::info(Channel::System, "The web admin has stopped.");
    services::set(services::WEB_ADMIN, State::Stopped, "Shut down.");
}

/// Reads one request and answers it.
fn handle(stream: &mut TcpStream, port: u16) -> Next {
    let request = match http::read_request(stream) {
        Ok(Some(request)) => request,
        // Closed early, too big, or not HTTP.  Nothing worth answering.
        Ok(None) => return Next::KeepGoing,
        Err(e) => {
            scribe::debug_with(Channel::System, &e, "The web admin couldn't read a request.");
            return Next::KeepGoing;
        }
    };

    let (answer, next) = route(&request, port);
    if let Err(e) = http::respond(stream, answer.status, answer.content_type, &answer.extra, &answer.body) {
        // Most likely the browser gave up or the tab closed.
        scribe::debug_with(Channel::System, &e, &format!("The web admin couldn't answer {} {}.",
                                                        request.method,
                                                        request.path));
    }
    next
}

/// An answer, before it's sent.
struct Answer {
    status: &'static str,
    content_type: &'static str,
    extra: Vec<String>,
    body: Vec<u8>,
}

impl Answer {
    fn new(status: &'static str, content_type: &'static str, body: impl Into<Vec<u8>>) -> Answer {
        Answer { status, content_type, extra: Vec::new(), body: body.into() }
    }

    fn plain(status: &'static str, text: &str) -> Answer {
        Answer::new(status, "text/plain; charset=utf-8", text)
    }
}

/// Works out the answer to a request.
fn route(request: &Request, port: u16) -> (Answer, Next) {
    if !host_is_ours(request.header("host"), port) {
        return (Answer::plain("403 Forbidden", "That isn't this server's address."), Next::KeepGoing);
    }

    // The page and the login are open to anyone on this machine: the page
    // is only markup, and its login card needs somewhere to post.
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => {
            let mut answer = Answer::plain("303 See Other", "The web admin is at /Opus.");
            answer.extra.push("Location: /Opus".to_string());
            return (answer, Next::KeepGoing);
        }
        ("GET", "/Opus") | ("GET", "/Opus/") => {
            return (Answer::new("200 OK", "text/html; charset=utf-8", PAGE), Next::KeepGoing);
        }
        ("POST", "/Opus/login") => return log_in(request),
        _ => {}
    }

    // Everything else needs a login.  401 is what the page looks for: it
    // shows the login card and stops asking until somebody's in.
    let Some(role) = login::role_of(request) else {
        return (Answer::plain("401 Unauthorized", "Log in first."), Next::KeepGoing);
    };

    match (request.method.as_str(), request.path.as_str()) {
        ("POST", "/Opus/logout") => {
            if request.header("x-opus") != Some("login") {
                return (Answer::plain("403 Forbidden", "Log out from the page."), Next::KeepGoing);
            }
            login::log_out(request);
            let mut answer = Answer::new("200 OK", "application/json", "{\"logged_out\":true}");
            answer.extra.push(login::clear_cookie_line());
            (answer, Next::KeepGoing)
        }
        ("GET", "/Opus/status") => {
            let after = request.query_value("after").and_then(|after| after.parse().ok()).unwrap_or(0);
            let log_file = scribe::current_file();
            let (open_notices, newest_notices) = notices::newest(NEWEST_NOTICES);
            let body = json::status(&server::status(),
                                    role,
                                    conductor_monitor::latest().as_ref(),
                                    &services::list(),
                                    &diskman::status(),
                                    &conductor_networking::status(),
                                    open_notices,
                                    &newest_notices,
                                    &scribe::recent_lines(after),
                                    log_file.as_deref());
            (Answer::new("200 OK", "application/json", body), Next::KeepGoing)
        }
        ("GET", "/Opus/threads") => {
            let Some(pid) = request.query_value("pid").and_then(|pid| pid.parse::<u32>().ok()) else {
                return (Answer::plain("400 Bad Request", "Which process?  /Opus/threads?pid=N"), Next::KeepGoing);
            };
            let threads = conductor_monitor::probe::threads_of(pid);
            (Answer::new("200 OK", "application/json", json::threads_of(pid, threads.as_deref())), Next::KeepGoing)
        }
        ("GET", "/Opus/notices") => {
            (Answer::new("200 OK", "application/json", json::notices(&notices::all())), Next::KeepGoing)
        }
        ("POST", "/Opus/notices/ack") => {
            if let Some(turned_away) = only_admin(role) {
                return (turned_away, Next::KeepGoing);
            }
            if request.header("x-opus") != Some("ack") {
                return (Answer::plain("403 Forbidden", "ACK from the page."), Next::KeepGoing);
            }
            let Some(id) = request.query_value("id").and_then(|id| id.parse::<u64>().ok()) else {
                return (Answer::plain("400 Bad Request", "Which notice?  /Opus/notices/ack?id=N"), Next::KeepGoing);
            };
            // Already gone (ACKed from another browser, say) is fine too:
            // either way it isn't open any more.
            let cleared = notices::ack(id);
            scribe::debug(Channel::System, &format!("Notice {id} ACKed from the web admin."));
            (Answer::new("200 OK", "application/json", format!("{{\"cleared\":{cleared}}}")), Next::KeepGoing)
        }
        ("POST", "/Opus/notices/ack-all") => {
            if let Some(turned_away) = only_admin(role) {
                return (turned_away, Next::KeepGoing);
            }
            if request.header("x-opus") != Some("ack") {
                return (Answer::plain("403 Forbidden", "ACK from the page."), Next::KeepGoing);
            }
            let cleared = notices::ack_all();
            scribe::debug(Channel::System, &format!("All {cleared} notice(s) ACKed from the web admin."));
            (Answer::new("200 OK", "application/json", format!("{{\"cleared\":{cleared}}}")), Next::KeepGoing)
        }
        ("POST", "/Opus/notices/test") => {
            if let Some(turned_away) = only_admin(role) {
                return (turned_away, Next::KeepGoing);
            }
            if request.header("x-opus") != Some("ack") {
                return (Answer::plain("403 Forbidden", "Test from the page."), Next::KeepGoing);
            }
            let id = notices::publish(Level::Notice, "Web admin", "Test notification from the web admin.");
            (Answer::new("200 OK", "application/json", format!("{{\"id\":{id}}}")), Next::KeepGoing)
        }
        ("GET", "/Opus/settings") => {
            (Answer::new("200 OK", "application/json", json::settings(&settings_states())), Next::KeepGoing)
        }
        ("POST", "/Opus/wwwhook/settings/save") => settings_save(request, role),
        ("POST", "/Opus/wwwhook/settings/discard") => settings_discard(request, role),
        ("POST", "/Opus/wwwhook/tcp/kick") => tcp_kick(request, role),
        ("GET", "/Opus/networking") => {
            (Answer::new("200 OK", "application/json", json::access(conductor_networking::access_lists().as_ref())),
             Next::KeepGoing)
        }
        ("POST", "/Opus/wwwhook/networking/addip") => networking_list(request, role, ListChange::Add),
        ("POST", "/Opus/wwwhook/networking/removeip") => networking_list(request, role, ListChange::Remove),
        ("POST", "/Opus/wwwhook/start") => server_command(request, role, Command::Start),
        ("POST", "/Opus/wwwhook/stop") => server_command(request, role, Command::Stop),
        ("POST", "/Opus/wwwhook/restart") => server_command(request, role, Command::Restart),
        ("POST", "/Opus/shutdown") => {
            if let Some(turned_away) = only_admin(role) {
                return (turned_away, Next::KeepGoing);
            }
            if request.header("x-opus") != Some("shut-down") {
                scribe::warn(Channel::System, "The web admin turned away a shutdown that didn't come from its \
                    own page.");
                return (Answer::plain("403 Forbidden", "Shut down from the page."), Next::KeepGoing);
            }
            scribe::info(Channel::System, "Shut down from the web admin.");
            (Answer::new("200 OK", "application/json", "{\"shutting_down\":true}"), Next::ShutDown)
        }
        (_, "/") | (_, "/Opus") | (_, "/Opus/") | (_, "/Opus/login") | (_, "/Opus/logout") | (_, "/Opus/status")
        | (_, "/Opus/threads") | (_, "/Opus/notices") | (_, "/Opus/notices/ack") | (_, "/Opus/notices/ack-all")
        | (_, "/Opus/notices/test") | (_, "/Opus/wwwhook/start") | (_, "/Opus/wwwhook/stop")
        | (_, "/Opus/wwwhook/restart") | (_, "/Opus/settings") | (_, "/Opus/wwwhook/settings/save")
        | (_, "/Opus/wwwhook/settings/discard") | (_, "/Opus/wwwhook/tcp/kick") | (_, "/Opus/networking")
        | (_, "/Opus/wwwhook/networking/addip") | (_, "/Opus/wwwhook/networking/removeip") | (_, "/Opus/shutdown") => {
            (Answer::plain("405 Method Not Allowed", "Not like that."), Next::KeepGoing)
        }
        _ => (Answer::plain("404 Not Found", "There's nothing here."), Next::KeepGoing),
    }
}

/// The login card's ask.  A good name and password get a cookie and the
/// same `login` object the status carries; a wrong one gets a 403 that
/// doesn't say which half was wrong.
fn log_in(request: &Request) -> (Answer, Next) {
    if request.header("x-opus") != Some("login") {
        return (Answer::plain("403 Forbidden", "Log in from the page."), Next::KeepGoing);
    }
    let Some((name, password)) = login::fields(&request.body) else {
        return (Answer::plain("400 Bad Request", "Send a name = ... line and a password = ... line."),
                Next::KeepGoing);
    };
    match login::log_in(&name, &password) {
        Login::In { token, role } => {
            let mut answer = Answer::new("200 OK", "application/json", json::login(role));
            answer.extra.push(login::cookie_line(&token));
            (answer, Next::KeepGoing)
        }
        Login::Wrong => (Answer::plain("403 Forbidden", "Wrong name or password."), Next::KeepGoing),
        Login::NoToken(e) => (Answer::plain("500 Internal Server Error", &format!("Conductor couldn't make a login \
            token: {e}.  Nobody can log in until the OS gives out random bytes again.")), Next::KeepGoing),
    }
}

/// The answer that turns `user` away from a route that changes something,
/// or `None` for `admin`.
fn only_admin(role: Role) -> Option<Answer> {
    if role.can_change() {
        None
    } else {
        Some(Answer::plain("403 Forbidden", "Only admin can do that.  You're logged in as user, who can look \
            and not touch."))
    }
}

/// START, STOP and RESTART SERVER from the Control Panel.  The launcher
/// does the work; this only checks the ask came from the page, from
/// admin, and fits where the server is right now.  The answer is sent
/// before anything starts or stops, and the page sees it happen through
/// the status.
fn server_command(request: &Request, role: Role, command: Command) -> (Answer, Next) {
    if let Some(turned_away) = only_admin(role) {
        return (turned_away, Next::KeepGoing);
    }
    if request.header("x-opus") != Some("server") {
        scribe::warn(Channel::System, "The web admin turned away a server command that didn't come from its \
            own page.");
        return (Answer::plain("403 Forbidden", "Start and stop the server from the page."), Next::KeepGoing);
    }

    let word = match command {
        Command::Start => "start",
        Command::Stop => "stop",
        Command::Restart => "restart",
    };
    match server::ask(command) {
        Ok(()) => {
            scribe::info(Channel::System, &format!("Asked to {word} the server from the web admin."));
            (Answer::new("200 OK", "application/json", format!("{{\"asked\":\"{word}\"}}")), Next::KeepGoing)
        }
        Err(state) => {
            scribe::debug(Channel::System, &format!("The web admin was asked to {word} the server while it's \
                {state}.  Not now."));
            (Answer::plain("409 Conflict", &format!("Not now.  The server is {state}.")), Next::KeepGoing)
        }
    }
}

/// KICK on the TCP tab: closes one connection at the door, by its number
/// in the status.  Admin only, and from the page only.
fn tcp_kick(request: &Request, role: Role) -> (Answer, Next) {
    if let Some(turned_away) = only_admin(role) {
        return (turned_away, Next::KeepGoing);
    }
    if request.header("x-opus") != Some("tcp") {
        scribe::warn(Channel::System, "The web admin turned away a kick that didn't come from its own page.");
        return (Answer::plain("403 Forbidden", "Kick from the page."), Next::KeepGoing);
    }
    let Some(id) = request.query_value("id").and_then(|id| id.parse::<u64>().ok()) else {
        return (Answer::plain("400 Bad Request", "Which connection?  /Opus/wwwhook/tcp/kick?id=N"), Next::KeepGoing);
    };
    match conductor_networking::kick(id) {
        conductor_networking::Kicked::Yes => {
            (Answer::new("200 OK", "application/json", format!("{{\"kicked\":{id}}}")), Next::KeepGoing)
        }
        conductor_networking::Kicked::NotOpen => {
            (Answer::plain("404 Not Found", "That connection isn't open any more."), Next::KeepGoing)
        }
        conductor_networking::Kicked::NotListening => {
            (Answer::plain("409 Conflict", "The TCP side isn't listening."), Next::KeepGoing)
        }
    }
}

/// Which way a list is being changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListChange {
    Add,
    Remove,
}

/// The Whitelist and Blacklist tabs' ADD and REMOVE, and the Connections
/// tab's menu: `?list=whitelist|blacklist&entry=<address or range>`.
/// The entry is checked here first, so a typo is a 400 with the reason
/// in words, before networking is asked anything.
fn networking_list(request: &Request, role: Role, change: ListChange) -> (Answer, Next) {
    if let Some(turned_away) = only_admin(role) {
        return (turned_away, Next::KeepGoing);
    }
    if request.header("x-opus") != Some("networking") {
        scribe::warn(Channel::System, "The web admin turned away a list change that didn't come from its own page.");
        return (Answer::plain("403 Forbidden", "Change the lists from the page."), Next::KeepGoing);
    }
    let Some(list) = request.query_value("list").and_then(conductor_networking::List::parse) else {
        return (Answer::plain("400 Bad Request", "Which list?  ?list=whitelist or ?list=blacklist"), Next::KeepGoing);
    };
    let entry_text = unescape(request.query_value("entry").unwrap_or(""));
    let entry = match conductor_networking::Entry::parse(&entry_text) {
        Ok(entry) => entry,
        Err(why) => return (Answer::plain("400 Bad Request", &why), Next::KeepGoing),
    };
    let answer = match change {
        ListChange::Add => conductor_networking::list_address(list, entry).map(|listed| json::listed(&listed)),
        ListChange::Remove => {
            conductor_networking::unlist_address(list, entry).map(|unlisted| json::unlisted(&unlisted))
        }
    };
    match answer {
        Ok(body) => (Answer::new("200 OK", "application/json", body), Next::KeepGoing),
        Err(why) => (Answer::plain("409 Conflict", &why), Next::KeepGoing),
    }
}

/// A query value with its `%XX` escapes undone and `+` read as a space,
/// which is how the page sends a range's slash.  Anything that isn't a
/// proper escape is kept as it is.
fn unescape(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        if bytes[i] == b'%' && i + 3 <= bytes.len() {
            let pair = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(pair, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Every config file as the Settings tab sees it.  A file that hasn't
/// been loaded this run is shown as it sits on disk, since that is what
/// its next load reads; if there's no file either, the defaults, which is
/// what the load would write.
fn settings_states() -> Vec<json::FileState> {
    constellations::FILES.iter().map(|&file| {
        let loaded = constellations::is_loaded(file);
        let running = if loaded {
            constellations::values(file)
        } else {
            constellations::file_values(file).unwrap_or_else(|| constellations::values(file))
        };
        json::FileState { file, loaded, running, waiting: constellations::waiting(file) }
    }).collect()
}

/// The checks every settings route makes: the ask came from the page and
/// from admin, and names a file in the table.  The file on success, the
/// answer to send on a failure.
fn settings_file(request: &Request, role: Role) -> Result<&'static constellations::ConfigFile, Answer> {
    if let Some(turned_away) = only_admin(role) {
        return Err(turned_away);
    }
    if request.header("x-opus") != Some("settings") {
        return Err(Answer::plain("403 Forbidden", "Change settings from the page."));
    }
    request.query_value("file")
        .and_then(constellations::file_named)
        .ok_or_else(|| Answer::plain("404 Not Found", "Which file?  ?file=<name>, one from the Settings tab."))
}

/// SAVE on the Settings tab.  Constellations does the checking and the
/// writing; a complaint about a line comes back as a 400 with the
/// complaints as JSON, and a disk failure as a 500 the same way.
fn settings_save(request: &Request, role: Role) -> (Answer, Next) {
    let file = match settings_file(request, role) {
        Ok(file) => file,
        Err(answer) => return (answer, Next::KeepGoing),
    };
    match constellations::save_waiting(file, &request.body) {
        Ok(()) => {
            scribe::debug(Channel::System, &format!("{} saved from the web admin.", file.name));
            (Answer::new("200 OK", "application/json", "{\"saved\":true}"), Next::KeepGoing)
        }
        Err(problems) => {
            // Every complaint about a line starts with its number.  One
            // that doesn't is the disk saying no.
            let status = if problems.iter().all(|problem| problem.starts_with("line ")) {
                "400 Bad Request"
            } else {
                "500 Internal Server Error"
            };
            (Answer::new(status, "application/json", json::problems(&problems)), Next::KeepGoing)
        }
    }
}

/// DISCARD on the Settings tab.  Nothing waiting is fine too: it's gone
/// either way.
fn settings_discard(request: &Request, role: Role) -> (Answer, Next) {
    let file = match settings_file(request, role) {
        Ok(file) => file,
        Err(answer) => return (answer, Next::KeepGoing),
    };
    match constellations::discard_waiting(file) {
        Ok(()) => (Answer::new("200 OK", "application/json", "{\"discarded\":true}"), Next::KeepGoing),
        Err(e) if e.is_not_found() => {
            (Answer::new("200 OK", "application/json", "{\"discarded\":false}"), Next::KeepGoing)
        }
        Err(e) => (Answer::plain("500 Internal Server Error", &format!("Couldn't discard it: {e}")), Next::KeepGoing),
    }
}

/// True if the browser was asking for this server by its own address.
/// A page on some other site that points its own name at 127.0.0.1 would
/// send that name instead, and gets turned away.
fn host_is_ours(host: Option<&str>, port: u16) -> bool {
    let Some(host) = host else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    host == format!("127.0.0.1:{port}") || host == format!("localhost:{port}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(method: &str, path: &str, headers: &[(&str, &str)]) -> Request {
        Request {
            method: method.to_string(),
            path: path.to_string(),
            query: String::new(),
            headers: headers.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect(),
            body: String::new(),
        }
    }

    const HOST: (&str, &str) = ("host", "127.0.0.1:9996");

    /// Logs `name` in (the passwords are the defaults, since nothing in
    /// the tests loads wgui.cfg) and hands back the `Cookie` header's
    /// value.
    fn cookie_for(name: &str) -> String {
        let Login::In { token, .. } = login::log_in(name, name) else {
            panic!("{name} / {name} is the default");
        };
        format!("opus_session={token}")
    }

    #[test]
    fn only_our_own_address_gets_in() {
        assert!(host_is_ours(Some("127.0.0.1:9996"), 9996));
        assert!(host_is_ours(Some("LOCALHOST:9996"), 9996));
        assert!(!host_is_ours(Some("evil.example:9996"), 9996));
        assert!(!host_is_ours(Some("127.0.0.1:9997"), 9996));
        assert!(!host_is_ours(None, 9996));

        let (answer, _) = route(&request("GET", "/Opus", &[("host", "evil.example:9996")]), 9996);
        assert_eq!(answer.status, "403 Forbidden");
    }

    #[test]
    fn the_page_is_open_and_everything_else_needs_a_login() {
        let (answer, _) = route(&request("GET", "/Opus", &[HOST]), 9996);
        assert_eq!(answer.status, "200 OK");
        assert!(answer.content_type.starts_with("text/html"));

        let (answer, _) = route(&request("GET", "/", &[HOST]), 9996);
        assert_eq!(answer.status, "303 See Other");
        assert_eq!(answer.extra, vec!["Location: /Opus".to_string()]);

        for (method, path) in [("GET", "/Opus/status"), ("GET", "/Opus/notices"), ("POST", "/Opus/shutdown"),
                               ("POST", "/Opus/wwwhook/start"), ("POST", "/Opus/logout"), ("GET", "/nope")] {
            let (answer, next) = route(&request(method, path, &[HOST, ("x-opus", "shut-down")]), 9996);
            assert_eq!(answer.status, "401 Unauthorized", "{method} {path}");
            assert!(matches!(next, Next::KeepGoing));
        }
        let (answer, _) = route(&request("GET", "/Opus/status", &[HOST, ("cookie", "opus_session=made-up")]), 9996);
        assert_eq!(answer.status, "401 Unauthorized");
    }

    #[test]
    fn a_login_gets_a_cookie_and_a_logout_takes_it_back() {
        let mut asking = request("POST", "/Opus/login", &[HOST, ("x-opus", "login")]);
        asking.body = "name = admin\npassword = admin\n".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "200 OK");
        assert_eq!(answer.body, b"{\"name\":\"admin\",\"can_change\":true}");
        assert_eq!(answer.extra.len(), 1);
        let cookie = answer.extra[0].trim_start_matches("Set-Cookie: ").split(';').next().unwrap().to_string();
        assert!(cookie.starts_with("opus_session="));

        let (answer, _) = route(&request("GET", "/Opus/status", &[HOST, ("cookie", cookie.as_str())]), 9996);
        assert_eq!(answer.status, "200 OK");
        assert!(answer.body.starts_with(b"{\"server\":{\"state\":\""));
        assert!(String::from_utf8_lossy(&answer.body).contains("\"login\":{\"name\":\"admin\",\"can_change\":true}"));

        let (answer, _) = route(&request("POST", "/Opus/logout", &[HOST, ("cookie", cookie.as_str())]), 9996);
        assert_eq!(answer.status, "403 Forbidden");
        let asking = request("POST", "/Opus/logout", &[HOST, ("cookie", cookie.as_str()), ("x-opus", "login")]);
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "200 OK");
        assert!(answer.extra[0].contains("Max-Age=0"));
        let (answer, _) = route(&request("GET", "/Opus/status", &[HOST, ("cookie", cookie.as_str())]), 9996);
        assert_eq!(answer.status, "401 Unauthorized");
    }

    #[test]
    fn a_wrong_login_is_turned_away_without_saying_which_half() {
        let mut asking = request("POST", "/Opus/login", &[HOST, ("x-opus", "login")]);
        asking.body = "name = admin\npassword = nope\n".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "403 Forbidden");
        assert!(answer.extra.is_empty());
        let wrong_password = answer.body;

        asking.body = "name = nobody\npassword = admin\n".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "403 Forbidden");
        assert_eq!(answer.body, wrong_password);

        asking.body = "just some text".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "400 Bad Request");

        // Without the page's header the body isn't even looked at.
        let mut asking = request("POST", "/Opus/login", &[HOST]);
        asking.body = "name = admin\npassword = admin\n".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "403 Forbidden");

        let (answer, _) = route(&request("GET", "/Opus/login", &[HOST, ("x-opus", "login")]), 9996);
        assert_eq!(answer.status, "401 Unauthorized");
    }

    #[test]
    fn user_can_look_and_not_touch() {
        let user = cookie_for("user");
        let (answer, _) = route(&request("GET", "/Opus/status", &[HOST, ("cookie", user.as_str())]), 9996);
        assert_eq!(answer.status, "200 OK");
        assert!(String::from_utf8_lossy(&answer.body).contains("\"login\":{\"name\":\"user\",\"can_change\":false}"));
        let (answer, _) = route(&request("GET", "/Opus/notices", &[HOST, ("cookie", user.as_str())]), 9996);
        assert_eq!(answer.status, "200 OK");

        let id = notices::publish(Level::Notice, "Test", "user can't ack me");
        for (path, header) in [("/Opus/notices/ack", "ack"), ("/Opus/notices/ack-all", "ack"),
                               ("/Opus/notices/test", "ack"), ("/Opus/wwwhook/start", "server"),
                               ("/Opus/wwwhook/stop", "server"), ("/Opus/wwwhook/restart", "server"),
                               ("/Opus/wwwhook/tcp/kick", "tcp"), ("/Opus/wwwhook/networking/addip", "networking"),
                               ("/Opus/wwwhook/networking/removeip", "networking"), ("/Opus/shutdown", "shut-down")] {
            let mut asking = request("POST", path, &[HOST, ("cookie", user.as_str()), ("x-opus", header)]);
            asking.query = format!("id={id}&list=blacklist&entry=1.2.3.4");
            let (answer, next) = route(&asking, 9996);
            assert_eq!(answer.status, "403 Forbidden", "{path}");
            assert!(String::from_utf8_lossy(&answer.body).starts_with("Only admin"), "{path}");
            assert!(matches!(next, Next::KeepGoing), "{path}");
        }
        assert!(notices::ack(id));
    }

    #[test]
    fn the_settings_are_read_by_anyone_logged_in_and_changed_only_by_admin() {
        let user = cookie_for("user");
        let (answer, _) = route(&request("GET", "/Opus/settings", &[HOST, ("cookie", user.as_str())]), 9996);
        assert_eq!(answer.status, "200 OK");
        let body = String::from_utf8_lossy(&answer.body).into_owned();
        assert!(body.starts_with("{\"files\":[{\"name\":\"conductor_globals.cfg\","), "{body}");
        assert!(body.contains("{\"name\":\"wgui.cfg\","));
        assert!(body.contains("{\"name\":\"postgres.cfg\","));
        // Nothing is loaded in the tests, and DiskMan isn't running, so
        // every file shows its defaults and nothing waits.
        assert!(body.contains("\"loaded\":false,\"waiting\":false,"));
        assert!(!body.contains("\"loaded\":true"));

        for path in ["/Opus/wwwhook/settings/save", "/Opus/wwwhook/settings/discard"] {
            let mut asking = request("POST", path, &[HOST, ("cookie", user.as_str()), ("x-opus", "settings")]);
            asking.query = "file=wgui.cfg".to_string();
            asking.body = "user_password = other\n".to_string();
            let (answer, _) = route(&asking, 9996);
            assert_eq!(answer.status, "403 Forbidden", "{path}");
            assert!(String::from_utf8_lossy(&answer.body).starts_with("Only admin"), "{path}");
        }
    }

    #[test]
    fn a_save_checks_every_line_before_writing_any() {
        let admin = cookie_for("admin");

        // Without the page's header, or a file we know, nothing is looked at.
        let mut asking = request("POST", "/Opus/wwwhook/settings/save", &[HOST, ("cookie", admin.as_str())]);
        asking.query = "file=wgui.cfg".to_string();
        asking.body = "user_password = other\n".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "403 Forbidden");

        let mut asking = request("POST", "/Opus/wwwhook/settings/save",
                                 &[HOST, ("cookie", admin.as_str()), ("x-opus", "settings")]);
        asking.query = "file=nope.cfg".to_string();
        asking.body = "user_password = other\n".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "404 Not Found");
        asking.query = String::new();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "404 Not Found");

        // A bad line: the complaints come back, one per line, as JSON.
        asking.query = "file=conductor_globals.cfg".to_string();
        asking.body = "scribe_log_dir = logs\nwgui_port = seventy\nnope = 1\n".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "400 Bad Request");
        assert_eq!(answer.content_type, "application/json");
        let body = String::from_utf8_lossy(&answer.body).into_owned();
        assert!(body.starts_with("{\"problems\":[\"line 2: wgui_port is \\\"seventy\\\","), "{body}");
        assert!(body.contains("\"line 3: There is no setting called nope."), "{body}");

        // Good lines, but DiskMan isn't running in the tests, so the
        // write fails: that's the disk saying no, not a bad line.
        asking.body = "scribe_log_dir = logs\nwgui_port = 9997\n".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "500 Internal Server Error");
        assert!(String::from_utf8_lossy(&answer.body).starts_with("{\"problems\":[\"Couldn't write"));

        // A GET is the wrong way round.
        let asking = request("GET", "/Opus/wwwhook/settings/save", &[HOST, ("cookie", admin.as_str())]);
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "405 Method Not Allowed");
    }

    #[test]
    fn a_kick_needs_the_page_header_a_number_and_a_listening_door() {
        let admin = cookie_for("admin");
        let mut asking = request("POST", "/Opus/wwwhook/tcp/kick", &[HOST, ("cookie", admin.as_str())]);
        asking.query = "id=1".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "403 Forbidden");

        let mut asking = request("POST", "/Opus/wwwhook/tcp/kick", &[HOST, ("cookie", admin.as_str()),
                                                                   ("x-opus", "tcp")]);
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "400 Bad Request");

        // Nothing is listening in a test, so no connection can be kicked.
        asking.query = "id=1".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "409 Conflict");

        let (answer, _) = route(&request("GET", "/Opus/wwwhook/tcp/kick", &[HOST, ("cookie", admin.as_str())]), 9996);
        assert_eq!(answer.status, "405 Method Not Allowed");
    }

    #[test]
    fn a_list_change_needs_the_page_header_a_list_an_entry_and_a_running_server() {
        let admin = cookie_for("admin");
        let user = cookie_for("user");

        // The lists can be read by anyone logged in.  Nothing is running
        // in a test, so they aren't loaded.
        let (answer, _) = route(&request("GET", "/Opus/networking", &[HOST, ("cookie", user.as_str())]), 9996);
        assert_eq!(answer.status, "200 OK");
        assert_eq!(String::from_utf8_lossy(&answer.body),
                   "{\"running\":false,\"mode\":\"off\",\"whitelist\":[],\"blacklist\":[]}");

        for path in ["/Opus/wwwhook/networking/addip", "/Opus/wwwhook/networking/removeip"] {
            // Without the page's header, nothing is looked at.
            let mut asking = request("POST", path, &[HOST, ("cookie", admin.as_str())]);
            asking.query = "list=blacklist&entry=1.2.3.4".to_string();
            let (answer, _) = route(&asking, 9996);
            assert_eq!(answer.status, "403 Forbidden", "{path}");

            let mut asking = request("POST", path, &[HOST, ("cookie", admin.as_str()), ("x-opus", "networking")]);
            let (answer, _) = route(&asking, 9996);
            assert_eq!(answer.status, "400 Bad Request", "{path}");
            assert!(String::from_utf8_lossy(&answer.body).starts_with("Which list?"), "{path}");

            asking.query = "list=greylist&entry=1.2.3.4".to_string();
            let (answer, _) = route(&asking, 9996);
            assert_eq!(answer.status, "400 Bad Request", "{path}");

            asking.query = "list=blacklist&entry=potato".to_string();
            let (answer, _) = route(&asking, 9996);
            assert_eq!(answer.status, "400 Bad Request", "{path}");
            assert!(String::from_utf8_lossy(&answer.body).starts_with("\"potato\" isn't an address"), "{path}");

            asking.query = "list=blacklist&entry=".to_string();
            let (answer, _) = route(&asking, 9996);
            assert_eq!(answer.status, "400 Bad Request", "{path}");

            // A good entry, a range with its slash escaped the way the
            // page sends it, but no server running to change.
            asking.query = "list=blacklist&entry=1.2.3.0%2F24".to_string();
            let (answer, _) = route(&asking, 9996);
            assert_eq!(answer.status, "409 Conflict", "{path}");
            assert!(String::from_utf8_lossy(&answer.body).starts_with("Networking isn't running"), "{path}");

            let (answer, _) = route(&request("GET", path, &[HOST, ("cookie", admin.as_str())]), 9996);
            assert_eq!(answer.status, "405 Method Not Allowed", "{path}");
        }
    }

    #[test]
    fn a_query_value_comes_unescaped() {
        assert_eq!(unescape("1.2.3.0%2F24"), "1.2.3.0/24");
        assert_eq!(unescape("1.2.3.0%2f24"), "1.2.3.0/24");
        assert_eq!(unescape("2001%3Adb8%3A%3A%2F32"), "2001:db8::/32");
        assert_eq!(unescape("a+b"), "a b");
        assert_eq!(unescape("plain"), "plain");
        assert_eq!(unescape(""), "");
        // A stray percent, or one with no room for two digits, stays.
        assert_eq!(unescape("100%"), "100%");
        assert_eq!(unescape("%2"), "%2");
        assert_eq!(unescape("%zz"), "%zz");
        assert_eq!(unescape("%%2F"), "%/");
    }

    #[test]
    fn a_process_is_asked_for_by_its_number() {
        let admin = cookie_for("admin");
        let mut asking = request("GET", "/Opus/threads", &[HOST, ("cookie", admin.as_str())]);
        asking.query = format!("pid={}", std::process::id());
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "200 OK");
        assert!(answer.body.starts_with(format!("{{\"pid\":{},", std::process::id()).as_bytes()));

        asking.query = "pid=nope".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "400 Bad Request");
    }

    #[test]
    fn an_ack_needs_the_page_header_and_clears_the_notice() {
        let admin = cookie_for("admin");
        let id = notices::publish(Level::Notice, "Test", "ack me");

        let mut asking = request("POST", "/Opus/notices/ack", &[HOST, ("cookie", admin.as_str())]);
        asking.query = format!("id={id}");
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "403 Forbidden");
        assert!(notices::all().iter().any(|notice| notice.id == id));

        let mut asking = request("POST", "/Opus/notices/ack", &[HOST, ("cookie", admin.as_str()), ("x-opus", "ack")]);
        asking.query = format!("id={id}");
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "200 OK");
        assert!(!notices::all().iter().any(|notice| notice.id == id));
    }

    #[test]
    fn the_test_button_raises_a_notice() {
        let admin = cookie_for("admin");
        let (answer, _) = route(&request("POST", "/Opus/notices/test", &[HOST, ("cookie", admin.as_str())]), 9996);
        assert_eq!(answer.status, "403 Forbidden");

        let asking = request("POST", "/Opus/notices/test", &[HOST, ("cookie", admin.as_str()), ("x-opus", "ack")]);
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "200 OK");
        let id: u64 = String::from_utf8_lossy(&answer.body)
            .trim_start_matches("{\"id\":")
            .trim_end_matches('}')
            .parse()
            .expect("the answer should carry the new notice's id");
        assert!(notices::ack(id));
    }

    // Only the turned-away paths here.  The switch is one for the whole
    // program, and server.rs's own test walks it through its states.
    #[test]
    fn a_server_command_needs_the_page_header_and_a_post() {
        let admin = cookie_for("admin");
        let (answer, next) = route(&request("POST", "/Opus/wwwhook/start", &[HOST, ("cookie", admin.as_str())]), 9996);
        assert_eq!(answer.status, "403 Forbidden");
        assert!(matches!(next, Next::KeepGoing));

        let asking = request("GET", "/Opus/wwwhook/stop", &[HOST, ("cookie", admin.as_str()), ("x-opus", "server")]);
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "405 Method Not Allowed");

        let asking = request("POST", "/Opus/wwwhook/dance", &[HOST, ("cookie", admin.as_str()), ("x-opus", "server")]);
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "404 Not Found");
    }

    #[test]
    fn a_shutdown_needs_the_page_header_and_a_post() {
        let admin = cookie_for("admin");
        let (answer, next) = route(&request("POST", "/Opus/shutdown", &[HOST, ("cookie", admin.as_str())]), 9996);
        assert_eq!(answer.status, "403 Forbidden");
        assert!(matches!(next, Next::KeepGoing));

        let asking = request("GET", "/Opus/shutdown", &[HOST, ("cookie", admin.as_str()), ("x-opus", "shut-down")]);
        let (answer, next) = route(&asking, 9996);
        assert_eq!(answer.status, "405 Method Not Allowed");
        assert!(matches!(next, Next::KeepGoing));

        let asking = request("POST", "/Opus/shutdown", &[HOST, ("cookie", admin.as_str()), ("x-opus", "shut-down")]);
        let (answer, next) = route(&asking, 9996);
        assert_eq!(answer.status, "200 OK");
        assert!(matches!(next, Next::ShutDown));
    }
}
