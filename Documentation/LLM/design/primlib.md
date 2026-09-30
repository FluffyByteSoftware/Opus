<!--
File:       Opus/Documentation/LLM/design/primlib.md
Component:  Documentation
Author:     Jacob Chacko
-->

# primlib

A lib crate: `Conductor/dev/primlib/`, the crate `conductor-primlib`, so code says `conductor_primlib::World`.
Jacob named it, 2026-09-30 ("prim for primitive").  It's the game library: the world's objects, the
components that make them what they are, and the templates and blueprints they're made from.  An ECS
(entity, component, system), written by hand.  Decisions here are Jacob's, from 2026-09-30.

## Where it stands

**Part one is Rust only**: the entities, the first components, the world that holds them, templates and
blueprints, and tests (`cargo test -p conductor-primlib`).  Built and tested on Linux, no warnings.  It
isn't a server piece itself: the GameClock (`design/gameclock.md`) owns a `World`, made fresh on every
START SERVER, and hands it to its five checks.  Nothing spawns anything into it yet.

**The character, Part A** (2026-09-30, written, **not built yet**): the first step of Jacob's map, "finishing
out character as a template for hydrating from an account".  Templates that take in other templates, the
`Living` and `Character` templates, `PlayerCharacter`, a GameObject remembering its templates, and saving
a GameObject as Lua text and making a character back from it.  See "The character and saving" below.
Part B, the `player_characters` table the text goes in, is in TODO.md under Protogame.

**Part two is the Lua**: templates and blueprints written as scripts under `Content/scripts/`, calling into
this crate.  See "Lua, part two" below.

## Skeleton

```
primlib/
├── Cargo.toml         no dependencies
└── src/
    ├── lib.rs         lists the pieces
    ├── entity.rs      Entity (a slot number and a generation); Entities, which hands them out and takes them back
    ├── store.rs       Store<T>: one kind of component, a slot per entity
    ├── components.rs  the components, each with its saved() and load(); Kind (which one, by name);
    │                    Component (one of them, with its value)
    ├── world.rs       World: the entities, a Store per kind, and each entity's templates; spawn, despawn,
    │                    add, remove, get, templates(), is()
    ├── template.rs    Template (NPC) and Blueprint (goblin_a); take_in(), templates(), is()
    ├── gameobject.rs  the built-in templates: living_template(), character_template(); character_from_save(),
    │                    check_living(); LIVING, CHARACTER
    └── save.rs        Save (templates and saved fields): of(), apply(), to_lua(); Fields; Value
```

## Decided

- **Written by hand**, no crate (`bevy_ecs` and `hecs` are both a dependency, and heavy on generics and
  macros).  One small generic, `Store<T>`, so each kind of component doesn't need its own copy of the same
  list code.
- **An entity is a number and a generation.**  The number is its slot.  When an entity is despawned its
  slot is reused, and the generation goes up by one, so an old handle to a dead goblin can't read the
  new goblin that took its slot.  Every call checks it.
- **A component is plain data**, one struct per kind.  There are eleven: Jacob's sample NPC, with Position,
  Rotation and Scale made one `Transform`, and what the client draws (a `Model`, a `PrimitiveShape` and an
  `Animator`; Jacob: "may need to divide our current components up more"):
  - `Transform`: a position, a rotation and a scale, each a `Vector3` (x, y, z as `f32`, what Unity uses).
    The scale starts at 1, 1, 1.  Jacob said it "holds the rotation and position of its parent", and the
    parent there is the object the component is on, not another object.  So no hierarchy: a transform is
    in the world's terms.
  - `Model`: a string, the path the client loads the model from ("in a previous iteration I tried using
    an enum but that got messy").  The server never opens it; it's the client's to make sense of.
  - `PrimitiveShape`: the shape the client draws if it can't draw the model ("cube, capsule, etc.").
    Unity's six built-in shapes: cube (the default), sphere, capsule, cylinder, plane, quad.
  - `Animator`: "controls animation state on the server".  A skeleton for now ("I'm not worried about
    animations in game yet"): `is_looping_currently`, and `current_track` ("idle", say), the name of what
    the model is doing, sent to the client.
  - `ShortName` ("goblin") and `LongName` ("goblin archer").
  - `Titles`: a list, with one of them picked as the current title ("the plucky").
  - `Health`, `Endurance` and `Mana`: the same shape, a `Pool` (current and max).  A pool made with just
    a max starts full: "No need to Health.Set it, it auto sets to max unless specified."  Damage stops at
    0 and healing at the max.
  - `PlayerCharacter`: the account's `id` and the `player_characters` row's `id`.  It says a player steers
    this GameObject, and it's how the game gets back to the account ("their account is what we track").
- **The world holds a `Store` per kind.**  Adding a new kind of component: in `components.rs`, the struct
  with its `saved()` and `load()`, a line in `Kind`, `Kind::ALL` and `name()`, a line in `Component`,
  `kind()`, `default_of()`, `saved()` and `load()`; in `world.rs`, its store, a line in each of the four
  matches (`add`, `remove`, `has`, `component`), and its two getters.
- **Template, blueprint, copy.**  Three layers:
  - The **template** (`NPC`) is a cheat sheet: components with their defaults, "so I don't write the
    same 50 lines in 50 npcs".
  - The **blueprint** (`goblin_a`) is "an actual NPC file", asked what a new goblin_a looks like.  It
    starts as a copy of its template's list, and can change a value, add a component the template
    doesn't have, and drop one it does.  "Components that are attached in a template could be removed
    in the set up of the actual goblin_a."
  - The **copies** are the goblins in the world.  `world.spawn(&goblin_a)` makes one; a hundred calls
    make a hundred, each with its own values.
- **A template is a starting set, not a contract.**  Anything can be added or removed after.
- **A rotation is three angles in degrees** (x, y, z), the way Unity's inspector shows a rotation and
  what a script would type (`0, 0, -90`).  Jacob: "whatever the standard is."  Unity itself keeps a
  rotation as a quaternion (four numbers that avoid the snags three angles have when two of the axes
  line up), but it turns three angles into one with `Quaternion.Euler(x, y, z)`, so Ensemble can take
  the angles as they are.  The server doesn't do any rotation math yet; if it ever has to (turning to
  face a player, say), the question comes back, and only `Rotation` changes.
- **Y is up**, as in Unity.
- **Saving the copies**: "when the goblin needs to be saved from memory it is saved to the database as a
  per instance item.  This can happen during shut down or rather when the server is being stopped."  So
  memory is what the game reads and writes while the server runs (not a row kept live, since the game loop
  never waits on the database), and on STOP SERVER every copy is written to the database as a row of its
  own, through Archivist.  START SERVER loads them back: "when the world is respawned the goblins will come
  back as if they never left".  Players' characters are saved too (`player_characters`, in TODO.md).  Not
  built yet.
- **Every copy has a UUID**: "you will be able to search NPCs by their UUIDs (which is unique to every
  instantiated one)".  From Fingerprinter (`new_uuid()`), like every row's.  The entity number is only
  good while the server runs; the UUID is the copy's name for good.  Built together with saving, not
  before: that's where it starts to matter, and it's when primlib first needs `conductor-tools`.
- **And an internal name**, made from the copy's `ShortName` with its spaces made `_`, then `_` and a
  number: "goblin" gives `goblin_1`, `goblin_2`, `goblin_3`, and "goblin archer" gives `goblin_archer_1`.
  A short name that already ends in a number just gets another: "goblin_1" gives `goblin_1_1`,
  `goblin_1_2`.  The count goes by the name, so two blueprints whose copies share a short name share it.
  Since the copies come back after a restart, their names come back with them, and the number carries on
  from the highest one in use, so there are never two `goblin_1`s.  Built with the UUID, together with
  saving.

## The character and saving

Jacob's answers, 2026-09-30, for the first step of his map.  Written, not built yet.

- **A template can take in another whole**: `Template::take_in()`.  "Its similar to inheritance in old
  discworld mudlib okay?  inherit STD_LIVING;"  The components come in at the other template's values, and
  whatever the template sets after wins.
- **Living is a "micro template"**: `ShortName`, `LongName`, `Health`, `Endurance`, `Mana`.  The three pools
  stay separate components, since "some objects may have health and no endurance... but all living objects
  will have all 3", and the names are in it because "all living objects will have to have a name.  Its a
  requirement."  `check_living()` holds a Living blueprint to that: all five, and a short name that isn't
  blank.
- **A Character is `Transform`, Living, `PrimitiveShape` (capsule) and `PlayerCharacter`.**  "Nothing else for
  now."  **A new one starts with 10 health, 10 endurance and 10 mana** (Jacob: "10 hp, 10 endurance, 10
  mana"), set in the Character template (`STARTING_POOLS`), not in Living."
- **A GameObject remembers the templates it came from**, its own first (`Character`, `Living`), so the game
  can ask `world.is(entity, "Living")`, the way Discworld's `living(ob)` did.  Blueprints carry the list, and
  the world keeps it per entity beside the stores (not a component: nothing adds or drops it after).
- **The save is the GameObject**: "our save needs to be the GameObject and all of its components, and the
  components settings", and its templates.  Saved as Lua text, one column on the row, with the name and last
  position as columns of their own (the mix, Jacob's yes).
- **What's saved is picked per field** ("I think we do it per field and attribute?  like we add this above
  in C# or something [SavedField]"), the plain way (Jacob: "plain way I'll get used to it either way"): a
  `saved()` right under each struct names the fields it keeps, and a `load()` reads them back.  A field not
  named isn't saved and keeps the template's value.  Today:
  - `Transform`: `position`, `rotation` and `scale` ("Transform should probably hold scale as well").
  - `ShortName`, `LongName`: `text`.
  - `Titles`: `list`, and `picked` counted from 1 (0 is none).
  - `Health`, `Endurance`, `Mana`: `current` and `max`.
  - `Model`, `PrimitiveShape`, `Animator`: nothing (the template's, and what a model was doing doesn't
    outlast a logout).  `PlayerCharacter`: nothing, since it's put back from the row.
- **The text** is a script that hands back one table:
  ```lua
  return {
    templates = { "Character", "Living" },
    components = {
      Transform = { position = { x = 12.5, y = 0, z = -7.25 }, rotation = { x = 0, y = 90, z = 0 },
                    scale = { x = 1, y = 1, z = 1 } },
      ShortName = { text = "jacob" },
      Health = { current = 150, max = 200 },
    },
  }
  ```
  `lua-parser`'s `read_save()` runs it in the locked-down Lua and hands back a `Save`.  Anything in it that
  isn't data (a function) or is the wrong shape turns the save away; a component name the game has dropped
  is a Warn and the rest loads.
- **Hydrating**: `character_from_save()` starts from the Character template, lays the save over it, then
  puts the `PlayerCharacter` on from the row.  A component added to Character later turns up on old saves at
  its default.
- Open: when a character is saved (leaving the world, STOP SERVER, every so often), for spawning.

## Lua, part two

Jacob's picture for when templates and blueprints are scripts:

- A template packs its components and their defaults into a `setup()`.
- An object has an `awake()`, "called right before its actually instantiated", where the object's own
  code takes away a component the template gave it and adds more.

Open for then: whether `setup()` runs once when the blueprint is read or on every copy (the Rust side
today builds the blueprint once and copies it); whether `awake()` runs on every copy (like Unity's
`Awake`, so each goblin can come out a bit different); where the files live under `Content/scripts/`.

## Open

- **Saving**: whether anything is saved between stops, so a crash doesn't lose the lot; the table's shape.
- **The internal name**: what a copy with no `ShortName` is called; whether the name and the UUID are a
  component or something every entity has "baked".
- **Which blueprint a copy came from.**  The spawn system (in TODO.md) needs to count the goblin_as, so a
  copy will need to know its blueprint.  Whether that's a component or "baked" (the question TODO.md
  already has for characters) waits on the spawn system.
- **Behaviour**: in the components (the Unity way) or in systems that run over every entity with a given
  set of components.  The GameClock's five checks are the place for the second; waits on the Lua.
