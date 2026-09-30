//! File:       Opus/Conductor/dev/networking/src/dns/windows.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! A reverse lookup on Windows: `getnameinfo` from ws2_32, the same call
//! as Linux's with Windows's own numbers for the address families and the
//! flag.  Winsock has to be started before any of its calls, and Rust's
//! standard library does that the first time a socket is made; the TCP
//! listener is bound before this thread is started, so that's done.
//!
//! Never built yet.  Linux is where Conductor is built and tested; see
//! CLAUDE.md.

use std::ffi::c_char;
use std::net::IpAddr;

/// `SOCKADDR_IN` as Windows lays it out: 16 bytes.
#[repr(C)]
struct SockAddrIn {
    family: u16,
    /// Network byte order.  Always 0 here; a name lookup has no port.
    port: u16,
    addr: [u8; 4],
    zero: [u8; 8],
}

/// `SOCKADDR_IN6` as Windows lays it out: 28 bytes.
#[repr(C)]
struct SockAddrIn6 {
    family: u16,
    port: u16,
    flowinfo: u32,
    addr: [u8; 16],
    scope_id: u32,
}

const AF_INET: u16 = 2;
const AF_INET6: u16 = 23;

/// Fail instead of handing the address back as text.
const NI_NAMEREQD: i32 = 0x04;

/// The longest name `getnameinfo` will write, NUL included.
const NI_MAXHOST: usize = 1025;

// Rust note: an `extern` block lists functions that live outside Rust.
// Calling one is `unsafe`, since Rust can't check what the other side
// does with the pointers we hand it.
#[link(name = "ws2_32")]
unsafe extern "system" {
    fn getnameinfo(addr: *const u8,
                   addrlen: i32,
                   host: *mut c_char,
                   hostlen: u32,
                   serv: *mut c_char,
                   servlen: u32,
                   flags: i32) -> i32;
}

/// The address's name, or `None` when it has none or the resolver can't
/// be reached.  Can take seconds; never call it on a thread a player is
/// waiting on.
pub fn reverse(address: IpAddr) -> Option<String> {
    let mut host = [0u8; NI_MAXHOST];

    // The `unsafe` here is us handing Windows a pointer to a struct we
    // own, laid out the way it expects and sized by the length beside
    // it, and a buffer of NI_MAXHOST bytes it writes a NUL-ended name
    // into.  Both stay put until the call is back.  No service buffer, so
    // that pointer is null with a length of 0.
    let result = match address {
        IpAddr::V4(v4) => {
            let sockaddr = SockAddrIn { family: AF_INET, port: 0, addr: v4.octets(), zero: [0; 8] };
            unsafe {
                getnameinfo((&raw const sockaddr).cast(), size_of::<SockAddrIn>() as i32,
                            host.as_mut_ptr().cast(), NI_MAXHOST as u32,
                            std::ptr::null_mut(), 0, NI_NAMEREQD)
            }
        }
        IpAddr::V6(v6) => {
            let sockaddr = SockAddrIn6 { family: AF_INET6, port: 0, flowinfo: 0, addr: v6.octets(), scope_id: 0 };
            unsafe {
                getnameinfo((&raw const sockaddr).cast(), size_of::<SockAddrIn6>() as i32,
                            host.as_mut_ptr().cast(), NI_MAXHOST as u32,
                            std::ptr::null_mut(), 0, NI_NAMEREQD)
            }
        }
    };
    if result != 0 {
        return None;
    }
    name_in(&host)
}

/// The NUL-ended name in the buffer, as text.  `None` for an empty one.
fn name_in(buffer: &[u8]) -> Option<String> {
    let end = buffer.iter().position(|&byte| byte == 0).unwrap_or(buffer.len());
    let name = String::from_utf8_lossy(&buffer[..end]).into_owned();
    if name.is_empty() { None } else { Some(name) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_stops_at_the_nul() {
        assert_eq!(name_in(b"desk.lan\0junk"), Some("desk.lan".to_string()));
        assert_eq!(name_in(b"\0"), None);
    }

    #[test]
    fn the_structs_are_the_size_c_expects() {
        assert_eq!(size_of::<SockAddrIn>(), 16);
        assert_eq!(size_of::<SockAddrIn6>(), 28);
    }
}
