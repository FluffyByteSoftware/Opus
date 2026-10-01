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
admin's Server tab.  Ensemble is Unity 6000.6; its first project settings are on `main` (Jacob's commit),
the rest on his machine.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` is still at `8bf9f70`, released at the hand-off of 2026-09-30.  `unstable` and
`testing` are level with each other and carry the last two sessions on top (the game library ready for the
spawn, and the spawn), Jacob's `Cargo.lock` commits and his merge included.  `main` moves when Jacob says.

**Built and tested on Linux**: everything, the last session's code included.  The only check left on
TEST_CHECKLIST.html is the Parked Windows one.  **On Windows**: built and runs, START SERVER included,
without a database; the world, the characters, character select, the world save and the spawn haven't been
tried there.

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
it with a real client".  At this hand-off: **"roll this session up and next one we start work on
Ensemble"**.  Chat, the other half of 0.0.1, isn't designed yet.

## Last session -- 2026-10-01, networking's half of the spawn

Jacob: "CLAUDE its TIME to BUILD".  The loop, as he OKed it: login, character select, the pick, the
character in the world at its last save, and out again with its player however the session ends, saved.
Planned, talked through, built and checked.  `design/conductor-networking.md`, "The spawn", has all of it;
TODO.md, "Picking a character to play", has his answers in the order they came.

- **Protocol version 6.**  `UserPressPlay` (`0x27`, the ask and the uuid) and `CharacterEnteredWorld`
  (`0x28`, the ask, the uuid, the name, x, y, z as f32s): Jacob's names.  A pick that can't be played gets
  a CommandRefused.  Kicked reason `6`, "character locked for a moment".
- **Protogame does the slow part** (`play()`, `bring_in()` in `protogame.rs`): the row and the save, an
  unplayable character turned away, the save laid over the Character template (a save that won't load
  marks it unplayable), then `conductor_gameclock::enter()`.  Only then is the character written on the
  player in the book; a player who left meanwhile has it taken straight back out.
- **Every way out of the book takes the character out**, through `remove_player_in()` and `with_book()`
  calling `conductor_gameclock::leave()`, and `clear()` on STOP SERVER.
- **No way back to character select from the world**: "you log out back to log in screen every time", and
  "Even if you camp out, you go back to login screen not char select."  Character select's asks are
  refused once the character is in.
- **The race, and Jacob's fix for it.**  A second login could read a character's row before the last
  session's save of it landed.  After a misunderstanding (I read his first "lockout" as a leaving lock; he
  meant a load lock), the answer was **the lock both ways** ("we lock it when it does that"): 1 second on
  loading, 1 second on leaving and as long after as its save is on its way; a pick of a locked character
  gets Kicked 6 and logs in again.  And his "do we have any way to force a save on the connection being
  kicked before the new one pops in?": the GameClock marks a leaving character **"saving"** until its save
  lands (`saving()`, `wait_until_saved()`, a Condvar), and the login that logged the other session out
  waits for it before its ticket goes out, **up to 5 seconds** (his number), past that Login Unavailable.
- **The Connections tab** has the character beside the account on the UDP list ("character select",
  greyed, until there is one), and the log says "at character select" on the Connect and "is in the world
  as Spawny" once the character is.
- **The test client**: `--play NAME`.  The checklist's account is now `testuser123` / `Testpass1!`, with
  `--host 10.0.0.84` (Jacob's `bind_address`).

**What fought back**: nothing in the code; it built with no warnings and every test passed first time (22 in
the GameClock, 75 in networking, 39 in the web admin).  What took three goes was understanding the lock,
which is why CLAUDE.md's "ask before building" earned its keep: a Kicked can mean "back to the login" or
"try again", and a lock can sit on loading or on leaving.

**Tested by Jacob, all passed**: the build and the tests; into the world and out; the Connections tab and
the Services tab's player count; character select only; logging the other session out (the wait for the
save, the lock's Kicked, then in on the next try); KICK and STOP SERVER taking the character out; a save
broken in DataGrip refused, on the bell and UNPLAYABLE.

## What Ensemble has to speak

PROTOCOL.md is the whole contract, written for somebody building a client who has never seen Conductor's
code: TLS 1.3 against the one certificate (`Content/certs/conductor.crt`, the copy the client keeps), the
Login over TCP, the Ticket, then UDP: Connect, KeepAlive once a second, character select with ask numbers
(resend after half a second, same number), UserPressPlay, CharacterEnteredWorld, Goodbye, and Kicked with
its six reasons (5 is ACCOUNT TERMINATED in the client's words; 6 logs straight back in).  Numbers are
little-endian, strings a u32 count then UTF-8, floats IEEE f32, which is what C#'s `BinaryWriter` writes.
`networking/test_client.py` is a working client in a few hundred lines of Python to read beside it.  After
CharacterEnteredWorld the server sends nothing yet: no chunks, no other players, no movement.

## Where the next session starts

Jacob: "next one we start work on Ensemble".  What that first step is, is his to say; ask before planning.
What's known (TODO.md and the list below): its project settings are committed, including
`Assembly-CSharp*.csproj` and `Opus.Ensemble.slnx`, which Unity rewrites on every open and usually stay out
of git, and the nested `Ensemble/dev/Opus.Ensemble/.gitignore` has no header.  Before more of it goes in, a
look together at what a Unity project commits, where the purchased art goes, and LFS.  CLAUDE.md's "Client
rules (Ensemble)" section is still a FILL IN.  Ensemble has no way to find `Content/` yet.  A new folder,
a new piece or a new name goes past Jacob first (CLAUDE.md).

## What's waiting

- **Ensemble**: above.
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
