<!--
File:       Opus/Documentation/LLM/design/primlib.md
Component:  Documentation
Author:     Jacob Chacko
-->

# primlib

A lib crate: `Conductor/dev/primlib/`, the crate `conductor-primlib`, so code says `conductor_primlib::World`.
Jacob named it, 2026-09-30 ("prim for primitive").  It's the game library: the world's objects, the
components that make them what they are, and the templates and blueprints they're made from.  An ECS
(entity, component, system), written by hand.

## Where it stands

The first part is Rust only (Jacob, 2026-09-30): the entities, the first components, the world that
holds them, templates and blueprints, and tests.  Written, built and tested on Linux on 2026-09-30:
no warnings, all 23 tests pass.  Nothing calls it yet.  It isn't a server piece yet
either, since there's no game loop to run it; it goes in `start_server()` and `stop_server()` when there
is one.

**Part 2 is the Lua** (Jacob, 2026-09-30): templates and blueprints written as scripts under
`Content/scripts/`, calling into this crate.  See "Lua, part 2" below.

## Skeleton

```
primlib/
├── Cargo.toml         no dependencies
└── src/
    ├── lib.rs         lists the pieces
    ├── entity.rs      Entity (a slot number and a generation); Entities, which hands them out and takes them back
    ├── store.rs       Store<T>: one kind of component, a slot per entity
    ├── components.rs  the components; Kind (which one, by name); Component (one of them, with its value)
    ├── world.rs       World: the entities and a Store per kind; spawn, despawn, add, remove, get
    └── template.rs    Template (NPC) and Blueprint (goblin_a)
```

## Decided

- **Written by hand**, no crate (`bevy_ecs` and `hecs` were the choices; both are a dependency, and
  heavy on generics and macros).  One small generic, `Store<T>`, so each kind of component doesn't
  need its own copy of the same list code.
- **An entity is a number and a generation.**  The number is its slot.  When an entity is despawned its
  slot is reused, and the generation goes up by one, so an old handle to a dead goblin can't read the
  new goblin that took its slot.  Every call checks it.
- **A component is plain data**, one struct per kind.  The first ten are Jacob's sample NPC, with
  Position, Rotation and Scale made one `Transform` (Jacob, 2026-09-30), and what the client draws, a
  `Model`, a `PrimitiveShape` and an `Animator`:
  - `Transform`: a position, a rotation and a scale, each a `Vector3` (x, y, z as `f32`, what Unity
    uses).  The scale starts at 1, 1, 1.  No parent yet.
  - `Model`: the path the client loads the model from.
  - `PrimitiveShape`: cube (the default), sphere, capsule, cylinder, plane or quad.
  - `Animator`: `current_track` ("idle") and `is_looping_currently`.  A skeleton.
  - `ShortName` ("goblin") and `LongName` ("goblin archer").
  - `Titles`: a list, with one of them picked as the current title ("the plucky").
  - `Health`, `Endurance` and `Mana`: the same shape, a `Pool` (current and max).  A pool made with just
    a max starts full: "No need to Health.Set it, it auto sets to max unless specified."  Damage stops at
    0 and healing at the max.
- **The world holds a `Store` per kind.**  Adding a new kind of component is a struct in
  `components.rs`, a line in `Kind` and `Component`, a field in `World`, and its two getters.
- **Template, blueprint, copy** (Jacob, 2026-09-30).  Three layers:
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

## Lua, part 2

Jacob's picture, 2026-09-30, for when templates and blueprints are scripts:

- A template packs its components and their defaults into a `setup()`.
- An object has an `awake()`, "called right before its actually instantiated", where the object's own
  code takes away a component the template gave it and adds more.

Open for then: whether `setup()` runs once when the blueprint is read or on every copy (the Rust side
today builds the blueprint once and copies it); whether `awake()` runs on every copy (like Unity's
`Awake`, so each goblin can come out a bit different); where the files live under `Content/scripts/`.

## Open

- **Saving the copies.**  Decided (Jacob, 2026-09-30): "when the goblin needs to be saved from memory
  it is saved to the database as a per instance item.  This can happen during shut down or rather when
  the server is being stopped."  So memory is what the game reads and writes while the server runs,
  and on STOP SERVER every copy is written to the database as a row of its own, through Archivist.
  (His first thought, a row managed live, ran into the game loop never waiting on the database.)
  START SERVER loads them back: "when the world is respawned the goblins will come back as if they
  never left" (Jacob, 2026-09-30).  Open: whether anything is saved between stops, so a crash doesn't
  lose the lot; the table's shape.  Players'
  characters are saved too (`player_characters`, in TODO.md).
- **Every copy has a UUID** (Jacob, 2026-09-30): "you will be able to search NPCs by their UUIDs (which
  is unique to every instantiated one)".  From Fingerprinter (`new_uuid()`), like every row's.  The
  entity number is only good while the server runs; the UUID is the copy's name for good.  Built
  together with saving (Jacob, 2026-09-30), not before: that's where it starts to matter, and it's
  when primlib first needs `conductor-tools`.
- **And an internal name**, made from the copy's `ShortName` with its spaces made `_`, then `_` and a
  number (Jacob, 2026-09-30): "goblin" gives `goblin_1`, `goblin_2`, `goblin_3`, and "goblin archer"
  gives `goblin_archer_1`.  A short name that already ends in a number just gets another:
  "goblin_1" gives `goblin_1_1`, `goblin_1_2`.  The count goes by the name, so two blueprints whose
  copies share a short name share it.  Since the copies come back after a restart, their names come
  back with them, and the number carries on from the highest one in use, so there are never two
  `goblin_1`s.  Built with the UUID, together with saving.  Open: what a copy with no `ShortName` is
  called; whether the name and the UUID are a component or something every entity has "baked".
- **Visuals, split up** (Jacob, 2026-09-30, mid-session: "may need to divide our current components up
  more").  Decided:
  - **`Transform`** takes over Position, Rotation and Scale, the way Unity has it.  Jacob said it
    "holds the rotation and position of its parent", and the parent there is the object the component
    is on, not another object (2026-09-30).  So no hierarchy: a transform is in the world's terms.
  - **`PrimitiveShape`**: the shape the client draws if it can't draw the model ("cube, capsule,
    etc.").  Unity's six built-in shapes: cube, sphere, capsule, cylinder, plane, quad.  Built
    (2026-09-30), a cube unless something says otherwise.
  - **`Animator`**: "controls animation state on the server".  A skeleton for now (Jacob, 2026-09-30:
    "I'm not worried about animations in game yet"): `is_looping_currently`, and `current_track`, the
    name of what the model is doing, sent to the client.
  - **`Model`**: a string, the path the client loads the model from (Jacob, 2026-09-30: "in a previous
    iteration I tried using an enum but that got messy").  The server never opens it; it's the
    client's to make sense of, and the PrimitiveShape is what the client falls back on when it can't.
- **Which blueprint a copy came from.**  The spawn system (in TODO.md) needs to count the goblin_as,
  so a copy will need to know its blueprint.  Whether that's a component or something every entity has
  "baked" (the question TODO.md already has for characters) waits on the spawn system.
- **Behaviour**: in the components (the Unity way) or in systems that run over every entity with a
  given set of components.  Waits on the game loop and the Lua.
- **Tests**: `cargo test -p conductor-primlib`.
