//! File:       Opus/Conductor/dev/conductor-wgui/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The web admin.  A small web server on a thread of its own that shows
//! how Conductor is doing and is how the admin shuts it down.  The console
//! is nothing but Scribe's output now, so this is the only way in.
//!
//! It listens on 127.0.0.1 and nowhere else, so it can't be reached from
//! another machine, and it's plain HTTP.  HTTPS would mean a certificate,
//! and with nothing leaving the machine it would buy us nothing yet.  When
//! Security brings in TLS for the game, this can use it too.
//!
//! What it answers:
//!
//! - `GET /Opus` -- the page (`page.html`, baked in).  `/` sends you there.
//! - `GET /Opus/status?after=N` -- the monitor's latest look, the services,
//!   DiskMan's numbers, and Scribe's lines after line N, as JSON.  The page
//!   asks once a second.
//! - `GET /Opus/threads?pid=N` -- one process's threads, for when the admin
//!   clicks it on the System tab.  It only reads, like the status.
//! - `POST /Opus/shutdown` -- shuts Conductor down.
//!
//! One request at a time, one per connection.  It's one admin with one
//! page, asking once a second, so there is nothing to gain from more.
//!
//! Listening on 127.0.0.1 keeps other machines out, but not other web
//! pages open in the admin's own browser.  Any site could have the browser
//! send a POST to 127.0.0.1:9996/Opus/shutdown.  Two checks stop that.
//! The `Host` header has to be this server's own address, and the shutdown
//! has to carry an `X-Opus` header, which a browser won't let another
//! site's page add.

mod http;
mod json;

use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

use conductor_tools::diskman;
use conductor_tools::scribe::{self, Channel};
use conductor_tools::services::{self, State};
use conductor_tools::threads;

use http::Request;

/// The page, baked into Conductor so there's no file to lose.
const PAGE: &str = include_str!("page.html");

/// How long a connection gets to send its request, and to take the
/// answer.  A browser needs a few milliseconds.  This is so a connection
/// that sends nothing can't hold the server up forever.
const TIME_LIMIT: Duration = Duration::from_secs(2);

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

/// Waits until the web admin stops, which is when somebody presses Shut
/// Down on the page.  main sits here for as long as Conductor runs.
pub fn wait() {
    let handle = {
        let mut guard = SERVER.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.take()
    };
    if let Some(handle) = handle {
        if handle.join().is_err() {
            scribe::error(Channel::System, "The web admin's thread died.  Conductor is shutting down, \
                since there is no other way in.");
        }
    }
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

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => {
            let mut answer = Answer::plain("303 See Other", "The web admin is at /Opus.");
            answer.extra.push("Location: /Opus".to_string());
            (answer, Next::KeepGoing)
        }
        ("GET", "/Opus") | ("GET", "/Opus/") => {
            (Answer::new("200 OK", "text/html; charset=utf-8", PAGE), Next::KeepGoing)
        }
        ("GET", "/Opus/status") => {
            let after = request.query_value("after").and_then(|after| after.parse().ok()).unwrap_or(0);
            let log_file = scribe::current_file();
            let body = json::status(conductor_monitor::latest().as_ref(),
                                    &services::list(),
                                    &diskman::status(),
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
        ("POST", "/Opus/shutdown") => {
            if request.header("x-opus") != Some("shut-down") {
                scribe::warn(Channel::System, "The web admin turned away a shutdown that didn't come from its \
                    own page.");
                return (Answer::plain("403 Forbidden", "Shut down from the page."), Next::KeepGoing);
            }
            scribe::info(Channel::System, "Shut down from the web admin.");
            (Answer::new("200 OK", "application/json", "{\"shutting_down\":true}"), Next::ShutDown)
        }
        (_, "/") | (_, "/Opus") | (_, "/Opus/") | (_, "/Opus/status") | (_, "/Opus/threads")
        | (_, "/Opus/shutdown") => {
            (Answer::plain("405 Method Not Allowed", "Not like that."), Next::KeepGoing)
        }
        _ => (Answer::plain("404 Not Found", "There's nothing here."), Next::KeepGoing),
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
        }
    }

    const HOST: (&str, &str) = ("host", "127.0.0.1:9996");

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
    fn the_page_and_the_status_are_there() {
        let (answer, _) = route(&request("GET", "/Opus", &[HOST]), 9996);
        assert_eq!(answer.status, "200 OK");
        assert!(answer.content_type.starts_with("text/html"));

        let (answer, _) = route(&request("GET", "/Opus/status", &[HOST]), 9996);
        assert_eq!(answer.status, "200 OK");
        assert!(answer.body.starts_with(b"{\"monitor\":"));

        let (answer, _) = route(&request("GET", "/", &[HOST]), 9996);
        assert_eq!(answer.status, "303 See Other");
        assert_eq!(answer.extra, vec!["Location: /Opus".to_string()]);

        let (answer, _) = route(&request("GET", "/nope", &[HOST]), 9996);
        assert_eq!(answer.status, "404 Not Found");
    }

    #[test]
    fn a_process_is_asked_for_by_its_number() {
        let mut asking = request("GET", "/Opus/threads", &[HOST]);
        asking.query = format!("pid={}", std::process::id());
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "200 OK");
        assert!(answer.body.starts_with(format!("{{\"pid\":{},", std::process::id()).as_bytes()));

        asking.query = "pid=nope".to_string();
        let (answer, _) = route(&asking, 9996);
        assert_eq!(answer.status, "400 Bad Request");
    }

    #[test]
    fn a_shutdown_needs_the_page_header_and_a_post() {
        let (answer, next) = route(&request("POST", "/Opus/shutdown", &[HOST]), 9996);
        assert_eq!(answer.status, "403 Forbidden");
        assert!(matches!(next, Next::KeepGoing));

        let (answer, next) = route(&request("GET", "/Opus/shutdown", &[HOST, ("x-opus", "shut-down")]), 9996);
        assert_eq!(answer.status, "405 Method Not Allowed");
        assert!(matches!(next, Next::KeepGoing));

        let (answer, next) = route(&request("POST", "/Opus/shutdown", &[HOST, ("x-opus", "shut-down")]), 9996);
        assert_eq!(answer.status, "200 OK");
        assert!(matches!(next, Next::ShutDown));
    }
}
