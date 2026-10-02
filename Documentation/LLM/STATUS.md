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

**The branches**: `main` is at `testing`'s tip, released 2026-10-02 as **0.0.1** (a player in the world,
chatting, from Ensemble; the review's eleven fixes and PleaseWait built, tested and checked).  The `v0.0.1`
tag and the two packages are Jacob's to make; `Documentation/HowTo/RELEASE.md` walks through it.
`unstable`, `testing` and `main` are level.  `main` moves when Jacob says.

**Built and tested on Linux**: all of Conductor up to chat, `/who` and the anti-flood, and the commands' move
into `conductor-player-commands` (built, 93 tests pass, chat works through it; its one run check with the test
client is still in TEST_CHECKLIST.html).  **In Unity**: everything up to character select's PLAY passed; the
chat window passed every check (the look on 1440p, chat both ways with the test client, the anti-flood's
refusal, `/who`'s box in Retro, the 300 limit, `/camp`, and EverQuest's keys); the crate's run check passed
too.  **The 0.0.1 review's four bug fixes and seven risk fixes are built and tested** (2026-10-02, 365 tests); their run checks are in TEST_CHECKLIST.html, with one Parked check (Spans for real).  **On Windows**:
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

## Last session -- 2026-10-02, release prep: the review, the release docs, the four bugs

Jacob opened it with "we are ready to prepare for release 0.0.1" and a review of all of Conductor.

- **The review** (`CODE_REVIEW_0.0.1.md`): every `.rs` file read against CLAUDE.md's rules, four crate
  groups at a time, each claim checked against the code.  Four bugs, twenty-one risks (seven worth fixing
  before the tag), nine inefficiencies, a page of stale words, and a "checked and fine" list so nothing is
  re-investigated.  TODO.md's entry has the short version.
- **`main` moved to `7d85f1f`** on Jacob's "merge everything into main": a fast-forward from `testing`, 58
  commits.  (The session's clone was shallow and made the branches look unrelated; `git fetch --unshallow`
  first, before trusting a count.)  `testing` has moved on since (the docs and the fixes below), and `main`
  stays where it is until the fixes are built and tested: "Wait till we fix bugs and validate everything
  works."
- **The release docs**: `Documentation/HowTo/RELEASE.md` (testing to main, the version numbers, Conductor's
  tar and Ensemble's zip, the GitHub Release, by hand or `gh`) and
  `Documentation/HowTo/INSTALLATION_INSTRUCTIONS.md` (installing the two packages: Postgres, the key, the
  three settings to change, accounts, the client), linked from README.md's "Running it".
- **The version is 0.0.1** everywhere (Jacob: "we changing to version 0.0.1"): every crate's `Cargo.toml`
  and its `Cargo.lock` line, and Unity's `bundleVersion` (`0.0.1`, which `client_versions` already lists).
  `ProjectSettings.asset` was edited by hand, so Jacob pulls before Unity gets focus, or Unity's own
  rewrite of it wins.
- **The four bugs fixed, written and NOT BUILT**: the next session starts by expecting compile fixes.
  - `sessions.rs`: `issue_in` refuses a ticket over a player in the world (`Issued::Playing`) under the one
    lock, with a test; `tcp.rs`'s `talk()` goes round at most `ISSUE_TRIES` (3) times, asking the client
    about the other session once (`ask_about_the_other()`) and kicking it each time it's back
    (`log_the_other_out()`), then Login Unavailable.
  - `regionmap.rs` and `heights.rs`: the grid's size is `checked_mul` on the numbers off the disk, a test
    each with width and depth at 65,535.
  - `wgui/accounts.rs`: a delete the database is late with still takes the player out.
- **The four bugs built and tested by Jacob**: `cargo build` clean, 361 tests pass, the three new ones among
  them.  The run checks (the double login, the account delete, the corrupt map) are still in
  TEST_CHECKLIST.html.
- **The seven risks fixed, built and tested** (Jacob's "Yes" to the plan; `cargo build` clean, 365 tests
  pass), **and every run check passed** (the four bugs' and the seven risks', eight checks, 2026-10-02: the
  blank passwords for `user`, the 3-second handshake, the fifth connection closed, the slow save off the
  bell, the double login, the account delete, the corrupt map).  The first build caught a wrong line in the
  review: the web admin's two passwords were
  `Kind::Text`, not `Secret`, so they're a new `Kind::Password` (a Secret that can't be empty).
  - R1 (`wgui/src/json.rs`, `lib.rs`): `json::settings()` takes the role, and a Secret goes out as `""` to
    `user`; a test each side.
  - R2 (`tcp.rs`, `ledger.rs`): `HANDSHAKE_WAIT` (3 s, inside the login deadline) on the handshake alone,
    and `MOST_OPEN_PER_ADDRESS` (4) in the acceptor, a fifth closed at the door as `End::TooManyFromOne`
    (new, with its words; the ledger's words test now names every ending).
  - R3 (`udp.rs`): the refused Connect is Debug.  R4 (`tcp.rs`, `udp.rs`): a failing accept or receive is
    one Warn, a Debug each time after, and said again only after a success.
  - R5 (`diskman/cache.rs`): `forget_files()` keeps an entry with something waiting (the disk's stamp and a
    clean copy forgotten, a dirty one kept whole), with a test.
  - R6 (`notices.rs`): `MOST_OPEN` 1,000; past it the oldest go and the oldest left is rewritten to say how
    many; `publish_in()` so the test has a list of its own.
  - R7 (`archivist/status.rs`): the slow job is Debug, the label only made when slow.
  - `design/conductor-networking.md`, `conductor-wgui.md` and `conductor-tools.md` say so.
- The rest of the review (R8 on, the inefficiencies, the stale words) is the clean-up's list; nothing else
  in the code changed.

## Where the next session starts

**The kick bug Jacob found
while testing** (2026-10-02): a second login that logs the other out and presses PLAY inside the
character's one-second lock gets Kicked, reason 6, back to the login.  **Built from his "please add the
PleaseWait feature before we release", written and NOT BUILT, Conductor and Ensemble both**: protocol
version 10's PleaseWait (`0x3B`, the ask's number and words, general for any ask that will take a moment);
Protogame sends one for a locked pick and waits the lock out (`sessions::wait_for_loading_lock()`, up to
`LOCK_WAIT` of 5 s), then plays; past that the Kicked as before.  Ensemble's `GameConnection.Waiting()`,
`Session.AskWaiting()`, the words on character select's status line, `Protocol.Version` 10;
`test_client.py` prints it.  PROTOCOL.md, CLAUDE.md and both networking design docs say so.  **Built and
tested, both halves** (`cargo build` clean, 365 tests, a clean Unity Console, and the three run checks
passed: the double login from Ensemble shows the words and then the HUD, and the test client prints the
PleaseWait line and goes in).  **`main` is at `testing`'s tip, `b3ea5a8`, the 0.0.1 release**; the `v0.0.1`
tag and the two packages are Jacob's (RELEASE.md, steps 3 to 5).  The "major clean up" he named before is
the rest of the review's list.

Accounts to log in with: `testuser123` / `Testpass1!` (Tester), and `testuser456` / `Testpass1!` (Chatter).

## What's waiting

- **The 0.0.1 release's tag and packages**: `main` is at the release commit and the version numbers are
  set; the `v0.0.1` tag, Conductor's tar and Ensemble's zip are Jacob's, by `Documentation/HowTo/RELEASE.md`
  steps 3 to 5.
- **The 0.0.1 code review's findings** (`CODE_REVIEW_0.0.1.md`, 2026-10-02): the four bugs are fixed and
  waiting on a build; the seven risks are next, on Jacob's OK; then the inefficiencies and the stale words.
  The clean-up he named for next can start from it.
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
