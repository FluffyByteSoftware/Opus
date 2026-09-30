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
TCP that hands a player a ticket for UDP, the UDP side, character select (Protogame), a ledger of every
connection, and the access lists.
`conductor-lua-parser` (lib) runs the Lua scripts, locked down, and reads saved GameObjects back.
`conductor-primlib` (lib) is the game library, an ECS in memory, with the Living and Character templates.
`conductor-gameworld` (lib) is GameWorld, the ground.  `conductor-gameclock` (lib) is the GameClock, the game
loop: five checks of 50 ms to a 250 ms cycle, and it owns primlib's `World` and GameWorld's `Terrain`.
`conductor-wgui` (lib) is the web admin at `http://127.0.0.1:9996/Opus`.  `conductor-launcher` (bin) boots
the program and waits on the web admin's Server tab.  Ensemble is Unity 6000.6; its first project settings
are on `main` (Jacob's commit), the rest on his machine.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` was released from `testing` at this hand-off (`8bf9f70`, Jacob's say, a
fast-forward); `unstable` and `testing` carry this note on top.  `main` moves again when Jacob says.

**Built and tested on Linux**: everything, this session's code included.  The only check left on
TEST_CHECKLIST.html is the Parked Windows one.  **On Windows**: built and runs, START SERVER included,
without a database; the world, the characters and character select haven't been tried there.

## Jacob's map (2026-09-30)

**The 0.0.1 goal**: "get a player spawned in the world and able to chat."

His words: "We are going to work on marrying the network code to the game by finishing out character as a
template for hydrating from an account.  Then we will build the character selection (start of UDP
connection), then the log in to the world, and spawn character in world."  And the flow: "account logs in
(done) -> character selection -> selected character spawns in world at its last save loc (0,0,0 for
now)".  His to change.

1. **The character as a template**, hydrated from an account.  **Done.**
2. **Character selection**, at the start of the UDP connection.  **Done**: list, make, delete, reset home.
3. **Logging in to the world**, and the character spawned there, at its last saved spot (0,0,0 for now).

**Before step 3**, at the close of this session: "Next session we are going to clean things up and prepare
the game lib."

**Step 3 in his order** (2026-09-30, the session preparing the game library): the game library's half of
the spawn first (the GameClock's mailbox, loading, saving; no packets), then "building the network
infrastructure up then the chapter after that will be testing wtih a py script then we're on to building
the client".

## Last session -- 2026-09-30, the characters, the Characters tab, and character select

Jacob kept the session going through four pieces ("we're not even at 40% token use"), each planned, built
and checked on its own:

1. **The character functions in `conductor-accounts`** (`accounts/src/characters.rs`): `list()`,
   `list_all()`, `load()`, `create()`, `save()`, `delete()`, the name rule by hand, and the unplayable flag.
   Jacob's answers: **a new character takes the first empty slot**; **all three full is refused** ("we
   refuse to even allow them to create"); **only the player deletes, and only their own** ("Player can
   delete their character from their account that's it").  The unplayable flag (a save that won't load) is
   an Error on the bell and a flag in memory; the launcher's `stop_server()` forgets it, so the next START
   SERVER tries the save again.
2. **The Characters tab** under GAME MANAGEMENT: every player's character, look only, for `admin` and
   `user` both (Jacob's pick), "their name, their X,Y,Z, and which account", and the UUID.  Read through
   `GET /Opus/Content/characters` (Jacob's yes) when the tab opens and on REFRESH.  Editing characters and
   NPCs there is its own conversation (TODO.md, "A tab for the game's entities").
3. **Character select, step 2** (`design/conductor-networking.md`, "Character select and Protogame";
   PROTOCOL.md, version 5).  Jacob's packet names in `0x2_` over UDP: CharacterListRequest /
   CharacterListDelivery (a playable byte per character, "just add a bool in it"), CreateCharacter /
   CharacterCreateResult, DeleteCharacter (with the typed word; only DELETE deletes) /
   CharacterDeleteResult, CharacterRequestResetHome ("sends character back to 0, 0, 0"); and the general
   CommandAccepted / CommandRefused in `0x3_` ("This can be reused elsewhere").  **Protogame**, Jacob's
   word ("the character selection and character construction are proto game then become game objects after
   load"), is a thread in networking that answers the asks, so the UDP thread never waits on the database.
   **Every ask carries a u32 ask number**, and the book keeps the last answer, so a lost answer is sent
   again, never the ask done twice.  A new character is the Character template with the name in
   `ShortName` only ("short name here only"; the long name waits until they're in game), saved with
   primlib's new `Save::of_blueprint()`.  A reset home reads the save back through lua-parser, moves the
   character and saves it whole.  Networking now depends on primlib and lua-parser.
4. **The test client** does character select: `--create`, `--delete`, `--delete-word`, `--reset-home`.

**What fought back**: nothing in the builds (no warnings, first try each time).  Jacob read a refused
`--create testchar2` as an overwrite: the list printed after it still showed the old character, and the hex
didn't make the refusal stand out ("would have been more obvious if it was visual").  The same UUID before
and after showed nothing was touched.

**Tested by Jacob, all passed**: every test in every crate (67 in networking, 43 in primlib, 13 in accounts,
39 in the web admin), the Characters tab by eye, and all eight character select checks against Conductor.

## Where the next session starts

Jacob's words: "clean things up and prepare the game lib".  What that covers is his to say; ask him before
planning.  What's lying about that could count as cleaning up: the stale words in the code (TODO.md, "The
rest"), the Debug switch and moving routine log lines to Debug, `wgui_port` into `wgui.cfg`, the Unity files
on `main` (below).  What could count as preparing the game library for the spawn: how a character loaded
from its row goes into the GameClock's `World` (only the GameClock's thread touches it), when a character
is saved, what happens to a player's copy when they leave, and saving and loading primlib's copies
(`design/primlib.md`, "What's open").

## What's waiting

- **Spawning in the world**, step 3 of the map: picking a character at character select, loading it into
  the `World`, and what the client is sent.  When it comes, the Connections tab and the log stop calling a
  player at character select "in the world", and a reset home of a character in the world moves its copy.
  TODO.md, under Protogame.
- **Chat**, the rest of the 0.0.1 goal.  Not designed (TODO.md).
- **Editing characters and NPCs** from GAME MANAGEMENT: whether an edit goes to the row or the copy in the
  world, what can be edited, the routes.  TODO.md.
- **When a character is saved** (leaving the world, STOP SERVER, every so often).  `design/primlib.md`.
- **The world's part two**: saving changed chunks.  **Sending chunks to a client**, and how Ensemble gets
  `region.map`.  **Loading around players who move**.  `design/world.md`.
- **What goes in each of the GameClock's checks**: an input mailbox and packet, a brain, movement, the
  broadcast.  `design/gameclock.md`.
- **Saving primlib's copies** with their UUIDs and internal names, **the spawn system**, **primlib in Lua**
  (part 2).  `design/primlib.md`, TODO.md.
- **What a region does**, and blending biomes.  **A Tick evaluator tab**.  In TODO.md.
- **The blocked names list** (whether it checks character names too is open), **playtime metrics**,
  **moving `wgui_port` into `wgui.cfg`**, **the stale words in the code**, **where the test client
  lives**: in TODO.md.
- Archivist retrying on its own while disconnected; the Debug switch in `conductor_globals.cfg`; catching
  Ctrl-C.
- The server on Windows with a database.  Parked in TEST_CHECKLIST.html.
- **Soundcheck**, the patcher, and a certificate for every client.  LONGTERM_TODO.md.
- **Ensemble**, its own session.  Its first project settings are on `main`, including
  `Assembly-CSharp*.csproj` and `Opus.Ensemble.slnx`, which Unity rewrites on every open and usually stay
  out of git; and the nested `Ensemble/dev/Opus.Ensemble/.gitignore` has no header.  A look together before
  more of it goes in: what a Unity project commits, where the purchased art goes, and LFS.
- **A GDD**: Jacob is writing one with another chat.  What it settles comes in through him and goes into
  these docs.
