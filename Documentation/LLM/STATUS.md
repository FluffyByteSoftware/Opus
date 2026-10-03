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
**Everything speaks protocol version 14** as built and tested (session 9, the world's objects); **session 10
moved all four (Conductor, the test client, Ensemble, Soundcheck) to 15, movement, and none of it is built
yet**.
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

**Movement, round one** (session 10, `design/ensemble-world.md`, "Movement"; `design/gameclock.md`,
"Movement"; PROTOCOL.md, "Movement"): **written and pushed, not yet built**.  EverQuest's way: the client
walks its own character and says where it went (PlayerMoved), and the GameClock takes each move or pulls it
back to the last good spot (MoveCorrection, numbered).  Built against the rubber banding that sank Jacob's
earlier server.  Conductor and the test client only: Ensemble reads version 15 and walks nothing yet.  The
server's ground now follows each player.  `player.cfg` (`turn_degrees_per_second`, 450) and `game.cfg`'s
`movement_tolerance_blocks` (16) are new; the walk, 4 blocks a second, is fixed in code.  **Jacob's pick for
when watchers hear a move is (b), EQ's, at once from networking, not built**: round one passes moves on
once a cycle.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: **0.0.1 is released** (2026-10-02): the tag `0.0.1` is at `bc7009e`, with the two packages on
the GitHub Release.  **`main` was fast-forwarded to `testing` at the end of session 9** (2026-10-03, Jacob:
"merge this to main please"), at `887a6d5`, sessions 0 to 9 past the tag, with no tag of its own; `testing`
and `unstable` are level, session 10's commits past it (round one of movement, unbuilt).  `main` moves when
Jacob says.

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
0.46 to 0.51 s, all on the worker, 302 drawn.  TEST_CHECKLIST.html has session 10's seven checks for
movement's round one, none run, and one Parked: the launcher's own restart after a patch.  **On Windows**: Conductor builds and runs, START SERVER included, without a database;
nothing since the world has been tried there (GitHub issue #10), and neither Soundcheck nor the new Ensemble
has been built there at all.

**Jacob's settings worth knowing**: `world_size = 32`, `view_chunks = 8` ("keeping as 8", session 3; the
code's default stays 4), `world_save_seconds = 1800`, `map_cooldown_seconds = 5`, `bind_address =
10.0.0.84`, and from session 10 `movement_tolerance_blocks = 16`, `turn_degrees_per_second = 450`.  In Unity: GroundView on a GameObject with five material slots (grass in GOLD's for now), the
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
instead we are going to redesign our voxel world."**  (b)'s plan and its four questions are in the session's
chat, unanswered.  His to change.

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
  **Not built yet.**  What EQ does looked up in EQEmu's source.

## Last session -- session 10, 2026-10-03, movement, round one

Jacob, opening it: **"in this session we do the task I failed at last time I worked on this game server
engine... synchronizing movement across clients with Conductor being the central authority on where a unit
is at any given moment."**  Talked through in five rounds, every answer in `design/ensemble-world.md`
("Movement") in his words, then round one written.  **None of the code has been built**: the next session
starts by expecting compile fixes.

- **The answers**: last time "the client and server kept fighting about your position (you basically kept
  rubber banding)"; **EverQuest's way**, "the client like has local authority and the server kinda just
  periodically validates the client movement and pulls it backward"; **Project Zomboid's keys** (D turns
  the character to face screen-right, then walks it there), a fixed camera (first person maybe, TODO.md);
  **a player anchor** in the scene with the controller on it, the prefab "just a skin we make the shell
  wear"; walking follows the ground, one block steps up, falls, walls stop it, jumping later; **4 blocks a
  second**, fixed in code (speed modifiers to come, TODO.md); the ground loaded as players walk, all of it
  now; **a mesh collider a chunk**, capsules on characters, collider shapes in primlib; **a character
  standing still blocks on the server, a moving one only on screens**; the turn in **`player.cfg`**, the
  server's; **Warns** at 1 to 16 blocks past and a turn 90 degrees off, one a character a minute; the
  tolerance in `game.cfg`, 16; check-ins every half second, hard-coded in the client; the capsule stays 1
  by 2; the server holds only the ground it needs; **(b)**, moves passed on at once, EQ's way.
- **The rule turned round**: CLAUDE.md's "the client is just a dumb renderer" has one exception now, the
  player's own character.
- **Round one, written** (protocol version 15; PROTOCOL.md's "Movement", `design/gameclock.md`'s
  "Movement"): `gameclock/src/movement.rs` (the mailbox, the check against the time since the last good
  spot, numbered pull-backs that moves must say they had, the Warns, the quiet walker stopped) and
  `ground.rs` (2 chunks each way of each player, let go of every 4 seconds); `view.rs` sends velocities, the
  collider, the pull-backs and each player's column, and never a player's own moves; primlib's `Collider`
  and `Transform`'s velocity; `Terrain` remembers what's coming and what failed, reads a block anywhere;
  networking's PlayerMoved and MoveCorrection, the walk and turn at CharacterEnteredWorld, the chunks
  following the character with one more each way; `player.cfg`; the test client's `--walk`, `--jump`,
  `--spin`; Ensemble and Soundcheck read 15.
- **The input check**: Jacob asked what professional games do; once a cycle as built.  Then "What does EQ
  do?": read in EQEmu's source (the client's update is taken and passed on the moment it arrives; its cheat
  check averages speed over 2.5 seconds and only logs).  He picked (b), not built.
- A lint suppression slipped into the first draft of `movement.rs` and was taken out before the push.

## Where the next session starts

**First, round one's build**: Jacob runs `cargo build`, `cargo test`, `dotnet build` of Soundcheck, and the
checks under "Movement, round one" in TEST_CHECKLIST.html.  Expect compile fixes: nothing of session 10's
was compiled.

**Then (b), Jacob's pick**: "B but this might have to be dumped to a new conversation".  Networking passes
each PlayerMoved to the players near it the moment it arrives, before the GameClock judges it, and a
pull-back corrects the watchers too.  What's open is at the end of `design/ensemble-world.md`, "Movement":
who is near whom outside the GameClock, a watcher told a move twice, a pull-back to the watchers, the cost.
A plan, and his OK, before building.

**Still to come for movement** (`design/ensemble-world.md`, "Movement"): round two, Ensemble (the player
anchor, `CameraAnchor` renamed `PlayerAnchor` with its `.meta`, a PlayerController with Zomboid's turning and
Unity's CharacterController, a MeshCollider a chunk, capsules from the Hydrate's collider, sending moves every
half second and on a change, taking MoveCorrections, never its own place from ObjectsMoved or the roll call,
the chat field taking the keys while focused); round three, Ensemble pulling the chunks as it walks and
letting the far ones go.  Jacob also wants to talk about **fewer voxels a column, EQ Next's style**
(TODO.md).  The model draw (session 9's pick) is still waiting.

Accounts to log in with: `testuser123` / `Testpass123!` (Tester, Chatter, Poopy); `testuser` /
`Testpass123!` (Asdf), for a second player beside the first.

## What's waiting

- **The HUD's rest** (TODO.md, "A resizable chat window"): a lock/unlock icon on a widget's title bar
  in place of the flashing, Reset HUD To Default where a player can reach it, the pointers on Windows.
- **The world to the client's rest**: the distance from the simple overworld map, drawn.  Conductor's
  leftovers (TODO.md, "The chunks streamed"): a stamp on a chunk's pieces once blocks change, a faked
  address getting a player flooded (`design/conductor-networking.md`).  The GameClock's ground following
  the players, and letting go of what nobody's near, is written in session 10 (unbuilt); GameWorld's
  squeezed copies are still kept until STOP SERVER.
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
- **Movement** (0.0.2 on WAYPOINTS.md): round one built and checked, (b), Ensemble's rounds two and
  three (above); jumping, speed modifiers, first person (TODO.md).
- **The model draw** (session 9's pick): `design/ensemble-world.md`, "The player in the world".
- **Fewer voxels a column, EQ Next's style**: Jacob's to bring up.  TODO.md.
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
