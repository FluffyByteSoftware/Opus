//! File:       Opus/Conductor/dev/accounts/src/lib.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Accounts: the one way in to the accounts table.  Nothing else in
//! Conductor writes SQL for it.
//!
//! An account is never held.  Whatever needs one reads it from its row
//! when it needs it, and every change goes straight back to the row, so
//! the row is the only copy and nothing can write an old copy over a new
//! one.  Jacob's rule, 2026-09-29, the day after the account had been
//! held in memory with its player: an edit from the web admin while that
//! player was online would have been written over when they left.
//!
//! `Account` is a look at a row, taken with `load()` or `list()`.  It can
//! be changed and `save()`d straight away, or just read and dropped.  The
//! login time is the one thing written on its own (`stamp_login()`), the
//! moment a player comes into the world over UDP.
//!
//! The password hash is not in `Account`, on purpose.  An account gets
//! printed, logged and put on the page; a hash never should be.  The
//! login reads it on its own with `password_hash()`, and `create()` and
//! `set_password()` take it on its own.
//!
//! Everything here goes through Archivist and hands back a `Pending`, so
//! nothing ever waits on the database unless it chooses to.  Archivist
//! has one worker and does its jobs in the order they came.
//!
//! `desk.rs` is the web admin's side of it: the jobs that need a password
//! hashed first, done on a thread of their own.

pub mod desk;

use std::io;
use std::time::SystemTime;

use conductor_tools::archivist::{self, Param, Pending, Row};
use conductor_tools::fingerprinter;
use conductor_tools::scribe::{self, Channel};

/// The account name rule, the same one the accounts table checks: 8 to
/// 32 of `a-z`, `0-9` and `_`.
const NAME_MIN: usize = 8;
const NAME_MAX: usize = 32;

/// Every column but the password hash.  The UUID comes out as text: the
/// `postgres` crate won't read a UUID column into a `String` on its own.
const COLUMNS: &str = "id, uuid::text AS uuid, account_username, owner_first_name, owner_last_name, owner_email, \
    created_at, last_login_datetime";

/// The one read of the hash, for the login.
const HASH_SQL: &str = "SELECT password_hash FROM accounts WHERE account_username = $1";

/// What `save()` and `edit()` write: the owner's names and email.  The
/// login time has its own write, and nothing else about a row changes.
const SAVE_SQL: &str = "UPDATE accounts SET owner_first_name = $1, owner_last_name = $2, owner_email = $3 \
    WHERE account_username = $4";

/// A new row.  Postgres hands back the two things only it knows.
const CREATE_SQL: &str = "INSERT INTO accounts (uuid, account_username, owner_first_name, owner_last_name, \
    owner_email, password_hash) VALUES ($1::text::uuid, $2, $3, $4, $5, $6) RETURNING id, created_at";

/// Is the name or the email in use?  The email the way the table's index
/// sees it, whatever the capitals.  `$3` is the account to leave out, for
/// an edit to an account's own email.
// The casts are there because `lower()` has more than one kind of
// argument, so Postgres can't tell on its own that `$2` is text.
const TAKEN_SQL: &str = "SELECT account_username = $1::text AS name_taken, \
    lower(owner_email) = lower($2::text) AS email_taken FROM accounts \
    WHERE (account_username = $1::text OR lower(owner_email) = lower($2::text)) AND account_username <> $3::text";

const STAMP_SQL: &str = "UPDATE accounts SET last_login_datetime = $1 WHERE account_username = $2";
const PASSWORD_SQL: &str = "UPDATE accounts SET password_hash = $1 WHERE account_username = $2";
const DELETE_SQL: &str = "DELETE FROM accounts WHERE account_username = $1";

/// An account: a look at its row.
///
/// The owner's names and email are there to be changed; `save()` writes
/// them.  Everything else is the row's own and is only read.
#[derive(Debug, Clone)]
pub struct Account {
    /// The row's number.  `None` for an account made in memory that isn't
    /// in the table yet.
    id: Option<i64>,
    uuid: String,
    username: String,
    /// `None` until the row exists: Postgres stamps it.
    created_at: Option<SystemTime>,
    /// `None` until the first login.
    last_login: Option<SystemTime>,

    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

impl Account {
    /// A new account, in memory only, with a UUID from Fingerprinter.  It
    /// has no row until `create()` writes one.  Nothing is checked here:
    /// `check_new()` does that, and the table checks again.  Fails only if
    /// the OS won't give random bytes.
    pub fn new(username: &str, first_name: &str, last_name: &str, email: &str) -> io::Result<Account> {
        Ok(Account {
            id: None,
            uuid: fingerprinter::new_uuid()?,
            username: username.to_string(),
            created_at: None,
            last_login: None,
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
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

    /// The name it logs in with, lowercase.  It never changes.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// When the row was made, once there is a row.
    pub fn created_at(&self) -> Option<SystemTime> {
        self.created_at
    }

    /// When a player last came into the world on it, over UDP.
    pub fn last_login(&self) -> Option<SystemTime> {
        self.last_login
    }

    /// Writes the owner's names and email back to the account's row, the
    /// way they are now.  How many rows it changed, which is 0 if the row
    /// has gone.  No checks here: `edit()` is the one that says an email
    /// is taken in words.
    ///
    /// An account with no row can't be saved, only created.  That's a bug
    /// in the caller, so it's a Warn, and nothing is sent.
    pub fn save(&self) -> Option<Pending<u64>> {
        if self.id.is_none() {
            scribe::warn(Channel::Database, &format!("Account {} was saved before it had a row.  Nothing was \
                written; create() makes the row.", self.username));
            return None;
        }
        let params: Vec<Param> = vec![Box::new(self.first_name.clone()), Box::new(self.last_name.clone()),
                                      Box::new(self.email.clone()), Box::new(self.username.clone())];
        Some(archivist::execute(SAVE_SQL, params))
    }

    /// An account from a row with `COLUMNS` in it.
    fn from_row(row: &Row) -> Account {
        Account {
            id: Some(row.get("id")),
            uuid: row.get("uuid"),
            username: row.get("account_username"),
            created_at: Some(row.get("created_at")),
            last_login: row.get("last_login_datetime"),
            first_name: row.get("owner_first_name"),
            last_name: row.get("owner_last_name"),
            email: row.get("owner_email"),
        }
    }
}

/// What `create()` did.
#[derive(Debug)]
pub enum Created {
    /// The new account, with its id and when it was made.
    Made(Account),
    /// Another account has that name.
    NameTaken,
    /// Another account has that email, whatever the capitals.
    EmailTaken,
}

/// What `edit()` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edited {
    Saved,
    /// No account by that name.
    NoSuchAccount,
    /// Another account has that email, whatever the capitals.
    EmailTaken,
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// Reads an account from its row.  `None` if there's no account by that
/// name.  The name is taken as it is; the table's names are lowercase.
// Rust note: this is a transaction only because a transaction's closure
// runs on Archivist's thread, so the row can be turned into an Account
// there and handed back ready.  It only reads.
pub fn load(username: &str) -> Pending<Option<Account>> {
    let username = username.to_string();
    archivist::transaction("load an account", move |tx| {
        let sql = format!("SELECT {COLUMNS} FROM accounts WHERE account_username = $1");
        let rows = tx.query(sql.as_str(), &[&username])?;
        Ok(rows.first().map(Account::from_row))
    })
}

/// Every account, by name, for the web admin's Accounts tab.
pub fn list() -> Pending<Vec<Account>> {
    archivist::transaction("list the accounts", move |tx| {
        let sql = format!("SELECT {COLUMNS} FROM accounts ORDER BY account_username");
        let rows = tx.query(sql.as_str(), &[])?;
        Ok(rows.iter().map(Account::from_row).collect())
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

/// Whether a name or an email is in use by some account other than
/// `besides` (`""` for none): the name first, then the email.  For
/// checking a new account before its password costs a hash.
pub fn taken(username: &str, email: &str) -> Pending<(bool, bool)> {
    let username = username.to_string();
    let email = email.to_string();
    archivist::transaction("check a name and an email", move |tx| taken_in(tx, &username, &email, ""))
}

fn taken_in(tx: &mut archivist::Transaction<'_>, username: &str, email: &str, besides: &str)
            -> Result<(bool, bool), archivist::PostgresError> {
    let rows = tx.query(TAKEN_SQL, &[&username, &email, &besides])?;
    let name = rows.iter().any(|row| row.get::<_, bool>("name_taken"));
    let email = rows.iter().any(|row| row.get::<_, bool>("email_taken"));
    Ok((name, email))
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// Writes an account made with `Account::new()` as a new row, with the
/// password hash from Security beside it, and hands it back with its id
/// and when it was made.  A name or email already in use comes back as
/// `NameTaken` or `EmailTaken`, checked in the same transaction as the
/// write.  Anything else the table turns away is the error.
pub fn create(mut account: Account, password_hash: String) -> Pending<Created> {
    archivist::transaction("make an account", move |tx| {
        let (name_taken, email_taken) = taken_in(tx, &account.username, &account.email, "")?;
        if name_taken {
            return Ok(Created::NameTaken);
        }
        if email_taken {
            return Ok(Created::EmailTaken);
        }
        let row = tx.query_one(CREATE_SQL, &[&account.uuid, &account.username, &account.first_name,
                                             &account.last_name, &account.email, &password_hash])?;
        account.id = Some(row.get("id"));
        account.created_at = Some(row.get("created_at"));
        Ok(Created::Made(account))
    })
}

/// Changes an account's owner names and email, straight in its row.  An
/// email in use by another account comes back as `EmailTaken` and
/// nothing is written.
pub fn edit(username: &str, first_name: &str, last_name: &str, email: &str) -> Pending<Edited> {
    let username = username.to_string();
    let first_name = first_name.to_string();
    let last_name = last_name.to_string();
    let email = email.to_string();
    archivist::transaction("edit an account", move |tx| {
        let (_, email_taken) = taken_in(tx, "", &email, &username)?;
        if email_taken {
            return Ok(Edited::EmailTaken);
        }
        let changed = tx.execute(SAVE_SQL, &[&first_name, &last_name, &email, &username])?;
        Ok(if changed == 0 { Edited::NoSuchAccount } else { Edited::Saved })
    })
}

/// Stamps an account's last login as now: a player just came into the
/// world on it over UDP.  That moment, not the TLS login before it, is
/// when playing starts, and it's what playtime will be counted from
/// (Jacob, 2026-09-29).  Not waited on by anybody; Archivist logs a write
/// that fails.
pub fn stamp_login(username: &str) -> Pending<u64> {
    let params: Vec<Param> = vec![Box::new(SystemTime::now()), Box::new(username.to_string())];
    archivist::execute(STAMP_SQL, params)
}

/// Puts a new password hash from Security on an account.  How many rows
/// it changed: 0 if there's no account by that name.  It takes at the
/// account's next login; one already in the world stays in.
pub fn set_password(username: &str, password_hash: String) -> Pending<u64> {
    let params: Vec<Param> = vec![Box::new(password_hash), Box::new(username.to_string())];
    archivist::execute(PASSWORD_SQL, params)
}

/// Deletes an account's row.  How many rows it deleted: 0 if there was
/// no account by that name.  Its player, if there is one in the world,
/// is networking's to kick; the web admin does both.
pub fn delete(username: &str) -> Pending<u64> {
    let params: Vec<Param> = vec![Box::new(username.to_string())];
    archivist::execute(DELETE_SQL, params)
}

// ---------------------------------------------------------------------------
// The rules, the same ones the table checks
// ---------------------------------------------------------------------------

/// The account name rule: 8 to 32 of `a-z`, `0-9` and `_`.  The same
/// rule the accounts table checks, so a name that passes here can be
/// looked up and one that doesn't can't be a row.
pub fn username_allowed(name: &str) -> bool {
    (NAME_MIN..=NAME_MAX).contains(&name.len())
        && name.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// Why a name can't be an account's, in words for the admin.
pub fn check_username(name: &str) -> Result<(), String> {
    if username_allowed(name) {
        Ok(())
    } else {
        Err(format!("An account name is {NAME_MIN} to {NAME_MAX} characters: lowercase letters, digits and _ only."))
    }
}

/// Why an owner's first or last name can't be stored, in words.  The
/// table only asks that it isn't empty; one made of spaces isn't much of a
/// name either.  `which` is "first" or "last", for the words.
pub fn check_owner_name(which: &str, name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err(format!("The owner's {which} name can't be empty."));
    }
    if name.chars().any(char::is_control) {
        return Err(format!("The owner's {which} name can't have a tab or a line break in it."));
    }
    Ok(())
}

/// Why an email can't be stored, in words.  The table's rule, loose on
/// purpose: something, an @, then something with a dot in it, and no
/// spaces.  A strict email pattern turns away real addresses.
pub fn check_email(email: &str) -> Result<(), String> {
    let loose = "An email is something, an @, then something with a dot in it, with no spaces.";
    if email.chars().any(char::is_whitespace) {
        return Err(loose.to_string());
    }
    let Some((local, domain)) = email.split_once('@') else {
        return Err(loose.to_string());
    };
    if local.is_empty() || domain.contains('@') {
        return Err(loose.to_string());
    }
    // A dot with something on both sides of it.
    let dotted = domain.char_indices().any(|(at, c)| c == '.' && at > 0 && at + 1 < domain.len());
    if !dotted {
        return Err(loose.to_string());
    }
    Ok(())
}

/// Every check on a new account's fields at once, each complaint with the
/// field's name in front (`username: ...`), so the page can put it beside
/// the field.  The password's rules are Security's; the two copies of it
/// have to match.  Empty if it's all fine.
pub fn check_new(username: &str, first_name: &str, last_name: &str, email: &str, password: &str,
                 password_again: &str) -> Vec<String> {
    let mut problems = check_owner(first_name, last_name, email);
    if let Err(why) = check_username(username) {
        problems.insert(0, format!("username: {why}"));
    }
    problems.extend(check_new_password(password, password_again));
    problems
}

/// The checks on an edit: the two names and the email, each complaint
/// with its field's name in front.
pub fn check_owner(first_name: &str, last_name: &str, email: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if let Err(why) = check_owner_name("first", first_name) {
        problems.push(format!("first_name: {why}"));
    }
    if let Err(why) = check_owner_name("last", last_name) {
        problems.push(format!("last_name: {why}"));
    }
    if let Err(why) = check_email(email) {
        problems.push(format!("email: {why}"));
    }
    problems
}

/// The checks on a new password, typed twice.
pub fn check_new_password(password: &str, password_again: &str) -> Vec<String> {
    if let Err(why) = conductor_tools::security::check_password_rules(password) {
        return vec![format!("password: {why}")];
    }
    if password != password_again {
        return vec!["password_again: The two passwords don't match.".to_string()];
    }
    Vec::new()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_account_has_a_uuid_and_no_row() {
        let account = Account::new("throwaway_01", "Throw", "Away", "throwaway@example.com").unwrap();
        assert!(fingerprinter::looks_like_uuid(account.uuid()));
        assert_eq!(account.id(), None);
        assert_eq!(account.created_at(), None);
        assert_eq!(account.last_login(), None);
        assert_eq!(account.username(), "throwaway_01");
    }

    #[test]
    fn an_account_with_no_row_isnt_saved() {
        let account = Account::new("throwaway_01", "Throw", "Away", "throwaway@example.com").unwrap();
        assert!(account.save().is_none());
    }

    #[test]
    fn the_name_rule_is_the_tables() {
        assert!(username_allowed("jacob_01"));
        assert!(username_allowed("a2345678"));
        assert!(username_allowed(&"a".repeat(32)));

        assert!(!username_allowed("jacob"));
        assert!(!username_allowed(&"a".repeat(33)));
        assert!(!username_allowed("Jacob_01"));
        assert!(!username_allowed("jacob-01"));
        assert!(!username_allowed("jacob 01"));
        assert!(!username_allowed(""));
        assert!(check_username("jacob").is_err());
    }

    #[test]
    fn the_email_rule_is_the_tables() {
        assert_eq!(check_email("jacob@example.com"), Ok(()));
        assert_eq!(check_email("a@b.c"), Ok(()));
        assert_eq!(check_email("first.last@mail.example.co.uk"), Ok(()));

        assert!(check_email("").is_err());
        assert!(check_email("jacob").is_err());
        assert!(check_email("jacob@example").is_err());
        assert!(check_email("@example.com").is_err());
        assert!(check_email("jacob@.com").is_err());
        assert!(check_email("jacob@example.").is_err());
        assert!(check_email("jacob@@example.com").is_err());
        assert!(check_email("jacob@exa@mple.com").is_err());
        assert!(check_email("jacob @example.com").is_err());
    }

    #[test]
    fn an_owner_needs_both_names() {
        assert!(check_owner("Jacob", "Chacko", "jacob@example.com").is_empty());
        let problems = check_owner("", "  ", "nope");
        assert_eq!(problems.len(), 3);
        assert!(problems[0].starts_with("first_name: "));
        assert!(problems[1].starts_with("last_name: "));
        assert!(problems[2].starts_with("email: "));
        assert!(check_owner_name("first", "Ja\ncob").is_err());
    }

    #[test]
    fn a_new_password_is_typed_twice() {
        assert!(check_new_password("Abcdef1!", "Abcdef1!").is_empty());
        assert_eq!(check_new_password("Abcdef1!", "Abcdef1?"),
                   vec!["password_again: The two passwords don't match.".to_string()]);
        // The rules come first: a bad password typed the same twice is
        // still a bad password.
        let problems = check_new_password("short", "short");
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("password: "));
    }

    #[test]
    fn a_new_account_is_checked_field_by_field() {
        assert!(check_new("jacob_01", "Jacob", "Chacko", "jacob@example.com", "Abcdef1!", "Abcdef1!").is_empty());
        let problems = check_new("Jacob", "", "Chacko", "jacob@example.com", "Abcdef1!", "nope");
        let fields: Vec<&str> = problems.iter().map(|problem| problem.split(':').next().unwrap()).collect();
        assert_eq!(fields, vec!["username", "first_name", "password_again"]);
    }
}
