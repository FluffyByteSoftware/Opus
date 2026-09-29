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
