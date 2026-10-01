<!--
File:       Opus/Documentation/LLM/LONGTERM_TODO.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Long-term TODO

The big features: a run of sessions each, and waiting less on any one piece than on the game existing
around them.  TODO.md holds what's deferred and the small ideas; this holds what Opus is going to be.
Entries are added as they come up and stay until they're built.  Nothing here is ordered; Jacob picks.

## The scripting language

Content (items, quests, NPCs, whatever the world is made of) is written in **Lua 5.4**, embedded through
`mlua` with Lua built in (Jacob: "Lua is a fine language for this actually").  The first step is in
(`lua-parser`, `design/lua-parser.md`): every script under `Content/scripts/` runs once on START SERVER,
locked down, with the log the only thing it can call.  Templates and blueprints exist in Rust
(`conductor-primlib`); writing them as scripts is primlib's part 2 (`design/primlib.md`).

**Behaviour**: an object gets a behaviour script added to it that controls what it does.  Jacob would
like GOAP (goal-oriented action planning) for the thinking; not settled.  And maybe "just develop like
Unity -- I attach a script to an object and it fires off behaviour, or comes prepackaged with behaviour to
manipulate the object."  He wants to think it over.

Still to settle:

- **What it looks like**: Jacob's NPC sample below, and an item and a quest in the same shape, in Lua,
  before any code.
- **Whether a script is attached to an object** the way a Unity component is, whether a template or
  blueprint comes with its behaviour attached, and how `Content/scripts/` is laid out for that.
- **Whether Ensemble sees any of it** (an item's description, say).  The protocol's question.
- **Hot loading**: no, for now.  Scripts load on START SERVER and again on RESTART SERVER.

**Jacob's first sample**, written before Lua was picked (MudOS / LPC in spirit).  A template is a named
set of components; `create` makes a blueprint from it and sets the values:

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

`spawn goblin_a x 100` makes a hundred goblins, each with its own values.

## The world

`conductor-gameworld` is part one: a seamless world 8 km a side, in 1 m blocks and chunks of 32 a side,
eleven chunks tall (-32 to +319), cut into regions (a region is a zone is a biome).  The first world is Alpha (flat) and
Omega (hills).  `design/world.md` has it.  What's left, a session or more each:

- **Saving changed chunks** (part two): on STOP SERVER and every `save_minutes` (15, in `game.cfg`).
  Comes with the first thing that changes a block.
- **Sending chunks to a client**: only the ones near it, since the server decides what each client sees.
  A protocol change.  And how Ensemble gets `region.map`.
- **Loading around players who move**, instead of only around 0,0,0.
- **What a region does** beyond its name: what grows and what spawns there.
- **Blending one biome into the next** where two regions meet.
- **A bigger world** ("it may _grow_ later"): `region.map` keeps its size in its header for that.  Jacob,
  2026-10-01: "we're gonna make the world twice as big in the next session".  Whether that's twice as wide
  (16 km a side) or twice the ground (about 11.6 km), and whether today's world is made again or grown,
  is to ask (STATUS.md).

## Soundcheck, the patcher, and a certificate for every client

**Opus.Soundcheck** is the patcher (Jacob's name).  It runs before the game and gives the client a
certificate, and the server turns away any connection without a valid one ("then both the client should
be able to trust the server and vice-versa").  Mutual TLS: today only the server shows a certificate
(`with_no_client_auth()` in `tls.rs`), and a player proves who they are with the password.

What it buys: anything that isn't our client is turned away in the TLS handshake, before it gets near
Security's line, so a bot throwing logins at the door never costs a hash.  What it doesn't: a changed
client still has its certificate, so it keeps out strangers, not cheaters (the server deciding what each
client sees handles those).

Open when it opens:

- **One certificate per client**, so one can be taken back from one player.  That needs our own
  certificate authority (a key that signs the clients' certificates, which the server trusts), something
  to do the signing (Soundcheck asking the server, most likely), and a ban list of certificates.  rustls
  checks certificates but doesn't make them; making them in Rust is a crate (`rcgen`, Jacob's call), or
  openssl by hand.
- **What Soundcheck is written in**, where it lives (a new top-level folder is Jacob's call), and what
  else it does (the game's files, updates).
- **How it proves who's asking** before it hands a certificate out: the account's password, most likely,
  so a certificate belongs to an account, not just an install.
- **The test client** gets one too, and a `--client-cert`.
- **PROTOCOL.md**: the handshake asks for the client's certificate.  Whether that bumps the version (it
  happens before the Hello, so an old client fails in the handshake, not on the version).

## A program of its own for account management

Jacob: "eventually we will make a separate account management program I think for running the
modifications and updating the database while its running."  Today the Accounts tab's changes wait in
Security's line with the logins; a separate program would do its own hashing and write to the database
beside a running Conductor.

Open when it opens: its name (Jacob names a new piece before it's made), how it and a running Conductor
agree on an account that's in the world while it's changed, and whether the Accounts tab stays.
