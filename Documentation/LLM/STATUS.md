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
Everything speaks protocol version 10.  CLAUDE.md's "Client rules" and "Launcher rules" have the detail.

**The world to the client** (0.0.13) has its first step: GameWorld writes `simple_overworld.map`, the
world's rough shape for the client's distance, before the door opens (session 1; SIMPLE_OVERWORLD_MAP.md,
`design/world.md`).  Nothing sends it yet.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: **0.0.1 is released** (2026-10-02): the tag `0.0.1` is at `bc7009e`, with the two packages on
the GitHub Release.  `main` is two docs commits past the tag; `testing` and `unstable` are level with each
other, sessions 0 and 1 past `main`.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor as released, and sessions 0 and 1 since (375 tests pass),
Soundcheck through PLAY and the way back, and Ensemble through the start screen, dev mode and the launcher's
ticket.  **Built and tested 2026-10-02**: PUBLISH, the check at start, the patch (two files, and the game's
program itself fetched back runnable), the extra file, the web server down, a 404, a file missing from the web
folder, a bad hash, `--debug`, the farewell after a KICK, all against the real address.  **Hashing the 655 MB
build takes 1.3 s** at a start (193 files; the disk cache does most of that, a cold start will be slower).
TEST_CHECKLIST.html is down to two Parked checks (session 0's and 1's all passed): the launcher's own restart
after a patch (needs Soundcheck shipped beside the game) and Spans for real.  **On Windows**: Conductor builds
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
simple overworld map.  His to change.

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

## Last session -- session 1, 2026-10-03, the world to the client: the simple overworld map

Jacob opened with a web admin bug (after a login, the page "kicks you to a random web page"); nothing in
`page.html` or the server navigates anywhere, and a headless Chromium against a fake server (not Conductor)
stayed on `/Opus`.  He couldn't make it happen again, so it was let go.  If it comes back: the URL it lands
on, the browser, and whether a private window does it too.

Then the world.  "Players need to download the bulk map data and then stream in the chunk changes as they
walk through the world right?"  Talked through and written down as it settled, every answer in Jacob's
words in `design/world.md` ("The simple overworld map") and TODO.md ("The world's dump"):

- **The bulk is smoothed**, the surface's shape and nothing under it, so the secrets aren't in it; a seed
  was weighed and left out (it rebuilds the whole world).  16 by 16 blocks a patch, each its average height
  and its commonest top block, which colours the distance.  Not squeezed.
- **Redesigned once**: first the patcher was to ship it from the web folder (`download/map/`); then "the
  unity client is going to have to download this at play", **from Conductor over UDP**, in pieces, behind a
  loading bar, into `Application.persistentDataPath`, by the same means the chunks will stream.
- **PlayerReady** is the client saying it has the terrain, and the character isn't spawned until it comes.
- **Hidden things** are sent ahead and hidden by the client, with room to tighten later.
- **The terrain save and the character save are one save**, every `world_save_seconds`.
- **If the map can't be written, the door stays shut**; **a client that can't get it tells the player** to
  delete the local map file (or the client) and try again.  "The end user" was the player: the session took
  it for the admin first, so the Error on the bell says to delete the file (which stands, as the admin's
  half), and `with_overworld()`'s comment in `gameworld/src/lib.rs` quoted that answer as its reason.  The
  comment was corrected at the hand-off, on Jacob's ask (a comment only, **not built**; nothing to test).

Built: `gameworld/src/overworld.rs` (makes, writes, reads the map), `build.rs`'s `top()` (one column's
top, shared by the chunks and the map), `lib.rs` (`with_overworld()`, before the first chunk goes out),
`regionmap.rs`'s `block_bounds()`, and SIMPLE_OVERWORLD_MAP.md, the contract, with a worked example the
tests check and a C# reader sketch.  **Built and tested by Jacob**: every check passed, the door staying
shut on an unwritable file included; 1.17 s at `world_size` 16 (`--release`), and his world is 32, 16.8 MB
(a guess of 4 to 5 s to make, unmeasured).

## Where the next session starts

**Jacob's pick**: "next session we prepare Ensemble for receiveing this".  What's written: the file's layout
and a C# reader in SIMPLE_OVERWORLD_MAP.md; where it's kept (`Application.persistentDataPath`, which is
`PlayerFiles.PathOf()`'s folder, CLAUDE.md's Client rules); the loading bar at PLAY, PlayerReady, and the
player told to start over when the map can't be had (`design/world.md`).  **The packets aren't designed**:
how the map goes in pieces, how a lost piece is sent again, what PlayerReady carries, and the protocol
bump.  Whether Ensemble's side starts before, with or after them is the plan's first question for him.

Accounts to log in with: `testuser123` / `Testpass1!` (Tester), and `testuser456` / `Testpass1!` (Chatter).

## What's waiting

- **Soundcheck's rest** (TODO.md): Conductor's report up and debug clients
  (open), Windows, one package; and the certificate for every client (LONGTERM_TODO.md).
- **The simple overworld map's rest** (TODO.md, "The world's dump"): the packets, PlayerReady, Ensemble's
  half, writing it again at the world save.
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
