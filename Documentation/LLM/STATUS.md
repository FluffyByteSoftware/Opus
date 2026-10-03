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
**Everything speaks protocol version 14** (session 9, the world's objects; 13 was session 8, `/who`).
CLAUDE.md's "Client rules" and "Launcher rules" have the detail.  **The built game in
`Ensemble/build/Linux/0.0.12/` still speaks 12**, so Soundcheck without `--debug` can't play it against
today's Conductor: a fresh build first, or the editor with `--debug`.

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

**The HUD is the player's** (session 7, `design/ensemble-hud.md`, "Moving, resizing and locking"): every
widget moves, chat resizes by its edges and corners, a right-click gives LOCK / UNLOCK and chat's FONT SIZE
(18 to 42), unlocked widgets flash, Jacob's two pointers from his purchased pack show where a drag can
start, and it's all kept in `<character>_hud_layout.json` (HUD_FORMATS.md's layout version 2).

**`/who` is a line a character, EverQuest's way** (session 8, `design/conductor-networking.md`, "/who"):
`Chatter is at [0, 0, 0] [16 days, 12 minutes online]`, the one in the world longest at the top, a blank
line, "There are 2 Legends online." and the time it ran.  `/who list` went into `/who`; every `/who` is
answered from the GameClock, which notes when each character came in.

**The player in the world** (session 9, `design/ensemble-world.md`, "The player in the world"; PROTOCOL.md,
"The world's objects"): Jacob's rule, **"The server will be the authority, always on where the object
actually is in the world.  The client is just a dumb renderer."**  Every object in a player's view (the
chunks' square, `view_chunks` each way) is sent whole as a **Hydrate** when it comes into view, then only
what changes (ObjectsMoved, ObjectsGone), with a **RollCall** once a second to mend lost packets; the
client asks about a number it doesn't know (ObjectAsk).  Each object has a number for the session;
CharacterEnteredWorld carries the player's own.  Ensemble draws each as its fallback shape (a character is
a capsule) under a **FluffyGameObject**, with an **Actor** on top for anything Living (its short name over
its head); `CameraAnchor` (the old stand-in Cube) keeps the Cinemachine camera on the player's own.
Velocity is in the packets and always 0 until movement; models come with the model draw.  **A character
saved inside the ground** is stood on top of its column at PLAY, and told so in its chat (`design/world.md`).

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: **0.0.1 is released** (2026-10-02): the tag `0.0.1` is at `bc7009e`, with the two packages on
the GitHub Release.  **`main` was fast-forwarded to `testing` at the end of session 9** (2026-10-03, Jacob:
"merge this to main please"), at `887a6d5`, sessions 0 to 9 past the tag, with no tag of its own; `testing`
and `unstable` are level, this one docs commit past it.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor as released, and sessions 0 to 8 since; Soundcheck through
PLAY and the way back, the check at start and the patch against the real web folder; Ensemble through the
start screen, dev mode, the launcher's ticket, the map at PLAY, the chunks and the ground on screen, the
HUD moved, resized and locked.
**Session 3, everything**: the build and the tests, the squeezed sizes, the test client pulling the whole
view, the refusal past the edge, the Services tab's line, Soundcheck and the editor at version 12.
**Measured**: the map takes 0.08 to 0.16 s on the LAN (16.8 MB at `world_size` 32), a guess of some 13 s over
the internet at a 50 ms ping; **a whole view of chunks at `view_chunks` 8 is 3,179 chunks, 88,746 bytes
squeezed, 0.04 s on the LAN**, the biggest chunk 693 bytes, every chunk one packet; squeezing is 13 to 14 us a
chunk; hashing the 655 MB build takes 1.3 s at Soundcheck's start.  **Session 5's meshing**: 437 chunks in
0.46 to 0.51 s, all on the worker, 302 drawn.  TEST_CHECKLIST.html has no open checks, only one
Parked: the launcher's own restart after a patch (a character on the GOLD passed in session 9, Spans for
real came off on Jacob's say).  **On Windows**: Conductor builds and runs, START SERVER included, without a database;
nothing since the world has been tried there (GitHub issue #10), and neither Soundcheck nor the new Ensemble
has been built there at all.

**Jacob's settings worth knowing**: `world_size = 32`, `view_chunks = 8` ("keeping as 8", session 3; the
code's default stays 4), `world_save_seconds = 1800`, `map_cooldown_seconds = 5`, `bind_address =
10.0.0.84`.  In Unity: GroundView on a GameObject with five material slots (grass in GOLD's for now), the
stand-in Cube with CharacterStandIn, a Cinemachine camera on it; ScreenRoot's Move Pointer and Grip Pointer
are Move_PremiumCursor and Hand2_PremiumCursor, each imported at Max Size 32 with its hotspot at 2, 2; the
scene isn't committed.

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

At session 6's hand-off (2026-10-03): **"wrap it up bro!  We're good"**, no pick for next.  Session 7 opened
on one he'd forgotten and then found: "Resizable chat window".  Done.

At session 7's hand-off (2026-10-03): **"next session we're gonna try to get a character loaded into the
world as a rep for the player"**.  Done in session 9 (Jacob opening it: "This will be necessary to get
movement set up next session").

At session 8's hand-off (2026-10-03): **"next session gonna be hard I think"**, no pick named.

In session 9 (2026-10-03), on the player in the world: **"we'll do the model draw next session"**: "I will
build an actor in the client and then we'll make a plug in to dump it into some sort of data that the
server can take in and use", tracked "with a living document and by clicking the prefab in Unity and
looking at its script for "FluffyGameObject"", and "we may want to store this animation data in the
database and not on a catalog file on disk?" (open).  His to change.

At session 9's end (2026-10-03), asked whether `main` catching up was a release: **"not yet... we're gonna
get movement then release"**.  So no tag or packages until movement is in.  His to change.

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
  (its last two Unity checks passed in session 7).
- **Session 7, 2026-10-03**: Jacob's forgotten pick, found: the HUD moved, resized (chat) and locked by the
  player, a right-click menu with chat's font size, his own two pointers, a layout file a character
  (layout version 2).  Built and tested, every check.
- **Session 8, 2026-10-03**: `/who` a line a character, EverQuest's way, protocol version 13: where each
  stands and its time online, the one in longest first, the count in digits and the time under it;
  `/who list` gone into `/who`, every `/who` through the GameClock; Ensemble's `WhoBox.cs` became
  `WhoLines.cs`.  Built and tested, every check.  `/who <character name>` is for later (TODO.md).
- **Session 9, 2026-10-03**: the player in the world, protocol version 14: the GameClock's view (Hydrate,
  ObjectsMoved, ObjectsGone, the roll call, ObjectAsk), object numbers, CharacterEnteredWorld with the
  player's own; Ensemble's `WorldObjects`, `WorldObjectsView`, `FluffyGameObject`, `Actor` (the name over
  the head), `CharacterStandIn` renamed `CameraAnchor`; a character saved inside the ground stood on top at
  PLAY, told in its chat.  A stale `accounts` test fixed.  Built and tested, every check (the view: 24.28 ms
  for 500 players all in sight, every one moved).

## Last session -- session 9, 2026-10-03, the player in the world

Session 7's pick, opened by Jacob: **"we're gonna be preparing both the server (Conductor) and Ensemble
(client) with representing the player in the world.  This will be necessary to get movement set up next
session.  The server will be the authority, always on where the object actually is in the world.  The
client is just a dumb renderer."**  Talked through in rounds (every answer in `design/ensemble-world.md`,
"The player in the world", in his words), then built in two rounds, every check passed:

- **The answers**: every character in the world within your view ("if they're within your visible
  range"), the chunks' square; **only what changed**, with a roll call once a second to mend a lost packet
  ("or we could consider redesigning the UDP service to have "reliable ordered" packets?": TODO.md); WASD,
  so a velocity; "full rotation"; the short name over the head; "define an Actor in the client as well"
  and "a simple datagram that the client can use to hydrate an actor with or an inanimate game object".
- **Models went round**: a path can't load a model in a built game, so "UUID matched", then "a backwards
  way": he builds an Actor prefab in Unity and a plugin dumps it into data the server takes in, tracked in
  a living document and by the prefab's FluffyGameObject; maybe in the database.  "We'll do the model draw
  next session."  The Hydrate carries a model's uuid as a string, empty for none, so the packet won't change.
- **Round one, Conductor** (protocol version 14): `gameclock/src/view.rs` (numbers from `enter()`, the view
  every cycle in the broadcast check, the roll call every 4th, `ask_about()`), `networking/src/view.rs`
  (the packets, sent through the slot networking fills itself), PROTOCOL.md's "The world's objects", the
  test client (`--miss-first-hydrate`, `--show-roll-calls`), Soundcheck at 14.  Timed: 24.28 ms for 500
  players in sight of each other, every one moved, half the broadcast's 50 ms (a cheaper shape is in
  TODO.md, "The view by column").
- **Round two, Ensemble**: `Code/World/WorldObjects.cs`; `Scripts/World/WorldObjectsView.cs`,
  `FluffyGameObject.cs`, `Actor.cs` (a TextMesh name, "they look fine actually :D"); `CameraAnchor.cs`
  (`CharacterStandIn.cs` renamed with its `.meta`).  In the scene: an empty "World Objects" with World
  Objects View on it.
- **Asdf in the GOLD**: saved at 0, 0, 0 before spawn points, it came in half inside the GOLD.  Jacob:
  "whenever a player is spawned into the world if the space they were in is now occupied with impassable
  voxel (IE: not air) it should move them on top of it".  Built (`design/world.md`, "A saved character
  inside the ground"): both blocks it fills checked at PLAY, stood on top of its column in the middle, the
  spawn point if that's too high, told in chat; GameWorld not answering sends it to the spawn point's last
  known place, saved, and PLAY refused (his "b").  "An unreachable spawn point... moves them to the next
  spawn point and deletes the invalid one": TODO.md, there being one spawn point.
- **Along the way**: an `accounts` test broken since session 6 (`create()` gained a position, the test
  didn't) found by this session's `cargo test` and fixed; "Spans for real" taken off Parked on his say.
- The session's clone opened with a local `unstable` off `origin/unstable` again (a shallow clone's
  doing); it was set to `origin/unstable`.

## Where the next session starts

**Jacob's pick: "we'll do the model draw next session".**  Read `design/ensemble-world.md` ("The player in
the world", the model bullets) first.  What he's said: he builds an Actor prefab in Unity from his purchased
art; a plugin (Tools > Opus) dumps it into data the server takes in, the server never assigning the model
its name ("the client does and we'll just start with 0000001 and work up"); he keeps track by a living
document the session keeps and by the prefab's FluffyGameObject in the Inspector; and "we may want to store
this animation data in the database and not on a catalog file on disk?".  **Open, to ask before building**:
the file or the database (a table has an `id` and a `uuid`, and the game names a model by its uuid, which
the Hydrate already carries as text); what the plugin writes and where Conductor reads it; how the client
finds a prefab by it at runtime (a list the plugin fills, in a slot or an asset); the living document's
name and place.  Purchased art never goes in our four folders.

Accounts to log in with: `testuser123` / `Testpass123!` (Tester, Chatter, Poopy); `testuser` /
`Testpass123!` (Asdf), for a second player beside the first.

## What's waiting

- **The HUD's rest** (TODO.md, "A resizable chat window"): a lock/unlock icon on a widget's title bar
  in place of the flashing, Reset HUD To Default where a player can reach it, the pointers on Windows.
- **The world to the client's rest**: the distance from the simple overworld map, drawn.  Conductor's
  leftovers (TODO.md, "The chunks streamed"): a stamp on a chunk's pieces once blocks change, forgetting
  chunks nobody's near once players move, the GameClock loading around a player who isn't at 0,0,0, a faked
  address getting a player flooded (`design/conductor-networking.md`).
- **Soundcheck's rest** (TODO.md): Conductor's report up and debug clients (open), Windows, one package;
  and the certificate for every client (LONGTERM_TODO.md).
- **Spawn points' rest** (TODO.md): which one a character gets once there are many; the height from a
  column changed since the last world save, once blocks change.
- **A web admin login that went to "a random web page"**, once (session 1's morning), not seen again; nothing in
  `page.html` or the server navigates anywhere.  If it comes back: the URL it lands on, the browser, a private
  window.
- **The simple overworld map's rest** (TODO.md, "The world's dump"): writing it again at the world save
  once blocks change, a timing over the internet, the stuck PLAY (LOG OUT is the way out).
- **The 0.0.1 code review's rest** (`CODE_REVIEW_0.0.1.md`): R8 onward, the inefficiencies and the stale
  words.
- **Saying things without a `/`**, **`/who <character name>`**, **kicking a player who keeps flooding**,
  **`/help`**, **whether the web admin sees the chat**, **who may see positions**, **a Math class on the
  client**: TODO.md.
- **The Remember Me file is readable by other users on the same Linux machine.**  TODO.md.
- **Ensemble**: the HUD's Phases 2 and 3; its project files in git (Packages/, LFS for scenes): TODO.md.
- **Movement** (0.0.2 on WAYPOINTS.md): the input packet, where velocity lives on the server, the
  client smoothing between moves; characters standing in each other.
- **The world's objects' rest** (TODO.md): reliable ordered UDP, the view by column, an unreachable
  spawn point.
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
