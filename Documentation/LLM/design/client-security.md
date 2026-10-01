<!--
File:       Opus/Documentation/LLM/design/client-security.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Security in the client: the password's key

Started 2026-10-01.  Jacob: "even though its going over TLS we don't want to save it to their local disk as
plain text!"  The password a player types never crosses the internet and never lands on a disk as typed: the
client turns it into a **key** the moment SUBMIT is pressed, and the key is what's sent and what Remember Me
keeps.  "I suppose there will be a few moments where its in memory": the typed string sits in the client's
memory until it's reused, since C# can't wipe a string.

**The client's half is built and tested** (Ensemble, 2026-10-01, all six checks passed; the key matched
Python's to the byte).  **Conductor's half is
written down below and not built**: it's a session of its own, and until it's in, a key sent as the password
won't match any account.  Nothing sends one yet (there's no network client).

What a key does and doesn't do, said when it was planned: the key logs in as well as the password would, so
whoever copies the Remember Me file can log in as that player.  What they can't do is learn the password and
try it on the player's email or bank.  The same goes for Conductor's memory: it only ever sees the key.

## The key: the contract

Ensemble and Conductor both make it, and they have to make it the same way to the byte.  Once accounts exist,
changing any of this locks everybody out, so it changes only with every account deleted, or with a new
`v2` salt and a way to move accounts over.

- **PBKDF2 with HMAC-SHA256.**  Jacob: "whatever will work with the _server_".  Built into Unity's .NET
  (`Rfc2898DeriveBytes`), Python's standard library (`hashlib.pbkdf2_hmac`) and the browser; Conductor needs
  a crate (below).  Argon2 on the client would have meant a C# package.
- **The password**: its UTF-8 bytes, as typed.  The password rules (Security's `check_password_rules()`)
  already keep it to printable ASCII, so there's only one way to write it.
- **The salt**: the UTF-8 bytes of `Opus login v1:` followed by the username with A to Z made lowercase and
  nothing else touched (Conductor's `to_ascii_lowercase()`).  So `Jacob_01` and `jacob_01` make the same key.
- **600,000 rounds**, 32 bytes out, **settled** (Jacob, 2026-10-01: "Make this the full 600,000").  Timed in
  the Unity editor at **2852 ms** (Python: 150 ms).  It already runs on a worker thread, so the screen keeps
  drawing; Jacob: "it should happen in the background while the player moves forward in login", meaning
  (his pick) that once the net code is in, SUBMIT connects to the server while the key is made, and the
  Login goes the moment the key's ready.
- **Written as 64 lowercase hex characters**, and that string is what goes in the Login packet where the
  password was.
- **Worked example**: username `jacob_01`, password `Correct horse 1!` (PROTOCOL.md's example) makes
  `fc71f0c94665dfd6ff4e217891cd7ff81c8fd8ed7b20cf498c122c1bbd1f8855`.  Ensemble, `test_client.py` and Conductor
  are each checked against it.

## Ensemble's half (built and tested)

In `Assets/Code/`:

- **`Security/PasswordKey.cs`**: `Make()` (the recipe above) and `MakeAsync()`, which runs it on a worker
  thread so the screen keeps drawing; `LooksLikeKey()`; `AsciiLower()`.  The bytes it makes are wiped after;
  the strings can't be.
- **`Security/RememberedLogin.cs`**: Remember Me's file, `remembered_login.json` in Unity's
  folder every player file goes in (Jacob: "we'll make our own file and save it in the preferred users
  directory"), `PlayerFiles.cs`: `~/.config/unity3d/FluffyByte/Opus.Ensemble/` on Linux,
  `%USERPROFILE%\AppData\LocalLow\FluffyByte\Opus.Ensemble\` on Windows.  (It was Unity's
  `persistentDataPath` at first, `.../FluffyByte/Opus_Ensemble/`; moved at the hand-off.)  It holds the server IP,
  the port, the username and the key, never the password.  Written to a temp file and swapped in.  A file
  that doesn't hold up (no username, a key that isn't 64 hex) is ignored with a warning.
- **`Hud/Widgets/LoginForm.cs`**: where the login's widgets meet.  Each hands its box over as it's built, so
  SUBMIT can read them, and a remembered login fills them in.
  - **SUBMIT**: the password comes out of the box at once (dots stand in), the key is made off the main
    thread, and the Console says how long it took.  Then Remember Me ticked writes the file, and unticked
    deletes it.  It still doesn't log in; the network client sends the key when it comes.  The key is never
    logged.
  - **A remembered login**: the boxes fill from the file, Remember Me is ticked, and the Password box shows a
    stand-in row of dots (Jacob: yes).  Clicking in clears it so typing starts fresh; clicking out of an empty
    box puts it back.  Typing a password drops the remembered key, and so does changing the username, since
    the key was made from it.

## Conductor's half (to build, a session of its own)

1. **The Login carries the key**: `PROTOCOL_VERSION` 7.  The Login's fourth string is the password's key, 64
   lowercase hex, made as above.  `protocol.rs`, PROTOCOL.md (its example gets the key above) and
   `test_client.py` change together; `test_client.py` makes the key with `hashlib.pbkdf2_hmac`.
2. **A login that isn't a key is refused without a hash.**  In `tcp.rs`, beside the username check: anything
   that isn't 64 of `0-9a-f` is Invalid Credentials at once.  The shape is no secret, so a quick answer gives
   nothing away.  Otherwise the key goes through `security::verify_password()` / `verify_no_account()` just
   as the password did: Security, Argon2 and the line don't change.
3. **The Accounts tab: Conductor makes the key** (Jacob: "we'll have conductor do it").  The admin types the
   password on the page; it goes to Conductor on `127.0.0.1` (never the internet), and the account desk
   (`desk.rs`) checks it against the password rules, makes the key from the account's name and the password,
   and hands the key to `security::hash_password()`.  The stored hash is Argon2 of the key.  The plaintext is
   dropped there.
   - **This needs two crates, `pbkdf2` and `sha2`** (RustCrypto, the same people as the `argon2` crate
     already in use).  Asked again when that session plans, since every new dependency is OKed first.
   - Making a key takes a moment (600,000 rounds); it runs on the desk's thread, the same as the hash.
4. **Every account is deleted** (Jacob: "we'll delete all accounts then").  Their stored hashes are of the
   password, so none would match a key.  Jacob deletes them (DELETE ACCOUNT on the Accounts tab, or SQL that
   session writes for him to run), then makes them again.  Their characters go with them.

## Later

- **The Remember Me file is readable by other users on the same Linux machine** (Unity's .NET can't set a
  file's permissions without reaching into the OS).  TODO.md.
- **The client checking the server's certificate** (`Content/certs/conductor.crt`), and Soundcheck handing
  each client a certificate of its own: TODO.md, LONGTERM_TODO.md.
