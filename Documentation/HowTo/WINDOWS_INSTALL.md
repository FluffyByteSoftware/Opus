<!--
File:       Opus/Documentation/HowTo/WINDOWS_INSTALL.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Building Conductor on Windows

Conductor is written and tested on Linux.  The Windows code is wired in behind the same functions, and
this is what it took to build Conductor on a Windows laptop.  Everything here was done on 2026-09-30, the
first time Conductor was built on Windows.  It built with no errors and no warnings, Conductor runs, and
START SERVER works.  The laptop has no Postgres, so Archivist times out and the data tabs stay locked, which
is what it should do.  So far this covers the build and a first run.

The commands are for `cmd`, the plain Windows terminal, sitting in `Conductor\dev` in the clone.  None of
them need an admin terminal; the Build Tools installer asks for permission on its own.

## What it needs

- **Git**, to clone the repo.  The laptop uses Git GUI, which comes with Git for Windows.
- **Rust** through `rustup`, on the **MSVC toolchain**.  This is the one that matters.  Rust on Windows
  comes in two flavors: MSVC, which builds with Microsoft's tools, and GNU, which builds with MinGW's.  We
  use MSVC, because `mlua` builds Lua from its C source and on Windows that's Visual Studio's compiler.
- **Build Tools for Visual Studio**, with "Desktop development with C++" (the C compiler `cl.exe`, the
  linker `link.exe` and the Windows SDK).  VS Code is a different product and doesn't count.

## Step by step

**1. Put Rust on MSVC.**  If Rust was installed on the GNU toolchain, this switches it:

```
rustup default stable-x86_64-pc-windows-msvc
```

To check, run `rustc -vV`.  The `host:` line should end in `windows-msvc`.

**2. Install the Build Tools**, with the C++ part.  It's a few GB and takes a while:

```
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

Without `winget`, download "Build Tools for Visual Studio" from
`https://visualstudio.microsoft.com/visual-cpp-build-tools/` and tick **Desktop development with C++**.
Leave the Windows SDK ticked.

**3. Close the terminal and open a new one**, so it sees the new tools.

**4. Build.**  The `cargo clean` is only needed if an earlier build on the other toolchain left half-built
pieces behind:

```
cargo clean
```
```
cargo build
```

**5. Run it**, the same as on Linux:

```
cargo run -p conductor-launcher
```

Then open <http://127.0.0.1:9996/Opus>, log in as `admin` (the password is in `Content\cfg\wgui.cfg`), and
press START SERVER.  Conductor finds `Content\` by walking up from `Conductor\dev`, so nothing needs setting.

## What went wrong on the way

These are the errors we hit, in the order we hit them.

- **`error calling dlltool 'dlltool.exe': program not found`**, from `windows-sys` and `which`.  Rust was
  on the GNU toolchain, which wants MinGW's `dlltool` to build those two.  It wasn't a bug in our code; the
  build never got that far.  Step 1 fixes it.
- **`timeout reading rustc version`** after switching toolchains.  It looks bad but isn't: `rustup` got
  tired of waiting for `rustc` to say its version, probably because Windows Defender was scanning the new
  `rustc.exe` the first time it ran.  The switch had still worked.
- **`linker 'link.exe' not found`**, from the build scripts of `getrandom`, `proc-macro2` and friends.
  Rust was on MSVC now, but Microsoft's linker wasn't installed.  Step 2 fixes it.

`cmd` has `dir`, not `ls`.

## Not tried yet

The server on Windows with a database, which needs PostgreSQL 18 on the machine and the TLS certificate
(README.md has the `openssl` command; Git for Windows carries an `openssl.exe` under
`C:\Program Files\Git\usr\bin`).  The monitor's kernel32 code, Fingerprinter's `BCryptGenRandom()` and the
rest of the server's Windows paths only show once the database is there and the tabs unlock.

GameWorld came after that first build, so a build with it in, and a world made on Windows, haven't been tried
either.  This file grows as they are.
