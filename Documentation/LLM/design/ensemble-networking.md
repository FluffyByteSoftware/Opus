<!--
File:       Opus/Documentation/LLM/design/ensemble-networking.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Ensemble -- the client's net code

Started 2026-10-02.  Jacob: "its time to build up the client to submit and move over to character selection!"
PROTOCOL.md is the contract; this file is how Ensemble speaks it.  `test_client.py` is still the working example
of every byte.

**Built and tested** (2026-10-02, in Unity against Conductor on Linux): all ten checks passed.  The first compile
failed on TLS 1.3 (below, "TLS"); after that it compiled clean.

## Settled (Jacob, 2026-10-02)

- **Get there this session**: SUBMIT logs in, and the player lands on character select, which lists the account's
  characters and has LOG OUT.  CREATE, DELETE and PLAY are a session of their own.
- **Already logged in elsewhere**: the client offers to kick the other session or log off, for 30 seconds (the
  server's own wait); "if no answer it disconnects this session not the existing".  The server already holds the
  Ticket until the other session's character has its save in (the "safety" lock), so the client only waits.
- **The certificate**: the client carries a copy of `conductor.crt` and refuses any server that shows it another.
- **TLS**: Conductor spoke 1.3 only.  If Unity's TLS can't, Conductor takes 1.2 as well (rustls's `tls12`
  feature, no new crate), "but I'm pretty sure it will do 1.3".  **The first compile said so**: Unity's .NET
  has no `SslProtocols.Tls13` ("'SslProtocols' does not contain a definition for 'Tls13'"), so the client
  asks for 1.2 and Conductor takes it.
- **The client version**: "we're not ready for 0.0.1 yet".  The Login carries Player Settings' Version as it is
  (`Application.version`, `0.0.1` since 2026-10-02), and `networking.cfg`'s `client_versions` is `0.0.0.1, 0.0.1` (the
  second for `test_client.py`, whose default is still `0.0.1`).

## The login moved out (2026-10-02, built and tested)

Soundcheck does the login now (`design/soundcheck.md`: "remove login from the game like monsters and memories
did").  So everything below about TLS, the certificate, `LoginConnection.cs`, `ServerCertificate.cs`, the
`Security/` folder, `LoginForm.cs`, `login_status` and the login screen is history: those files left Ensemble
(Soundcheck has its own copies, ported the same day), and `Session.cs` lost `LogIn()`, `Choose()`, the two
login stages and the login's events.  What's there instead:

- **`Net/Ticket.cs`**: the ticket from the launcher, read out of the game's environment at start
  (`OPUS_SERVER`, `OPUS_UDP_PORT`, `OPUS_TOKEN`, and `OPUS_SOUNDCHECK` for the way back), and in the editor
  only, out of Soundcheck's `debug_ticket.json` (dev mode).  A token is good once, for 30 seconds, and is
  never logged.
- **`Session.Enter(ticket)`** opens UDP at once; `Welcomed` is character select as before.  `Notice` and
  `NoticeChanged` are the start screen's line ("Joining the world...", or why the session ended);
  `SessionOver` replaces `BackAtLogin` and rings only when the game stays open.  `Finish()` starts Soundcheck
  again (`BackToTheLauncher()`, `System.Diagnostics.Process`, the four variables taken out of what it
  inherits) and closes the game; with no launcher path, or in the editor, it's the start screen with why.
  `CloseTheGame()` (QUIT, `/camp desktop`) closes without the launcher.
- **The start screen** replaces the login on ScreenRoot: `start_default.json`, `start.uss`, three widgets
  (`start_background`, `start_logo`, `start_card`).  The card says the line, has QUIT, and in the editor
  carries dev mode: `box.schedule.Execute().Every(1000)` looks for a fresh ticket while the card is on
  screen, and `Session.Enter()`s it.  ScreenRoot's Login Text Color and Font became Screen Text Color and
  Font (`FormerlySerializedAs` keeps the values); the Server Certificate slot is gone.
- **`Protocol.cs`** keeps the whole packet table (it mirrors PROTOCOL.md's) and lost LoginResult's answers,
  SessionChoice's values, the secret word and the TCP frame cap.  Player Settings' Version is no longer sent
  anywhere: the version the server checks is Soundcheck's.
- **Conductor's `tls12` feature** can go now; nothing speaks 1.2 (TODO.md).

## As written (2026-10-02, before the login moved out)

In `Assets/Code/Net/`, namespace `Opus.Net`, plain C#, nothing added to the project:

- **`Protocol.cs`**: the version (7), the secret word, the packet types, the answers, and the words the player sees
  for each Kicked reason (the server only sends the number).
- **`Packets.cs`**: `PacketWriter` and `PacketReader`, PROTOCOL.md's bytes.  A string that isn't UTF-8, a packet
  too short or one with bytes left over is a `ProtocolException`.
- **`ServerCertificate.cs`**: the certificate's bytes out of its PEM text, and the byte-for-byte match.  Self-signed
  means the usual checks always complain, so they're ignored; the match is the check, the same as `test_client.py
  --cert`.  The copy is `Assets/Data/Certs/conductor_crt.txt` (Unity won't take `.crt` as a TextAsset), on
  ScreenRoot's Server Certificate slot.  A new certificate on the server means copying it over again.
- **`LoginConnection.cs`**: the TCP half, a thread of its own.  It connects and brings TLS up while the key is
  still being made (Jacob's pick, 2026-10-01), reads the Hello, sends the Login once the key's ready, then reads
  InLine, LoginResult or the Ticket.  The SessionChoice waits on the player for 30 seconds, then hangs up.  TLS
  1.2, the newest Unity's .NET has; it logs which came up.  The handshake took 3 ms on Conductor's side (Jacob's log,
  2026-10-02, the game and the server on one machine at 10.0.0.84).
- **`GameConnection.cs`**: the UDP half, two threads.  One listens; the other sends on time and otherwise waits on
  the next thing due: the Connect every half second for up to 10 seconds, a KeepAlive once a second, an ask every
  half second until it's answered (10 seconds, then given up).  Nothing from the server in **15 seconds** and it's
  gone (the server's own wait for us is 40).  A Kicked ends it.  Close() sends a Goodbye.  The OS's "nobody on
  that port" after a send is ignored; the resends and the quiet timer have the last word.  It sends on its own
  threads, not Unity's Update, so the keep-alives go on with the window in the background.
- **`MainThread.cs`**: the threads post what they hear here, and ScreenRoot's `Update()` runs it once a frame.
- **`Session.cs`**: the flow, main thread only.  `LogIn()`, `Choose()`, `LogOut()`, `Quit()`; the stage the
  player is at; the characters; events for the screens (`StatusChanged`, `OtherSessionAsked`, `LoggedIn`,
  `WrongPassword`, `ReachedCharacterSelect`, `CharactersChanged`, `BackAtLogin`).  A message from a connection
  that's already been dropped is ignored.  Every way out ends at the login.

On the screens (`design/ensemble-hud.md` has the layouts):

- **`login_status`**, a new widget under SUBMIT: what's happening, why it didn't work, why the player is back.
  Trouble is a dark red band behind the words, since ScreenRoot's Login Text Color is set on every word and would
  beat a colour from the style sheet.  KICK OTHER SESSION and LOG OFF show under it when asked, with the seconds
  left.
- **`LoginForm.cs`**: SUBMIT checks the boxes, forgets the remembered login at once if Remember Me is off, starts
  the key and the login together, and greys the boxes while it runs.  **Remember Me is written only once the
  Ticket comes**, so a wrong password is never kept.  Invalid Credentials drops the key, so the password has to
  be typed.  A key made for a login that couldn't start is kept, so trying again doesn't mean typing again.
- **Character select** is a third screen on ScreenRoot: `character_select_background`, `character_select_list`
  and `character_select_log_out`, from `character_select_default.json`, styled by `character_select.uss`, with
  the login's text colour and font.  The list asks once on arriving and says so until the answer comes.
- **The login is only built again if it isn't showing**, so a failed login keeps what was typed.  Back from
  character select, it's built fresh: the remembered key, or the password typed again.

## Character select, the rest of it (2026-10-02)

Jacob: "we're ready for the next segment which is character select/delete/play/create!"  Conductor's half is
all there (PROTOCOL.md, "Character select"), so this is Ensemble's alone.

### Settled (Jacob, 2026-10-02)

- **The screen, as proposed**: a click on a row selects it; PLAY, DELETE and CREATE are widgets of their own
  under the list.  PLAY is greyed until a playable character is picked; DELETE takes any character, an
  unplayable one too (the server lets it); CREATE shows only while a slot is empty.  CREATE and DELETE each
  open a card: a name to type, or DELETE to type.  A status line under the list, like the login's.  The
  buttons grey while an ask waits on its answer, since the server takes one at a time.
- **On CharacterEnteredWorld**: "Just display a message "in the world as <Shortname>" and then a log out
  button."  So the list gives way to that line, and LOG OUT stays.  No HUD yet.
- **The client checks the name rule before it sends**: "we may as well prevent spamming the server if
  possible.  The server will hard check too".  Same words as the server's answer 1.
- **RESET HOME is a button too** ("oh yes yes yes yes").
- **No double-click to play**: "you need to explicitly hit play".
- **The server's TLS key had gone into git** with the `.meta` commit (`Assets/Data/Certs/conductor.key`,
  Jacob's yes that it's the real one).  Taken out with the two copies of the certificate beside it, and the
  `.gitignore` keeps out every `*.key`.  A new key pair is made with README's openssl command, and its
  certificate copied in as `conductor_crt.txt`.

### As written (2026-10-02, built and tested)

In Unity against Conductor on Linux, all eleven checks passed, the new key pair included (Jacob: "the fucking
loop worked!"): log in, pick, make, delete, reset home, play, log out, and in again.

- **`GameConnection.Ask(kind, params string[] fields)`**: every ask with fields carries strings only.  It
  reads CharacterCreateResult, CharacterDeleteResult, CommandAccepted and CharacterEnteredWorld now
  (`PacketReader.F32()` for its x, y, z).
- **`Session.cs`**: `CreateCharacter(name)`, `DeleteCharacter(uuid, typed)`, `ResetHome(uuid)` and
  `Play(uuid)`, and a stage past character select, `InWorld`.  `Asking` is the ask out (its packet type), so a
  CommandAccepted or CommandRefused goes to the ask it answers: the list's trouble in the list, the rest on the
  status line (`Said`).  A made or deleted character asks for the list again, since the answers don't say
  which slot.  The name rule (`NameRule`, `NameFollowsRule()`) and DELETE are checked before anything is
  sent, in the server's own words.  `CharactersChanged` became `CharacterSelectChanged` (anything on the
  screen), with `AskAnswered` before it so a card closes on a yes.
- **Seven new widgets**: `character_select_status`, `character_select_play`, `_create`, `_delete`,
  `_reset_home`, and `character_select_create_card` and `_delete_card` on layer 2 over the list, hidden
  until opened.  Enter is a card's button, Escape closes it.  `CharacterSelectForm.cs` holds the pick and
  which card is open, and fills it all in.  The layout puts the list, the line, the four buttons in a row
  and LOG OUT down the middle.

## One moment: PleaseWait (2026-10-02, built and tested)

Jacob found it testing the double login: the second client logs the first out, presses PLAY inside the
character's one-second lock, and gets Kicked, reason 6, back to the login.  His call: "the client is told to
wait and then pulled in".  So protocol version 10 adds PleaseWait (`0x3B`, the ask's number and words), and
the server sends one for a locked pick and waits the lock out itself.  In the client:
`GameConnection.Waiting()` takes it for the ask that's out (an older ask's is ignored), starts the ask's
10-second clock again, and posts `Session.AskWaiting()`, which puts the words on character select's status
line (not as trouble) and leaves the ask out, so the buttons stay grey.  CharacterEnteredWorld then comes as
usual and clears the line; a refusal shows as before.  For a typed line it goes in the chat box, in case a
command ever sends one.  `Protocol.Version` is 10.

## In the world: chat and /who (2026-10-02, built and tested)

The server's side is PROTOCOL.md's "In the world"; the chat window is `design/ensemble-hud.md`.

- **`Session.SendLine(line)`**: a line typed in the chat box, as typed, goes as a PlayerCommand through the
  same `Ask()` as character select's, one at a time; a line typed while the last waits replaces it.  Its
  CommandRefused, or no answer in 10 seconds, goes to the chat box (`ChatLine`), not the status line.
  `/camp` and `/camp desktop` are caught here and never sent.
- **`GameConnection`** hears ChatDelivery (no ask number) and hands its lines to `ChatLine`, reads
  WhoDelivery as an answer (`WhoAnswer`, `WhoEntry`; `PacketReader.I32()` for the blocks) for
  `WhoAnswered`, and puts Spans back together: the pieces kept by number for the ask waiting, the ask's
  resend bringing the missing ones, the whole read as if it had come in one packet, and given up 2 seconds
  after the first piece ("The server didn't answer.").
- **`Session.ReachedWorld`**: PLAY's answer, which ScreenRoot turns into the HUD (since version 11,
  PlayerReady's).

## The map at PLAY (2026-10-03, session 2, built and tested)

Protocol version 11, PROTOCOL.md's "The map at PLAY".  Jacob: "A packets first in the server and then we'll in
this conversation also integrate Ensemble with receipt of those packets"; a fresh download every PLAY ("we're
just gonna write over whatever the client already has every time"); a client that can't get it goes back to
the launcher with the message.

- **`GameConnection`** reads PLAY's answer, the OverworldMapOffer, and starts a **`MapDownload`**
  (`Code/Net/MapDownload.cs`): room for the whole map, which pieces are in, and the next request, the lowest
  64 missing.  The sender thread asks for them at once, again as soon as the last 64 are all in (the
  listener wakes it), and every quarter second for whatever didn't come; with no new piece in 10 seconds
  it gives up (`Session.MapFailed`).  The listener tells Session how far along it is once a whole percent,
  and hands it the whole map when the last piece lands.
- **`Session`** has a new stage, **LoadingWorld**, between AtCharacterSelect and InWorld.  With the whole map
  (`MapArrived`) it checks its SHA-256 against the offer's, reads it once (`SimpleOverworldMap.FromBytes()`,
  `Code/World/SimpleOverworldMap.cs`, namespace `Opus.World`, the contract's reader) and keeps it in
  `SimpleOverworldMap.Current`, writes it over the player's copy (`simple_overworld.map` through
  `PlayerFiles.PathOf()`, a `.part` file moved into place), then sends **PlayerReady** with the hash.  On
  the main thread: a few MB hashed, written and read once a PLAY, behind a full bar.
- **Anything wrong** (no new piece in the wait, an offer that doesn't add up, a hash that isn't the offer's,
  a file that can't be written, PlayerReady refused or unanswered) ends the session with a Goodbye and goes
  back to the launcher with "Couldn't get the world's map. Delete <the file> (or reinstall the game) and try
  again."  The why is in the log.
- **The loading bar** is `design/ensemble-hud.md`'s.

## The chunks around the player (2026-10-03, session 3: only the version)

Protocol version 12, PROTOCOL.md's "The chunks around the player"; Conductor's half is built and tested
(`design/world.md`, "The chunks streamed").  Ensemble's only change so far: `Protocol.Version` is 12, the
three chunk packets have names in `Protocol.cs`, and `GameConnection` reads the OverworldMapOffer's new end
(`f32 x, y, z`, where the character will stand, and `u8` view, how many chunks each way it sees) and skips
it, just after the hash.  Asking for the chunks, holding them and drawing them are Ensemble's own sessions
(Jacob: "to bring it in line with these server changes"); STATUS.md lists what's to settle first.  The test
client's `fetch_chunks()` is a working pull to copy.

## The chunks, Ensemble's half (2026-10-03, session 4)

Jacob: "the Ensemble half first ... its ability to stream in the terrain data... then next session we're
gonna put it all together and try to render the world around our player".  The map's half was already
there (session 2); this is the chunks, received and held, nothing drawn.

### Settled (Jacob, 2026-10-03)

- **Map, then chunks**: the map comes in whole first, as now, then the chunks are asked for.
- **The red bar**: the map as now, then "Loading the ground", counted in chunks (by bytes the chunks
  would hardly move it: 88 KB against the map's 16 MB).
- **Held**: a chunk all of one kind (most of a view is air) is kept as just that kind; any other is
  unsqueezed to its 32,768 blocks.  About 37 MB for a view at 8 on Omega's hills, where every chunk as
  blocks would be 199 MB.  Drawing (next session) reads the blocks straight out.
- **Trouble**: no new chunk in 10 seconds gives up and goes back to the launcher ("Couldn't get the
  ground around you"), the same as the map.  A chunk the server calls unavailable stays empty, a warning
  in the log, and the game goes on.
- **PlayerReady goes once the nearest chunks are in**, Minecraft's and 7 Days to Die's way (Jacob: "Their
  way"): the 3 by 3 columns around the character, every row, 99 chunks, each in or refused for good.  The
  rest of the view keeps coming after, with the character in the world, so the download already runs in
  the world when movement (0.0.2) needs it to.  The 10 seconds' give-up holds in the world too.  The other
  shape was all of the view before PlayerReady (0.04 s on the LAN today), simpler, with streaming in the
  world left to 0.0.2.
- **Nothing drawn, so nothing to look at**: one line in the Console when the whole view is in, the test
  client's summary (how many, how many bytes, how long, the blocks by kind, the block at 0,0,0).  Drawing,
  plain colours or the purchased art (an atlas), is for when the chunks are drawn.

### As written (2026-10-03, session 4, built and run)

**Measured** (Jacob, in the editor against Conductor on Linux, `view_chunks` 8): 3,179 chunks, all in, 0
refused, 88,746 bytes squeezed, 3,179 packets, 50 requests, 0 "not yet"s, in 0.12 s; **the nearest 99 in
0.02 s**, then PlayerReady.  The blocks the same as the test client's to the block, 0 that didn't unsqueeze,
GOLD at 0,0,0.  **Held: 561 chunks as blocks, 35.1 MB, and 2,618 all one kind** (the guess was about 37).
The map before it: 16.8 MB in 0.17 s.


- **`Code/Net/ChunkDownload.cs`**, the chunks' `MapDownload`: every chunk within the offer's view of the
  character's column, every row, sorted nearest first the test client's way; which are done (in, refused
  for good, or unreadable); the pieces of a chunk in more than one; `NextRequest()`, the first 64 not done;
  the nearest 99 counted on their own (`NearCount`, `NearDone`); and the Console's summary.
- **`Code/World/Chunk.cs`**: `ChunkPlace` (x, z, row), the block numbers' names (`Blocks`), and `Chunk`
  with PROTOCOL.md's `Unsqueeze()`: one kind kept as `OnlyKind`, any other as 32,768 blocks, and how many
  of each kind counted from the runs.  `BlockAt(x, y, z)` inside the chunk.
- **`Code/World/Ground.cs`**: the chunks the game has, by place, main thread only, emptied with the
  session; `BlockAt()` in world blocks.  **Not `Terrain`**, Conductor's word, because `UnityEngine.Terrain`
  is Unity's own and a file with `using UnityEngine;` couldn't tell the two apart.
- **`GameConnection`** keeps the offer's x, y, z and view; `FetchChunks()` starts the download (from
  Session, once the map is kept).  The sender asks for the next 64 at once when the last 64 are done, and
  every quarter second otherwise; no new chunk in 10 seconds is `Session.GroundFailed`.  The listener
  unsqueezes each whole chunk itself and posts it; a chunk that won't unsqueeze, and one the server calls
  unavailable, is a warning and left empty.
- **`Session`**: `MapArrived` asks for the chunks instead of sending PlayerReady; `ChunkArrived` puts each
  in the Ground; `NearGroundIn` sends PlayerReady; `GroundAllIn` writes the summary (with how many are held
  as blocks, the MB, and the block at 0,0,0); `GroundFailed` goes back to the launcher with "Couldn't get
  the ground around you."  `MapProgressed` became **`LoadingProgressed`**, with `GroundHave` and
  `GroundNeed` beside `MapReceived` and `MapSize`.
- **The bar** (`CharacterSelectForm.FillLoading()`): the map in MB, then from empty again, "Loading the
  ground...  N of 99 chunks", then "The ground is in.  Entering the world...".
- `PacketWriter.I16()` and `PacketReader.I16()`, and `Protocol.ChunksAtOnce` and the three refusals.
