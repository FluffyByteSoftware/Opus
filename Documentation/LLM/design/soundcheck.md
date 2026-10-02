<!--
File:       Opus/Documentation/LLM/design/soundcheck.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Soundcheck -- the launcher

Started 2026-10-02, the session after 0.0.1 went out.  Opus.Soundcheck is the program a player opens; the game
(Ensemble) is what it starts.  It logs the player in, checks every file of the installed client against the
manifest we stamped, mends what's wrong, and hands Ensemble a ticket for UDP.  Nothing is built yet: this file
is the design as it settles.

Why it exists at all is in LONGTERM_TODO.md ("Soundcheck, the patcher, and a certificate for every client").
The short of it: a certificate for every client (mutual TLS) needs something that runs before the game, and
a patcher is that something.  The certificate half is still to come; this is the first half.

## Settled (Jacob, 2026-10-02)

- **C# on .NET 10 with Avalonia 11** (11.3.22, the newest of the 11 line; 12 is out and untried) for the
  window.  Jacob's `dotnet --version` is 10.0.111.  **Built 2026-10-02**: the login works over TLS 1.3, the
  key takes 208 ms.  It's a plain
  .NET program, not Unity, so it gets the current runtime: TLS 1.3 again (Unity's .NET has no
  `SslProtocols.Tls13`, which is why Conductor took 1.2 in the first place), a fast PBKDF2 for the password's
  key, and `System.Text.Json` built in.
- **Where it lives**: `Soundcheck/dev/` for the project and `Soundcheck/build/` for what the compiler makes,
  beside Conductor and Ensemble.  Linux and Windows both.
- **The login moves out of the game and into the launcher**, "like Monsters and Memories did".  Soundcheck
  does everything that's TCP today: the TLS connection, Hello, the version, the Login with the key, the
  "already logged in elsewhere" choice, and the Ticket.  Ensemble never speaks TCP and never sees a password;
  it opens on character select with a ticket in hand.  The login screen, `LoginConnection.cs`, the server's
  certificate copy, `PasswordKey.cs` and Remember Me leave Ensemble for Soundcheck.  They're plain C# with no
  Unity in them, so they move as they are.
- **Two modes, admin and user.**  Admin mode runs on the server's machine: Jacob points it at the "correct"
  client folder and presses WRITE MANIFEST, which reads every file, takes its length and its checksum, and
  writes `patch_manifest.json`.  That file goes to `Content/patch/patch_manifest.json`.  User mode is the
  player's: it logs in, is handed that manifest, and checks itself against it.
- **The check is the client's first, then the server's.**  "Most of the time its just gonna be a legit reason
  and not a hacker."  After the login the player is made to download the manifest.  Soundcheck compares its
  own files with it; whatever's off, it asks for; the server sends those files; Soundcheck compares again.
  Then it sends its own manifest up as a report, and the server checks that against the stamp too.  A pass is
  the Ticket.  A second fail on the client's side is an error the player reads, not a third try.
- **The manifest covers the whole client**, not just the world: "it needs to validate more than just the
  world... All of them."  The world is the least of it: the client carries a "broad stroke" map (region.map
  and the heights file, the ground as Conductor would build it) so the distance doesn't vanish, and the chunks
  around the player are streamed over UDP and override it.  So the world's files are two more lines in the
  manifest, and a changed world is a new stamp.
- **The download is paced**: 15 Mbps, one client at a time, everybody else in line and told their place, "so
  it doesn't choke the play for other users".  Files go in 3 MB pieces (below).
- **Conductor serves the files itself**, over the same TLS connection the login came in on.  No file server
  beside it.  `serde` and `serde_json` come in for reading the manifest (Jacob: "yes add it").
- **PLAY is a second login.**  The first login patches; PLAY logs in again with the key still in memory, the
  check runs again (quick: nothing's wrong), and that Ticket starts Ensemble.  So no connection is ever held
  open while a player sits at the launcher.
- **The hand-off is environment variables**, read by Ensemble at start.  On Linux a process's command line is
  readable by every user on the machine for as long as it runs, while its environment is its own user's, and
  a one-use token shouldn't sit in `ps`.
- **Soundcheck closes once Ensemble is up**, and **Ensemble quits back to Soundcheck** when its session ends:
  it starts Soundcheck again on its way out, so a kicked player is looking at the login.  Ensemble finds
  Soundcheck through one more environment variable, its path.
- **Admin mode remembers its folder where Remember Me lives** (Jacob: "where the Remember Me saves is where
  this should save for the client"): `soundcheck_admin.json` in the player folder, beside
  `remembered_login.json`.
- **A debug mode**, for Jacob: "we will need a way to locally enter debug mode and bypass the patcher because
  I don't want to patch every time I test a fix in the game engine", and, asked: "every time I make a change
  to the client (Ensemble) I don't want to have to repatch!"  So it's about the check, not the editor: a
  changed Ensemble, built or in the editor, plays without a new stamp.  Below.

## Built 2026-10-02: what the first build taught

- **The key takes 208 ms** on .NET 10 against Unity's 2852.
- **Avalonia 11.3.2 was two years stale**: 11.3.22 is the newest of the 11 line, and the NuGet warning on
  the DBus package it pulled in (`NU1903`) went with the bump.  12 is out and untried.
- **The crash on close is Avalonia's, on KDE's Wayland session** (issue 19523, open): after the window's
  thread stops, a late DBus message is handed to it, the DBus library takes the throw for a broken
  connection, tells its listeners on the same dead thread, and that one is uncaught.  Not our threads (the
  login's had ended seconds before), not the input method (none set), not the global menu (off, and still
  crashed).  The way round it: on Avalonia's `Exit` event, which fires while the thread is still alive,
  Soundcheck ends the process itself and skips Avalonia's tidy-up.  A launcher that has just started the game
  has nothing left to tidy.

## The flow, user mode

1. The player opens Soundcheck.  Server, port, username, password, Remember Me, SUBMIT: the login screen as
   Ensemble had it.  The key is made from the password the same way as today (`client-security.md`), on a
   worker thread.  Remember Me's file is the same `remembered_login.json` in the same player folder
   (`~/.config/unity3d/FluffyByte/Opus.Ensemble/`, `PlayerFiles.cs`'s path), so there's one Remember Me, not
   two.
2. Soundcheck connects over TLS 1.3 to Conductor's TCP port, trusting its copy of `conductor.crt` byte for
   byte.  Hello, the version, the Login.  The server's answers are PROTOCOL.md's: Invalid Credentials, the
   other-session choice, Unavailable.
3. **New**: the password passed, and instead of the Ticket the server sends the manifest.  Soundcheck hashes
   every file under its install folder and compares.
4. Everything matches: Soundcheck sends its manifest up as the report, the server compares it with the stamp,
   and the Ticket comes.  On to 7.
5. Something doesn't: Soundcheck asks for the files by name and goes in the download line.  In line it
   hears its place now and then ("2 ahead of you", `PleaseWait`).  At the front, the server sends each file
   in 3 MB pieces, paced to the limit.  Soundcheck writes the pieces to a temp file beside the real one and
   swaps it in when the last piece lands, so a download that dies halfway leaves the old file whole.
6. Soundcheck compares again.  A pass goes to 4.  A second fail is an error the player reads ("Couldn't
   repair the game: <file>") and the connection closes.
7. PLAY shows.  Pressing it logs in again (2 to 4, nothing to download) and the Ticket from that login
   is the one Ensemble gets: Soundcheck starts Ensemble with the server's address, the UDP port, the token
   and its own path in the environment, and closes.  Ensemble sends Connect.  From there nothing changes:
   character select, the world, the chat.

The token is good once, for 30 seconds, and it's issued after the files pass, so Ensemble is started on a
fresh token and nothing waits on a download.

## The flow, admin mode

Started with `--admin` on the command line (it reads a folder on this machine and talks to no server, so
there's nothing to log in to).  One screen: the client folder, WRITE MANIFEST, and where it wrote.  It walks
the folder, hashes every file, and writes `patch_manifest.json` where Jacob says; he puts it in
`Content/patch/`.  The manifest never sits inside the client folder, or it would have to list itself.

## Debug mode

`--debug` on Soundcheck's command line.  The window says DEBUG MODE in its title and its corner, the login
runs as it always does, and the point of it is the first of these:

- **The file check is skipped.**  Once the check exists, debug mode tells the server it isn't going to send a
  manifest, and the server lets it through only if `patch.cfg` says debug clients are allowed (off by
  default, so nobody skips the check on a live server by typing `--debug`).  A server that doesn't allow it
  refuses the login with words that say so.
- **The ticket also goes to a file.**  A built Ensemble is started by Soundcheck with the ticket in its
  environment as always (once PLAY is built).  An Ensemble running inside Unity's editor is already running,
  so there's nothing to start: for it, debug mode writes `debug_ticket.json` in the player folder (the
  server's address, the UDP port, the token, when it was issued; `Net/DebugTicket.cs`), and the editor's
  Ensemble reads it when PLAY is pressed there.  A token on the disk is a token somebody else on the machine
  could read, which is fine for debug mode and for nothing else; it's good once, for 30 seconds, so a stale
  file is harmless.  Today, with PLAY not built, the file is all debug mode does with the ticket.

Debug mode is Soundcheck's switch, not Ensemble's: a built Ensemble started by Soundcheck never sees it.

## The manifest

`patch_manifest.json`.  The client's version (what the Login carries), then one entry per file: its path
(forward slashes, relative to the install folder, so the same file has the same name on Linux and Windows),
its size in bytes, and its SHA-256 as 64 lowercase hex.  The exact shape is a contract between admin mode,
user mode and Conductor, and gets its own document the way the packets and region.map have one; it's written
with the first one.

SHA-256 rather than MD5: Conductor already has the `sha2` crate for the password's key, .NET has it built in,
and MD5 buys nothing here.

Over TCP the manifest goes as the protocol's own bytes (a count, then path, size and hash each), the same
packet down (the stamp) and up (the report), not the JSON text.  The JSON is the file on disk.  A TCP frame is
capped at 4,096 bytes today (`MAX_PACKET_BYTES`), and a Unity build is a few hundred files, so the manifest
goes in pieces the way Spans do, or the cap rises for these packets.

## The pieces

Each file is sent as it is, in 3 MB pieces, each piece one frame with the file's path, the piece's number and
how many there are.  Not zipped: a Unity build is mostly assets Unity has already compressed, so a zip would
save little and cost a crate in Conductor (the `zip` crate) to make one.  Jacob had both ways in an earlier
project ("one if its more than one file we zip them... Or we just chunk each file into 3 MB chunks"); this is
the plain one, and it needs nothing new on either side.  A frame of 3 MB means the cap rises for a download,
on the downloader thread only.

## Conductor's side

- **`Content/patch/`**: `patch_manifest.json`, and the correct client folder Conductor sends files from.
  Whether the folder sits in `Content/patch/` too or is pointed at by a setting is open (below).  Either
  way it's gitignored like the logs and the world.  At START SERVER the files are checked against the
  manifest once, so Conductor never sends a file that wouldn't pass; a mismatch is a Warn and the downloads
  are refused until it's fixed.  How long that check takes on a Unity build is a guess until it's measured.
- **The download thread.**  The login pool is 8 threads (`login_threads`) with a 30-second deadline, and a
  download holds a connection for minutes, so it can't run there.  Sending the manifest and reading the
  report are quick and stay on the login thread; a connection that asks for files is handed, TLS and all, to
  one downloader thread with a queue, and the login thread goes back to logins.  The 15 Mbps is that thread
  pacing its writes; when the download's done the connection goes back for the report and the Ticket.
- **`patch.cfg`** (soft), most likely: the limit in Mbps, the piece size, the client folder.  A new config
  file is one entry in Constellations' table and shows up on the Settings tab on its own.
- **The protocol**: the manifest packet (both ways), the ask for files, a file's piece, and the download's
  end; `PleaseWait` reused for the place in line; `PROTOCOL_VERSION` bumps.  Conductor speaks the same bytes
  to Soundcheck as it did to Ensemble up to the Login, so `test_client.py` keeps working with a
  `--client-folder` of its own.
- **The version in the Login** becomes Soundcheck's: it and Ensemble ship as one package, and
  `client_versions` lists that number.

## Ensemble's side

- Loses the login screen and the TCP half of `Assets/Code/Net/`.  Starts on character select.
- Reads the ticket, and Soundcheck's path, from the environment at start.  No ticket there is a screen
  saying to start the game from the launcher, and a button that closes.
- When the UDP session ends (kicked, the server gone, `/camp`), it starts Soundcheck and quits.  `/camp
  desktop` quits without.
- **The world's files** go in `Assets/StreamingAssets/World/`.  A Unity build packs everything under
  `Assets/` into its own archives; `StreamingAssets/` is the one folder it copies as loose files
  (`Opus.Ensemble_Data/StreamingAssets/`), and a patcher writes loose files.  That's a fifth folder of ours
  under `Assets/`, into the `.gitignore` with its `.meta`.

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

- **Where the correct client folder is on the server**: inside `Content/patch/` beside the manifest, or
  anywhere, pointed at by a setting in `patch.cfg`.  Admin mode's remembered folder is settled (the player
  folder), but Conductor doesn't read that, so it still has to be told.
- **Which comes first to build**: Soundcheck's user mode against today's Conductor (it logs in and gets the
  Ticket, with no manifest yet), or Conductor's side.
