<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is ten crates, each in a folder without the `conductor-` in front (`Conductor/dev/tools/`) while
the crate keeps it (`conductor-tools`, `conductor_tools::` in code).  `conductor-tools` (lib) holds DiskMan,
Scribe, Constellations, Fingerprinter, Security, Archivist, the notices, the clock, the thread list, the
services list and the server's switch (`server.rs`).  `conductor-accounts` (lib) is the one way in to the
accounts table and `player_characters`, and the account desk.  `conductor-monitor` (lib) looks at the
process and the machine once a second.  `conductor-networking` (lib) is the front door: a login over TLS on
TCP that hands a player a ticket for UDP, the UDP side, character select and the spawn (Protogame), a
ledger of every connection, and the access lists.
`conductor-lua-parser` (lib) runs the Lua scripts, locked down, and reads saved GameObjects back.
`conductor-primlib` (lib) is the game library, an ECS in memory, with the Living and Character templates.
`conductor-gameworld` (lib) is GameWorld, the ground.  `conductor-gameclock` (lib) is the GameClock, the game
loop: five checks of 50 ms to a 250 ms cycle; it owns primlib's `World` and GameWorld's `Terrain`, takes
players' characters in and out through a mailbox, and saves the world.  `conductor-wgui` (lib) is the web
admin at `http://127.0.0.1:9996/Opus`.  `conductor-launcher` (bin) boots the program and waits on the web
admin's Server tab.

Ensemble is Unity 6000.6: its project settings and our four folders under `Assets/` (`Editor/`, `Code/`,
`Scripts/`, `Data/`) are committed, the rest is on Jacob's machine, the purchased art in
`Assets/Purchased/`.  It has one editor tool, Tools > Opus > Copy Anims From FBX Pack, and three screens built
from layout files by our own builder: **the login**, character select and the HUD
(`design/ensemble-hud.md`).  ScreenRoot, beside the UI Document component, owns them and starts on the
login.  **The login turns the password into a key** on SUBMIT, and Remember Me keeps the key, never the
password (`design/client-security.md`).  **Ensemble logs in** (2026-10-02, `design/ensemble-networking.md`):
over TLS 1.2 to the Ticket, then UDP, and on to **character select**, a third screen that lists the account's
characters, look only, with LOG OUT.  **The game's name is Forgotten Legends**; the project, its folders and
code stay Opus.  Unity's Company Name is FluffyByte and its Product Name Opus.Ensemble.  Every file the game
keeps for a player goes in `~/.config/unity3d/FluffyByte/Opus.Ensemble/` (`PlayerFiles.cs`).

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` is at `33c418e`, released 2026-10-02 (Jacob: "merge everything to main please").
`unstable` and `testing` are level with each other, ahead of `main` by this session (the client's net code
and TLS 1.2).  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor, `world_size`, the password's key (protocol version 7) and TLS
1.2 included.  **In Unity**: Copy Anims From FBX Pack, its CLASHES view included, the HUD's Phase 1, the login,
the password's key with Remember Me, and logging in to character select, every check passed.
TEST_CHECKLIST.html is empty: the Windows check that sat Parked there is GitHub issue #10 now (Jacob opened
it, "Windows x86/x64 Untested").  **On Windows**: Conductor builds and runs, START SERVER included, without a
database; the world, the characters, character select, the world save, the spawn and the password's key
haven't been tried there.

## Jacob's map (2026-09-30, and on)

**The 0.0.1 goal**: "get a player spawned in the world and able to chat."

His words: "We are going to work on marrying the network code to the game by finishing out character as a
template for hydrating from an account.  Then we will build the character selection (start of UDP
connection), then the log in to the world, and spawn character in world."  And the flow: "account logs in
(done) -> character selection -> selected character spawns in world at its last save loc (0,0,0 for
now)".  His to change.

1. **The character as a template**, hydrated from an account.  **Done.**
2. **Character selection**, at the start of the UDP connection.  **Done**: list, make, delete, reset home.
3. **Logging in to the world**, and the character spawned there, at its last saved spot.  **Done**
   (2026-10-01): the game library's half, then networking's.

His order after that (2026-10-01): "building the network infrastructure up then the chapter after that will
be testing wtih a py script then we're on to building the client", and "then we're ready to start testing
it with a real client".  Chat, the other half of 0.0.1, isn't designed yet.

On the world (2026-10-01): "we will test a mountain out after we get the client up".

At the `world_size` hand-off (2026-10-01): **"wrap up and prepare to go back to the client"**.  Then, on
the client: **"we need to build our HUD up for login and char select"**, "just the login submit screen
first".

At the login's hand-off (2026-10-01): **"next conversation we start building security into the
client... then the conversation after that is the net code... then the next UI element, then stitching
the whole thing together"**.  His to change.

At this hand-off (2026-10-01): **"next one is going to be brutal we'll start the net code for the
client"**, then, told Conductor doesn't take the key yet: **"we'll do netcode update on server next
then"**.  So Conductor's half of the password's key came first.  His to change.

At the hand-off of Conductor's half of the key (2026-10-02): **"wrap up here back to Ensemble"**.  So the
client's net code came next.

At this hand-off (2026-10-02), the client logging in to character select: **"Hand off to
create/delete/select/play character next round"**.  His to change.

## Last session -- 2026-10-02, the client's net code (Ensemble)

**Built and tested** in Unity against Conductor on Linux, all ten checks passed; nothing waits on a build.
`design/ensemble-networking.md` has all of it.

- **Jacob's ask**: "its time to build up the client to submit and move over to character selection!"  His
  answers: get there this session (the list and LOG OUT; making, deleting and playing are their own
  session); an account already playing gets KICK OTHER SESSION or LOG OFF for 30 seconds, and no answer
  logs this one off ("if no answer it disconnects this session not the existing"); the client carries a
  copy of the certificate; TLS 1.2 if Unity can't do 1.3; the client version is Player Settings' `0.0.0.1`
  ("we're not ready for 0.0.1 yet"), and `networking.cfg`'s `client_versions` is `0.0.0.1, 0.0.1`.
- **`Assets/Code/Net/`**, namespace `Opus.Net`: `Protocol.cs`, `Packets.cs`, `ServerCertificate.cs`,
  `LoginConnection.cs` (TLS on its own thread, connecting while the key is made), `GameConnection.cs` (UDP,
  a listening thread and a sending one: Connect, KeepAlive once a second, asks resent every half second, 15
  seconds of quiet and it's gone), `MainThread.cs` (run from ScreenRoot's `Update()`), `Session.cs` (the
  flow and the screens' events).
- **The screens**: `login_status`, a line under SUBMIT (a dark red band for trouble, the two buttons when
  asked); character select, `character_select_default.json` and `.uss` with three widgets and
  `CharacterSelectForm.cs`.  ScreenRoot got Server Certificate, Character Select Layout and Character Select
  Style slots.  **Remember Me is written only once the login works.**
- **TLS 1.2**: the first compile failed on `SslProtocols.Tls13`, which Unity's .NET doesn't have.  The client
  asks for 1.2, and Conductor's `rustls` got its `tls12` feature (Jacob had OKed it for this case).  The
  handshake took 3 ms on Conductor's side.  `test_client.py` still insists on 1.3.
- **Two scares that weren't the code**: a first SUBMIT went to the web admin's port (Conductor's log had no
  "Connection from" line, and the web admin timed out reading a request; Jacob: "a local network issue"),
  and after the certificate check ScreenRoot's Server Certificate slot was empty and a stray
  `conductor_crt.b4.meta` was left in `Data/Certs/`.  Refilling the slot and restarting the client fixed it.
- **The new `.meta` files weren't pushed** at the hand-off: Jacob's commit of them (`git add -A` of
  `Assets` and `ProjectSettings`) is still to come.  If it hasn't landed, the next session asks for it
  before touching Ensemble.

## Where the next session starts

**Character select, the rest of it** (Jacob: "create/delete/select/play character next round").  The
packets are all there and tested with `test_client.py` (PROTOCOL.md, "Character select"): CreateCharacter,
DeleteCharacter (the player types DELETE), UserPressPlay and CharacterEnteredWorld, and Kicked reason 6 for
a locked character.  On the client: `GameConnection.Ask()` takes only a type today, so the asks with fields
come in; CREATE isn't offered with three slots full; an unplayable character is greyed and can't be picked;
and what the screen does on CharacterEnteredWorld (the HUD, most likely) is Jacob's to say.

Accounts to log in with: `testuser123` / `Testpass1!`.

## What's waiting

- **The Remember Me file is readable by other users on the same Linux machine.**  TODO.md.
- **Ensemble's client code**: CREATE, DELETE and PLAY at character select, then the world on screen, and the
  HUD's Phases 2 and 3 (the catalog's export, the web layout editor).  **Ensemble's project files in git**
  (Packages/, the .csproj files, LFS for scenes): TODO.md.
- **Chat**, the rest of the 0.0.1 goal.  Not designed (TODO.md).
- **What the client is sent after CharacterEnteredWorld**: the world around it (chunks, `region.map`),
  other players, movement.  `design/world.md`, `design/gameclock.md`.
- **Editing characters and NPCs** from GAME MANAGEMENT: whether an edit goes to the row or the copy in the
  world, what can be edited, the routes.  TODO.md.
- **The world's part two**: saving changed chunks, and then **a `world_size` change that keeps the
  digging**.  **A character saved outside a smaller world**.  **Loading around players who move**.  **An all-air
  chunk that costs nothing**, now that most of a player's 891 are air.  **A mountain**, once the client is
  up.  `design/world.md`, LONGTERM_TODO.md.
- **What goes in each of the GameClock's checks**: an input packet, a brain, movement, the broadcast.
  `design/gameclock.md`.
- **Saving primlib's copies** with their UUIDs and internal names (and the snapshot's cost when they join
  the world save: 32.88 ms for 10,000 characters), **the spawn system** for NPCs, **primlib in Lua** (part
  2).  `design/primlib.md`, TODO.md.
- **What a region does**, and blending biomes.  **A Tick evaluator tab**.  In TODO.md.
- **The blocked names list** (whether it checks character names too is open), **playtime metrics**,
  **moving `wgui_port` into `wgui.cfg`**, **the stale words in the code**, **where the test client lives**:
  in TODO.md.
- Archivist retrying on its own while disconnected; the Debug switch in `conductor_globals.cfg`; catching
  Ctrl-C.
- The server on Windows with a database.  GitHub issue #10, "Windows x86/x64 Untested" (Jacob opened it).
- **Soundcheck**, the patcher, and a certificate for every client.  LONGTERM_TODO.md.
- **A GDD**: Jacob is writing one with another chat.  What it settles comes in through him and goes into
  these docs.
