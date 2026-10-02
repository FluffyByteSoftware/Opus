<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is eleven crates, each in a folder without the `conductor-` in front (`Conductor/dev/tools/`) while
the crate keeps it (`conductor-tools`, `conductor_tools::` in code).  `conductor-tools` (lib) holds DiskMan,
Scribe, Constellations, Fingerprinter, Security, Archivist, the notices, the clock, the thread list, the
services list and the server's switch (`server.rs`).  `conductor-accounts` (lib) is the one way in to the
accounts table and `player_characters`, and the account desk.  `conductor-monitor` (lib) looks at the
process and the machine once a second.  `conductor-networking` (lib) is the front door: a login over TLS on
TCP that hands a player a ticket for UDP, the UDP side, character select and the spawn (Protogame), what a
player types in the world (a table of commands, `/chat` and `/who`, with an anti-flood), a ledger of every
connection, and the access lists.
`conductor-player-commands` (lib, 2026-10-02, waiting on a build) is what a player types in the world:
the table of commands and the anti-flood, `/chat` and `/who`, out of networking and wired to it by the
launcher.
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
the world, and **the HUD comes up over the Unity scene** (2026-10-02): the placeholder health bar and minimap,
and **the chat window**, bottom-left, 700 x 300, 50% black, in Jacob's Retro font: what's typed goes to the
server as typed and is echoed in yellow, the chat, refusals and `/who`'s box come back in white, Spans are put
back together, and `/camp` (to the login) and `/camp desktop` (closes the game) are the way out.  **The game's
name is Forgotten Legends**; the project, its folders and code stay Opus.  Unity's Company Name is FluffyByte
and its Product Name Opus.Ensemble.  Every file the game keeps for a player goes in
`~/.config/unity3d/FluffyByte/Opus.Ensemble/` (`PlayerFiles.cs`).  Ensemble speaks protocol version 9.

**Conductor and the server are two things.**  The program (DiskMan, Scribe, Constellations, the web admin)
is up from the moment the launcher runs.  The server (Fingerprinter, Security, Archivist, the account desk,
Lua, GameWorld, the GameClock, the monitor, and networking last) only runs between START SERVER and STOP
SERVER on the web admin's Server tab.  **Networking opens only once the ground around 0,0,0 is in**.

**The branches**: `main` is at `7d85f1f`, released 2026-10-02 as **0.0.1** (a player in the world, chatting,
from Ensemble): Jacob's "merge everything into main", before the clean-up rather than after.  The tag and
the packages are his to make; `Documentation/HowTo/RELEASE.md` walks through it.  `unstable`, `testing` and
`main` are level at that commit.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor up to chat, `/who` and the anti-flood, and the commands' move
into `conductor-player-commands` (built, 93 tests pass, chat works through it; its one run check with the test
client is still in TEST_CHECKLIST.html).  **In Unity**: everything up to character select's PLAY passed; the
chat window passed every check (the look on 1440p, chat both ways with the test client, the anti-flood's
refusal, `/who`'s box in Retro, the 300 limit, `/camp`, and EverQuest's keys); the crate's run check passed
too.  Nothing waits on a build; TEST_CHECKLIST.html has one Parked check (Spans for real).  **On Windows**:
Conductor builds and runs, START SERVER included, without a database; nothing since the world has been tried
there (GitHub issue #10).

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

During the chat window (2026-10-02): **"we need to rip the commands out of networking and put them into their
own crate I think... conductor::player_commands then we'll probably also have admin_commands"**, and "make
that todo for next session that seems urgent to me before we get too deep in commands".  Then, the chat
window pushed: **"Okay let's go ahead and push those commands to their own crate please we got tokens."**
Done, the same session.

At this hand-off (2026-10-02, the chat window and the commands crate): **"Next conversation we're gonna do
major clean up of code and documentation"**, and, asked whether to release: **"not yet we're gonna do code
clean up next session then merge to main and release 0.0.1"**.  His to change.

## Last session -- 2026-10-02, the chat window (Ensemble) and conductor-player-commands

Two steps, each planned, OKed and pushed on its own.  `design/ensemble-hud.md` ("The chat window"),
`design/ensemble-networking.md` ("In the world: chat and /who") and `design/conductor-networking.md`
("Commands and the anti-flood", the last bullet) have the detail; TODO.md's chat entry has Jacob's answers.

- **The chat window, built, compiled and chatting.**  After PLAY the HUD comes up over the Unity scene ("the
  HUD as it is", "render the game scene for now"), and character select's "In the world as" line and LOG OUT
  are gone.  The window is Jacob's spec: lower left, a "Chat" header, the input field on the bottom row, his
  lines echoed as `>/chat hello` in yellow, the server's in white, the whole window 50% black.  350 x 200
  with 16 px text as asked, then on 1440p "holy shit we need to make the font bigger... and the window needs
  to be twice as wide!": 700 x 300, 24 px lines, a 28 px header ("much better!").  The font is a slot,
  Chat Font on ScreenRoot: **Retro** from his Font Nation pack (purchased, never committed; `fc-query` found
  it and Arcade to be the pack's two monospaced fonts, and Fatality isn't one).  `Session.SendLine()`,
  `GameConnection` reading ChatDelivery, WhoDelivery and Spans (2 seconds), `WhoBox.cs` drawing the box to
  the window's width in letters, `/camp` and `/camp desktop` caught on the client, `Translator.NumberToWords()`
  in the footer at last.
- **`conductor-player-commands`, built and tested** (lib, `Conductor/dev/player-commands/`).  `commands.rs`,
  `chat.rs` and `who.rs` moved whole; networking's `typed.rs` keeps `Asker`, `Outcome` and the slot the
  launcher fills with `conductor_player_commands::wire()` in `start_server()`, with the GameClock's two
  senders.  Networking never names the crate.  "Commands Unavailable" with nothing in the slot.  Admin
  commands, when they come: "its a permissions difference but the commands will otherwise be the same".
- **EverQuest's keys, built and tested** (after the first hand-off; Jacob tested EQ for it: "we're mimicking
  their behavior after all").  `GameFocus` (`Assets/Code/Hud/GameFocus.cs`, Jacob's "focus place holder for
  the game") is an invisible focusable element on the HUD that holds the keyboard whenever no widget does;
  Enter or `/` on it shifts the keys to the chat field (`/` already typed); Enter in the field sends the line
  and hands them back, and so do Escape and a click away.  Four rounds in Unity to get there: a plain
  `Blur()` lasts one key (Unity's runtime panel hands the focus back to the last widget on the next key),
  Enter off a text field comes as a NavigationSubmitEvent and not a key event, and one Enter is two events
  with the focus moving between them, so each direction has a frame guard (`tookKeysFrame`,
  `gaveKeysFrame`).  `GameFocus.Has`, `Taken` and `Lost` are for movement.  "The window I last used" with
  more than one window is in TODO.md.
- Jacob's `Cargo.lock`, `WhoBox.cs.meta` and `GameFocus.cs.meta` commits may still be on his machine at
  this hand-off: the next session fetches first.

## Where the next session starts

**"Major clean up of code and documentation"** (Jacob).  Nothing is planned for it yet; it's his to lay
out.  Things seen along the way that a clean-up could take: TODO.md's "stale words in the code" entry;
`design/conductor-networking.md` still describes networking as holding the commands in places (the "Chat"
and "/who" sections were patched, not rewritten); STATUS.md's "Where things stand" has grown long;
`design/ensemble-hud.md`'s "The chat window" grew by patches through the keys' four rounds.

Accounts to log in with: `testuser123` / `Testpass1!` (Tester), and `testuser456` / `Testpass1!` (Chatter).

## What's waiting

- **The 0.0.1 release**: `main` is at the release commit; the `v0.0.1` tag, the version numbers (Unity's
  `0.0.0.1`, the crates' `0.1.0`) and the two packages are still to do.  `Documentation/HowTo/RELEASE.md`.
- **The 0.0.1 code review's findings** (`CODE_REVIEW_0.0.1.md`, 2026-10-02): four bugs and seven risks
  worth fixing before the tag, then the inefficiencies and the stale words.  Nothing fixed yet; Jacob
  picks.  The clean-up he named for next can start from it.
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
