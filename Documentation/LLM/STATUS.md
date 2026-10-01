<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is ten crates, each in a folder without the `conductor-` in front (`Conductor/dev/tools/`) while
the crate keeps it (`conductor-tools`, `conductor_tools::` in code).  `conductor-tools` (lib) holds DiskMan,
Scribe, Constellations, Fingerprinter, Security, Archivist, the notices, the clock, the thread list, the
services list and the server's switch (`server.rs`).  `conductor-accounts` (lib) is the one way in to the
accounts table and `player_characters`, and the account desk.  `conductor-monitor` (lib) looks at the
process and the machine once a second.  `conductor-networking` (lib) is the front door: a login over TLS on
TCP that hands a player a ticket for UDP, the UDP side, character select and the spawn (Protogame), a
ledger of every connection, and the access lists.
`conductor-lua-parser` (lib) runs the Lua scripts, locked down, and reads saved GameObjects back.
`conductor-primlib` (lib) is the game library, an ECS in memory, with the Living and Character templates.
`conductor-gameworld` (lib) is GameWorld, the ground.  `conductor-gameclock` (lib) is the GameClock, the game
loop: five checks of 50 ms to a 250 ms cycle; it owns primlib's `World` and GameWorld's `Terrain`, takes
players' characters in and out through a mailbox, and saves the world.  `conductor-wgui` (lib) is the web
admin at `http://127.0.0.1:9996/Opus`.  `conductor-launcher` (bin) boots the program and waits on the web
admin's Server tab.  Ensemble is Unity 6000.6: its project settings and our three folders under `Assets/`
(`Editor/`, `Code/`, `Scripts/`) are committed, the rest is on Jacob's machine, the purchased art in
`Assets/Purchased/`.  So far it has one editor tool, Tools > Opus > Copy Anims From FBX Pack, and no game
code.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` is still at `8bf9f70`, released at the hand-off of 2026-09-30.  `unstable` and
`testing` are level with each other and carry everything since, this session included (and Jacob's commit of
Unity's `.meta` files and `ProjectSettings.asset`).  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor.  **Ensemble's editor tool** compiled and ran in Unity
(653 clips copied from `Male`, the Console clean), all but its last change: **the CLASHES view hasn't been
confirmed compiled yet**, so expect a fix there first if Unity complains.  Five of the plugin's six checks
passed and are out of TEST_CHECKLIST.html; the CLASHES one is left (Jacob's tab predated it).  **On Windows**: Conductor builds and
runs, START SERVER included, without a database; the world, the characters, character select, the world
save and the spawn haven't been tried there.

## Jacob's map (2026-09-30, and on)

**The 0.0.1 goal**: "get a player spawned in the world and able to chat."

His words: "We are going to work on marrying the network code to the game by finishing out character as a
template for hydrating from an account.  Then we will build the character selection (start of UDP
connection), then the log in to the world, and spawn character in world."  And the flow: "account logs in
(done) -> character selection -> selected character spawns in world at its last save loc (0,0,0 for
now)".  His to change.

1. **The character as a template**, hydrated from an account.  **Done.**
2. **Character selection**, at the start of the UDP connection.  **Done**: list, make, delete, reset home.
3. **Logging in to the world**, and the character spawned there, at its last saved spot.  **Done**
   (2026-10-01): the game library's half, then networking's.

His order after that (2026-10-01): "building the network infrastructure up then the chapter after that will
be testing wtih a py script then we're on to building the client", and "then we're ready to start testing
it with a real client".  Chat, the other half of 0.0.1, isn't designed yet.

On Ensemble (2026-10-01): "first things first" was the animation tool.  At this hand-off: **"we're going to
move on to setting up the client code (the underlying logic that's going to drive our dynamic UI
generation)"**.

## Last session -- 2026-10-01, Ensemble's first: Copy Anims From FBX Pack

Jacob: "it is time to start working on Opus::Ensemble!", then "first things first", the animation data: the
animation packs come as one FBX per animation (`RPG-Character@Unarmed-Attack-L1`, `HumanM@...`) with the
clip inside it, and he wanted a plugin to find them all under a folder and pull them out.

- **Tools > Opus > Copy Anims From FBX Pack** (`Assets/Editor/CopyAnimsFromFbxPack.cs`, an EditorWindow,
  plain IMGUI).  A source and a destination folder (a path box and BROWSE each; the first version's object
  fields read as asking for a GameObject).  FIND lists every clip in every model file under the source and
  its subfolders, all ticked, grouped by folder, Unity's `__preview__` clips left out.  A box per prefix
  (the file name up to and including the `@`): whatever's typed replaces it in every file that has it,
  blank drops it (Jacob's `HumanM@` -> `Male_Humanoid_`).  COPY writes the ticked ones as `.anim` under the
  destination in the same folders as the source's.
- **A copy, not a move**: a clip in an FBX is made fresh from it on every import and can't be taken out.
  Jacob: "Yes the copy is what I would do".  A second COPY overwrites in place (his pick), into the `.anim`
  already there, so its GUID stays and whatever uses it keeps it.
- Two rules of mine, not objected to: an FBX with several clips names each copy after its clip, and two
  ticked clips landing on the same file show in red and hold COPY.  Jacob hit that once with the whole
  `Assets/Purchased/` (about 1000 clips) and asked for **a CLASHES view** of only the red ones ("a bitch
  scrolling through 300+ to find it"); it didn't come back on `Male`.  TODO.md has it.
- **Our folders under `Assets/`**, Jacob's names: `Assets/Editor/` for the plugins, `Assets/Code/` for the
  plain C# ("more our raw C# stuff for net I imagine"), `Assets/Scripts/` for the scripts.  The root
  `.gitignore` keeps all of `Assets/` out but those three and their `.meta`s; the nested
  `Ensemble/dev/Opus.Ensemble/.gitignore` (a lone `Assets/`, which beat the root's exceptions) is gone.
  The copies can't go in the three (they're the packs' art) and the window refuses a destination there.
- CLAUDE.md's "Client rules (Ensemble)" isn't a FILL IN any more: the folders, where `.meta` files come
  from, Tools > Opus, Jacob runs Unity.

**What fought back**: git, not the code.  The `.meta` commit went round twice: the commit ran without the
`git add` before it, so it committed nothing, and the push said everything was up to date.  CLAUDE.md now
says the add comes first, by full path, every time.

## What Ensemble has to speak

The session before this one finished networking's half of the spawn (protocol version 6, UserPressPlay and
CharacterEnteredWorld, the character's lock both ways, no way back to character select from the world);
`design/conductor-networking.md`, "The spawn", has all of it, and it was all tested by Jacob.

PROTOCOL.md is the whole contract, written for somebody building a client who has never seen Conductor's
code: TLS 1.3 against the one certificate (`Content/certs/conductor.crt`, the copy the client keeps), the
Login over TCP, the Ticket, then UDP: Connect, KeepAlive once a second, character select with ask numbers
(resend after half a second, same number), UserPressPlay, CharacterEnteredWorld, Goodbye, and Kicked with
its six reasons (5 is ACCOUNT TERMINATED in the client's words; 6 logs straight back in).  Numbers are
little-endian, strings a u32 count then UTF-8, floats IEEE f32, which is what C#'s `BinaryWriter` writes.
`networking/test_client.py` is a working client in a few hundred lines of Python to read beside it.  After
CharacterEnteredWorld the server sends nothing yet: no chunks, no other players, no movement.

## Where the next session starts

Jacob: **"setting up the client code (the underlying logic that's going to drive our dynamic UI
generation)"**.  What that means is his to say; ask before planning.  "Dynamic UI generation" could be UI
built from code at run time (UI Toolkit or uGUI made by script) or screens driven by what the server sends,
and "the underlying logic" could be the screens' state (login, character select, the world) or the network
client under them; each reads differently in practice.  What's known: the plain C# goes in `Assets/Code/`,
scripts in `Assets/Scripts/` (CLAUDE.md); `Packages/` is ignored, so the first package the client needs
makes the manifest question real (TODO.md, "Ensemble's project files in git"); Conductor speaks TLS 1.3 only,
and whether Unity's TLS does 1.3 is unchecked, so the first login from Unity should be the handshake alone.
The CLASHES view is still to compile.

## What's waiting

- **Ensemble's client code**: above.  **Ensemble's project files in git** (Packages/, the .csproj files,
  LFS for scenes): TODO.md.
- **Chat**, the rest of the 0.0.1 goal.  Not designed (TODO.md).
- **What the client is sent after CharacterEnteredWorld**: the world around it (chunks, `region.map`),
  other players, movement.  `design/world.md`, `design/gameclock.md`.
- **Editing characters and NPCs** from GAME MANAGEMENT: whether an edit goes to the row or the copy in the
  world, what can be edited, the routes.  TODO.md.
- **The world's part two**: saving changed chunks.  **Loading around players who move**.  `design/world.md`.
- **What goes in each of the GameClock's checks**: an input packet, a brain, movement, the broadcast.
  `design/gameclock.md`.
- **Saving primlib's copies** with their UUIDs and internal names (and the snapshot's cost when they join
  the world save: 32.88 ms for 10,000 characters), **the spawn system** for NPCs, **primlib in Lua** (part
  2).  `design/primlib.md`, TODO.md.
- **What a region does**, and blending biomes.  **A Tick evaluator tab**.  In TODO.md.
- **The blocked names list** (whether it checks character names too is open), **playtime metrics**,
  **moving `wgui_port` into `wgui.cfg`**, **the stale words in the code**, **where the test client lives**:
  in TODO.md.
- Archivist retrying on its own while disconnected; the Debug switch in `conductor_globals.cfg`; catching
  Ctrl-C.
- The server on Windows with a database.  Parked in TEST_CHECKLIST.html.
- **Soundcheck**, the patcher, and a certificate for every client.  LONGTERM_TODO.md.
- **A GDD**: Jacob is writing one with another chat.  What it settles comes in through him and goes into
  these docs.
