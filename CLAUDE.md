<!--
File:       Opus/CLAUDE.md
Component:  Opus
Author:     Jacob Chacko
-->

# Opus

Opus is the codename for this project: a multiplayer game made of a server and a
client. When I say "Opus" I mean this project, not the Claude model.

Opus is a separate project from Stratum and Mantle. Do not pull code from them
unless I explicitly ask. Their *workflow* carried over; their code did not.

Project root: `/opt/storage/Coding/Opus`

---

## Components

- **Conductor** -- the game server. Authoritative: it owns the game state and the
  tick loop. Clients ask, Conductor decides.
  - Language: Rust, edition 2024
  - Folder: `Conductor/`
- **Ensemble** -- the game client that players run.
  - Engine / language: [FILL IN -- e.g. Unity 6 (C#) or Godot 4 (C#)]
  - Folder: `Ensemble/`
- **Documentation** -- project docs. `Documentation/LLM/` holds the working
  docs (status, TODOs, design, protocol) and is the source of truth for anything
  not in the code.
- **Shared protocol** -- whatever both sides need to agree on (message types,
  encoding) is documented in `Documentation/LLM/PROTOCOL.md` and is the single
  source of truth.

If a future third piece appears (tools, admin console, test client), ask me what
to call it before creating it.

---

## Folder layout

```
Opus/
├── CLAUDE.md              # this file
├── .gitignore
├── Conductor/             # server
│   ├── dev/               # source code (the Cargo project lives here)
│   └── build/             # compiled output -- never committed
├── Ensemble/              # client
│   ├── dev/               # source code (the engine project lives here)
│   └── build/             # compiled output -- never committed
├── Content/               # runtime data both programs read and write -- never committed
│   ├── cfg/               # config files (conductor_globals.cfg lives here)
│   └── logs/              # log files
└── Documentation/
    └── LLM/               # working docs
        ├── STATUS.md      # bridge between sessions
        ├── TODO.md        # pending work + future ideas
        ├── PROJECT_OPUS.md# skeletal layout of the whole project
        ├── PROTOCOL.md    # server/client contract
        ├── WRITINGSTYLE.md# my voice for public docs and comments
        └── design/        # one markdown file per system or feature
```

Each component keeps two folders: `dev/` is where code is written, `build/` is
what the compiler makes. Runtime data for both components lives in one shared
`Content/` folder at the root (one per component was redundant). Files in it are
named so it's clear who owns them (`conductor_globals.cfg`, `*.scribe.log`).
Don't create new top-level folders without asking me.

Because `Content/` is never committed, a fresh checkout has no `Content/` folder
at all. Programs must create it and any file they need there (config, logs,
saves) with sensible defaults when it's missing, and never crash because it's
absent. Conductor finds it through the `OPUS_CONTENT` environment variable, or
by walking up from the working directory until it sees a `Content/` folder, or
by creating `./Content` when neither works.

---

## Start of every session

1. Read `Documentation/LLM/STATUS.md`, `Documentation/LLM/PROJECT_OPUS.md`, and `Documentation/LLM/TODO.md` to get
   your bearings. Read anything in `Documentation/LLM/design/` that touches today's work.
2. Re-read any source file before editing it. I hand-edit files between sessions,
   and my edits are the master copy. Never overwrite my changes with an older
   version from memory.
3. Tell me in a couple of lines where things stand, then ask what I want to work on.

## During a session

- **One feature per session.** If a new feature comes up mid-session, add it to
  `Documentation/LLM/TODO.md` and keep going on the current one. It gets its own session later.
- **Plan before building** anything bigger than a small fix: tell me the files
  you'll touch and the approach, and wait for my OK.
- **Things that can't be done yet** (because a dependency isn't built) go in
  `Documentation/LLM/TODO.md`, not half-implemented in code.
- **Future ideas** that come up in conversation also go in `Documentation/LLM/TODO.md`.
- **I build, run, and test everything myself** and paste back the output.
  Do not run `cargo check`, `cargo build`, `cargo test`, the server, or the
  client. Stick to writing the code. When it's written, put your questions at
  the top of the reply, then tell me what I need to modify by hand (if
  anything) and exactly which commands to run. I paste back what happens and
  we go from there.
- Do not predict or number future sessions ("next session is X, then Y").
  I pick what to open next and I'm free to change my mind.

## End of every session (hand-off)

When I say we're wrapping up:

1. Update `Documentation/LLM/STATUS.md`. It holds only the last session plus any earlier session
   that directly matters for the next one. It is a bridge, not a rolling log.
   List what's waiting, unordered.
2. Update `Documentation/LLM/TODO.md`, `Documentation/LLM/PROJECT_OPUS.md`, and any `Documentation/LLM/design/`
   files the session changed, so they match reality.
3. Update `README.md` if anything about the project's overview changed.
4. Suggest any additions to this CLAUDE.md based on how the session went.
5. Make no code changes during hand-off unless there's a glaring bug, and if so,
   tell me first.

---

## How to talk to me

- I'm an amateur hobbyist. Comfortable in C and C#, still learning Rust.
- Explain Rust plainly. Don't translate Rust into C terms unless I ask.
- **Questions for me go at the top of your reply under a visible `## Questions`
  header** so I don't miss them.
- Keep replies short at first. Expand when I engage.
- When you change files, end with a short list of which files changed and why.

---

## Code rules (all components)

- Simple and readable over clever. If there's a clever way and a plain way, use
  the plain way.
- **All time is UTC.** Any time shown to a person ends with `Z`.
- Ask before adding any new dependency (crate, package, plugin). Minimal
  dependencies is the default.
- No references to AI, Claude, or assistants anywhere in source code, comments,
  the README, or commit messages. `CLAUDE.md` and `Documentation/LLM/` are the
  only exceptions. (The repo is private today, but keep the code clean in case any of
  it is ever shared.)
- Comments and public docs are written in my voice per `Documentation/LLM/WRITINGSTYLE.md`.
- Line width: 120 columns; comments wrap at 78. Details in `Documentation/LLM/WRITINGSTYLE.md`.
- **Every file starts with a header** saying where it lives, as a path from the
  repo root beginning with `Opus/` (never the full drive path), which component
  it belongs to, and who wrote it, then a line or three on what the file is for.
  When a file moves or is renamed, its `File:` line changes with it.

  Rust (`//!` so it shows up in `cargo doc`):
  ```rust
  //! File:       Opus/Conductor/dev/src/main.rs
  //! Component:  Conductor
  //! Author:     Jacob Chacko
  //!
  //! What this file is for, in a sentence or three.
  ```
  C#:
  ```csharp
  // File:       Opus/Ensemble/dev/Assets/Scripts/Net/Connection.cs
  // Component:  Ensemble
  // Author:     Jacob Chacko
  // What this file is for (only if the name doesn't already say it).
  ```
  TOML, `.gitignore`, shell scripts and `.conf` files use the same lines with `#`.
  Markdown uses an HTML comment at the very top, so it doesn't show when rendered:
  ```markdown
  <!--
  File:       Opus/Documentation/LLM/STATUS.md
  Component:  Documentation
  Author:     Jacob Chacko
  -->
  ```
  The only files without a header are ones that can't hold comments or that a
  tool generates and rewrites: JSON, Unity's `.meta` / `.unity` / `.asset` /
  `.prefab` files, lock files, and anything under `build/` or `Content/`.

## Rust rules (Conductor)

- **Never use `#[allow(dead_code)]`** or any other lint suppression. I would
  rather see the warnings.
- Whenever you create a new crate, say explicitly whether it is a **bin** or a
  **lib**.
- Prefer clear ownership and simple types over heavy generics or macros.
- [FILL IN any tick rate / threading rules, e.g. "fixed 50 ms tick"]

## Client rules (Ensemble)

- [FILL IN engine-specific conventions once Ensemble is set up]

---

## Git rules

- **I commit and push. You don't** -- unless I ask you to in that session.
- The whole `Opus/` folder is one **private** repo: code, docs, and assets.
  It must stay private -- it holds purchased art assets that can't be
  redistributed. Never suggest making it public or pushing it anywhere else.
- Never commit build output, runtime data, or engine caches: both `build/`
  folders, `Content/`, Rust `target/`, Unity `Library/` `Temp/`
  `Obj/` `Logs/`, Godot `.godot/`. If something like that
  shows up in `git status`, tell me and suggest a `.gitignore` line.
- Large binary assets (models, textures, audio, `.blend`, `.unitypackage`) go
  through Git LFS. If you see one about to be committed without LFS, flag it.
- Committing is part of my hand-off routine.
