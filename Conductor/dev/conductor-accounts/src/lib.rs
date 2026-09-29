//! File:       Opus/Conductor/dev/conductor-accounts/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Accounts: an account as the server holds it in memory.  `load()` reads
//! one from its row in the accounts table, the server changes it in
//! memory for as long as it's held, and `save()` writes it back.  A save
//! with nothing changed since the row was last read or written doesn't
//! go to the database at all.  `create()` writes a new one, made in
//! memory with `Account::new()`, as a new row.  Jacob's design,
//! 2026-09-29: the account is held with its player from the login until
//! they leave the world, and dumped to the database then.
//!
//! The password hash is not in `Account`, on purpose.  An account gets
//! printed, logged and put on the page; a hash never should be.  The
//! login reads it on its own with `password_hash()`, and `create()` takes
//! it beside the account.
//!
//! Everything here goes through Archivist and hands back a `Pending`, so
//! nothing ever waits on the database unless it chooses to.  Archivist
//! has one worker and does its jobs in the order they came, so a save
//! sent before a load is always in the row by the time the load reads it.

use std::io;
use std::time::SystemTime;

use conductor_tools::archivist::{self, Param, Pending, Row};
use conductor_tools::fingerprinter;
use conductor_tools::scribe::{self, Channel};

/// Every column but the password hash.  The UUID comes out as text: the
/// `postgres` crate won't read a UUID column into a `String` on its own.
const LOAD_SQL: &str = "SELECT id, uuid::text AS uuid, account_username, owner_first_name, owner_last_name, \
    owner_email, created_at, last_login_datetime FROM accounts WHERE account_username = $1";

/// The one read of the hash, for the login.
const HASH_SQL: &str = "SELECT password_hash FROM accounts WHERE account_username = $1";

/// What `save()` writes: the parts of an account that can change.  Found
/// by `id`, which never changes.
const SAVE_SQL: &str = "UPDATE accounts SET owner_first_name = $1, owner_last_name = $2, owner_email = $3, \
    last_login_datetime = $4 WHERE id = $5";

/// A new row.  Postgres hands back the two things only it knows.
const CREATE_SQL: &str = "INSERT INTO accounts (uuid, account_username, owner_first_name, owner_last_name, \
    owner_email, password_hash, last_login_datetime) VALUES ($1::text::uuid, $2, $3, $4, $5, $6, $7) \
    RETURNING id, created_at";

/// An account, in memory.
///
/// The owner's names and email and the last login are there to be
/// changed; `save()` writes them.  The id, the UUID, the name and when it
/// was made are the row's own and are only read.
#[derive(Debug)]
pub struct Account {
    /// The row's number.  `None` for an account made in memory that isn't
    /// in the table yet.
    id: Option<i64>,
    uuid: String,
    username: String,
    /// `None` until the row exists: Postgres stamps it.
    created_at: Option<SystemTime>,

    pub first_name: String,
    pub last_name: String,
    pub email: String,
    /// `None` until the first login.
    pub last_login: Option<SystemTime>,

    /// What the row held the last time we read it or wrote it, so a save
    /// with nothing new can be skipped.  `None` while there's no row.
    stored: Option<Changeable>,
}

/// The parts of an account that can change, on their own, so they can be
/// compared with what the row held.
#[derive(Debug, Clone, PartialEq)]
struct Changeable {
    first_name: String,
    last_name: String,
    email: String,
    last_login: Option<SystemTime>,
}

impl Account {
    /// A new account, in memory only, with a UUID from Fingerprinter.  It
    /// has no row until `create()` writes one.  Nothing is checked here:
    /// the accounts table checks the name and the email itself.  Fails
    /// only if the OS won't give random bytes.
    pub fn new(username: &str, first_name: &str, last_name: &str, email: &str) -> io::Result<Account> {
        Ok(Account {
            id: None,
            uuid: fingerprinter::new_uuid()?,
            username: username.to_string(),
            created_at: None,
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            last_login: None,
            stored: None,
        })
    }

    /// The row's number, once there is a row.
    pub fn id(&self) -> Option<i64> {
        self.id
    }

    /// The game's name for the account, unique across the whole server.
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    /// The name it logs in with, lowercase.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// When the row was made, once there is a row.
    pub fn created_at(&self) -> Option<SystemTime> {
        self.created_at
    }

    /// True if anything has changed since the row was last read or written,
    /// or there's no row yet.
    pub fn changed(&self) -> bool {
        self.stored.as_ref() != Some(&self.changeable())
    }

    /// Writes what has changed back to the account's row.  `None` if
    /// there was nothing to write, and nothing goes to the database.
    ///
    /// The account counts as saved the moment the job goes in Archivist's
    /// mailbox, not when it's done.  If the write fails, Archivist logs it,
    /// and the account isn't sent again unless something else changes.
    /// Nobody waits on it: it's called as a player leaves, and there's
    /// nothing to do for them either way.
    ///
    /// An account with no row can't be saved, only created.  That's a bug
    /// in the caller, so it's a Warn.
    pub fn save(&mut self) -> Option<Pending<u64>> {
        let Some(id) = self.id else {
            scribe::warn(Channel::Database, &format!("Account {} was saved before it had a row.  Nothing was \
                written; create() makes the row.", self.username));
            return None;
        };
        let now = self.changeable();
        if self.stored.as_ref() == Some(&now) {
            return None;
        }

        let params: Vec<Param> = vec![Box::new(now.first_name.clone()), Box::new(now.last_name.clone()),
                                      Box::new(now.email.clone()), Box::new(now.last_login), Box::new(id)];
        self.stored = Some(now);
        Some(archivist::execute(SAVE_SQL, params))
    }

    fn changeable(&self) -> Changeable {
        Changeable {
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            email: self.email.clone(),
            last_login: self.last_login,
        }
    }

    /// An account from a row of `LOAD_SQL`.
    fn from_row(row: &Row) -> Account {
        let mut account = Account {
            id: Some(row.get("id")),
            uuid: row.get("uuid"),
            username: row.get("account_username"),
            created_at: Some(row.get("created_at")),
            first_name: row.get("owner_first_name"),
            last_name: row.get("owner_last_name"),
            email: row.get("owner_email"),
            last_login: row.get("last_login_datetime"),
            stored: None,
        };
        account.stored = Some(account.changeable());
        account
    }
}

/// Reads an account from its row.  `None` if there's no account by that
/// name.  The name is taken as it is; the table's names are lowercase.
// Rust note: this is a transaction only because a transaction's closure
// runs on Archivist's thread, so the row can be turned into an Account
// there and handed back ready.  It only reads.
pub fn load(username: &str) -> Pending<Option<Account>> {
    let username = username.to_string();
    archivist::transaction("load an account", move |tx| {
        let rows = tx.query(LOAD_SQL, &[&username])?;
        Ok(rows.first().map(Account::from_row))
    })
}

/// The account's password hash, for the login to hand to Security.
/// `None` if there's no account by that name.
pub fn password_hash(username: &str) -> Pending<Option<String>> {
    let username = username.to_string();
    archivist::transaction("read a password hash", move |tx| {
        let rows = tx.query(HASH_SQL, &[&username])?;
        Ok(rows.first().map(|row| row.get("password_hash")))
    })
}

/// Writes an account made with `Account::new()` as a new row, with the
/// password hash from Security beside it, and hands it back with its id
/// and when it was made.  If the table turns it away (a name or email
/// already taken or not allowed, or an account that already has a row),
/// the error says why, and the account is gone with it: the caller still
/// has whatever it was made from.
pub fn create(mut account: Account, password_hash: String) -> Pending<Account> {
    archivist::transaction("make an account", move |tx| {
        let row = tx.query_one(CREATE_SQL, &[&account.uuid, &account.username, &account.first_name,
                                             &account.last_name, &account.email, &password_hash,
                                             &account.last_login])?;
        account.id = Some(row.get("id"));
        account.created_at = Some(row.get("created_at"));
        account.stored = Some(account.changeable());
        Ok(account)
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// An account as if it had just been read from row 7.
    fn loaded() -> Account {
        let mut account = Account::new("throwaway_01", "Throw", "Away", "throwaway@example.com").unwrap();
        account.id = Some(7);
        account.created_at = Some(SystemTime::now());
        account.stored = Some(account.changeable());
        account
    }

    #[test]
    fn a_new_account_has_a_uuid_and_no_row() {
        let account = Account::new("throwaway_01", "Throw", "Away", "throwaway@example.com").unwrap();
        assert!(fingerprinter::looks_like_uuid(account.uuid()));
        assert_eq!(account.id(), None);
        assert_eq!(account.created_at(), None);
        assert_eq!(account.username(), "throwaway_01");
        // Nothing in the table says otherwise.
        assert!(account.changed());
    }

    #[test]
    fn an_unchanged_account_isnt_written() {
        let mut account = loaded();
        assert!(!account.changed());
        assert!(account.save().is_none());
    }

    #[test]
    fn a_change_is_written_once() {
        let mut account = loaded();
        account.last_login = Some(SystemTime::now());
        assert!(account.changed());

        // The tests never start Archivist, so the job itself goes nowhere.
        // What matters is that it was sent, and that it isn't sent again.
        assert!(account.save().is_some());
        assert!(!account.changed());
        assert!(account.save().is_none());

        account.email = "someone.else@example.com".to_string();
        assert!(account.save().is_some());
    }

    #[test]
    fn changing_it_back_is_no_change() {
        let mut account = loaded();
        account.first_name = "Thrown".to_string();
        account.first_name = "Throw".to_string();
        assert!(account.save().is_none());
    }
}
