<!--
File:       Opus/Documentation/LLM/PROTOCOL.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Protocol

What Conductor and its clients say to each other, down to the byte.  Written for somebody building a
client who has never seen Conductor's code.  Conductor's half is `Conductor/dev/networking/src/protocol.rs`.
The Python test client beside the crate (`networking/test_client.py`) speaks all of it, and Ensemble
(`Assets/Code/Net/`, 2026-10-02) the login and character select so far, not chat yet; when any of them
disagrees with this document, it is the code that gets fixed.

Protocol version **10**.  The number goes up when a packet changes, and the server says it in the first
thing it sends, so a client built against another version can stop right there.  Version 10 (2026-10-02)
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
   second, character select, the character picked coming into the world, and the game's packets as they
   come.

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

TLS 1.3 or 1.2.  It was 1.3 only until Ensemble (2026-10-02): Unity's .NET can't speak 1.3, so the server
takes 1.2 as well.  Nothing in the packets changed, so the protocol version didn't either.  The server has one
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
character select, between the login and the world, over UDP.  `0x3_` is the game, over UDP.

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
| `0x28` | CharacterEnteredWorld | server to client | u32 ask, string uuid, string name, f32 x, y, z  |
| `0x30` | Connect        | client to server | string token                                             |
| `0x31` | ConnectResult  | server to client | u8 answer, string message                                |
| `0x32` | KeepAlive      | both ways        | nothing                                                  |
| `0x33` | Goodbye        | client to server | nothing                                                  |
| `0x34` | Kicked         | server to client | u32 reason                                               |
| `0x35` | CommandAccepted | server to client | u32 ask                                                 |
| `0x36` | CommandRefused | server to client | u32 ask, string why                                      |
| `0x37` | PlayerCommand  | client to server | u32 ask, string the line as typed                        |
| `0x38` | ChatDelivery   | server to client | u8 count, then that many strings, each a finished line   |
| `0x39` | WhoDelivery    | server to client | u32 ask, u32 seconds since midnight UTC, u8 list, u16 count, then each: string name, and with a list i32 x, y, z |
| `0x3A` | Span           | server to client | u32 ask, u8 piece, u8 pieces, then the piece's bytes     |
| `0x3B` | PleaseWait     | server to client | u32 ask, string words                                    |

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
- **CharacterRequestResetHome** with a character's uuid puts it back at 0, 0, 0 and gets a
  **CommandAccepted**, or a **CommandRefused** saying why not (no such character on the account, the
  character is unplayable, its save won't load, or the server can't right now).

- **UserPressPlay** with a character's uuid brings it into the world, where its last save left it, and gets
  a **CharacterEnteredWorld**: its uuid and name, and where it stands, x, y and z (y up).  Or a
  **CommandRefused** saying why not: no such character on the account, the character is unplayable, its
  save won't load (it's marked unplayable then, and the admin told), or the server can't right now.
- **A character is locked for a moment** whenever it moves between the database and the world: for 1
  second from the moment the server starts loading it, and for 1 second after it leaves the world, longer
  if its save from leaving hasn't reached the database yet.  A UserPressPlay for a locked character isn't
  read until the lock clears: the client gets a **PleaseWait** (version 10) with the words to show, "Your
  character is still being saved from its last session. One moment.", the server waits the lock out, and
  the CharacterEnteredWorld (or a CommandRefused) follows under the same ask number.  The client keeps
  resending the ask meanwhile, as for any ask, and the server drops the resends.  If the lock is still
  held after 5 seconds the client gets a **Kicked** with reason `6` instead, and logs in again.  So one
  character is never brought in twice at once, or on the save before its last.
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

## In the world, over UDP

Once a CharacterEnteredWorld has come, the player is in the world.  For now what they can do there is chat.

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

**`/who`** asks who's in the world, and **`/who list`** where each of them stands.  Only characters in
the world count (players at character select don't), and only the one who asked gets the answer, a
**WhoDelivery** carrying the ask number, so it's sent again for a resend like any answer:

- the time it ran, in **seconds since midnight UTC** (0 to 86,399), a u32;
- whether it's a list: `0` for `/who`, `1` for `/who list`;
- a u16 count, then each character, A to Z whatever the capitals: its name, and in a list its x, y and z,
  each an i32 in **whole blocks**, rounded down (1.5 is block 1, -1.5 is block -2).

`/who list` waits for the next game cycle, so its answer comes up to 250 ms later.  Anything else after
`/who` gets a **CommandRefused**, "Try /who, or /who list.", and "Who Unavailable" means the server can't
right now.

The client draws the rest, in the player's own time zone (the date from its own clock, the time from the
packet) and to the width of its chat box, the count written out (Ensemble's `Translator.NumberToWords()`).
Jacob's old MUD's box, at 79 wide:

```text
-----------------------======] Forgotten Legends [======-----------------------
                          Fri Oct  2 03:53:24 2026
----------------------------------] Players [----------------------------------
Aldric   Bujin    Eetius   Guesty   Kriket   Malachy  Trzk     Zeleya
-----------------> There are eight legends currently online. <-----------------
```

The names in columns as wide as the longest name and two spaces, as many to a row as fit.  For one:
"There is one legend currently online."  A `/who list` has a line each in place of the columns:

```text
[Aldric] is currently at [0, 0, 0]
[Jacob] is currently at [1, 0, -2]
```

A `/who` as ask 5 at 03:53:24 UTC (14,004 seconds, `0x36B4`), with Aldric and Jacob in the world:

```text
37                                            PlayerCommand
05 00 00 00                                   ask 5
04 00 00 00  2F 77 68 6F                      "/who"

39                                            WhoDelivery
05 00 00 00                                   ask 5
B4 36 00 00                                   14,004 seconds after midnight UTC
00                                            names only
02 00                                         2 characters
06 00 00 00  41 6C 64 72 69 63                "Aldric"
05 00 00 00  4A 61 63 6F 62                   "Jacob"
```

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

An answer bigger than 1200 bytes (a `/who list` of more than about thirty characters) goes out as
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
