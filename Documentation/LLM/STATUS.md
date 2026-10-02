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
remembered.  Admin mode (`--admin`) writes `patch_manifest.json` of a client folder.  **The manifest check
isn't built**: Soundcheck logs in and starts the game without checking a single file against the server, and
Conductor has no manifest packets, no downloader and no `patch.cfg` yet.  **The game's name is Forgotten
Legends**; the project, its folders and code stay Opus.  Ensemble speaks protocol version 10.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: **0.0.1 is released** (2026-10-02): the tag `0.0.1` is at `bc7009e`, with the two packages on
the GitHub Release.  `main` is two docs commits past the tag; `testing` and `unstable` are level with each
other, this session's work past `main`.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor as released (365 tests), Soundcheck through PLAY and the way
back, and Ensemble through the start screen, dev mode and the launcher's ticket; every check this session
passed, and TEST_CHECKLIST.html is back to its one Parked check (Spans for real).  **On Windows**: Conductor
builds and runs, START SERVER included, without a database; nothing since the world has been tried there
(GitHub issue #10), and neither Soundcheck nor the new Ensemble has been built there at all.

**One thing not yet on GitHub** (at this hand-off): Jacob's `.meta` round for this session.  The old
login's files are still in the repo (`git ls-files` shows 30 of them) and the new files' `.meta`s aren't.
On his machine the `git rm` was run and Unity made the `.meta`s; `git meta` from `Conductor/dev` commits
and pushes both.  The next session checks `git ls-files Ensemble/dev/Opus.Ensemble/Assets | grep Login`
before anything else, and asks for `git meta` if it still lists them.

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
yet"**.  He's right: it isn't (above).  His to change.

## Last session -- 2026-10-02, the login out of Ensemble, PLAY, and the way back

Three steps, each planned, OKed, built and checked on its own.

- **Ensemble's half of the login move.**  Jacob asked what still used the old login; the answer was all of
  Ensemble's (by design: that half was left for this step) and three small things in Conductor (`tls12`,
  `0.0.0.1` in `client_versions`, a comment).  So the login screen, `LoginConnection.cs`,
  `ServerCertificate.cs`, the `Security/` folder, the eleven Login widgets, `login_default.json`,
  `login.uss` and `Data/Certs/` left Ensemble; `Net/Ticket.cs` reads the launcher's ticket from the
  environment (and, in the editor, Soundcheck's `debug_ticket.json`); `Session.cs` lost `LogIn()`,
  `Choose()` and the login's stages and events, and gained `Enter()`, `Notice`, `SessionOver` and
  `BackToTheLauncher()`; the start screen (`start_default.json`, `start.uss`, three widgets) replaced the
  login on ScreenRoot, whose Login Text Color and Font became Screen Text Color and Font
  (`FormerlySerializedAs`).  **The session couldn't delete the old files**: the harness refused the `git rm`
  as destructive, so the line went to Jacob and he ran it.  Every check passed in the editor.
- **PLAY in Soundcheck.**  A second login with the key in memory; its Ticket starts the game
  (`GameLauncher.cs`, `Process.Start` with the four variables in the environment), and the window closes.
  The server hands the second login a new ticket and lets the first die, so no "already logged in".  The
  game is beside the launcher, or `--game <path>`, remembered in `soundcheck_dev.json` since the game can't
  pass `--game` along on the way back.  Soundcheck's window grew to 840 x 1040 (debug mode's words didn't
  fit).  Every check passed against a fresh 0.0.12 build (the 0.0.1 build still had the login screen in it).
- **The way back says why.**  A KICK closed the game and Soundcheck opened knowing nothing, so Ensemble now
  hands it `OPUS_SESSION_OVER` and `OPUS_SESSION_TROUBLE`, shown in the status box at start, red for
  trouble.  Jacob: "this works locally".
- **Conductor wasn't touched.**  `tls12` and `0.0.0.1` can go now that nothing speaks 1.2 (TODO.md).

## Where the next session starts

**Jacob's pick**: ship to another person on Windows, and Soundcheck validating off the host first, since it
doesn't yet.  What that takes, unordered (TODO.md, "Soundcheck", has each):

- **Conductor's half of the manifest**: after the Login, the stamp down and the report up (the manifest in
  the protocol's own bytes, past the 4,096-byte frame cap or in pieces), the ask for files, a file in 3 MB
  pieces, the download's end; `PleaseWait` for the place in line; one downloader thread with a queue, paced
  to 15 Mbps; `patch.cfg` (soft: the limit, the piece size, the client folder, `allow_debug_clients`);
  `Content/patch/` checked at START SERVER; `serde` and `serde_json` (OKed).  `PROTOCOL_VERSION` bumps.
  Where the correct client folder is on the server is still open.
- **The check in user mode**: hash the install, compare with the stamp, ask for what's off, write each file
  to a temp beside it and swap it in, check again, report.  `--debug` skips it, only if the server allows.
- **Windows**: a Windows build of Ensemble (`Ensemble.exe`) and of Soundcheck, Soundcheck's log file (a
  windowed program has no terminal there), and whether `Process.Start` and the environment hand-off behave
  the same.  Nothing of either has been built on Windows yet.
- **One package**: Soundcheck and Ensemble together, the manifest written in admin mode and put in
  `Content/patch/`, with Unity's `Ensemble_BackUpThisFolder_ButDontShipItWithYourGame` left out.
  RELEASE.md's steps change.
- **The other person's server**: Conductor's `bind_address` and firewall (TCP 9997, UDP 9998), and
  `client_versions` listing Soundcheck's version; `0.0.0.1` and `tls12` out.

Nothing waits on a build, but the `.meta` round above has to land first.

Accounts to log in with: `testuser123` / `Testpass1!` (Tester), and `testuser456` / `Testpass1!` (Chatter).

## What's waiting

- **Soundcheck's rest** (above, and TODO.md): Conductor's half, the check in user mode, Windows, one
  package, Soundcheck patching itself, a new build as a patch; and the certificate for every client
  (LONGTERM_TODO.md).
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
