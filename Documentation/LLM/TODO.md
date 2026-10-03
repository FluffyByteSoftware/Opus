<!--
File:       Opus/Documentation/LLM/TODO.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- TODO

The big features, the ones that are a run of sessions each, are in LONGTERM_TODO.md instead.  What's built
isn't here: the design files hold Jacob's answers and what came of them, and git has the rest.

## Deferred

Things that wait on a piece that doesn't exist yet, or on Jacob wanting them.

### The game

- **An all-air chunk that costs nothing** (2026-10-01, put forward with the 1 m blocks, not Jacob's ask
  yet).  The world is eleven chunks tall now, so a player's 891 chunks are mostly air, each kept whole at
  64 KB (about 57 MB a player).  A chunk that's all one kind could be held as that one kind until a block
  in it changes.  Worth it if memory bites with more players.  `CODE_REVIEW_0.0.1.md`, I1, has the
  measurement.
- **A `world_size` change that keeps the digging** (2026-10-01, Jacob's yes, for when chunks are saved).
  Today a change deletes the whole world and makes a new one from a new seed, which costs nothing while
  nothing writes chunk files.  Once digging is saved, it would wipe every dug chunk.  Keeping the old seed
  instead makes Omega's hills the same wherever the two sizes overlap (the heights come from the seed
  column by column), so a new `region.map` and heights file could be made at the new size with the dug
  chunks inside its edges kept.  The chunks outside them need a call: deleted, or kept aside.
- **A character saved outside a smaller world** (2026-10-01).  A smaller `world_size` can leave a
  character's last saved spot past the edge.  Everybody stands at 0,0,0 today, so it can't happen yet;
  it can once there's movement.  Where it goes then (0,0,0, the nearest edge) is Jacob's call.  As built
  in session 9, GameWorld can't read a column past the edge, so PLAY's ground check fails and the
  character goes to the spawn point's last known place with "The server couldn't check where your
  character stands..." (`design/world.md`), which may be answer enough; his to say.
- **Chat's rest** (chat, `/who` and the anti-flood are built: `design/conductor-networking.md`, "Chat",
  "/who" and "Commands and the anti-flood"; the chat window is `design/ensemble-hud.md`):
  - **A line without a `/`** (Jacob, 2026-10-02): "anything typed will default to being said -- something
    we won't implement yet but TODO!"  Saying things nearby waits on positions; until then a line without
    a `/` is refused.
  - **Nearby chat**, once there are positions.
  - **Whether the web admin sees the chat.**
  - **Who may see positions.**  `/who` shows everybody's today ("thats fine for now"); the world's objects
    go only to players whose view they're in (session 9).
  - **Kicking a player who keeps flooding** (2026-10-02): today a command too soon is only refused.
    Whether enough of them in a row (say 20 in 10 seconds) gets a Kicked, and with what reason, is Jacob's
    call.
  - **`/who <character name>`** (Jacob, 2026-10-03): "eventually we're gonna make who able to do /who
    <character name> but not yet".  Until then anything after `/who` (`/who list` included) gets "Try
    /who.", which he kept.  What it shows for one character (the same line, or more) is open.
  - **`/help`** (2026-10-02): could list the table of commands.  Its own feature.
  - **Admin commands** (Jacob, 2026-10-02): "its a permissions difference but the commands will otherwise
    be the same".  So the same table, with who may run each; from where, still open.
- **The chat field and the Input System, once there's movement** (2026-10-02, Jacob: "Do we need to
  add or adjust anything in the new Unity input system?").  Not for the keys to the field: UI Toolkit's
  KeyDownEvents already reach it, and the panel's root sees them first.  Once WASD drives the character
  (`InputSystem_Actions.inputactions`), a focused field takes every key, movement included, and an
  unfocused one lets them through: the Player action map on while `GameFocus.Has` (its `Taken` and `Lost`
  events), off while a widget has the keys.  **Settled** (Jacob, 2026-10-02, from EQ): "if you move while
  chatting your movement keys just go to the chat when its focused... everything does its gotten me killed
  a few times before I realized I was chatting".  So no key is ever both; the game never moves you while
  you're typing.  Part of movement.
- **Movement, (b): moves passed on at once, EverQuest's way** (Jacob, 2026-10-03, session 10: "B", and
  "this might have to be dumped to a new conversation").  Networking passes each PlayerMoved to the players
  near it as it arrives, before the GameClock judges it, and a pull-back corrects the watchers too.  Round
  one as built judges once a cycle and passes on in the broadcast.  What's open is in
  `design/ensemble-world.md`, "Movement", at the end.
- **Movement speed modifiers** (Jacob, 2026-10-03, session 10: "we're going to have movement speed
  modification abilities (potions, spells, enchantments)").  The walk, 4 blocks a second, stays fixed in
  code as every character's base; a character's own speed, with what's on it, would be a component the
  server checks moves against, and a packet telling the client when it changes (CharacterEnteredWorld only
  says it once).  `design/ensemble-world.md`, "Movement".
- **Fewer voxels a column, EQ Next's style**: became smooth voxels, settled in session 11 (a separate chat,
  its summary brought back by Jacob): `design/smooth-voxels.md`, and LONGTERM_TODO.md's "Smooth voxels".
- **Jumping** (Jacob, 2026-10-03, session 10, on movement: "we can do jumping later").  Movement steps up
  one block on its own and falls off edges; a jump is its own feature.  `design/ensemble-world.md`,
  "Movement".
- **First person** (Jacob, 2026-10-03, session 10: "the camera stays fixed for now.  The only further
  improvement I might do is allowing you to go first person").  The camera is fixed behind the character
  for movement.
- **A block broken from its chunk gets a box collider** (Jacob, 2026-10-03, session 10: "Only an
  individual voxel would need a box if its been broken from its chunk").  The ground's chunks each have a
  mesh collider; this waits on breaking blocks.
- **A resizable chat window** (session 7, 2026-10-03, Jacob's pick: "Resizable chat window").  Being
  talked through.  What's there: the catalog already says chat is `Resizable` (min 320 x 160), but
  nothing in the game lets a player change a size, and nothing writes the player's `hud_layout.json`
  (only the web editor was to, Phase 3).  A widget never sizes its own box (`Widget.cs`), so a grip
  would be HudBuilder's, not ChatWidget's.  **Jacob**: "I'd like it so that you can right click it and
  lock it or unlock it and when unlocked if you go to the border of the chat box you can drag its edges
  out into the game screen space and resize it."  So a right-click menu on the window (Lock / Unlock),
  and while unlocked its border is a grip, any edge.  His answers (session 7): **every edge and corner**,
  "like windows window"; **unlocked by default**; **remembered**, "saved in userprefs and regenerated at
  boot"; **an unlocked window has "a flashing orange/red/yellow border temporarily drawn on it"**; **moving
  too**; **every widget** "become[s] movable and lockable and unlockable with this pass", not only chat;
  the text keeps its size when the window grows.  **Added**: the chat's right-click menu also sets the
  chat's font size, "on a sliding scale from 1 to 18 for now we'll make this a px measurement but it might
  be an algorithm later".  Unity's own right-click menu is editor-only, so the menu is ours; a runtime
  pointer can't turn into a resize arrow without a picture of one, so the edge under the mouse lights up.
  **Then** (Jacob): the file is **`playername_hud_layout.json`** in the player's folder ("we can do this
  even better"), a layout a player; **the border flashes the whole time a widget's unlocked** ("its gonna
  be annoying but yes until I get a lock/unlock icon in to draw on the title bar of the UI element");
  **the font size 18 to 42**; **moving by grabbing the inside, resizing by the edges, and "the mouse cursor
  should change appearance like it would in windows"**; **every widget movable, only chat resizable** for
  now.  The start screen and character select stay fixed.  "Playername" is **the character's name**
  (Jacob: "Yes"; Ensemble never learns the account's), and **the font size is the chat's contents, not its
  header** ("just the contents of the chat window not the header").  The pointers: he asked whether we
  can bring in our own, animated or not, and in what format, before the plan's OK.  **His answers**: he has
  fantasy pointers already, at 64 x 64 and 256 x 256; **not animated**; **the normal pointer stays**, and
  there are two of his own, "a bronze one to indicate moving the window and one to indicate gripping the
  edge".  What Unity's source says (UnityCsReference): a UI Toolkit `style.cursor` with a texture calls
  `Cursor.SetCursor(texture, hotspot, CursorMode.Auto)`, the hardware pointer, and the texture is imported
  with Read/Write on or Texture Type: Cursor.  The forums and Unity's issue tracker: Windows' hardware
  pointer is held to about 32 x 32, Linux takes 128 x 128; `CursorMode.ForceSoftware` draws any size, a
  frame behind the mouse.  So the 64 x 64s, Linux first; Windows untried.  **Then** (Jacob): they're a
  purchased pack, already in `Assets/Purchased/` (the PremiumCursors: Move, Hand1, Hand2 the fist, and
  twenty more), so slots on ScreenRoot; **the pointer changes the moment the mouse is where a drag can
  start, stays while dragging, and goes back to the normal one on letting go or leaving that place**.
  **Built** (session 7; it compiles, and saves, locks, drags and the font size all wrote the file): `design/ensemble-hud.md`, "Moving, resizing and
  locking"; HUD_FORMATS.md's layout version 2.  Left: a lock/unlock icon on a widget's title bar (Jacob's,
  to replace the flashing), Reset HUD To Default somewhere a player can reach it, and the pointers on Windows.
  A lock that seemed not to be kept (test 3) wasn't, on a second look (Jacob: "it doesn't appear that this
  is whats happening ignore this sorry").
- **More than one chat window** (2026-10-02, from EverQuest): Enter or `/` goes to "the chat window I
  last used".  With one window that's it; with several, the chat widgets share one remembered "last used"
  (the one last typed in or clicked), and Enter and `/` go there.  A few lines when a second window comes;
  the catalog's `MaxCount` for chat is 1 today.
- **A major clean-up of code and documentation** (Jacob, 2026-10-02, at the chat window's hand-off: "Next
  conversation we're gonna do major clean up of code and documentation").  **Pass one of the docs is done**
  (2026-10-02): the facts the code had overtaken, the built history in this file, the doubles, and CLAUDE.md's
  longer directions cut down; the stale words in the code went the day after, and CLAUDE.md became the
  rules and a map (Jacob: the aim was "to reduce token overhead with obsolete instructions and drift", not
  to trim for its own sake).  Seen and left:
  `ChatWidget`'s `Font` being a static set by ScreenRoot, which works but is the only slot handed over
  that way (the other screens' font goes through `ApplyText()`); `design/conductor-networking.md`'s "Chat"
  and "/who" sections, patched for the commands' move rather than rewritten; `design/ensemble-hud.md`'s
  login sections, kept as history (Jacob: leave them); REPORT.html, not looked at.
- **The view by column** (session 9, 2026-10-03): the GameClock's view looks at every player against every
  object each cycle, measured at 24.28 ms for 500 players all in sight of each other, every one moved, half
  the broadcast's 50 ms, and growing with the square.  Keeping the objects by their column of chunks, so a
  player only looks at the squares within their view, would make it grow with how crowded a place is
  instead.  Not needed at today's numbers (`design/gameclock.md`, "The view").
- **An unreachable spawn point** (Jacob, session 9, 2026-10-03): "if the player is in an unreachable spawn
  point, it moves them to the next spawn point and deletes the invalid one".  Waits on there being more than
  one spawn point, and on spawn points being data rather than `SPAWN_POINTS` in the code (generated with the
  world).  "Unreachable" today would be a column whose top is too high to stand on inside the world
  (`design/world.md`, "A saved character inside the ground").
- **"Reliable ordered" packets over UDP** (Jacob, session 9, 2026-10-03): "we could consider redesigning
  the UDP service to have "reliable ordered" packets?"  Today only a client's ask is sent again until
  answered; nothing the server starts on its own is (ChatDelivery, the world's objects).  The objects in
  view are mended by a roll call once a second instead (`design/ensemble-world.md`).  A channel the server
  sends again until the client says it has it, in order, would suit chat; positions are better without
  it, since a fresh one shouldn't wait behind a lost old one.  Its own feature.
- **A Math class on the client** (Jacob, 2026-10-02): "maths was going to be for any formulas we ended up
  repeatedly needing but we don't need it... yet put this in todo".  A static class beside the Translator,
  when there's a formula used in more than one place.
- **Protogame's rest** (character select and the spawn are built, `networking/src/protogame.rs`;
  `design/conductor-networking.md`, `design/conductor-accounts.md` and `design/primlib.md` have Jacob's
  answers):
  - **What the client is sent after CharacterEnteredWorld**: the chat, the chunks it asks for, and since
    session 9 the world's objects in its view (protocol version 14).  Movement's input packet is to come.
  - **The long name is the player's to capitalize, later** (Jacob: "there will be a way to set your
    _LONG_ to be capitalized how you want in game but when creating its this way").  `LongName` is where
    "McKay" goes; it's left empty until then.
  - Actor, Agent and Character (anything that acts; one the computer controls; one with a human on top)
    were named before primlib, and are likely sets of components now rather than types.
- **The GameClock's checks**: an input packet (the mailbox is there, for entering and leaving), a brain for
  the AI, movement into `Transform`, the broadcast (only what each player may see).  `design/gameclock.md`.
- **A spawn system** (Jacob, 2026-09-30): keeps count of the NPCs in the world and spawns more from their
  blueprint when a kind runs low ("when the number of goblin_as is growing low").  So a copy needs to know
  its blueprint.  It goes in the housekeeping check and has to wait for `conductor_gameclock::ready()`.
- **NPCs that keep the world awake**, and the world asleep where nothing is: LONGTERM_TODO.md, "The world
  asleep and awake" (Jacob, 2026-10-03, session 11).
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
  one; the test client's `--cert` is the client checking the server.  Soundcheck is where it goes
  (LONGTERM_TODO.md).
- **The whitelist and blacklist changeable while the server is stopped** (Jacob, 2026-09-30).  Today the
  tabs are locked until both listeners are up, and `addip` / `removeip` answer 409 while networking isn't
  running.  It would take: the two tabs open while stopped, like Settings; `/Opus/networking` reading the
  files when networking isn't running; a change while stopped written straight to the file through
  DiskMan, with nothing to enforce.  Open: whether the lists stay locked without the database, and
  whether Connections keeps its own lock.  CLAUDE.md's rule gets rewritten with it.
- **`access_list` switchable from the page at once.**  Today it takes on the next START SERVER, while the
  lists take at once.  Jacob's call if the reboot is a bother.
- The client versions are a list in `networking.cfg`, `0.0.0.1, 0.0.1` today.  `0.0.0.1` was Ensemble's
  pre-release version and is stale: the Login's version is Soundcheck's now (`0.0.1`, its csproj's
  `<Version>`), and Ensemble sends none.  Taken out with the `tls12` feature ("Soundcheck", below).
- Reverse DNS on macOS: `dns/other.rs` hands back no name.  macOS has `getnameinfo` with its own
  `sockaddr` layout (a length byte first).  Waits on a Mac.

### The web admin

- **A Tick evaluator tab under GAME MANAGEMENT** (Jacob, 2026-09-30): how the GameClock is keeping time.
  Today the Services tab's GameClock line is all there is.  Open: what it shows (each check's time, the
  longest and the latest, late cycles, maybe a graph of the last minute); the numbers the GameClock keeps
  for it (like the monitor's `latest()`); its read path under `/Opus/`, asked for when it's built.
  Nothing on it changes anything, so no `wwwhook` route.
- **Editing the game's entities under GAME MANAGEMENT** (Jacob, 2026-09-30: "this is going to be a heading
  under Game Management to edit player characters or NPCs since they're 'in game' entities").  Seeing them
  is built, the Characters tab, look only.  Editing is its own conversation (Jacob: "Next conversation we
  do this"): whether an edit goes to the row or to the copy in the world (only the GameClock's thread
  touches the `World`), what can be edited, NPCs, and its routes under `/Opus/wwwhook/`.  The admin doesn't
  delete a player's character there (only the player does).
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

### Soundcheck

Started 2026-10-02 (`design/soundcheck.md`).  Built and tested on Linux: the login over TLS 1.3 to the
Ticket, PLAY and the way back, Ensemble's half, admin mode's PUBLISH, the check at start (1.3 s for
655 MB) and the patch a file at a time.  Untested: the launcher's own restart after a patch (Parked in
TEST_CHECKLIST.html until Soundcheck ships beside the game), the Windows rename-aside, and anything else on
Windows.  What's left, a step each:

- **The world's dump** (Jacob, 2026-10-02).  **Dropped in session 12** (2026-10-03, Jacob: "we are
  dropping it... we don't need it anymore"): with nothing drawn past the view, the simple overworld map
  has no use left.  Taking it out is a step of its own, a protocol bump across all four programs
  (`design/smooth-voxels.md`, "Settled in session 12").  What it was: Conductor dumps a portable world to `Content/`, "a general
  shape of the world but 'smoothed'", for the client to carry (region.map and the heights, the ground as
  Conductor would build it, so the distance doesn't vanish; the chunks around the player stream over UDP
  and override it).  It ships as a file of its own, compressed, with its own line in the manifest beside
  the client's files, "because this is much more likely to need to be downloaded": a new world isn't a new
  client download.  Not designed yet: what's in it, how "smoothed", where it lands in the install
  (`Ensemble_Data/StreamingAssets/World/`, the fifth folder), and whether Conductor dumps it on START
  SERVER or on a button.  Its own session, after the patcher.
  **Session 1, 2026-10-03** (Jacob): the dump comes first, inside Conductor.  He pictures it **smoothed**
  ("I am imagining a smoothed shape"), not the exact ground: the point is "didn't want to give them the
  entire worlds voxel information so they could find all the secrets".  A seed was weighed and is out for
  the same reason (it rebuilds the whole world exactly).  So the dump is the surface's rough shape for the
  distance, nothing under it, and the real chunks come from the server near the player.  **Hidden things**
  (Jacob): "there may be voxels with chests on it and stuff we want to make 'hidden'... The client should
  know its there but the player needs to discover it."  Still open: whether "the client knows" means it's
  sent ahead (and a changed client can show it), or only sent once the player is near enough to find it.
  Also open: how coarse "smoothed" is, and whether the dump is compressed (Jacob asked what it buys).
  His answers after: **hidden things are sent ahead and hidden by the client** ("honestly with the way we're
  building this I am not too worried about cheating this isn't a serious game like that... not yet at least"),
  **with room to tighten later** ("make it so that we have the option I guess?  Like a flexibility to add more
  security"): so what a player is sent goes through one place on the server that can hold things back one
  day.  **16 by 16 blocks a patch** ("the world is going to be rather large though so maybe we go with
  16x16?"), his pick when the session had no better than a guess.  **Not compressed** (the session's lean, not
  OKed yet): about 1 MB a layer at `world_size` 16, nothing beside a 655 MB game; squeezing is for the chunk
  stream (64 KB a chunk, 4 KB a packet), run-length by hand, no crate.  **When it's written**: "When the world
  saves I think" (which save, below).  Asked how Minecraft draws the distance (the answer is in the session's
  reply: it doesn't, past what the server sent).
  **Then** (Jacob): each 16 by 16 patch carries **its average height and its most common top block** ("we can
  take the average with the highest occurence (if its dirt) and put dirt to color the whole thing"), the
  client colouring the distance by the kind, Minecraft's map colours' idea.  **Written when the world is made
  and at the terrain save** (every 15 minutes and on STOP SERVER, part two, not built: "We may have to turn
  that save rate down").  On the size: "It can be larger file...".  **It goes to the web folder**: "should
  copy it to /opt/storage/WWW/download/map/<somename>".  Open: the name; what "turn the save rate down" means;
  how the patcher learns the dump's hash, since the platform manifests are PUBLISH's and the dump changes on
  its own clock.
  **Redesigned** (Jacob, same session): the game downloads it, not the patcher: "the unity client is going to
  have to download this at play?  Yeah actually it should and it should override with whatever the server
  sends it every time."  So Ensemble fetches the dump at PLAY and the server's copy always wins over the one
  on the client; the stamp for Soundcheck goes.  Open: whether "the server" is the web folder over HTTP or
  Conductor over the game's own connection, and whether "every time" is a fresh download each PLAY or only
  when it changed.
  **Settled** (Jacob, same session): **Conductor sends it over UDP**, "broken into smaller packets obviously",
  and Ensemble keeps it in its own folder on the player's machine ("their userprefs folder").  **Every time
  the player first connects** they get it, with "a loading bar while they download the world before it puts
  them into the world".  New packets come of it; one Jacob named: **PlayerReady**, "verification from client
  it streamed the terrain and is good to display".  **The file is `simple_overworld.map`.**  **Written before
  connections are allowed** when it's missing ("yes should do this before we allow connections"), so a world
  made before it gets one.  **The terrain save more often**, "at like 2.5 minutes maybe I don't know... i dont
  know what numbers are gonna make this feel not like shit lol" (it was 15 minutes).  **No squeezing**: "no
  squeezing concern".
  **And** (Jacob, same session): the plan for Conductor's half is OKed ("yes build it").  **If the file can't
  be written, the door stays shut**.  **A client that can't get it tells the player** to "wipe their local
  copy and try again": "if the client can't get the file it needs to notify the person playing the game to
  delete the local map file or client and try again" (the session first took "the end user" for the admin
  and put the advice in Conductor's Error; it's the player's, Ensemble's half).  **The client
  keeps it in Unity's `Application.persistentDataPath`** (his "userprefs folder"), not
  `StreamingAssets/World/`.  **The download starts at PLAY**, by the same means the chunks will stream ("its
  going to have to be able to stream the chunk data anyways so we may as well use the same tool set").  **The
  character isn't spawned until PlayerReady**: "it doesn't show them or spawn them in the physical world until
  they're ready".  **The terrain save and the character save become one save**, every `world_save_seconds`, so
  the database and the chunk files hold the same moment; `save_minutes` goes.
  **Where it stands**: Conductor's file is written (session 1, `gameworld/src/overworld.rs`,
  SIMPLE_OVERWORLD_MAP.md, `design/world.md`), **not built by Jacob yet**.  What's left, a step each: the
  packets (the map in pieces over UDP at PLAY, PlayerReady, the character spawned on it; a protocol bump),
  then Ensemble's half (the loading bar, keeping it, drawing the distance), then writing it again at the
  world save once blocks change.  The heading says Soundcheck, but none of it is the patcher's any more.
  **Session 2, 2026-10-03** (Jacob): "A packets first in the server and then we'll in this conversation also
  integrate Ensemble with receipt of those packets."  The packets as planned and OKed: UserPressPlay loads the
  character as before but holds it, answered by **OverworldMapOffer** (`0x40`: size, pieces, SHA-256); the
  client pulls pieces with **OverworldMapRequest** (`0x41`, up to 64 at once, no ask number) and gets
  **OverworldMapPiece**s (`0x42`, 1024 bytes each) straight from the UDP thread; **PlayerReady** (`0x29`, an
  ask carrying the hash it has) puts the character in the world, answered by CharacterEnteredWorld.  Protocol
  version 11.  His answers: **a fresh download every PLAY** ("we're just gonna write over whatever the client
  already has every time"), no skipping on a matching hash; **the loading bar draws over character select's
  list** "and look like an enemy healthbar going backwards lol"; **`sha2` in networking**, OK; **a client that
  can't get the map goes back to the launcher with the message**.  Both halves written in session 2:
  Conductor's ran (Soundcheck logged in on version 11), and PLAY in the editor worked through the map to
  the world (Jacob: "it worked with the live client I just had to alunch in debug mode for soundcheck").
  The built game from before it got "The server didn't answer." and then, on a second PLAY, "Your
  character is on its way into the world.": it didn't know the offer, and a new PLAY while the character
  is held is refused until the session ends.  A fix was offered (a PLAY for the held character gets the
  offer again); the cooldown below overtook it.
  **A cooldown on the download** (Jacob, session 2, asked in answer to the stuck PLAY): "server puts a
  cooldown on an IP after it downloads and that IP must wait 5 minutes before it can attempt a download
  again".  Not built; being talked through: what counts as a download (the offer, or the map finished),
  what a PLAY inside the five minutes gets (every PLAY downloads, so a refusal blocks a quick log out and
  back in), and players sharing one address.  His answers: a PLAY inside it **is refused**, "You are
  temporarily cooling down from download for DDOS protection. You have <X> seconds remaining."; it starts
  **when the server sends the offer**; **by account** ("By Account I guess"), not by address; **a setting in
  `networking.cfg`**.  What it's for: "its more for DDOS protection I think".  On the stuck PLAY: "yes and
  no".  The plan OKed ("yes"), `map_cooldown_seconds` (0 to 3600, 0 off, 300), and the stuck-PLAY fix left
  out ("leave it out"): a second offer is a second download (`design/conductor-networking.md`).  Built
  and tested: the second PLAY got "... You have 199 seconds remaining."; four of its checks are still on
  TEST_CHECKLIST.html.
  **Left of the map**: drawing the distance from `SimpleOverworldMap.Current`; writing the map again at the
  world save once blocks change; a timing over the internet (the LAN's is 0.08 to 0.16 s for 16.8 MB; a
  guess of some 13 s at a 50 ms ping, since each 64 pieces waits a round trip), and asking for the next 64
  before the last are in if it drags; the stuck PLAY, with LOG OUT the way out.  **The chunks streamed** are
  the other half, and Jacob's pick for the next session ("we're gonna stream the chunks").
- **The chunks streamed** (session 3, 2026-10-03, being talked through): Conductor first, "prepare
  conductor for 'streaming' the world around the player in its chunk data and voxel data... we essentially
  want to copy minecraft."  The simple overworld map is "a 'broad outline'... we're gonna use to draw at a
  distance for the client"; the stream "is meant to give the high resolution details".  His answers:
  **the client pulls** (the map's way, the session's lean), the server checking each chunk is in the
  player's view and answering from a cache, keeping no list per player; **what comes before PlayerReady is
  the client's call**, "but I think we are gonna want to wait till most of the scene is filled"; **squeezed
  by hand** ("yes absolutely"), a chunk's kinds listed and its blocks as runs, and he asked whether zipping
  on the fly on top would make a difference (talked through below); **Ensemble gets its own sessions
  later** "to bring it in line with these server changes", so this session is Conductor and the test
  client.  He asked how far a player sees with `view_chunks` 4: 128 to 159 blocks ahead, by where in its
  chunk the player stands, every row up and down.  **Then** (Jacob): "runs only for now, keep view_chunks
  4, build it".  So no zip crate; each squeezed chunk starts with a byte saying how it's squeezed (1, runs),
  so zipping can come as a second kind without new packets.  The plan OKed: protocol version 12,
  ChunkRequest (`0x43`, up to 64 chunks), ChunkPiece (`0x44`) and ChunkRefused (`0x45`); GameWorld's thread
  squeezes each chunk as it loads it, into a cache networking sends from; a player waiting on the map or in
  the world may ask, for chunks within `view_chunks` of where their character stands; the test client's
  `--chunks`.  **Built and tested in session 3** (every check; 3,179 chunks, 88,746 bytes squeezed, 0.04 s
  on the LAN at `view_chunks` 8).  One change from the plan, found while
  writing it: the client didn't know where its character stands until after PlayerReady, so the
  OverworldMapOffer now ends with its x, y, z and the view.  Soundcheck's and Ensemble's version went to
  12 with it, and Ensemble reads (and skips) the offer's new end; nothing else of Ensemble's changed.
  `design/world.md` has the rest ("The chunks streamed").  **Left**: blocks on screen (Ensemble's asking,
  PlayerReady and reading the squeezed chunks are session 4's, `design/ensemble-networking.md`); a stamp on a chunk's pieces once blocks can
  change, so two versions' pieces never mix; forgetting squeezed chunks nobody's near once players move
  (GameWorld's copies; the GameClock's own ground follows the players since session 10, unbuilt); zipping, if busier ground ever calls for it
  (it has nothing to win today); a faked player's address getting that player flooded with chunks or the map's pieces
  (`design/conductor-networking.md`, "What's open").  `view_chunks`: Jacob keeps 8 in his `game.cfg`
  ("keeping as 8"); the default stays 4.  At the hand-off: "prepare next conversation for wiring up
  ensemble to receive the streams and the map and the hardest part - rendering it" (STATUS.md has what's
  to settle).
- **The spawn's height** (session 5): a block at x, y, z fills x to x+1 each way and a character's y is its
  feet, so a character at 0, 0, 0 stands in the GOLD, one block into Alpha's ground (the DIRT layer is y 0
  to 1).  Whether the spawn (and RESET HOME) should be y 1, or the server should put a character on top of
  whatever is under it, is Conductor's to settle.  Ensemble draws it where the server says.  **Jacob
  (session 6): "yes characters start at Y=1"**, and the drawing stays as it is ("I'll jsut set characters
  to stand on top of 0").  A plan for it (new characters and RESET HOME at 0, 1, 0) was **turned round**
  (Jacob, same session): "we shouldn't give them a fixed position to spawn at (I mean no Y=X) instead we
  need ot make a designated spawn point which for right now is only 0,0,0 but the code needs to be able to
  tell how many physical voxels (not air) are on top of 0,0,0 and then put the player on top of the
  highest voxel."  So a spawn point is a column (0, 0 today, more later), and the character stands on the
  highest block in it that isn't AIR, worked out from the ground as it is.  Open: when (new characters and
  RESET HOME only, or every PLAY), where it's worked out (the GameClock holds the chunks), and whether the
  character stands in the middle of the block (0.5, 0.5) or on its corner.  **His answers**: only **new
  characters and RESET HOME**; **the middle of the block**; and on where it's worked out, "Is this good
  though when they're not spawning at 0,0,0?  When we get to where the map is generating we're gonna fill
  it with spawn points so we want to be ready for that", so **GameWorld** works it out (any column in the
  world, its files or its region's ground), not the GameClock (only what's loaded).  Plan OKed ("Yes").
  **Built and tested** (session 6): `design/world.md`, "Spawn points".  What's left of it: the
  height from a column changed since the last world save is the old one, once anything changes blocks
  (the GameClock holds the newer chunk); and which spawn point a character gets, once there's more than
  one.
- **The world's files on the client**, overtaken (session 1): it was `Assets/StreamingAssets/World/`, a
  fifth folder of ours, for files the patcher would ship (Jacob, 2026-10-02).  The simple overworld map
  comes from Conductor at PLAY into `Application.persistentDataPath` instead, so nothing of the world ships
  with the client and no fifth folder is needed, unless something else wants one.
- **Conductor's half**, what's left of it: the manifest and the files come from the web folder, so Conductor
  sends nothing and serves nothing.  What could still be its: the client's report up after the check
  (the manifest in the protocol's own bytes, in pieces or past the 4,096-byte frame cap) and Conductor
  checking it against its own copy, so a changed client can't just skip the check; and `allow_debug_clients`
  (below).  Neither is started, and whether the report is worth building is open.
- **Debug mode's Conductor half**: `allow_debug_clients`, off by default; a client that says it skipped the
  check is let through to the Ticket only when it's on, else refused with words that say so.  Only means
  something once the report up (above) exists; today `--debug` skips the check on the client alone, and
  nothing on the server knows.
- **Conductor's `tls12` feature can go**: nothing speaks 1.2 any more (Soundcheck and `test_client.py` both
  insist on 1.3).  `networking/Cargo.toml`'s feature list and the comment in `tls.rs`; and `0.0.0.1` comes
  out of `client_versions` in `Content/cfg/networking.cfg` (Ensemble's old pre-release version; the Login's
  version is Soundcheck's now, `0.0.1`).  A small Conductor step.
- **The check runs once, at the start**: neither SUBMIT nor PLAY hashes the install again.  Hashing a
  Unity build takes seconds, so once is the plain way; a file changed while the launcher sits open isn't
  caught until the next start.  Open whether that matters.
- **The Remember Me file is readable by other users on the same Linux machine.**  It was Unity's .NET
  that couldn't set a file's permissions; the file is Soundcheck's now (`Security/RememberedLogin.cs`),
  whose .NET can (`File.SetUnixFileMode`).  Worth doing before players have it.
- **Shipping to a player on Windows** (Jacob, 2026-10-02: "get this all ready to ship to another person on
  Windows... make sure soundcheck is set up properly to validate off the host"): the check and the patch
  are built; then a Windows build of Ensemble (`Ensemble.exe`) and of Soundcheck, neither tried yet,
  published as the Windows half of the web folder; whether `Process.Start`, the environment hand-off and
  the rename-aside behave there; one package (below); the other person's server reachable
  (`bind_address`, the firewall on TCP 9997 and UDP 9998, `client_versions`); the web folder up at
  `opusensemble.duckdns.org:8553`.  The install has to be somewhere the player can write (not `Program
  Files`), and INSTALLATION_INSTRUCTIONS.md will say so.
- **A log file for Soundcheck**: on Windows a windowed program has no terminal, so `Log.cs` shows nothing
  there.  A file in the player folder, probably.
- **A new build of Ensemble as a patch**: with a file at a time from the web folder, a new build is just a
  big patch (hundreds of files).  Whether the first install is still a zip to download by hand, or the
  launcher alone with the game fetched by the patcher, is a call for the one-package step.
- **One package for a release**, Soundcheck and Ensemble together, with RELEASE.md's steps for PUBLISH in
  admin mode, both platforms, and the web folder served.
- **The certificate for every client**: LONGTERM_TODO.md.  Soundcheck is where it goes.

### The rest

- **The 0.0.1 code review** (`CODE_REVIEW_0.0.1.md`): the four bugs and the seven risks worth fixing before
  the tag are fixed, built and tested (2026-10-02, 365 tests).  What's left is R8 on, the inefficiencies and
  the cleanliness list, Jacob's to pick from; the stale words below are the words half of it.
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
- Ensemble has no way to find `Content/` yet, if it ever needs to.  Nothing has needed it: the server's
  certificate is Soundcheck's, beside the program.
- **The purchased art lives in Ensemble** (2026-10-01): `Assets/Purchased/` in the Unity project, ignored
  like the rest of `Assets/` but our four folders.  It isn't in the repo at all, LFS or not.  Whether
  `Content/Assets/` (ignored, empty) still has a use is open; nothing reads it.
- **Ensemble's project files in git**, still to look at together:
  - `Packages/` is ignored, so `Packages/manifest.json` (which packages the project uses) and
    `packages-lock.json` aren't committed.  A Unity project usually commits both, and the first package
    the client code needs (the Input System, say) makes it matter.
  - `.gitattributes` sends `.unity` scenes and `.anim` through LFS.  Both are text Unity can merge;
    asked on 2026-10-01, not answered yet.
  - Done (2026-10-02, Jacob's yes): the `.csproj`, `.sln` and `.slnx` are out of git and in the
    `.gitignore`, since Unity's rewrite of them on every compile stopped a checkout.
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
  - The start screen and character select are shipped layouts (`screen` `"start"` and
    `"character_select"`), built; only the HUD's layout is ever the player's.
- **Effects between screens** (Jacob, 2026-10-01: "cool ass effects if we can", "I don't know yet").
- **Stale words in the code**: done 2026-10-03 (the tabs' old names, "the Control Panel", the senders,
  the soft files, the Windows files, the test client's usage lines, and the rest of the list).  Left on
  purpose: `accounts.sql`'s header says a player gets a clear message (the admin makes accounts now),
  since the file is frozen.
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
