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
holds them, templates and blueprints, and tests.  **Written 2026-09-30, not built yet.**  Nothing calls it yet.  It isn't a server piece yet
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
- **A component is plain data**, one struct per kind.  The first nine are Jacob's sample NPC:
  - `Position` and `Scale`: x, y, z as `f32` (what Unity uses).  Scale starts at 1, 1, 1.
  - `Rotation`: see below.
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
- **Rotation is three angles in degrees** (x, y, z), the way Unity's inspector shows a rotation and
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
  Open: whether START SERVER loads them back (the same goblins, hurt where they were hurt); whether
  anything is saved between stops, so a crash doesn't lose the lot; the table's shape.  Players'
  characters are saved too (`player_characters`, in TODO.md).
- **Every copy has a UUID** (Jacob, 2026-09-30): "you will be able to search NPCs by their UUIDs (which
  is unique to every instantiated one)".  From Fingerprinter (`new_uuid()`), like every row's.  The
  entity number is only good while the server runs; the UUID is the copy's name for good.
- **And an internal name**, "like goblin_archer_1".  Open: whether it's the blueprint's name and a
  number that goes up with each copy, and whether the number carries on past a restart (it would have
  to, if the copies are loaded back, or there'd be two goblin_archer_1s).
- **Visuals, split up** (Jacob, 2026-09-30, mid-session: "may need to divide our current components up
  more").  Decided:
  - **`Transform`** takes over Position, Rotation and Scale, the way Unity has it.  Jacob said it
    "holds the rotation and position of its parent"; open whether that means objects hang off other
    objects (a sword in a goblin's hand, moving with it) and a transform is relative to its parent,
    the way Unity's local position is.  Built without a parent for now.
  - **`PrimitiveShape`**: the shape the client draws if it can't draw the model ("cube, capsule,
    etc.").  Unity's six built-in shapes: cube, sphere, capsule, cylinder, plane, quad.
  - **`Animator`**: "controls animation state on the server".  Open: what it holds (the name of the
    state it's in, "idle" or "walk"; a speed; whether it loops).
  - Open: where the model itself goes (a `Model` component?) and what names it (a name Ensemble looks
    up, or a path under `Content/Assets/`).  An object without an `Animator` isn't animated, so there's
    no flag for it.
- **Which blueprint a copy came from.**  The spawn system (in TODO.md) needs to count the goblin_as,
  so a copy will need to know its blueprint.  Whether that's a component or something every entity has
  "baked" (the question TODO.md already has for characters) waits on the spawn system.
- **Behaviour**: in the components (the Unity way) or in systems that run over every entity with a
  given set of components.  Waits on the game loop and the Lua.
- **Tests**: `cargo test -p conductor-primlib`.
