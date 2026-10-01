<!--
File:       Opus/Documentation/LLM/PROTOCOL.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Protocol

What Conductor and its clients say to each other, down to the byte.  Written for somebody building a
client who has never seen Conductor's code.  Conductor's half is `Conductor/dev/networking/src/protocol.rs`,
and the Python test client beside the crate (`networking/test_client.py`) is the other half until Ensemble
speaks it; when either disagrees with this document, it is the code that gets fixed.

Protocol version **6**.  The number goes up when a packet changes, and the server says it in the first
thing it sends, so a client built against another version can stop right there.  Version 6 (2026-10-01)
added the spawn: UserPressPlay and CharacterEnteredWorld (`0x27`, `0x28`), and reason `6`, character still
leaving the world, to Kicked.  Version 5 (2026-09-30)
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
   second, character select, the character picked coming into the world, and the game's packets as they
   come.

When the UDP session ends, for any reason, the player is gone.  There is no reconnect: the client goes back
to the login screen and starts over from TCP.  The reasons it ends: the client sent a Goodbye, the client
went quiet past the server's UDP timeout (40 seconds by default), the account logged in from somewhere
else and chose to log this session out, the server stopped, the admin banned the address (put it on
the blacklist, or took it off the whitelist), the admin kicked the player, the admin deleted the
account, or the player picked a character that left the world less than a second ago.  For the last six
the client hears a Kicked first; for the timeout it hears nothing, and knows from its own silence.  A
character in the world leaves it with its player's session, saved on the way out; there's no going back
to character select from the world but logging out and in again.

### TLS

TLS 1.3 only.  The server has one self-signed certificate, and a client trusts that certificate and no
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
character select, between the login and the world, over UDP.  `0x3_` is the game, over UDP.

| Type   | Name           | Way              | Payload                                                  |
|--------|----------------|------------------|----------------------------------------------------------|
| `0x10` | Hello          | server to client | u8: protocol version                                     |
| `0x11` | Login          | client to server | string version, string secret word, string username, string password |
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
| `0x28` | CharacterEnteredWorld | server to client | u32 ask, string uuid, string name, f32 x, y, z  |
| `0x30` | Connect        | client to server | string token                                             |
| `0x31` | ConnectResult  | server to client | u8 answer, string message                                |
| `0x32` | KeepAlive      | both ways        | nothing                                                  |
| `0x33` | Goodbye        | client to server | nothing                                                  |
| `0x34` | Kicked         | server to client | u32 reason                                               |
| `0x35` | CommandAccepted | server to client | u32 ask                                                 |
| `0x36` | CommandRefused | server to client | u32 ask, string why                                      |

## The login, over TCP

1. The client connects and TLS comes up.  The client has until the server's login deadline (10 seconds by
   default) from the moment it connected to get through step 3.
2. The server sends **Hello** with the protocol version.
3. The client sends one **Login**: its version, the secret word, the username and the password.  The
   username is folded to lowercase on the server.  The secret word is not a secret from anybody with a copy
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
     `a-z`, `0-9` and `_`; one that isn't gets this answer without the server looking it up.
   - **LoginResult** with answer `2`, "This account is already logged in.": the password was right, and the
     account is in the world from somewhere else.  Only ever sent after the right password.  The
     connection stays open, and the client has 30 seconds to answer with a **SessionChoice**: `0` logs the
     other session out (it hears a Kicked) and this login carries on to a Ticket; `1` hangs this one up and
     leaves the other alone.  (So a shared account doesn't kick your brother off because you wanted to
     play.)
   - **LoginResult** with answer `3`, "Outdated Client Failure": the client's version isn't on the server's
     list.  Checked before the password.
   - **LoginResult** with answer `4`, "Login Unavailable": the server can't check logins right now, or
     couldn't make a token after the right password.  Nothing the player did.

   Every answer takes at least 150 ms from the moment the Login arrived, whichever it is, so the time it
   takes can't tell a real username from a made-up one.

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
   UserPressPlay left the world less than a second ago (nothing is wrong; logging in again gets it).  The
   client goes back to the login screen.

Anything else from an address the server knows counts as hearing from that player (the game's packets go
here later).  Anything at all from an address it doesn't know, other than a Connect, gets no answer.

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
- **CharacterRequestResetHome** with a character's uuid puts it back at 0, 0, 0 and gets a
  **CommandAccepted**, or a **CommandRefused** saying why not (no such character on the account, the
  character is unplayable, its save won't load, or the server can't right now).

- **UserPressPlay** with a character's uuid brings it into the world, where its last save left it, and gets
  a **CharacterEnteredWorld**: its uuid and name, and where it stands, x, y and z (y up).  Or a
  **CommandRefused** saying why not: no such character on the account, the character is unplayable, its
  save won't load (it's marked unplayable then, and the admin told), or the server can't right now.  A
  character that left the world less than a second ago (its last save may still be on its way to the
  database) gets a **Kicked** with reason `6` instead, and the client logs in again.
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

A UserPressPlay as ask 3, and the answer for Jacob standing at 1.5, 0, -2 (the uuid shown short; it's 36
characters):

```text
27                                            UserPressPlay
03 00 00 00                                   ask 3
24 00 00 00  30 31 39 39 ...                  the uuid, 36 bytes

28                                            CharacterEnteredWorld
03 00 00 00                                   ask 3
24 00 00 00  30 31 39 39 ...                  the uuid
05 00 00 00  4A 61 63 6F 62                   "Jacob"
00 00 C0 3F                                   x 1.5
00 00 00 00                                   y 0
00 00 00 C0                                   z -2
```

## A worked example

A Login as `jacob_01` with the password `Correct horse 1!`, client version `0.0.1`, secret word `potato`,
as the bytes go over TLS.  The length is 1 + (4 + 5) + (4 + 6) + (4 + 8) + (4 + 16) = 52, which is `0x34`:

```text
34 00 00 00                                   length 52
11                                            Login
05 00 00 00  30 2E 30 2E 31                   "0.0.1"
06 00 00 00  70 6F 74 61 74 6F                "potato"
08 00 00 00  6A 61 63 6F 62 5F 30 31          "jacob_01"
10 00 00 00  43 6F 72 72 65 63 74 20 68 6F    "Correct horse 1!"
             72 73 65 20 31 21
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
