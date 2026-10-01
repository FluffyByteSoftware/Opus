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
`Assets/Purchased/`.  It has one editor tool, Tools > Opus > Copy Anims From FBX Pack, and the HUD's Phase
1: widgets placed on the screen from a layout file (`design/ensemble-hud.md`).  No networking yet.  **The
game's name is Forgotten Legends** (Unity's Product Name); the project, its folders and code stay Opus.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` is still at `8bf9f70`, released at the hand-off of 2026-09-30.  `unstable` and
`testing` are level with each other and carry everything since, this session's docs included.  `main`
moves when Jacob says.

**Built and tested on Linux**: all of Conductor, `world_size` included.  **In Unity**: Copy
Anims From FBX Pack, its CLASHES view included, and the HUD's Phase 1, every check passed.
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

## Last session -- 2026-10-01, the login screen settled (no code)

The HUD's Phase 1 session carried on after the `world_size` one, so this is its tail.  **Nothing in this
session needs building**: it tidied up and settled the login screen's design, and its plan waits on Jacob's
OK.

- **Tidied**: `UIElementsSchema/` (UI Builder's schemas, which Unity writes beside `Assets/`) is gitignored
  and out of the repo; `Assets/Data/Ui/` (one empty UXML file) is gone.  The player's layout folder,
  `FluffyByte Studios/Opus.Ensemble/Unity/`, is "the folder we're stuck with for now".  **The game's name
  is Forgotten Legends** (CLAUDE.md, README.md); the project stays Opus.
- **The login screen** (`design/ensemble-hud.md`, "The login and character select screens"), Jacob's
  answers: built as a shipped layout by our own builder ("The one we built earlier"), on a 1920 x 1080
  reference; **a widget per piece** ("its more work up front but may make it more mutatable later"); SUBMIT
  only presses down for now ("we want to get a feel for the HUD"); effects not decided ("don't worry
  about it"); **HudRoot becomes ScreenRoot**; on the screen "just for now": Server IP and Server Port side
  by side, Username, Password, Remember Me, and SUBMIT.
- **The plan put to him** (not OK'd yet; the chat ran out): six widgets, `login_server_ip`,
  `login_server_port`, `login_username`, `login_password` (dots), `login_remember_me` and `login_submit`,
  each its label and its box together, `"screens": ["login"]`, the four text ones sharing one helper;
  `Assets/Data/Layouts/login_default.json` (`"screen": "login"`, 1920 x 1080, a column in the middle, IP
  and port side by side); `Assets/Scripts/Hud/ScreenRoot.cs` in place of `HudRoot.cs`, with slots for the
  login and HUD layouts and the stylesheets, starting on the login, and Show Login, Show HUD and Reset HUD
  To Default on its right-click menu; `LayoutLoader` loading any screen (only the HUD reads the player's
  file); `Assets/Data/Styles/login.uss`.  No networking, no switching on SUBMIT, no effects, Remember Me
  only ticks.  In Unity he'd swap HudRoot for ScreenRoot and drag the four files in.
- **Still to ask with it**: whether the boxes start filled (`9997`, Conductor's TCP port; `10.0.0.84`) or
  empty, and what Remember Me remembers (a guess: the IP, the port and the username, never the password).

**On Jacob's side**: the HUD's `.meta` files and the company name (FluffyByte Studios, set in Player
Settings) still aren't committed.  File > Save Project in Unity, then the `git add -A` of the `Assets` and
`ProjectSettings` folders (CLAUDE.md, Client rules), `git status --short`, commit, pull, push.

## The session before -- 2026-10-01, the world's size in game.cfg

`world_size` in `game.cfg`, 2 to 32, 16 to start, 1024 blocks a side per step; a change deletes the world
and makes a new one on the next START SERVER (`design/world.md`, "The shape").  At 32, all of Conductor
sat around 800 MB: "I have so much room to work with.  I think I'm really just CPU limited."  Every check
passed.

## Earlier -- 2026-10-01, the HUD's Phase 1

Not handed off at the time, so here.  Jacob's brief (`HUD_LAYOUT_SYSTEM.md`) answered in
`design/ensemble-hud.md`, the two file formats' contract in `HUD_FORMATS.md`.  Phase 1 is the HUD built
from a layout file: `Assets/Code/Hud/` (the widgets, the registry, the loader, the checker, the builder),
`Assets/Scripts/Hud/HudRoot.cs`, the default layout in `Assets/Data/Layouts/`, the stylesheet in
`Assets/Data/Styles/`.  The player's own layout lives in `FluffyByte Studios/Opus.Ensemble/Unity/` beside
Unity's per-player folder, not under the game's name.  Every screen is made with the tool, and only the
player's HUD is the player's to change.  Sizes are reference pixels, scaled to the real screen.  All its
checks passed.  Phases 2 and 3 are in TODO.md.

## Where the next session starts

**The login screen's plan, above, waits on Jacob's OK**, with the two questions (the boxes filled or empty,
what Remember Me remembers).  Put it back to him short, then build it.  Expect him to need the Unity steps
spelled out window by window (Hierarchy, Project, Inspector); "add HudRoot to it" didn't land the first
time.

## What's waiting

- **Ensemble's client code**: the login screen (above), then character select's layout, the network
  client, and the HUD's Phases 2 and 3 (the catalog's export, the web layout editor).  **Ensemble's project
  files in git** (Packages/, the .csproj files, LFS for scenes): TODO.md.
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
