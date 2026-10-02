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
TCP that hands a player a ticket for UDP, the UDP side, character select and the spawn (Protogame), a ledger
of every connection, and the access lists.  `conductor-player-commands` (lib) is what a player types in the
world: the table of commands and the anti-flood, `/chat` and `/who`.  `conductor-lua-parser` (lib) runs the
Lua scripts, locked down, and reads saved GameObjects back.  `conductor-primlib` (lib) is the game library,
an ECS in memory, with the Living and Character templates.  `conductor-gameworld` (lib) is GameWorld, the
ground.  `conductor-gameclock` (lib) is the GameClock, the game loop: five checks of 50 ms to a 250 ms cycle;
it owns primlib's `World` and GameWorld's `Terrain`, takes players' characters in and out through a mailbox,
sends the chat out and answers `/who list` from its broadcast check, and saves the world.  `conductor-wgui`
(lib) is the web admin at `http://127.0.0.1:9996/Opus`.  `conductor-launcher` (bin) boots the program and
waits on the web admin's Server tab.

**The three programs, and how a player gets in** (2026-10-02): **Soundcheck** (`Soundcheck/dev/`, C# on .NET
10 with Avalonia 11, `design/soundcheck.md`) is the launcher the player opens.  It does the login over TLS 1.3
(the password's key, Remember Me, the other-session choice), and PLAY logs in a second time and starts
**Ensemble** with the Ticket in its environment (`OPUS_SERVER`, `OPUS_UDP_PORT`, `OPUS_TOKEN`,
`OPUS_SOUNDCHECK`), then closes.  Ensemble (Unity 6000.6; its project settings and our four folders under
`Assets/` are committed) never logs in and never sees a password: it reads the ticket at start, sends Connect
over UDP, and the first screen a player sees is character select; PLAY puts the character in the world with the
HUD and the chat window over the scene.  When the session ends (Kicked, the server gone, LOG OUT, `/camp`) the
game starts Soundcheck again with why in its environment (`OPUS_SESSION_OVER`, `OPUS_SESSION_TROUBLE`) and
closes, so the player is looking at the login with the reason in the status box; `/camp desktop` and QUIT
close the game without.  A game with no ticket (started by hand, or in Unity's editor) shows **the start
screen**: "Start Forgotten Legends from the launcher." and QUIT.  In the editor that screen is **dev mode**:
it watches for the ticket Soundcheck's `--debug` SUBMIT writes to `debug_ticket.json` and joins the world when
a fresh one lands.  Soundcheck finds the game beside itself (`Ensemble.x86_64`), or `--game <path>`,
remembered.  **Admin mode (`--admin`) publishes a platform's build into the web folder**
(`/opt/storage/WWW`, served at `http://opusensemble.duckdns.org:8553/`): the build mirrored into
`download/<platform>/`, and `linux_manifest.json` or `windows_manifest.json` written at the root from it
(PATCH_MANIFEST.md, format 3).  **User mode checks the install at start** (built and tested 2026-10-02),
with the login locked: it fetches the manifest for the OS it's on (`--www <url>` for a test), hashes the
game's folder against it, fetches whatever's missing or changed a file at a time into a temp beside it and
swaps it in, checks again, and unlocks the login on a pass; a launcher that replaced one of its own files
starts itself again (`--patched`).  Conductor has no part in it: it sends nothing and serves nothing.  **The game's name is Forgotten
Legends**; the project, its folders and code stay Opus.  Ensemble speaks protocol version 10.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: **0.0.1 is released** (2026-10-02): the tag `0.0.1` is at `bc7009e`, with the two packages on
the GitHub Release.  `main` is two docs commits past the tag; `testing` and `unstable` are level with each
other, this session's work past `main`.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor as released (365 tests), Soundcheck through PLAY and the way
back, and Ensemble through the start screen, dev mode and the launcher's ticket.  **Built and tested 2026-10-02**: PUBLISH, the check at start, the patch (two files, and the game's program
itself fetched back runnable), the extra file, the web server down, a 404, a file missing from the web
folder, a bad hash, `--debug`, the farewell after a KICK, all against the real address.  **Hashing the 655 MB
build takes 1.3 s** at a start (193 files; the disk cache does most of that, a cold start will be slower).
TEST_CHECKLIST.html is down to two Parked checks: the launcher's own restart after a patch (needs
Soundcheck shipped beside the game) and Spans for real.  **On Windows**: Conductor builds and runs, START SERVER included,
without a database; nothing since the world has been tried there (GitHub issue #10), and neither Soundcheck
nor the new Ensemble has been built there at all.  The `.meta` round from the login's move landed before
this session (`git ls-files` shows none of the old login's files).

## Jacob's map (2026-09-30, and on)

**The 0.0.1 goal**: "get a player spawned in the world and able to chat."  **Done and released** (2026-10-02).

His words: "We are going to work on marrying the network code to the game by finishing out character as a
template for hydrating from an account.  Then we will build the character selection (start of UDP
connection), then the log in to the world, and spawn character in world."  And the flow: "account logs in
(done) -> character selection -> selected character spawns in world at its last save loc (0,0,0 for
now)".  His to change.

On the world (2026-10-01): "we will test a mountain out after we get the client up".

On the milestones (2026-10-02, opening the server's side of chat): **"If we can get it where people can log
in and chat with each other... that's release 0.0.1 then movement is 0.0.12"**, and then, asked: movement
is **"0.0.0.12"**.  His to change.

At the 0.0.1 hand-off (2026-10-02): **"next session we're gonna start on Soundcheck I think"**.

The Soundcheck session (2026-10-02): **"we should remove login from the game like monsters and memories
did.  The patcher should be what goes through the tcp and TLS then we hand over udp to the ensemble"**; the
check is of every file, **"the world is the least of our concerns"**; **"we're gonna build two modes to
Soundcheck one is the admin the other is a user"**; and a debug mode: **"every time I make a change to the
client (Ensemble) I don't want to have to repatch!"**  At the hand-off: **"I'll build a copy of the game and
put in a folder and we'll next session try to get it started from the patcher"**.

This session (2026-10-02): opened with **"can we check our existing code and see if we have any hang over in
Ensemble or the Server that uses the old log in method?"**; on the editor's way in, **"Play/Dev Mode but
yes"** (the game watches for the ticket file, no button); on the world's files, out of this pass; then
**"keep going with PLAY"**; the game is built into **`Ensemble/build/Linux/0.0.12/`** ("yeah we'll build a
0.0.12").  At the hand-off: **"next session we need to get this all ready to ship to another person on
Windows... I need to make sure soundcheck is set up properly to validate off the host and I don't think it is
yet"**.  His to change.

The manifests session (2026-10-02): **"we need to fix up soundcheck now so that admin mode builds a working
manifest for Windows and Linux -- then Soundcheck needs to know which environment its being run from in its
user mode... and then look for that manifest which we're gonna store at this web address"**; then, the same
day, a zip of the client ("Redesign number 23852357235"), then no zip: **"the patcher knows if they're on
linux or not and looks for linux_manifest.json or windows_manifest.json :P  Fuck it!  Then we'll reach to
the opusensemble.duckdns.org:8553/download/windows/<this will mimic the client directory so you find the
file> and the same for Linux?  our admin patcher can pack and move the files where they need to be"**.
So the stamp and the files come from a web folder, not from Conductor, and the world's dump (his) is a file
of its own there, later.  All three shapes are in `design/soundcheck.md`, in his words.

## Last session -- 2026-10-02, the web folder: PUBLISH, the check at start, the patch

Two rounds in one chat, built and tested the same evening.  The first (two manifests from a web address, the
check after SUBMIT) went up and was overtaken the same afternoon by Jacob's redesign (above); the second
is what's on `unstable` now, OKed by him ("yup") after the plan was read back.

- **The web folder** (PATCH_MANIFEST.md, format 3): `linux_manifest.json` and `windows_manifest.json` at
  its root, `download/linux/` and `download/windows/` exact copies of the clients.  A manifest line gained
  `executable` for a Linux program, since a download comes with no permissions.  The patcher's leftovers
  (`.patch`, `.old`) are skipped by the walk and cleaned up at the next start.
- **Admin mode** (`Patch/Mirror.cs`, `Screens/AdminScreen`): Linux / Windows radio buttons, a build folder
  remembered per platform, the web folder (default `/opt/storage/WWW`), PUBLISH: the mirror (copied by size
  and time, stale files taken out, Unity's backup folder skipped), then the manifest hashed from the mirror.
  The old `soundcheck_admin.json` remembered one folder; the box starts blank once.
- **User mode** (`Patch/ManifestSource.cs`, `Patch/Patcher.cs`, `Screens/LoginScreen`): the check runs at
  start with the boxes locked; what's off is fetched from `download/<platform>/<path>` (each piece
  URL-escaped) into a `.patch` temp, its hash checked, the execute bit set back, and swapped in; then the
  check again.  A replaced file directly in the launcher's own folder means `Patcher.Restart()` with the
  same command line and `--patched`, and the window closes; a `--patched` launcher whose check fails is
  "couldn't repair the game".  On Windows a file in use is renamed aside (`.old`).  The game's farewell
  ("You were kicked by the admin.") is kept in front of the check's words.  `--www <url>` replaced
  `--manifest`.  SUBMIT and PLAY are back to their simple shape: the pass is a flag they look at.
- **Nothing in Conductor changed**, and nothing of its half is left but the report up and
  `allow_debug_clients`, both open (TODO.md).

## Where the next session starts

The patcher is tested on Linux against the real web folder (above).  What the build taught, fixed the same
evening: the CA1416 guard has to be `OperatingSystem.IsWindows()`; a worker's last progress message can land
after the words that follow it, so each carries its phase; and the check wants the game's *folder*, not its
program, or a deleted `Ensemble.x86_64` stops the check instead of being fetched.

Unordered (TODO.md, "Soundcheck", has each): the world's dump (Conductor's, and its line in the
manifest); the Windows builds of Ensemble and Soundcheck published as the Windows half; Soundcheck's log
file for Windows; one package; the other person's server.

Accounts to log in with: `testuser123` / `Testpass1!` (Tester), and `testuser456` / `Testpass1!` (Chatter).

## What's waiting

- **Soundcheck's rest** (above, and TODO.md): the world's dump, Conductor's report up and debug clients
  (open), Windows, one package; and the certificate for every client (LONGTERM_TODO.md).
- **The world's files on the client** (`Assets/StreamingAssets/World/`, a fifth folder of ours), left out of
  this session's pass.
- **The 0.0.1 code review's rest** (`CODE_REVIEW_0.0.1.md`): R8 onward, the inefficiencies and the stale
  words.
- **Saying things without a `/`**, **kicking a player who keeps flooding**, **`/help`**, **whether the web
  admin sees the chat**, **who may see positions**, **a Math class on the client**: TODO.md.
- **The Remember Me file is readable by other users on the same Linux machine.**  TODO.md.
- **Ensemble's client code**: the world on screen, and the HUD's Phases 2 and 3.  **Ensemble's project files
  in git** (Packages/, LFS for scenes): TODO.md.
- **Movement** (0.0.0.12, Jacob's number), and what the client is sent after CharacterEnteredWorld.
- **Editing characters and NPCs** from GAME MANAGEMENT.  TODO.md.
- **The world's part two**: saving changed chunks, a `world_size` change that keeps the digging, loading
  around players who move, an all-air chunk that costs nothing, a mountain.
- **What goes in each of the GameClock's checks**, **saving primlib's copies**, **the spawn system**,
  **primlib in Lua** (part 2), **what a region does**, **a Tick evaluator tab**, **the blocked names list**,
  **playtime metrics**, **moving `wgui_port`**: TODO.md.
- Archivist retrying on its own while disconnected; the Debug switch in `conductor_globals.cfg`; catching
  Ctrl-C in Conductor.
- The server on Windows with a database.  GitHub issue #10.
- **A GDD**: Jacob is writing one with another chat.  What it settles comes in through him.
