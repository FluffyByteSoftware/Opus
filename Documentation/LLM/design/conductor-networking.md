<!--
File:       Opus/Documentation/LLM/design/conductor-networking.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-networking

A lib crate (`conductor-networking`, folder `networking/`), and a server piece: the part of Conductor that
talks to players.  Named by Jacob.  One crate beyond our own, `rustls`, default features off: `ring` for the
crypto, `std` for the sockets and the PEM files, and `tls12`, so TLS 1.3 or 1.2.  It was 1.3 only until
Ensemble's first compile (2026-10-02): Unity's .NET has no TLS 1.3, and Jacob had OKed 1.2 for that case.

**The door opens once the world is in** (Jacob, 2026-09-30: nobody gets in before there's a voxel to step
on).  START SERVER only calls `wait_for_world()`, which puts both services on "Waiting on the world".  The
launcher's command loop (`take_commands()`, round every 250 ms) asks `conductor_gameclock::ready()` while
the server runs with the door shut, and calls `start()` once the ground around 0,0,0 is in.  STOP SERVER
takes networking down right after the monitor and before the GameClock, so every player is told before
anything a login leans on goes.  `stop()` is safe when the door never opened, and says so on the Services tab.

## Skeleton

```
networking/
├── Cargo.toml         conductor-accounts, conductor-gameclock, conductor-lua-parser, conductor-primlib,
│                        conductor-tools, rustls 0.23 ("ring", "std")
├── test_client.py     the stand-in client: TLS, Login, Ticket, Connect, character select (--create,
│                        --delete, --delete-word, --reset-home), --play, --type (chat), keep-alives,
│                        Goodbye.  Python 3.
└── src/
    ├── lib.rs         start(), wait_for_world(), stop(), status() -> Status { tcp, udp, players, tickets,
    │                    connections, in_world, access, whitelisted, blacklisted }, kick(id), terminate(account),
    │                    access_lists(), list_address(), unlist_address() -> Changed; enforce();
    │                    timed_out(), wake_address()
    ├── settings.rs    networking.cfg as networking reads it: struct Settings, load()
    ├── tls.rs         server_config(settings) -> Arc<ServerConfig>; make_pair(), the openssl command
    ├── protocol.rs    the packets, byte for byte, PROTOCOL_VERSION 9: PacketType, LoginAnswer, ConnectAnswer,
    │                    KickReason, Choice, CreateAnswer, DeleteAnswer, ListedCharacter, EnteredCharacter;
    │                    frame(), take_packet(), take_datagram(); hello(), in_line(), login_result(), ticket(),
    │                    connect_result(), keep_alive(), kicked(), command_accepted(), command_refused(),
    │                    character_list(), create_result(), delete_result(), entered_world(),
    │                    chat_deliveries(), who_delivery(), spans(); WhoEntry; read_login(),
    │                    read_session_choice(), read_connect(),
    │                    read_list_request(), read_create(), read_delete(), read_reset_home(),
    │                    read_user_press_play(), read_player_command()
    ├── typed.rs       a line a player typed: Asker, Outcome (Answer, or Later for the GameClock to
    │                    answer), set_runner() (the slot conductor-player-commands' wire() fills), run()
    ├── protogame.rs   Protogame, thread protogame: start(), stop(), hand_in(from, account, ask, Work) -> the
    │                    answer at once if it isn't running; enum Work { List, Create, Delete, ResetHome,
    │                    Play }; play() and bring_in(), the spawn's slow part
    ├── sessions.rs    the book: tickets by token, players by address, each account's whereabouts
    │                    and each player's character in the world (InWorld), the one-second lockout
    │                    and each character's lock (Lock, LOCK_FOR); playing(), issue(), connect(), heard(),
    │                    begin_ask() -> Ask, finish_ask(), entered(), leave_world(), lock_for_loading(),
    │                    turn_away(), leave(), kick() (with the kicked character's row id),
    │                    terminate(), kick_login(), sweep(), clear(), counts(), players(), in_world(),
    │                    names_in_world(), may_command(),
    │                    drop_where();
    │                    with_book(), which asks the GameClock to take out whoever left
    ├── ledger.rs      the door's ledger: every connection since START SERVER; Stage, End, Gone, Connection
    │                    start(), clear(), arrived(), set(), ended(), linkdead(), is_done(), snapshot()
    ├── access.rs      the whitelist and the blacklist: Mode, List, Entry (an address or a range), Verdict
    │                    start(mode, paths), stop(), verdict(ip), mode(), add(), remove(), snapshot(), counts()
    ├── dns.rs         reverse DNS on thread net-dns, with a cache: start(), stop(), ask(), name_of()
    ├── dns/linux.rs   reverse(ip) around getnameinfo from the C library
    ├── dns/windows.rs the same around ws2_32's
    ├── dns/other.rs   macOS and the rest: no names
    ├── tcp.rs         the acceptor, the login threads, TLS, the login flow, the failure hold; kick(),
    │                    close_where() for a ban, listening_on(); wait_for_save() after logging the other
    │                    session out
    └── udp.rs         the one UDP thread: Connect, KeepAlive, Goodbye, character select's asks handed to
                         Protogame, PlayerCommand answered on the spot, answers in Spans when too big, the
                         sweep; tell(), tell_all(), tell_answer(),
                         listening_on()
```

## What Jacob asked for

In his words, near enough: a TLS wrapper for login sessions, a TCP connection used to authenticate and
deliver the user post-authentication to the UDP server, and UDP handles everything after login.  You
authenticate and are given a token that tells you where to go for UDP.  If the UDP session fails or ends,
drop and disconnect the player; the player goes back to the login screen and starts over.  Iterate on
Stratum's networking with lower CPU, letting RAM grow if it has to.  One login is processed at a time
because of the CPU cost.

## What we decided

- **TCP is only the login.**  The connection closes the moment the Ticket is sent, so a connection lives for
  one TLS handshake and one login, and no thread is kept for it.
- **A fixed pool of login threads**, `login_threads` (8), taking connections off a queue the acceptor fills.
  A thread per connection would cost a thread start each and a threads-list entry for good, and buy
  nothing, since Security hashes one login at a time however many threads wait.  A full queue
  (`max_waiting_logins`, 64) closes the connection at the door; one that waited past the login deadline
  (`login_deadline_seconds`, 10) is closed unserved.  **The TLS handshake has three seconds of its own**
  (`HANDSHAKE_WAIT`, inside the login deadline; 2026-10-02): a connection that sends nothing would
  otherwise hold a login thread for the whole ten, and eight of those every ten seconds would keep every
  real player out.  **An address has at most four connections at the door at once**
  (`MOST_OPEN_PER_ADDRESS`); its fifth is closed at the door (the ledger says so).
- **No thread polls.**  Every read is armed with exactly the time left to the connection's deadline.
  `stop()` wakes the acceptor by connecting to it and the login threads by shutting their sockets, then
  gives the login threads 2 seconds and the DNS thread 1 before a Warn and going on without them.  The only
  timers: the UDP thread wakes once a second to sweep and check in, a login thread in Security's line wakes
  once a second to send an InLine, and a failed accept or receive rests 100 ms so a broken socket can't spin.
- **The login is one packet.**  Hello, then one Login (client version, secret word, username, and the
  password's key, never the password: protocol version 7, `design/client-security.md`), then the answer.
  Cheapest checks first: the version (an old client is told so without a hash), the secret word, the name's
  shape (the rule is in the schema, so a quick no gives nothing away), the key's shape (64 of `0-9a-f`), and
  only then the accounts table and Security's line.  A name that could be an account but isn't still costs a hash
  (`verify_no_account()`), and `pad_login_time()` evens out the rest.  Every failure gets the same answer,
  and puts the address on a 2-second hold: the acceptor closes its next connection without queuing it,
  rather than a thread sleeping out the hold.
- **The client is told its place in Security's line**: the login thread waits on its `Ticket` a second at a
  time (`wait_for()`) and sends an InLine with `place()`'s numbers in between.  Jacob asked for that when
  Security was built.
- **The right password for an account already in the world gets asked** (SessionChoice, 30 seconds to
  answer): log the other session out (it hears Kicked over UDP) or hang up.  Jacob's answer, 2026-09-29.
- **A ticket is a token from Fingerprinter, good once, for `token_deadline_seconds` (30).**  The first
  address to Connect with it is the player; the same address again gets the same answer (a lost reply); any
  other address is refused.  A second login for an account with an unused ticket replaces the ticket.
- **The ticket and the player have the account's name, and only the name.**  An account is never held in
  memory (conductor-accounts reads the row when it's needed), so the web admin can change an account while
  its player is online and nothing writes an old copy back.  The login reads only the password hash.  The
  ticket's first Connect stamps the login time straight in the row (`stamp_login()`, once the book's lock
  is let go): the UDP time is the login time, Jacob's call, for playtime later.  Nothing is saved when a
  player leaves.
- **A player is known by their address.**  Three maps (tickets by token, players by address, accounts by
  name), so nothing is found by a search: a keep-alive is one lookup and one send, the same cost with 5
  players or 5000, for more memory per player.  Jacob's trade.  The answers that never change are built
  once, and a UDP packet is read in the buffer it arrived in.
- **The server echoes every KeepAlive** from a player it knows; a stranger's gets nothing.  So the client
  can tell the server is gone and go back to the login on its own.
- **Quiet for `udp_timeout_seconds` (40) and the player is dropped, told nothing, forgotten.**  Jacob's
  number.  Goodbye, a kick and STOP SERVER end a session the same way, with a Kicked first where the player
  can hear it.  Nothing is kept for a reconnect.
- **Each new Kicked reason bumps the protocol**: banned (`3`) made it version 2, kicked by the admin (`4`)
  version 3, account terminated (`5`) version 4.
- **The certificate is made by hand, once, with openssl**, a crate lighter than making it ourselves.  When
  the files are missing the log gives the command (`tls::make_pair()`) with the full paths from
  `networking.cfg`, so it can be pasted from any folder without making a stray `Content`.  A P-256 key,
  since the server signs on every handshake and an EC signature costs a tenth of an RSA one.  Self-signed:
  a client trusts the copy it was given, so a new certificate locks out every client with the old one.  In
  `Content/certs/` (Jacob's call); the `.key` is ignored by git, the `.crt` committed for clients.
- **Everything is in `networking.cfg`, a soft file**: the address, both ports, the TLS file paths, the
  secret word, the client versions (a comma list), the deadlines, the pool and queue sizes, the UDP timeout,
  and the access switch and list paths.  The Settings tab shows it with no page work.
- **Two services**, "Network (TCP)" on thread `net-tcp` and "Network (UDP)" on `net-udp`; the login threads
  are `net-login-1` and up.  UDP checks in once a second; the acceptor has no loop to check in from.  TLS
  files missing or TCP unable to listen: TCP shows trouble, UDP says "Not started: the TCP side couldn't.",
  nothing listens.  UDP unable to listen: UDP shows trouble and TCP comes back down.  Neither stops the rest
  of the server.
- **Log levels.**  Connections, TLS, hang-ups and timeouts are Debug on the Network channel.  Who logged in,
  who failed, who's in the world and who left are Info on the Security channel.  A full queue and a failed
  accept or send are Warn.  Missing TLS files and a token that can't be made are capitals Errors.
- **The door's ledger** (`ledger.rs`), for the Connections tab.  Jacob's spec: every connection that reached
  the listener, by IP address and DNS name if known, its place in the login queue if it isn't logged in
  yet, and no account information, tracked purely by address.  Entries are numbered as they arrive (the
  ones closed at the door too); the login thread moves each a stage at a time and every way out writes the
  ending, and the first ending written wins, so a kick isn't overwritten by "hung up".  A handful of lock
  touches per login, never per byte.  An entry stays until STOP SERVER, so the Historical view is the whole
  run beside Recent, the newest five (Jacob's ask); past 10,000 the oldest finished ones go early, one in
  progress never.  Wiped on START SERVER and STOP SERVER.
- **LINKDEAD** (Jacob's catch and his word).  A login's row would otherwise read green, "Logged in", long
  after its player left, since TCP closed at the ticket.  The ticket carries its row's number, the player
  takes it over, and every way out of the book marks the row LINKDEAD with why (`Gone`): logged out by a
  second login, Goodbye, gone quiet, banned, kicked by the admin, account terminated, or never came over UDP
  (ticket ran out or replaced).  The row stays in both views, greyed.  The book notes rows under its own
  lock and `with_book()` hands them to the ledger after, so the two locks are never held together.
- **Reverse DNS on its own thread**, `net-dns`, since a lookup can take seconds: the acceptor only queues
  the address, and the thread asks the OS's own `getnameinfo` (an `extern` block per OS, no crate) and fills
  a cache the snapshot reads.  One lookup per address per START SERVER, at most 4096 cached.  No name is
  never a failure: the tab shows the address on its own.
- **KICK on any row** (Jacob's ask).  A clone of every open socket is kept under its ledger number from
  accept until its login thread is done (the same map `stop()` shuts).  `kick(id)` tries the door first: an
  open connection is shut where it stands and the client sees it drop.  If it's gone,
  `sessions::kick_login()` finds the login's player by row number (a walk, since an admin's click needn't be
  quick) and takes them out with Kicked, reason `4`, row LINKDEAD "kicked by the admin"; an unused ticket
  is killed instead.  The route is `/Opus/wwwhook/tcp/kick?id=N` (Jacob: "we can use the existing ROUTE"):
  200, with `from_world` for a player, and 404 only when nothing from the row is left.  A kicked player can
  log straight back in; a ban is what keeps somebody out.
- **Deleting an account takes its player out.**  The Accounts tab deletes the row, then calls
  `terminate(account)`: the player hears Kicked with reason `5` (the client says ACCOUNT TERMINATED, Jacob's
  words), an unused ticket dies, and the row reads "LINKDEAD: account terminated".
- **The whitelist and the blacklist** (`access.rs`), Jacob's ask.  `Content/cfg/whitelist.cfg` and
  `blacklist.cfg`, one address (`1.2.3.4`, `::1`) or range (`1.2.3.0/24`, since a ban on one home address
  is easy to step around) a line.  `.cfg` by his call, but not Constellations' kind (a list that grows from
  the page doesn't fit `key = value`), so `networking.cfg` points at them and `access_list = off |
  whitelist | blacklist` says which one the door looks at.  All read on every START SERVER.  A change from
  the page takes at once and rewrites the file (comments in it are lost): the only hot swap in Conductor,
  because a ban that waited for a STOP SERVER wouldn't be much of a ban.  While the server is stopped the
  page can't change them; the files can be edited by hand.  The acceptor checks before the failure hold, so
  a listed address costs an accept and a close, logged as Debug, never a Warn (a banned address hammering
  the door shouldn't fill the bell); UDP checks a Connect too.  Host bits are cleared on the way in, and an
  IPv4 address wrapped in IPv6 (`::ffff:1.2.3.4`) is checked as the IPv4 one.  **A blacklisting while the
  blacklist is on is a ban, and so is taking an entry off the whitelist while the whitelist is on**, Jacob's
  rules: `enforce()` asks the verdict again for everybody online (so a removal is right when another entry
  still covers the address), closes their open TCP connections and drops their players with Kicked, reason
  `3`.  The other two changes kick nobody.  An entry on a list that isn't switched on does nothing, and the
  page says so.  The whitelist on and empty means nobody can log in, and the start says so with a Warn.
- **The players for the page**: `sessions::players()` (address, account, connected when, for how long,
  quiet how long), newest first.  The account is on it, unlike the ledger: the world is about who's in it.
- **The Python test client** stands in for Ensemble: standard library only, UDP over IPv4 only.  It trusts
  `--cert` (by default `Content/certs/conductor.crt`, found from the script's own folder; with neither it
  checks nothing and says so), prints every packet, and asks whether to log out another session
  (`--leave-other-alone` says no).  `--leave-after N` says Goodbye after N seconds, `--go-quiet` stops the
  keep-alives, and `--pause-before-login N` sits open after TLS so the row can be kicked or banned.

## Character select and Protogame (2026-09-30)

Jacob's packets and names, protocol version 5, `0x2_` over UDP (PROTOCOL.md has the bytes):
CharacterListRequest / CharacterListDelivery ("makes it less confusing"), CreateCharacter /
CharacterCreateResult, DeleteCharacter / CharacterDeleteResult, CharacterRequestResetHome, and the general
CommandAccepted / CommandRefused in `0x3_` ("This can be reused elsewhere").  Whether a character can be
played is a byte per character in the list ("just add a bool in it"), not a packet of its own.

- **Protogame** is Jacob's word for it: "the character selection and character construction are proto game
  then become game objects after load".  A module of this crate for now (as a crate of its own it would need
  networking and networking would need it), with its own thread `protogame` and its own line on the
  Services tab.  Started before the UDP side and stopped after it.
- **The UDP thread never waits on the database.**  It reads the ask, asks the book, and hands a new one to
  Protogame's mailbox; Protogame waits on conductor-accounts (10 seconds at most, then "Unavailable"), keeps
  the answer in the book, and sends it with `udp::tell()`.
- **The ask number** (proposed, taken): a u32 on every ask.  The book keeps each player's ask in the works
  and their last answer.  The same number again gets the kept answer, so a lost CharacterCreateResult
  doesn't turn into "That name is taken."; one ask at a time per player, so nobody queues up database jobs.
- **The account is the book's**, never the packet's: a player only sees and changes their own characters.
- **Creating**: the name rule first (no database job for a bad name), then primlib's `new_character(name)`
  saved with `Save::of_blueprint()` as the row's Lua, then `characters::create()`: the first empty slot, all
  three full refused.
- **Deleting** (Jacob: "player presses delete, and the client pre-reqs to ask them to type in delete then
  sends the packet to the server with the typed in word.  Server either approves or denies"): only DELETE,
  any capitals, deletes.
- **Reset home** ("sends character back to 0, 0, 0"): the save holds the position too, so the save is read
  back with lua-parser, made into the character, moved (rotation and scale kept), and saved again whole with
  the position columns.  A save that won't load marks the character unplayable, with the Error on the bell,
  the same as the spawn will.
- **Bigger answers than asks**, unlike the Connect: a list is a few hundred bytes for a five-byte ask.  Only a
  known player's address ever gets one, so a faked sender address has to be a player's.

## The spawn (2026-10-01)

Jacob's packets and names, protocol version 6: **UserPressPlay** (`0x27`, the ask and the character's uuid)
and **CharacterEnteredWorld** (`0x28`, the ask, the uuid, the name, and x, y, z as f32s).  A pick that can't
be played gets a CommandRefused.  The GameClock's half (`enter()`, `leave()`, the world save) was built the
session before (`design/gameclock.md`).

- **Protogame does the slow part** (`play()`, `bring_in()`): the row and the save read through
  conductor-accounts, an unplayable character turned away, the save read back through lua-parser and laid
  over the Character template (a save that won't load marks it unplayable, the Error on the bell, the same
  as a reset home), and the finished blueprint handed to `conductor_gameclock::enter()`.  The answer says
  where the character stands, from its save's Transform.
- **The character goes on the player in the book only after `enter()`** (`sessions::entered()`, with the
  answer kept for a repeat, like any ask).  If the player left while it was being brought in, `entered()`
  says so and Protogame takes it straight back out (`leave_world()`): the GameClock's mailbox is in order,
  so the leave lands after the enter, and nothing is left standing.
- **Every way out of the book takes the character out**, since all of them go through
  `remove_player_in()`: Goodbye, gone quiet, logged out by a second login, banned, kicked by the admin,
  account terminated.  The book notes the character's row id, and `with_book()` calls
  `conductor_gameclock::leave()` once the lock is let go, which saves it and despawns it.  STOP SERVER's
  `clear()` does the same for everybody; the GameClock stops after networking, so it's there to take them,
  and its last world save on the way down has anybody whose note it didn't get to.
- **No way back to character select from the world** (Jacob: "you log out back to log in screen every
  time").  Any character select ask from a player whose character is in the world is refused from the UDP
  thread (`Ask::InWorld`), and the refusal kept like any answer.  So a reset home can never move a
  character that's in the world.  The same goes for logging out in game when it comes: "Even if you camp
  out, you go back to login screen not char select."  So a camp is a session ending, like a Goodbye.
- **The race**: a character leaves, and before the GameClock has taken the note (up to 250 ms) and
  Archivist written its save, a second login picks it and reads the old row.  It bites after "log the other
  session out", where the second login's hash is done before the first is kicked.  Jacob's answers, in
  order: a lock ("a lockout on a character being instantiated for like 1 second?  The player should get a
  reject disconnected packet but its so short they just reconnect"); what he meant, a load lock ("a
  temporary 'load' lock on a character as its pulled from database to memory... and loaded in the world...
  All that lock does is prevent another one from being instantiated"); shown that a load lock alone
  doesn't stop the old row being read, the lock both ways ("we lock it when it does that"); and "do we have
  any way to force a save on the connection being kicked before the new one pops in?"  So, three pieces:
  - **The GameClock's "saving" mark** (`design/gameclock.md`): from the moment `leave()` is called until
    the character's leaving save has landed (or failed).  `conductor_gameclock::saving(id)` asks;
    `wait_until_saved(id, limit)` waits on a Condvar, the OS's own wait.
  - **The login that logged the other session out waits for its save** before handing out its ticket
    (`tcp.rs`, `wait_for_save()`): `sessions::kick()` hands back the old character's row id, by then asked
    out, and the login thread waits up to `OLD_SAVE_WAIT` (Jacob's 5 seconds), a second at a time so a
    STOP SERVER is heard.  Past that the database is stuck: a Warn, and the new login gets Login
    Unavailable rather than come in on the save before.  The old session is out either way, and its save
    lands when the database catches up.
  - **The lock, both ways** (`sessions.rs`, `Lock`, `LOCK_FOR` of 1 second, fixed in code): Protogame locks
    a character for a second before it reads the row (`lock_for_loading()`), and a character that leaves
    the world is locked for a second, and after that for as long as the GameClock says it's saving.  A
    UserPressPlay for a locked character isn't read until the lock clears: the player is sent a PleaseWait
    (protocol version 10, "Your character is still being saved from its last session. One moment."),
    Protogame waits the lock out (`wait_for_loading_lock()`, the rest of its second slept, the save waited
    on the GameClock's bell, up to `LOCK_WAIT` of 5 s) and then plays it.  Jacob, 2026-10-02, after the
    second login's PLAY inside the first one's second got the Kicked: "the client is told to wait and then
    pulled in".  Past 5 s (the save is stuck) the player is sent a Kicked, reason `6`, and goes back to the
    login screen; their row reads "LINKDEAD: picked a character locked for a moment; logs in again".
    Protogame is one thread, so another player's ask waits behind the wait, nothing at these numbers.  This covers the other ways back in (Goodbye, gone
    quiet, a kick from the page, then a fresh login), where no login is waiting.  The lock is by uuid,
    lowercased.  A loading lock is swept once its second is over; a leaving one keeps the row id until STOP
    SERVER (one per character that left this run), so a later pick can still ask the GameClock about its
    save.  Today a load lock can't actually catch anything (Protogame is one thread, and an account has one
    session); it's there if that ever changes.
- **The log**: a Connect is "at character select" now (it said "in the world"), and "is in the world as
  Spawny" is its own Info line on the Security channel, once the character is in.
- **The Connections tab's UDP list** has the character beside the account ("character select", greyed,
  until there is one), and its count says how many are in the world and how many at character select.

## Chat (2026-10-02)

Jacob's command and packets, protocol version 8, the 0.0.1 release ("If we can get it where people can log
in and chat with each other... that's release 0.0.1").  PROTOCOL.md has the bytes.  Built and tested on Linux,
all nine checks passed, Ensemble still logging in on version 8.

- **The client sends what was typed**: "whatever is sent there is sent as a plaintext string to the server
  and the server goes "oh hey that started with / that means look for a command"".  **PlayerCommand**
  (`0x37`) is an ask number and the line.  There's one command, EverQuest's `/chat <message>`, any capitals.
  A line without a `/` "will default to being said -- something we won't implement yet but TODO!", so it's
  refused for now.
- **The message**: plain English ("letters numbers special characters, spaces"), printable ASCII, and only
  its first 300 characters ("The server will just ignore everything after 300"); the client will stop at
  300 itself.  Spaces at either end are left off; nothing left is refused.
- **Everybody in the world hears it, the speaker too**, as `[Chat] Jacob: Yo yo yo!`, one fixed channel
  ("like the way the old shit muds did it!").  Players at character select don't.
- **It goes out on the GameClock's beat**: "on the next "chat" GameClock tick that carries chat (which
  should be every beat)".  The UDP thread reads the line on the spot (`player-commands/src/chat.rs`; no
  database, nothing in the world), answers CommandAccepted or CommandRefused through the book like any ask, so a resend isn't
  said twice, and leaves the finished line in the GameClock's chat mailbox.  The broadcast check takes the
  cycle's lines and calls `chat::send_out()`, handed to the GameClock by `wire()` (it was networking's start)
  (`set_chat_sender()`), since the GameClock can't depend on networking: networking depends on it.  That
  builds **ChatDelivery** (`0x38`, a count and the lines, split to stay under 1200 bytes) and sends it to
  `sessions::in_world()` with `udp::tell_all()`, from the GameClock's thread.  Sent once; a lost one is lost.
- **The log**: every line said is a Debug on the Game channel, the account with it.
- **Later** (TODO.md): saying things without a `/`, nearby, once there are positions; a limit on how fast
  one player can chat; whether the web admin sees the chat; Ensemble's chat box.

## /who (2026-10-02)

Jacob's ask, protocol version 9: "a /who that shows all connected players".  PROTOCOL.md has the bytes and
the box.  Built and tested on Linux, every check passed.

- **Characters in the world only** ("I agree characters in the world only"), A to Z, to the one who asked.
- **`/who` is the names; `/who list` each with its block**: "A but if they do /who list It shows [Aldric]
  is currently at [0, 0, 0]".  Whole blocks, rounded down ("a block is the width of a player so they can
  only really fit on one").  A zone or biome name goes there later ("Eventually we will be putting in a
  biome name there (or zone)").
- **The client draws it**: "I like the idea of a who being a packet a list of names and the time from the
  server The Who was run".  The box is his old MUD's, "] Forgotten Legends [" and "There are seven legends
  currently online."  The time is seconds since midnight UTC (his pick), shown in the player's own time
  zone with the client's own date; the columns fit the chat box ("make it fit our actual chat size"); the
  count is written out by Ensemble's `Translator.NumberToWords()`, British ("IN the honor of Discworld!").
  So the server sends **WhoDelivery** (`0x39`): the ask, the seconds, list or not, the names (and blocks).
- **Where it's answered**: `/who` from the book on the UDP thread, on the spot
  (`player-commands/src/who.rs`).  `/who list` needs positions, which only the GameClock's thread reads, so it goes in the GameClock's `/who list`
  mailbox (`who_list()`), and the broadcast check reads every player's character's name and block once and
  calls `who::send_list()` for each ask (handed over as `set_who_sender()` when networking starts), which
  finishes the ask in the book and sends the answer.  The ask stays open in the book until then, so the
  client's resend in between is dropped, and one after gets the kept answer.
- **Spans** (`0x3A`), Jacob's "span packet": an answer over 1200 bytes goes in pieces, each saying which of
  how many, the client waiting 2 seconds at most for them all.  Any answer through `send_answer()` in
  `udp.rs` gets it.  `/who list` passes 1200 bytes at about thirty characters.
- **Everybody sees everybody's position**: "thats fine for now".  Who may see positions is for later.

## Commands and the anti-flood (2026-10-02)

Jacob, asking for a second's cooldown on `/who`, then: "I think we're doing this stupid.  We can just make
it so there's anti flood prevention on the server for any chat commands right?"  And on the shape: "in c#
mg temptation would be to make a command interface and then make it so we could easily stuff new commands
in", "the default should be 500 ms but if we make a command that hits the database a bunch maybe that
needs longer".  Built and tested on Linux, every check passed.

- **Every command is a line in `COMMANDS`** (`player-commands/src/lib.rs`): its name, its wait, and its
  `run()`, in a file of its own beside it.  A table of plain structs, like the GameClock's checks, rather than a
  trait: it does what a C# interface would.  A new command is a new file and a new line.
- **The anti-flood is in the one dispatcher**: after a command goes through, the player waits that
  command's wait before the next, whichever it is.  `DEFAULT_WAIT` is 500 ms, two game cycles; `/who` is 1
  second (his first number).  The book keeps each player's last command and its wait (`may_command()`).
  A line too soon gets a CommandRefused, "You can't do that again so soon.", before it's looked at any
  further, and doesn't push the wait back.  A line without a `/` or an unknown command waits the default.
  A resend of the same ask is answered from the book before it gets here.
- **The client shows the refusal's words** ("client interprets it as command can't be run so soon"); no
  reason number, so no protocol change.
- **Later** (TODO.md): kicking a player who keeps flooding.
- **A crate of its own** (2026-10-02, written, waiting on a build): `conductor-player-commands`, folder
  `Conductor/dev/player-commands/`.  Jacob, mid-way through Ensemble's chat window: "we need to rip the
  commands out of networking and put them into their own crate I think... conductor::player_commands then
  we'll probably also have admin_commands", "before we get too deep in commands".  `commands.rs` and
  `commands/` moved there whole (`lib.rs`, `chat.rs`, `who.rs`), tests and all.  It leans on networking
  and the GameClock, and networking never names it: `typed.rs` keeps `Asker`, `Outcome` and a slot
  (`set_runner()`), which `conductor_player_commands::wire()` fills, with the GameClock's two senders,
  from the launcher's `start_server()`, after the GameClock starts.  `lib.rs` re-exports what the commands
  need of the book and the UDP side (`may_command`, `in_world`, `names_in_world`, `finish_ask`,
  `tell_all`, `tell_answer`); the modules stay private.  With nothing in the slot, a line is refused with
  "Commands Unavailable" and the first is a Warn.  No thread, no service, nothing to stop.  Admin commands:
  "its a permissions difference but the commands will otherwise be the same".

## What's open

- **Client management** is all TODO: a player limit ("The server is full."), reconnecting with a token
  instead of a fresh hash, and anything an admin does to a player beyond KICK.
- **NAT rebinding.**  A player is their address, so a home router that changes the port mid-session ends
  the session.  A session id in each UDP packet would survive it.  Later, if it bites.
- **Windows**: it builds there (2026-09-30) but hasn't run networking yet (no world made, no certificate,
  no database).  The OS-specific parts are the three `dns/` files and the `ConnectionReset` line in
  `udp.rs`, Windows telling us about a bounced packet.  macOS gets no DNS names until there's a Mac.
- **What the client is sent after CharacterEnteredWorld**: the chat (above), and nothing else yet.  The
  world around it (chunks, `region.map`), other players and movement are the game's packets, to come.
