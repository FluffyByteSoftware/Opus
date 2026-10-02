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
beside the UI Document component, owns both and starts on the login.  **The login turns the password into
a key** on SUBMIT, and Remember Me keeps the key, never the password (`design/client-security.md`).
**Conductor takes the key** where it took the password (protocol version 7, 2026-10-02).  Ensemble has no
networking yet.  **The
game's name is Forgotten Legends**; the project, its folders and code stay Opus.  Unity's Company Name is
FluffyByte and its Product Name Opus.Ensemble.  Every file the game keeps for a player goes in
`~/.config/unity3d/FluffyByte/Opus.Ensemble/` (`PlayerFiles.cs`).

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` is at `33c418e`, released 2026-10-02 (Jacob: "merge everything to main please"),
a fast-forward from `testing` carrying everything since the release of 2026-09-30 (`8bf9f70`).  `unstable`
and `testing` are level with each other, and only this line ahead of `main`.  `main` moves when Jacob
says.

**Built and tested on Linux**: all of Conductor, `world_size` and the password's key (protocol version 7)
included.  **In Unity**: Copy Anims From FBX Pack, its CLASHES view included, the HUD's Phase 1, the login, and
the password's key with Remember Me, every check passed.  TEST_CHECKLIST.html is empty: the Windows check that
sat Parked there is GitHub issue #10 now (Jacob opened it, "Windows x86/x64 Untested").  **On Windows**:
Conductor builds and runs, START SERVER included, without a database; the world, the characters, character
select, the world save, the spawn and the password's key haven't been tried there.

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

At this hand-off (2026-10-01): **"next one is going to be brutal we'll start the net code for the
client"**, then, told Conductor doesn't take the key yet: **"we'll do netcode update on server next
then"**.  So Conductor's half of the password's key came first.  His to change.

At this hand-off (2026-10-02), Conductor's half done: **"wrap up here back to Ensemble"**.  So the client's
net code is what his earlier word put next.  His to change.

## Last session -- 2026-10-02, Conductor's half of the password's key

**Built and tested on Linux**, all six checks passed; nothing waits on a build.  `design/client-security.md`,
"Conductor's half" and "As built", has all of it.

- **Jacob's ask**: "we just made it so the client is going to pass a hashed password instead of plaintext
  (this is so it can save it on the client)".  Built as the plan in the design file said; his OK on the two
  crates: "Yes".
- **Protocol version 7**: the Login's fourth string is the password's key, 64 lowercase hex.
  `protocol.rs` (`LoginRequest.key`), PROTOCOL.md (a section "The password's key", the worked example's
  bytes redone, 100 long) and `test_client.py` changed together.
- **A Login that isn't a key** is Invalid Credentials at once, without a hash (`tcp.rs`, beside the name
  check), logged without what was sent.  Otherwise the key goes through Security's line as the password did.
- **Security makes the key** (`password_key()`, `looks_like_key()` in `tools/src/security.rs`), with the
  **`pbkdf2` 0.13 and `sha2` 0.11 crates** (OKed; `sha2` and `hmac` were already in the build through
  `postgres`).  Tested against the worked example, the full 600,000 rounds.
- **The account desk** (`desk.rs`) makes the key from what the admin types, on its own thread, and hashes
  that.  The Accounts tab didn't change.
- **`test_client.py`** makes the key with `hashlib` from the password on its command line; `--no-key`
  sends the password as typed, to see it refused.
- **Every account was deleted** by Jacob before the build; `testuser123` / `Testpass1!` was made again on
  the Accounts tab, and logs in.

## The session before -- 2026-10-01, security in the client (Ensemble)

Built and tested in Unity.  The key: PBKDF2 with HMAC-SHA256, the salt `Opus login v1:` plus the username
with A to Z made lowercase, 600,000 rounds (Jacob: "Make this the full 600,000"; 2852 ms in the Unity
editor), 32 bytes as 64 lowercase hex.  `Assets/Code/Security/PasswordKey.cs` makes it on a worker thread,
`Security/RememberedLogin.cs` keeps Remember Me's file (the server, the port, the username and the key),
`Hud/Widgets/LoginForm.cs` is SUBMIT's work.  **SUBMIT still doesn't log in**: there's no network client.
At its hand-off every player file went into one folder, `~/.config/unity3d/FluffyByte/Opus.Ensemble/`
(`Assets/Code/PlayerFiles.cs`).

## Where the next session starts

**Back to Ensemble** (Jacob: "wrap up here back to Ensemble").  By his earlier word that's the client's
net code (his to change): the login over TLS (PROTOCOL.md, version 7: the Login carries the key), the
client checking the server's certificate (TODO.md), and how far into UDP the first step goes.  **SUBMIT
connects while the key is made** (Jacob's pick, "in the background while the player moves forward in
login"): the TLS connection goes up as the key is worked out, and the Login goes the moment it's ready.
`test_client.py` is the working example of every byte the client sends and reads.

Accounts to log in with: `testuser123` / `Testpass1!` (made again after the switch).

## What's waiting

- **The Remember Me file is readable by other users on the same Linux machine.**  TODO.md.
- **Ensemble's client code**: the network client (Conductor takes the key now), character select's layout,
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
