<!--
File:       Opus/Documentation/LLM/design/ensemble-networking.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Ensemble -- the client's net code

Started 2026-10-02.  Jacob: "its time to build up the client to submit and move over to character selection!"
PROTOCOL.md is the contract; this file is how Ensemble speaks it.  `test_client.py` is still the working example
of every byte.

**Written, not built yet** (2026-10-02): nobody has compiled it in Unity.  The checks are in TEST_CHECKLIST.html.

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
  (`Application.version`, `0.0.0.1` today), and `networking.cfg`'s `client_versions` is `0.0.0.1, 0.0.1` (the
  second for `test_client.py`, whose default is still `0.0.1`).

## As written

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
  1.2, the newest Unity's .NET has; it logs which came up.
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

## Later

- CREATE, DELETE and PLAY at character select, and what the client does with CharacterEnteredWorld (the HUD).
- An ask with fields after its number (`GameConnection.Ask()` takes only the type today).
- The key taking longer than the server's 10-second login deadline on a slow machine: the server hangs up, and
  the player sees "The server hung up."  2852 ms in the Unity editor, so not today's problem.
