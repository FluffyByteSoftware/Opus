<!--
File:       Opus/Documentation/LLM/design/soundcheck.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Soundcheck -- the launcher

Started 2026-10-02, the session after 0.0.1 went out.  Opus.Soundcheck is the program a player opens; the game
(Ensemble) is what it starts.  At start it checks every file of the installed client against the manifest
admin mode published, mends what's wrong a file at a time, then logs the player in and hands Ensemble a
ticket for UDP.  All of it is built and tested on Linux; the launcher's own restart after a patch and anything
on Windows aren't.  This file is the design as it settled, with the shapes it went through on the way, in
Jacob's words.

Why it exists at all is in LONGTERM_TODO.md ("Soundcheck, the patcher, and a certificate for every client").
The short of it: a certificate for every client (mutual TLS) needs something that runs before the game, and
a patcher is that something.  The certificate half is still to come; this is the first half.

## Settled (Jacob, 2026-10-02)

- **C# on .NET 10 with Avalonia 11** (11.3.22, the newest of the 11 line; 12 is out and untried) for the
  window.  Jacob's `dotnet --version` is 10.0.111.  It's a plain .NET program, not Unity, so it gets the
  current runtime: TLS 1.3 again (Unity's .NET has no `SslProtocols.Tls13`, which is why Conductor took 1.2
  in the first place), a fast PBKDF2 for the password's key, and `System.Text.Json` built in.
- **Where it lives**: `Soundcheck/dev/` for the project and `Soundcheck/build/` for what the compiler makes,
  beside Conductor and Ensemble.  Linux and Windows both.
- **The login moves out of the game and into the launcher**, "like Monsters and Memories did".  Soundcheck
  does everything that's TCP: the TLS connection, Hello, the version, the Login with the key, the
  "already logged in elsewhere" choice, and the Ticket.  Ensemble never speaks TCP and never sees a password;
  it opens on character select with a ticket in hand.  The login screen, `LoginConnection.cs`, the server's
  certificate copy, `PasswordKey.cs` and Remember Me left Ensemble for Soundcheck.  They're plain C# with no
  Unity in them, so they moved as they were.
- **Two modes, admin and user.**  Admin mode runs on the server's machine: Jacob ticks Linux or Windows,
  points it at that platform's build folder, and PUBLISH puts the build and its manifest in the web folder
  (below, "The flow, admin mode").  User mode is the player's: the check, the login, PLAY.  (His first shape
  had admin mode write `manifest_lin.json` into a folder of his choosing, to put up by hand, and the check
  after the login; the redesign below moved both.)
- **The manifests and the files come from a web folder, not from Conductor** (Jacob, 2026-10-02: "admin mode
  builds a working manifest for Windows and Linux -- then Soundcheck needs to know which environment its
  being run from in its user mode... and then look for that manifest which we're gonna store at this web
  address").  One manifest a platform; Soundcheck asks .NET which OS it's on (`Platforms.Here`) and fetches
  its own, plain HTTP.  A manifest says its platform inside, so the wrong file under the right name is
  caught.  The address is a constant in `Patch/ManifestSource.cs`; `--www <url>` points at another web
  folder for a test (`python3 -m http.server 8553` on this machine).  Where the *files* come from was open
  for an afternoon (Conductor over TLS, as first designed, or the same address); the redesign settled it on
  the web folder.
- **The manifest covers the whole client**, not just the world: "it needs to validate more than just the
  world... All of them."  The world is the least of it: the client carries a "broad stroke" map (region.map
  and the heights file, the ground as Conductor would build it) so the distance doesn't vanish, and the chunks
  around the player are streamed over UDP and override it.  So the world's files are lines in the manifest,
  and a changed world is a new publish (TODO.md, "The world's dump").
- **PLAY is a second login.**  The first login proves the account; PLAY logs in again with the key still in
  memory, and that Ticket starts Ensemble.  So no connection is ever held open while a player sits at the
  launcher.  SUBMIT's Ticket turns PLAY on and is dropped unlogged; PLAY finds the game first (so a login
  never happens for nothing), logs in again, and its Ticket starts the game (`GameLauncher.cs`:
  `Process.Start` with the four variables in the game's environment, the working directory the game's
  folder) and closes the window, which ends Soundcheck.  The server hands the second login a new ticket and
  lets the first die, so there's no "already logged in" in the way.  A game that won't start leaves the
  launcher open saying why, and PLAY can be pressed again.  Neither login hashes the install again: the
  check runs once, at start.
- **Where the game is**: beside the launcher, `Ensemble.x86_64` (Linux; Unity's name for it), `Ensemble`
  or `Ensemble.exe`, since the two ship as one package.  **`--game` and a path** points at a build anywhere
  (Jacob's, while Soundcheck runs out of `bin/Debug/`), and that path is remembered in
  `soundcheck_dev.json` in the player folder, because Ensemble starts Soundcheck again on its way out and
  can't pass `--game` along: the order is `--game`, beside the launcher, then the remembered path.  Under
  `dotnet run` the process can be `dotnet` itself, which started alone is no launcher, so `OPUS_SOUNDCHECK`
  is then the program beside the dll.
- **The hand-off is environment variables**, read by Ensemble at start.  On Linux a process's command line is
  readable by every user on the machine for as long as it runs, while its environment is its own user's, and
  a one-use token shouldn't sit in `ps`.  **The four** (settled with Ensemble's half, 2026-10-02; the
  contract between the two programs, `Soundcheck/dev/...` on one side and Ensemble's `Net/Ticket.cs` on
  the other): `OPUS_SERVER` (the server's address, as the player typed it), `OPUS_UDP_PORT` (the Ticket's
  port), `OPUS_TOKEN` (the Ticket's token, 64 hex) and `OPUS_SOUNDCHECK` (the launcher's own path, for the
  way back).  Ensemble takes all four out of the environment it hands Soundcheck on the way back, and puts
  **two of its own** in (2026-10-02, after a KICK showed nothing: "the client didn't show a reason"):
  `OPUS_SESSION_OVER`, why the session ended in the game's words ("You were kicked by the admin.", "Logged
  out."), and `OPUS_SESSION_TROUBLE`, `1` when it was something gone wrong.  Soundcheck shows them in its
  status box at start, red for trouble, and takes them out of the environment it starts the game with.
- **Soundcheck closes once Ensemble is up**, and **Ensemble quits back to Soundcheck** when its session ends:
  it starts Soundcheck again on its way out, so a kicked player is looking at the login.
- **Admin mode remembers its folders where Remember Me lives** (Jacob: "where the Remember Me saves is where
  this should save for the client"): `soundcheck_admin.json` in the player folder, beside
  `remembered_login.json`: a build folder per platform, and the web folder.
- **A debug mode**, for Jacob: "we will need a way to locally enter debug mode and bypass the patcher because
  I don't want to patch every time I test a fix in the game engine", and, asked: "every time I make a change
  to the client (Ensemble) I don't want to have to repatch!"  So it's about the check, not the editor: a
  changed Ensemble, built or in the editor, plays without a new publish.  Below.

## The first design, and Jacob's redesign (2026-10-02)

The first shape had the check after the login and Conductor serving the files: the client fetches the
manifest, compares, asks the server for whatever's off over the login's TLS connection, and the server sends
each file in 3 MB pieces, paced to 15 Mbps, one client at a time with the rest in line ("so it doesn't choke
the play for other users"); then the client sends its own manifest up as a report, the server checks that
against the stamp too, and a pass is the Ticket.  The client's half of the compare was built that way; the
download, the report and the server's half never were.

The same day, his redesign: "ok so what we're gonna do is zip the client up and then Conductor can dump a
portable world to the Content directory which can be sent at this point (validated against) the general
world dump will be a general shape of the world but 'smoothed'.  Redesign number 23852357235.  Patcher in
admin mode zips up the build directory (which I want to verify with you can we just combine the linux and
windows builds into the same folder?)  The patcher puts the file in /opt/storage/WWW (or whatever folder you
specify) and writes a manifest into the same folder.  This happens before I release a build to the same WWW
folder with the patcher in it.  Player launches Soundcheck... soundcheck immediately 'blocks' and starts
scanning local files and building its own manifest of its files.  Then it reaches out to my WWW
(opusensemble.duckdns.com:8553)/manifest.json.  Then compares its list against that list... if it doesn't
match it downloads the zip file, uncompresses and overwrites the local installer files (this shouldn't
effect any config files because we will not zip those in with it)".

What that changed, as read back to him: the check moves to the start, before the login, and blocks it (the
manifest coming from a web address means no login is needed to fetch it); mending is one zip from the web
folder rather than a file at a time from Conductor, so Conductor serves nothing and the download thread, the
pieces, the pacing and `patch.cfg` go; then the launcher restarts itself ("then it extracts the zip file,
restarts launcher and redoes the process"), a second fail in a row being "couldn't repair the game"; on
Windows a running program can't be written over but can be renamed, so the launcher's own files are renamed
aside first and the leftovers cleaned up at the next start; the world's dump is Conductor's, shipped with the
client and checked like any file; config files aren't in the zip (today the client keeps none in its install
folder, so nothing is excluded yet).

Settled the same day, his answers: **one zip a platform** ("1 zip a platform thats fine"; he'd wondered
whether the two builds share everything past the executable, and they don't: `Ensemble_Data/` is in both
and built per platform, the shaders compiled for each one's graphics API, the native plugins `.so` or
`.dll`); **whatever is in the build folder is in the zip, Soundcheck included if it's there** ("it _could_
be because we're gonna make it flexible for the user to decide"), so the patcher has to cope with its own
files going under it; **the address is `opusensemble.duckdns.org:8553`** (".org sorry"); **the world's
dump is a separate file**, compressed, with its own line in the manifest, "because this is much more likely
to need to be downloaded", and it isn't built or designed yet (TODO.md: Conductor's dump, and the client's
side of it).

### The shape after that (2026-10-02, the same chat; OKed, built and tested the same evening)

His next message undid the zip: "Actually we're gonna make it so the patcher knows if they're on linux or
not and looks for linux_manifest.json or windows_manifest.json :P.  Fuck it!  Then we'll reach to the
opusensemble.duckdns.org:8553/download/windows/<this will mimic the client directory so you find the file>
and the same for Linux?  our admin patcher can pack and move the files where they need to be".

So: **no zip, a file at a time, from a mirror of the client on the web folder.**

- The web folder (`/opt/storage/WWW`, served at `http://opusensemble.duckdns.org:8553/`) holds
  `linux_manifest.json` and `windows_manifest.json` at its root, and `download/linux/` and
  `download/windows/`, each an exact copy of that platform's client folder.
- **Admin mode** ticks the platform, takes the build folder, and *mirrors* it into `download/<platform>/`
  in the web folder (copies what's new or changed, takes out what the build no longer has, skips Unity's
  backup folder), then writes `<platform>_manifest.json` at the web folder's root from it.
- **User mode**, at startup, blocking the login: knows its OS, fetches its manifest, hashes the install,
  compares; each file that's missing or changed is fetched from `download/<platform>/<its path>` (the
  path's segments URL-escaped), written to a temp beside the real one, checked against the manifest's
  hash, and swapped in; then the check runs again.  A pass unlocks the login.  A second fail is "couldn't
  repair the game".  Only when one of the launcher's own files was replaced does Soundcheck start itself
  again and end (on Windows the running files are renamed aside first); a patch of the game alone needs
  no restart.
- What this buys over the zip: a player downloads only what's off, one wrong byte is one file, and most
  patches never touch the launcher, so the restart is the rare case.

**Written 2026-10-02** (Jacob's "yup"), on top of the morning's code: PATCH_MANIFEST.md at format 3 (the
names, the web folder's layout, `executable` for Linux programs, since a download comes with no
permissions); `Patch/Mirror.cs` (admin mode's copy, by size and time, stale files taken out, Unity's backup
folder skipped); `Patch/Patcher.cs` (the temp-and-swap, the hash check of each copy, the execute bit back,
the rename-aside on Windows, `CleanUp` of the leftovers at the next start, `Restart` with `--patched`);
`Patch/ManifestSource.cs` (the duckdns address, the manifest and the file fetches, one `HttpClient`, 15 s
for the manifest and no limit on a file but the window closing); the admin screen's PUBLISH (the web
folder, default `/opt/storage/WWW`); the login screen's check at start with the boxes locked, the patch,
the second check, the restart, and the game's farewell kept in front of the check's words.  The rule for
the restart: a replaced file *directly in the launcher's folder* means the launcher starts again (Unity's
top-level files trip it too, which costs a second and nothing else); with `--game` pointing elsewhere it
never does.  `--www <url>` replaced `--manifest`.

**Built and tested 2026-10-02** against the real web folder at `opusensemble.duckdns.org:8553`: PUBLISH of
the 0.0.12 build (193 files, 655.5 MB), the check at start, a two-file patch, the game's own program
deleted and fetched back runnable, an extra file left alone, the web server down, a 404, a file missing from
the web folder, a bad hash caught, `--debug`, and the farewell kept after a KICK.  **The check costs 1.3 s**
for the 655 MB (the disk cache doing most of it; a cold start will be slower).  Three things the build
taught: the platform guard the compiler knows is `OperatingSystem.IsWindows()`, not our own
`RuntimeInfo.IsWindows` (two CA1416 warnings); a worker's last "193 of 193" can reach the window after the
"Published" line and write over it, so every progress message carries the phase it was sent in and a late
one is dropped; and the check has to start from the game's *folder*, since a deleted `Ensemble.x86_64` is
exactly what it's for, and `GameLauncher.Find` had stopped it at "can't find the game".  The launcher's own
restart after a patch is untested (Parked until Soundcheck ships beside the game).

## Built and tested 2026-10-02: what the first build taught

Every check passed: the build, the window, a login, a wrong password, Remember Me across a restart, the
other-session choice against the test client, admin mode's manifest, debug mode's ticket file.

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

## The window's size (2026-10-03, session 1, written, not built yet)

Jacob: "is there any way to rely on avalonia to make the window the right size?  Like scaled?", then
"padding on the edges thats equivalent to 5% of the total size of the window (so there's plenty of space
between the text and the edge of the window)", and "if the window is 1000 pixels wide by the time its
'scaled' to fit its content we want to add 5% of the 1000 as padding".  It was a fixed 840 by 1040.

- **Avalonia fits it**: `SizeToContent="WidthAndHeight"`, at the screen's own scaling, which Avalonia
  already does for every size it's given.  Then `MainWindow`'s `FitOnce()`, when the window opens, adds 5 %
  of the fitted width and height, half on each edge, on top of the 24 round the edge it had, and fixes the
  size there.  Admin mode the same (his "yes").
- **It stays that size** ("Stay the same fixed size"), so nothing on a screen may change its height: what
  comes and goes (the status box, the progress bar, the two buttons for an account playing elsewhere) keeps
  its room while hidden (`Screens/Reserved.cs`: see-through, not clickable, not tabbable), and **the status
  box is four lines high**, scrolling past that ("Status should hold 4").  Admin mode's line under the web
  folder is two lines at most.  The other shapes were fitting the width only, and growing but never
  shrinking.

## The flow, user mode

1. The player opens Soundcheck.  The boxes are locked while the check runs (`Patch/ManifestCheck.cs`,
   `Patch/Patcher.cs`): the manifest for this OS comes from the web folder, the game's folder (beside the
   launcher, or `--game`'s) is hashed on a worker thread and held against it, and whatever's missing or
   changed is fetched a file at a time into a `.patch` temp beside the real one, its hash checked, a Linux
   program's execute bit set back, and swapped in; then the check runs again.  A pass unlocks the boxes.  A
   file that's there and not listed is said and left alone (a patcher mends, it doesn't delete).  A second
   fail is "couldn't repair the game".  A replaced file directly in the launcher's own folder means the
   launcher starts itself again with `--patched` and ends.  The manifest unreachable, or one for another
   client version, is words in the status box and no login.  `--debug` skips all of it.
2. Server, port, username, password, Remember Me, SUBMIT: the login screen as Ensemble had it.  The key is
   made from the password the same way as before (`client-security.md`), on a worker thread.  Remember Me's
   file is the same `remembered_login.json` in the same player folder
   (`~/.config/unity3d/FluffyByte/Opus.Ensemble/`, `PlayerFiles.cs`'s path), so there's one Remember Me, not
   two.
3. Soundcheck connects over TLS 1.3 to Conductor's TCP port, trusting its copy of `conductor.crt` byte for
   byte.  Hello, the version, the Login.  The server's answers are PROTOCOL.md's: Invalid Credentials, the
   other-session choice, Unavailable.  The Ticket turns PLAY on and is dropped unlogged.
4. PLAY logs in again (nothing is checked again) and that Ticket is the one Ensemble gets: Soundcheck starts
   Ensemble with the server's address, the UDP port, the token and its own path in the environment, and
   closes.  Ensemble sends Connect.  From there nothing changes: character select, the world, the chat.

The token is good once, for 30 seconds, and PLAY's login issues it, so Ensemble is started on a fresh token
and nothing waits on the check.

## The flow, admin mode

Started with `--admin` on the command line (it reads folders on this machine and talks to no server, so
there's nothing to log in to).  One screen: Linux or Windows, that platform's build folder, the version, the
web folder (`/opt/storage/WWW` by default), PUBLISH, and what it did.  PUBLISH mirrors the build into
`download/<platform>/` in the web folder (`Patch/Mirror.cs`: what's new or changed copied by size and time,
what the build no longer has taken out, Unity's `Ensemble_BackUpThisFolder_ButDontShipItWithYourGame`
skipped) and writes `<platform>_manifest.json` at the web folder's root from the mirror, so the two always
agree.  A build folder is remembered per platform, so publishing both for a release is tick, PUBLISH, tick,
PUBLISH.  The manifest never sits inside the client folder, or it would have to list itself.

## Debug mode

`--debug` on Soundcheck's command line.  The window says DEBUG MODE in its title and its corner, the login
runs as it always does, and:

- **The file check is skipped**, so a changed Ensemble, built or in the editor, plays without a new publish.
  Nothing on the server knows: one day debug mode tells the server it skipped the check, and
  `allow_debug_clients` (off by default, so nobody skips the check on a live server by typing `--debug`)
  decides whether it's let through (TODO.md, with the report up).
- **The ticket also goes to a file.**  A built Ensemble is started by Soundcheck with the ticket in its
  environment as always.  An Ensemble running inside Unity's editor is already running, so there's nothing
  to start: for it, debug mode writes `debug_ticket.json` in the player folder (the server's address, the
  UDP port, the token, when it was issued; `Net/DebugTicket.cs`), and the editor's Ensemble watches for it
  (**dev mode**, Jacob's "Play/Dev Mode", 2026-10-02): its start screen looks once a second, takes a fresh
  ticket the moment it lands, and joins the world; the same ticket is never taken twice, and one older than
  30 seconds is left alone.  So the round is Play in Unity, SUBMIT in Soundcheck, and the editor is at
  character select.  A token on the disk is a token somebody else on the machine could read, which is fine
  for debug mode and for nothing else; it's good once, for 30 seconds, so a stale file is harmless.

Debug mode is Soundcheck's switch, not Ensemble's: a built Ensemble started by Soundcheck never sees it.

## The manifest

`linux_manifest.json` and `windows_manifest.json`, one a platform, at the web folder's root: which platform
it's for, the client's version (what the Login carries), then one entry per file: its path (forward slashes,
relative to the install folder, so the same file has the same name on Linux and Windows), its size in bytes,
its SHA-256 as 64 lowercase hex, and `executable` for a Linux program.  The exact shape is a contract between
admin mode and user mode, and has its own document the way the packets and region.map have one:
PATCH_MANIFEST.md, at format 3.

SHA-256 rather than MD5: Conductor already has the `sha2` crate for the password's key, .NET has it built in,
and MD5 buys nothing here.

## Conductor's side

Nothing, today.  The manifests and the files come from the web folder, so Conductor sends nothing and serves
nothing for the patcher; the first design's download thread, pieces, pacing and `patch.cfg` went with the
redesign.  What could still be its (TODO.md, "Conductor's half", neither started): the client's report up
after the check, as the protocol's own bytes (a count, then path, size and hash each; a TCP frame is capped
at 4,096 bytes, `MAX_PACKET_BYTES`, so it would go in pieces the way Spans do, or the cap rises for it),
checked against Conductor's own copy so a changed client can't just skip the check; and `allow_debug_clients`.
The version in the Login is Soundcheck's (`ClientVersion.cs`, the csproj's `<Version>`), since it and Ensemble
ship as one package, and `client_versions` lists that number.

## Ensemble's side (built and tested 2026-10-02)

- **Lost the login screen and the TCP half of `Assets/Code/Net/`**: `LoginConnection.cs`,
  `ServerCertificate.cs`, the `Security/` folder, the login's eleven widget files, its layout and style, and
  the certificate copy in `Data/Certs/`.  `Protocol.cs` keeps the packet table whole (it mirrors PROTOCOL.md)
  and lost only LoginResult's answers, SessionChoice's values and the secret word.
- **Reads the ticket, and Soundcheck's path, from the environment at start** (`Net/Ticket.cs`, the four
  variables above), and sends Connect at once, so the first screen a player sees is character select.
- **The start screen** (`start_default.json`, `start.uss`; `start_background`, `start_logo`, `start_card`) is
  what shows when there's no ticket: "Start Forgotten Legends from the launcher." and QUIT.  It's also
  the screen behind "Joining the world..." for the moment before Welcome, and where a session that ends
  with the game still open says why.  In the editor its card is dev mode (above).
- **When the UDP session ends** (kicked, the server gone, LOG OUT, `/camp`), it starts Soundcheck from
  `OPUS_SOUNDCHECK`, with the four variables taken out of the environment it hands over, and quits.  `/camp
  desktop` and QUIT quit without.  A game with no launcher path (started by hand, or the editor) stays open
  on the start screen instead.
- **The world's files** (not this pass) go in `Assets/StreamingAssets/World/`.  A Unity build packs everything under
  `Assets/` into its own archives; `StreamingAssets/` is the one folder it copies as loose files
  (`Opus.Ensemble_Data/StreamingAssets/`), and a patcher writes loose files.  That's a fifth folder of ours
  under `Assets/`, into the `.gitignore` with its `.meta`.

## Open

- **The world's dump**, and the world's files on the client: TODO.md, "Soundcheck".
- **Windows**: the builds of both programs, the rename-aside and the restart, a log file: TODO.md.
- **One package**, Soundcheck and Ensemble together: TODO.md.
- **The certificate for every client**: LONGTERM_TODO.md.  Soundcheck is where it goes.
