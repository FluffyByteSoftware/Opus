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
admin's Server tab.

Ensemble is Unity 6000.6: its project settings and our four folders under `Assets/` (`Editor/`, `Code/`,
`Scripts/`, `Data/`) are committed, the rest is on Jacob's machine, the purchased art in
`Assets/Purchased/`.  It has one editor tool, Tools > Opus > Copy Anims From FBX Pack, and two screens built
from layout files by our own builder: **the login** and the HUD (`design/ensemble-hud.md`).  ScreenRoot,
beside the UI Document component, owns both and starts on the login.  No networking yet.  **The
game's name is Forgotten Legends** (Unity's Product Name); the project, its folders and code stay Opus.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` is still at `8bf9f70`, released at the hand-off of 2026-09-30.  `unstable` and
`testing` are level with each other and carry everything since, this hand-off included.  `main` moves
when Jacob says.

**Built and tested on Linux**: all of Conductor, `world_size` included.  **In Unity**: Copy
Anims From FBX Pack, its CLASHES view included, the HUD's Phase 1 and the login, every check passed.
TEST_CHECKLIST.html is empty: the Windows check that sat Parked there is GitHub issue #10 now (Jacob opened
it, "Windows x86/x64 Untested").  **On Windows**: Conductor builds and runs, START SERVER included, without
a database; the world, the characters, character select, the world save and the spawn haven't been tried
there.

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

On the world (2026-10-01): "we will test a mountain out after we get the client up".

At the `world_size` hand-off (2026-10-01): **"wrap up and prepare to go back to the client"**.  Then, on
the client: **"we need to build our HUD up for login and char select"**, "just the login submit screen
first".

At the login's hand-off (2026-10-01): **"next conversation we start building security into the
client... then the conversation after that is the net code... then the next UI element, then stitching
the whole thing together"**.  His to change.

## Last session -- 2026-10-01, the login screen

**Built and tested in Unity**, every check passed (Jacob: "That was smooth!").  Nothing in it waits on a
build.  `design/ensemble-hud.md`, "The login, as written", has the details.

- **Eight widgets**, all `"screens": ["login"]`: a background (black for now, a picture later), a logo
  (a box that says LOGO), Server IP and Server Port side by side, filled with `10.0.0.84` and `9997`,
  Username, Password (dots), Remember Me (only ticks) and SUBMIT (presses down and says so in the
  Console; it doesn't log in).  The four text boxes share `LoginField.cs`.
- **The background is a widget that fills the screen** (Jacob's answer to "the screen's own, or a
  widget?").  The catalog's new `fillsScreen` (catalog version 2, `HUD_FORMATS.md`): the builder
  stretches it over the whole real screen, whatever shape, and ignores its anchor, offset and size.
- **`login_default.json`** at 1920 x 1080, a column in the middle; **`login.uss`** its look.
- **ScreenRoot replaced HudRoot**, moved with its `.meta`, so the component in Jacob's scene turned into
  ScreenRoot by itself and the HUD's slots kept their files.  Its slots: Login Layout, Login Style,
  **Login Text Color and Login Text Font** (Jacob's pick over variables in `login.uss`; they live in the
  scene, which isn't committed), Hud Layout, Hud Style.  Show Login, Show HUD and Reset HUD To Default
  on its ⋮ menu.  Each screen carries its own style sheet, so the HUD's and the login's never meet.
- **Remember Me, later**: Jacob, "when we write our hash in it will hash the password and I think we may
  rewrite the server to accept a hash instead of plaintext".  Told with it: a hash the server takes as
  the login is as good as the password to whoever copies the file.  TODO.md.
- **The `.meta` commit was run before the pull** the first time, and committed nothing: the new files
  weren't on the disk yet, and Unity hadn't made their `.meta`s.  Done in the right order after
  (`30a72d6`).  CLAUDE.md has the lesson.

## Where the next session starts

**Security in the client**, Jacob's next (his map, above).  Nothing about it is designed: ask him what
it covers before planning.  TODO.md, "Security in the client", lists what's already written down that
touches it (the password hashed on the client, the client checking the server's certificate, Soundcheck
further off).

## What's waiting

- **Ensemble's client code**: security in the client, the network client, character select's layout,
  stitching the login, character select and the world together, and the HUD's Phases 2 and 3 (the
  catalog's export, the web layout editor).  **Ensemble's project files in git** (Packages/, the .csproj
  files, LFS for scenes): TODO.md.
- **Chat**, the rest of the 0.0.1 goal.  Not designed (TODO.md).
- **What the client is sent after CharacterEnteredWorld**: the world around it (chunks, `region.map`),
  other players, movement.  `design/world.md`, `design/gameclock.md`.
- **Editing characters and NPCs** from GAME MANAGEMENT: whether an edit goes to the row or the copy in the
  world, what can be edited, the routes.  TODO.md.
- **The world's part two**: saving changed chunks, and then **a `world_size` change that keeps the
  digging**.  **A character saved outside a smaller world**.  **Loading around players who move**.  **An all-air
  chunk that costs nothing**, now that most of a player's 891 are air.  **A mountain**, once the client is
  up.  `design/world.md`, LONGTERM_TODO.md.
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
- The server on Windows with a database.  GitHub issue #10, "Windows x86/x64 Untested" (Jacob opened it).
- **Soundcheck**, the patcher, and a certificate for every client.  LONGTERM_TODO.md.
- **A GDD**: Jacob is writing one with another chat.  What it settles comes in through him and goes into
  these docs.
