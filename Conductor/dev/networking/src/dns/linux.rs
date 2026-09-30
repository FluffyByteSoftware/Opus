//! File:       Opus/Conductor/dev/networking/src/dns/linux.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! A reverse lookup on Linux: `getnameinfo` from the C library, which
//! every Linux program already has linked in.  It asks the resolver the
//! way `/etc/nsswitch.conf` says to (`/etc/hosts` first, then DNS), so
//! `127.0.0.1` comes back as `localhost` and a machine on the LAN comes
//! back as whatever the router or the hosts file calls it.  NI_NAMEREQD
//! makes it fail rather than hand the address back as text when there's
//! no name.

use std::ffi::{c_char, c_int};
use std::net::IpAddr;

/// `sockaddr_in` as Linux lays it out: 16 bytes.
#[repr(C)]
struct SockAddrIn {
    family: u16,
    /// Network byte order.  Always 0 here; a name lookup has no port.
    port: u16,
    addr: [u8; 4],
    zero: [u8; 8],
}

/// `sockaddr_in6` as Linux lays it out: 28 bytes.
#[repr(C)]
struct SockAddrIn6 {
    family: u16,
    port: u16,
    flowinfo: u32,
    addr: [u8; 16],
    scope_id: u32,
}

const AF_INET: u16 = 2;
const AF_INET6: u16 = 10;

/// Fail instead of handing the address back as text.
const NI_NAMEREQD: c_int = 8;

/// The longest name `getnameinfo` will write, NUL included.
const NI_MAXHOST: usize = 1025;

// Rust note: an `extern` block lists functions that live outside Rust.
// Calling one is `unsafe`, since Rust can't check what the other side
// does with the pointers we hand it.
unsafe extern "C" {
    fn getnameinfo(addr: *const u8,
                   addrlen: u32,
                   host: *mut c_char,
                   hostlen: u32,
                   serv: *mut c_char,
                   servlen: u32,
                   flags: c_int) -> c_int;
}

/// The address's name, or `None` when it has none or the resolver can't
/// be reached.  Can take seconds; never call it on a thread a player is
/// waiting on.
pub fn reverse(address: IpAddr) -> Option<String> {
    let mut host = [0u8; NI_MAXHOST];

    // The `unsafe` here is us handing C a pointer to a struct we own,
    // laid out the way it expects and sized by the length beside it, and
    // a buffer of NI_MAXHOST bytes it writes a NUL-ended name into.  Both
    // stay put until the call is back.  No service buffer, so that
    // pointer is null with a length of 0, which the manual allows.
    let result = match address {
        IpAddr::V4(v4) => {
            let sockaddr = SockAddrIn { family: AF_INET, port: 0, addr: v4.octets(), zero: [0; 8] };
            unsafe {
                getnameinfo((&raw const sockaddr).cast(), size_of::<SockAddrIn>() as u32,
                            host.as_mut_ptr().cast(), NI_MAXHOST as u32,
                            std::ptr::null_mut(), 0, NI_NAMEREQD)
            }
        }
        IpAddr::V6(v6) => {
            let sockaddr = SockAddrIn6 { family: AF_INET6, port: 0, flowinfo: 0, addr: v6.octets(), scope_id: 0 };
            unsafe {
                getnameinfo((&raw const sockaddr).cast(), size_of::<SockAddrIn6>() as u32,
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
        assert_eq!(name_in(b"noend"), Some("noend".to_string()));
    }

    #[test]
    fn the_structs_are_the_size_c_expects() {
        assert_eq!(size_of::<SockAddrIn>(), 16);
        assert_eq!(size_of::<SockAddrIn6>(), 28);
    }
}
