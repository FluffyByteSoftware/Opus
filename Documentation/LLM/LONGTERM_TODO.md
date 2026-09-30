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

## A scripting language of our own

Jacob's, 2026-09-29: a scripting language for writing the game's content in -- items, quests, NPCs, and
whatever else the world is made of -- written by us, for Opus, rather than a crate's language bolted on.

Why our own and not Lua or the like: it's the part of the game Jacob wants to build, and a language shaped
for items and quests can say those things in fewer words than a general one.  That's the pitch; the cost
is that a language is a parser, an interpreter, error messages, and docs, each its own piece of work.

Nothing is decided yet.  What has to be, when it opens:

- **What it's for, exactly.**  Data (an item's stats, an NPC's lines) or behaviour (what happens when the
  quest's third step is done), or both.  Data alone is a file format; behaviour is a language.
- **What it looks like.**  A sample of an item, a quest and an NPC written in it, before any code.  The
  sample decides the grammar.
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

Its name is Jacob's to give when it opens.

**Jacob's first sample** (2026-09-30, while the game library's ECS was talked through), MudOS / LPC in
spirit.  A template is a named set of components; `create` makes an object from a template and sets its
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

## A program of its own for account management

Jacob's, 2026-09-29, while the web admin's account manager was being planned: "eventually we will make a
separate account management program I think for running the modifications and updating the database while
its running."  The web admin's account manager changes accounts while the server runs, through Security's
line (one hash at a time, so a password change waits behind logins and they wait behind it).  A separate
program would do its own hashing and write to the database beside a running Conductor.

Open when it opens: its name (a third piece gets named by Jacob before it's made), how it and a running
Conductor agree on an account that's in the world while it's changed, and whether the web admin's
account manager stays or moves into it.
