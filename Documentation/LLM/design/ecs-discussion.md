<!--
File:       Opus/Documentation/LLM/design/ecs-discussion.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- the ECS, a brief for a separate chat

Written at the close of the 2026-09-30 session, for Jacob to hand to a regular Claude chat and talk the
game library's ECS through before any of it is written.  What comes out of that chat comes back to the
Opus sessions, which fold it into TODO.md and this file.  Everything below is written to the chat.

---

## Who you're talking to

Jacob is an amateur hobbyist building a multiplayer game called **Opus** (a codename; it has nothing to
do with any model's name).  He's comfortable in C and C#, and still learning Rust.

- Explain Rust plainly.  Don't translate it into C terms unless he asks.
- Keep replies short at first; expand when he engages.
- He steers.  When something he says could mean more than one thing, ask, and say what each reading
  would mean in practice.  Don't fill gaps with guesses.
- When there's a clever way and a plain way, he wants the plain way.
- Put questions for him at the **bottom** of a reply, under a loud header:
  `# >>>>>>>>>> QUESTIONS FOR JACOB <<<<<<<<<<`
- This chat is for **design, not code**.  Short Rust sketches to show a shape are welcome; a finished
  implementation isn't, because the code is written in the Opus sessions under their own rules.

## What Opus is

- **Conductor** is the game server, in Rust (edition 2024), on Linux first.  It's authoritative: it owns
  the game state and the tick loop.  Clients ask, Conductor decides.
- **Ensemble** is the client, in Unity 6000.6 (C#).  It isn't part of this talk; the ECS here is the
  server's.  (Unity has its own ECS; nothing here has to match it.)
- The world is going to be **voxels**.  Nothing about the world is designed yet: chunk size, world
  size, flat or generated, where it's kept.
- A player logs in over TLS on TCP, is handed a ticket, and plays over **UDP**.  When the UDP session
  ends, the player is gone and starts over at the login screen.

Conductor today is a Cargo workspace of six crates: `conductor-tools` (disk, logging, config, UUIDs,
password hashing, the database worker, the thread and service lists), `conductor-accounts` (the one way
in to the accounts table), `conductor-monitor`, `conductor-networking` (the login and the UDP side),
`conductor-wgui` (a web admin on 127.0.0.1), and `conductor-launcher` (the program).  There's no game
yet: the UDP side only keeps a logged-in player alive.

## The rules the ECS has to live with

These are Conductor's standing rules.  If a design runs into one, say so plainly rather than working
round it; Jacob decides whether the rule bends.

- **Plain over clever.**  Clear ownership and simple types over heavy generics or macros.
- **Minimal dependencies.**  Any new crate is Jacob's call, asked for first.
- **No async runtime.**  Anything slow (the database, the disk, the network) runs on its own thread,
  and the caller gets the answer back later (a `Pending` it checks).  **The game loop never waits on
  it.**
- **Every thread** goes through Conductor's own `threads::spawn(name, ...)`, so it shows on the web
  admin.  A crate that starts threads of its own (rayon, say) bypasses that, which has already
  counted against one crate.
- **Lower CPU, more RAM if it buys that** (Jacob's steer for networking; it likely carries over).
- **Every file read and write** goes through Conductor's disk manager, never `std::fs`.
- **The database** is PostgreSQL 18, through the blocking `postgres` crate on its own worker thread.
  Every table has an `id` (`BIGINT` identity, how tables point at each other) and a `uuid` (UUID v7,
  the game's name for the row, used by the game, the client, the web admin and the logs).  A schema
  file is frozen once its table exists; every change after that is a numbered migration.
- **An account is never held in memory**: whatever needs one reads its row when it needs it, and every
  change goes straight back to the row, so there's one copy and nothing writes an old one over a new
  one.  Whether characters follow the same rule while they're in the world is open (they will have to
  live in memory while they're played; the question is what's the copy and what's the master).
- **Two restarts**: a soft reboot brings the server pieces down and up with the program still
  running; a hard reboot runs the program again.  A server piece has to be able to stop and start
  again in the same run, so the game library does too.
- **All time is UTC.**

## What's settled (2026-09-30)

- **Two libraries.**  **Protogame** is "game adjacent": not the game, but what connects a logged-in
  player to it.  Messages to and from a character are protogame's.  The **game library** is the world
  and holds **Actor**, **Agent** and **Character**.
- **Actor, Agent, Character.**  An Actor is anything that acts in the world.  An Agent is an Actor the
  computer controls.  A Character is an Actor with a human controller on top.  That was said before the
  ECS came up; with an ECS these likely stop being types and become sets of components (a controller
  component, `HumanController` or `AiController`, being the difference).  Confirming or changing that is
  part of this talk.
- **The game library is going to be an ECS.**  A character is "a few components", not one type.
- **`CharacterSnapshot`** lives in `conductor-accounts`: a read-only copy of a character's surface (its
  name, where it is), read from its row, so it's whatever the last save to the database says.  For
  things outside the world: character select, the web admin.  Never written back.
- **The database**: a `player_characters` table, each row with its account's `id` (`account_id`).
  Every account has three slots, `character_slot_1` to `character_slot_3`, each the `id` of a
  `player_characters` row or empty.  `conductor-accounts` writes the SQL for both tables, and nothing
  else does.  Postgres does the wiping: deleting an account deletes its characters (`ON DELETE
  CASCADE`), and deleting a character empties its slot (`ON DELETE SET NULL`).
- **What the row holds is waiting on this talk.**

## What to talk through

In whatever order Jacob likes.  These are the questions the Opus session left open.

1. **Written by hand, or a crate?**  `bevy_ecs` and `hecs` are the usual crates.  Either is a new
   dependency and leans on generics and macros; a hand-written one is plain at this size, teaches Rust
   and is ours to keep up.  What does a hand-written one need at minimum, what would it cost later
   (thousands of entities, queries over several components, adding and removing components while
   the tick runs), and how would it look in plain Rust?  The session leaned hand-written.
2. **What's an entity** (a number, a number with a generation so an old one can't be mistaken for a new
   one), and how does it relate to the database's `id` and `uuid` for things that are saved?
3. **Which components make a character** at first, and **what makes an Actor**: what does an Actor have
   that a dropped sword doesn't?  A first guess was `Name`, `Position` and `HumanController`.
4. **Systems and the tick.**  What runs each tick and in what order; the tick rate (Conductor has no
   game loop yet, and the rate is unset); one thread for the world or more.
5. **Saving.**  A character's components have to reach its `player_characters` row, without the game
   loop waiting on the database.  When (every so often, on logout, on a change), and in what shape:
   - **A.** a column per saved field on `player_characters` (`name`, `x`, `y`, `z`, ...): plain and
     readable, and a new saved component is a migration;
   - **B.** a table per component, keyed by the character's `id` (each with its own `id` and `uuid`,
     by the rule);
   - **C.** one column holding every component in an encoding of our own: flexible, hard to read.
   The session leaned A to start.
6. **Which components are saved at all**, and which only live while the server runs.
7. **Protogame's edge.**  Where the network's messages come into the ECS (a player's input becoming
   components, or events) and how what a character sees goes back out, without protogame knowing the
   game library's insides.  Which crate depends on which: Rust doesn't allow a circle.
8. **The voxels.**  Whether the world's chunks are entities or something beside the ECS, and whether
   the world belongs to the game library.  It may be too early; say so if it is.

## What to bring back

End the chat with a summary Jacob can paste into the Opus session as it stands.  In this shape:

```
## ECS talk -- <date>

### Decided
- one line each, with Jacob's reason where he gave one

### Leaning, not decided
- ...

### Open
- ...

### Runs into a Conductor rule
- which rule, and what Jacob said about it

### Sketch
<a short Rust sketch of the shapes agreed: the entity, a couple of components,
one system, how a character is put together -- enough to show the shape, no more>
```
