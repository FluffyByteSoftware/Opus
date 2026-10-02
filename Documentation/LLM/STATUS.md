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
over TLS 1.2 to the Ticket, then UDP, and on to **character select**, a third screen: the account's
characters, a click to pick one, PLAY, CREATE, DELETE and RESET HOME, and LOG OUT.  PLAY puts the character in
the world, and the screen says "In the world as <name>" with LOG OUT; there's no world on screen yet.  **The game's name is Forgotten Legends**; the project, its folders and
code stay Opus.  Unity's Company Name is FluffyByte and its Product Name Opus.Ensemble.  Every file the game
keeps for a player goes in `~/.config/unity3d/FluffyByte/Opus.Ensemble/` (`PlayerFiles.cs`).

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` is at `90f2e6f`, released 2026-10-02 at this session's end (Jacob: "merge everything
to main as well", then "move main up to the most recent build since everything tested well"): the client's net
code, TLS 1.2, character select's buttons, and the `.csproj` and solution files out of git.  `unstable` and
`testing` are level with each other, ahead of `main` only by this line.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor, `world_size`, the password's key (protocol version 7) and TLS
1.2 included.  **In Unity**: Copy Anims From FBX Pack, its CLASHES view included, the HUD's Phase 1, the login,
the password's key with Remember Me, logging in to character select, and character select's CREATE, DELETE, RESET
HOME and PLAY, every check passed.
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

At the hand-off of the client logging in to character select (2026-10-02): **"Hand off to
create/delete/select/play character next round"**.  Done.

At this hand-off (2026-10-02), character select done: **"actually next session we're gonna make it so when you
log in you get a chat box -- server doesn't support this yet so that will be thes ession after next"**.  So the
client's chat box first, then the server's side of chat.  His to change.

On the milestones (2026-10-02, opening the server's side of chat): **"If we can get it where people can log
in and chat with each other... that's release 0.0.1 then movement is 0.0.12"**, and then, asked: movement is **"0.0.0.12"**.  His to change.

## Last session -- 2026-10-02, character select's CREATE, DELETE, RESET HOME and PLAY (Ensemble)

**Built and tested** in Unity against Conductor on Linux, all eleven checks passed (Jacob: "the fucking loop
worked!"); nothing waits on a build.  `design/ensemble-networking.md`, "Character select, the rest of it", has it.

- **Jacob's answers**: the screen as proposed (a click picks a row; PLAY, CREATE, DELETE, RESET HOME as widgets
  of their own under the list; CREATE and DELETE open a card); on CharacterEnteredWorld "Just display a message
  "in the world as <Shortname>" and then a log out button"; the client checks the name rule before it sends
  ("we may as well prevent spamming the server if possible.  The server will hard check too"); RESET HOME is a
  button ("oh yes yes yes yes"); no double-click to play ("you need to explicitly hit play").
- **The client**: `GameConnection.Ask(kind, params string[])` and every character select answer read;
  `Session.cs` makes the four asks and routes each answer by the ask that's out; `CharacterSelectForm.cs`
  rewritten for the pick, the buttons, the two cards and the status line; seven new widgets; the layout and
  style sheet.  Conductor and the protocol didn't change.
- **The server's TLS key had gone into git** with the last `.meta` commit (`Assets/Data/Certs/conductor.key`,
  beside two copies of the certificate).  All three are out, the `.gitignore` keeps out every `*.key`, and
  Jacob made a new pair with README's openssl command and copied the certificate in as `conductor_crt.txt`.
  The old key is still in git's history, but nothing uses it now.
- **Jacob's commit of the new `.meta` files and the new certificate** landed (`06571e6`): the seven widgets'
  `.meta`s, `conductor_crt.txt`, and `Content/certs/conductor.crt`, the two the same bytes, no `.key`.

## Where the next session starts

**A chat box on the client** (Jacob: "next session we're gonna make it so when you log in you get a chat box --
server doesn't support this yet so that will be thes ession after next").  The server's side of chat is the
session after.  **Still to ask**: where "when you log in" puts it (in the world after PLAY, where the screen
only says "In the world as <name>" today, or the HUD with its chat placeholder from Phase 1, or somewhere
else), and what it does before the server carries chat (shows what's typed locally, the way the HUD's
placeholder does).  Chat itself isn't designed: who hears whom, the packets (a protocol bump), whether the
web admin sees it (TODO.md).

Accounts to log in with: `testuser123` / `Testpass1!`.

## What's waiting

- **The Remember Me file is readable by other users on the same Linux machine.**  TODO.md.
- **Ensemble's client code**: the chat box, the world on screen, and the HUD's Phases 2 and 3 (the catalog's export, the web layout editor).  **Ensemble's project files in git**
  (Packages/, LFS for scenes; the .csproj and solution files are out of git now): TODO.md.
- **Chat**, the rest of the 0.0.1 goal: the client's box next, the server's side after.  Not designed (TODO.md).
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
