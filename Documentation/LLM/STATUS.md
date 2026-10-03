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
map at PLAY").  An account waits `map_cooldown_seconds` (5, down from 300) between maps.  The detail: **the client pulls
the chunks around its character** (session 3; PROTOCOL.md's "The chunks around the player"), up to 64 at a
time, each squeezed as runs, out of a cache GameWorld's thread fills; the offer at PLAY now says where the
character will stand and how many chunks it sees.  **Ensemble's side is the map's download and nothing
more**: it reads the offer's new end and skips it, asks for no chunks, and draws nothing of the world.
That's what Jacob opens next (below).

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: **0.0.1 is released** (2026-10-02): the tag `0.0.1` is at `bc7009e`, with the two packages on
the GitHub Release.  `main` is two docs commits past the tag; `testing` and `unstable` are level with each
other, sessions 0 to 3 past `main`.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor as released, and sessions 0 to 3 since; Soundcheck through
PLAY and the way back, the check at start and the patch against the real web folder; Ensemble through the
start screen, dev mode, the launcher's ticket and the map at PLAY.  **Session 3, everything**: the build and
the tests, the squeezed sizes, the test client pulling the whole view, the refusal past the edge, the
Services tab's line, Soundcheck and the editor at version 12.  **Measured**: the map takes 0.08 to 0.16 s
on the LAN (16.8 MB at `world_size` 32), a guess of some 13 s over the internet at a 50 ms ping; **a whole
view of chunks at `view_chunks` 8 is 3,179 chunks, 88,746 bytes squeezed, 0.04 s on the LAN**, the biggest
chunk 693 bytes, every chunk one packet; squeezing is 13 to 14 us a chunk; hashing the 655 MB build takes
1.3 s at Soundcheck's start.  TEST_CHECKLIST.html has only its two Parked checks left (the test client's
summary passed, 2026-10-03): the launcher's own restart after a patch (needs Soundcheck shipped beside the
game) and Spans for real.  **On Windows**: Conductor builds and runs, START SERVER included, without a database; nothing
since the world has been tried there (GitHub issue #10), and neither Soundcheck nor the new Ensemble has
been built there at all.

**Jacob's settings worth knowing**: `world_size = 32`, `view_chunks = 8` ("keeping as 8", session 3; the
code's default stays 4), `world_save_seconds = 1800`, `bind_address = 10.0.0.84`.

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
streams and the map and the hardest part - rendering it"**.  His to change.

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

## Last session -- session 3, 2026-10-03, the chunks streamed (Conductor's half)

Jacob opened with: "prepare conductor for 'streaming' the world around the player in its chunk data and
voxel data I think... we essentially want to copy minecraft."  The simple overworld map is "a 'broad outline'
map... we're gonna use to draw at a distance"; the stream "is meant to give the high resolution details".
Every answer is in `design/world.md` ("The chunks streamed") and TODO.md.

- **His answers**: **pull** ("Pull your preference"), not Minecraft's push; before PlayerReady is "the
  clients call but I think we are gonna want to wait till most of the scene is filled?"; **squeezed by
  hand, runs only** ("yes absolutely", then "runs only for now"), after asking whether zipping on the fly
  would help (it might halve what's left; not worth a crate yet, and the measurement says there's little
  left); `view_chunks` as it is ("keep view_chunks 4", then, told his `game.cfg` says 8, "keeping as 8");
  **Ensemble in sessions of its own** "to bring it in line with these server changes".
- **Built**: GameWorld's thread squeezes every chunk it reads or builds (`gameworld/src/squeeze.rs`: a
  byte for how it's squeezed, the kinds listed, then runs) and keeps it until STOP SERVER
  (`squeezed()`); a chunk a player asks for that nobody loaded is read or built for them, "not yet"
  meanwhile.  Networking: **ChunkRequest** (`0x43`, up to 64 places), **ChunkPiece** (`0x44`, up to 1,192
  bytes a piece), **ChunkRefused** (`0x45`: outside the view, not yet, unavailable); `chunks.rs`'s
  `may_see()`, every chunk within the view of the character's column, every row; the book's
  `standing()`, set at PLAY.  The test client's `--chunks` and `--chunk-outside`.
- **A change from the plan**, found while writing: the client didn't know where its character stands
  until after PlayerReady, so it couldn't ask before it.  The **OverworldMapOffer now ends with x, y, z and
  the view** (u8).  So Soundcheck's and Ensemble's version went to 12, and Ensemble reads and skips the
  new end; nothing else of Ensemble's changed.
- **Measured** (Jacob, Linux): Alpha's 891 chunks squeeze to 7,938 bytes, Omega's to 36,925; a view at 8,
  3,179 chunks, 88,746 bytes, the biggest 693, in 0.04 s on the LAN, against 199 MB as they are.
- **A wrong turn in the testing**: the test client's `--chunk-outside` looked stuck after 100%: it was
  counting 104 million blocks one at a time in Python.  It counts from the runs now, and prints at once (passed).

## Where the next session starts

**Jacob's pick**: "wiring up ensemble to receive the streams and the map and the hardest part - rendering
it".  That's three things, and the session should say so and let him pick where to start and where to
stop: **receiving the chunks** (the net code), **drawing the chunks**, and **drawing the distance from the
map**.  Plan with him first; none of the client's half is designed.

**What's there to build on**:
- The contracts: PROTOCOL.md's "The chunks around the player" (the packets, which chunks, the squeezed
  layout, a worked example, **a C# `Unsqueeze()` to copy**) and "The map at PLAY" (the offer's new end:
  `f32 x, y, z, u8 view`); SIMPLE_OVERWORLD_MAP.md for the map (a patch is 16 by 16 blocks, its average
  height and commonest top block).  REGION_MAP.md if the client wants regions.
- Ensemble's net code (`Assets/Code/Net/`, `design/ensemble-networking.md`): `GameConnection.cs` reads the
  offer (the new end skipped, just after `packet.String()`), `MapDownload.cs` is the pattern for a
  download (the next 64, again after 250 ms, given up after 10 s, the sender thread woken by the listener),
  `Session.cs` has the LoadingWorld stage, where PlayerReady goes once the map is in.
  `Code/World/SimpleOverworldMap.cs` holds the map as `SimpleOverworldMap.Current`, read and drawn by
  nothing.  The test client's `fetch_chunks()` is a working pull to copy: nearest first, 64 at a time,
  "not yet" asked again.
- The block numbers: AIR 0, DIRT 1, STONE 2, WOOD 3, GOLD 4, BEDROCK 5.  A block is 1 m; a chunk 32 a side;
  rows 0 to 10 from y -32; block x, y, z inside a chunk is `(y * 32 + z) * 32 + x`.

**Not designed, to talk through with Jacob**:
- **Receiving**: when PlayerReady goes ("most of the scene": all of the view, or the rings nearest the
  character, or a share of it?); whether the red bar covers the chunks too; what a refused chunk does on
  screen.
- **Holding them**: a view at 8 is 3,179 chunks; as `ushort[32768]` each that's 199 MB on the client.  Kept
  as runs, or as bytes, or only the chunks with something but air in them?
- **Drawing the chunks**, the hard part: a mesh per chunk with only the faces between a block and air
  (Minecraft's way; a face at a chunk's edge needs the chunk beside it), maybe merged into bigger faces
  later; built off the main thread with Unity's Mesh filled on it; colours per block kind (plain colours,
  or the purchased art's textures?); 32-bit indices where a chunk needs them.
- **Where things are**: our x is east, z north, y up; Unity is y up, so x east and z north map straight on.
  Does block 0,0,0 fill 0 to 1 on each axis, and does a character at 0, 0, 0 stand on the GOLD or in it?
- **The distance**: the map as a coarse ground, 16 by 16 blocks a patch, coloured by its top block, hidden
  where real chunks are; at `world_size` 32 that's a million patches, so it wants tiles, or a coarser cut
  further out.
- **A camera to look with**: nobody moves until 0.0.2.

Accounts to log in with: `testuser123` / `Testpass123!` (Tester), and `testuser456` / `Testpass1!` (Chatter).

## What's waiting

- **The world to the client, Ensemble's half** (above): receiving the chunks, drawing them, drawing the
  distance.  Conductor's leftovers (TODO.md, "The chunks streamed"): a stamp on a chunk's pieces once
  blocks change, forgetting chunks nobody's near once players move, the GameClock loading around a player
  who isn't at 0,0,0, a faked address getting a player flooded (`design/conductor-networking.md`).
- **Soundcheck's rest** (TODO.md): Conductor's report up and debug clients (open), Windows, one package;
  and the certificate for every client (LONGTERM_TODO.md).
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
