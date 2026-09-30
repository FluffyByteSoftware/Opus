<!--
File:       Opus/Documentation/LLM/LONGTERM_TODO.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Long-term TODO

The big features: things that are a run of sessions each, not one, and that don't wait on any one piece
so much as on the game existing around them.  TODO.md holds what's deferred and the small ideas; this holds
what Opus is going to be.  Rolling, in the sense that entries get added as they come up and stay until
they're built.  Nothing here is ordered; Jacob picks what opens.

## The scripting language

Jacob's, 2026-09-29: a scripting language for writing the game's content in -- items, quests, NPCs, and
whatever else the world is made of.  It started as one of our own (it's the part of the game Jacob wants
to build, and a language shaped for items and quests says them in fewer words; the cost is a parser, an
interpreter, error messages and docs, each its own piece of work).

**Lua 5.4, 2026-09-30** (Jacob: "Lua is a fine language for this actually"), embedded in Conductor
through the **`mlua`** crate (Jacob said yes to the dependency), with Lua built in ("vendored"), so
there's nothing to install; it needs a C compiler, which Nobara has, and Visual Studio's on Windows.  Its
first step is in and tested (`lua-parser`, 2026-09-30; `design/lua-parser.md`): every script under
`Content/scripts/` runs once on START SERVER in a locked-down Lua, with the log the only thing it can
call.  The game library it was waiting for is in (`conductor-primlib`, 2026-09-30; `design/primlib.md`),
with templates and blueprints built in Rust for now; writing them as scripts is primlib's part 2.  Locked
down by never loading Lua's `io` and `os` libraries (or anything else that reaches the disk or the machine).  Jacob's sample below is the shape to aim for,
written as Lua.

**Behaviour** (Jacob, 2026-09-30): an object gets a behaviour script added to it, and that script
controls or changes what it does.  He'd like GOAP (goal-oriented action planning) for the thinking; not
settled.

**Like Unity, maybe** (Jacob, 2026-09-30, asked where the scripts live): "I was thinking we'd just
develop like Unity -- I attach a script to an object and it fires off behaviour, or comes prepackaged
with behaviour to manipulate the object."  He wants to think it over.  Open: whether a script is
attached to an object the way a Unity component is, whether a template or blueprint comes with its
behaviour already attached, and how the folder is laid out for that.  It's the ECS's question as much as
Lua's.

Still to settle:

- **What it looks like.**  Jacob's NPC sample is below; an item and a quest in the same shape, in Lua,
  before any code.
- **Where it runs.**  Conductor reads and runs it, since Conductor is authoritative.  Whether Ensemble
  ever sees any of it (an item's description, say) is the protocol's question.
- **How the folder is laid out** under `Content/scripts/`, once the Unity question above is answered.
- **Hot loading.**  Scripts load at START SERVER and again on RESTART SERVER; whether a changed file can
  be picked up without one is the same question the config editor has.  No, for now.

Settled with the first step (`lua-parser`, 2026-09-30; `design/lua-parser.md` has the rest):

- **Where the files live**: `Content/scripts/`, the root of every script, folders inside it and all
  (Jacob).  Read through DiskMan.
- **What a mistake looks like**: a Warn naming the file and the line, and the rest still run.  "Any
  errors or warnings from Lua" are Warns, never Errors, and a script "shouldn't be able to do anything
  to crush the underlying systems" (Jacob): a second's time limit, 64 MB, and 50 log lines a run.
- **What it can't do**: no `io`, `os`, `package` or `debug`, no `dofile`, `loadfile`, `load` or
  `string.dump`.  The log is all it can call.  That keeps a bad quest from being a bad server.


**Jacob's first sample** (2026-09-30, while the game library's ECS was talked through), MudOS / LPC in
spirit, written before Lua was picked.  A template is a named set of components; `create` makes an object from a template and sets its
components' values.  Conductor knows from it which components to put on the object, and sends the
object to the clients as it changes.

```
define template as "NPC"
{
	// adds the position X, Y, Z floats to this game object
	AddComponent(Position);
	// adds the rotation directions, X, Y, Z floats for this game object
	AddComponent(Rotation);
	// Adds the scalar values for X, Y, Z floats for this game object
	AddComponent(Scale);
	// Adds a "short name" to refer to this item as (this is usually lower case, examples goblin instead of
	// "a Goblin Warrior")
	AddComponent(ShortName);
	// Adds a "long name" with proper capitalization and title or unique name if necessary (ie: "a Goblin
	// Warrior named, Serah")
	AddComponent(LongName);
	// Adds a titles component like "the Cursed" this is an array with one value selected as CurrentTitle;
	AddComponent(Titles);
	AddComponent(Health);
	AddComponent(Endurance);
	AddComponent(Mana);
}

// Tell our script parser that we are creating a gameobject called goblin_a that is a template NPC (comes
// prefixed with these components)
create NPC goblin_a{
	// Spawns this at 0 0 0 facing 0 0 0
	Position.SetPosition(0, 0, 0);
	Rotation.SetRotation(0, 0, -90);
	Scale.SetScale(1,1,1);
	ShortName.Set("goblin");
	LongName.Set("goblin archer");
	Titles.Set("the plucky");
	Health.SetMax(2000);
	// No need to Health.Set it, it auto sets to max unless specified
	Endurance.SetMax(200);
	Mana.SetMax(2);
}
```

What it settles so far: the language describes objects as components with values, and a component can
have a default that follows from another value (current health starts at its max).  Behaviour (what the
goblin does) isn't in the sample yet.

**`goblin_a` is a blueprint** (Jacob, 2026-09-30): `spawn goblin_a x 100` makes a hundred goblins, each
with its own health and the rest.  So there are three layers: the template (`NPC`, which components),
the blueprint (`goblin_a`, their starting values), and the copies in the world.

## The world: voxels and zones

Jacob's map at the 2026-09-30 close (STATUS.md has it): the world tick first, "then after that we build
our world (voxel information and zone management after that)".  The tick is built (the GameClock,
2026-09-30), so the world is what's next on his map.  The world is going to be **voxels**.
Nothing about it is designed yet.  Open when it opens:

- **Voxel information**: what a voxel holds; the chunk size; the world's size; flat or generated; where
  it's kept (the database, files through DiskMan, or both) and when it's saved; what the client is sent
  and when (the server decides what each client sees, so a player gets the chunks near them, not the
  world).
- **Whether the world belongs to primlib**, beside its `World` of entities, or is a piece of its own, and
  whether chunks are entities or something beside the ECS.  TODO.md's protogame entry has asked since
  2026-09-30.
- **Zone management**: what a zone is (a fixed square of chunks, or drawn by hand), what it's for (who
  hears what, what gets ticked, spawn areas for the spawn system, loading and unloading what nobody is
  near), and how the tick handles one.
- How it's seen on the web admin, if at all.

## Soundcheck, the patcher, and a certificate for every client

**Opus.Soundcheck** is the patcher (Jacob named it, 2026-09-30).  Jacob's, the same day, after the first login from outside: a patcher runs before the game and gives the
client a certificate, and the server turns away any connection without a valid one ("then both the
client should be able to trust the server and vice-versa").  Mutual TLS: today only the server shows a
certificate (`with_no_client_auth()` in `tls.rs`), and a player proves who they are with the password.

What it buys: anything that isn't our client is turned away in the TLS handshake, before it gets near
Security's line, so a bot throwing logins at the door never costs a hash.  What it doesn't: a changed
client still has the certificate its install was given, so it keeps out strangers, not cheaters (the
server deciding what each client sees is what handles those).

Open when it opens:

- **One certificate per client** (Jacob, 2026-09-30), not one for everybody, so one can be taken back
  from one player.  That needs a signer: our own certificate authority (a key that signs the clients'
  certificates, which the server trusts), something to do the signing (Soundcheck asking the server,
  most likely), and a way to take one back (a ban list of certificates).  rustls checks certificates
  but doesn't make them; making them in Rust is a crate (`rcgen`, Jacob's call), or openssl by hand as
  today.
- **What Soundcheck is written in**, where it lives in the repo (a new top-level folder, which is
  Jacob's call), and what else it does (the game's files, updates).
- **How Soundcheck proves who's asking** before it hands a certificate out: the account's password,
  most likely, which would make a certificate belong to an account, not just an install.
- **The test client** gets one too, and a `--client-cert` of its own.
- **PROTOCOL.md** changes: the handshake asks for the client's certificate.  Whether that's a protocol
  version bump (it happens before the Hello, so an old client would fail in the handshake, not on the
  version).

## A program of its own for account management

Jacob's, 2026-09-29, while the web admin's account manager was being planned: "eventually we will make a
separate account management program I think for running the modifications and updating the database while
its running."  The web admin's account manager changes accounts while the server runs, through Security's
line (one hash at a time, so a password change waits behind logins and they wait behind it).  A separate
program would do its own hashing and write to the database beside a running Conductor.

Open when it opens: its name (a third piece gets named by Jacob before it's made), how it and a running
Conductor agree on an account that's in the world while it's changed, and whether the web admin's
account manager stays or moves into it.
