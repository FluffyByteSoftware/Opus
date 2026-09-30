<!--
File:       Opus/Documentation/LLM/design/conductor-accounts.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-accounts

A lib crate.  The one way in to the `accounts` table: nothing else in Conductor writes SQL for it.  Once
`player_characters` exists, that table is this crate's too (Jacob, 2026-09-30: making or deleting a
character writes both, so one crate writes them, in one transaction).  Everything goes through Archivist
(and, for the account desk, Security), so it works exactly while they do: only while the server is running.

## Skeleton

```
accounts/
├── Cargo.toml     depends on conductor-tools, nothing else
└── src/
    ├── lib.rs     struct Account (Account::new(), save()); enum Created, enum Edited
    │                load(), list(), password_hash(), taken(), create(), edit(), stamp_login(),
    │                set_password(), delete(); the field checks (username_allowed(), check_*())
    └── desk.rs    the account desk: start(), stop(), hand_in(Job) -> Result<number, words>,
                     outcome(number) -> Option<Outcome { progress, text }>
                     enum Job { Create, Password }, enum Progress { Working, Done, Failed }
```

## An account is never held

Jacob's rule, 2026-09-29.  Whatever needs an account reads it from its row when it needs it, and every
change goes straight back to the row.  The row is the only copy, so nothing can write an old copy over a
new one (an edit from the web admin while the player is online would otherwise be lost when they leave).
When the game needs something from an account during play, it asks Archivist the same way, and never
waits on the answer.

So networking's book has the account's name and nothing else, and nothing is saved when a player leaves.
The one write networking makes is the login time (`stamp_login()`), straight to the row the moment a
ticket is used and the player is in the world over UDP.  That's the moment Jacob picked for playtime
("it's their UDP connection time we want"), and a Conductor that dies without a clean stop loses nobody's.

## The account

`Account` is a look at a row, from `load()` or `list()`: every column but the password hash.

- read only: `id()` and `created_at()` (`None` until there's a row), `uuid()`, `username()`, `last_login()`
- changeable, plain public fields: `first_name`, `last_name`, `email`; `save()` writes them straight back

The username never changes once the account is made: it's what the player logs in with (Jacob,
2026-09-29).  The hash stays out on purpose.  An account gets printed, logged and shown on the page; a
hash never should be.  The login reads it on its own with `password_hash()`; `create()` and
`set_password()` take it on its own.

The times are `SystemTime`, which the `postgres` crate reads from a `TIMESTAMPTZ` with no extra features.
Shown to a person, they go through the clock and end in `Z`.

## The functions

Each hands back Archivist's `Pending`.

- `load(name)` -> `Option<Account>`.  `list()` -> every account, by name.
- `password_hash(name)` -> `Option<String>`, for the login.
- `taken(name, email)` -> whether each is in use (the email whatever the capitals, the way the table's
  index sees it).  The account desk asks it before a new account's password costs a hash.
- `create(account, hash)` -> `Created`: `Made(account)` with its id and `created_at`, or `NameTaken` or
  `EmailTaken`, checked in the same transaction as the insert.  The account comes from `Account::new()`,
  which gives it a UUID from Fingerprinter and no row.
- `edit(name, first, last, email)` -> `Edited`: `Saved`, `NoSuchAccount`, or `EmailTaken` (another
  account's), and nothing written then.
- `stamp_login(name)`: the login time, now.  Nobody waits on it.
- `set_password(name, hash)` and `delete(name)` -> how many rows, 0 for no such account.  Deleting doesn't
  touch networking; the web admin kicks the player once the row is gone.
- `account.save()`: the names and email back to the row, by name, no checks.  Nothing calls it yet; it's
  the plain way for something that loaded an account to write it back.  An account with no row is a Warn
  and nothing is sent.

`load()`, `list()`, `password_hash()`, `taken()`, `create()` and `edit()` are Archivist transactions, so
the rows are turned into answers on Archivist's thread (and, for `create()` and `edit()`, so the check
and the write happen together).

## The rules

The table checks the name and the email itself.  Conductor checks them first too, so the admin gets a
sentence instead of a database error:

- `username_allowed()` / `check_username()`: 8 to 32 of `a-z`, `0-9` and `_`, the table's rule.  The
  login uses it too, to fail a name that couldn't be an account without a hash.
- `check_owner_name()`: not empty, not only spaces, no tab or line break.
- `check_email()`: the table's loose rule, by hand: something, an @, then something with a dot in it, no
  spaces.
- `check_new()`, `check_owner()` and `check_new_password()` run them all and hand back every complaint
  with its field's name in front (`email: ...`), so the page can put each beside its field.  A new
  password is Security's `check_password_rules()`, then the two copies have to match (Jacob: typed twice,
  every time).

## The account desk (`desk.rs`)

Making an account and changing a password both need a hash, and a hash waits in Security's line with
the logins.  The web admin answers one request at a time, so it can't wait on one.  The desk is a thread
of its own (`account-desk`) that takes those jobs one at a time: `hand_in(job)` gives back a number at
once (or words, if the desk is stopped), and `outcome(number)` says working, done or failed, with words.
The newest 50 outcomes are kept, in memory only, never with the password.

- **Create**: `taken()` first, so a name or email in use costs no hash; then the hash; then `create()`.
- **Password**: the hash, then `set_password()`.  It takes at the account's next login; a player in the
  world stays in.

A server piece: the launcher starts it after Security and Archivist and stops it before them, and
`stop()` finishes every job already handed in first, so none is left half done.  It's on the Services
tab as "Account desk".  It doesn't check in with `seen()`: it sleeps until a job comes.

## Deleting

The web admin deletes the row, and only once it's gone asks networking to `terminate()` the account: its
player, if there's one in the world, hears Kicked, reason 5, account terminated (the client says ACCOUNT
TERMINATED, Jacob's words), and any unused ticket dies.  Their Connections row reads "LINKDEAD: account
terminated".  A login already past its password check in the few milliseconds between the delete and the
kick could still get a ticket; its login time would write to no row.  Not worth closing while the admin
is the only one who deletes.
