<!--
File:       Opus/Documentation/LLM/design/soundcheck.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Soundcheck -- the launcher

Started 2026-10-02, the session after 0.0.1 went out.  Opus.Soundcheck is the program a player opens; the game
(Ensemble) is what it starts.  It logs the player in, checks every file of the installed client against what
we shipped, mends what's wrong, and hands Ensemble a ticket for UDP.  Nothing is built yet: this file is the
design as it settles, and the open questions at the bottom are open.

Why it exists at all is in LONGTERM_TODO.md ("Soundcheck, the patcher, and a certificate for every client").
The short of it: a certificate for every client (mutual TLS) needs something that runs before the game, and
a patcher is that something.  The certificate half is still to come; this is the first half.

## Settled (Jacob, 2026-10-02)

- **C# on .NET 10 with Avalonia** for the window.  Jacob's `dotnet --version` is 10.0.111.  It's a plain
  .NET program, not Unity, so it gets the current runtime: TLS 1.3 again (Unity's .NET has no
  `SslProtocols.Tls13`, which is why Conductor took 1.2 in the first place), a fast PBKDF2 for the password's
  key, and `System.Text.Json`.
- **The login moves out of the game and into the launcher**, "like Monsters and Memories did".  Soundcheck
  does everything that's TCP today: the TLS connection, Hello, the version, the Login with the key, the
  "already logged in elsewhere" choice, and the Ticket.  Ensemble never speaks TCP and never sees a password;
  it opens on character select with a ticket in hand.  The login screen, `LoginConnection.cs`, the server's
  certificate copy, `PasswordKey.cs` and Remember Me leave Ensemble for Soundcheck.  They're plain C# with no
  Unity in them, so they move as they are.
- **After the login, before Ensemble, the manifest check.**  "This happens AFTER LOGIN ONLY BUT BEFORE WE GO
  TO ENSEMBLE."  Soundcheck hashes every file of the installed client and sends the list; Conductor compares
  it with the one we stamped at build time, names the files that are wrong, and sends them; Soundcheck
  writes them and checks again.  A second failure is an error on Soundcheck's screen, not a third try.  A
  pass puts PLAY on the screen, and PLAY starts Ensemble.
- **The manifest covers the whole client**, not just the world: "it needs to validate more than just the
  world... All of them."  A tool run at build time walks the built client, hashes every file, and writes
  `manifest.json`; that file is the stamp, and Conductor gets a copy with the files it names.
- **The world is the least of it**: the client carries a "broad stroke" map (region.map and the heights file,
  the ground as Conductor would build it) so the distance doesn't vanish, and the chunks around the player
  are streamed over UDP and override it.  So the world's files are two more lines in the manifest, and a
  changed world is a new stamp.
- **The download is paced**: 15 Mbps, one client at a time, everybody else in line and told their place, "so
  it doesn't choke the play for other users".
- **Conductor serves the files itself**, over the same TLS connection the login came in on.  No file server
  beside it.  (I asked about an HTTPS server; Jacob's sketch has Conductor do it.)

## The flow

1. The player opens Soundcheck, types the username and password, or Remember Me fills them in.  The key is
   made from the password the same way as today (`client-security.md`), on a worker thread.
2. Soundcheck connects over TLS 1.3 to Conductor's TCP port, trusting its copy of `conductor.crt` byte for
   byte.  Hello, the version, the Login.  The server's answers are PROTOCOL.md's: Invalid Credentials, the
   other-session choice, Unavailable.
3. **New**: the password passed, and instead of the Ticket the server asks for the manifest.  Soundcheck
   sends it (every file: its path under the install folder, its size, its SHA-256).
4. The server compares.  Every file matches: the Ticket, and on to 7.  Some don't: the server names them,
   and Soundcheck goes in the download line.
5. In line, Soundcheck hears its place now and then ("2 ahead of you").  At the front, the server sends the
   named files, paced to the limit.  Soundcheck writes each to a temp file and swaps it in.
6. Soundcheck hashes again and sends the manifest again.  A pass is the Ticket.  A second fail is an error
   the player reads ("Couldn't repair the game: <file>"), the server closes, and that's that.
7. PLAY.  Soundcheck starts Ensemble with the server's address, the UDP port and the token, and Ensemble
   sends Connect.  From there nothing changes: character select, the world, the chat.

The Ticket's token is good once, for 30 seconds.  Since it comes after the files pass, Ensemble is started on
a fresh token and nothing waits on a download.  Where PLAY sits against that clock is open (below).

## The manifest

`manifest.json`, at the root of the installed client.  One entry per file: its path (forward slashes, relative
to the install folder, so the same file has the same name on Linux and Windows), its size in bytes, and its
SHA-256 as 64 lowercase hex.  Plus the client's version, which is what the Login carries.  The exact shape is
a contract between the stamp tool, Soundcheck and Conductor, and gets its own document the way the packets
and region.map have one; it's written when the first one is made.

SHA-256 rather than MD5: Conductor already has the `sha2` crate for the password's key, .NET has it built in,
and MD5 buys nothing here.

What's sent over TCP is the manifest in the protocol's own bytes (a count, then path, size and hash each),
not the JSON text.  The JSON is the file on disk.  A TCP frame is capped at 4,096 bytes today
(`MAX_PACKET_BYTES`), and a manifest of a Unity build is a few hundred files, so either the cap rises for
these packets or the manifest goes in pieces the way Spans do.

## Conductor's side

- **The stamped client lives in `Content/`**, a copy of exactly what shipped (Soundcheck and Ensemble both)
  with its `manifest.json` at the root, gitignored like the logs and the world.  The folder's name is open
  (below).  At START SERVER the files are checked against the manifest once, so Conductor never sends a file
  that wouldn't pass; a mismatch is a Warn and the downloads are refused until it's fixed.
- **The download thread.**  The login pool is 8 threads (`login_threads`) with a 30-second deadline, and a
  download holds a connection for minutes, so it can't run there.  The manifest check itself is quick and
  stays on the login thread; a connection that needs files is handed, TLS and all, to one downloader thread
  with a queue, and the login thread goes back to logins.  The 15 Mbps is that thread pacing its writes.
  The limit and the queue's depth go in `networking.cfg` (soft).
- **Reading `manifest.json`.**  Conductor has no JSON reader: the web admin's `json.rs` only writes.  Either
  the `serde_json` crate (with `serde`), or a reader by hand for this one flat shape.  Jacob's call, below.
- **The protocol**: three or four packets after the Login (the server's ask, the manifest, the verdict with
  the files to replace, a file's bytes in pieces), `PleaseWait` reused for the place in line, and
  `PROTOCOL_VERSION` bumps.  Conductor speaks the same bytes to Soundcheck as it did to Ensemble up to the
  Login, so `test_client.py` keeps working with a `--manifest` of its own.
- **The version in the Login** becomes Soundcheck's: it and Ensemble ship as one package, and
  `client_versions` lists that number.

## Ensemble's side

- Loses the login screen and the TCP half of `Assets/Code/Net/`.  Starts on character select.
- Reads the ticket Soundcheck hands it.  Environment variables, most likely: on Linux a process's command
  line is readable by every user on the machine for as long as it runs, while its environment is its own
  user's, and a one-use token shouldn't sit in `ps`.  Open, below.
- **When the UDP session ends** (kicked, the server gone, `/camp`) there's no login screen to go back to.
  Open, below: quit back to Soundcheck, or a "disconnected" screen with one button.
- **The world's files** go in `Assets/StreamingAssets/World/`.  A Unity build packs everything under
  `Assets/` into its own archives; `StreamingAssets/` is the one folder it copies as loose files
  (`Opus.Ensemble_Data/StreamingAssets/`), and a patcher writes loose files.  That's a fifth folder of ours
  under `Assets/`, into the `.gitignore` with its `.meta`.

## The stamp tool

Run once per release on the built client folder, before it's zipped: walks it, hashes every file, writes
`manifest.json`, and that folder goes to the players and to Conductor's `Content/`.  Whether it's a
command-line mode of Soundcheck itself (`Opus.Soundcheck --stamp <folder>`, so there's one program and no
new name) or a small program of its own is open, below.  RELEASE.md gets the step either way.

## What this is not, yet

- **Soundcheck patching itself.**  A running program can't overwrite its own files on Windows.  The first
  step: Soundcheck's own files are in the manifest, but one of them wrong is an error telling the player to
  download the launcher again, not a patch.  Writing the new file beside and swapping on the next start is
  later.
- **A new build of Ensemble as a patch.**  The manifest carries it the same way as any file, but a Unity
  build changes hundreds of files at once and that's a long download at 15 Mbps.  Whether a new build is a
  patch or a fresh download is a call for when there's a second build.
- **The certificate for every client.**  LONGTERM_TODO.md.  Soundcheck is where it goes.

## Open

- **Where PLAY sits.**  Between the files passing and PLAY, a player may sit for minutes, and holding that
  connection open idle is a thread each (every wait is the OS's own, no polling).  The plain way: the Ticket
  isn't issued at the pass; PLAY opens a new connection and logs in again with the key still in memory,
  sends the manifest again (it's small, and it's the real check), and that pass gets the Ticket.  The cost is
  a second Argon2 in Security's line per play.  The other way: the whole thing happens on PLAY, one login.
- **The stamp tool's name**, or whether it's a mode of Soundcheck.
- **`serde_json`** or a reader by hand.
- **The folder in `Content/`** for the stamped client.  `Content/client/`?
- **Soundcheck's folder**: `Soundcheck/dev/Opus.Soundcheck/` and `Soundcheck/build/`, beside the other two.
- **The ticket's hand-off**: environment variables, or the command line.
- **Ensemble when its session ends**: quit, or a "disconnected" screen.
- **Soundcheck while the game runs**: stays open, or closes.
- **Platforms**: Linux and Windows both, like Ensemble.
