//! File:       Opus/Conductor/dev/conductor-wgui/src/http.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Just enough HTTP for one admin on one machine.  A request is a line
//! like `GET /Opus/status?after=12 HTTP/1.1`, then header lines, then a
//! blank line.  We read that much and no more, since nothing we take has a
//! body worth reading.  The answer is a status line, a few headers, a
//! blank line, and the body, and then the connection closes.  One request
//! per connection keeps it simple, and the browser doesn't mind.

use std::io::{self, Read, Write};
use std::net::TcpStream;

/// The most we read of a request before giving up on it.  A browser's
/// request is well under 2 KB.
const MOST_REQUEST_BYTES: usize = 16 * 1024;

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

/// Reads the request line and headers off the connection.  `None` if the
/// other end closed early, sent too much, or sent something that isn't
/// HTTP.
pub(crate) fn read_request(stream: &mut TcpStream) -> io::Result<Option<Request>> {
    let mut received = Vec::new();
    let mut chunk = [0_u8; 2048];

    loop {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Ok(None);
        }
        received.extend_from_slice(&chunk[..count]);

        if let Some(end) = find_blank_line(&received) {
            return Ok(parse_head(&String::from_utf8_lossy(&received[..end])));
        }
        if received.len() > MOST_REQUEST_BYTES {
            return Ok(None);
        }
    }
}

/// Where the blank line after the headers starts, if it has arrived.
fn find_blank_line(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|four| four == b"\r\n\r\n")
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

    Some(Request { method, path: path.to_string(), query: query.to_string(), headers })
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
}
