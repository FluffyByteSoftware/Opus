<!--
File:       Opus/Documentation/LLM/WRITINGSTYLE.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Writing Style

This document tells Claude how to write anything in the Opus repo, so that it reads like Jacob wrote it.  That
covers:

- `README.md` and every document in `Documentation/` (STATUS.md, TODO.md, PROJECT_OPUS.md, PROTOCOL.md,
  REGION_MAP.md, the files in `design/` and `HowTo/`)
- Every comment in every source file, in Conductor (Rust), Ensemble and Soundcheck (C#): headers, doc
  comments, inline comments
- Every `Cargo.toml`, `.csproj`, `.conf`, `.gitignore` and shell script, and any plain file Jacob edits by hand
- Anything the programs write for a person to read: log messages, the comments inside a config file they
  generate, the messages a player sees, and any menus and prompts
- Commit messages, if we write any together

It does **not** cover how Claude talks to Jacob in the session itself.  CLAUDE.md covers that.

## Where this came from

Carried over from Stratum's writing style, which was built from Jacob's own writing and his edits to what Claude
wrote there.  The voice is settled; this file tightens as Jacob edits what's written for Opus.

## The voice in one paragraph

Jacob writes like he is talking to a friend across a workbench.  He says the first true thing first, gives the
real reason for a decision (including the unflattering ones), and admits what he doesn't know yet.  He is a
hobbyist and says so.  There is dry humor in it, but no sales pitch -- nothing is ever "powerful" or "seamless",
it either works or it doesn't yet.

## Traits to keep

1. **"We" for the project, "I" for opinions and admissions.**  "We write to a temp file first."  "I don't fully
   understand lifetimes yet, so this is the simple version."
2. **Lead with the point.**  Natural openers: "The idea is...", "My thought was...", "The problem has been...",
   "Essentially...", "So to summarize".
3. **Honest hedging.**  "probably", "inevitably", "for now", "this will likely change".  If something is
   unfinished or a guess, say so in plain words.
4. **Short sentences, then one longer one that runs the thought all the way through.**  Fragments are fine when
   they land.
5. **Asides use ` -- ` or parentheses.**  Never the long em-dash character.  An ellipsis is allowed once in a
   while for a change of direction, not as decoration.
6. **Plain words.**  When a technical term is needed, use it, and say what it means the first time.
7. **Blunt about bad tech and bad ideas** -- cleaned up for reading, but not softened into mush.  "The old
   approach was slow and ugly, so we dropped it."
8. **Reasoning as a chain.**  "If the save dies halfway, and the old file is already gone, then somebody loses a
   character.  So we never touch the old file until the new one is complete."
9. **Two spaces after a period in code comments.**  Markdown collapses them anyway, so don't worry about it in
   the `.md` files.  Player-facing text is the exception: one space, the way the messages read on screen.
10. **Mechanism first, then what it buys us.**  Describe what the thing does, then "This way if..." or "So..."
    and the reason.  "Temp data gets dumped before each algorithm runs over it.  This way if the weights are
    wrong we can reprocess without starting over."
11. **Bugs get told as a story.**  What we saw, what we thought it was, what it actually was, what we did.  The
    turn is almost always "It wasn't X, it was Y."
12. **Real numbers over adjectives, with the number's case next to it.**  Not "handles many NPCs efficiently"
    -- "5000 wandering NPCs and we're still under 1 GB of RAM."  And not "enough to wear out an SSD in weeks"
    on its own -- "58 GiB a minute in tick-sim's worst case, which would wear out an SSD in weeks if the server
    ever wrote like that."  A number without its case reads as a warning about the thing as it is.
13. **Define a term in parentheses the first time it shows up.**  "(regions are made up of zones, and zones are
    512x512 tiles)".  Words we made up or are using loosely go in quotes the first time: "reprocess",
    "physics".
14. **A short closer after a long sentence.**  "But it worked."  The next step can be a wry one-liner: "Now to
    make the terrain look less ugly."
15. **Unsolved problems are stated, not hidden.**  "The one thing I don't like and can't figure out how to fix
    yet is..."
16. **Keep Jacob's reasons in his words.**  When he gives a reason in the session that says why a design
    exists, it goes into the code or the docs nearly as he said it, with only the chat washed out.  From
    Stratum: "(So a shared account doesn't kick your brother off because you wanted to play.)"  That explains
    the design better than any rewording would.

## What gets cleaned up

Jacob's chat voice is the source, but anything written into the repo gets a wash:

| In chat | In the repo |
|---|---|
| lowercase starts, typos left alone | normal capitalization, spelling fixed |
| profanity | none -- the bluntness stays, the swearing goes |
| "lol", "bro", "aye" | dropped |
| "anyways I have this idea to..." | "The idea is to..." |
| humor | stays, kept dry and short |

The wash is for what Claude writes.  **Jacob's own lines are the master copy** and are left exactly as he wrote
them -- one space after a period, lowercase, a trailing space and all.  If something in one of his lines looks
like a slip, point it out; don't fix it.

## What to avoid

- Marketing words: robust, seamless, powerful, comprehensive, cutting-edge, elegant, blazing.
- Stock doc openers: "This module provides...", "This function is responsible for...".
- Comments that restate the name: `/// Gets the player.` above `get_player()`.
- Confidence we don't have.  Don't call something "the correct approach" when it is just the approach we picked.
- Emoji, exclamation marks, and closing summaries that repeat what was just said.
- Bullets where one sentence would do.
- **Any mention of an AI assistant** in source code, comments, `README.md` or commit messages.  Not in a
  comment, not in a commit message, not in the "how this was written" sense.  The repo is private, but the code
  stays clean in case any of it is ever shared.  `CLAUDE.md` and `Documentation/LLM/` are exempt, because they
  are literally about the sessions.

## Code comments

### File header

Every file starts with a header.  The `File:` line is the path from the repo root, starting with `Opus/` --
never the full path on the drive.  The `Component:` line is Conductor, Ensemble, Soundcheck, Documentation,
or Opus for files at the root.  When a file moves or is renamed, its `File:` line moves with it.

Rust uses `//!`, so the header also shows up in `cargo doc`:

```rust
//! File:       Opus/Conductor/dev/tools/src/diskman.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! DiskMan, the disk manager.  The layer between the server and the disk:
//! every file Conductor writes goes through here, and so does every file
//! it reads.
```

C# is tighter, the way Jacob cut Probe's header himself: no blank comment lines inside it, and the paragraph
only when it says something the file's name doesn't.

```csharp
// File:       Opus/Ensemble/dev/Opus.Ensemble/Assets/Scripts/Net/Frame.cs
// Component:  Ensemble
// Author:     Jacob Chacko
// Reads and writes the frame: a length, a type byte, then the payload.
```

A `.csproj` gets the banner and the three fields and nothing else.

Every `Cargo.toml`, `.conf`, `.gitignore` and shell script gets the same header written with `#`.  So does any
file Jacob edits by hand, like a version list:

```toml
# File:       Opus/Conductor/dev/Cargo.toml
# Component:  Conductor
# Author:     Jacob Chacko
#
# The server, as a Cargo workspace.
```

Markdown puts the header in an HTML comment at the very top, so it doesn't show when rendered.  Files that
can't hold a comment, or that a tool writes and rewrites (JSON, Unity's `.meta`, `.unity`, `.asset` and
`.prefab` files, lock files, anything in `build/` or `Content/`), have no header.

### Doc comments

**Doc comments** (`///` in both languages) say what the thing does and why it exists, in a sentence or three.
In Rust, an example inside one goes in a ` ```text ` fence.  A library's `cargo test` tries to compile anything
else as a test, and that includes a line indented four spaces.

**Inline comments** (`//`) explain *why*, not *what*.  If the line is obvious, it gets no comment.

**Rust notes.**  Where the code does something that would look strange to somebody coming from C or C#, leave a
short `// Rust note:` -- a few lines at most.  Anything longer than that belongs in the session, not in the
file.

**TODO markers** take the form `// TODO(topic): ...` and every one of them gets a matching line in
`Documentation/LLM/TODO.md`.

### Line width

Source lines stay inside **120 columns**.  That goes for code, comments and the strings inside the code.  (120
was RustRover's hard wrap on Jacob's machine for Stratum.  If his editor for Opus says otherwise, his editor
wins and this line changes.)

- Comments wrap at 78.  That is a habit for reading, not the limit -- a comment is easier to take in at 78 than
  at 120, and it leaves room to get indented a few levels.
- Code doesn't have to be pushed out to 120 either.  Wrap a line when wrapping makes it easier to read, the way
  Jacob does by hand.  A long `format!` gets an argument a line, a long signature a parameter a line.
- A long call gets broken after a comma, with the rest lined up under the first argument.
- The lock idiom in Rust is two lines, every time, even when it would fit on one:

```rust
    let mut guard = TCP.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
```

- A long message string in Rust gets split with a `\` at the end of the line.  Rust throws away the line break
  and the indentation on the next line, so the message still comes out as one line.  The two spaces after a
  period stay in front of the `\`, because spaces at the start of the next line are the ones that get thrown
  away:

```rust
scribe::error(Channel::Security, "Error on the Security channel.  \
    Nothing is actually wrong.");
```

- The width is a hard edge, because Jacob reads code in a window narrower than the full screen.  If a line
  doesn't fit, wrap it.  A sample in a comment (a log line, a command) gets wrapped too, with a note that it is
  one line for real.
- No trailing spaces in anything Claude writes.
- Before handing work back, check the width:

```
awk 'length($0) > 120 {print FILENAME": "FNR": "length($0)}' $(git ls-files '*.rs' '*.cs')
```

### Example

Not this:

```rust
/// Robustly handles the seamless serialization of player entities to disk.
```

This:

```rust
/// Writes the player out to disk.  We write to a temp file first and then
/// rename it over the old one -- that way a crash halfway through a save
/// can't eat somebody's character.
```

And a Rust note:

```rust
// Rust note: the `?` here means "if this failed, return the error to
// whoever called us".  Same idea as checking a return code in C, except
// the compiler won't let us forget.
let file = File::create(&temp_path)?;
```

### Example: a bug write-up, before and after the wash

How Jacob told it in chat (condensed):

> actually I found the problem... and its the weirdest thing.  It wasn't latency or anything, it's because it's
> udp.  So movement packets aren't in order.  The server was getting move northeast from a microsecond before
> you switched to southwest and it made it an illegal move.  now we just put whatever the most recent time
> stamped move intent packet was into the queue and everything works right.  that was a pain

How the same thing reads in STATUS.md:

> Found the movement bug, and it was a weird one.  It wasn't latency -- it was UDP doing what UDP does.
> Movement packets don't arrive in order, so the server would get a "move northeast" that was sent a hair
> before the player switched to southwest, and it would reject the move as illegal.  Now we only queue the
> move intent with the newest timestamp and throw the stale ones away.  Everything works.  That one was a pain.

Same order, same turn ("It wasn't X, it was Y"), same closer.  Only the spelling, the capitals and the swearing
changed.

## Text a program shows a person

Three kinds of reader, three registers.

- **Most log lines are Debug.**  Info is for the few milestones an admin cares about; the routine lines
  (loaded, connected, ran) are Debug, and a switch in the config hides them.  See CLAUDE.md.
- **Log lines** use Jacob's layout, `[ 02:16:43 PM - 09-28-26 Z ] - [ System / Info ] - [ message ] [ Caller:
  file, Line: n ]`, and the message inside is plain sentences, one fact each, with numbers and their case in
  the same sentence ("Generated a new world: 256 chunk(s), seed N, in 5 ms.").  An error the admin has to fix
  by hand opens in capitals so it can't be missed in a scroll of lines: "NO POSTGRES PASSWORD, FILL IT IN BY
  HAND: /path/to/Content/cfg/postgres.cfg.  THERE IS NO DATABASE UNTIL IT IS, ..."  Paths and names keep their
  own case -- a path cares about case, and a `grep` for the name should still find the line.  Capitals are for
  "the admin must do something", not for anything that is merely bad.  Account names are lowercase in the
  log; characters are shown capitalized.
- **Admin messages, menus and prompts** are flat and plain, one fact a line ("That isn't one of the choices.",
  "Stop the server first.").  A prompt says what Enter does, because that is the one thing the admin can't
  guess.  When a log line already says it, the menu doesn't repeat it.
- **Player messages** are flatter still, with one space after a period.  A player isn't Jacob's friend across
  the workbench.  A failure the player can act on is a sentence ("The server is full.", no "Sorry").  A
  failure they can't do anything about is a short label in title case, no period, like Jacob's own "Outdated
  Client Failure" and "Invalid Credentials".  When Jacob gives a label or a message, keep it as he wrote it.

## Per-document notes

- **README.md** -- what Opus is, the honest state of it (hobby project, in progress, things will break), how to
  build and run each part, and the folder layout.  No feature list for features that don't exist.  No mention
  of how the code gets written.
- **STATUS.md** -- the bridge between sessions, not a log.  Where things stand, the last session (what we did,
  what fought back, what Jacob decided), the last day's sessions as a line each (his "oh shit" fallback,
  2026-10-03), and anything earlier that bears on what's next.  What's waiting is listed unordered and
  unnumbered, since Jacob doesn't predict future sessions.
- **CLAUDE.md** -- the rules that hold in every session and a map of where the rest is.  A rule that only
  matters inside one piece goes in that piece's design file (2026-10-03).
- **TODO.md** -- two sections: *Deferred* (stubs and things waiting on pieces that don't exist yet) and *Ideas*
  (things we thought of along the way).  A good idea that doesn't fit this week's work still gets a home here.
- **PROJECT_OPUS.md** -- the skeleton: folder tree, one line per file on what it is for, and a table of the
  named pieces with their state.
- **PROTOCOL.md** and **REGION_MAP.md** -- what Conductor and Ensemble say to each other, and the one file they
  both read, written for somebody building the other side who has never seen its code.  The layout in words
  first, then the bytes in a text block with offsets, then one worked example in hex.  A sentence on why a
  choice was made where it helps, and nothing about how either side's code is organized.
- **design/*.md** -- one file per system.  What we decided, the reason (trait 16), the numbers that back it,
  and what's still open.  Decisions are written down once, here, and other documents point at them.
- **HowTo/*.md** -- for a person, in Jacob's voice, like README.md: the steps, the commands, and what went
  wrong on the way.

## Reading Jacob

Not about the voice, but about understanding what he means, learned over Stratum's sessions.

- **"I think so" is a yes**, the same as "I have to trust your judgment".
- **Restating a number is reading, not doubt.**  "ah a chunk is 32^3 okay" needs no reassurance.
- **A design restated with a question mark is a correction.**  "I thought that's how we designed it?" means the
  question misread the design.  Say so in one line and record the design as he said it.
- **Correcting a name usually means correcting the meaning.**  Look at what his next line says the thing *is*.
- **"We may" is a maybe.**  File it as a maybe, not a decision.
- **Read his short lines as he means them** before calling one a slip.  "Check probes packets" used "probes" as
  a verb.  When unsure, ask what it means rather than offering a fix.
- **A skipped question may mean "I don't know", not "I missed it".**  Asking again doesn't help.  Say what each
  answer means in practice and which one Claude would pick.
- **A question can get answered about the wrong thing.**  Asked *how*, he may answer *when*.  If the answer
  doesn't fit the question, the question is still open.
- **His design can turn mid-answer.**  When it does, write both down as an open question rather than picking
  one for him.
- **When a comment has gone stale, he cuts rather than rewrites.**  Follow that lead.
- **Offered a list, he may answer with his own design instead.**  Asked which of Stratum's Scribe features to
  keep, he described how reading the log should work.  That is the answer.  Ask only about what it leaves
  open.
- **He names pieces as he goes.**  `conductor-launcher` arrived inside an answer about something else.  Use a
  name as he gives it.
- **An uploaded Stratum file is a model, not a port.**  "Model after" means take the shape and write it fresh
  for Opus.

## Refinement log

A new lesson gets a dated line here as Jacob edits what's written.
