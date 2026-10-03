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
  TODO.md, the doubles merged, Soundcheck's first design told as history; the stale words in the code (a
  build to run); CLAUDE.md cut to the rules and a "where to look" map; this rolling day.

## Last session -- session 0, 2026-10-03, the docs clean-up, pass one

Jacob: "I'd like to go over our documentation and prune any out of date information or repeat information
on the first pass.  Let's also see if theres any directions you can rephrase in fewer words."  What went:
facts the code had overtaken (Ensemble's certificate copy, "four crates", "ten members", the GameClock's
senders handed over "at networking's start", the design shapes Soundcheck went through written up as if
live), TODO.md's built-and-tested history (the design files have it), and the doubles (CLAUDE.md's two
player-commands bullets, its copy of the web admin's tabs, STATUS's crate list).  His answers after: trim
CLAUDE.md down to the rules and pointers at the documents, so a session reads the one for the concern in
front of it (done: a "Where to look" table, and each piece's detail left to its design file); STATUS.md
keeps the last 24 hours of sessions as one-line summaries (done, above); the pass-two candidates
(`ensemble-hud.md`'s login sections, networking's "Chat" and "/who", the launcher rules, REPORT.html) are
left alone; the stale words in the code go now (done: comments, log lines and the Server tab's note, no
behaviour changed; three checks in TEST_CHECKLIST.html, **not built by Jacob yet**).  At the hand-off: the
session count starts at 0 here; CLAUDE.md's depth is right ("we don't need to trim that much... I just mostly
wanted to reduce token overhead with obsolete instructions and drift"); and his pick for next, below.

## The session before -- 2026-10-02, the web folder: PUBLISH, the check at start, the patch

Kept because the next code session starts from it.  Two rounds in one chat, built and tested the same
evening.  The first (two manifests from a web address, the
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

**Jacob's pick** (2026-10-03): "figure out how to serve the world up to the client", waypoint 0.0.13
(WAYPOINTS.md).  It starts with a plan,
talked through here with the docs at hand.  What's written so far: `design/world.md` ("Where it stands":
part two, and "Saving"); LONGTERM_TODO.md, "The world" ("Sending chunks to a client: only the ones near it,
since the server decides what each client sees.  A protocol change.  And how Ensemble gets `region.map`";
"Loading around players who move"); TODO.md, "Soundcheck" ("The world's dump", Jacob's "broad stroke" world
the client carries, shipped through the manifest, and "The world's files on the client",
`Assets/StreamingAssets/World/`); REGION_MAP.md (the C# reader for Ensemble); `design/gameclock.md`'s open
list (the positions in the broadcast, only what each player may see).  Nothing of it is designed past
those lines: what a chunk packet is, how many go and when, what the client does with them and with the dump,
and whether movement comes with it, are his to settle.

The patcher's polish is still in TODO.md under "Soundcheck", for whenever he wants it.  **The stale-words
build from session 0 hasn't been run**: `cargo build` and `cargo test` first, and the three checks in
TEST_CHECKLIST.html.

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
