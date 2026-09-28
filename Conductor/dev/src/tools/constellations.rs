//! File:       Opus/Conductor/dev/src/tools/constellations.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Constellations loads `Content/cfg/conductor_globals.cfg` once at startup
//! and holds the result as globals the rest of the server reads.  The file
//! is plain `key = value` lines with `#` comments, so it can be edited by
//! hand with the server stopped.  If the file is missing, Constellations
//! writes one with the defaults and carries on -- a fresh checkout has no
//! `Content/` folder at all, and that must never be a crash.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Where the config lives, under the Content folder.
const CONFIG_FILE: &str = "cfg/conductor_globals.cfg";

/// The environment variable that points at the Content folder when the
/// walk-up search would land in the wrong place.
const CONTENT_ENV: &str = "OPUS_CONTENT";

/// Everything the config file can set.  Paths are already resolved against
/// the Content folder by the time they land here.
#[derive(Debug)]
pub struct Globals {
    /// The folder Scribe writes its daily log files into.
    pub scribe_log_dir: PathBuf,
    /// Keys the file had that we don't know.  Kept so main can log a warning
    /// about each once Scribe is up, since a typo in a key is otherwise
    /// silent.
    pub unknown_keys: Vec<String>,
}

// Rust note: `OnceLock` is a global that can be written exactly once and
// read from anywhere after that.  It is how Rust does a "set at startup,
// read forever" global without `unsafe`.
static CONTENT_DIR: OnceLock<PathBuf> = OnceLock::new();
static GLOBALS: OnceLock<Globals> = OnceLock::new();

/// Finds the Content folder, reads the config (writing the default one first
/// if it is missing) and stores the result.  Call once, before anything
/// else, and before Scribe starts.
pub fn load() -> Result<(), String> {
    let content = CONTENT_DIR.get_or_init(find_content_dir);
    let path = content.join(CONFIG_FILE);

    if !path.exists() {
        write_default(&path)?;
    }

    let text = fs::read_to_string(&path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let (mut values, bad_lines) = parse(&text);
    if let Some(line) = bad_lines.first() {
        return Err(format!("{} line {line} is not a \"key = value\" line", path.display()));
    }

    let scribe_log_dir = content.join(values.remove("scribe_log_dir").unwrap_or_else(|| "logs".to_string()));
    let mut unknown_keys: Vec<String> = values.into_keys().collect();
    unknown_keys.sort();

    let globals = Globals {
        scribe_log_dir,
        unknown_keys,
    };
    GLOBALS
        .set(globals)
        .map_err(|_| "the globals were already loaded".to_string())
}

/// The loaded globals.  Panics if `load` has not run yet, because that is a
/// bug in startup order, not something to limp past.
pub fn globals() -> &'static Globals {
    GLOBALS.get().expect("constellations::load must run before the globals are read")
}

/// The Content folder that was found (or made) at startup.
pub fn content_dir() -> &'static Path {
    CONTENT_DIR.get_or_init(find_content_dir)
}

/// The full path of the config file.
pub fn config_path() -> PathBuf {
    content_dir().join(CONFIG_FILE)
}

/// Works out where `Content/` is.  `OPUS_CONTENT` wins if it is set.
/// Otherwise we walk up from the working directory looking for a folder
/// that already has a `Content/` in it -- from `Conductor/dev` that is two
/// levels up, and the same from `Conductor/build`.  If nothing turns up we
/// make `./Content` and use that, so a bare binary in an empty folder still
/// runs.
fn find_content_dir() -> PathBuf {
    if let Some(from_env) = std::env::var_os(CONTENT_ENV) {
        return PathBuf::from(from_env);
    }
    if let Ok(cwd) = std::env::current_dir() {
        for dir in cwd.ancestors() {
            let candidate = dir.join("Content");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    PathBuf::from("Content")
}

/// Splits the file into `key = value` pairs.  Blank lines and lines starting
/// with `#` are skipped.  Returns the pairs and the line numbers of anything
/// that had no `=` in it.
fn parse(text: &str) -> (HashMap<String, String>, Vec<usize>) {
    let mut values = HashMap::new();
    let mut bad_lines = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.split_once('=') {
            Some((key, value)) => {
                values.insert(key.trim().to_string(), value.trim().to_string());
            }
            None => bad_lines.push(index + 1),
        }
    }
    (values, bad_lines)
}

/// Writes the config file with every default in it and a comment on each,
/// so the file explains itself to whoever opens it next.
fn write_default(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
    }
    let text = "\
# Conductor's globals.  One \"key = value\" a line, and \"#\" starts a comment.
# Paths are relative to the Content folder unless they start with \"/\".
# Conductor wrote this file with its defaults because there wasn't one.
# Stop the server before editing it; it is only read at startup.

# The folder Scribe writes its logs into.  One file a day, named by the
# UTC date, rolling over at midnight UTC.
scribe_log_dir = logs
";
    fs::write(path, text).map_err(|e| format!("could not write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skips_comments_and_blanks() {
        let (values, bad) = parse("# a comment\n\n  scribe_log_dir = logs  \nother=x=y\n");
        assert_eq!(values.get("scribe_log_dir").map(String::as_str), Some("logs"));
        // Only the first '=' splits, so a value can hold one.
        assert_eq!(values.get("other").map(String::as_str), Some("x=y"));
        assert!(bad.is_empty());
    }

    #[test]
    fn parse_reports_the_line_with_no_equals() {
        let (_, bad) = parse("good = 1\nthis line is broken\n");
        assert_eq!(bad, vec![2]);
    }
}
