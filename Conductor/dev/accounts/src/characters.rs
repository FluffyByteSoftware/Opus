//! File:       Opus/Conductor/dev/accounts/src/characters.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Characters: the one way in to the player_characters table, and to the
//! three slots on an account that point at them.  Nothing else in
//! Conductor writes SQL for either.
//!
//! A character is two things.  In the world it's a GameObject in primlib,
//! made from the Character template with its save laid over it.  Here
//! it's its row: which account it belongs to, its name, where it last
//! stood, and the save itself as Lua text.  This file never runs the Lua
//! and never reads what's in it; the game writes it and the game reads it.
//!
//! `CharacterSnapshot` is a look at a row without the save in it, for
//! character select and the web admin.  It's the last save, so a
//! character in the world has moved on from it, and it's never written
//! back.
//!
//! Everything goes through Archivist and hands back a `Pending`, like the
//! accounts.  Archivist has one worker that does its jobs in the order
//! they came, so two characters made on one account at the same moment
//! can't both take the same slot.
//!
//! The player's side names an account by its username (that's all
//! networking's book has) and a character by its uuid (the game's name
//! for it).  `save()` and `save_all()` name it by the row's id, since
//! that's what the character's `PlayerCharacter` carries in the world.
//!
//! A character whose save won't load is unplayable for the rest of the
//! run: `mark_unplayable()`.  That's kept in memory only, and STOP SERVER
//! forgets it, so the next START SERVER tries the save again (Jacob,
//! 2026-09-30: "the admin will need to restart the server to have it
//! attempt again").

use std::collections::BTreeSet;
use std::io;
use std::sync::Mutex;
use std::time::SystemTime;

use conductor_primlib::Save;
use conductor_tools::archivist::{self, Pending, Row};
use conductor_tools::fingerprinter;
use conductor_tools::scribe::{self, Channel};

/// How many characters an account can have.  One column each on the
/// accounts table, `character_slot_1` to `character_slot_3`.
pub const SLOTS: usize = 3;

/// The slots' columns, in order.  The one place a column name is picked
/// in code: from this list, never from anything a player sends.
const SLOT_COLUMNS: [&str; SLOTS] = ["character_slot_1", "character_slot_2", "character_slot_3"];

/// The character name rule, the same one the table checks: 4 to 20
/// letters, a to z, and only the first can be a capital.
const NAME_MIN: usize = 4;
const NAME_MAX: usize = 20;

/// Every column a snapshot needs, from the character's row and its
/// account's (`pc` and `a`), the account's name included.  `slot` is which of the account's slots
/// points at it, 1 to 3, or 0 if none does (which shouldn't happen).
const SNAPSHOT_COLUMNS: &str = "pc.id, pc.uuid::text AS uuid, pc.account_id, a.account_username, pc.character_name, \
    pc.position_x, pc.position_y, pc.position_z, pc.created_at, pc.saved_at, \
    CASE pc.id WHEN a.character_slot_1 THEN 1 WHEN a.character_slot_2 THEN 2 \
    WHEN a.character_slot_3 THEN 3 ELSE 0 END AS slot";

/// The account's id and its three slots, locked until the transaction
/// ends.
const SLOTS_SQL: &str = "SELECT id, character_slot_1, character_slot_2, character_slot_3 FROM accounts \
    WHERE account_username = $1 FOR UPDATE";

/// Is the name in use by any character, whatever the capitals?  The same
/// way the table's unique index sees it.
// The cast is there because `lower()` has more than one kind of argument.
const NAME_TAKEN_SQL: &str = "SELECT 1 FROM player_characters WHERE lower(character_name) = lower($1::text)";

/// A new row.  Postgres hands back the three things only it knows.
const CREATE_SQL: &str = "INSERT INTO player_characters (uuid, account_id, character_name, save_lua) \
    VALUES ($1::text::uuid, $2, $3, $4) RETURNING id, created_at, saved_at";

/// A save: the Lua text and the position together, so the columns never
/// fall out of step with the save.  The name never changes.
const SAVE_SQL: &str = "UPDATE player_characters SET save_lua = $1, position_x = $2, position_y = $3, \
    position_z = $4, saved_at = now() WHERE id = $5";

/// Only a character on this account.  Its slot empties on its own: the
/// slots are `ON DELETE SET NULL`.
const DELETE_SQL: &str = "DELETE FROM player_characters WHERE uuid = $1::text::uuid \
    AND account_id = (SELECT id FROM accounts WHERE account_username = $2)";

/// The characters found unplayable this run, by row id.  Emptied on STOP
/// SERVER.
// Rust note: a BTreeSet is a sorted set.  It's here because it can be
// made empty in a `static`, where a HashSet can't.
static UNPLAYABLE: Mutex<BTreeSet<i64>> = Mutex::new(BTreeSet::new());

/// A look at a character's row, without the save.  Everything is read
/// only: a character changes in the world, and only `save()` writes it
/// back.
#[derive(Debug, Clone)]
pub struct CharacterSnapshot {
    id: i64,
    uuid: String,
    account_id: i64,
    account_username: String,
    slot: u8,
    name: String,
    position: [f32; 3],
    created_at: SystemTime,
    saved_at: SystemTime,
    unplayable: bool,
}

impl CharacterSnapshot {
    /// The row's number.  The account's slot points at it, and the
    /// character's `PlayerCharacter` carries it in the world.
    pub fn id(&self) -> i64 {
        self.id
    }

    /// The game's name for the character, unique across the whole server.
    /// What the client names it by.
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    /// The row's number of the account it belongs to.
    pub fn account_id(&self) -> i64 {
        self.account_id
    }

    /// The name of the account it belongs to.
    pub fn account_username(&self) -> &str {
        &self.account_username
    }

    /// Which of the account's slots it's in, 1 to 3.
    pub fn slot(&self) -> u8 {
        self.slot
    }

    /// Its name, as it was made.  It never changes.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where it last stood when it was saved: x, y, z, with y up.  0, 0, 0
    /// until its first save.
    pub fn position(&self) -> [f32; 3] {
        self.position
    }

    /// When it was made.
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }

    /// When it was last saved.  The same as `created_at()` until then.
    pub fn saved_at(&self) -> SystemTime {
        self.saved_at
    }

    /// Whether its save failed to load this run.  The player still sees
    /// it at character select and can't play it.
    pub fn unplayable(&self) -> bool {
        self.unplayable
    }

    /// A snapshot from a row with `SNAPSHOT_COLUMNS` in it.
    fn from_row(row: &Row) -> CharacterSnapshot {
        let id: i64 = row.get("id");
        CharacterSnapshot {
            id,
            uuid: row.get("uuid"),
            account_id: row.get("account_id"),
            account_username: row.get("account_username"),
            slot: slot_number(row.get("slot")),
            name: row.get("character_name"),
            position: [row.get("position_x"), row.get("position_y"), row.get("position_z")],
            created_at: row.get("created_at"),
            saved_at: row.get("saved_at"),
            unplayable: is_unplayable(id),
        }
    }
}

/// A character to spawn: its snapshot, and its save as Lua text for
/// lua-parser's `read_save()`.
#[derive(Debug, Clone)]
pub struct CharacterSave {
    pub snapshot: CharacterSnapshot,
    pub save_lua: String,
}

/// What `create()` did.
#[derive(Debug)]
pub enum CharacterCreated {
    /// The new character, in the account's first empty slot.
    Made(CharacterSnapshot),
    /// No account by that name.
    NoSuchAccount,
    /// All three of the account's slots are full.  Nothing was written
    /// (Jacob: "we refuse to even allow them to create").
    SlotsFull,
    /// Another character has that name, whatever the capitals.
    NameTaken,
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// An account's characters, in slot order, for character select.  Empty
/// if it has none, or there's no account by that name.  No Lua is run.
pub fn list(username: &str) -> Pending<Vec<CharacterSnapshot>> {
    let username = username.to_string();
    archivist::transaction("list an account's characters", move |tx| {
        let sql = format!("SELECT {SNAPSHOT_COLUMNS} FROM player_characters pc \
            JOIN accounts a ON a.id = pc.account_id WHERE a.account_username = $1 ORDER BY slot, pc.id");
        let rows = tx.query(sql.as_str(), &[&username])?;
        Ok(rows.iter().map(CharacterSnapshot::from_row).collect())
    })
}

/// Every character on the server, by name, for the web admin's
/// Characters tab.  No Lua is run.
pub fn list_all() -> Pending<Vec<CharacterSnapshot>> {
    archivist::transaction("list every character", move |tx| {
        let sql = format!("SELECT {SNAPSHOT_COLUMNS} FROM player_characters pc \
            JOIN accounts a ON a.id = pc.account_id ORDER BY lower(pc.character_name)");
        let rows = tx.query(sql.as_str(), &[])?;
        Ok(rows.iter().map(CharacterSnapshot::from_row).collect())
    })
}

/// A character and its save, for the spawn.  `None` if there's no such
/// character on that account, so a player can only ever load their own.
/// The uuid is taken as it is; one that isn't a UUID finds nothing.
pub fn load(username: &str, uuid: &str) -> Pending<Option<CharacterSave>> {
    let username = username.to_string();
    let uuid = uuid.to_string();
    archivist::transaction("load a character", move |tx| {
        if !fingerprinter::looks_like_uuid(&uuid) {
            return Ok(None);
        }
        let sql = format!("SELECT {SNAPSHOT_COLUMNS}, pc.save_lua FROM player_characters pc \
            JOIN accounts a ON a.id = pc.account_id \
            WHERE a.account_username = $1 AND pc.uuid = $2::text::uuid");
        let rows = tx.query(sql.as_str(), &[&username, &uuid])?;
        Ok(rows.first().map(|row| CharacterSave {
            snapshot: CharacterSnapshot::from_row(row),
            save_lua: row.get("save_lua"),
        }))
    })
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// Makes a character on an account, in its first empty slot, in one
/// transaction: the row, then the slot pointing at it.  `save_lua` is the
/// new character as the game wrote it (the Character template, with the
/// name the player picked); it's stored as it is.  The name should have
/// passed `check_character_name()` first; the table checks it again.
///
/// A missing account, full slots and a name in use come back as answers,
/// checked in the same transaction as the write, and nothing is written.
/// Fails before anything is sent only if the OS won't give random bytes
/// for the UUID.
pub fn create(username: &str, name: &str, save_lua: String) -> io::Result<Pending<CharacterCreated>> {
    let uuid = fingerprinter::new_uuid()?;
    let username = username.to_string();
    let name = name.to_string();
    Ok(archivist::transaction("make a character", move |tx| {
        let Some(account) = tx.query(SLOTS_SQL, &[&username])?.into_iter().next() else {
            return Ok(CharacterCreated::NoSuchAccount);
        };
        let account_id: i64 = account.get("id");
        let empty = SLOT_COLUMNS.iter().position(|column| account.get::<_, Option<i64>>(*column).is_none());
        let Some(slot_index) = empty else {
            return Ok(CharacterCreated::SlotsFull);
        };
        if !tx.query(NAME_TAKEN_SQL, &[&name])?.is_empty() {
            return Ok(CharacterCreated::NameTaken);
        }

        let row = tx.query_one(CREATE_SQL, &[&uuid, &account_id, &name, &save_lua])?;
        let id: i64 = row.get("id");
        let slot_sql = format!("UPDATE accounts SET {} = $1 WHERE id = $2", SLOT_COLUMNS[slot_index]);
        tx.execute(slot_sql.as_str(), &[&id, &account_id])?;

        Ok(CharacterCreated::Made(CharacterSnapshot {
            id,
            uuid,
            account_id,
            account_username: username,
            slot: slot_index as u8 + 1,
            name,
            position: [0.0, 0.0, 0.0],
            created_at: row.get("created_at"),
            saved_at: row.get("saved_at"),
            unplayable: false,
        }))
    }))
}

/// Writes a character's save back to its row: the Lua text, and the
/// position it was at, from the same GameObject so the two agree.  How
/// many rows it changed: 0 if the row has gone.  A write that fails comes
/// back in the `Pending`, and only there, so whoever asked looks.
pub fn save(character_id: i64, position: [f32; 3], save_lua: String) -> Pending<u64> {
    let [x, y, z] = position;
    let params: Vec<archivist::Param> = vec![Box::new(save_lua), Box::new(x), Box::new(y), Box::new(z),
                                             Box::new(character_id)];
    archivist::execute(SAVE_SQL, params)
}

/// One character as the GameClock copied it out of the world, for
/// `save_all()`: its row's id, where it stood, and its save, not yet
/// turned into Lua text.
#[derive(Debug, Clone)]
pub struct SavedCharacter {
    pub character_id: i64,
    pub position: [f32; 3],
    pub save: Save,
}

/// Writes every character handed in back to its row, in one transaction,
/// so they all land or none do: a world save is the world at one moment,
/// and the database keeps the last whole one.  Each save is turned into
/// Lua text here, on Archivist's thread, one after another as it's
/// written, so the GameClock only ever copies (Jacob, 2026-09-30: "I'm
/// just worried about blocking or lagging the regular tick").  `what`
/// names the job for Archivist's status.  How many rows it changed.
pub fn save_all(what: &str, characters: Vec<SavedCharacter>) -> Pending<u64> {
    archivist::transaction(what, move |tx| {
        let mut written = 0;
        for character in characters {
            let [x, y, z] = character.position;
            let save_lua = character.save.to_lua();
            written += tx.execute(SAVE_SQL, &[&save_lua, &x, &y, &z, &character.character_id])?;
        }
        Ok(written)
    })
}

/// Deletes one of an account's characters, for the player at character
/// select, the only one who deletes a character (Jacob, 2026-09-30).  How
/// many rows it deleted: 0 if that account has no such character, so a
/// player can't delete somebody else's.  The slot empties on its own.
pub fn delete(username: &str, uuid: &str) -> Pending<u64> {
    let username = username.to_string();
    let uuid = uuid.to_string();
    archivist::transaction("delete a character", move |tx| {
        if !fingerprinter::looks_like_uuid(&uuid) {
            return Ok(0);
        }
        tx.execute(DELETE_SQL, &[&uuid, &username])
    })
}

// ---------------------------------------------------------------------------
// Unplayable
// ---------------------------------------------------------------------------

/// Flags a character unplayable for the rest of the run, and tells the
/// admin with an Error, which goes on the bell.  For whatever found its
/// save won't load: `why` is what was wrong with it.  The player still
/// sees it at character select and can't play it; the admin works out
/// whether it can be saved.  Flagging it twice says so once.
pub fn mark_unplayable(character_id: i64, name: &str, why: &str) {
    let newly = lock_unplayable().insert(character_id);
    if newly {
        scribe::error(Channel::Game, &format!("Character {name} (row {character_id} in player_characters) \
            is unplayable: its save won't load ({why}).  Nobody can play it until the server is stopped and \
            started again, when the save is tried again."));
    }
}

/// Whether a character has been flagged unplayable this run.  The spawn
/// asks, and turns the player away if so, since a changed client could
/// ignore the flag at character select.
pub fn is_unplayable(character_id: i64) -> bool {
    lock_unplayable().contains(&character_id)
}

/// Forgets every unplayable flag.  The launcher calls it on STOP SERVER,
/// so the next START SERVER tries each save again.
pub fn forget_unplayable() {
    lock_unplayable().clear();
}

fn lock_unplayable() -> std::sync::MutexGuard<'static, BTreeSet<i64>> {
    UNPLAYABLE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------
// The name rule, the same one the table checks
// ---------------------------------------------------------------------------

/// The character name rule: 4 to 20 letters, a to z, and only the first
/// can be a capital.  "Jacob" and "Mckay" pass; "JaCob" and "McKay"
/// don't.  The long name is where a player puts capitals of their own,
/// in game, later.
pub fn character_name_allowed(name: &str) -> bool {
    let bytes = name.as_bytes();
    (NAME_MIN..=NAME_MAX).contains(&bytes.len())
        && bytes[0].is_ascii_alphabetic()
        && bytes[1..].iter().all(|byte| byte.is_ascii_lowercase())
}

/// Why a name can't be a character's, in words for the player.  Whether
/// it's taken is `create()`'s to say.
pub fn check_character_name(name: &str) -> Result<(), String> {
    if character_name_allowed(name) {
        Ok(())
    } else {
        Err(format!("A character's name is {NAME_MIN} to {NAME_MAX} letters, a to z, and only the first can be \
            a capital."))
    }
}

/// The slot number out of the SQL, 1 to 3, or 0 for none.
fn slot_number(slot: i32) -> u8 {
    match slot {
        1..=3 => slot as u8,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use conductor_tools::archivist::ArchivistError;

    #[test]
    fn the_name_rule_is_the_tables() {
        // Jacob's own examples.
        assert!(character_name_allowed("Jacob"));
        assert!(character_name_allowed("Mckay"));
        assert!(!character_name_allowed("JaCob"));
        assert!(!character_name_allowed("McKay"));

        assert!(character_name_allowed("jacob"));
        assert!(character_name_allowed("Abcd"));
        assert!(character_name_allowed(&format!("A{}", "b".repeat(19))));

        assert!(!character_name_allowed("Abc"));
        assert!(!character_name_allowed(&format!("A{}", "b".repeat(20))));
        assert!(!character_name_allowed(""));
        assert!(!character_name_allowed("Jacob1"));
        assert!(!character_name_allowed("Ja cob"));
        assert!(!character_name_allowed("Ja_cob"));
        assert!(!character_name_allowed("1acob"));
        assert!(!character_name_allowed("Jacób"));
        assert!(check_character_name("McKay").is_err());
        assert_eq!(check_character_name("Mckay"), Ok(()));
    }

    #[test]
    fn a_slot_is_one_to_three_or_none() {
        assert_eq!(slot_number(1), 1);
        assert_eq!(slot_number(3), 3);
        assert_eq!(slot_number(0), 0);
        assert_eq!(slot_number(4), 0);
        assert_eq!(slot_number(-1), 0);
    }

    #[test]
    fn every_job_with_no_archivist_says_so() {
        // The tests never start Archivist, so there is no worker to answer.
        assert!(matches!(list("jacob_01").wait(), Err(ArchivistError::NotRunning)));
        assert!(matches!(list_all().wait(), Err(ArchivistError::NotRunning)));
        let loaded = load("jacob_01", "0192a7c4-0000-7000-8000-000000000000").wait();
        assert!(matches!(loaded, Err(ArchivistError::NotRunning)));
        let made = create("jacob_01", "Jacob", "return {}".to_string()).unwrap().wait();
        assert!(matches!(made, Err(ArchivistError::NotRunning)));
        assert!(matches!(save(1, [0.0, 0.0, 0.0], "return {}".to_string()).wait(),
                         Err(ArchivistError::NotRunning)));
        let deleted = delete("jacob_01", "0192a7c4-0000-7000-8000-000000000000").wait();
        assert!(matches!(deleted, Err(ArchivistError::NotRunning)));
    }

    // Every look at the unplayable flags is in this one test, since the
    // tests run side by side and the flags are shared.
    #[test]
    fn unplayable_lasts_until_forgotten() {
        assert!(!is_unplayable(-7));
        mark_unplayable(-7, "Testchar", "a test");
        mark_unplayable(-7, "Testchar", "a test, twice");
        assert!(is_unplayable(-7));
        assert!(!is_unplayable(-8));
        forget_unplayable();
        assert!(!is_unplayable(-7));
    }
}
