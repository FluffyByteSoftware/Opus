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
**Everything speaks protocol version 12** (session 3).  CLAUDE.md's "Client rules" and "Launcher rules" have
the detail.

**The world to the client** (0.0.13 on WAYPOINTS.md): **Conductor's side is whole**.  The bulk:
GameWorld writes `simple_overworld.map`, the world's rough shape for the distance, before the door opens
(session 1), and **every PLAY sends it** over UDP behind a red loading bar, checked by its SHA-256 and kept
in the player's folder, before PlayerReady puts the character in the world (session 2; PROTOCOL.md's "The
map at PLAY").  An account waits `map_cooldown_seconds` (5, down from 300 in session 6) between maps.  The detail: **the
client pulls the chunks around its character** (session 3; PROTOCOL.md's "The chunks around the player"),
up to 64 at a time, each squeezed as runs, out of a cache GameWorld's thread fills; the offer at PLAY says
where the character will stand and how many chunks it sees.  **Ensemble pulls them** (session 4), nearest
first, **and draws them** (session 5, `design/ensemble-world.md`): a mesh a chunk, faces only against air,
a material a block kind from GroundView's slots, a stand-in box for the Cinemachine camera.  **PlayerReady
waits until the nearest 99 are drawn** (session 6), 10 s at most, so nobody comes in on ground that isn't on
screen.  **The distance from the simple overworld map isn't drawn yet.**  New characters and RESET HOME
stand on top of a **spawn point**'s highest block (session 6, `design/world.md`), 0.5, 1, 0.5 today.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: **0.0.1 is released** (2026-10-02): the tag `0.0.1` is at `bc7009e`, with the two packages on
the GitHub Release.  `main` is two docs commits past the tag; `testing` and `unstable` are level with each
other, sessions 0 to 6 past `main`.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor as released, and sessions 0 to 6 since; Soundcheck through
PLAY and the way back, the check at start and the patch against the real web folder; Ensemble through the
start screen, dev mode, the launcher's ticket, the map at PLAY, the chunks and the ground on screen.
**Session 3, everything**: the build and the tests, the squeezed sizes, the test client pulling the whole
view, the refusal past the edge, the Services tab's line, Soundcheck and the editor at version 12.
**Measured**: the map takes 0.08 to 0.16 s on the LAN (16.8 MB at `world_size` 32), a guess of some 13 s over
the internet at a 50 ms ping; **a whole view of chunks at `view_chunks` 8 is 3,179 chunks, 88,746 bytes
squeezed, 0.04 s on the LAN**, the biggest chunk 693 bytes, every chunk one packet; squeezing is 13 to 14 us a
chunk; hashing the 655 MB build takes 1.3 s at Soundcheck's start.  **Session 5's meshing**: 437 chunks in
0.46 to 0.51 s, all on the worker, 302 drawn.  TEST_CHECKLIST.html has two open checks from session 6 (the
bar's three stages, too fast to see; GroundView switched off sends the player back after 10 s) and three
Parked: seeing a character stand on the GOLD (no player model yet), the launcher's own restart after a patch,
and Spans for real.  **On Windows**: Conductor builds and runs, START SERVER included, without a database;
nothing since the world has been tried there (GitHub issue #10), and neither Soundcheck nor the new Ensemble
has been built there at all.

**Jacob's settings worth knowing**: `world_size = 32`, `view_chunks = 8` ("keeping as 8", session 3; the
code's default stays 4), `world_save_seconds = 1800`, `map_cooldown_seconds = 5`, `bind_address =
10.0.0.84`.  In Unity: GroundView on a GameObject with five material slots (grass in GOLD's for now), the
stand-in Cube with CharacterStandIn, a Cinemachine camera on it; the scene isn't committed.

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
chunks"**.  Done in session 3, Conductor's half.

At session 3's hand-off (2026-10-03): **"prepare next conversation for wiring up ensemble to receive the
streams and the map and the hardest part - rendering it"**.  Receiving and the chunks drawn: sessions 4 and
5 ("THE GROUND WORKED"); the distance from the map, not yet.

At session 6's hand-off (2026-10-03): **"wrap it up bro!  We're good"**, no pick for next.  His to make.

The game is built into **`Ensemble/build/Linux/0.0.12/`** ("yeah we'll build a 0.0.12").

## The last day

A line a session, newest last, Jacob's "oh shit" fallback (2026-10-03): what each did, so a lost chat or a
wrong turn can be seen at a glance.  A line older than a day comes out unless it still bears on what's next.
**Sessions are numbered from 0**, which is 2026-10-03's docs clean-up (Jacob: "we're going to start tracking
each session date and number starting from 0 on this one"); the next is 1, and so on.  The lines before it
have no number.

- **2026-10-02**: the login out of Ensemble; the start screen and dev mode; PLAY starts the game with the
  ticket; the way back with the reason.  Built and tested.  (Kept: it's how Ensemble is tested in the
  editor, with Soundcheck's `--debug`.)
- **Session 0, 2026-10-03**: the docs clean-up, pass one: facts the code had overtaken, the built history out of
  TODO.md, the doubles merged, Soundcheck's first design told as history; the stale words in the code;
  CLAUDE.md cut to the rules and a "where to look" map; this rolling day.  Built and tested in session 1.
- **Session 1, 2026-10-03**: the world to the client designed with Jacob (the bulk and the stream; the bulk
  smoothed; sent by Conductor over UDP at PLAY; PlayerReady; one world save); `simple_overworld.map` written
  by GameWorld before the door opens, with its contract.  Built and tested (1.17 s at `world_size` 16).
- **Session 2, 2026-10-03**: the map at PLAY, protocol version 11 (the offer, the pieces 64 at a time,
  PlayerReady), Conductor and the test client, then Ensemble (`MapDownload`, `SimpleOverworldMap`, the red
  loading bar, back to the launcher on trouble); the map's cooldown by account, `map_cooldown_seconds`.
  Built and tested, cooldown and all.
- **Session 3, 2026-10-03**: the chunks streamed, Conductor's half, protocol version 12: pulled by the client
  (ChunkRequest, ChunkPiece, ChunkRefused), squeezed as runs by hand (`squeeze.rs`), cached by GameWorld,
  `may_see()` the one place that says what a player gets; the offer says where and how far; the test
  client's `--chunks`.  Built and tested: 3,179 chunks, 88,746 bytes, 0.04 s.
- **Session 4, 2026-10-03** (its own chat, no hand-off of its own; written in from its commits by session
  6): Ensemble receives the chunks (`ChunkDownload`, `Chunk`, `Ground`), nearest first, PlayerReady once the
  nearest 99 were in.  Built and tested: 3,179 in 0.12 s, the nearest 99 in 0.02 s, 35.1 MB held.
- **Session 5, 2026-10-03** (the same): the ground on screen (`ChunkMesher`, `GroundView`,
  `CharacterStandIn`, `design/ensemble-world.md`), a fixed Cinemachine camera on a stand-in.  Built and
  tested in session 6: "THE GROUND WORKED", 437 chunks meshed in 0.51 s.
- **Session 6, 2026-10-03** (session 1's chat, reopened): the map's cooldown down to 5 s; Soundcheck's
  window fits itself, 5 % bigger, and keeps that size; PlayerReady once the nearest 99 are drawn, 10 s at
  most; spawn points, a new character and RESET HOME on top of the highest block.  All built and tested
  but two Unity checks.

## Last session -- session 6, 2026-10-03, the afternoon after the ground

Session 1's chat, reopened after sessions 4 and 5 had run in chats of their own and left no hand-off (both
are written into the rolling day above from their commits).  Jacob came back to "THE GROUND WORKED" and
asked for a run of small things, each planned, OKed, built and checked on its own:

- **The `.meta` round** for sessions 4 and 5's files (Jacob's `git meta`), and the scene set up: GroundView
  wasn't on a GameObject at first, so nothing was drawn and nothing was said.  **Measured**: 437 chunks
  meshed, 246,903 faces, in 0.51 s on the worker, 302 drawn (`design/ensemble-world.md`).
- **The map's cooldown, 300 s to 5** ("well our DDOS protection works xD"): the default and the committed
  `networking.cfg`.
- **Soundcheck's window fits itself** ("is there any way to rely on avalonia to make the window the right
  size?"): `SizeToContent`, then 5 % of that added round the edges and the size fixed; the pieces that come
  and go keep their room (`Screens/Reserved.cs`), the status box four lines and scrolling
  (`design/soundcheck.md`, "The window's size").
- **PlayerReady once the nearest 99 are drawn**, not only in ("I just don't want a situation where a player
  walks forward and 'falls' until the server catches up"): GroundView says each chunk it's done with, the
  bar says "Drawing the ground...", and 10 s without it sends the player back with his words.  No packet
  change.
- **The ground stays where it's drawn** (a block at y fills y to y+1); characters stand on top of it.
- **Spawn points** (turned round from "start at Y=1" to "a designated spawn point ... put the player on top
  of the highest voxel"): a column, `SPAWN_POINTS` in `gameworld/src/spawn.rs`, its top worked out on
  GameWorld's thread from the chunk files or the region's ground, so any column in the world will do;
  only new characters and RESET HOME use it; the middle of the block.  **Tested**: Poopy (new) and Tester
  (reset) at 0.5, 1, 0.5; Asdf and Chatter still at 0, 0, 0, as saved.
- Earlier in the same chat (session 1, the morning): a web admin login that "kicks you to a random web
  page" couldn't be made to happen again; nothing in `page.html` or the server navigates anywhere.  If it
  comes back: the URL it lands on, the browser, a private window.

## Where the next session starts

**No pick from Jacob** ("We're good").  What's open, for him to choose from:
- **0.0.13's last piece, the distance**: the simple overworld map drawn as coarse ground past the chunks
  (16 by 16 blocks a patch, coloured by its top block; a million patches at `world_size` 32, so tiles or a
  coarser cut further out).  Whether 0.0.13 needs it before it's released is his call (WAYPOINTS.md).
- **0.0.2, movement**, the next waypoint: the GameClock loading around players who move, chunks forgotten
  when nobody's near, the client pulling more as it walks.
- The two open checks in TEST_CHECKLIST.html (session 6).

Accounts to log in with: `testuser123` / `Testpass123!` (Tester, Chatter, Poopy).

## What's waiting

- **The world to the client's rest**: the distance from the simple overworld map, drawn.  Conductor's
  leftovers (TODO.md, "The chunks streamed"): a stamp on a chunk's pieces once blocks change, forgetting
  chunks nobody's near once players move, the GameClock loading around a player who isn't at 0,0,0, a faked
  address getting a player flooded (`design/conductor-networking.md`).
- **Soundcheck's rest** (TODO.md): Conductor's report up and debug clients (open), Windows, one package;
  and the certificate for every client (LONGTERM_TODO.md).
- **Spawn points' rest** (TODO.md): which one a character gets once there are many; the height from a
  column changed since the last world save, once blocks change.
- **A web admin login that went to "a random web page"**, once, not seen again (above).
- **The simple overworld map's rest** (TODO.md, "The world's dump"): writing it again at the world save
  once blocks change, a timing over the internet, the stuck PLAY (LOG OUT is the way out).
- **The 0.0.1 code review's rest** (`CODE_REVIEW_0.0.1.md`): R8 onward, the inefficiencies and the stale
  words.
- **Saying things without a `/`**, **kicking a player who keeps flooding**, **`/help`**, **whether the web
  admin sees the chat**, **who may see positions**, **a Math class on the client**: TODO.md.
- **The Remember Me file is readable by other users on the same Linux machine.**  TODO.md.
- **Ensemble**: the HUD's Phases 2 and 3; its project files in git (Packages/, LFS for scenes): TODO.md.
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
