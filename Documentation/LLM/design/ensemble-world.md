<!--
File:       Opus/Documentation/LLM/design/ensemble-world.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Ensemble -- the world on screen

Started 2026-10-03, session 5.  Jacob: "next step is to render the voxels in Ensemble to the player".  The chunks
come in as `design/ensemble-networking.md` says ("The chunks, Ensemble's half") and sit in the Ground
(`Code/World/Ground.cs`); this file is how they're drawn, and the camera that looks at them.

## Settled (Jacob, 2026-10-03, session 5)

- **The chunks only**: the distance from the simple overworld map is a session of its own.
- **Minecraft's and 7 Days to Die's way at the edges**: a chunk is drawn only once all six chunks round it
  are in (above row 10 counts as air, below row 0 as nothing), so the outermost ring of the view is never
  drawn: at `view_chunks` 8 the player sees 7 out.  The other shape was the edge drawn as walls, a cut-out
  diorama, with chunks drawn again as their neighbours come.
- **A mesh a chunk, faces only between a solid block and air**; an all-air chunk has none.  The arrays
  built on a worker thread, Unity's Mesh filled on the main thread.  A line in the Console says how many,
  how many faces and how long (a guess at the cost is no use; the line is the measurement).
- **The materials are Jacob's**: "we'll make the materials on my end -- for now they're just colors".  A
  block kind is a submesh with its own material, from a slot each in the Inspector.  No shader of ours.
- **The camera**: "we're gonna be using a cinemachine following camera in the style of zomboid except a bit
  more direct".  Project Zomboid's is a fixed, angled view from above that follows the character.
  **Fixed**: "I'm going for the zomboid style where you can walk towards the camera but the camera only
  zooms in and out.  It doesn't swivel or rotate."  Cinemachine (3, already in the project) is set up by
  Jacob in the editor, following a **stand-in** the code puts where the character stands: a box 1 block
  wide and 2 tall, its feet at the server's y.  The other shape was the code building the camera rig.
- **The slots**: one material each for DIRT, STONE, WOOD, GOLD and BEDROCK.  An empty slot's faces aren't
  drawn, and the Console says so once.  (The stand-in was to have a slot too; as written it's a Cube in the
  scene, so its material is the Cube's own.)
- **Where the character stands**: a block at x, y, z fills x to x+1 each way, and the character's y is its
  feet, so a character at 0, 0, 0 stands in the GOLD, a block into Alpha's ground.  Drawn as it is; the
  spawn's height is a Conductor question in TODO.md.

## Built and working (Jacob, 2026-10-03: "THE GROUND WORKED")

**Measured** (Jacob, Linux, in the editor, a view of 8 at `world_size` 32): "World: 437 chunks meshed,
246903 faces, in 0.51 s (the worker's share 0.51 s, 1.16 ms a chunk); 302 chunks drawn in all."  The 135
meshed and not drawn had no faces: buried stone and BEDROCK with no air beside them.  All of the 0.51 s is
on the worker; the main thread only fills Unity's Mesh, 32 a frame.

## As written (2026-10-03, session 5)

- **`Code/World/ChunkMesher.cs`**: a chunk and the six round it into lists (`ChunkMesh`): four corners and
  a normal a face, two triangles, kept by block kind.  A face wherever a block that isn't air meets air;
  over the top row is air, under the bottom row solid.  Plain C# but for `Vector3`, so it runs on a
  worker.  Times itself.
- **`Scripts/World/GroundView.cs`** (a new folder under `Scripts/`): the five material slots and
  Meshes Per Frame (32).  It hears `Ground.Added` and sends a chunk to its worker thread ("Ground mesher")
  once it and the six round it are in; an all-air chunk is never sent.  `Update()` makes Unity's Mesh from
  up to 32 results a frame (32-bit indices only past 65,535 corners), a GameObject a chunk under
  GroundView, named "Chunk x,z row r", at its corner in blocks.  `Ground.Cleared` throws it all away,
  and a result for the old Ground is dropped when it comes back.  The Console's line when the worker runs
  out: "World: N chunks meshed, F faces, in S s (the worker's share W s, M ms a chunk); D chunks drawn in
  all."
- **`Scripts/World/CharacterStandIn.cs`**: on a Cube in the scene, scaled 1 by 2 by 1.  Hidden until
  `Session.ReachedWorld`, then moved so its bottom is at the character's feet (`Session.Standing`, new,
  from CharacterEnteredWorld's x, y, z); hidden again at `SessionOver`.  The Cinemachine camera follows it.
- **`Ground`** has `Added` and `Cleared`.

## PlayerReady once the ground is drawn (2026-10-03, session 6; built and tested)

Jacob, after the ground worked: "can we make so the world starts 'loading in chunks' before the player is
drawn?  We need to sync this to the server too so that the server knows when the player client is ready".
Today PlayerReady goes once the nearest 99 chunks have *arrived* (`Session.NearGroundIn()`), and meshing
runs after, so the character can be put in the world, and the stand-in shown, before the ground round it
is on screen.  The server already waits for PlayerReady before it puts the character in, with no deadline
but the UDP timeout.  Open: what "ready" is (the nearest 99 drawn, or the whole view), whether the server
needs more than PlayerReady at a later moment, and what the loading bar says meanwhile.

**His answers**: ready is **the nearest 99 visible**: "I just don't want a situation where a player walks
forward and 'falls' until the server catches up.  I'd rather make them wait and have a more seamless
world loaded.  Hell we can give them a loading screen."  **PlayerReady at that later moment is enough**;
the server seeing the loading's progress is "over kill but I like where your head is at".  **The loading
bar stays up through the drawing**, saying so.  **If it isn't drawn 10 s after it's in** (a near chunk
refused, so the ones beside it can't be meshed either, or no GroundView in the scene): "kick them back to
the main screen and state: Your connection may be to slow or the server is unresponsive.  Please try
again.  If this happens repeatedly please talk to the admin."  (Written "too slow" on screen.)

**As built** (session 6; Jacob's Console showed "in.  Drawing them." then "drawn.  PlayerReady." before
the character came in): `Session.NearGroundIn()` no longer sends PlayerReady; it
starts the 10 s wait.  GroundView calls `Session.ChunkShown(place)` for every chunk it's done with: drawn,
meshed to nothing, or all air (never sent to the worker).  Session counts the nearest 99 among them
(`GroundDrawn`, the column from `GameConnection.FetchChunks()`, `ChunkDownload.NearColumns`), and
`ReadyIfDrawn()` sends PlayerReady once they're in and drawn; the wait running out is `DrawWaitOver()`,
`Finish()` with his words.  The bar: "Loading the ground... N of 99", then "Drawing the ground... N of
99", then "The ground is drawn.  Entering the world...".  No packet or Conductor change: the server
already waits on PlayerReady.

## Where the ground sits in Unity (2026-10-03, session 6; settled)

Jacob, after the ground worked: "it loaded the whole damn world with the split perfectly" (Alpha flat to
the west, Omega's hills to the east; a grass material in place of GOLD for now), and "we need to origin
this at -1 on Y I think so characters I put down at 0 are on top of it".  Today a block at y fills y to
y+1, so the ground's top at 0 is at Unity's y 1, and anything stood at 0 is a block deep in it.  Open: a
client-only shift (the whole ground drawn a block lower, so a block's top face is at its own y) or the
server's spawn at y 1 with the drawing as it is; and which the server's own idea of "standing on" follows.
**His answer**: the drawing stays as it is: "actually I'll jsut set characters to stand on top of 0 that
seems easier".  So a block at y fills y to y+1, and a character standing on the ground at 0 is at y 1.

## The player in the world (session 9, 2026-10-03; built and tested)

Jacob, opening it: "we're gonna be preparing both the server (Conductor) and Ensemble (client) with
representing the player in the world.  This will be necessary to get movement set up next session."  And
the rule it's built on: **"The server will be the authority, always on where the object actually is in the
world.  The client is just a dumb renderer."**  So the client draws a character where the server last said
it is, its own character included, and never decides a position of its own.

Put to him: whether every character in the world is drawn or only the player's own; how far a player sees
others (everybody, or the chunks' view); how the server tells the client (a snapshot every cycle, or only
what changed); what the client draws a character with (a model in a slot, or a model a name); facing and
names over heads.

**His answers** (session 9):
- **Who**: "Every character in the world, if they're within your visible range", the player's own included.
- **How far**: the chunks' view, `view_chunks` of them each way of the player's own column (8 in his
  `game.cfg`), so nobody is shown standing on ground the client hasn't got.
- **How the server tells**: **only what changed**, not a snapshot every cycle: "so the client knows
  character Chacko is coming into view heading towards 3, 1, 3 at 1 voxel per second or something like
  that.  It can render that and then if it updates the server can just notify the client?"  He wondered
  whether the GameClock would have to check every 50 ms "to keep animations smooth".  (The other shape
  was the whole view every 250 ms cycle, a lost packet mended by the next.)
- **What's drawn**: "I think we may as well define an Actor in the client as well?"  And: "we need to come
  up with a way to like send a simple datagram that the client can use to hydrate an actor with or an
  inanimate game object".  So one packet describes any object the client is to draw, from its components,
  and the client builds an Actor (or a plain object) from it.
- **Facing**: "full rotation if possible", the `Transform`'s whole rotation.
- **Over the head**: the short name.

Then put to him: how a lost "came into view" or "left" is mended; a heading and speed, or a velocity; what
an Actor is on the client; how a model's name finds a model.

**His answers** (session 9, the second round):
- **A lost packet**: **a roll call once a second**, every object in view with its number and its movement
  as it is now; the client drops what isn't in it and asks about a number it doesn't know.  "Or we could
  consider redesigning the UDP service to have "reliable ordered" packets?"  That one's in TODO.md, its
  own feature.  (The other shape was the client confirming each packet and the server sending again.)
- **Movement**: "WASD movement", so **a velocity**, blocks a second along x, y and z.  (The other shape
  was a destination and a speed, which suits click-to-move.)
- **The hydrate packet**: "the transform information, fallback enum shape, and animation name", and the
  rest as put to him: an **Actor** on the client for anything Living, with its short name over its head;
  a plain world object for everything else.  For the model: "we're gonna have two ways of sending it:
  UUID and a string literal to the relative path in the client's data directory that should be with the
  executable?  Or relative to the executable's path?"  And: "we may have to build a plug in together to
  help visualize this so I don't get confused as the dev who doesn't speak UUID - but something that might
  show the UUID to which model in a browser."  Being talked through: what a path can mean to Unity.
- **The list of models in the Inspector** (a name and a prefab each, the fallback shape for a name not in
  it): "sounds good".  **The Character template's model is named "Actor"**: "Actor not character or human".
- **What a path can mean to Unity** (put to him): a built game can't load a model from a loose file, so a
  path is either a name in a list in the Inspector, a path under a `Resources` folder (both inside the
  build), or an asset bundle beside the executable (patchable by Soundcheck, needs bundles built).  **His
  answer: "okay then its going to be UUID matched."**  So the server's `Model` carries a model's UUID, and
  the client finds the model by it; the viewer he asked for shows which UUID is which model.
- **Then, the other way round** (put to him: Unity's own ID or one we make; a window in Unity or a tab on
  the web admin; this session or its own): "we're gonna have to do this in a backwards way... I will build
  an actor in the client and then we'll make a plug in to dump it into some sort of data that the server
  can take in and use?  That way it doesn't assign a UUID but the client does and we'll just start with
  0000001 and work up from there type deal?"  So **a model is a number**, handed out by the plugin from 1
  up as Jacob makes each prefab in Unity, and the plugin writes what the server needs into a file it
  reads.  0 is no model: the fallback shape.  (A number in a catalogue, not a row; if models ever go in
  the database, the table gets its `id` and `uuid` like any other.)
- **Keeping track** ("Two ways"): "help me keep track with a living document", a file in the docs listing
  every model's number and what it is, kept up to date by the session; "and by clicking the prefab in
  Unity and looking at its script for "FluffyGameObject" :)", a script on every prefab that shows its
  number in the Inspector.
- **When**: "we'll do the model draw next session".  This one draws every object as its fallback shape,
  with the packets as they'll stay.
- **The database, maybe** (Jacob, on the `id` and `uuid` rule): "The ID however is relative to the
  insertion point in the table, the UUID is the games unique identifier for a thing.  We actually may want
  to store this animation data in the database and not on a catalog file on disk?"  For the model draw's
  session.  Whichever it is, the game names a model by its uuid, so **the Hydrate carries the model's uuid
  as a string, empty for none** (the fallback shape), and the packet doesn't change when models come.
- **FluffyGameObject** (yes): "every single drawn object to the client is going to need a FluffyGameObject
  now.  It's going to define how to represent it in the client I think."  Actor goes on top of it for
  anything Living.
- **The OK** (session 9): round one, Conductor and the test client, then Ensemble.

### As written, round one: Conductor and the test client (session 9; built and tested)

Protocol version 14.  PROTOCOL.md's "The world's objects" has the packets byte for byte; `design/gameclock.md`
("The view") and `design/conductor-networking.md` ("The world's objects") the server's half.  In short:

- **Each object has a number** (from 1, never 0), handed out by `conductor_gameclock::enter()` and sent at the
  end of CharacterEnteredWorld, so the client knows which object is its own before any Hydrate comes.
- **Once a cycle the GameClock's broadcast** works out, for each player in the world, what their client is to
  be told about the objects within `view_chunks` of their column: a **Hydrate** for each one it hasn't had
  whole (number, uuid, Living, short name, position, rotation, velocity, scale, model's uuid, fallback shape,
  what it's doing), **ObjectsMoved** for the ones that moved, **ObjectsGone** for the ones that left, and once a
  second a **RollCall** of everything it's believed to have.  A number on a roll call the client doesn't know,
  it asks about with an **ObjectAsk**, and the next cycle answers.
- **Today**: the objects are players' characters, a capsule each with no model, never moving (velocity 0).
- **The test client** prints all of it, asks about what it doesn't know, and with `--miss-first-hydrate`
  drops one to see the roll call mend it.

Built and tested: every check passed (session 9).

### As written, round two: Ensemble (session 9; built and tested)

- **`Code/World/WorldObjects.cs`**: what the client knows, by number (`WorldObject`, `ObjectMotion`).  A Hydrate
  is `Put()`, ObjectsMoved `Move()`, ObjectsGone `Remove()`; `RollCallPiece()` puts a roll call's pieces
  together and, once they're all in, drops what isn't on it, moves what is, and hands back the numbers it
  doesn't know, which Session asks about (`GameConnection.AskAbout()`, an ObjectAsk).  `Added`, `Moved`,
  `Removed`, `Cleared` for the screen.  The Console says each object coming into view and going.
- **`Session.OwnObject`**, from the end of CharacterEnteredWorld; the session ending clears it all.
- **`Scripts/World/WorldObjectsView.cs`**, on an empty GameObject in the scene: a GameObject under it for each
  object, "Object N (Name)" (", yours" for the player's own), with a **FluffyGameObject** (its number, uuid and
  what it's doing in the Inspector; `Place()` puts its feet at the position, turned to the rotation, and its
  `Update()` moves it along the velocity), the fallback shape as a child with its bottom at the feet and no
  collider, and an **Actor** for anything Living.  Slots: Name Font (Unity's own when empty), Name Size (a
  capital's height in blocks, 0.3) and Name Gap (0.25 over the shape).  A model's uuid is noted once in the
  Console and the shape drawn, until the model draw.
- **`Scripts/World/Actor.cs`**: the short name over the head, a TextMesh (Unity's own, no package) turned to
  face the camera every frame.  Its size is a guess until seen.
- **`Scripts/World/CameraAnchor.cs`**, `CharacterStandIn.cs` renamed with its `.meta`, so the Cube in the scene
  keeps it: never drawn (its renderers off for good), at CharacterEnteredWorld's spot at first and then on the
  player's own object every frame, its middle as high over the feet as before, so the Cinemachine camera
  frames the same.

Built and tested (session 9): Tester, Chatter and Asdf seen as capsules with their names over them, coming
and going; the names read well in the Game window ("they look fine actually :D"; the Scene window shows
them tilted, since they face the game's camera).


## Movement (session 10, 2026-10-03; being talked through)

0.0.2 on WAYPOINTS.md.  Jacob, opening it: "synchronizing movement across clients with Conductor being the
central authority on where a unit is at any given moment".  It's the one he "failed at last time" on an
earlier server of his.

Put to him: what went wrong last time; whether his own character waits for the server or moves at once;
the controls; the ground (stepping up, falling, walls, jumping); the speed; the edge of the loaded ground;
characters bumping into each other.

**His answers** (session 10, the first round):
- **Last time**: "I kept having janky ass movement where the client and server kept fighting about your
  position (you basically kept rubber banding)".  So not rubber banding is the thing this is built
  around.
- **His own character moves at once, EverQuest's way**: "we're gonna simulate the way EverQuest handles
  movement where the client like has local authority and the server kinda just periodically validates the
  client movement and pulls it backward if it doesn't match to the last known good spots.  I had that
  working but the rubberbanding happened to much and we never did get it straight  I got so frustrated I
  deleted the entire thing lol".  (The other shape was the client sending keys and the server doing all
  the moving, a press waiting up to a cycle and a round trip before anything moved.)  This turns round
  session 9's "the client is just a dumb renderer" for the player's own character: the server still has
  the last word (it takes a move or pulls the character back), but the client says where it went.
  Everybody else's character is still drawn where the server says.
- **The controls, Project Zomboid's way**: "your playing in 3rd person this game... when you hold D it
  will first turn your character to the right on the screen and then the longer you hold it the more they
  start walking in that direction until they are walking forward in that direction same with S your
  character will turn around and then start walking towards the camera."  So the keys are directions on
  the screen, not turns: the character turns to face the way the key points and walks faster the more it
  faces it.
- **A player anchor in the scene**: "I'll need to set up a player anchor in the scene for this so that
  program wise you're generating the Player Actor game object with all its mesh data, and our anchor has
  all its local scripts.  The player controller script would go on this object I think.  Then the prefab
  is just a skin we make the shell wear?"
- **The ground**: walking follows it, a step up of one block happens on its own, a character falls off an
  edge, and a wall two blocks high stops it.  "yes all of that we can do jumping later".  Jumping is in
  TODO.md.
- **The speed**: "we'll start with 4 blocks per second".
- **The edge of the loaded ground**: all of it now ("b"): the server loading the ground around each player
  as they walk, and the client pulling new chunks as it walks and letting the far ones go.
- **Colliders**: "the prefabs should have a capsule collider and the voxels should have a cube collider".

Then put to him: the rule change for his own character; how fast the character turns, and how the walk
grows as it faces the key's way; whether the camera turns; a mesh collider on each chunk or a box a block;
characters bumping on the screen but not on the server; how forgiving the server is.

**His answers** (session 10, the second round):
- **The rule change**: yes.  And how he sees it working: "the server checks in periodically if they're
  moving in the same direction as the last check in.  The second their input changes (which the server
  should detect because it should be ingesting input on every 50 ms cycle) that would fire an event that
  notifies the Conductor that there is a directional change which is how it can than see which characters
  are going to need a network update about this character changing its direction or rotation or something
  else.  That _should_ work."  So the client speaks when its movement changes, and checks in now and then
  while it doesn't; the server passes a change on to whoever sees the character.
- **The turn**: "make it a value we can adjust in player.cfg (a new config file)".
- **The camera**: "the camera stays fixed for now.  The only further improvement I might do is allowing you
  to go first person."  First person is in TODO.md.
- **The ground's colliders**: "we can probably make a mesh collider around the chunks I mean.  Only an
  individual voxel would need a box if its been broken from its chunk."  A broken-off block's box is in
  TODO.md, with breaking blocks.
- **Colliders in the game library**: "we're gonna design in our game library collider primitives
  capsule/cylinder, and cube that should cover our needs for this.  This will have to be our
  representative of the player in the servers memory.  Is it not possible to make it so a player blocks
  another player up front then?"
- **How forgiving**: "we're gonna make this configurable in game.cfg I think...
  MOVEMENT_TOLERANCE_BLOCK_THRESHHOLD or something... for now let's default it to 16 until we get a feel".
  (The other shape was a quarter over 4 blocks a second and up to a second of allowance saved up.)

Then put to him: whether the input check runs once a cycle or every 50 ms; which side `player.cfg` is on
and whether the walk speed goes in it; how often a check-in comes while walking straight; whether
characters block each other on the server; the character's capsule's size.

**His answers** (session 10, the third round):
- **Blocking**: a character standing still blocks others on the server too, a moving one only on the
  screens ("a").  "It would also force NPCs to recalculate how to walk somewhere."  (The other shape was
  always blocking on the server, with two players walking into each other each pulled back.)
- **The input check**: "I don't know what do professional games do for input cycles?  50 might be more
  aggressive than the human can respond... maybe every second 50 ms check?"  Answered in the reply that
  followed (below, "The input check").
- **`player.cfg` is the server's**: "server player.cfg file that holds the turn speed in it.  Clients
  should never determine this.  They can cheat by hacking it but the server will force them back."
- **Warnings for the admin**: "if your rotation is more than 90 deg off that should flag a warning to the
  sys admin along with the being 1-16 blocks past a point expected to be at."
- **Check-ins**: "we'll start at half a second.  This is probably going to be hard coded so we'll see
  when we get passed my own client."
- **The capsule**: "no, our current shape", 1 block wide and 2 tall.  (Put to him: Minecraft's 0.6 by
  1.8, since 1 wide fills a one-block doorway and 2 tall only just fits a two-block gap.)
- **The plan for round one**: "Let me know if you need more help".

### The input check, answered (session 10)

Jacob asked what professional games do for input, "maybe every second 50 ms check?".  Put to him: shooters
run their whole simulation 64 or 128 times a second, and MMOs a lot slower, with what others see of a player
sent a handful of times a second and smoothed between (a guess from what's generally known, not measured or
looked up); and here the player walking sees their own move at once on their own screen, so the check only
decides how soon everybody else hears of it.  Taking moves in every 50 ms only helps if they also go out
every 50 ms, five times the packets.  So **once a cycle**, in the input check, as built; one line to change
if he says otherwise.

### As written, round one: Conductor and the test client (session 10; not yet built)

Protocol version 15.  PROTOCOL.md ("Movement") has the packets; `design/gameclock.md` ("Movement") the
judging and the ground following the players; `design/conductor-networking.md` ("Movement") networking's
half; `design/primlib.md` the `Collider` and the `Transform`'s velocity.  In short:

- **PlayerMoved** (client to server): the move's number, the last pull-back had, position, rotation and
  velocity.  **MoveCorrection** (server to client): the pull-back's number, position and rotation.
- **The check**, against the time since the last good spot: 4 blocks a second along the ground, a block of
  slack silent, up to 16 more taken with a Warn, past that pulled back with a Warn; a turn more than 90
  degrees past `player.cfg`'s 450 a second pulled back with a Warn; inside a block, on ground the server
  hasn't got, into a character standing still, or hanging in the air, pulled back quietly.
- **The player's own character** isn't in their ObjectsMoved; only a MoveCorrection moves it.
- **The collider** goes in the Hydrate; the Character's is a capsule 0.5 round and 2 tall.
- **CharacterEnteredWorld** ends with the walk (4) and the turn (450).
- **The server's ground follows the players**, 2 chunks each way of each; the chunks a player may ask for
  follow their character, one more each way than the view.
- **Ensemble and Soundcheck** read version 15 (the Hydrate's collider and the walk and turn are read and
  kept; nothing walks yet), so the game goes on working against the new server until round two.
- **The test client**: `--walk SECONDS` walks west over Alpha's flat ground, a move when it sets off, every
  half second and when it stops; `--jump BLOCKS` and `--spin` are pulled back on purpose.

Round two is Ensemble: the player anchor and its controller (Zomboid's turning, a CharacterController, 4
blocks a second, falling), the chunks' mesh colliders, capsules from the Hydrate, sending moves and taking
pull-backs, everybody else gliding.  Round three is Ensemble pulling the ground as it walks.

**His answers** (session 10, the fourth round, on round one's questions):
- **The walk stays in code**: "no walking will be fixed because we're going to have movement speed
  modification abilities (potions, spells, enchantments)".  So 4 blocks a second is every character's
  base, and a character's own speed, modified, is to come (TODO.md, "Movement speed modifiers").
- **A Warn a character a minute**: "ok".
- **The server's ground**: "yeah just what terrain is necessary".  And for later: "we are going to
  eventually do a couple things that I will need to discuss next session with you... about possibly
  trimming down the number of voxels per column because we want the EQ Next style voxels not Minecraft
  really but that's for next iteration".  In TODO.md.
- **The input check**: "What does EQ do?"  Looked up in EQEmu, the open-source EverQuest server, which
  speaks the real client's packets (`zone/client_packet.cpp`, `Handle_OP_ClientUpdate`;
  `zone/cheat_manager.cpp`, read on GitHub in session 10): the client sends an OP_ClientUpdate with its
  position, heading, its deltas (a velocity) and its animation; the server takes it **the moment it
  arrives**, not on a tick, makes it the player's position as sent, and passes it straight on to every
  client in range (and the group, wherever they are), only when something changed.  Its cheat check adds up
  the distance across updates and judges the average speed over at least 2.5 seconds against the
  character's run speed: over it is logged as a possible warp (a "light" one, or a large one past one and
  a half times), with exemptions for a knockback, Shadow Step or a port.  It **logs and doesn't pull
  back**.  (What the real EverQuest servers did isn't public; EQEmu is the nearest.)  Put to him: ours
  takes the moves once a 250 ms cycle and passes them on in the broadcast, so others hear up to a cycle
  later than EQ's would; the check is EQ's in shape (distance against speed over time, with slack) but
  pulls back, as he asked.
