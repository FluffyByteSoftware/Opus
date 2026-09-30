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
accounts table (and soon `player_characters`), and the account desk.  `conductor-monitor` (lib) looks at the
process and the machine once a second.  `conductor-networking` (lib) is the front door: a login over TLS on
TCP that hands a player a ticket for UDP, the UDP side, a ledger of every connection, and the access lists.
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

**The branches**: `unstable` and `testing` are on this hand-off.  `main` is still at Jacob's Ensemble
project settings commit, behind both; it moves when Jacob says.

**Built and tested on Linux**: everything, this session's code included.  The only check left on
TEST_CHECKLIST.html is the Parked Windows one.  **On Windows**: built and runs, START SERVER included,
without a database; the world and this session's code haven't been tried there.

## Jacob's map (2026-09-30)

**The 0.0.1 goal**: "get a player spawned in the world and able to chat."

His words: "We are going to work on marrying the network code to the game by finishing out character as a
template for hydrating from an account.  Then we will build the character selection (start of UDP
connection), then the log in to the world, and spawn character in world."  And the flow: "account logs in
(done) -> character selection -> selected character spawns in world at its last save loc (0,0,0 for
now)".  His to change.

1. **The character as a template**, hydrated from an account.  **Part A done** (the GameObject side), and
   **Part B's table done**; the functions in `conductor-accounts` are what's left of this step.
2. **Character selection**, at the start of the UDP connection.  The player makes a character here.
3. **Logging in to the world**, and the character spawned there, at its last saved spot (0,0,0 for now).

The map before this one (the tick, the world's voxels, zones) is done.

## Last session -- 2026-09-30, the docs tidied, and the character

**The docs**, first: re-evaluated at Jacob's ask, the old session notes trimmed out of every file, the
skeletons checked against the code, and the README made a short front page saying where everything stands.
Stale comments found in the code on the way are listed in TODO.md.  `design/ecs-discussion.md` is gone.
CLAUDE.md's id-and-uuid rule became "no game data without an `id` and a `uuid`", with
`archivist_migrations` the one exception (Jacob's words).

**The character, designed with Jacob** (`design/primlib.md`, "The character and saving", has his words):

- **Living is a "micro template"**, taken in whole the way `inherit STD_LIVING;` did in the Discworld
  mudlib: `ShortName`, `LongName`, `Health`, `Endurance`, `Mana` ("all living objects will have to have a
  name.  Its a requirement").  **Character** is Living, a `Transform`, a capsule and `PlayerCharacter` (the
  account's and the row's `id`).  A new one starts with 10 of each pool.
- **A GameObject remembers the templates it came from**, so the game can ask `world.is(entity, "Living")`.
- **The save is the GameObject**, as Lua text in one column, with the name and last position as columns of
  their own (the mix).  **What's saved is picked per field**, the plain way: a `saved()` under each struct
  (Jacob wanted C#'s `[SavedField]`; a real attribute would have been a macro crate and two dependencies).
- **The table**: `player_characters` (`account_id` with `ON DELETE CASCADE`, `character_name`, three `REAL`
  positions, `save_lua`, `created_at`, `saved_at`), and three slots on `accounts` by `id` (migration `0002`,
  `ON DELETE SET NULL`).  **A name is 4 to 20 letters, only the first a capital, unique whatever the
  capital** ("Jacob is fine JaCob is not Mckay is fine but not McKay"); the long name will be the player's
  to capitalize in game later.

**What's built**: `Template::take_in()`, `gameobject.rs` (the two templates, `character_from_save()`,
`check_living()`), `PlayerCharacter`, the templates list per entity, `saved()` / `load()` on every
component, `save.rs` (`Save`, `Fields`, `Value`, `to_lua()`), and `read_save()` in lua-parser (which now
depends on primlib; `sandbox::evaluate()` runs a script and hands back what it returns).  The schema file
and migration `0002`; Archivist's schema test now looks for `DELETE FROM`, so `ON DELETE CASCADE` passes.

**What fought back**: nothing in the build (no warnings, first try).  A checklist message came from a tab
opened before a push, naming checks already taken out; Jacob's own words covered the one left.  A push was
turned away once by Jacob's `Cargo.lock` commit, merged in.  The name rule changed after the schema was
pushed; Jacob looked in DataGrip, the table wasn't made yet, so the schema file took it and no migration was
needed.

**Tested by Jacob, all passed**: the build, primlib's 41 tests, lua-parser's 16, everything else, START SERVER
green, the table and its indexes, the slots, and the migration list ending at 2.

## This session so far (2026-09-30)

The character functions are **built and tested** (the build with no warnings, 13 tests in the accounts
crate, START and STOP SERVER clean): `accounts/src/characters.rs` (list, load, create, save, delete, the name
rule, the unplayable flag), and the launcher's `stop_server()` forgets the unplayable flags.  Nothing calls
them until character select.  `design/conductor-accounts.md`, "Characters", has it.

Then **the Characters tab** under GAME MANAGEMENT, **built and tested** (every test passes, and Jacob looked:
"it looks correct"): every player's character, look
only, for `admin` and `user` both (name, UUID, x, y, z, account), through `GET /Opus/Content/characters`
(Jacob's yes) and `list_all()` in `characters.rs`.  Editing characters and NPCs there is next conversation
(Jacob's words), in TODO.md.

## Where the next session starts

The functions in `conductor-accounts` for `player_characters`, the last of step 1.  Read
`accounts/src/lib.rs`, `accounts/src/desk.rs` and `design/conductor-accounts.md` first, and bring Jacob a plan.
What they'd need to do, as far as it's settled: make a character (the row and its slot in one transaction,
the slot being one of the account's own), list an account's characters for character select (name, position
and `uuid`, never running Lua; the `CharacterSnapshot` in TODO.md), load one's `save_lua`, save one (the
columns kept in step with the save), delete one.  Everything through Archivist, so the game loop never
waits.  Settled at the start of the next session (TODO.md has Jacob's words): the first empty slot, all
three full is refused, only the player deletes, and the characters get a tab of their own.  And the corrupted character (Jacob, after the hand-off: "fail out the
character and send a notification to admin and mark this as a corrupted player character somehow"): an
Error, the character flagged unplayable, still listed at character select with a flag the client greys out
(and the server refuses).  The flag lives in memory for the run, so a restart tries again; no migration
(TODO.md, under Protogame).

## What's waiting

- **The functions in `conductor-accounts`**, above.
- **Character selection** and **spawning in the world**, steps 2 and 3 of the map: which messages protogame
  carries, the packets (protocol version 5), how the test client shows it.  TODO.md, under Protogame.
- **Chat**, the rest of the 0.0.1 goal.  Not designed (TODO.md).
- **When a character is saved** (leaving the world, STOP SERVER, every so often).  `design/primlib.md`.
- **The world's part two**: saving changed chunks.  **Sending chunks to a client**, and how Ensemble gets
  `region.map`.  **Loading around players who move**.  `design/world.md`.
- **What goes in each of the GameClock's checks**: an input mailbox and packet, a brain, movement, the
  broadcast.  `design/gameclock.md`.
- **Saving primlib's copies** with their UUIDs and internal names, **the spawn system**, **primlib in Lua**
  (part 2).  `design/primlib.md`, TODO.md.
- **What a region does**, and blending biomes.  **A Tick evaluator tab**.  In TODO.md.
- **The blocked names list**, **playtime metrics**, **moving `wgui_port` into `wgui.cfg`**, **the stale
  words in the code**, **where the test client lives**: in TODO.md.
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
