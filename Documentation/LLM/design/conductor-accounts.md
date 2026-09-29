<!--
File:       Opus/Documentation/LLM/design/conductor-accounts.md
Component:  Documentation
Author:     Jacob Chacko
-->

# conductor-accounts

A lib crate.  An account as the server holds it in memory: read from its row in the `accounts` table,
changed in memory while it's held, and dumped back to the row when it's let go.  Jacob's design,
2026-09-29.  Not a server piece on its own: nothing to start or stop, and everything it does goes
through Archivist, so it works exactly while Archivist does.

## Skeleton

```
conductor-accounts/
├── Cargo.toml     depends on conductor-tools, nothing else
└── src/
    └── lib.rs     struct Account; load(), password_hash(), create(); Account::new(), save(), changed()
```

## The account

`Account` holds every column but the password hash:

- read only, the row's own: `id()` (`None` until there's a row), `uuid()`, `username()`,
  `created_at()` (`None` until there's a row)
- changeable, plain public fields: `first_name`, `last_name`, `email`, `last_login`

The hash stays out on purpose.  An account gets printed and logged, and later shown on the page; a hash
never should be.  The login reads it on its own with `password_hash()`, and `create()` takes it beside
the account.

The times are `SystemTime`, which the `postgres` crate reads from a `TIMESTAMPTZ` with no extra
features.  Shown to a person, they go through the clock and end in `Z`.

## Loading, saving, making

- `load(name)` -> `Pending<Option<Account>>`.  `None` if there's no such account.
- `account.save()` -> `Option<Pending<u64>>`.  Writes the changeable fields back, found by `id`.  The
  account remembers what the row held when it was last read or written; if nothing differs, nothing is
  sent and it hands back `None` (Jacob: "the same if it's the same, don't even bother writing").  It
  counts as saved once the job is in Archivist's mailbox; a failed write is logged by Archivist and not
  retried.  An account with no row can't be saved (a Warn: it's a bug).
- `Account::new(name, first, last, email)` makes one in memory, with a UUID from Fingerprinter and no
  row.  `create(account, hash)` -> `Pending<Account>` writes it as a new row and hands it back with its
  `id` and `created_at`.  Nothing checks the name or the email here yet; the table does.  Meant for the
  web admin's game account management, which is next to build on it.

`load()`, `password_hash()` and `create()` are each an Archivist transaction, only so the row can be
turned into the answer on Archivist's thread.

## Who holds one

The login (`tcp.rs`) reads the hash, checks the password, deals with an account already in the world,
then loads the `Account` and puts the login time on it in memory.  The ticket holds it, then the
player (`sessions.rs`).  Whenever a ticket or player leaves the book, for any reason (Goodbye, gone
quiet, logged out by another login, kicked, banned, a ticket that ran out or was replaced, STOP
SERVER), its account is saved once the book's lock is let go.

The load comes after the other session is logged out on purpose.  Archivist has one worker and runs
jobs in order, so the kicked session's save is in the row before the new load reads it.

`last_login_datetime` is written when the player leaves, not when they log in.  If Conductor dies
without a clean stop (Ctrl-C isn't caught yet), the login time of everyone in the world is lost.
