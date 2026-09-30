//! File:       Opus/Conductor/dev/networking/src/dns/other.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Any OS that isn't Linux or Windows, which for now means macOS.  No
//! reverse lookup here yet: the TCP tab shows addresses without names.
//! macOS has the same `getnameinfo`, with its own `sockaddr` layout (a
//! length byte first), and that's a job for when there's a Mac to test
//! it on.

use std::net::IpAddr;

/// Always `None`: nothing looked up here yet.
pub fn reverse(_address: IpAddr) -> Option<String> {
    None
}
