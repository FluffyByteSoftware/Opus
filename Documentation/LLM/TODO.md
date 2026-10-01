<!--
File:       Opus/Documentation/LLM/TODO.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- TODO

The big features, the ones that are a run of sessions each, are in LONGTERM_TODO.md instead.

## Deferred

Things that wait on a piece that doesn't exist yet, or on Jacob wanting them.

### The game

- **Blocks 1 m a side, Minecraft's size** (Jacob, 2026-10-01: "revise conductor voxels so that they are
  more in line with the size of a Minecraft voxel").  Being settled; `design/world.md` still says 50 cm
  until it's built.  The code counts in blocks only, so most of it is what the numbers mean.
  - **1 m, cubes**: "it will be simpler to start there for now and then we may make different non cubed
    voxels".
  - **Chunks stay 32 a side**, so 32 m.
  - **The world stays 8 km by 8 km** ("I thought we bout 8k x 8k?"): 8192 blocks a side, -4096 to 4095,
    256 by 256 chunks.
  - **Minecraft's height**: "we're gonna squeeze more memory and go Minecraft height and depth values for
    now", with the depth Jacob's own: **-31 is BEDROCK**, "can dig to -30 and stand on top of -31".  Read
    as: build up to +319, Minecraft's top.
  - **Omega's hills stay ±5** for this step: "we will test a mountain out after we get the client up".
  - **A player is 2 blocks tall, 2 m** (Jacob: "yes").
  - **Still to recheck**: `view_chunks` 4 is now 128 m.

- **Chat** (Jacob, 2026-09-30: the 0.0.1 goal is "get a player spawned in the world and able to chat").
  Nothing designed: who hears whom (everybody, or those nearby), what the packets are (a protocol bump),
  whether the web admin sees it.
- **Protogame**: the game-adjacent piece between a logged-in player and the world (Jacob's word).  Built:
  character select and the spawn (`networking/src/protogame.rs`).  The history of how it was settled is
  kept below.  Settled for its database side, the first step:
  - **`player_characters`**, a new table (its own schema file, `id` and `uuid` like every table).  Each
    row has its account's `id` (`account_id`, `ON DELETE CASCADE`, so deleting an account wipes its
    characters).
  - **Three slots on the account**: `character_slot_1` to `character_slot_3` on `accounts`, each the `id`
    of a `player_characters` row or empty (`ON DELETE SET NULL`).  `accounts.sql` is frozen, so it's a
    migration (`0002_...`).  The link is in two places, so making or deleting a character is two writes
    in one transaction.
  - **`conductor-accounts` writes the SQL for both tables**; protogame and the game library call its
    functions.
  - **`CharacterSnapshot`** lives in `conductor-accounts` beside `Account`: the surface of a character
    (its name, where it is) for whatever needs one outside the world (character select, the web admin).
    Read from the row, so it's the last save, and never written back.  The character itself lives in the
    game library, and only it writes its row.
  - Actor, Agent and Character (anything that acts; one the computer controls; one with a human on top)
    were named before primlib, and are likely sets of components now rather than types.

  - **The flow** (Jacob, 2026-09-30): "account logs in (done) -> character selection -> selected character
    spawns in world at its last save loc (0,0,0 for now)".  Character selection is the start of the UDP
    connection, so a character is picked after the UDP connect, not at the TLS login (protocol version 5).
  - **The character is a template** (primlib's), hydrated from the account: Jacob's first step of the
    three on his map in STATUS.md.  Part A, the GameObject side (Living, Character, `PlayerCharacter`,
    saving as Lua text), is written: `design/primlib.md`, "The character and saving", has Jacob's
    answers.  **Part B** is what's left of this step:
    - **`player_characters`**, as above, with the name and the last position as columns of their own and
      the rest of the save as Lua text in one column (the mix, Jacob's yes: character select lists names
      and the spawn reads the position without running anything).  Every UPDATE rewrites the row whole,
      so how often it's saved doesn't pick the shape; what SQL needs to see does.
    - **The account's slots point at the character by `id`**, per CLAUDE.md ("refer to CLAUDE on this").
    - **Built and in the database** (2026-09-30, every check passed): the schema file
      `player_characters.sql` and migration `0002_character_slots_on_accounts.sql`.  **A character's name
      is 4 to 20 letters, a to z, and only the first can be a capital** (Jacob: "Character names are 4 to
      20 alphabetical only, can start with a capital.  All names must be unique"), unique across the
      server whatever the capital: "Jacob is fine JaCob is not Mckay is fine but not McKay".  The position
      is three `REAL` columns, an f32 each like the Transform (Jacob's yes).
    - **The long name is the player's to capitalize, later** (Jacob: "there will be a way to set your
      _LONG_ to be capitalized how you want in game but when creating its this way").  The name rule
      above is for making a character; `LongName` is where "McKay" goes.
    - **The functions in `conductor-accounts`**: make, list, load and save a character, and delete one.
      Jacob's answers, 2026-09-30:
      - **A new character takes the first empty slot.**
      - **All three full, and making one is refused**: "we refuse to even allow them to create".  So the
        client isn't offered it, and the server turns it away too.
      - **Only the player deletes a character, and only one on their own account**: "Player can delete
        their character from their account that's it."  Not the admin.
      - **The unplayable flag is built with them** (Jacob: "Now"), though nothing calls it until the spawn
        loads a save.
      - **Built and tested** (2026-09-30): `accounts/src/characters.rs`.  `design/conductor-accounts.md`,
        "Characters", has what each does.
    - **A corrupted character** (Jacob, 2026-09-30): a save that won't load fails the whole character
      (built), "send a notification to admin and mark this as a corrupted player character somehow" (built).  Jacob's answers:
      - **The mark is "unplayable", and that's all**: "it should just flag a character as unplayable so
        the admin has to go and figure out if its salvageable or delete it".
      - **The notice is an Error.**
      - **The player still sees it at character select, and can't play it**: "the player would see the
        character name but unable to play it (it won't let you in the client) so maybe we send in the
        display characters packet to the client a boolean for this?"  So the list of characters carries
        an unplayable flag per character (protocol version 5, with character select).  The server turns
        it away too, since a changed client could ignore the flag.
      - **The flag lives in memory, for the run only** (Jacob's pick): "the admin will need ot restart the
        server to have it attempt again".  STOP SERVER and START SERVER forget it, and the next try loads
        the save again.  No column and no migration; the Error in that day's log keeps the why.
      - Built in `characters.rs` (`mark_unplayable()`, `is_unplayable()`, `forget_unplayable()`), and the
        flag is in `CharacterListDelivery`; a reset home marks a broken save.  What's left is the spawn
        calling it and turning an unplayable character away.
  - **The player makes a character at character select**, from the client, so nothing makes one until
    step 2, and it's tested through the game then: "We'll build it to test it through the game".

  Open: which messages protogame carries; how the test client shows it working.  Jacob, 2026-09-30, once
  the Characters tab was in: "to test it we'll need to build up our script and the networking portion (the
  packets to support character creation)", so character select is `test_client.py` and the packets
  together, and it's what puts the first character on the Characters tab.
  - **The packets, Jacob's names** (2026-09-30), over UDP in `0x2_` (PROTOCOL.md keeps it free for "a
    character select, say"), protocol version 5, each ask with a u32 ask number so a lost answer can be
    asked for again (proposed, and his names kept):
    - `CharacterListRequest`, client to server, and `CharacterListDelivery`, server to client ("makes it
      less confusing").
    - `CreateCharacter`, and `CharacterCreateResult`.
    - `DeleteCharacter`, and `CharacterDeleteResult`.
    - `CharacterRequestResetHome`, client to server only: "sends character back to 0, 0, 0".
    - `CharacterIsPlayable`, server to client, "so client can gray their name out".
    - "That's all I can think of right now."
    - Jacob's answers after (2026-09-30):
      - **A general answer packet that says "command accepted"**, "reused elsewhere" too: the answer to
        `CharacterRequestResetHome`, and to any later command that needs no more than that.
      - **The playable flag is a bool per character in `CharacterListDelivery`**, not a packet of its own
        ("make it the server to client response already just add a bool in it").
      - **Protogame is the name**: "the character selection and character construction are proto game then
        become game objects after load".
      - **A new character's name goes in `ShortName` only** ("short name here only").
      - **Deleting is typed out**: "player should have to type out and send back delete and get a
        deleteapproved or denied then the server deletes".  Then, asked how: "player presses delete, and the client
        pre-reqs to ask them to type in delete then sends the packet to the server with the typed in
        word.  Server either approves or denies."  One round: `DeleteCharacter` carries the uuid and the
        typed word, and the server deletes only when the word is DELETE.
      - **Two general packets in `0x3_`**, `CommandAccepted` (the ask number) and `CommandRefused` (the ask
        number and why), Jacob's pick of the two readings.
      - **The long name is left empty** until the player is in game ("the longname is not dealt with until
        they're in game yup").
    - **Built and tested** (2026-09-30): Protogame (`networking/src/protogame.rs`), the nine packets
      (protocol version 5), the ask number in the book, primlib's `new_character()` and
      `Save::of_blueprint()`, and the test client's `--create`, `--delete`, `--delete-word` and
      `--reset-home`.  `design/conductor-networking.md`, "Character select and Protogame", has it.
    - **Picking a character to play** is the spawn, step 3 of Jacob's map.  **Done, both halves.**
      **The game library's half is built and tested** (2026-10-01): `conductor_gameclock::enter()`
      and `leave()`, the players' list, saving on leaving and the world save (`design/gameclock.md`,
      "Players and the world save").
      **Networking's half, Jacob's answers (2026-10-01)**, built and tested, every check passed
      (`design/conductor-networking.md`, "The spawn"):
      - **The loop** is login, character select, the pick, the character in the world at its last save,
        and the session ending whichever way it ends, with the character saved and taken out.
      - **The packets are `UserPressPlay`** (client to server: the ask and the character's uuid) **and
        `CharacterEnteredWorld`** (server to client: the ask, the uuid, the name, and x, y, z).  A pick that
        can't be played gets a CommandRefused.  Protocol version 6.
      - **No way back to character select from the world**: "you log out back to log in screen every
        time".  Once a player is in the world, character select's asks are refused.  And when there's a
        way to log out in game: "Even if you camp out, you go back to login screen not char select."
      - **The race on a quick re-login** (the pick reading the row before the last session's leaving save
        lands): "let's set a lockout on a character being instantiated for like 1 second?  The player
        should get a reject disconnected packet but its so short they just reconnect".  So a character
        that left the world can't be picked for 1 second, and a pick inside that second gets a Kicked
        (a new reason, 6) and the client goes back to the login screen.  It can only bite after "log the
        other session out", where the second login's hash is done before the first is kicked.  Jacob
        confirmed the reading (a Kicked and the login screen, not a CommandRefused): "Yes, that's correct."
        Then, his correction: what he meant was a "load" lock, "a temporary 'load' lock on a character as
        its pulled from database to memory... and loaded in the world... locked for 1 second and then
        released?  All that lock does is prevent another one from being instantiated."  Shown that a load
        lock alone doesn't stop the stale save on a quick re-login: **both ways** ("yeah... that's the
        solution we lock it when it does that"), so 1 second on loading and 1 second on leaving.  And he
        asked: "do we have any way to force a save on the connection being kicked before the new one pops
        in?"  The GameClock marks a leaving character "saving" until its save lands, and the login that
        kicked it waits for that before handing out its ticket: up to 5 seconds (Jacob: "5 seconds"), and
        past that Login Unavailable (proposed with it; his "yes" to the plan).  Built and tested;
        `design/conductor-networking.md`, "The spawn", has it.
      - **The character goes beside the account on the Connections tab's UDP list** now ("yes").
      - **Still open**: what the client is sent after CharacterEnteredWorld (the world around it, other
        players, movement: the game's packets, to come).
- **The GameClock's checks**: an input packet (the mailbox is there, for entering and leaving), a brain for
  the AI, movement into `Transform`, the broadcast (only what each player may see).  `design/gameclock.md`.
- **A spawn system** (Jacob, 2026-09-30): keeps count of the NPCs in the world and spawns more from their
  blueprint when a kind runs low ("when the number of goblin_as is growing low").  So a copy needs to know
  its blueprint.  It goes in the housekeeping check and has to wait for `conductor_gameclock::ready()`.
- **Saving primlib's copies** on STOP SERVER, with their UUIDs and internal names (`goblin_archer_1`),
  and loading them back on START SERVER.  `design/primlib.md`.  When they join the world save, its snapshot's
  cost matters: 32.88 ms for 10,000 characters (2026-10-01), likely most of it `Save::of()`'s small
  allocations.  `design/gameclock.md`, "Its cost".
- **The world's part two**, sending chunks to a client, loading around players who move: LONGTERM_TODO.md
  and `design/world.md`.

### Networking

- **Client management**, not this iteration:
  - A player limit: "The server is full."  Today `max_waiting_logins` caps the door and nothing caps the
    world.
  - A login token on reconnect (`fingerprinter::new_token()`), so a player who drops and comes back
    doesn't pay for a hash.  Against today's rule on purpose (a dropped UDP session is gone, start over),
    so it's a design change when it comes, not a fix.
  - A session id in every UDP packet, so a home router changing the port mid-session doesn't end it.
  - Messaging a player from the web admin.
- **A client certificate for every client** (mutual TLS).  Today the server never asks a client for
  one; the test client's `--cert` is the client checking the server.  It waits on Soundcheck
  (LONGTERM_TODO.md).
- **The whitelist and blacklist changeable while the server is stopped** (Jacob, 2026-09-30).  Today the
  tabs are locked until both listeners are up, and `addip` / `removeip` answer 409 while networking isn't
  running.  It would take: the two tabs open while stopped, like Settings; `/Opus/networking` reading the
  files when networking isn't running; a change while stopped written straight to the file through
  DiskMan, with nothing to enforce.  Open: whether the lists stay locked without the database, and
  whether Connections keeps its own lock.  CLAUDE.md's rule gets rewritten with it.
- **`access_list` switchable from the page at once.**  Today it takes on the next START SERVER, while the
  lists take at once.  Jacob's call if the reboot is a bother.
- The protocol version in the Hello is `6` and the client versions are a list in `networking.cfg`.
  Whether Ensemble reports a version string or a number is Ensemble's call.
- Reverse DNS on macOS: `dns/other.rs` hands back no name.  macOS has `getnameinfo` with its own
  `sockaddr` layout (a length byte first).  Waits on a Mac.

### The web admin

- **A Tick evaluator tab under GAME MANAGEMENT** (Jacob, 2026-09-30): how the GameClock is keeping time.
  Today the Services tab's GameClock line is all there is.  Open: what it shows (each check's time, the
  longest and the latest, late cycles, maybe a graph of the last minute); the numbers the GameClock keeps
  for it (like the monitor's `latest()`); its read path under `/Opus/`, asked for when it's built.
  Nothing on it changes anything, so no `wwwhook` route.
- **A tab for the game's entities under GAME MANAGEMENT** (Jacob, 2026-09-30: "Yes let's build a tab for
  characters", then "this is going to be a heading under Game Management to edit player characters or
  NPCs since they're 'in game' entities").  So it edits, not only looks, and covers NPCs as well as
  players' characters.  The admin doesn't delete a player's character there (only the player does).
  **Seeing the characters is built first** (Jacob, 2026-09-30: "implement the improvement to the web gui
  for character visibility under Game Management"), look only.  A Characters tab, `GET /Opus/Content/characters` (Jacob's
  yes), seen by `admin` and `user` both (Jacob's pick, though it shows account names), showing only "their
  name, their X,Y,Z, and which account they're connected to", and the UUID.  **Editing is its own conversation**
  (Jacob: "Next conversation we do this"): whether an edit goes to the row or to the copy in the world
  (only the GameClock's thread touches the `World`), what can be edited, NPCs, and its routes under
  `/Opus/wwwhook/`.
- **A list of blocked names** (Jacob, 2026-09-30), its own session:
  - `Content/cfg/blocked_names.txt`, beside the two access lists, one entry a line and not in
    Constellations' table, so CLAUDE.md's "one exception" becomes three files.
  - A Blocked Names tab under CONFIGURATION, with REMOVE on each and an ADD field.
  - For now it's curse words, and it checks account usernames.  Open: whether it checks character names
    too, now players make them (2026-09-30).
  - A blocked word of 4 letters or more blocks any name with the whole word in it: "shit" blocks
    "Shitfox", "bastard" blocks "Bastardfox", but "bastard" doesn't block "Starlight".  A word under 4
    letters blocks nothing ("ass" lets "Ass" and "Cassandra" through).
  - Read on START SERVER; a change from the page waits for the next one (Jacob: "we don't need this to be
    hot swappable").
- **Move `wgui_port` from `conductor_globals.cfg` into `wgui.cfg`** (Jacob's call, 2026-09-29).  A new
  setting in `wgui.cfg`'s table in `constellations/files.rs`, the old one taken out of the globals, and
  the launcher reading it from there.  Still a hard reboot.
- Hash the two passwords in `wgui.cfg` through Security.  Security only runs with the server, and a login
  has to work before START SERVER, so it waits on Security being up from boot, or a hash on the caller's
  thread.  Plain text is fine while the page only listens on this machine.
- HTTPS.  rustls is in the build, so it waits only on wanting it, and on the browser's warning for a
  self-signed certificate.

### The rest

- **Scribe: the Debug switch** in `conductor_globals.cfg` that drops Debug lines when off.  Then go
  through every log line and move the routine ones to Debug, per CLAUDE.md.  Archivist's connect, schema
  and settings lines first, and the launcher's four start and stop lines.
- **Archivist: reconnect on its own** every few seconds while disconnected.  Today it only tries when a
  job comes in, so the web admin's database lock never lifts by itself.  Not answered yet.
- **Launcher: catch Ctrl-C** and shut down cleanly (or ignore it).  Ctrl-C loses whatever DiskMan hasn't
  written yet.  Without a crate it's a signal handler on Linux and a console handler on Windows.
- **Playtime metrics** (Jacob: "cool metrics later").  Only the latest login is kept, and nothing records
  when a player left.  It needs a table of play sessions (account, in, out, how it ended), a row written
  as each player leaves.  A new table, so its own session.
- **Where the test client lives** and what it's called, once it outgrows `networking/`.
- Monitor on macOS: `proc_pidinfo` / `proc_pid_rusage` from libproc.  Waits on a Mac to test on.
- Ensemble has no way to find `Content/` yet, if it ever needs to (it needs the certificate,
  `Content/certs/conductor.crt`, to check the server).  Whichever Ensemble session first needs it.
- **The purchased art lives in Ensemble** (2026-10-01): `Assets/Purchased/` in the Unity project, ignored
  like the rest of `Assets/` but our four folders.  It isn't in the repo at all, LFS or not.  Whether
  `Content/Assets/` (ignored, empty) still has a use is open; nothing reads it.
- **Ensemble's project files in git**, still to look at together:
  - `Packages/` is ignored, so `Packages/manifest.json` (which packages the project uses) and
    `packages-lock.json` aren't committed.  A Unity project usually commits both, and the first package
    the client code needs (the Input System, say) makes it matter.
  - `Assembly-CSharp*.csproj` and `Opus.Ensemble.slnx` are committed, but Unity rewrites them on every
    open and they usually stay out.
  - `.gitattributes` sends `.unity` scenes and `.anim` through LFS.  Both are text Unity can merge;
    asked on 2026-10-01, not answered yet.
- **Copy Anims From FBX Pack** (`Assets/Editor/CopyAnimsFromFbxPack.cs`), small things if they bite:
  - The red "would land on the same file" showed up once with the whole `Assets/Purchased/` as the
    source (about 1000 clips), and not again on `Male`.  With several packs at once, two packs' files
    can come out with the same name in a folder of the same name, or an FBX holding several clips names
    each copy after its clip.  The CLASHES view shows which, the next time.
  - Unticking is the only way to sort out one clip; a prefix renames every file that has it.  A rename
    box per clip, or the FBX's name in a several-clip copy's name, if that's not enough.
- **The HUD's Phase 2 and Phase 3** (Jacob's brief, `HUD_LAYOUT_SYSTEM.md`; `design/ensemble-hud.md`):
  - Phase 2, the catalog's export: an editor menu item (Tools > Opus) writing `WidgetRegistry.All()` out as
    the catalog JSON, as `HUD_FORMATS.md` has it.
  - Phase 3, the web layout editor: a static page that loads a catalog and a layout, a canvas at the
    layout's reference (2560 x 1440 to start, the maker can change it, shown one for one and scrolled),
    drag, move, resize to the minimum, anchor, remove, snap to a grid, and save as a layout JSON.  It's a
    new piece of the project, so it needs a name from Jacob, and a home (a new folder at the repo root is
    his call).
  - Then the login and character select as layouts (`screen` `"login"` and `"character_select"`), shipped
    with the game, never the player's.
- **Stale words in the code**, for whichever session next touches each file:
  - `access.rs`: a Warn the admin sees says "the web admin's Networking tab" (the tabs are Whitelist and
    Blacklist), and a comment the same.
  - `dns.rs`, `dns/other.rs` and a comment in the web admin's `lib.rs` still say "the TCP tab".
  - `security.rs` says the arena is kept for as long as Conductor runs (it goes with the server).
  - `snapshot.rs` says uptime is a moment less than Conductor's (it's since START SERVER).
  - `tcp.rs`'s header says a stop has no deadline (it has 2 seconds).
  - `json.rs`'s notes and the Server tab's note on the page leave out the Settings tab and networking.
  - The header of `constellations.rs` names only `postgres.cfg` as soft.
  - The launcher's boot line (`main.rs:69`) says a changed postgres.cfg or networking.cfg needs STOP SERVER
    and START SERVER, and leaves out game.cfg (seen 2026-10-01, with `world_save_seconds` in it).
  - The monitor's `Cargo.toml` header leaves out the process list.
  - `json.rs` and the web admin's `Cargo.toml` say "the Network Admin tabs", and `json.rs` says the page
    shows "the Control Panel" while the server is stopped (the Server tab).
  - "The Control Panel" where the Server tab is meant: `accounts.rs:202` (text the admin sees), the launcher's
    boot line (`main.rs:78`), and comments in the web admin's `lib.rs`; `page.html:20` has a history note.
  - The web admin's `lib.rs`: "KICK on the TCP tab"; "when Security brings in TLS for the game" (networking
    has it); the header's list of what needs `X-Opus` leaves out LOG OUT, the settings, the kick, the lists
    and the accounts.
  - The launcher's `main.rs`: `stop_server()` says networking goes first (the monitor does), and the header
    calls `start_server()` / `stop_server()` the list of what the server is (networking opens from
    `take_commands()`).  Its boot line names the soft files and leaves out `game.cfg`.
  - The monitor's `lib.rs` header leaves out the process list, per-core load and the machine's RAM.
  - `protocol.rs`: `LoginAnswer::Unavailable`'s comment leaves out Fingerprinter failing to make a token.
  - gameworld: `Ground::Flat`'s doc leaves out BEDROCK at -16; `lib.rs` says saving chunks "comes next".
  - `test_client.py`'s usage lines only work from inside `networking/` (the terminal sits in
    `Conductor/dev`), and its `--cert` example is relative to the working directory.
  - `security/windows.rs`, `fingerprinter/windows.rs` and `fingerprinter.rs` say the Windows code was never
    built (it was, 2026-09-30).  The tools' `Cargo.toml` header lists only some of what's in the crate.
  - `accounts.sql`'s header says a player gets a clear message (the admin makes accounts now).  The file is
    frozen, so it stays.
- `RegionMap::from_bytes` never checks for a count of 0 regions.  REGION_MAP.md says 1 to 255, and a map of
  0 is refused anyway unless its width or depth is also 0.  The code is what gets fixed, if it's worth it.

## Ideas

Things we thought of along the way.  None of them are promised.

- The world: blending one biome into the next where two regions meet.  Today Alpha and Omega meet in a
  sharp divide.  Jacob: "in a real build and not this test, we'll have a blending technique".
- Scribe: a size limit that starts a second file for the day (`2026_09_28.1.scribe.log`).
- Scribe: the lines logged before `move_to()` stay in the default folder if the config points elsewhere.
  Not worth fixing while both are the same folder.
- Scribe: a lost log file is only retried at midnight UTC or on `move_to()`.  A retry every few minutes
  would get it back sooner after a full disk is cleaned up.
- Scribe: the caller shows the path Rust compiled with (`launcher/src/main.rs`).  Trim to the file name
  if that gets noisy.
- Constellations: a stray `Content/` inside `Conductor/dev` shadows the real one, since the walk up takes
  the nearest.  Skipping a `Content` whose parent holds a `Cargo.toml` would rule it out.
- The Storage tab could show `swaps_waiting` from DiskMan's status (it's in the struct, not the JSON).
- Archivist: a password that starts or ends with a space loses it, since every value is trimmed.  Quotes
  around the value would fix it, if it ever matters.
- Archivist: more than one worker, if one ever can't keep up.  Two workers can finish jobs out of order
  (a SELECT missing the UPDATE sent just before it), so it needs one player's jobs kept on one worker.
- Monitor: a SQL read / write split by rows, not just by jobs; Postgres's own view
  (`pg_stat_database`) as an Archivist job every few seconds.
- The Storage tab: "last read / last write" by file.  DiskMan sees every file, so it's ready when wanted.
- DiskMan on Windows: it leans on `fs::rename` replacing a file (its writes and the `.wait4server` swap
  both), and skips flushing the folder.  It runs there; nobody has looked closer.
- Notices: no cap.  A flood of Warns left alone for days keeps growing in memory.
- Web admin: keep the CPU and memory history on the server, so a page opened late sees the last minutes.
- Web admin: a Debug on / off switch for the Log tab, once Scribe has its switch.
- Web admin: saved page layouts per account, and more accounts than `user` and `admin`.
- Web admin: the Settings tab could show each default, with a "back to default" click.
- Web admin: pin Conductor to the top of the System tab's process list, if busiest-first buries it.
- Web admin: a setting that starts the server when Conductor boots, for a machine nobody sits at.
- Web admin: a line on the Server tab saying what a RESTART is for (a changed soft file is read again).
- Security: raise the memory (128 MiB, say) once the benchmark says what 64 MiB costs.
- Security: the `parallel` (rayon) feature would split a hash across cores.  A crate, and its threads
  bypass `threads::spawn()`, so it's not taken.
- Security: say on the Services tab whether the huge pages actually landed (`AnonHugePages` in
  `/proc/self/smaps`).
- Security: wipe passwords from memory once they're hashed (`zeroize`).  Off for now.
- Networking: make the TLS pair on first start (the `rcgen` crate) instead of openssl by hand.  Jacob
  picked the command for now.
- Networking: the private key sits in DiskMan's cache while Conductor runs.  Reading it past DiskMan
  would keep it out, if it ever matters.
- Networking: the login threads' `serving` list and the failure hold are two small maps under two locks,
  and the access lists a Vec walked on every accept.  Fine at this size; look here first if the door ever
  sees thousands.
- Networking: a ban from the page rewrites the list file, so a hand-written comment in it is lost.
- Running Conductor with no console window.  Closing the console kills it today without a clean shutdown.
  Ways out: start it detached (`setsid`, `nohup`), a systemd or Windows service, or a Windows build with
  no console.  Jacob's pick when it matters.
