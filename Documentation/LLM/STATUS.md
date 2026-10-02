<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is eleven crates, each in a folder without the `conductor-` in front (`Conductor/dev/tools/`) while
the crate keeps it (`conductor-tools`, `conductor_tools::` in code).  `conductor-tools` (lib) holds DiskMan,
Scribe, Constellations, Fingerprinter, Security, Archivist, the notices, the clock, the thread list, the
services list and the server's switch (`server.rs`).  `conductor-accounts` (lib) is the one way in to the
accounts table and `player_characters`, and the account desk.  `conductor-monitor` (lib) looks at the
process and the machine once a second.  `conductor-networking` (lib) is the front door: a login over TLS on
TCP that hands a player a ticket for UDP, the UDP side, character select and the spawn (Protogame), what a
player types in the world (a table of commands, `/chat` and `/who`, with an anti-flood), a ledger of every
connection, and the access lists.
`conductor-player-commands` (lib, 2026-10-02, waiting on a build) is what a player types in the world:
the table of commands and the anti-flood, `/chat` and `/who`, out of networking and wired to it by the
launcher.
`conductor-lua-parser` (lib) runs the Lua scripts, locked down, and reads saved GameObjects back.
`conductor-primlib` (lib) is the game library, an ECS in memory, with the Living and Character templates.
`conductor-gameworld` (lib) is GameWorld, the ground.  `conductor-gameclock` (lib) is the GameClock, the game
loop: five checks of 50 ms to a 250 ms cycle; it owns primlib's `World` and GameWorld's `Terrain`, takes
players' characters in and out through a mailbox, sends the chat out and answers `/who list` from its
broadcast check, and saves the world.  `conductor-wgui` (lib) is the web
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
the world, and **the HUD comes up over the Unity scene** (2026-10-02): the placeholder health bar and minimap,
and **the chat window**, bottom-left, 700 x 300, 50% black, in Jacob's Retro font: what's typed goes to the
server as typed and is echoed in yellow, the chat, refusals and `/who`'s box come back in white, Spans are put
back together, and `/camp` (to the login) and `/camp desktop` (closes the game) are the way out.  **The game's
name is Forgotten Legends**; the project, its folders and code stay Opus.  Unity's Company Name is FluffyByte
and its Product Name Opus.Ensemble.  Every file the game keeps for a player goes in
`~/.config/unity3d/FluffyByte/Opus.Ensemble/` (`PlayerFiles.cs`).  Ensemble speaks protocol version 10.

**Soundcheck is the third program** (2026-10-02, `design/soundcheck.md`): the launcher players open, C# on
.NET 10 with Avalonia 11, in `Soundcheck/dev/`.  **The login is moving out of Ensemble into it** ("like
Monsters and Memories did"): Soundcheck does the TLS login (1.3 again, since it isn't Unity's .NET) to the
Ticket, and will check every file of the installed client against a manifest the server holds before it
starts Ensemble, which will open on character select over UDP and never speak TCP.  **Built and tested**:
user mode logs in and remembers (the key takes 208 ms against Unity's 2852); admin mode (`--admin`) writes
`patch_manifest.json` of a client folder (`PATCH_MANIFEST.md`); debug mode (`--debug`) skips the check and
leaves the ticket in `debug_ticket.json` for an Ensemble in the editor.  PLAY is greyed: Ensemble can't take a
ticket yet, and Conductor has no manifest check yet.  Soundcheck ends the process itself on Avalonia's `Exit`
event, round an Avalonia shutdown crash on KDE's Wayland session (issue 19523).

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: **0.0.1 is released** (2026-10-02): the tag `0.0.1` is at `bc7009e`, with the two packages on
the GitHub Release.  `main` is two docs commits past the tag; `testing` and `unstable` are level with each
other, Soundcheck's first step past `main`.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor as released, 365 tests, and Soundcheck's first step, every
check passed; TEST_CHECKLIST.html holds one Parked check (Spans for real).  **In Unity**: everything through
the chat window, EverQuest's keys and PleaseWait passed.  **On Windows**: Conductor builds and runs, START
SERVER included, without a database; nothing since the world has been tried there (GitHub issue #10).

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
it with a real client".  Chat, the other half of 0.0.1, is the server's side done (2026-10-02); the client's
box is still to come.

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

After chat passed (2026-10-02): **"We're barely 30% so we're gonna keep going"**, with a `/who`.  Asked
whether to release chat to `main`: **"Keep in testing for now"**.

On the milestones (2026-10-02, opening the server's side of chat): **"If we can get it where people can log
in and chat with each other... that's release 0.0.1 then movement is 0.0.12"**, and then, asked: movement
is **"0.0.0.12"**.  His to change.

At this hand-off (2026-10-02, after `/who` and the anti-flood): **"wrap our documents up and we'll go work on
Ensemble again"**, and then, every check passed: **"next one we're gonna integrate chat into the client"**.
His to change.

During the chat window (2026-10-02): **"we need to rip the commands out of networking and put them into their
own crate I think... conductor::player_commands then we'll probably also have admin_commands"**, and "make
that todo for next session that seems urgent to me before we get too deep in commands".  Then, the chat
window pushed: **"Okay let's go ahead and push those commands to their own crate please we got tokens."**
Done, the same session.

At this hand-off (2026-10-02, the chat window and the commands crate): **"Next conversation we're gonna do
major clean up of code and documentation"**, and, asked whether to release: **"not yet we're gonna do code
clean up next session then merge to main and release 0.0.1"**.  His to change.

The release session (2026-10-02): **"we are ready to prepare for release 0.0.1"**, with a review of all of
Conductor first; then **"merge everything into main"**, and once the review was in, **"Wait till we fix bugs
and validate everything works"** before `main` moved again; **"we changing to version 0.0.1"**; the kick
found in testing: **"the client is told to wait and then pulled in"**, **"please add the PleaseWait feature
before we release"**.  And at the hand-off, 0.0.1 up: **"next session we're gonna start on Soundcheck I
think"**.  His to change.

The Soundcheck session (2026-10-02): **"we should remove login from the game like monsters and memories
did.  The patcher should be what goes through the tcp and TLS then we hand over udp to the ensemble"**; the
check is of every file, **"the world is the least of our concerns"**; **"we're gonna build two modes to
Soundcheck one is the admin the other is a user"**; and a debug mode: **"every time I make a change to the
client (Ensemble) I don't want to have to repatch!"**  At the hand-off: **"I'll build a copy of the game and
put in a folder and we'll next session try to get it started from the patcher"**.  His to change.

## Last session -- 2026-10-02, Soundcheck's first step

Jacob opened it with "Let us prepare development of a simpler app.  The c# avalonia app that will patch our
client", and the design grew as we talked (`design/soundcheck.md` has every answer; the short of it is under
"Where things stand").  Then the first step was built and every check passed.

- **New component**: `Soundcheck/dev/`, a .NET 10 project with the three Avalonia packages (11.3.22, the
  newest of the 11 line; 11.3.2, pinned from memory, was two years stale and pulled in a flagged DBus
  package).  CLAUDE.md has a "Launcher rules (Soundcheck)" section.
- **User mode**: Ensemble's login moved over as the same six files (`Net/Protocol.cs`, `Packets.cs`,
  `ServerCertificate.cs`, `LoginConnection.cs`, `Security/PasswordKey.cs`, `RememberedLogin.cs`) with new
  headers; `LoginConnection` asks for TLS 1.3 only and talks to the screen through `ILoginListener`.  The same
  `remembered_login.json` in the same player folder, so Remember Me is one file.  The certificate is
  `conductor.crt` beside the program.  The Ticket's token is dropped and PLAY stays greyed until Ensemble
  can take one.
- **Admin mode** (`--admin`): the client folder, the version, where to write, WRITE MANIFEST; remembers all
  three in `soundcheck_admin.json` in the player folder.  `Patch/Manifest.cs` writes and reads
  `patch_manifest.json`; `PATCH_MANIFEST.md` is its contract (format 1).
- **Debug mode** (`--debug`): the file check (to come) is skipped, and the ticket goes to `debug_ticket.json`
  in the player folder for an Ensemble running in Unity's editor.
- **The crash on close** was Avalonia's, on KDE's Wayland session (issue 19523, open): a late DBus message
  handed to the stopped window thread.  Not our threads, not the input method, not the global menu (a wrong
  guess, corrected).  Soundcheck now ends the process on Avalonia's `Exit` event, before Avalonia's shutdown.
- **Settled on the way**: PLAY is a second login; 3 MB pieces, no zip; Conductor serves the files itself at
  15 Mbps, one client at a time, the rest in line; `serde` and `serde_json` are OKed for Conductor's reader;
  the world's files go in `Assets/StreamingAssets/World/`; Soundcheck closes once Ensemble is up and
  Ensemble starts Soundcheck again on its way out; the ticket goes to Ensemble in environment variables.
- **Every check passed** (2026-10-02): the build, the window, the login, a wrong password, Remember Me across
  a restart, the other-session choice, admin mode's manifest, debug mode.  TEST_CHECKLIST.html is back to
  its one Parked check.

## Where the next session starts

**Jacob's pick**: "I'll build a copy of the game and put in a folder and we'll next session try to get it
started from the patcher."  So: a built Ensemble in a folder on his machine, and Soundcheck's PLAY starting it
with the ticket.  That is PLAY's second login in Soundcheck, Soundcheck starting Ensemble with the ticket in
its environment and closing, and Ensemble's half: reading the ticket at start, opening on character select,
and quitting back to Soundcheck when its session ends (TODO.md, "Soundcheck").  How Soundcheck finds the
game to start (beside itself, most likely, since they ship as one package) is to settle first.  Nothing waits
on a build.

Accounts to log in with: `testuser123` / `Testpass1!` (Tester), and `testuser456` / `Testpass1!` (Chatter).

## What's waiting

- **Soundcheck's next steps** (TODO.md, "Soundcheck", a step each): PLAY and Ensemble's half (above);
  Conductor's half (the manifest packets after the Login, the downloader thread and its 15 Mbps line,
  `patch.cfg` with `allow_debug_clients`, `Content/patch/` checked at START SERVER, `serde_json`); the check in
  user mode; where Conductor finds the correct client folder; a log file on Windows; Soundcheck patching
  itself; one package for a release.  Then Conductor's `tls12` feature can go.
- **The 0.0.1 code review's rest** (`CODE_REVIEW_0.0.1.md`): R8 onward, the inefficiencies and the stale
  words.  The biggest: Scribe writing to the console under its lock (R8), the Lua time limit not stopping a
  C call (R9), the page redrawing every tab every second (I2), DiskMan scanning its map per log line (I4),
  the all-air chunks (I1).
- **Saying things without a `/`**, nearby, once there are positions.  **Kicking a player who keeps
  flooding**, **`/help`**, **whether the web admin sees the chat**, **who may see positions**, **a Math class
  on the client**: TODO.md.
- **The Remember Me file is readable by other users on the same Linux machine.**  TODO.md.
- **Ensemble's client code**: the world on screen, and the HUD's Phases 2 and 3 (the catalog's export, the
  web layout editor).  **Ensemble's project files in git** (Packages/, LFS for scenes): TODO.md.
- **Movement** (0.0.0.12, Jacob's number), and what the client is sent after CharacterEnteredWorld: the
  world around it (chunks, `region.map`), other players.  `design/world.md`, `design/gameclock.md`.
- **Editing characters and NPCs** from GAME MANAGEMENT.  TODO.md.
- **The world's part two**: saving changed chunks, a `world_size` change that keeps the digging, a character
  saved outside a smaller world, loading around players who move, an all-air chunk that costs nothing, a
  mountain.  `design/world.md`, LONGTERM_TODO.md.
- **What goes in each of the GameClock's checks**: an input packet, a brain, movement, the positions in the
  broadcast.  `design/gameclock.md`.
- **Saving primlib's copies**, **the spawn system** for NPCs, **primlib in Lua** (part 2).
  `design/primlib.md`, TODO.md.
- **What a region does**, and blending biomes.  **A Tick evaluator tab**.  In TODO.md.
- **The blocked names list**, **playtime metrics**, **moving `wgui_port` into `wgui.cfg`**, **the stale words
  in the code**, **where the test client lives**: in TODO.md.
- Archivist retrying on its own while disconnected; the Debug switch in `conductor_globals.cfg`; catching
  Ctrl-C in Conductor.
- The server on Windows with a database.  GitHub issue #10.
- **A certificate for every client** (mutual TLS), Soundcheck's second half.  LONGTERM_TODO.md.
- **A GDD**: Jacob is writing one with another chat.  What it settles comes in through him and goes into
  these docs.
