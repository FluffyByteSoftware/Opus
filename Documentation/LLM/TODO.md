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

- **An all-air chunk that costs nothing** (2026-10-01, put forward with the 1 m blocks, not Jacob's ask
  yet).  The world is eleven chunks tall now, so a player's 891 chunks are mostly air, each kept whole at
  64 KB (about 57 MB a player).  A chunk that's all one kind could be held as that one kind until a block
  in it changes.  Worth it if memory bites with more players.
- **A `world_size` change that keeps the digging** (2026-10-01, Jacob's yes, for when chunks are saved).
  Today a change deletes the whole world and makes a new one from a new seed, which costs nothing while
  nothing writes chunk files.  Once digging is saved, it would wipe every dug chunk.  Keeping the old seed
  instead makes Omega's hills the same wherever the two sizes overlap (the heights come from the seed
  column by column), so a new `region.map` and heights file could be made at the new size with the dug
  chunks inside its edges kept.  The chunks outside them need a call: deleted, or kept aside.
- **A character saved outside a smaller world** (2026-10-01).  A smaller `world_size` can leave a
  character's last saved spot past the edge.  Everybody stands at 0,0,0 today, so it can't happen yet;
  it can once there's movement.  Where it goes then (0,0,0, the nearest edge) is Jacob's call.

- **Chat** (Jacob, 2026-09-30: the 0.0.1 goal is "get a player spawned in the world and able to chat").
  **The order changed** (Jacob, 2026-10-02, "rewinding a bit"): the server's side first, the client's box
  after.  Being settled, the server's side:
  - **The command** (Jacob, 2026-10-02): "since EverQuest set precedent it will be `/chat <message>` (up to
    300 chars)", and it posts `[Chat] <Player>: Yo yo yo`.  Only a player whose character is in the world
    can chat.
  - **Who hears it: everybody in the world** (Jacob, 2026-10-02: "for now its going to be everyone this is
    our 0.0.1 release milestone").  Nearby chat waits on movement and positions.
  - **What the client sends** (Jacob, 2026-10-02): "the entire command is sent... we have a "chat window" in
    the client and whatever is sent there is sent as a plaintext string to the server and the server goes
    "oh hey that started with / that means look for a command"".
  - **What goes out** (Jacob, 2026-10-02): "we send a packet to all users including the person who sent the
    message on the next "chat" GameClock tick that carries chat (which should be every beat)", the line
    reading `[Chat] Jacob: Yo yo yo!`.  So chat goes out from the GameClock's broadcast check, once a
    250 ms cycle, and the speaker hears their own line back ("yes").
  - **What a message may hold** (Jacob, 2026-10-02): "Plain english characters letters numbers special
    characters, spaces."  Printable ASCII.
  - **300 characters** (Jacob, 2026-10-02): "The server will just ignore everything after 300", so a longer
    message is cut, not refused.  **Ensemble**: "I want the client to refuse to generate more than 300
    characters": the chat box stops taking keys at 300.  For the chat box's session.
  - **One fixed channel** (Jacob, 2026-10-02, asked whether the server sends the pieces): "nah a fixed chat
    channel is sufficient like the way the old shit muds did it!"  The server sends the finished line.
  - **Built and tested** (2026-10-02, all nine checks passed): `design/conductor-networking.md`, "Chat",
    and PROTOCOL.md have it.  The plan, OKed: protocol version 8; `PlayerCommand` (`0x37`, an ask number and the
    line) answered with CommandAccepted or CommandRefused; the line into a chat mailbox in the GameClock;
    the broadcast check sends `ChatDelivery` (`0x38`, a count and the lines) to everybody in the world,
    through a function networking hands the GameClock at its start.
  - **A line without a `/`** (Jacob, 2026-10-02): "anything typed will default to being said -- something
    we won't implement yet but TODO!"  Saying things (nearby, once there are positions) is still to come;
    until then a line without a `/` is refused.
  - Later: a limit on how fast one player can chat, whether the web admin sees chat, nearby chat.
  - **Ensemble's chat box**, after: whatever is typed goes out as a PlayerCommand as it was typed, every
    ChatDelivery's lines are printed, and it stops taking keys at 300 characters.  Open: where "when you
    log in" puts it (in the world after PLAY, or the HUD with Phase 1's chat placeholder).  Ensemble speaks
    version 8 already (the number only) and logs a ChatDelivery as one it doesn't handle yet.
- **`/who`** (Jacob, 2026-10-02, after chat passed, "we're barely 30% so we're gonna keep going"): "a /who
  that shows all connected players", each as "[PlayerName] is currently at [x,y,z]".  Planning: who counts as
  connected, who sees the answer, where the positions come from, the line's exact form.  His answers:
  - **Characters in the world only** ("I agree characters in the world only").  Only the one who asked
    sees the answer.
  - **The line**: `[Aldric] is currently at [0, 0, 0]`, brackets and all.  "Eventually we will be putting
    in a biome name there (or zone)".
  - **Whole blocks**: "1, 0, -2 a block is the width of a player so they can only really fit on one".
  - **Everybody seeing everybody's position**: "thats fine for now".  Who may see positions is for later.
  - **The look** (his old MUD's, 79 wide), "except it will says ] Forgotten Legends [" and "There are
    seven legends currently online.":

    ```text
    ----------------------======] Realms of the Dragon [======---------------------
                                Fri Oct  2 03:53:24 2026
    ----------------------------------] Players [----------------------------------
    Bujin    Eetius   Guesty   Kriket   Malachy  Trzk     Zeleya
    --------------> There are seven players in the Realms right now. <-------------
    ```

    **Both** (Jacob, 2026-10-02): "A but if they do /who list It shows [Aldric] is currently at [0, 0, 0]
    and so on".  So `/who` is the grid of names, `/who list` the line each with the position.
  - **The time** (Jacob, 2026-10-02): "we are going to send this to the client as seconds from midnight UTC
    and let the client determine the local time zone".  **The count**: "Write them all out", and "we can
    build a new tool that converts a number into written words?  Up to one million".  **The grid**: "make
    it fit our actual chat size".  So the client draws the date, and the grid to its own chat box's width;
    being settled: what the packet carries, and where the words tool lives.
  - **The packet** (Jacob, 2026-10-02): "I like the idea of a who being a packet a list of names and the
    time from the server The Who was run".  **The time is seconds since today's midnight UTC** (his pick of
    the two), and the client takes the date from its own clock.
  - **Numbers into words**: "I'd make it a static class in c# but words isn't right... I think actually we
    build a new lib.  This isn't anything like tools.  It's just a Translator and a Math object in here."
    Up to the largest int ("Past that it would show the number").  **It's the client's** (Jacob,
    2026-10-02): "a static class `string NumberToWords(int number)`", British, with the "and" ("IN the honor
    of Discworld!").  The server sends the names; the client counts them and writes the count out.
  - **A big answer** (Jacob, 2026-10-02): "we need to build a "span packet" that tells the client there are
    X of Y packets about to show up and to wait till all are received or a specified time elapses?"  Each
    piece says which of how many (OKed, "yes"), and the client waits 2 seconds ("2s is fine").
  - **Written, waiting on Jacob's build** (2026-10-02): protocol version 9, WhoDelivery (`0x39`) and Span
    (`0x3A`); `Translator.NumberToWords()` in Ensemble now ("a"), its `.meta` from Jacob's machine; the test
    client draws the box at 79 with the count in digits.  `design/conductor-networking.md`, "/who", and
    PROTOCOL.md have it.
  - **Ensemble, with the chat box**: draw the WhoDelivery (the box to the chat box's width, the time in the
    player's time zone, the count with `NumberToWords()`), and put Spans back together (2 seconds).
- **A Math class on the client** (Jacob, 2026-10-02): "maths was going to be for any formulas we ended up
  repeatedly needing but we don't need it... yet put this in todo".  A static class beside the Translator,
  when there's a formula used in more than one place.
  - **Ensemble**: these lines only line up in a monospaced font, so the chat box needs one.
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
- The client versions are a list in `networking.cfg`.  Ensemble sends Player Settings' Version as it is
  (`0.0.0.1`, 2026-10-02: "we're not ready for 0.0.1 yet"); the list is `0.0.0.1, 0.0.1` for it and the test
  client.  Its default in `constellations/files.rs` is still `0.0.1`.
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
- Ensemble has no way to find `Content/` yet, if it ever needs to.  The certificate didn't need it: the
  client carries a copy, `Assets/Data/Certs/conductor_crt.txt` (2026-10-02).
- **The purchased art lives in Ensemble** (2026-10-01): `Assets/Purchased/` in the Unity project, ignored
  like the rest of `Assets/` but our four folders.  It isn't in the repo at all, LFS or not.  Whether
  `Content/Assets/` (ignored, empty) still has a use is open; nothing reads it.
- **Ensemble's project files in git**, still to look at together:
  - `Packages/` is ignored, so `Packages/manifest.json` (which packages the project uses) and
    `packages-lock.json` aren't committed.  A Unity project usually commits both, and the first package
    the client code needs (the Input System, say) makes it matter.
  - **Done** (2026-10-02, Jacob's yes): `Assembly-CSharp*.csproj`, `Opus.Ensemble.sln` and `.slnx` are out
    of git and in the `.gitignore`.  Unity rewrote them on every compile, and the rewrite stopped a
    `git checkout main`.
  - `.gitattributes` sends `.unity` scenes and `.anim` through LFS.  Both are text Unity can merge;
    asked on 2026-10-01, not answered yet.
- **Copy Anims From FBX Pack** (`Assets/Editor/CopyAnimsFromFbxPack.cs`), small things if they bite:
  - The red "would land on the same file" showed up once with the whole `Assets/Purchased/` as the
    source (about 1000 clips), and not again on `Male`.  With several packs at once, two packs' files
    can come out with the same name in a folder of the same name, or an FBX holding several clips names
    each copy after its clip.  The CLASHES view (built and tested, 2026-10-01) shows which, the next
    time.
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
  - The login and character select as layouts (`screen` `"login"` and `"character_select"`), shipped
    with the game, never the player's.  **The login is built and tested** (2026-10-01,
    `design/ensemble-hud.md`, "The login, as written"); character select's layout is still to come.
- **Security in the client** (Jacob, 2026-10-01, at the login's hand-off: "next conversation we start
  building security into the client").  **The client's half is built and tested** (2026-10-01, every check
  passed); `design/client-security.md` has the contract.  **Conductor's half is built and tested**
  (2026-10-02, every check passed): protocol version 7, the key refused if it isn't one, the account desk
  making the key from what the admin types (`pbkdf2` and `sha2`, OKed), every account deleted.  What it
  covers, Jacob's answers:
  - **The password is turned into a key on the client**, in `Assets/Code/`, on SUBMIT and for Remember
    Me: "even though its going over TLS we don't want to save it to their local disk as plain text!"
    The point is that the password never crosses the internet or lands on disk as typed; "I suppose
    there will be a few moments where its in memory."
  - **This session builds the client half** and writes down the change Conductor needs (it takes the
    key where it took the password); Conductor's half is built in a session of its own.
  - **Every account is deleted** when the switch comes, rather than carried over (their stored hashes
    are of the password, not the key).
  - **Remember Me is a file of our own** in the folder every player file goes in,
    `~/.config/unity3d/FluffyByte/Opus.Ensemble/` (`PlayerFiles.cs`).
  - **The hash**: "whatever will work with the _server_".  PBKDF2 with SHA-256, **600,000 rounds,
    settled** (Jacob: "Make this the full 600,000"), timed at 2852 ms in the Unity editor.
  - **The key made "in the background while the player moves forward in login"** (Jacob, 2026-10-01).
    It already runs on a worker thread.  **Jacob picked: after SUBMIT, the client connects to the server
    (TCP, TLS) while the key is still being made, and sends the Login the moment the key's ready.**
    Part of the client's net code; not started when the player leaves the Password box.
  - **The Accounts tab: Conductor makes the key** from what the admin types (Jacob: "we'll have conductor
    do it"); the page doesn't.
  - **The Remember Me file is readable by other users on the same Linux machine**: Unity's .NET can't set
    a file's permissions without reaching into the OS (a `chmod` through libc).  Worth doing before
    players have it.
  - The client checking the server's TLS certificate is written (2026-10-02, with the net code).  Further
    off, Soundcheck handing each client a certificate of its own (LONGTERM_TODO.md).
- **The login screen, later** (2026-10-01): SUBMIT logs in and Remember Me works (2026-10-02).  Still
  open: the effects between screens ("cool ass effects if we can", "I don't know yet").
- **The client's net code** (Jacob, 2026-10-02: "its time to build up the client to submit and move over
  to character selection!").  **Built and tested** (2026-10-02, ten checks): `design/ensemble-networking.md`
  has it.  `Assets/Code/Net/` (the protocol, the login over TLS, the server's certificate, UDP), a
  `login_status` line under SUBMIT, character select's screen.  Jacob's answers:
  - **Get there this session**: character select shows the account's characters and LOG OUT.  CREATE,
    DELETE and PLAY are a session of their own.
  - **Already logged in elsewhere**: the client offers to log the other session out or to log off, for
    30 seconds (the server's own wait); "if no answer it disconnects this session not the existing".
    The server already holds the ticket until the other character's save is in (the "safety" lock).
  - **The certificate**: the client carries a copy of `conductor.crt` and refuses any other server.
  - **TLS 1.2**: if Unity can't do 1.3, Conductor takes 1.2 as well (rustls's `tls12`), "but I'm pretty
    sure it will do 1.3".  Unity's .NET has no `SslProtocols.Tls13` (the first compile), so it's done.
  - **The client version**: "we're not ready for 0.0.1 yet".  Ensemble sends Player Settings' Version,
    `0.0.0.1`, and `networking.cfg`'s `client_versions` takes it.
  - **Next** (Jacob, at the hand-off: "create/delete/select/play character next round"): CREATE, DELETE,
    RESET HOME and PLAY at character select.  **Built and tested** (2026-10-02, eleven checks):
    `design/ensemble-networking.md`, "Character select, the rest of it".  On CharacterEnteredWorld the
    client says "In the world as <name>" with LOG OUT; the world on screen is still to come.
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
