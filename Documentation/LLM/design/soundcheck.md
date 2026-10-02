<!--
File:       Opus/Documentation/LLM/design/soundcheck.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Soundcheck -- the launcher

Started 2026-10-02, the session after 0.0.1 went out.  Opus.Soundcheck is the program a player opens; the game
(Ensemble) is what it starts.  It logs the player in, checks every file of the installed client against the
manifest we stamped, mends what's wrong, and hands Ensemble a ticket for UDP.  The login, PLAY, admin mode's
manifests and the check are built; mending (the download) isn't.  This file is the design as it settles.

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
- **Two modes, admin and user.**  Admin mode runs on the server's machine: Jacob ticks Linux or Windows,
  points it at the "correct" client folder for that platform and presses WRITE MANIFEST, which reads every
  file, takes its length and its checksum, and writes that platform's manifest, `manifest_lin.json` or
  `manifest_win.json`, in the folder he gives.  User mode is the player's: it logs in, fetches the manifest
  for the OS it's on, and checks itself against it.
- **One manifest a platform, from a web address, not from Conductor** (Jacob, 2026-10-02: "admin mode builds
  a working manifest for Windows and Linux -- then Soundcheck needs to know which environment its being run
  from in its user mode... and then look for that manifest which we're gonna store at this web address").
  The two files sit at `http://opusensemble.com:8553/manifest_lin.json` and
  `http://opusensemble.com:8553/manifest_win.json`, plain HTTP; Soundcheck asks .NET which OS it's on
  (`Platforms.Here`) and fetches its own.  A manifest says its platform inside, so the wrong file under the
  right name is caught.  The address is a constant in `Patch/ManifestSource.cs`; `--manifest <url>` points
  at another copy for a test (a `python3 -m http.server 8553` on this machine).  So the stamp doesn't come
  down the TCP connection after the login, as first designed (below): Conductor never sends it.  Where the
  *files* come from when something's off is still open: Conductor over TLS as designed, or the same web
  address.
- **The check is the client's first, then the server's.**  "Most of the time its just gonna be a legit reason
  and not a hacker."  After the login the player is made to download the manifest.  Soundcheck compares its
  own files with it; whatever's off, it asks for; the server sends those files; Soundcheck compares again.
  Then it sends its own manifest up as a report, and the server checks that against the stamp too.  A pass is
  the Ticket.  A second fail on the client's side is an error the player reads, not a third try.  **The
  client's half of the compare is built** (2026-10-02, `Patch/ManifestCheck.cs`): after SUBMIT's Ticket the
  manifest is fetched, the install (the game's folder) is hashed with the same walk admin mode uses, and PLAY
  comes alive only when every listed file is there with the same size and hash.  A listed file missing or
  changed fails it, with the names in the status box and "install the game again", since the download isn't
  built; a file that's there and not listed is said and left alone (a patcher mends, it doesn't delete).  A
  manifest for another client version stops at "download the launcher again".  `--debug` skips the check.
  The report up and the server's half aren't built.
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
  open while a player sits at the launcher.  **Built and tested 2026-10-02**: SUBMIT's Ticket
  turns PLAY on; PLAY finds the game first (so a login never happens for nothing), logs in again, and its
  Ticket starts the game (`GameLauncher.cs`: `Process.Start` with the four variables in the game's
  environment, the working directory the game's folder) and closes the window, which ends Soundcheck.  The
  server hands the second login a new ticket and lets the first die, so there's no "already logged in" in
  the way.  A game that won't start leaves the launcher open saying why, and PLAY can be pressed again.
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
  it starts Soundcheck again on its way out, so a kicked player is looking at the login.  Ensemble finds
  Soundcheck through one more environment variable, its path.
- **Admin mode remembers its folder where Remember Me lives** (Jacob: "where the Remember Me saves is where
  this should save for the client"): `soundcheck_admin.json` in the player folder, beside
  `remembered_login.json`.
- **A debug mode**, for Jacob: "we will need a way to locally enter debug mode and bypass the patcher because
  I don't want to patch every time I test a fix in the game engine", and, asked: "every time I make a change
  to the client (Ensemble) I don't want to have to repatch!"  So it's about the check, not the editor: a
  changed Ensemble, built or in the editor, plays without a new stamp.  Below.

## Jacob's redesign (2026-10-02, after the two manifests went up; not built, not yet OKed)

His words, the same day the two manifests and the check were pushed: "ok so what we're gonna do is zip the
client up and then Conductor can dump a portable world to the Content directory which can be sent at this
point (validated against) the general world dump will be a general shape of the world but 'smoothed'.
Redesign number 23852357235.  Patcher in admin mode zips up the build directory (which I want to verify
with you can we just combine the linux and windows builds into the same folder?)  The patcher puts the file
in /opt/storage/WWW (or whatever folder you specify) and writes a manifest into the same folder.  This
happens before I release a build to the same WWW folder with the patcher in it.  Player launches
Soundcheck... soundcheck immediately 'blocks' and starts scanning local files and building its own manifest
of its files.  Then it reaches out to my WWW (opusensemble.duckdns.com:8553)/manifest.json.  Then compares
its list against that list... if it doesn't match it downloads the zip file, uncompresses and overwrites the
local installer files (this shouldn't effect any config files because we will not zip those in with it)".

What changes against what's built, as read back to him (his answers go here as they come):

- **The check moves to the start**, before the login, and blocks it: the boxes are locked while the
  install is hashed and compared, and unlock on a pass.  (It was after SUBMIT's Ticket, from his earlier
  "AFTER LOGIN ONLY BUT BEFORE WE GO TO ENSEMBLE"; the manifest coming from a web address means no login
  is needed to fetch it, so the earlier rule falls away.)
- **Mending is one zip**, not a file at a time: a fail downloads the whole client zip from the web folder,
  checks its hash against the manifest, unzips it over the install, and checks again.  So Conductor
  serves nothing, and the download thread, the 3 MB pieces, the pacing and `patch.cfg` go.  The cost is
  that one wrong byte is the whole zip again, hundreds of megabytes for a Unity build; his call.
- **Then the launcher restarts itself** (his next line: "then it extracts the zip file, restarts launcher
  and redoes the process"): the unzip done, Soundcheck starts a new copy of itself and ends, and the new
  one checks from the top, so a patched Soundcheck is the one that goes on.  A second fail in a row
  (the new run started with `--patched`, say) is "couldn't repair the game", not another download.  On
  Windows a running program can't be written over but can be renamed, so the launcher's own files are
  renamed aside before the new ones go in, and the leftovers are cleaned up on the next start.
- **One `manifest.json`**, not one a platform: it names both zips, and each zip's file list.  The address
  is a duckdns name now, on the same port.
- **Admin mode zips** the build folder into the WWW folder and writes the manifest beside it.
- **The world's dump** is Conductor's (a portable, "smoothed" shape of the world, dumped to `Content/`),
  shipped with the client and checked like any file.  Its own session; TODO.md.
- **Config files aren't in the zip.**  Today the client keeps no config file in its install folder: every
  player file is in the player folder (`~/.config/unity3d/FluffyByte/Opus.Ensemble/`), and Soundcheck's
  `conductor.crt` sits beside the launcher.  So nothing is excluded yet; the rule stands for when one comes.

Settled the same day, his answers: **one zip a platform** ("1 zip a platform thats fine"; he'd wondered
whether the two builds share everything past the executable, and they don't: `Ensemble_Data/` is in both
and built per platform, the shaders compiled for each one's graphics API, the native plugins `.so` or
`.dll`); **whatever is in the build folder is in the zip, Soundcheck included if it's there** ("it _could_
be because we're gonna make it flexible for the user to decide"), so the patcher has to cope with its own
files going under it; **the address is `opusensemble.duckdns.org:8553`** (".org sorry"); **the world's
dump is a separate file**, compressed, with its own line in the manifest, "because this is much more likely
to need to be downloaded", and it isn't built or designed yet (TODO.md: Conductor's dump, and the client's
side of it).

### The shape after that (2026-10-02, the same chat; not built, not yet OKed)

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
  patches never touch the launcher, so the restart is the rare case.  The manifest built today (format 2,
  the platform inside, the file list) is already this shape; only its name and the download base change.

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

## The flow, user mode

1. The player opens Soundcheck.  Server, port, username, password, Remember Me, SUBMIT: the login screen as
   Ensemble had it.  The key is made from the password the same way as today (`client-security.md`), on a
   worker thread.  Remember Me's file is the same `remembered_login.json` in the same player folder
   (`~/.config/unity3d/FluffyByte/Opus.Ensemble/`, `PlayerFiles.cs`'s path), so there's one Remember Me, not
   two.
2. Soundcheck connects over TLS 1.3 to Conductor's TCP port, trusting its copy of `conductor.crt` byte for
   byte.  Hello, the version, the Login.  The server's answers are PROTOCOL.md's: Invalid Credentials, the
   other-session choice, Unavailable.
3. **Built**: the Ticket comes, and Soundcheck fetches the manifest for its OS from the web address and
   hashes every file under its install folder (the game's folder: beside the launcher, or `--game`'s) and
   compares.  Fetching it, or a manifest for another version, failing is words in the status box and no PLAY.
4. Everything matches: Soundcheck sends its manifest up as the report, the server compares it with the stamp,
   and the Ticket comes.  On to 7.
5. Something doesn't (**not built**: today this is "install the game again" and no PLAY): Soundcheck asks
   for the files by name and goes in the download line.  In line it
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
there's nothing to log in to).  One screen: Linux or Windows, that platform's client folder, the version, the
folder to write in, WRITE MANIFEST, and where it wrote.  It walks the folder, hashes every file, and writes
`manifest_lin.json` or `manifest_win.json` there; Jacob puts the two up at the web address.  A folder is
remembered per platform, so writing both for a release is tick, write, tick, write.  The manifest never
sits inside the client folder, or it would have to list itself, and Unity's
`Ensemble_BackUpThisFolder_ButDontShipItWithYourGame` is left out, since the package leaves it out.

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
  Ensemble watches for it (**dev mode**, Jacob's "Play/Dev Mode", 2026-10-02): its start screen looks once a
  second, takes a fresh ticket the moment it lands, and joins the world; the same ticket is never taken
  twice, and one older than 30 seconds is left alone.  So the round is Play in Unity, SUBMIT in Soundcheck,
  and the editor is at character select.  A token on the disk is a token somebody else on the machine
  could read, which is fine for debug mode and for nothing else; it's good once, for 30 seconds, so a stale
  file is harmless.  Today, with PLAY not built, the file is all debug mode does with the ticket.

Debug mode is Soundcheck's switch, not Ensemble's: a built Ensemble started by Soundcheck never sees it.

## The manifest

`manifest_lin.json` and `manifest_win.json`, one a platform.  Which platform it's for, the client's version
(what the Login carries), then one entry per file: its path (forward slashes, relative to the install
folder, so the same file has the same name on Linux and Windows), its size in bytes, and its SHA-256 as 64
lowercase hex.  The exact shape is a contract between admin mode, user mode and Conductor, and has its own
document the way the packets and region.map have one: PATCH_MANIFEST.md, at format 2.

SHA-256 rather than MD5: Conductor already has the `sha2` crate for the password's key, .NET has it built in,
and MD5 buys nothing here.

The stamp comes down from the web address as the JSON itself (above), so no packet carries it.  If the
report still goes up over TCP for the server's half, it goes as the protocol's own bytes (a count, then path,
size and hash each), not the JSON text; a TCP frame is capped at 4,096 bytes today (`MAX_PACKET_BYTES`), and
a Unity build is a few hundred files, so it would go in pieces the way Spans do, or the cap rises for it.

## The pieces

Each file is sent as it is, in 3 MB pieces, each piece one frame with the file's path, the piece's number and
how many there are.  Not zipped: a Unity build is mostly assets Unity has already compressed, so a zip would
save little and cost a crate in Conductor (the `zip` crate) to make one.  Jacob had both ways in an earlier
project ("one if its more than one file we zip them... Or we just chunk each file into 3 MB chunks"); this is
the plain one, and it needs nothing new on either side.  A frame of 3 MB means the cap rises for a download,
on the downloader thread only.

## Conductor's side

- **Nothing of it is built**, and the manifest coming from the web address (2026-10-02) shrinks it: Conductor
  never sends the stamp.  What's left, if the files come from Conductor: `Content/patch/` with the two
  manifests and the correct client folders Conductor sends files from (whether they sit in `Content/patch/`
  too or are pointed at by a setting is open, below; either way gitignored like the logs and the world); at
  START SERVER the files checked against the manifests once, so Conductor never sends a file that wouldn't
  pass, a mismatch a Warn and the downloads refused until it's fixed (how long that takes on a Unity build is
  a guess until it's measured).  If the files come from the web address instead, none of this, and
  Conductor's half is the report and `allow_debug_clients` only.
- **The download thread.**  The login pool is 8 threads (`login_threads`) with a 30-second deadline, and a
  download holds a connection for minutes, so it can't run there.  Sending the manifest and reading the
  report are quick and stay on the login thread; a connection that asks for files is handed, TLS and all, to
  one downloader thread with a queue, and the login thread goes back to logins.  The 15 Mbps is that thread
  pacing its writes; when the download's done the connection goes back for the report and the Ticket.
- **`patch.cfg`** (soft), most likely: the limit in Mbps, the piece size, the client folder.  A new config
  file is one entry in Constellations' table and shows up on the Settings tab on its own.
- **The protocol**: the report (one way, up), the ask for files, a file's piece, and the download's end;
  `PleaseWait` reused for the place in line; `PROTOCOL_VERSION` bumps.  Conductor speaks the same bytes
  to Soundcheck as it did to Ensemble up to the Login, so `test_client.py` keeps working with a
  `--client-folder` of its own.
- **The version in the Login** becomes Soundcheck's: it and Ensemble ship as one package, and
  `client_versions` lists that number.

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

- **Where the files come from when the check fails**: Conductor over the login's TLS connection, in 3 MB
  pieces, paced (the design above), or the same web address the manifests are at, one plain GET a file
  (far less to build in Conductor, and the web server does the pacing).  Jacob's call.  Until it's made, a
  fail says "install the game again".
- **Where the correct client folder is on the server**: inside `Content/patch/` beside the manifest, or
  anywhere, pointed at by a setting in `patch.cfg`.  Admin mode's remembered folder is settled (the player
  folder), but Conductor doesn't read that, so it still has to be told.
- **Which comes first to build**: answered by doing it.  Soundcheck's user mode came first (2026-10-02),
  then Ensemble's half and PLAY the same day, then the two manifests and the check; the download and
  Conductor's half are what's left.