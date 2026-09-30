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

**Lua, 2026-09-30** (Jacob: "Lua is a fine language for this actually"), embedded in Conductor, and it
comes next, before the game library's ECS.  The crate that embeds it (`mlua` is the usual one) is a new
dependency, so it's asked for before it goes in; and which Lua (5.4, LuaJIT, or Luau, Roblox's, which
has a locked-down mode built in) is open.  Jacob's sample below is the shape to aim for, written as Lua.

**Behaviour** (Jacob, 2026-09-30): an object gets a behaviour script added to it, and that script
controls or changes what it does.  He'd like GOAP (goal-oriented action planning) for the thinking; not
settled.

Still to settle:

- **What it looks like.**  Jacob's NPC sample is below; an item and a quest in the same shape, in Lua,
  before any code.
- **Where it runs.**  Conductor reads and runs it, since Conductor is authoritative.  Whether Ensemble
  ever sees any of it (an item's description, say) is the protocol's question.
- **Where the files live.**  `Content/` is where both programs' data goes, so somewhere under it, with a
  name that says whose they are.  Through DiskMan, like every file.
- **How it loads.**  At server start, and again on a soft reboot; whether a changed file can be picked
  up without one is the same question the config editor has.
- **What a mistake looks like.**  A script with an error in it names the file and the line, the way
  Constellations complains about a bad config line, and never takes the server down.
- **What it can't do.**  A script can't touch the disk, the network or the database on its own; it asks
  the game, and the game decides.  That keeps a bad quest from being a bad server.


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

## A program of its own for account management

Jacob's, 2026-09-29, while the web admin's account manager was being planned: "eventually we will make a
separate account management program I think for running the modifications and updating the database while
its running."  The web admin's account manager changes accounts while the server runs, through Security's
line (one hash at a time, so a password change waits behind logins and they wait behind it).  A separate
program would do its own hashing and write to the database beside a running Conductor.

Open when it opens: its name (a third piece gets named by Jacob before it's made), how it and a running
Conductor agree on an account that's in the world while it's changed, and whether the web admin's
account manager stays or moves into it.
