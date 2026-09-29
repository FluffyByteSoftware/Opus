//! File:       Opus/Conductor/dev/conductor-wgui/src/http.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Just enough HTTP for one admin on one machine.  A request is a line
//! like `GET /Opus/status?after=12 HTTP/1.1`, then header lines, then a
//! blank line, then a body if `Content-Length` says there is one: the
//! login sends a name and a password, and a settings save sends the
//! lines to change.  We read exactly that many bytes and no more.  The
//! answer is a status line, a few headers, a blank line, and the body,
//! and then the connection closes.  One request per connection keeps it
//! simple, and the browser doesn't mind.

use std::io::{self, Read, Write};
use std::net::TcpStream;

/// The most we read of a request's head, and separately of its body,
/// before giving up on it.  A browser's request is well under 2 KB, and
/// the biggest body we take is a whole config file's worth of lines.
const MOST_REQUEST_BYTES: usize = 16 * 1024;

/// The blank line that ends the head.
const BLANK_LINE: &[u8] = b"\r\n\r\n";

/// One request, the parts of it we look at.
#[derive(Debug, PartialEq)]
pub(crate) struct Request {
    /// `GET`, `POST` and so on.
    pub(crate) method: String,
    /// The path, without the `?` and what comes after.
    pub(crate) path: String,
    /// What comes after the `?`, or nothing.
    pub(crate) query: String,
    /// Header names in lowercase, since HTTP doesn't care about their
    /// case and it's easier to look for one.
    pub(crate) headers: Vec<(String, String)>,
    /// What came after the blank line, as text.  Empty when there was no
    /// `Content-Length`.
    pub(crate) body: String,
}

impl Request {
    /// A header's value, by its name in lowercase.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    /// One value out of the query: `after` from `after=12&x=y` is `12`.
    pub(crate) fn query_value(&self, name: &str) -> Option<&str> {
        self.query.split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value)
    }
}

/// Reads one request off the connection: the request line and headers,
/// then the body if `Content-Length` says there is one.  `None` if the
/// other end closed early, sent too much, or sent something that isn't
/// HTTP.  A `Content-Length` bigger than what arrives runs into the
/// connection's time limit, which comes back as the error it is.
pub(crate) fn read_request(stream: &mut TcpStream) -> io::Result<Option<Request>> {
    let mut received = Vec::new();
    let mut chunk = [0_u8; 2048];

    // The head, up to and including the blank line.
    let body_starts = loop {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Ok(None);
        }
        received.extend_from_slice(&chunk[..count]);

        if let Some(end) = find_blank_line(&received) {
            break end + BLANK_LINE.len();
        }
        if received.len() > MOST_REQUEST_BYTES {
            return Ok(None);
        }
    };
    let head = String::from_utf8_lossy(&received[..body_starts - BLANK_LINE.len()]);
    let Some(mut request) = parse_head(&head) else {
        return Ok(None);
    };

    // The body: exactly as many bytes as the headers say, and no more
    // than we'd take for anything.
    let length = match request.header("content-length") {
        Some(length) => match length.parse::<usize>() {
            Ok(length) if length <= MOST_REQUEST_BYTES => length,
            _ => return Ok(None),
        },
        None => 0,
    };
    while received.len() < body_starts + length {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Ok(None);
        }
        received.extend_from_slice(&chunk[..count]);
    }
    request.body = String::from_utf8_lossy(&received[body_starts..body_starts + length]).into_owned();
    Ok(Some(request))
}

/// Where the blank line after the headers starts, if it has arrived.
fn find_blank_line(bytes: &[u8]) -> Option<usize> {
    bytes.windows(BLANK_LINE.len()).position(|four| four == BLANK_LINE)
}

/// Pulls the method, path, query and headers out of the text before the
/// blank line.
pub(crate) fn parse_head(head: &str) -> Option<Request> {
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split_whitespace();
    let method = first.next()?.to_string();
    let target = first.next()?;
    if !first.next()?.starts_with("HTTP/") {
        return None;
    }

    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect();

    Some(Request { method, path: path.to_string(), query: query.to_string(), headers, body: String::new() })
}

/// Sends a whole answer and lets the connection close.  `extra` is any
/// headers on top of the usual ones, each a whole `Name: value` line.
pub(crate) fn respond(stream: &mut TcpStream,
                      status: &str,
                      content_type: &str,
                      extra: &[String],
                      body: &[u8]) -> io::Result<()> {
    let mut head = format!("HTTP/1.1 {status}\r\n\
                            Content-Type: {content_type}\r\n\
                            Content-Length: {}\r\n\
                            Cache-Control: no-store\r\n\
                            X-Content-Type-Options: nosniff\r\n\
                            Connection: close\r\n",
                           body.len());
    for line in extra {
        head.push_str(line);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");

    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_comes_apart_into_its_pieces() {
        let request = parse_head("GET /Opus/status?after=12&x=y HTTP/1.1\r\nHost: 127.0.0.1:9996\r\n\
            X-Opus:  shut-down \r\n").expect("that's a good request");

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/Opus/status");
        assert_eq!(request.query_value("after"), Some("12"));
        assert_eq!(request.query_value("nope"), None);
        assert_eq!(request.header("host"), Some("127.0.0.1:9996"));
        assert_eq!(request.header("x-opus"), Some("shut-down"));
    }

    #[test]
    fn something_that_isnt_http_is_turned_away() {
        assert_eq!(parse_head("hello there"), None);
        assert_eq!(parse_head("GET /Opus SMTP/1.0"), None);
        assert_eq!(parse_head(""), None);
    }

    #[test]
    fn the_blank_line_is_found() {
        assert_eq!(find_blank_line(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n"), Some(23));
        assert_eq!(find_blank_line(b"GET / HTTP/1.1\r\nHost: x\r\n"), None);
    }

    /// Sends `text` down a loopback connection in two pieces, the way a
    /// browser's request can arrive, and reads it back as a request.
    fn read_over_loopback(text: &str) -> io::Result<Option<Request>> {
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .expect("a loopback port for the test");
        let address = listener.local_addr().expect("the port it got");
        let (first, second) = text.split_at(text.len() / 2);
        let (first, second) = (first.to_string(), second.to_string());
        let sender = std::thread::spawn(move || {
            let mut client = TcpStream::connect(address).expect("connect to the test's own listener");
            client.write_all(first.as_bytes()).expect("send the first half");
            client.write_all(second.as_bytes()).expect("send the second half");
        });
        let (mut stream, _) = listener.accept().expect("take the test's connection");
        stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).expect("a time limit");
        let request = read_request(&mut stream);
        sender.join().expect("the sender finished");
        request
    }

    #[test]
    fn a_body_is_read_to_its_length_and_no_further() {
        let request = read_over_loopback("POST /Opus/login HTTP/1.1\r\nHost: 127.0.0.1:9996\r\n\
            Content-Length: 30\r\n\r\nname = admin\npassword = admin\nextra that isn't counted")
            .expect("read fine")
            .expect("a good request");
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/Opus/login");
        assert_eq!(request.body, "name = admin\npassword = admin\n");

        let request = read_over_loopback("GET /Opus HTTP/1.1\r\nHost: 127.0.0.1:9996\r\n\r\n")
            .expect("read fine")
            .expect("a good request");
        assert_eq!(request.body, "");
    }

    #[test]
    fn a_body_that_is_too_big_or_that_never_comes_is_turned_away() {
        let request = read_over_loopback("POST /Opus/login HTTP/1.1\r\nHost: x\r\nContent-Length: 999999\r\n\r\n")
            .expect("read fine");
        assert_eq!(request, None);

        // Says 40 bytes, sends 10 and closes.
        let request = read_over_loopback("POST /Opus/login HTTP/1.1\r\nHost: x\r\nContent-Length: 40\r\n\r\n0123456789")
            .expect("read fine");
        assert_eq!(request, None);
    }
}
