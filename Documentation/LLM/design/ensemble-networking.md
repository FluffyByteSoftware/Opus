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

## One moment: PleaseWait (2026-10-02, written, waiting on Unity)

Jacob found it testing the double login: the second client logs the first out, presses PLAY inside the
character's one-second lock, and gets Kicked, reason 6, back to the login.  His call: "the client is told to
wait and then pulled in".  So protocol version 10 adds PleaseWait (`0x3B`, the ask's number and words), and
the server sends one for a locked pick and waits the lock out itself.  In the client:
`GameConnection.Waiting()` takes it for the ask that's out (an older ask's is ignored), starts the ask's
10-second clock again, and posts `Session.AskWaiting()`, which puts the words on character select's status
line (not as trouble) and leaves the ask out, so the buttons stay grey.  CharacterEnteredWorld then comes as
usual and clears the line; a refusal shows as before.  For a typed line it goes in the chat box, in case a
command ever sends one.  `Protocol.Version` is 10.

## In the world: chat and /who (2026-10-02, written, waiting on Unity)

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
- **`Session.ReachedWorld`**: PLAY's answer, which ScreenRoot turns into the HUD.

## Later

- The key taking longer than the server's 10-second login deadline on a slow machine: the server hangs up, and
  the player sees "The server hung up."  2852 ms in the Unity editor, so not today's problem.
