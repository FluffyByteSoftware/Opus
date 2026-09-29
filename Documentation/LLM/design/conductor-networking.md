<!--
File:       Opus/Documentation/LLM/design/conductor-networking.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-networking

A lib crate, and a server piece: it comes up on START SERVER after Archivist and down on STOP SERVER
before Security.  The part of Conductor that talks to players.  Started 2026-09-29, named by Jacob.  One
new crate, `rustls` (with `ring` for the crypto, TLS 1.3 only).  The PEM reading comes with its `std`
feature; a separate `rustls-pki-types` line with a `pem` feature was the first build error, since no such
feature exists.

## Skeleton

```
conductor-networking/
├── Cargo.toml
├── test_client.py       the stand-in client: TLS, Login, Ticket, Connect, keep-alives, Goodbye.  Python 3.
└── src/
    ├── lib.rs           start(), stop(), status() -> Status { tcp, udp, players, tickets, connections, in_world, access },
    │                      kick(id), access_lists(), list_address(), unlist_address(), enforce(); timed_out(), wake_address()
    ├── settings.rs      networking.cfg as networking reads it: struct Settings, load()
    ├── tls.rs           server_config(settings) -> Arc<ServerConfig>; MAKE_PAIR, the openssl command
    ├── protocol.rs      the packets, byte for byte: PacketType, LoginAnswer, ConnectAnswer, KickReason, Choice
    │                      frame(), take_packet(), take_datagram(); hello(), in_line(), login_result(), ticket(),
    │                      connect_result(), keep_alive(), kicked(); read_login(), read_session_choice(), read_connect()
    ├── sessions.rs      the book: tickets by token, players by address, each account's whereabouts
    │                      playing(), issue(), connect(), heard(), leave(), kick(), sweep(), clear(), counts(), players(),
    │                      drop_where()
    ├── ledger.rs        the door's ledger: every connection of the last five minutes and its Stage; End; Connection
    │                      clear(), arrived(), set(), ended(), is_done(), snapshot()
    ├── access.rs        the whitelist and the blacklist: Mode, List, Entry (an address or a range), Verdict
    │                      start(mode, paths), stop(), verdict(ip), add(), remove(), snapshot(), counts()
    ├── dns.rs           reverse DNS on thread net-dns, with a cache: start(), stop(), ask(), name_of()
    ├── dns/linux.rs     reverse(ip) around getnameinfo from the C library
    ├── dns/windows.rs   the same around ws2_32's; never built
    ├── dns/other.rs     macOS and the rest: no names
    ├── tcp.rs           the acceptor thread, the login threads, TLS, the login flow, the failure hold, kick(),
    │                      close_where() for a ban
    └── udp.rs           the one UDP thread: Connect, KeepAlive, Goodbye, the sweep; tell() for kicks
```

## What Jacob asked for

In his words, near enough: a TLS wrapper for login sessions, a TCP connection used to authenticate and
deliver the user post-authentication to the UDP server, and UDP handles everything after login.  You
authenticate and are given a token that tells you where to go for UDP.  If the UDP session fails or ends,
drop and disconnect the player; the player goes back to the login screen and starts over.  Iterate on
Stratum's networking with lower CPU, letting RAM grow if it has to.  One login is processed at a time
because of the CPU cost.

## What we decided

- **TCP is only the login.**  Stratum kept the TCP connection open for the whole session (chat, character
  select).  Opus closes it the moment the Ticket is sent.  So there are no long-lived connection threads
  at all: a connection lives for one TLS handshake and one login.
- **A fixed pool of login threads, not a thread per connection.**  `login_threads` in `networking.cfg`
  (8) threads take connections off a queue the acceptor fills.  A thread per connection would cost a
  thread start per connection and an entry on the threads list for good, and buy nothing, since Security
  hashes one login at a time however many threads wait on it.  A connection that finds the queue full
  (`max_waiting_logins`, 64) is closed at the door; one that waited in the queue past the login deadline
  is closed unserved.
- **No thread polls.**  Stratum's connection threads woke every 50 ms to look at flags.  Here every read is
  armed with exactly the time left to the connection's deadline, so a thread sleeps in the OS's read until
  bytes come or the time is up.  `stop()` wakes the acceptor by connecting to it, and the login threads by
  shutting down the sockets they're reading (a clone of each is kept on a list for that), so a stop takes
  as long as the slowest thread needs to notice.  The UDP thread wakes once a second, packets or none, to
  sweep and check in, and that's the only timer in the crate.
- **A failed login puts the address on a 2-second hold at the door**, not in a sleeping thread: the
  acceptor closes the next connection from that address without queuing it.  Stratum slept a thread for
  the hold.
- **The login is one packet.**  Stratum had Hello, secret word, "awaiting", credentials: two round trips
  before the password.  Opus is Hello, then one Login with the client version, the secret word, the
  username and the password, then the answer.  The version is checked first (an old client is told so
  without a hash), then the secret word (no hash), then the name's shape (no hash; the rule is in the
  schema, so a quick no gives nothing away), and only then the accounts table and Security's line.  A name
  that could be an account but isn't still costs a hash in the same line (`verify_no_account()`), and
  `pad_login_time()` evens out the rest.
- **The client is told its place in Security's line.**  The login thread waits on its `Ticket` a second at
  a time (`wait_for()`, added to `Pending` and `Ticket` for this), and between waits sends an InLine with
  `place()`'s numbers.  Jacob asked for that when Security was built.
- **The right password for an account already in the world gets asked**, Stratum's SessionChoice: log the
  other session out (it hears a Kicked over UDP) or hang up.  Jacob's answer, 2026-09-29.
- **A ticket is a token from Fingerprinter, good once, for `token_deadline_seconds` (30).**  The first
  address to Connect with it is the player; the same address again gets the same answer (a lost reply);
  any other address is refused.  A second login for an account with an unused ticket replaces the ticket.
- **A player is known by their address.**  `sessions.rs` keeps three maps (tickets by token, players by
  address, accounts by name) so nothing is ever found by a search: a keep-alive is one map lookup and one
  send.  That's more memory per player for a cost that's the same with 5 or 5000.  Jacob's trade.
- **The answers that never change are built once** and sent as they are.  A UDP packet is looked at in the
  buffer it arrived in; nothing is copied for a keep-alive.
- **The server echoes every KeepAlive.**  One tiny packet a second per player, so the client can tell the
  server is gone and go back to the login on its own.  Without it, a client whose server died would sit
  there.
- **Quiet for `udp_timeout_seconds` (40) and the player is dropped, told nothing, forgotten.**  Jacob's
  number.  Goodbye, a kick and STOP SERVER end a session the same way, with a Kicked first where the
  player can hear it.  Nothing is kept for a reconnect: that's TODO, with the rest of client management.
- **The certificate is made by hand, once, with openssl.**  Stratum made its own pair with the `rcgen`
  crate on first start; Opus stays a crate lighter and the log says the exact command when the files are
  missing (`tls::MAKE_PAIR`).  An elliptic curve key (P-256), since the server signs on every handshake
  and an EC signature costs a tenth of an RSA one.  The files live in `Content/certs/`; the `.key` is
  ignored by git and the `.crt` is committed, since clients need a copy.  Jacob's call on the folder.
- **Everything is in `networking.cfg`, a soft file**: the address, both ports, the two file paths, the
  secret word, the client versions (a comma list; Stratum baked them into the build), the login deadline,
  the pool and queue sizes, the token deadline and the UDP timeout.  One table entry each in
  `constellations/files.rs`, and the Settings tab shows the file with no page work.
- **Two services**, "Network (TCP)" on thread `net-tcp` and "Network (UDP)" on `net-udp`.  The login
  threads are `net-login-1` and up.  The UDP thread checks in once a second; the acceptor has no loop to
  check in from.  If the TLS files are missing or TCP can't listen, both show trouble and nothing listens;
  if UDP can't listen, TCP comes back down.  None of it stops the rest of the server.
- **Log levels.**  Connections, TLS, hang-ups and timeouts are Debug on the Network channel.  Who logged
  in, who failed, who's in the world and who left are Info on the Security channel.  A full queue and a
  failed accept or send are Warn.  Missing TLS files and a token that can't be made are capitals Errors.
- **The door's ledger** (2026-09-29, the session after), for the web admin's TCP tab.  Jacob's spec: every
  connection that reached the listener in the last 5 minutes, by IP address and DNS if known, its place
  in the login queue if it isn't logged in yet, and no account information, tracked purely by address.
  So `ledger.rs` is a BTreeMap of entries numbered as they arrive: the acceptor writes one in for every
  accept (the ones it closes at the door too, marked so), the login thread moves it a stage at a time
  (TLS, waiting for its Login, checking, in Security's line with `place()`'s numbers once a second,
  asked about another session), and every way out writes the ending.  A handful of lock touches per
  login, never per byte.  A finished entry stays `connections_remember_seconds` (five minutes) from its
  arrival, a setting since Jacob asked for one; one in progress stays whatever the clock says; past 1000 entries the oldest
  finished ones go early.  The first ending written wins, so a kicked connection reads "kicked" and not
  the login thread's "hung up" a moment later.  A queued connection's place is a count of the queued
  entries ahead of it, worked out when the snapshot is taken.  Wiped on START SERVER and STOP SERVER.
- **Reverse DNS on its own thread**, `net-dns` (`dns.rs`).  A lookup asks the OS's resolver and can take
  seconds, so the acceptor only drops the address on a queue; the thread looks it up with the OS's own
  `getnameinfo` (an `extern` block in `dns/linux.rs` and `dns/windows.rs`, the way the monitor talks to
  the OS, no crate) and writes the name into a cache the snapshot reads.  One lookup per address per
  START SERVER, at most 4096 cached.  No name is never a failure: the tab shows the address on its own.
  macOS gets no names until there's a Mac to test on.
- **KICK from the TCP tab** (Jacob, the same session): a clone of every open socket is kept under its
  ledger number from accept until its login thread is done with it (the same map `stop()` shuts to wake
  the threads; it used to hold only the ones being served).  `kick(id)` marks the ledger, shuts the
  socket, and the login thread finds its read failing, or skips the connection if it was still queued.
  The client sees the connection drop and nothing else.  A kick is an Info line on the Network channel.
- **The whitelist and the blacklist** (2026-09-29, the session after the TCP tab), Jacob's ask.  `access.rs`.
  Two files, `Content/cfg/whitelist.cfg` and `blacklist.cfg`, one address (`1.2.3.4`, `::1`) or range
  (`1.2.3.0/24`, since a ban on one home address is easy to step around) a line, `#` comments; `.cfg` by
  his call, but not Constellations' kind (a list that grows from the page doesn't fit `key = value`), so
  `networking.cfg` points at them (`whitelist_file`, `blacklist_file`) and a switch there, `access_list =
  off | whitelist | blacklist`, says which one the door looks at.  The switch and the files are read on
  every START SERVER, so the lists in memory are always rebuilt from disk at a start.  A change from the
  page takes at once, on the next connection, and rewrites the file the same moment: the first and only
  hot swap in Conductor, because a ban that waited for a STOP SERVER wouldn't be much of a ban.  While
  the server is stopped nothing is loaded, so the page can't change them; the files can be edited by hand
  then.  The check sits in the acceptor before the failure hold, so a listed address costs an accept and
  a close, and the ledger says "closed at the door: blacklisted" or "not on the whitelist" (Debug lines,
  never a Warn: a banned address hammering the door shouldn't fill the bell).  A range's host bits are
  cleared on the way in, an IPv4 address wrapped in IPv6 (`::ffff:1.2.3.4`, what a listener on `::` sees)
  is checked as the IPv4 one, and a check is one lock and a walk down a short list.  The UDP side checks
  a Connect too, so a listed address with a ticket in hand gets silence.  **A blacklisting while the
  blacklist is on is a ban, and so is taking an entry off the whitelist while the whitelist is on**,
  Jacob's rules: `enforce()` in `lib.rs` asks the verdict again for everybody online, so a whitelist
  removal is right when another entry still covers the address; `tcp::close_where()` shuts every open
  TCP connection the door would now turn away (the ledger says banned) and `sessions::drop_where()`
  drops every such player, each told a Kicked with reason `3`, banned, first.  That reason is new:
  protocol version 2, the same session.  Taking an address off the blacklist, or adding one to the
  whitelist, kicks nobody.  An entry on a list that isn't switched on is kept and does nothing, and the
  page says so.  With the whitelist on and empty, nobody can log in, and the start says so with a Warn.
- **The players for the page** (the same session): `sessions::players()` copies every player out
  (address, account, when their Connect was accepted, how long they've been in, how long since their
  last packet), newest first, for the Connections tab's UDP list.  The account is on it, unlike the
  door's ledger: the world is about who's in it.
- **The Python test client** stands in for Ensemble: standard library only, trusts the certificate file,
  prints every packet, and has switches for kicking or sparing the other session, leaving after N
  seconds, and going quiet to watch the timeout.  Jacob used one for Stratum too.

## What's open

- **Built and run on Linux, 2026-09-29**, the same day it was written: one Cargo.toml fix (a feature
  that didn't exist) and then the Python client did the whole loop against a throwaway account inserted
  by hand.  Two clients on one account, the quiet drop, the hold and a kick at STOP SERVER haven't been
  tried by hand yet; the unit tests cover the book and the bytes.
- **Client management** is all TODO: a player limit ("The server is full."), reconnecting with a token
  instead of a fresh hash, and anything an admin does to a player from the web admin.
- **NAT rebinding.**  A player is their address, so a home router that changes the port mid-session ends
  the session.  A session id in each UDP packet would survive it.  Later, if it bites.
- **The Windows side** has never been built.  Nothing here is OS-specific but the `ConnectionReset` line
  in `udp.rs`, which is Windows telling us about a bounced packet.
- **The web admin shows the door and the world** (the Connections tab, 2026-09-29): the players are listed by
  account; the character goes beside it once there is one.
- **A hand edit to a list file while Conductor runs** isn't seen on the next START SERVER: DiskMan serves a
  file it already holds from memory.  The same as the config files, in TODO.md under DiskMan.  A hand edit
  with Conductor down is read fine.
