<!--
File:       Opus/Documentation/HowTo/INSTALLATION_INSTRUCTIONS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Installing Forgotten Legends 0.0.1

How to get a released package running: the server (Conductor) on one machine, the client (Ensemble) on
the machines of whoever is playing.  This is for the packages on a GitHub Release, not for building from
source; building is in README.md.  It's early days, so the install is by hand and there are a few files to
edit.  What 0.0.1 does: a player logs in, picks a character, and stands in the world chatting with whoever
else is there.

The repo is private, so only people with access to it can get the packages, and they can't be passed on
(the client has purchased art in it).

## What you need

**For the server:**

- A Linux machine, x86_64.  Nobara and Fedora are what it's run on.  (It builds on Windows too, but a
  Windows package hasn't been tried with a database yet.)
- **PostgreSQL 18**, installed and running on the same machine.  18, not older: the tables use
  `uuidv7()`, which came in 18.  Keep it listening on `localhost` only, which is the default.
- **openssl**, to make the server's TLS certificate (the `openssl` command-line tool, which most Linux
  installs already have).
- A browser on the same machine, for the web admin.

**For a player:**

- A machine the client runs on (the package says which: Linux or Windows, x86_64).
- The server's address, and an account made for them on the server.  Players can't make their own.

## The server

### 1. Unpack

Unpack the Conductor package wherever you like.  Inside is the program and its `Content/` folder:

```
Opus-Conductor-0.0.1/
├── conductor-launcher
└── Content/
    ├── cfg/       the settings files
    ├── certs/     the TLS certificate (the key goes here too; step 3)
    ├── scripts/   the Lua scripts
    └── psql/      the database's table definitions, which Conductor runs itself
```

Keep the two together: Conductor finds `Content/` by walking up from the folder it's run in.  If you want
it somewhere else, set `OPUS_CONTENT` to the folder's path before running it.

### 2. The database

Conductor wants a database called `opusdb` and a role called `opus_game` that owns it.  As the Postgres
superuser (`sudo -u postgres psql` on Fedora), with a password of your own in place of `newpass`:

```
CREATE ROLE opus_game LOGIN PASSWORD 'newpass';
CREATE DATABASE opusdb OWNER opus_game;
```

That's all the SQL there is.  Conductor makes its own tables the first time it connects, and brings them
up to date itself after that (`Content/psql/`).  The role connects over TCP to `localhost:5432` with its
password, which is Postgres's default setup on Fedora; if yours asks for something else, `pg_hba.conf` is
where that's decided, and it isn't ours to change.

### 3. The TLS certificate

Players log in over TLS, and the server needs a certificate and its private key.  The package has the
certificate the clients were built to trust (`Content/certs/conductor.crt`) but not its key: a key never
goes in a download.  So either:

- **You were given the key** that goes with that certificate.  Put it beside the certificate as
  `Content/certs/conductor.key`.  This is the usual case for 0.0.1: the clients only trust that one
  certificate.
- **Or make a new pair.**  From the package's folder:

  ```
  openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes -keyout Content/certs/conductor.key -out Content/certs/conductor.crt -days 3650 -subj "/CN=Opus Conductor" -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
  ```

  But know that the client checks the certificate's bytes, not its name, and only trusts the one it was
  built with.  A new certificate means a client built with a copy of it, which for 0.0.1 means a new client
  package.  So for now, get the key.

Without a key Conductor still runs; the network just stays down, and the log and the Services tab say
why.

### 4. The settings

Everything is in `Content/cfg/`, one `key = value` a line, every setting with a comment above it saying
what it does.  Three to look at before the first run:

- **`postgres.cfg`**: `password = ` the password you gave `opus_game` in step 2.  The rest is right for a
  Postgres on the same machine.
- **`networking.cfg`**: `bind_address = ` the address players will connect to.  It ships as `10.0.0.84`,
  which is one particular machine on one particular LAN, so change it: your machine's LAN address, or
  `0.0.0.0` for every address the machine has.  The ports are `9997` (login, TCP) and `9998` (the game,
  UDP); both have to be reachable from the players' machines, so open them on the firewall if there is
  one.
- **`wgui.cfg`**: the web admin's two passwords, `admin` and `user` out of the box.  Change them.  The
  web admin only listens on the server machine itself, so they're about who at the keyboard may do what,
  not about the outside world.

The rest can wait.  A changed `postgres.cfg`, `networking.cfg` or `game.cfg` takes on the next START
SERVER; a changed `conductor_globals.cfg` or `wgui.cfg` needs Conductor run again.

### 5. Run it

From the package's folder:

```
./conductor-launcher
```

The terminal shows the log and takes no typing.  Everything is done from the web admin: open
<http://127.0.0.1:9996/Opus> in a browser on the same machine and log in as `admin`.

Press **START SERVER** on the Server tab.  The first start makes the world, which takes a while (the
Services tab shows GameWorld working on it; at the shipped `world_size` of 32 it's a minute or more, and
the Storage tab watches the files land in `Content/world/`).  The door for players only opens once the
ground around the spawn is in, and the Services tab's networking lines say "Listening on" when it has.

If the Services tab says Archivist can't connect, it's step 2 or the password in step 4.  If it says the
TLS files are missing, it's step 3.

### 6. Accounts

Players can't make accounts; you do, on the web admin's Accounts tab, with the server running: NEW
ACCOUNT, a username (8 to 32 of `a-z`, `0-9` and `_`), the owner's names and email, a password typed twice.
The player makes their characters themselves, in the game.

### 7. Stopping

**SHUT DOWN** on the Server tab stops the server (every player is told, and the world is saved) and then
Conductor itself, writing out everything it holds first.  Ctrl-C in the terminal kills it outright and can
lose whatever hadn't been written yet, so use the button.

## The client

These are the released 0.0.1's steps, where the game logs in itself.  Since then the login has moved into
Soundcheck, the launcher, which does the login and starts the game; the next release ships the two
together, and these steps change with it.

1. Unpack the Ensemble package anywhere and run the program in it (`Opus.Ensemble` on Linux,
   `Opus.Ensemble.exe` on Windows).
2. On the login screen, the **Server IP** is the server's `bind_address` from step 4 (or the machine's
   address, if that was `0.0.0.0`), and the **Server Port** is `9997`.  They're filled in with one
   particular machine's, so change them.
3. Log in with the username and password the admin made.  Remember Me keeps the password's key, never the
   password, in your own user folder.
4. Pick a character, or CREATE one, and PLAY.  You're standing at 0,0,0 with the chat window at the bottom
   left: `/chat hello` says hello to everybody in the world, `/who` lists who's there, and `/camp` logs
   out to the login screen (`/camp desktop` closes the game).

Every file the game keeps for you is in one folder: `~/.config/unity3d/FluffyByte/Opus.Ensemble/` on
Linux.

## When it doesn't work

- **"Outdated Client Failure" at login**: the client's version isn't in `client_versions` in
  `networking.cfg`.  Add it there (a comma-separated list) and STOP SERVER, START SERVER.
- **The login just fails**: the wrong password, or an account that doesn't exist, or the client's secret
  word doesn't match `secret_word` in `networking.cfg` (both ship as `potato`).  All three get the same
  answer on purpose.  The server's log (the Log tab) says which.
- **The client can't reach the server at all**: the address and port on the login screen, the firewall on
  the server (TCP 9997 and UDP 9998 both), or the server's door isn't open yet (the Services tab).
- **"ACCOUNT TERMINATED"**: the admin deleted the account while you were playing.

The log is in `Content/logs/`, one file a day, and the web admin's bell shows anything that went wrong
until it's acknowledged.
