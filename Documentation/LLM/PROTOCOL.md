<!--
File:       Opus/Documentation/LLM/PROTOCOL.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Protocol

What Conductor and its clients say to each other, down to the byte.  Written for somebody building a
client who has never seen Conductor's code.  Conductor's half is `Conductor/dev/networking/src/protocol.rs`.
The Python test client beside the crate (`networking/test_client.py`) speaks all of it; Soundcheck, the
launcher (`Soundcheck/dev/Net/`), speaks the login over TCP, and Ensemble (`Assets/Code/Net/`) everything over
UDP, from the Connect the launcher's ticket earns it (2026-10-02); when any of them disagrees with this
document, it is the code that gets fixed.

Protocol version **17**.  The number goes up when a packet changes, and the server says it in the first
thing it sends, so a client built against another version can stop right there.  Version 17 (2026-10-03)
dropped the simple overworld map (Jacob: "we are dropping it... we don't need it anymore"): UserPressPlay is
answered with a **GroundOffer** (`0x40`), only where the character will stand and how many chunks it sees;
OverworldMapRequest (`0x41`) and OverworldMapPiece (`0x42`) are gone and their numbers never used again;
PlayerReady carries its ask number and nothing else; and the account's map cooldown went with the map
(below, "The way into the world").  Version 16 (2026-10-03)
changed the block kinds a squeezed chunk can carry, and no packet's shape: GOLD (4) is gone and never used
again, and MASONED_STONE (6), grey bricks, is new (below, "A chunk, squeezed").  Version 15 (2026-10-03)
added movement, EverQuest's way: the client walks its own character and says where it went with a
**PlayerMoved** (`0x55`), and the server takes each move or pulls the character back to its last good spot
with a **MoveCorrection** (`0x56`) (below, "Movement").  CharacterEnteredWorld now ends with how fast a
character walks and turns, the Hydrate carries the room an object takes up (its collider) after its shape,
and a player is no longer sent their own character's moves.  Version 14 (2026-10-03)
added the world's objects, the group `0x5_`: what a player sees of the world around them, the server
deciding where everything is and the client only drawing it.  Hydrate (`0x50`) sends an object whole as it
comes into view, ObjectsMoved (`0x51`) where known ones are now, ObjectsGone (`0x52`) the ones that left, and
RollCall (`0x53`), once a second, everything the player is believed to know; the client asks about a number it
doesn't know with an ObjectAsk (`0x54`).  CharacterEnteredWorld now ends with the number of the player's own
character (below, "The world's objects").  Version 13 (2026-10-03)
made `/who` a line a character, EverQuest's way: WhoDelivery lost its list byte (`/who list` went into
`/who`), and every character in it carries where it stands and its seconds online, the one in the world
longest first (below, "/who").  Version 12 (2026-10-03)
added the chunks around the player, pulled by the client: ChunkRequest (`0x43`), ChunkPiece (`0x44`) and
ChunkRefused (`0x45`), each chunk squeezed as runs (below, "The chunks around the player"); and the
OverworldMapOffer now ends with where the character will stand and how many chunks each way it sees, so the
client knows which chunks to ask for before PlayerReady.  Version 11 (2026-10-03)
added the simple overworld map at PLAY, and the group `0x4_`, the ground: UserPressPlay is answered with an
OverworldMapOffer (`0x40`) instead of the character going straight in, the client fetches the map with
OverworldMapRequests (`0x41`) and OverworldMapPieces (`0x42`), and **PlayerReady** (`0x29`) puts the
character in the world, answered with the CharacterEnteredWorld that used to answer UserPressPlay (below,
"The way into the world", as it is now).  Version 10 (2026-10-02)
added PleaseWait (`0x3B`): the server is working on an ask and says it'll take a moment, with the words to
show; the answer follows under the same ask number.  A UserPressPlay for a character locked for a moment
gets one, and the lock is waited out, where before it got a Kicked.  Version 9 (2026-10-02)
added `/who`: WhoDelivery (`0x39`), and Span (`0x3A`), an answer too big for one packet in pieces (below,
"Answers in pieces").  Version 8 (2026-10-02)
added chat: PlayerCommand (`0x37`), a line the player typed, and ChatDelivery (`0x38`), the chat going out to
everybody in the world (below, "In the world, over UDP").  Version 7 (2026-10-02)
made the Login's fourth string the password's key instead of the password (below, "The password's key").
Version 6 (2026-10-01)
added the spawn: UserPressPlay and CharacterEnteredWorld (`0x27`, `0x28`), and reason `6`, character
locked for a moment, to Kicked.  Version 5 (2026-09-30)
added character select (`0x20` to `0x26`) and the two general answers, CommandAccepted and CommandRefused
(`0x35`, `0x36`).  Version 4 (2026-09-29)
added reason `5`, account terminated, to Kicked.  Version 3 (the same day) added reason `4`, kicked by
the admin.  Version 2 (the same day) added reason `3`, banned.  Version 1 was everything before it.

## The shape of it

A session is two halves, over two transports.

1. **TCP, inside TLS, is the login.**  The client connects to the TCP port, TLS comes up, the server says
   Hello, the client sends one Login, and the server answers with a Ticket or a LoginResult.  Then the
   server closes the connection.  Nothing else ever goes over TCP.
2. **UDP is the game.**  The client sends the Ticket's token to the UDP port in a Connect, the server
   answers with a ConnectResult, and from then on everything goes over UDP: a KeepAlive each way once a
   second, character select, the world's map fetched at PLAY, the character picked coming into the world,
   and the game's packets as they come.

When the UDP session ends, for any reason, the player is gone.  There is no reconnect: the client goes back
to the login screen and starts over from TCP.  The reasons it ends: the client sent a Goodbye, the client
went quiet past the server's UDP timeout (40 seconds by default), the account logged in from somewhere
else and chose to log this session out, the server stopped, the admin banned the address (put it on
the blacklist, or took it off the whitelist), the admin kicked the player, the admin deleted the
account, or the player picked a character locked for a moment (below).  For the last six
the client hears a Kicked first; for the timeout it hears nothing, and knows from its own silence.  A
character in the world leaves it with its player's session, saved on the way out; there's no going back
to character select from the world but logging out and in again.

### TLS

TLS 1.3 or 1.2.  It was 1.3 only until Ensemble logged in itself (2026-10-02): Unity's .NET can't speak
1.3, so the server took 1.2 as well.  The login is Soundcheck's now, whose .NET speaks 1.3, so 1.2 is on its
way out again.  Nothing in the packets changed, so the protocol version didn't either.  The server has one
self-signed certificate, and a client trusts that certificate and no
authority: it keeps a copy of the certificate file and refuses any server that shows it a different one.
Nothing in the login goes over the wire until TLS is up.

### Bytes

Numbers are **little-endian** (lowest byte first).  An **f32** is a 4-byte IEEE 754 float (C#'s `float`,
what `BinaryWriter` writes).  A **string** is a u32 byte count and then that many
bytes of UTF-8, no terminator.  Bytes that aren't UTF-8, or anything left over after a packet's last
field, make the packet one the server can't read.

Every TCP packet goes in a frame:

```text
offset  size  what
0       4     length, u32: the type byte plus the payload.  Not counting itself.
4       1     type
5       n     payload
```

The largest length a frame may claim is 4096, and the smallest 1.  Anything else is treated as a failed
login (below): a LoginResult `1`, the 2-second hold, and the connection closed.

A UDP packet has no length in front, since UDP keeps packets whole:

```text
offset  size  what
0       1     type
1       n     payload
```

The largest UDP packet the server takes is 1200 bytes.  A larger one is dropped without a word.

### Packet types

The high four bits are the group, the low four which one in it.  `0x1_` is the login, over TCP.  `0x2_` is
character select, between the login and the world, over UDP.  `0x3_` is the game, over UDP.  `0x4_` is the
ground, over UDP: PLAY's GroundOffer, and the chunks around the player.  `0x5_` is the world's
objects, over UDP: what the player sees standing in the world around them, and since version 15 the player
walking their own.

| Type   | Name           | Way              | Payload                                                  |
|--------|----------------|------------------|----------------------------------------------------------|
| `0x10` | Hello          | server to client | u8: protocol version                                     |
| `0x11` | Login          | client to server | string version, string secret word, string username, string key |
| `0x12` | InLine         | server to client | u32 how many logins are ahead, u32 about how many ms     |
| `0x13` | LoginResult    | server to client | u8 answer, string message                                |
| `0x14` | SessionChoice  | client to server | u8: 0 log the other session out, 1 hang this one up      |
| `0x15` | Ticket         | server to client | string token (64 hex characters), u16 UDP port           |
| `0x20` | CharacterListRequest | client to server | u32 ask                                            |
| `0x21` | CharacterListDelivery | server to client | u32 ask, u8 count, then each: string uuid, string name, u8 slot, u8 playable |
| `0x22` | CreateCharacter | client to server | u32 ask, string name                                    |
| `0x23` | CharacterCreateResult | server to client | u32 ask, u8 answer, string message              |
| `0x24` | DeleteCharacter | client to server | u32 ask, string uuid, string the typed word            |
| `0x25` | CharacterDeleteResult | server to client | u32 ask, u8 answer, string message              |
| `0x26` | CharacterRequestResetHome | client to server | u32 ask, string uuid                          |
| `0x27` | UserPressPlay  | client to server | u32 ask, string uuid                                     |
| `0x28` | CharacterEnteredWorld | server to client | u32 ask, string uuid, string name, f32 x, y, z, u32 object, f32 walk, f32 turn |
| `0x29` | PlayerReady    | client to server | u32 ask                                                  |
| `0x30` | Connect        | client to server | string token                                             |
| `0x31` | ConnectResult  | server to client | u8 answer, string message                                |
| `0x32` | KeepAlive      | both ways        | nothing                                                  |
| `0x33` | Goodbye        | client to server | nothing                                                  |
| `0x34` | Kicked         | server to client | u32 reason                                               |
| `0x35` | CommandAccepted | server to client | u32 ask                                                 |
| `0x36` | CommandRefused | server to client | u32 ask, string why                                      |
| `0x37` | PlayerCommand  | client to server | u32 ask, string the line as typed                        |
| `0x38` | ChatDelivery   | server to client | u8 count, then that many strings, each a finished line   |
| `0x39` | WhoDelivery    | server to client | u32 ask, u32 seconds since midnight UTC, u16 count, then each: string name, i32 x, y, z, u32 seconds online |
| `0x3A` | Span           | server to client | u32 ask, u8 piece, u8 pieces, then the piece's bytes     |
| `0x3B` | PleaseWait     | server to client | u32 ask, string words                                    |
| `0x40` | GroundOffer    | server to client | u32 ask, f32 x, y, z, u8 view                            |
| `0x43` | ChunkRequest   | client to server | u8 how many (1 to 64), then each: i16 x, i16 z, u8 row   |
| `0x44` | ChunkPiece     | server to client | i16 x, i16 z, u8 row, u8 piece, u8 pieces, then the piece's bytes |
| `0x45` | ChunkRefused   | server to client | i16 x, i16 z, u8 row, u8 why                             |
| `0x50` | Hydrate        | server to client | u32 object, string uuid, u8 living, string short name, 9 f32 motion, 3 f32 scale, string model, u8 shape, u8 collider, 3 f32 collider size, string doing |
| `0x51` | ObjectsMoved   | server to client | u8 count, then each: u32 object, 9 f32 motion            |
| `0x52` | ObjectsGone    | server to client | u16 count, then each: u32 object                         |
| `0x53` | RollCall       | server to client | u32 roll, u8 piece, u8 pieces, u8 count, then each: u32 object, 9 f32 motion |
| `0x54` | ObjectAsk      | client to server | u8 how many (1 to 64), then each: u32 object             |
| `0x55` | PlayerMoved    | client to server | u32 move, u32 last pull-back had, 9 f32 motion (no object number) |
| `0x56` | MoveCorrection | server to client | u32 pull-back, f32 x, y, z, f32 rotation x, y, z         |

## The login, over TCP

1. The client connects and TLS comes up.  The client has until the server's login deadline (10 seconds by
   default) from the moment it connected to get through step 3.
2. The server sends **Hello** with the protocol version.
3. The client sends one **Login**: its version, the secret word, the username and the password's key
   (below), never the password as typed.  The username is folded to lowercase on the server.  The secret word is not a secret from anybody with a copy
   of the client; it turns port scanners away before they cost the server a hash.
4. While the password is checked, the server may send **InLine** once a second: how many logins are ahead
   of this one and about how long that is, since the server checks one password at a time.  A client
   shows it or ignores it.
5. The server answers with one of:
   - **Ticket**: the login worked.  The token and the UDP port to take it to.  The server closes the
     connection right after (a proper TLS close).
   - **LoginResult** with answer `1`, "Invalid Credentials": the secret word, the username or the password
     was wrong.  One answer for all three, on purpose.  The address then can't connect for 2 seconds (the
     hold; answers `3` and `4` don't start one).  A username, once lowercased, is 8 to 32 characters of
     `a-z`, `0-9` and `_`; one that isn't gets this answer without the server looking it up.  So does a
     key that isn't 64 characters of `0-9` and `a-f` (a password sent as typed, say).
   - **LoginResult** with answer `2`, "This account is already logged in.": the password was right, and the
     account is in the world from somewhere else.  Only ever sent after the right password.  The
     connection stays open, and the client has 30 seconds to answer with a **SessionChoice**: `0` logs the
     other session out (it hears a Kicked) and this login carries on to a Ticket, once the other session's
     character, if it had one in the world, has its save in the database (normally a fraction of a second;
     if that takes over 5 seconds, this login gets a LoginResult `4` instead); `1` hangs this one up and
     leaves the other alone.  (So a shared account doesn't kick your brother off because you wanted to
     play.)
   - **LoginResult** with answer `3`, "Outdated Client Failure": the client's version isn't on the server's
     list.  Checked before the password.
   - **LoginResult** with answer `4`, "Login Unavailable": the server can't check logins right now, or
     couldn't make a token after the right password.  Nothing the player did.

   Every answer takes at least 150 ms from the moment the Login arrived, whichever it is, so the time it
   takes can't tell a real username from a made-up one.

### The password's key

The client never sends the password as typed.  It sends a key made from it, and the server only ever sees
the key: the account's stored hash is of the key.  The client may keep the key on the player's disk
(Remember Me) and send it again without the password.  The key is:

- **PBKDF2 with HMAC-SHA256**, over the password's UTF-8 bytes as typed (the password rules keep it to
  printable ASCII).
- **Salted** with the UTF-8 bytes of `Opus login v1:` followed by the username with `A` to `Z` made
  lowercase and nothing else touched.  So `Jacob_01` and `jacob_01` make the same key.
- **600,000 rounds, 32 bytes**, written as **64 lowercase hex characters**.  That string is the Login's
  fourth string.

`jacob_01` with the password `Correct horse 1!` makes
`fc71f0c94665dfd6ff4e217891cd7ff81c8fd8ed7b20cf498c122c1bbd1f8855`.  Changing any of this locks out every
account there is.

Anything out of turn as the first packet after the Hello (a type the server doesn't expect, a Login it
can't read) is treated as a failed login.  While the server waits on a SessionChoice, anything but a
readable SessionChoice closes the connection without an answer and without the hold.  A client that hasn't
logged in when the deadline passes is closed without an answer, and so is one that stops reading: a send
the server can't finish in 5 seconds ends it.

The message string is for the player, as it is.  Failures the player can act on are sentences; ones they
can't are short labels with no period.

## The game, over UDP

1. The client sends **Connect** with the token, and sends it again every half second until it hears back.
   The token is good once, for 30 seconds from the Ticket by default, and only from the first address that
   uses it; the same address asking again (because the answer got lost) gets the same answer.
2. The server answers with **ConnectResult**: `0` "Welcome to the world." and the client is in, or `1`
   "Invalid Credentials" and it isn't (a token it doesn't know, one already used from another address, or
   a player's own address asking with a different token).  A Connect it can't read (a token that isn't
   exactly 64 characters, bytes left over, a packet over 1200 bytes) gets no answer at all, and neither
   does one from an address the access lists turn away.  A ConnectResult is always smaller than a Connect,
   so the server can't be used to flood a faked address.
3. The client sends **KeepAlive** once a second, and the server sends one straight back.  A client that
   hears none for a while should assume the server is gone and go back to the login screen.  The server
   drops a player it hasn't heard from in its UDP timeout (40 seconds by default), silently.
4. **Goodbye** from the client ends the session.  No answer.
5. **Kicked** from the server ends the session: reason `1` the account logged in elsewhere and chose to
   log this session out, `2` the server is stopping, `3` the address was banned (the admin put it on the
   blacklist, or took it off the whitelist; its next login is closed at the door), `4` the admin kicked
   them (nothing stops them logging in again), `5` the admin deleted the account (the client says
   ACCOUNT TERMINATED; the account is gone, so logging in again fails), `6` the character picked with
   UserPressPlay was still locked after the server waited 5 seconds for it (its save from its last
   session is stuck; logging in again tries again).  The client goes back to the login screen.

Anything else from an address the server knows counts as hearing from that player (more of the game's
packets go here later).  Anything at all from an address it doesn't know, other than a Connect, gets no answer.

## Character select, over UDP

Once the ConnectResult says Welcome, the player is at character select, until a UserPressPlay puts their
character in the world.  A player only ever sees, makes and deletes characters on the account
they logged in as: the server goes by who logged in, never by anything in the packet.

**The ask number.**  Every packet the client sends here starts with a u32 it picks, one higher for every new
ask, and the answer carries it back.  UDP can lose a packet either way, so a client that hears nothing in
half a second sends the same ask again, same number and all.  If the server has answered that number, it
sends the same answer again rather than doing it twice (a second CreateCharacter would find its own name
taken).  A player has one ask at a time: one sent while the last is still being worked on is dropped, and
the client's resend picks it up once it's done.  An answer for an ask number the client has moved past is
an old one, and the client ignores it.  One that can't be read gets no answer.

- **CharacterListRequest** gets a **CharacterListDelivery**: every character on the account, in slot order,
  3 at most.  For each: its uuid, its name, its slot (1 to 3), and playable, `1` or `0`.  `0` means its save
  wouldn't load this run; the client greys it out, and the server won't let it in.  (It stays that way
  until the admin stops and starts the server.)  A list the server can't read right now gets a
  **CommandRefused** instead.  A client doesn't offer CREATE when all three slots are full.
- **CreateCharacter** with a name gets a **CharacterCreateResult**.  A name is 4 to 20 letters, a to z, and
  only the first can be a capital; every name is unique on the server whatever the capitals.  The
  character goes in the account's first empty slot, at 0, 0, 0.  The answers:
  - `0` "Your character has been made."
  - `1` "A character's name is 4 to 20 letters, a to z, and only the first can be a capital."
  - `2` "That name is taken."
  - `3` "All three character slots are full."
  - `4` "Character Creation Unavailable": the server can't make one right now.  Nothing the player did.
- **DeleteCharacter** with the character's uuid and the word the player typed gets a
  **CharacterDeleteResult**.  The client asks the player to type DELETE before it sends it, and only
  DELETE deletes (any capitals).  Answer `0` approved: the character is gone.  `1` denied, and the message
  says why: the word was wrong, there's no such character on the account, or the server can't do it right
  now.
- **CharacterRequestResetHome** with a character's uuid puts it back at its spawn point (0, 0 today; it
  stands on top of the highest block there, in the middle of it, since 2026-10-03; it was 0, 0, 0) and gets
  a **CommandAccepted**, or a **CommandRefused** saying why not (no such character on the account, the
  character is unplayable, its save won't load, or the server can't right now).  A new character starts at
  the same place.

- **UserPressPlay** with a character's uuid loads it, where its last save left it, and gets a
  **GroundOffer** (version 17): the character is waiting, and this is where it will stand, for the client
  to fetch the ground around it before it comes in (below, "The way into the world").  Or a
  **CommandRefused** saying why not: no such character on the account, the character is unplayable, its save
  won't load (it's marked unplayable then, and the admin told), or the server can't right now.  (From version
  11 to 16 the answer was an OverworldMapOffer, and an account sent the map too recently was refused.)  Then
  **PlayerReady** brings it into the world and
  gets a **CharacterEnteredWorld**: its uuid and name, where it stands, x, y and z (y up), and the number
  its client knows it by among the world's objects (version 14; below, "The world's objects").  Before
  version 11 the CharacterEnteredWorld answered UserPressPlay itself.
- **A character is locked for a moment** whenever it moves between the database and the world: for 1
  second from the moment the server starts loading it, and for 1 second after it leaves the world, longer
  if its save from leaving hasn't reached the database yet.  A UserPressPlay for a locked character isn't
  read until the lock clears: the client gets a **PleaseWait** (version 10) with the words to show, "Your
  character is still being saved from its last session. One moment.", the server waits the lock out, and
  the GroundOffer (or a CommandRefused) follows under the same ask number.  The client keeps
  resending the ask meanwhile, as for any ask, and the server drops the resends.  If the lock is still
  held after 5 seconds the client gets a **Kicked** with reason `6` instead, and logs in again.  So one
  character is never brought in twice at once, or on the save before its last.
- **While the character waits to come in**, any other ask from character select gets a **CommandRefused**,
  "Your character is on its way into the world.", and a PlayerCommand gets "Commands work once your
  character is in the world."
- **Once the character is in the world, character select is behind the player.**  Any of the asks above
  gets a **CommandRefused**, "Your character is in the world.  Log out to get back to character select."
  The way back is logging out, to the login screen, every time (Jacob: "you log out back to log in screen
  every time").

**CommandAccepted** (the ask number) and **CommandRefused** (the ask number and a message for the player)
are general answers: any command that needs no more said than done or not done, here or in the game later,
gets one of them.

A character select exchange, a CreateCharacter as ask 2 for "Jacob":

```text
22                                            CreateCharacter
02 00 00 00                                   ask 2
05 00 00 00  4A 61 63 6F 62                   "Jacob"
```

and the answer, "Your character has been made." being 29 bytes:

```text
23                                            CharacterCreateResult
02 00 00 00                                   ask 2
00                                            made
1D 00 00 00  59 6F 75 72 20 ...               "Your character has been made."
```

A UserPressPlay as ask 3 is below, under "The way into the world", with the rest of it.

## The way into the world

Version 17 (2026-10-03).  A character isn't in the world until the client says the ground around it is in
and drawn (Jacob: "it doesn't show them or spawn them in the physical world until they're ready").

1. The client sends **UserPressPlay** (an ask).  The server loads the character and holds it, and answers
   with a **GroundOffer** under the same ask number: where the character will stand, x, y and z, and how
   many chunks each way it sees, `view_chunks` (4 by default), so the client can ask for the chunks around
   it (below, "The chunks around the player").  A lost offer is sent again for the ask sent again, like any
   answer.
2. The client pulls the chunks around that spot and draws them.  When it has enough is its own call.
3. The client sends **PlayerReady** (a new ask), its ask number and nothing else.
4. The server puts the character in the world, and answers with **CharacterEnteredWorld**.  Or a
   **CommandRefused**: "World Unavailable" when the server can't, "There's no character waiting to come
   into the world.  Press PLAY first." with no UserPressPlay before it, "Your character is already in the
   world." after one went in.

Ensemble goes back to the launcher with the server's words when PlayerReady is refused or not answered.
Leaving while the character waits leaves nothing behind: it was never in the world, so there's nothing to
save.

**Before version 17** the GroundOffer was the **OverworldMapOffer** (version 11): the simple overworld map's
size, its 1,024-byte pieces and its SHA-256 came first, the client fetched the map with OverworldMapRequests
(`0x41`) and OverworldMapPieces (`0x42`), every PLAY, and PlayerReady carried the hash of what it got.  The
map was the world's rough shape for drawing the distance, and was dropped with the distance (smooth voxels,
`design/smooth-voxels.md`): nothing is drawn past the view.

A UserPressPlay as ask 3, the uuid shown short (it's 36 characters):

```text
27                                            UserPressPlay
03 00 00 00                                   ask 3
24 00 00 00  30 31 39 39 ...                  the uuid, 36 bytes

40                                            GroundOffer
03 00 00 00                                   ask 3
00 00 C0 3F  00 00 00 00  00 00 00 C0         Jacob will stand at 1.5, 0, -2
04                                            and sees 4 chunks each way
```

Then, once the ground around Jacob at 1.5, 0, -2 is in and drawn:

```text
29                                            PlayerReady
04 00 00 00                                   ask 4

28                                            CharacterEnteredWorld
04 00 00 00                                   ask 4
24 00 00 00  30 31 39 39 ...                  the uuid
05 00 00 00  4A 61 63 6F 62                   "Jacob"
00 00 C0 3F                                   x 1.5
00 00 00 00                                   y 0
00 00 00 C0                                   z -2
07 00 00 00                                   object 7: Jacob, among the world's objects
00 00 80 40                                   walks 4 blocks a second
00 00 E1 43                                   turns 450 degrees a second
```

## The chunks around the player

Version 12 (2026-10-03).  The chunks are the real ground near the player, block by block; nothing is drawn
past them (since version 17; before it, the simple overworld map drew the distance).
**The client pulls them** (Jacob: "Pull"): it works out which chunks it hasn't got around where its
character stands, and asks for them.  The server keeps no list of who has what; it checks each chunk asked
for and sends it, or says why not.

**Which chunks.**  A chunk is a cube of 32 blocks a side (a block is 1 m), placed by x and z, counted in
chunks east and north from 0,0 (chunk 0,0 has its south-west corner at block 0,0, and block -1 is in chunk
-1), and its **row**, 0 to 10 from the bottom: row 0 is blocks -32 to -1, row 1 is 0 to 31, row 10 is 288
to 319.  A player may have every chunk within the offer's **view** of the column their character stands in,
east, west, north and south, every row: at 4, a square 9 chunks across, 11 rows, 891 chunks.  The column of
a position is its x and z rounded down, then divided by 32 rounded down (1.5, 0, -2 is block 1, -2, in
column 0, -1).  Nobody moves yet, so the column is the one in the offer.

1. The client sends a **ChunkRequest**: how many chunks, 1 to 64, then each one's x, z and row.  No ask
   number: asking for a chunk twice is harmless.  Only a player whose character is waiting to come in or is
   in the world gets an answer; anybody else, nothing.
2. For each chunk asked for, the server sends its **ChunkPieces**: the chunk's x, z and row, which piece
   (from 1) and how many, then up to 1,192 bytes of the chunk, squeezed (below).  The pieces' bytes put
   together in order are the squeezed chunk.  Most chunks are one piece.  Or a **ChunkRefused** with the
   chunk's place and why:
   - `1` outside the view: not within the view of the character's column, or past the edge of the world.
     Asking again won't help.
   - `2` not yet: the server is reading or building it.  Ask again in a moment.
   - `3` unavailable: it can't be had this run (its file on the server doesn't read right).  Asking again
     won't help.
3. The client asks again, after a moment, for whatever hasn't come whole and wasn't refused for good.

Ensemble and the test client ask the same way, once the GroundOffer is in: the nearest first (by the larger of
how far east-west and north-south, then by how far its row is from the character's), the first 64 not yet
in, waits for them or a quarter of a second, and asks again; they give up with no new chunk in 10 seconds.
**When the client has enough to send PlayerReady is its own call** (Jacob: "it can be the clients call but
I think we are gonna want to wait till most of the scene is filled"): the server checks nothing about it.
Ensemble sends it once the nearest 99 are in, the 3 by 3 columns round the character's, every row
(Jacob, 2026-10-03: Minecraft's way), and drawn on screen (the same day, later), and the rest keep coming
with the character in the world.

### A chunk, squeezed

A chunk is 32,768 blocks, two bytes each, 64 KB as it is.  It's sent as **runs** instead: the kinds of
block in it, listed once, then "this many of that one", in the same order as the chunk's file on the
server: the bottom layer first; in a layer, the south row first; in a row, west to east.  So block x, y, z
inside the chunk (each 0 to 31) is number `(y * 32 + z) * 32 + x`.  Every number is little-endian.

```text
u8           how it's squeezed: 1, runs.  The only one there is; a client that gets another can't read it.
u16          how many kinds of block, 1 to 32,768
u16 x kinds  the kinds, by their block numbers, in the order they first turn up
then runs, until all 32,768 blocks are covered:
  the run's length less one, 0 to 32,767: one byte for 0 to 127; for more, the low 7 bits with
    the top bit set (0x80), then the rest (the number shifted right 7) in a second byte
  which kind, as its place in the list from 0: a u8 when the list has 256 kinds or fewer, a u16
    when it has more
```

Nothing is left over after the last run, and the runs never come to more than 32,768 blocks.  The block
numbers are the world's: AIR 0, DIRT 1, STONE 2, WOOD 3, BEDROCK 5, MASONED_STONE 6 (version 16: 4 was
GOLD, and is never used again; nothing makes MASONED_STONE yet); a number never changes once it's
out there.  An all-air chunk is 8 bytes, a flat one 13; a chunk of Omega's hills under 700 (measured,
2026-10-03: a whole view of 3,179 chunks came to 88,746 bytes, every chunk one piece).  The worst there is, every
block different, is 163,843 bytes, 138 pieces.

### A worked example

Chunk -1, 0, row 1 is Alpha's flat ground just west of 0,0: one layer of DIRT at y 0, AIR over it.
Asked for with the chunk 300, -2 row 10 (outside the view of a character at 1.5, 0, -2):

```text
43                                            ChunkRequest
02                                            2 chunks
FF FF  00 00  01                              -1, 0, row 1
2C 01  FE FF  0A                              300, -2, row 10

44                                            ChunkPiece
FF FF  00 00  01                              -1, 0, row 1
01 01                                         piece 1 of 1
01                                            squeezed as runs
02 00                                         2 kinds
01 00  00 00                                  DIRT, AIR
FF 07  00                                     1,024 less one is 1,023 (0x3FF): FF 07, of DIRT
FF F7  01                                     31,744 less one is 31,743 (0x7BFF): FF F7, of AIR

45                                            ChunkRefused
2C 01  FE FF  0A                              300, -2, row 10
01                                            outside the view
```

A C# reader for the squeezed bytes, for Ensemble:

```csharp
// The blocks of a squeezed chunk, in the file's order: (y * 32 + z) * 32 + x.
static ushort[] Unsqueeze(byte[] bytes)
{
    const int Blocks = 32 * 32 * 32;
    using var reader = new BinaryReader(new MemoryStream(bytes));
    if (reader.ReadByte() != 1)
        throw new InvalidDataException("not squeezed as runs");
    int count = reader.ReadUInt16();
    if (count < 1 || count > Blocks)
        throw new InvalidDataException(count + " kinds");
    var kinds = new ushort[count];
    for (int i = 0; i < count; i++)
        kinds[i] = reader.ReadUInt16();

    var blocks = new ushort[Blocks];
    int at = 0;
    while (at < Blocks)
    {
        int first = reader.ReadByte();
        int length = first < 128 ? first + 1 : ((reader.ReadByte() << 7) | (first & 0x7F)) + 1;
        int place = count > 256 ? reader.ReadUInt16() : reader.ReadByte();
        if (place >= count || at + length > Blocks)
            throw new InvalidDataException("a bad run");
        Array.Fill(blocks, kinds[place], at, length);
        at += length;
    }
    if (reader.BaseStream.Position != bytes.Length)
        throw new InvalidDataException("bytes left over");
    return blocks;
}
```

## In the world, over UDP

Once a CharacterEnteredWorld has come, the player is in the world.  For now what they can do there is chat,
see who's around them (below, "The world's objects") and walk (below, "Movement").  The CharacterEnteredWorld
ends with two f32s (version 15): how fast the character **walks**, in blocks a second (4), and how fast it
**turns**, in degrees a second (`turn_degrees_per_second` in the server's `player.cfg`, 450 by default).
They're the server's numbers, and the moves the client sends are held to them.

**PlayerCommand** carries a line the player typed in the client's chat window, as it was typed, with an ask
number like character select's: the same number again gets the same answer again, so a line whose answer
got lost isn't said twice, and one ask at a time.  A line that starts with `/` is a command; the word after
the `/`, up to the first space or the end, says which, with any capitals.  There are two so far:

- **`/chat <message>`** says the message to everybody in the world.  The message is what comes after the
  first space, spaces at either end left off.  It's plain English: letters, numbers, punctuation and the
  space, bytes `0x20` to `0x7E`.  Only its first 300 characters count, and anything after them is dropped
  without a word (a client stops the player at 300 anyway).

**Anti-flood.**  After a command goes through, the player waits before the next one, whichever it is:
500 ms for most (two game cycles), 1 second after a `/who`, longer for any later command that costs the
server more.  A line in that wait gets a **CommandRefused**, "You can't do that again so soon.", and
doesn't make the wait any longer.  A line without a `/`, or a command there's no such thing as, waits the
500 ms too.  The same ask sent again isn't a new line, so it gets the answer it missed, not this.

The answer is a **CommandAccepted** when the line goes out, or a **CommandRefused** saying why it doesn't:

- "Saying things without a command isn't in yet.  Use /chat." for a line without a `/` (it'll be said
  out loud, nearby, later).
- "There's no command by that name.  For now there's /chat and /who."
- "Say something after /chat." for a `/chat` with nothing after it.
- "Chat is plain English: letters, numbers, punctuation and spaces." for anything else in the first 300.
- "Commands work once your character is in the world." from character select, for any command.
- "Chat Unavailable": the server can't right now.  Nothing the player did.
- "Commands Unavailable": nothing in the server runs commands (it was put together wrong).  Nothing the
  player did.

**ChatDelivery** is the chat going out.  The server gathers everything said and sends it once a game cycle
(every 250 ms, when there's anything to send) to every player whose character is in the world, the one who
said it too: a u8 count, then that many strings, oldest first, each the line as the player sees it, one
fixed channel:

```text
[Chat] Jacob: Yo yo yo!
```

A cycle with more than fits in 1200 bytes (or more than 255 lines) goes out as more than one ChatDelivery,
in order.  It carries no ask number and isn't answered; a lost one is lost.  Players at character select
don't get it.

A `/chat` as ask 4, and what everybody gets:

```text
37                                            PlayerCommand
04 00 00 00                                   ask 4
0F 00 00 00  2F 63 68 61 74 20 59 6F ...      "/chat Yo yo yo!", 15 bytes

35                                            CommandAccepted
04 00 00 00                                   ask 4

38                                            ChatDelivery
01                                            1 line
17 00 00 00  5B 43 68 61 74 5D 20 4A ...      "[Chat] Jacob: Yo yo yo!", 23 bytes
```

### /who

**`/who`** asks who's in the world and where each of them stands (version 13; `/who list` was the
second half until then, and went into `/who`).  Only characters in the world count (players at character
select don't), and only the one who asked gets the answer, a **WhoDelivery** carrying the ask number, so
it's sent again for a resend like any answer:

- the time it ran, in **seconds since midnight UTC** (0 to 86,399), a u32;
- a u16 count, then each character, **the one in the world longest first** and the newest last: its
  name; its x, y and z, each an i32 in **whole blocks**, rounded down (1.5 is block 1, -1.5 is block -2);
  and its **seconds online**, a u32, counted from when it came into the world (PlayerReady), not from the
  login.

`/who` waits for the next game cycle, since where everybody stands is the GameClock's, so its answer comes
up to 250 ms later.  Anything after `/who` (`/who list` included) gets a **CommandRefused**, "Try
/who.", and "Who Unavailable" means the server can't right now.

The client draws the rest, EverQuest's way (Jacob, 2026-10-03, "since the chat window is scaleable"): a
plain line a character, so nothing is laid out to the chat box's width and a long line just wraps; a blank
line; the count in digits; and the time it ran, in the player's own time zone (the date from its own
clock, the time from the packet).  Time online is days, hours and minutes, the ones that are 0 left out,
"1 minute" for one, and "under a minute" before the first; seconds never show.

```text
Chatter is at [0, 0, 0] [16 days, 12 minutes online]
Seliris is at [15, 1, 20] [3 hours, 4 minutes online]

There are 2 Legends online.
Sat Oct  3 03:53:24 2026
```

For one: "There is 1 Legend online."  A `/who` as ask 5 at 03:53:24 UTC (14,004 seconds, `0x36B4`), with
Aldric in the world for 16 days and 12 minutes (1,383,120 seconds, `0x151AD0`) and Jacob for 90 seconds:

```text
37                                            PlayerCommand
05 00 00 00                                   ask 5
04 00 00 00  2F 77 68 6F                      "/who"

39                                            WhoDelivery
05 00 00 00                                   ask 5
B4 36 00 00                                   14,004 seconds after midnight UTC
02 00                                         2 characters
06 00 00 00  41 6C 64 72 69 63                "Aldric"
00 00 00 00  00 00 00 00  00 00 00 00         at 0, 0, 0
D0 1A 15 00                                   1,383,120 seconds online
05 00 00 00  4A 61 63 6F 62                   "Jacob"
01 00 00 00  00 00 00 00  FE FF FF FF         at 1, 0, -2
5A 00 00 00                                   90 seconds online
```

## The world's objects

Version 14 (2026-10-03).  Jacob: **"The server will be the authority, always on where the object actually
is in the world.  The client is just a dumb renderer."**  So the client draws an object where the server
last said it is, its own character included, and never decides a place of its own.

**What a player sees**: every object within the offer's **view** of the column their character stands in,
the same square as the chunks they may have (`view_chunks` each way, every row), so nothing is shown
standing on ground the client hasn't got.  Today the objects are players' characters; NPCs join them when
there are NPCs.  Each object has a **number**, a u32 from 1 up, handed out when it comes into the world and
never used again while the server runs; every packet after the Hydrate names it by that, four bytes, not
its uuid.  0 is never a number.

**A motion** is 40 bytes: the object's number, then nine f32s, its **position** (x, y and z in blocks, y up;
a character's is its feet), its **rotation** (degrees about x, y and z, the way Unity has them) and its
**velocity** (blocks a second along x, y and z).  The client moves the object along its velocity every
frame until it's told otherwise, so the server only speaks when something changes.  The player's own
character is the one exception (version 15): the client walks it itself, and takes its place from a
MoveCorrection only, never from an ObjectsMoved or the roll call (below, "Movement").

**Only what changed is sent**, once a game cycle (250 ms), from the GameClock's broadcast:

- **Hydrate** (`0x50`) an object whole, when it comes into the player's view (and when the client asks about
  it): its number; its **uuid**; whether it's **Living** (u8, 1 yes, 0 no: the client makes a Living one an
  Actor, with its **short name** over its head, a string, empty for none); its motion's nine f32s; its
  **scale** (three f32s, 1, 1, 1 as the model was made); its **model**'s uuid (a string, empty for none:
  no object has one yet); the **shape** to draw without a model (u8: 0 cube, 1 sphere, 2 capsule, 3
  cylinder, 4 plane, 5 quad; a character is a capsule); the **collider**, the room it takes up in the
  world for bumping into things (u8: 0 none, 1 capsule, 2 cylinder, 3 box; version 15), and its size, three
  f32s: for a capsule or a cylinder its radius, its height and 0, for a box its size along x, y and z, for
  none 0s, standing with its bottom at the position like the shape (a character is a capsule 0.5 round and
  2 tall); and what it's **doing** (a string, the model's animation, "idle", empty for nothing).  One object
  to a packet.
- **ObjectsMoved** (`0x51`): objects the player knows whose motion changed since the cycle before, but
  their own character (version 15: they walked it there themselves).  A u8 count, then each one's motion;
  29 to a packet, more packets if there are more.
- **ObjectsGone** (`0x52`): objects gone out of the player's view, or out of the world.  A u16 count, then
  each one's number; 299 to a packet.  The client throws them away.

**The roll call.**  UDP loses a packet now and then, and a lost Hydrate or ObjectsGone would leave the client
wrong for good.  So every fourth cycle, once a second, each player in the world gets a **RollCall**
(`0x53`): everything the server believes their client knows, and each one's motion now.  The roll call's
number (a u32, one higher each time), which piece (u8, from 1) and how many pieces (u8), a u8 count, then
that many motions, 29 to a piece.  A player who knows nothing still gets one piece with a count of 0.  Once
every piece of one roll call is in (pieces of an older one still coming are given up), the client:

- **drops** every object it has that isn't on it;
- **asks about** every number on it it doesn't know, with an **ObjectAsk** (`0x54`): a u8 count (1 to 64),
  then each number.  No ask number, like a ChunkRequest: asking twice is harmless.  The next cycle answers it,
  a Hydrate for each one in the player's view and an ObjectsGone for any that isn't;
- moves everything else to where the roll call says, but its own character (version 15).

So a lost packet is mended within about a second.  A roll call can also, now and then, overtake a Hydrate
sent the cycle after it; the client drops the new object, and the next roll call has it asked about again.
Only a player whose character is in the world is sent any of this, or answered.

**Walked from the client's side.**  PlayerReady gets the CharacterEnteredWorld, which ends with the
character's own number: the client knows which object is its own before any Hydrate comes.  Within a cycle
the Hydrates come, its own character's among them, and everybody else's in view; the camera follows the one
with its own number.  Anybody coming into view, or into the world, comes as a Hydrate; anybody leaving it, as
an ObjectsGone.  A client that missed something finds out at the next roll call, and asks.

Jacob, object 7, standing at 0.5, 1, 0.5 facing 90 degrees round, a capsule with no model, comes into view:

```text
50                                            Hydrate
07 00 00 00                                   object 7
03 00 00 00  75 2D 31                         uuid "u-1" (a real one is 36 characters)
01                                            Living
05 00 00 00  4A 61 63 6F 62                   short name "Jacob"
00 00 00 3F  00 00 80 3F  00 00 00 3F         at 0.5, 1, 0.5
00 00 00 00  00 00 B4 42  00 00 00 00         facing 0, 90, 0
00 00 00 00  00 00 00 00  00 00 00 00         moving 0, 0, 0
00 00 80 3F  00 00 80 3F  00 00 80 3F         scale 1, 1, 1
00 00 00 00                                   no model
02                                            drawn as a capsule
01                                            takes up a capsule
00 00 00 3F  00 00 00 40  00 00 00 00         0.5 round, 2 tall
00 00 00 00                                   doing nothing
```

Then roll call 3, with Jacob the only one known, and Jacob gone:

```text
53                                            RollCall
03 00 00 00                                   roll call 3
01 01                                         piece 1 of 1
01                                            1 object
07 00 00 00  00 00 00 3F ...                  object 7, and its nine f32s as above

52                                            ObjectsGone
01 00                                         1 object
07 00 00 00                                   object 7
```

A client that got a roll call listing 9, which it never had, asks:

```text
54                                            ObjectAsk
01                                            1 object
09 00 00 00                                   object 9
```

## Movement

Version 15 (2026-10-03), EverQuest's way.  Jacob: "the client like has local authority and the server kinda
just periodically validates the client movement and pulls it backward if it doesn't match to the last known
good spots."  So the client walks the player's own character on its own screen the moment a key goes down,
and tells the server where it went; the server takes each move or pulls the character back.  Everybody else
sees it walk through ObjectsMoved, the way they see anything move.

**PlayerMoved** (`0x55`, client to server): the move's **number** (a u32, one higher every move, from 1), the
number of the **last MoveCorrection** the client had (a u32, 0 for none), then the character's position,
rotation and velocity, nine f32s as in a motion, without the object's number (it can only be the player's
own).  44 bytes after the type.  The client sends one **whenever which way the character walks or faces
changes** (setting off, stopping, turning), and **every half second** while it walks and nothing changes.
Standing still, it sends nothing.  No answer: a move that's taken is quiet.

The server takes the moves in the order they come, once a game cycle, and:

- **drops** a move whose number is no higher than the last one it took (UDP can hand them over out of
  order);
- **drops** every move that says a lower pull-back than the last one it sent, sending that MoveCorrection
  again: those were made before the client heard of it;
- **pulls the character back** if the move puts it further than walking could have taken it since its last
  good spot by more than `movement_tolerance_blocks` (the server's `game.cfg`, 16), turned more than 90
  degrees further than turning allows, inside a block, on ground the server hasn't loaded, into a character
  standing still (getting closer, more than a quarter block into its collider), or in the air for 2 seconds
  without falling a block (back to the last spot it stood on).  A step up of one block is free; falling is
  as fast as it is.  How far walking could have taken it counts the time since its last good spot, two
  seconds at most;
- **takes** anything else: it's the character's place now, and goes to everybody who can see it.  More than
  a block too far, but within the tolerance, it stands, and the server's admin hears of it.

The velocity a move carries is only what everybody else's screens carry the character along by between moves,
held to walking along the ground.  A character walking with nothing heard from its client for 2 seconds is
stopped where it was last good.

**MoveCorrection** (`0x56`, server to client): the pull-back's **number** (a u32, from 1, one higher each
time for this character), then where the character is put, standing still: its position and rotation, six
f32s.  The client puts it there, at once, standing still, and **every move after says this number**.  One
with a number it has already had is old, and changes nothing.  It goes out in the game cycle after the move
it answers, before anything else that cycle says to that player.

A player's own character isn't in the ObjectsMoved they're sent: they walked it there.  It's on the roll
call like everything they know, and the client doesn't take its place from there either.

The chunks a player may ask for follow where the server last took their character, and one chunk more each
way than the view, for a client a column ahead of the server while it walks.

Jacob, object 7, at 0.5, 1, 0.5 facing west (270), sets off west at 4 blocks a second, the 12th move of
the session, no pull-back had:

```text
55                                            PlayerMoved
0C 00 00 00                                   move 12
00 00 00 00                                   no pull-back had
00 00 00 3F  00 00 80 3F  00 00 00 3F         at 0.5, 1, 0.5
00 00 00 00  00 00 87 43  00 00 00 00         facing 0, 270, 0
00 00 80 C0  00 00 00 00  00 00 00 00         moving -4, 0, 0
```

His client then says he went 30 blocks west in no time, and he's put back, pull-back 1:

```text
56                                            MoveCorrection
01 00 00 00                                   pull-back 1
00 00 00 3F  00 00 80 3F  00 00 00 3F         at 0.5, 1, 0.5
00 00 00 00  00 00 87 43  00 00 00 00         facing 0, 270, 0
```

Every move after it says `01 00 00 00` where the first said none.

## One moment

**PleaseWait** (`0x3B`, version 10): the ask's number and the words to show while the server works on it.
It isn't the ask's answer, so it doesn't end the ask: the answer follows under the same number, and the
client goes on waiting (and resending, as for any ask; the server drops the resends while it works).  A
client that gives up on an ask after a fixed time starts that time again at a PleaseWait.  Today only a
UserPressPlay for a character locked for a moment gets one; any ask that will take a while may.

```text
3B                                            PleaseWait
03 00 00 00                                   ask 3
0A 00 00 00  4F 6E 65 20 6D 6F 6D 65 6E 74    "One moment"
```

## Answers in pieces

An answer bigger than 1200 bytes (a `/who` of more than about forty characters) goes out as
**Spans**: each one the ask number, which piece it is (from 1), how many pieces there are, and a piece of
the answer's bytes, 1193 at most.  The pieces' bytes put back together in order are the answer, type byte
and all, which the client then reads as if it had come whole.  An answer that fits goes as it is, never
as one Span.  At most 255 pieces.

Each piece says how many there are, rather than a packet in front announcing them, so any one that
arrives is enough to know what's coming.  A client missing pieces sends its ask again after half a second
as it always does, and the server sends every piece again from the answer it kept; the client keeps what
it had and fills the gaps.  **It gives up 2 seconds after the first piece** if it still hasn't got them
all (Jacob, 2026-10-02: "wait till all are received or a specified time elapses", "2s is fine").

## A worked example

A Login as `jacob_01` with the password `Correct horse 1!`, client version `0.0.1`, secret word `potato`,
as the bytes go over TLS.  What goes is the password's key, 64 characters.  The length is 1 + (4 + 5) +
(4 + 6) + (4 + 8) + (4 + 64) = 100, which is `0x64`:

```text
64 00 00 00                                   length 100
11                                            Login
05 00 00 00  30 2E 30 2E 31                   "0.0.1"
06 00 00 00  70 6F 74 61 74 6F                "potato"
08 00 00 00  6A 61 63 6F 62 5F 30 31          "jacob_01"
40 00 00 00  66 63 37 31 66 30 63 39 34 36    "fc71f0c94665dfd6ff4e217891cd7ff8
             36 35 64 66 64 36 66 66 34 65     1c8fd8ed7b20cf498c122c1bbd1f8855",
             32 31 37 38 39 31 63 64 37 66     the key for "Correct horse 1!"
             66 38 31 63 38 66 64 38 65 64
             37 62 32 30 63 66 34 39 38 63
             31 32 32 63 31 62 62 64 31 66
             38 38 35 35
```

The server's answer when it works, with a token shown short and UDP port 9998 (`0x270E`):

```text
47 00 00 00                                   length 71: 1 + (4 + 64) + 2
15                                            Ticket
40 00 00 00  <64 hex characters>              the token
0E 27                                         port 9998
```

And the answer when it doesn't.  "Invalid Credentials" is 19 bytes, so the length is 1 + 1 + 4 + 19 = 25:

```text
19 00 00 00                                   length 25
13                                            LoginResult
01                                            failed
13 00 00 00  49 6E 76 61 6C 69 64 20 43 72    "Invalid Credentials"
             65 64 65 6E 74 69 61 6C 73
```

Over UDP, a KeepAlive is the one byte `32`, and the answer is the same byte back.
