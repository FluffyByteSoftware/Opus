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
TCP that hands a player a ticket for UDP, the UDP side, character select and the spawn (Protogame), what a
player types in the world (a table of commands, `/chat` and `/who`, with an anti-flood), a ledger of every
connection, and the access lists.
`conductor-lua-parser` (lib) runs the Lua scripts, locked down, and reads saved GameObjects back.
`conductor-primlib` (lib) is the game library, an ECS in memory, with the Living and Character templates.
`conductor-gameworld` (lib) is GameWorld, the ground.  `conductor-gameclock` (lib) is the GameClock, the game
loop: five checks of 50 ms to a 250 ms cycle; it owns primlib's `World` and GameWorld's `Terrain`, takes
players' characters in and out through a mailbox, sends the chat out and answers `/who list` from its
broadcast check, and saves the world.  `conductor-wgui` (lib) is the web
admin at `http://127.0.0.1:9996/Opus`.  `conductor-launcher` (bin) boots the program and waits on the web
admin's Server tab.

Ensemble is Unity 6000.6: its project settings and our four folders under `Assets/` (`Editor/`, `Code/`,
`Scripts/`, `Data/`) are committed, the rest is on Jacob's machine, the purchased art in
`Assets/Purchased/`.  It has one editor tool, Tools > Opus > Copy Anims From FBX Pack, and three screens built
from layout files by our own builder: **the login**, character select and the HUD
(`design/ensemble-hud.md`).  ScreenRoot, beside the UI Document component, owns them and starts on the
login.  **The login turns the password into a key** on SUBMIT, and Remember Me keeps the key, never the
password (`design/client-security.md`).  **Ensemble logs in** (2026-10-02, `design/ensemble-networking.md`):
over TLS 1.2 to the Ticket, then UDP, and on to **character select**, a third screen: the account's
characters, a click to pick one, PLAY, CREATE, DELETE and RESET HOME, and LOG OUT.  PLAY puts the character in
the world, and the screen says "In the world as <name>" with LOG OUT; there's no world on screen yet.  **The game's name is Forgotten Legends**; the project, its folders and
code stay Opus.  Unity's Company Name is FluffyByte and its Product Name Opus.Ensemble.  Every file the game
keeps for a player goes in `~/.config/unity3d/FluffyByte/Opus.Ensemble/` (`PlayerFiles.cs`).  Ensemble has
no chat box yet; it speaks protocol version 9 (the number only) and has `Translator.NumberToWords()` waiting
for `/who`'s footer.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` is at `90f2e6f`, released 2026-10-02 (character select's buttons).  Chat passed on
`testing` and Jacob kept it there ("Keep in testing for now").  `unstable` and `testing` are level at this
hand-off.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor up to chat, `world_size`, the password's key and TLS 1.2
included; `/chat` (protocol version 8), and `/who`, `/who list`, Spans (version 9), the command table and the
anti-flood, and the test client's Ctrl-C: every check passed, nothing waits on a build.  **In Unity**:
everything up to character select's PLAY passed, and `Translator.cs` compiles (its `.meta` committed by Jacob,
`f9f1c6a`, with `Content/cfg/game.cfg`).  The test client's two typing checks passed too (2026-10-02);
and two Parked (Spans for real, and the chat box drawing `/who`).  **On Windows**: Conductor builds and runs, START SERVER included, without a
database; nothing since the world has been tried there (GitHub issue #10).

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
it with a real client".  Chat, the other half of 0.0.1, is the server's side done (2026-10-02); the client's
box is still to come.

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

At the hand-off of Conductor's half of the key (2026-10-02): **"wrap up here back to Ensemble"**.  So the
client's net code came next.

At the hand-off of the client logging in to character select (2026-10-02): **"Hand off to
create/delete/select/play character next round"**.  Done.

At this hand-off (2026-10-02), character select done: **"actually next session we're gonna make it so when you
log in you get a chat box -- server doesn't support this yet so that will be thes ession after next"**.  So the
client's chat box first, then the server's side of chat.  His to change.

After chat passed (2026-10-02): **"We're barely 30% so we're gonna keep going"**, with a `/who`.  Asked
whether to release chat to `main`: **"Keep in testing for now"**.

On the milestones (2026-10-02, opening the server's side of chat): **"If we can get it where people can log
in and chat with each other... that's release 0.0.1 then movement is 0.0.12"**, and then, asked: movement
is **"0.0.0.12"**.  His to change.

At this hand-off (2026-10-02, after `/who` and the anti-flood): **"wrap our documents up and we'll go work on
Ensemble again"**, and then, every check passed: **"next one we're gonna integrate chat into the client"**.
His to change.

## Last session -- 2026-10-02, chat, /who and the anti-flood (Conductor)

Jacob, "rewinding a bit": the server's side of chat before the client's box.  Four pieces, each planned and
OKed in its own commits.  `design/conductor-networking.md` ("Chat", "/who", "Commands and the anti-flood")
and PROTOCOL.md have all of it; TODO.md has every answer in his words.

- **`/chat`, built and tested** (protocol version 8, all nine checks passed).  The client sends the line as
  typed in a PlayerCommand (`0x37`); `/chat <message>` ("since EverQuest set precedent"), plain ASCII,
  anything past 300 characters dropped; everybody in the world gets `[Chat] Jacob: Yo yo yo!`, the speaker
  too, one fixed channel ("like the way the old shit muds did it!"), in a ChatDelivery (`0x38`) from the
  GameClock's broadcast check, once a cycle ("every beat").  Networking hands the GameClock its sender as a
  plain function, since the GameClock can't depend on networking.  A line without a `/` is refused until
  saying things nearby exists.
- **`/who` and `/who list`, built and tested** (protocol version 9).  WhoDelivery (`0x39`): the names of
  the characters in the world, A to Z, and with `/who list` each one's block (`[Aldric] is currently at [0,
  0, 0]`), and the time as seconds since midnight UTC.  The client draws Jacob's old MUD's box ("]
  Forgotten Legends [", "There are seven legends currently online."), in the player's time zone and to its
  chat box's width.  `/who` is answered from the book; `/who list` goes through the GameClock for positions.
  **Spans** (`0x3A`, Jacob's "span packet"): an answer over 1200 bytes in pieces, each "X of Y", the
  client waiting 2 seconds.  **Ensemble**: `Translator.NumberToWords(int)`, British ("IN the honor of
  Discworld!"), and `Protocol.cs` at 9.
- **The command table and the anti-flood, built and tested.**  "I think we're doing this stupid.  We can
  just make it so there's anti flood prevention on the server for any chat commands right?", and "make a
  command interface... we could easily stuff new commands in".  `COMMANDS` in `commands.rs`: name, wait,
  `run()`, a file each in `commands/`.  After a command, the player waits its wait before the next: 500 ms
  by default ("two full game ticks"), `/who` 1 second, longer for anything heavy later.  Too soon gets
  "You can't do that again so soon.", and doesn't push the wait back.
- **test_client.py**: lines typed live in its terminal once in the world (Jacob: "write it so we can do it
  that way"), `--type` lines, the `/who` box drawn at 79 wide, Spans joined, `--type-gap` (1.1 s by
  default; 0 floods), keep-alives printed only when unanswered, and Ctrl-C caught anywhere with a Goodbye.  Ctrl-C "doing nothing" turned out to be
  Jacob's terminal; a SIGINT line added for it was taken out again.

## Where the next session starts

**Chat in Ensemble** (Jacob: "next one we're gonna integrate chat into the client").  Conductor's side of chat and `/who` is built and
tested, so the client's chat box has a server to talk to.  For it,
TODO.md's chat entry has what's settled: a line typed goes out as a PlayerCommand as typed; ChatDelivery's
lines are printed; the box stops taking keys at 300; WhoDelivery drawn as the box (monospaced, to the chat
box's width, `NumberToWords()` for the count, the time in the player's time zone); Spans put back together;
"You can't do that again so soon." shown like any refusal.  Still to ask: where "when you log in" puts the
box (in the world after PLAY, or the HUD with Phase 1's chat placeholder).

Accounts to log in with: `testuser123` / `Testpass1!` (Tester), and `testuser456` / `Testpass1!` (Chatter).

## What's waiting

- **Ensemble's chat box**, and drawing `/who` in it (TODO.md).  Then 0.0.1 can go to `main`, Jacob's call.
- **Saying things without a `/`**, nearby, once there are positions.  **Kicking a player who keeps
  flooding**, **`/help`**, **whether the web admin sees the chat**, **who may see positions**, **a Math class
  on the client**: TODO.md.
- **The Remember Me file is readable by other users on the same Linux machine.**  TODO.md.
- **Ensemble's client code**: the world on screen, and the HUD's Phases 2 and 3 (the catalog's export, the
  web layout editor).  **Ensemble's project files in git** (Packages/, LFS for scenes): TODO.md.
- **Movement** (0.0.0.12, Jacob's number), and what the client is sent after CharacterEnteredWorld: the
  world around it (chunks, `region.map`), other players.  `design/world.md`, `design/gameclock.md`.
- **Editing characters and NPCs** from GAME MANAGEMENT.  TODO.md.
- **The world's part two**: saving changed chunks, a `world_size` change that keeps the digging, a character
  saved outside a smaller world, loading around players who move, an all-air chunk that costs nothing, a
  mountain.  `design/world.md`, LONGTERM_TODO.md.
- **What goes in each of the GameClock's checks**: an input packet, a brain, movement, the positions in the
  broadcast.  `design/gameclock.md`.
- **Saving primlib's copies**, **the spawn system** for NPCs, **primlib in Lua** (part 2).
  `design/primlib.md`, TODO.md.
- **What a region does**, and blending biomes.  **A Tick evaluator tab**.  In TODO.md.
- **The blocked names list**, **playtime metrics**, **moving `wgui_port` into `wgui.cfg`**, **the stale words
  in the code**, **where the test client lives**: in TODO.md.
- Archivist retrying on its own while disconnected; the Debug switch in `conductor_globals.cfg`; catching
  Ctrl-C in Conductor.
- The server on Windows with a database.  GitHub issue #10.
- **Soundcheck**, the patcher, and a certificate for every client.  LONGTERM_TODO.md.
- **A GDD**: Jacob is writing one with another chat.  What it settles comes in through him and goes into
  these docs.
