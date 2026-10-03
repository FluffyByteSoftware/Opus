<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is eleven crates under `Conductor/dev/`, a folder each without the `conductor-` in front while
the crate keeps it: the tools, accounts, the monitor, networking, the player commands, the Lua parser,
primlib, GameWorld, the GameClock, the web admin and the launcher.  README.md says what each is for, and
PROJECT_OPUS.md has every file and the table of the named pieces.

**The three programs, and how a player gets in** (2026-10-02): **Soundcheck** (`Soundcheck/dev/`, C# on .NET
10 with Avalonia 11, `design/soundcheck.md`) is the launcher the player opens.  At start, with the login
locked, it checks the game's folder against the manifest for its OS from the web folder (`/opt/storage/WWW`,
served at `http://opusensemble.duckdns.org:8553/`; PATCH_MANIFEST.md, format 3) and patches a file at a time;
then the login over TLS 1.3, and PLAY logs in a second time and starts **Ensemble** with the Ticket in its
environment.  Ensemble (Unity 6000.6) never logs in: it sends Connect over UDP, starts on character select,
and PLAY puts the character in the world with the HUD and the chat window over the scene.  When the session
ends it starts Soundcheck again with why, and closes; with no ticket (started by hand, or in the editor) it
shows the start screen, which in the editor is dev mode, watching for the ticket Soundcheck's `--debug`
SUBMIT writes.  Admin mode (`--admin`) publishes a platform's build into the web folder.  Conductor has no
part in the patcher.  **The game's name is Forgotten Legends**; the project, its folders and code stay Opus.
Everything speaks protocol version 11.  CLAUDE.md's "Client rules" and "Launcher rules" have the detail.

**The world to the client** (0.0.13) has its first half, the bulk: GameWorld writes `simple_overworld.map`,
the world's rough shape for the client's distance, before the door opens (session 1), and **every PLAY sends
it** over UDP behind a red loading bar, checked by its SHA-256 and kept in the player's folder, before
PlayerReady puts the character in the world (session 2, protocol version 11; PROTOCOL.md's "The map at
PLAY").  An account waits `map_cooldown_seconds` (300) between maps, Jacob's DDOS protection.  Nothing draws
the map yet, and the chunks themselves aren't streamed: that's the other half.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: **0.0.1 is released** (2026-10-02): the tag `0.0.1` is at `bc7009e`, with the two packages on
the GitHub Release.  `main` is two docs commits past the tag; `testing` and `unstable` are level with each
other, sessions 0, 1 and 2 past `main`.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor as released, and sessions 0 and 1 since (375 tests pass),
Soundcheck through PLAY and the way back, and Ensemble through the start screen, dev mode and the launcher's
ticket.  **Built and tested 2026-10-02**: PUBLISH, the check at start, the patch (two files, and the game's
program itself fetched back runnable), the extra file, the web server down, a 404, a file missing from the web
folder, a bad hash, `--debug`, the farewell after a KICK, all against the real address.  **Built and tested 2026-10-03, session 2**: the map at PLAY, both halves,
every check (the test client, the editor, a fresh build from Soundcheck, the hash and the file byte for
byte, a file that can't be written), and the cooldown's refusal.  **The map takes 0.08 to 0.16 s on the LAN**
(16.8 MB at `world_size` 32); over the internet a guess of some 13 s at a 50 ms ping, unmeasured.  **Hashing the 655 MB
build takes 1.3 s** at a start (193 files; the disk cache does most of that, a cold start will be slower).
TEST_CHECKLIST.html has the cooldown's last four checks (the tests, the Settings tab, the editor's red band,
0 turning it off, another account inside the first's wait) and two Parked: the launcher's own restart after
a patch (needs Soundcheck shipped beside the game) and Spans for real.  **On Windows**: Conductor builds
and runs, START SERVER included, without a database; nothing since the world has been tried there (GitHub
issue #10), and neither Soundcheck nor the new Ensemble has been built there at all.  The `.meta` round from
the login's move landed before session 0 (`git ls-files` shows none of the old login's files).

## Jacob's map (2026-09-30, and on)

What's still ahead, in his words.  The steps already taken (the character as a template, character select,
the spawn, chat, Soundcheck's login and patcher) are in the design files with the rest of what he said.

**The 0.0.1 goal**: "get a player spawned in the world and able to chat."  **Done and released** (2026-10-02).

On the versions ahead (2026-10-03): WAYPOINTS.md, his flow chart: 0.0.13 is the world served up to the
client ("way points of its own we'll say 0.0.13"), 0.0.2 "movement working and synchronzied over the
network with other clients", 0.0.3 "primitive NPCs in the game", 0.0.4 "more complex world generation",
"then I'm not sure from there".  (On 2026-10-02 movement was "0.0.0.12"; the
chart's numbers are the ones now.)

On the world (2026-10-01): "we will test a mountain out after we get the client up".

On shipping (2026-10-02): **"next session we need to get this all ready to ship to another person on
Windows... I need to make sure soundcheck is set up properly to validate off the host and I don't think it is
yet"**.  The check and the patch are built since; the Windows builds aren't (TODO.md, "Soundcheck").

At the patcher's hand-off (2026-10-02): **"we got a bit more polish to do next session.  Then we're going to
start working on the hard part getting the world to the client"**.  At the clean-up's (2026-10-03): **"I
think we are going to have to figure out how to serve the world up to the client :)"**.  His to change.

At session 1's hand-off (2026-10-03): **"next session we prepare Ensemble for receiveing this"**, the
simple overworld map.  Done in session 2.

At session 2's hand-off (2026-10-03): **"we'll move on to the next conversation where we're gonna stream the
chunks"**.  His to change.

The game is built into **`Ensemble/build/Linux/0.0.12/`** ("yeah we'll build a 0.0.12").

## The last day

A line a session, newest last, Jacob's "oh shit" fallback (2026-10-03): what each did, so a lost chat or a
wrong turn can be seen at a glance.  A line older than a day comes out unless it still bears on what's next.
**Sessions are numbered from 0**, which is 2026-10-03's docs clean-up (Jacob: "we're going to start tracking
each session date and number starting from 0 on this one"); the next is 1, and so on.  The lines before it
have no number.

- **2026-10-02**: chat on the server (`/chat`, protocol version 8), `/who` and `/who list` (version 9),
  Spans, the anti-flood for every command, the test client's Ctrl-C.  Built and tested.
- **2026-10-02**: Ensemble's chat window, `/who`'s box and `/camp`; the commands moved into
  `conductor-player-commands`; EverQuest's keys (Enter, `/`, Escape, `GameFocus`).  Built and tested.
- **2026-10-02**: the 0.0.1 review's four bugs and seven risks fixed; PleaseWait (version 10); **0.0.1
  released**: `main` to `testing`'s tip, the tag, the two packages, INSTALLATION_INSTRUCTIONS and RELEASE.
- **2026-10-02**: Soundcheck's first step: the login over TLS 1.3, Remember Me, admin mode's manifest,
  debug mode.  Built and tested.
- **2026-10-02**: the login out of Ensemble; the start screen and dev mode; PLAY starts the game with the
  ticket; the way back with the reason.  Built and tested; the old login's files deleted.
- **2026-10-02**: two manifests from a web address, then Jacob's redesigns: the web folder, PUBLISH, the
  check at start, the patch a file at a time.  Built and tested against the real web folder.
- **2026-10-02**: REPORT.html, the easy version of how Opus hangs together, with diagrams.
- **Session 0, 2026-10-03**: the docs clean-up, pass one: facts the code had overtaken, the built history out of
  TODO.md, the doubles merged, Soundcheck's first design told as history; the stale words in the code;
  CLAUDE.md cut to the rules and a "where to look" map; this rolling day.  Built and tested in session 1.
- **Session 1, 2026-10-03**: the world to the client designed with Jacob (the bulk and the stream; the bulk
  smoothed; sent by Conductor over UDP at PLAY; PlayerReady; one world save); `simple_overworld.map` written
  by GameWorld before the door opens, with its contract.  Built and tested (1.17 s at `world_size` 16).
- **Session 2, 2026-10-03**: the map at PLAY, protocol version 11 (the offer, the pieces 64 at a time,
  PlayerReady), Conductor and the test client, then Ensemble (`MapDownload`, `SimpleOverworldMap`, the red
  loading bar, back to the launcher on trouble); the map's cooldown by account, `map_cooldown_seconds`.
  Built and tested (0.16 s for 16.8 MB on the LAN), but for four of the cooldown's checks.

## Last session -- session 2, 2026-10-03, the map at PLAY

Jacob opened unsure whether it was the client or more Conductor; STATUS.md had his "next session we prepare
Ensemble for receiveing this".  His pick: "A packets first in the server and then we'll in this conversation
also integrate Ensemble with receipt of those packets."  Every answer is in TODO.md ("The world's dump") and
`design/world.md`.

- **The packets** (protocol version 11, the group `0x4_`, the ground): UserPressPlay loads the character and
  holds it on the player (`sessions::parked()`), answered with an **OverworldMapOffer** (size, pieces of
  1,024, SHA-256); the client asks with **OverworldMapRequest** (up to 64 pieces, no ask number) and the UDP
  thread sends **OverworldMapPiece**s out of packets built once when the door opens (`networking/src/
  overworld.rs`, the map's size again in RAM to spend no CPU); **PlayerReady** (`0x29`, the hash) puts the
  character in the world, answered with CharacterEnteredWorld.  GameWorld keeps the map's bytes
  (`overworld_map()`); `sha2` joined networking (Jacob: "ok").  PROTOCOL.md has "The map at PLAY" with a
  worked example; the test client fetches it at `--play` (`--save-map`, `--wrong-map-hash`).
- **Jacob's answers**: a fresh download every PLAY ("we're just gonna write over whatever the client already
  has every time"); the bar "draw over top the character select list please and look like an enemy healthbar
  going backwards lol", a red bar filling ("Yes we'll make it sexy later and look like you're fighting a
  boss"); a client that can't get the map goes back to the launcher with the message.
- **Ensemble**: `Code/Net/MapDownload.cs` (the pieces, the next 64, again after 250 ms, given up after 10 s
  without one), `GameConnection` drives it, `Session`'s new LoadingWorld stage checks the hash, reads the
  map once (`Code/World/SimpleOverworldMap.cs`, namespace `Opus.World`, `Current`) and writes it over
  `simple_overworld.map` in the player's folder, then PlayerReady; trouble ends the session and goes back
  to the launcher with "Couldn't get the world's map. Delete <the file> (or reinstall the game) and try
  again."  The bar is `CharacterSelectLoadingWidget.cs`.  The Console says how long the download took.
- **A wrong turn in the testing**: Soundcheck started the old built game, which didn't know the offer: "The
  server didn't answer.", then "Your character is on its way into the world." on a second PLAY.  The editor
  with Soundcheck's `--debug` worked.  It showed **the stuck PLAY**: a new PLAY while the character is held
  is refused until the session ends.  A re-offer was offered; Jacob asked for a cooldown instead, and the
  re-offer was left out ("leave it out"): a second offer is a second download.
- **The map's cooldown** (Jacob: "server puts a cooldown on an IP after it downloads and that IP must wait 5
  minutes", then by account, "its more for DDOS protection I think"): from the offer, `map_cooldown_seconds`
  in `networking.cfg` (300, 0 to 3600, 0 off), a PLAY inside it refused before anything is read, "You are
  temporarily cooling down from download for DDOS protection. You have <X> seconds remaining."  Kept past
  the player's leaving, gone at STOP SERVER.  So a log out and back in inside five minutes can't PLAY.

**Built and tested by Jacob**: everything but four of the cooldown's checks (TEST_CHECKLIST.html).  The map:
0.08 s in the test client, 0.16 s in the editor, for 16,777,244 bytes in 16,385 pieces at `world_size` 32.

## Where the next session starts

**Jacob's pick**: "the next conversation where we're gonna stream the chunks".  What's written already:
the chunk's file layout (`design/world.md`, 32,768 blocks, 64 KB a chunk), `view_chunks` in `game.cfg` (4),
the group `0x4_` kept for the ground (PROTOCOL.md), `MapDownload` meant as "the same tool set", TODO.md's
lean of run-length squeezing written by hand for the stream (not OKed), hidden things sent ahead with one
place on the server to hold things back.  **Not designed**: whether the server pushes the chunks around a
player or the client asks for them, what's sent before PlayerReady (the chunks under the spawn?), how a
chunk goes in pieces and how a lost one is asked for again, squeezing, and how the client puts blocks on
screen.  Nobody moves yet (movement is 0.0.2), so the chunks around where a character stands are all there
is to stream for now.  Plan with him first.

Accounts to log in with: `testuser123` / `Testpass123!` (Tester), and `testuser456` / `Testpass1!` (Chatter).

## What's waiting

- **Soundcheck's rest** (TODO.md): Conductor's report up and debug clients
  (open), Windows, one package; and the certificate for every client (LONGTERM_TODO.md).
- **The simple overworld map's rest** (TODO.md, "The world's dump"): drawing the distance from
  `SimpleOverworldMap.Current`, writing it again at the world save once blocks change, a timing over the
  internet (and asking for the next 64 before the last are in, if it drags), the stuck PLAY (a new PLAY
  while the character waits on the map is refused until the session ends; LOG OUT is the way out).
- **The chunks streamed to the client**: Jacob's pick for next (above).
- **The 0.0.1 code review's rest** (`CODE_REVIEW_0.0.1.md`): R8 onward, the inefficiencies and the stale
  words.
- **Saying things without a `/`**, **kicking a player who keeps flooding**, **`/help`**, **whether the web
  admin sees the chat**, **who may see positions**, **a Math class on the client**: TODO.md.
- **The Remember Me file is readable by other users on the same Linux machine.**  TODO.md.
- **Ensemble's client code**: the world on screen, and the HUD's Phases 2 and 3.  **Ensemble's project files
  in git** (Packages/, LFS for scenes): TODO.md.
- **Movement** (0.0.2 on WAYPOINTS.md), and what the client is sent after CharacterEnteredWorld.
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
