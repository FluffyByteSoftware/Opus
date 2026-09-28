//! File:       Opus/Conductor/dev/conductor-tools/src/archivist/schemas.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Getting the database into shape, every time Archivist connects.  First
//! the schemas, then the migrations.
//!
//! The schemas in `Content/psql/defaults/schemas/` are the tables as they
//! were first made.  They only CREATE ... IF NOT EXISTS, so a fresh
//! database gets its tables and one that has them is left alone.
//!
//! The migrations in `Content/psql/migrations/` are every change made to a
//! table after that, numbered, and each one runs exactly once.  Postgres
//! keeps the list of which ones have run in `archivist_migrations`.  A
//! fresh database runs the schemas and then every migration, and ends up
//! the same shape as one that has been around the whole time.  That only
//! works if a schema file is never changed once its table exists anywhere,
//! so every change goes in a migration.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use postgres::Client;

use crate::constellations;
use crate::diskman;
use crate::scribe::{self, Channel};

/// Where the default schemas live, under the Content folder.  One `.sql`
/// file per table, run in name order.
const SCHEMA_DIR: &str = "psql/defaults/schemas";

/// Where the migrations live, under the Content folder.
const MIGRATION_DIR: &str = "psql/migrations";

/// The default schemas, baked into Conductor when it's built.  If one of
/// the files goes missing from `Content/`, Archivist writes it back out
/// from here.  The file on disk is the one that gets run.
// Rust note: `include_str!` reads the file when the program is compiled
// and pastes its text in as a string.  The path is taken from this file's
// folder, so five `..` climb from src/archivist/ up to Opus/.
const DEFAULT_SCHEMAS: &[(&str, &str)] = &[
    ("accounts.sql", include_str!("../../../../../Content/psql/defaults/schemas/accounts.sql")),
];

/// The table that remembers which migrations have run.
const MIGRATION_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS archivist_migrations (
    number    INTEGER PRIMARY KEY,
    file_name TEXT NOT NULL,
    ran_at    TIMESTAMPTZ NOT NULL DEFAULT now()
)";

/// Runs the schemas and then the migrations.  The worker calls this each
/// time it connects, before it takes any jobs.
pub(super) fn get_in_shape(client: &mut Client) {
    run_schemas(client);
    run_migrations(client);
}

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

/// Puts back any default schema file that's missing, then runs every
/// `.sql` file in the schema folder, in name order.  A file that fails is
/// an Error in the log, and the rest still run.
fn run_schemas(client: &mut Client) {
    let folder = constellations::content_dir().join(SCHEMA_DIR);
    write_missing_schemas(&folder);

    let mut files = match sql_files(&folder) {
        Ok(files) => files,
        Err(e) => {
            scribe::error_with(Channel::Database, &e, &format!("Archivist can't read the schema folder {}.  \
                No tables were checked.", folder.display()));
            return;
        }
    };
    files.sort();

    let mut ran = 0;
    for file in &files {
        let sql = match diskman::read(file).wait().and_then(|bytes| diskman::as_text(&bytes)) {
            Ok(sql) => sql,
            Err(e) => {
                scribe::error_with(Channel::Database, &e, &format!("Archivist can't read {}.", file.display()));
                continue;
            }
        };
        match client.batch_execute(&sql) {
            Ok(()) => ran += 1,
            Err(e) => scribe::error_with(Channel::Database, &e, &format!("SCHEMA FILE FAILED, FIX IT BY HAND: \
                {}.  ITS TABLES MAY BE MISSING.", file.display())),
        }
    }

    scribe::info(Channel::Database, &format!("Archivist ran {ran} of {} schema file(s) from {}.",
                                             files.len(),
                                             folder.display()));
}

/// Writes out any default schema that isn't on disk.  One that is already
/// there is never touched.  DiskMan makes the folder if it has to, and
/// this waits on it (on Archivist's thread, not the game's), since the
/// files are read straight after.
fn write_missing_schemas(folder: &Path) {
    for (name, text) in DEFAULT_SCHEMAS {
        let path = folder.join(name);
        if path.exists() {
            continue;
        }
        match diskman::write(&path, text.as_bytes().to_vec()).wait() {
            Ok(()) => scribe::info(Channel::Database, &format!("Archivist wrote the default {}", path.display())),
            Err(e) => scribe::error_with(Channel::Database, &e, &format!("Archivist can't write the default {}.",
                                                                         path.display())),
        }
    }
}

/// Every `.sql` file in a folder.  Anything else (a README) is skipped.
fn sql_files(folder: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(folder)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "sql") {
            files.push(path);
        }
    }
    Ok(files)
}

// ---------------------------------------------------------------------------
// Migrations
// ---------------------------------------------------------------------------

/// Runs every migration that hasn't run yet, lowest number first.
///
/// Each one runs in a transaction along with its line in
/// `archivist_migrations`, so it either happens completely and is marked
/// done, or doesn't happen at all and gets tried again next connect.  The
/// first one that fails stops the rest, because a later one may lean on it.
fn run_migrations(client: &mut Client) {
    let folder = constellations::content_dir().join(MIGRATION_DIR);
    // DiskMan does files, not folders.  Making an empty folder and listing
    // what's in one stay out here.
    let _ = fs::create_dir_all(&folder);

    let paths = match sql_files(&folder) {
        Ok(paths) => paths,
        Err(e) => {
            scribe::error_with(Channel::Database, &e, &format!("Archivist can't read the migration folder {}.",
                                                               folder.display()));
            return;
        }
    };
    let names: Vec<String> = paths
        .iter()
        .filter_map(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect();

    let migrations = match in_order(names) {
        Ok(migrations) => migrations,
        Err(problem) => {
            scribe::error(Channel::Database, &format!("MIGRATIONS NOT RUN, FIX IT BY HAND: {}.  {problem}",
                                                      folder.display()));
            return;
        }
    };

    if let Err(e) = client.batch_execute(MIGRATION_TABLE) {
        scribe::error_with(Channel::Database, &e, "Archivist can't make the archivist_migrations table.  \
            No migrations were run.");
        return;
    }
    let done: HashSet<i32> = match client.query("SELECT number FROM archivist_migrations", &[]) {
        Ok(rows) => rows.iter().map(|row| row.get(0)).collect(),
        Err(e) => {
            scribe::error_with(Channel::Database, &e, "Archivist can't read archivist_migrations.  \
                No migrations were run.");
            return;
        }
    };

    for (number, name) in migrations {
        if done.contains(&number) {
            continue;
        }
        let path = folder.join(&name);
        let sql = match diskman::read(&path).wait().and_then(|bytes| diskman::as_text(&bytes)) {
            Ok(sql) => sql,
            Err(e) => {
                scribe::error_with(Channel::Database, &e, &format!("MIGRATION NOT RUN, FIX IT BY HAND: {}.  \
                    The ones after it wait.", path.display()));
                return;
            }
        };
        match run_one(client, number, &name, &sql) {
            Ok(()) => scribe::info(Channel::Database, &format!("Archivist ran migration {name}.")),
            Err(e) => {
                scribe::error_with(Channel::Database, &e, &format!("MIGRATION FAILED, FIX IT BY HAND: {}.  \
                    Nothing in it was kept, and the ones after it wait.", path.display()));
                return;
            }
        }
    }
}

/// One migration, all or nothing.
fn run_one(client: &mut Client, number: i32, name: &str, sql: &str) -> Result<(), postgres::Error> {
    let mut transaction = client.transaction()?;
    // A migration over a big table can take a while, and the time limit
    // from postgres.cfg is for the game's queries.  LOCAL means only for
    // this transaction.
    transaction.batch_execute("SET LOCAL statement_timeout = 0")?;
    transaction.batch_execute(sql)?;
    transaction.execute("INSERT INTO archivist_migrations (number, file_name) VALUES ($1, $2)",
                        &[&number, &name])?;
    transaction.commit()
}

/// Puts the migration file names in order by their number.  A name has to
/// start with a number and an `_`, like `0001_add_last_played.sql`.  A
/// name that doesn't, or two files with the same number, is a problem, and
/// none of them run until it's fixed, since we can't be sure of the order.
fn in_order(names: Vec<String>) -> Result<Vec<(i32, String)>, String> {
    let mut migrations = Vec::new();
    for name in names {
        let number = name
            .split_once('_')
            .and_then(|(number, _)| number.parse::<i32>().ok())
            .filter(|number| *number > 0);
        match number {
            Some(number) => migrations.push((number, name)),
            None => return Err(format!("{name} doesn't start with a number and an _, like 0001_what_it_does.sql.")),
        }
    }

    migrations.sort();
    for pair in migrations.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(format!("{} and {} have the same number.", pair[0].1, pair[1].1));
        }
    }
    Ok(migrations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn every_default_schema_is_a_sql_file_that_only_creates() {
        for (name, text) in DEFAULT_SCHEMAS {
            assert!(name.ends_with(".sql"), "{name}");
            // Anything that drops or changes a table would run on every
            // connect.  That's a migration, and it doesn't belong here.
            let upper = text.to_ascii_uppercase();
            for word in ["DROP ", "ALTER ", "DELETE ", "TRUNCATE "] {
                assert!(!upper.contains(word), "{name} has {word}");
            }
        }
    }

    #[test]
    fn migrations_run_by_number_not_by_name() {
        let sorted = in_order(names(&["10_c.sql", "0002_b.sql", "1_a.sql"])).unwrap();
        let numbers: Vec<i32> = sorted.iter().map(|(number, _)| *number).collect();
        assert_eq!(numbers, vec![1, 2, 10]);
    }

    #[test]
    fn a_badly_named_migration_stops_them_all() {
        for bad in ["add_column.sql", "0000_zero.sql", "12.sql", "x1_a.sql"] {
            assert!(in_order(names(&["0001_a.sql", bad])).is_err(), "{bad}");
        }
    }

    #[test]
    fn two_migrations_with_one_number_stop_them_all() {
        let problem = in_order(names(&["0001_a.sql", "1_b.sql"])).unwrap_err();
        assert!(problem.contains("same number"));
    }
}
