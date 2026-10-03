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
**Everything speaks protocol version 17**, built and tested (session 12): 15 was movement (session 10), 16
the block kinds (GOLD out, MASONED_STONE in), 17 the simple overworld map dropped (PLAY answered with a
GroundOffer).
CLAUDE.md's "Client rules" and "Launcher rules" have the detail.  **The built game in
`Ensemble/build/Linux/0.0.12/` still speaks 12**, so Soundcheck without `--debug` can't play it against
today's Conductor: a fresh build first, or the editor with `--debug`.

**The world to the client** (0.0.13 on WAYPOINTS.md): **Conductor's side is whole**.  The simple overworld
map sent at PLAY (sessions 1 and 2) is **dropped** (session 12): nothing is drawn past the view, so PLAY's
answer is a **GroundOffer** (where the character will stand and how far it sees) and PlayerReady its ask
number only (PROTOCOL.md, "The way into the world").  **The client pulls the chunks around its character** (session 3; PROTOCOL.md's "The chunks around the player"),
up to 64 at a time, each squeezed as runs, out of a cache GameWorld's thread fills; the offer at PLAY says
where the character will stand and how many chunks it sees.  **Ensemble pulls them** (session 4), nearest
first, **and draws them** (session 5, `design/ensemble-world.md`): a mesh a chunk, faces only against air,
a material a block kind from GroundView's slots, a stand-in box for the Cinemachine camera.  **PlayerReady
waits until the nearest 99 are drawn** (session 6), 10 s at most, so nobody comes in on ground that isn't on
screen.  **The distance from the simple overworld map isn't drawn yet.**  New characters and RESET HOME
stand on top of a **spawn point**'s highest block (session 6, `design/world.md`); with the GOLD gone that's
Omega's dirt at 0,0, under 0 in Jacob's world, so a character saved on the old GOLD at 0.5, 1, 0.5 comes in
standing over the ground.

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

**Movement, round one** (session 10, `design/ensemble-world.md`, "Movement"; `design/gameclock.md`,
"Movement"; PROTOCOL.md, "Movement"): **built and tested** (session 12, every check).  EverQuest's way: the client
walks its own character and says where it went (PlayerMoved), and the GameClock takes each move or pulls it
back to the last good spot (MoveCorrection, numbered).  Built against the rubber banding that sank Jacob's
earlier server.  Conductor and the test client only: Ensemble reads version 15 and walks nothing yet.  The
server's ground now follows each player.  `player.cfg` (`turn_degrees_per_second`, 450) and `game.cfg`'s
`movement_tolerance_blocks` (16) are new; the walk, 4 blocks a second, is fixed in code.  **Jacob's pick for
when watchers hear a move is (b), EQ's, at once from networking, not built**: round one passes moves on
once a cycle.  (b) was planned in session 11 and put down unanswered (the end of `design/ensemble-world.md`).

**Smooth voxels, stage 1, is settled and being built** (session 11, `design/smooth-voxels.md`): talked
through in a separate chat from a brief, its summary brought back by Jacob.  **Two steps built and tested in
session 12**: every voxel holds a density (a byte, 128 and over solid) beside its kind, the chunk file at
version 3, DIRT, STONE and BEDROCK terrain, WOOD and the new MASONED_STONE structure, GOLD gone; and the
simple overworld map dropped.  Surface nets is the mesher.  7 Days to Die's split: the terrain kinds
smooth (a density a voxel, a smooth mesher, Shader Graph fading one kind into the next), structures cubes;
1 m voxels; -32 to +319 kept; caves, catacombs and sewers from a 3D density; buildings voxel by voxel in
**Opus.Treble** (a Unity tool, named, not started) as `.fbm`s Conductor stamps in; a 45-degree slope on the
server; damage sent as the changed chunks again; nothing drawn past the view.  **A region, a biome and a
zone are three things now** (LONGTERM_TODO.md, "The world asleep and awake"), not yet in code.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: **0.0.1 is released** (2026-10-02): the tag `0.0.1` is at `bc7009e`, with the two packages on
the GitHub Release.  **`main` was fast-forwarded to `testing` at the end of session 9** (2026-10-03, Jacob:
"merge this to main please"), at `887a6d5`, sessions 0 to 9 past the tag, with no tag of its own; `testing`
and `unstable` are level, sessions 10 to 12 past it, all built and tested.  `main` moves when
Jacob says.

**Built and tested on Linux**: all of Conductor as released, and sessions 0 to 8 since; Soundcheck through
PLAY and the way back, the check at start and the patch against the real web folder; Ensemble through the
start screen, dev mode, the launcher's ticket, the map at PLAY, the chunks and the ground on screen, the
HUD moved, resized and locked.
**Session 3, everything**: the build and the tests, the squeezed sizes, the test client pulling the whole
view, the refusal past the edge, the Services tab's line, Soundcheck and the editor at version 12.
**Measured**: **a whole view of chunks at `view_chunks` 8 is 3,179 chunks, 88,746 bytes
squeezed, 0.04 s on the LAN**, the biggest chunk 693 bytes, every chunk one packet; squeezing is 13 to 14 us a
chunk; hashing the 655 MB build takes 1.3 s at Soundcheck's start.  **Session 5's meshing**: 437 chunks in
0.46 to 0.51 s, all on the worker, 302 drawn.  TEST_CHECKLIST.html has one check left, Parked: the launcher's own
restart after a patch.  **Conductor's memory** with the server up: about 810 MB before densities, 918 with
them, 905.8 with the map dropped (session 12).  **On Windows**: Conductor builds and runs, START SERVER included, without a database;
nothing since the world has been tried there (GitHub issue #10), and neither Soundcheck nor the new Ensemble
has been built there at all.

**Jacob's settings worth knowing**: `world_size = 32`, `view_chunks = 8` ("keeping as 8", session 3; the
code's default stays 4), `world_save_seconds = 1800`, `map_cooldown_seconds = 5`, `bind_address =
10.0.0.84`, and from session 10 `movement_tolerance_blocks = 16`, `turn_degrees_per_second = 450`; `map_cooldown_seconds` is gone (session 12).
In Unity: GroundView on a GameObject with five material slots (GOLD's slot became MASONED_STONE's in
session 12, empty; the grass that was in it went with it), the
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

Session 10 (2026-10-03) opened on movement instead of the model draw: **"in this session we do the task I
failed at last time I worked on this game server engine... synchronizing movement across clients with
Conductor being the central authority on where a unit is at any given moment."**  At its end, picking (b)
for when other players hear a move: **"B but this might have to be dumped to a new conversation"**.  And
for later: **"a couple things that I will need to discuss next session with you... about possibly trimming
down the number of voxels per column because we want the EQ Next style voxels not Minecraft really but
that's for next iteration"**.  The model draw is still waiting.  His to change.

Session 11 (2026-10-03) opened on (b), planned it, then turned to the world asleep and awake
(LONGTERM_TODO.md) and from there: **"we are not going to implement movement yet that will be next session.
instead we are going to redesign our voxel world."**  (b)'s plan and its four questions are written down at
the end of `design/ensemble-world.md`, unanswered.  The redesign was talked through in a separate chat, and
at the hand-off, with its summary in: **"Prepare a hand off to a new conversation with yourself that we will
begin implementation of this system."**  His to change.

Session 12 (2026-10-03) opened on it: **"We begin implementing smooth voxels, stage 1"**.  His answers: surface
nets; the simple overworld map "we are dropping it... we don't need it anymore"; "DIRT & STONE are terrain.
WOOD will be structure (i'll make it look like planks).  Drop gold.  Add in MASONED_STONE"; BEDROCK
"Probably terrain".  After step 1 his pick was "drop the overworld map"; then "wrap up this
conversation", and at the hand-off: **"next session we bring in movement on the client (Ensemble)"**.  His
to change.

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
- **Session 10, 2026-10-03**: movement talked through in five rounds (EverQuest's way, Zomboid's keys, a
  player anchor, colliders, 4 blocks a second, the ground following, Warns at 1 to 16 blocks and a turn 90
  degrees off, (b) for watchers); round one written, protocol version 15: the GameClock judges every
  PlayerMoved and pulls back with a MoveCorrection, `Collider` in primlib, the server's ground follows the
  players, `player.cfg`, the test client's `--walk`, `--jump`, `--spin`; Ensemble and Soundcheck read 15.
  Built and tested in session 12.  What EQ does looked up in EQEmu's source.
- **Session 11, 2026-10-03**: (b) planned (networking's own "whose client has it" list, the view's "last" as
  what the watchers were told) and put down unanswered; the world asleep and awake talked through (cold,
  warm and hot chunks, catching up, zone managers, wakers, the dragon; a region, a biome and a zone split);
  a brief written for a separate chat on smooth voxels, and its summary written in as stage 1
  (`design/smooth-voxels.md`); Opus.Treble named.  Docs only, no code.
- **Session 12, 2026-10-03**: movement round one built and every check passed; smooth voxels begun: step 1,
  a density a voxel (a byte, 128 solid), the chunk file version 3, terrain and structure kinds, GOLD gone
  and MASONED_STONE in (protocol 16); step 2, the simple overworld map dropped, PLAY answered with a
  GroundOffer (protocol 17), the cooldown gone.  Built and tested, every check; 905.8 MB.

## Last session -- session 12, 2026-10-03, smooth voxels steps 1 and 2, movement round one tested

Opened on smooth voxels, stage 1.  Session 10's movement, unbuilt till now, built clean first.  **Everything
in it is built and tested**: TEST_CHECKLIST.html's twelve checks passed, the Parked one left.

- **Stage 1's open questions answered** (`design/smooth-voxels.md`, "Settled in session 12"): surface nets;
  the simple overworld map dropped; a density is a byte, 0 to 255, 128 and over solid, every voxel holds
  one (air too: it says where the surface sits), and a kind is AIR exactly when its density is under 128;
  DIRT, STONE and BEDROCK terrain, WOOD ("planks") and the new MASONED_STONE ("gray bricks... a wall")
  structure, GOLD dropped, its number 4 never used again.
- **Step 1, what a voxel holds** (protocol 16): `Density` and `Block::is_terrain()` in `block.rs`; a
  density beside every kind in `Chunk`, the chunk file version 3 (version 2 still read), a file whose kind
  and density disagree turned away; the ground still built from the kinds, full or empty, so it looks as
  it did; no GOLD at 0,0,0, so 0,0 is Omega's dirt, under 0 in Jacob's world.  Squeezing still sends kinds
  only.  108 MB more measured (810 to 918).
- **Step 2, the simple overworld map dropped** (protocol 17): PLAY's answer a **GroundOffer** (`0x40`,
  where and how far), the map's packets (`0x41`, `0x42`) retired, PlayerReady its ask number only,
  `map_cooldown_seconds` gone ("its no longer a risk"); `overworld.rs` out of gameworld and networking,
  `sha2` out of networking, Ensemble's `MapDownload.cs` and `SimpleOverworldMap.cs` deleted (the session's
  own `git rm`, four files, went through), SIMPLE_OVERWORLD_MAP.md deleted.  905.8 MB after.
- **Movement round one** checked through: walks, the watcher, the pull-back on purpose, the ground
  following, the stop saved.  Gravity is the client's (EQ's way); the server only catches a character
  hanging in the air 2 s.
- **CLAUDE.md**: the questions remind Jacob to go through TEST_CHECKLIST.html while it has checks waiting.

## The session before -- session 11, 2026-10-03, (b) planned, the world asleep and awake, smooth voxels

Docs only.  (b) planned and put down unanswered (the end of `design/ensemble-world.md`, four questions); the
world asleep and awake talked through (LONGTERM_TODO.md, five questions open); smooth voxels' stage 1
settled in a separate chat and written in (`design/smooth-voxels.md`); Opus.Treble named.

## Where the next session starts

**Jacob's pick: movement on the client, Ensemble** ("next session we bring in movement on the client
(Ensemble)").  Everything is built and tested, so nothing waits on compile fixes; `unstable` and `testing`
are level.  Start by reading `design/ensemble-world.md`'s "Movement" (his five rounds of answers, in his
words), PROTOCOL.md's "Movement", `design/gameclock.md`'s "Movement" (how the server judges a move), and
`design/ensemble-networking.md` for the net code it plugs into.

**What's settled for Ensemble's half** (session 10, all his):
- **EverQuest's way**: the client walks its own character at once and says where it went (PlayerMoved:
  the move's number, the last pull-back had, position, rotation, velocity); the server takes it or pulls
  it back (MoveCorrection: the pull-back's number, position, rotation), and only a MoveCorrection moves the
  player's own character.  The server already does all of this (session 10, tested in session 12).
- **Project Zomboid's keys**: a key is a direction on the screen; the character turns to face it, at
  `player.cfg`'s 450 degrees a second, and walks faster the more it faces it, up to 4 blocks a second
  (both come in CharacterEnteredWorld; the walk is fixed in code, the turn is the server's setting).  The
  camera stays fixed (Cinemachine on `CameraAnchor`); first person maybe later.
- **A player anchor** in the scene with the controller on it: "the prefab is just a skin we make the shell
  wear".  The anchor is Jacob's to set up in the editor.
- **The ground**: walking follows it, a step up of one block is free, falling off edges, a wall two blocks
  high stops it.  Jumping later.  **Gravity is the client's**: the server only pulls back a character
  that hangs in the air 2 s without coming down a block.  (A character saved on the old GOLD comes in a
  block or more over Omega's ground at 0,0, and should just fall.)
- **Colliders**: a mesh collider a chunk (the cube mesh for now; smooth voxels' surface-nets mesh will
  replace it), capsules on characters, 1 by 2, from the Hydrate's collider.
- **Check-ins every half second** while walking, hard-coded, and a move whenever the input changes.
- **Everybody else** is still drawn where the server says, gliding along the velocity in their
  ObjectsMoved.
- Round two is all that: the anchor and its controller, the chunks' colliders, sending moves, taking
  pull-backs, others gliding.  **Round three** is Ensemble pulling new chunks as it walks and letting the
  far ones go (the server already lets a player have one chunk more each way than the view, following
  their character).

**To put to Jacob before building**: CharacterController (Unity's own, steps and slopes built in) or a
Rigidbody; which keys (WASD and the arrows?); how a pull-back looks on screen (a snap, or a quick slide);
whether rounds two and three are one session or two; the cost of a mesh collider a chunk, timed in the
Console line the meshing already prints.  **Separate and waiting**: **(b)**, others hearing a move the
moment it lands (its four questions at the end of `design/ensemble-world.md`): round two works without
it, watchers just hear moves up to a cycle late.

A new script is a `.meta` round: the pull, Unity's focus, then `git meta`.  Ensemble is tested in the editor
with Soundcheck's `--debug` until a fresh build.

**Smooth voxels, stage 1**, the rest waiting its turn (`design/smooth-voxels.md`): densities on the wire,
the world from a 3D density, Ensemble's surface-nets mesher, the server's move checks on the density.  The
mesher and movement both touch the chunks' colliders: whichever comes second builds on the first.

Accounts to log in with: `testuser123` / `Testpass123!` (Tester, Chatter, Poopy); `testuser` /
`Testpass123!` (Asdf), for a second player beside the first.

## What's waiting

- **The HUD's rest** (TODO.md, "A resizable chat window"): a lock/unlock icon on a widget's title bar
  in place of the flashing, Reset HUD To Default where a player can reach it, the pointers on Windows.
- **The world to the client's rest**: the distance is closed (nothing past the view, session 12).
  Conductor's leftovers (TODO.md, "The chunks streamed"): a stamp on a chunk's pieces once blocks change, a faked
  address getting a player flooded (`design/conductor-networking.md`).  The GameClock's ground following
  the players, and letting go of what nobody's near, is built (session 10, tested in session 12); GameWorld's
  squeezed copies are still kept until STOP SERVER.
- **Soundcheck's rest** (TODO.md): Conductor's report up and debug clients (open), Windows, one package;
  and the certificate for every client (LONGTERM_TODO.md).
- **Spawn points' rest** (TODO.md): which one a character gets once there are many; the height from a
  column changed since the last world save, once blocks change.
- **A web admin login that went to "a random web page"**, once (session 1's morning), not seen again; nothing in
  `page.html` or the server navigates anywhere.  If it comes back: the URL it lands on, the browser, a private
  window.
- **The 0.0.1 code review's rest** (`CODE_REVIEW_0.0.1.md`): R8 onward, the inefficiencies and the stale
  words.
- **Saying things without a `/`**, **`/who <character name>`**, **kicking a player who keeps flooding**,
  **`/help`**, **whether the web admin sees the chat**, **who may see positions**, **a Math class on the
  client**: TODO.md.
- **The Remember Me file is readable by other users on the same Linux machine.**  TODO.md.
- **Ensemble**: the HUD's Phases 2 and 3; its project files in git (Packages/, LFS for scenes): TODO.md.
- **Movement** (0.0.2 on WAYPOINTS.md): round one built and tested (session 12), (b) (planned in session
  11, its four questions open), Ensemble's rounds two and three; jumping, speed modifiers, first person
  (TODO.md).  **The server's own gravity** (NPCs, a player whose ground is blown away): asked in session
  12, unanswered, not in TODO.md yet.
- **Smooth voxels**, stage 1: steps 1 and 2 built (session 12), the rest above; **Opus.Treble** and the
  `.fbm` (LONGTERM_TODO.md).
- **The world asleep and awake**: cold, warm and hot chunks, zone managers, wakers, the dragon; a region,
  a biome and a zone split (LONGTERM_TODO.md, five questions open).
- **The model draw** (session 9's pick): `design/ensemble-world.md`, "The player in the world".
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
