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

- **Is a spawned NPC saved?**  Jacob's first thought (2026-09-30): once a copy is spawned it becomes a
  database entry, and its values are managed through the row until it's destroyed ("Flat files made
  this easier").  That runs into a rule: the game loop never waits on the database.  The world in
  memory has to be what the tick reads and writes.  A hundred goblins read from Postgres twenty times
  a second is 2,000 round trips a second through Archivist's one worker, before anything else wants it.
  So the choice is when the world's copy goes to the row, not whether memory holds it:
  - **Never, for NPCs**: a server restart starts the world empty, and the spawn system fills it again
    from the blueprints.  No table.
  - **Every so often and on STOP SERVER**, through Archivist, the game loop never waiting: a restart
    brings back the same goblins, hurt where they were hurt.  A table (or tables) for them.
  - Players' characters are saved either way (`player_characters`, in TODO.md).
- **Visuals** (Jacob, 2026-09-30, mid-session): a component for what the client draws: which model
  Ensemble loads, which animation it's in (if any), and whether it's animated at all.  "May need to
  divide our current components up more."  Open: one `Visuals` component, or split the way Unity
  does (a `Model` component, and an `Animation` component only on what moves, so "is it animated" is
  whether it has one); what names a model (a name Ensemble looks up, or a path under `Content/Assets/`);
  and which of the first nine get split too.
- **Which blueprint a copy came from.**  The spawn system (in TODO.md) needs to count the goblin_as,
  so a copy will need to know its blueprint.  Whether that's a component or something every entity has
  "baked" (the question TODO.md already has for characters) waits on the spawn system.
- **The entity and the database's `uuid`.**  An entity number is only good while the server runs.
  Anything saved, or named to the client and the logs, will want a `uuid` from Fingerprinter as well.
  Waits on the question above.
- **Behaviour**: in the components (the Unity way) or in systems that run over every entity with a
  given set of components.  Waits on the game loop and the Lua.
- **Tests**: `cargo test -p conductor-primlib`.
