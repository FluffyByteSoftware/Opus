//! File:       Opus/Conductor/dev/tools/src/constellations/text.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The one reader and writer for every config file, driven by the table
//! in `files.rs`.  Reading: a line at a time, `key = value`, `#` for
//! comments, keys in any case, the later of two wins.  Writing: the file's
//! comment, the reboot rule, then every setting as a comment and a line.
//! Whatever the writer puts out, the reader has to read back without a
//! complaint, and a test holds us to it.
//!
//! Nothing in here logs or touches the disk, which is what lets the tests
//! run it.  `constellations.rs` does both.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::files::{ConfigFile, Kind, Reboot, Setting};

/// A file's values, by key, as written.  Only keys from the file's table
/// are ever in here, and only values that passed their check.
// Rust note: the keys are the `&'static str`s out of the table itself, so
// `values.get("wgui_port")` works and there's no copy of the key.
pub type Values = BTreeMap<&'static str, String>;

/// What came of reading a file.
pub struct Parsed {
    /// The good values.
    pub values: Values,
    /// One complaint per line the reader couldn't use, with the line
    /// number.  A value is never echoed for a `Secret`, and a line that
    /// isn't `key = value` is never echoed at all, since it might be the
    /// password line with the `=` forgotten.
    pub problems: Vec<String>,
    /// Every key the file had, good value or not, lowercase.
    pub seen: HashSet<String>,
}

/// Every setting's default, for a file that hasn't been loaded.
pub fn defaults(file: &ConfigFile) -> Values {
    file.settings.iter().map(|setting| (setting.key, setting.default.to_string())).collect()
}

/// Goes through `text` a line at a time and keeps every good value.
pub fn parse(file: &ConfigFile, text: &str) -> Parsed {
    let mut parsed = Parsed { values: Values::new(), problems: Vec::new(), seen: HashSet::new() };
    // The line each key was last set on.
    let mut set_on: HashMap<&'static str, usize> = HashMap::new();

    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            parsed.problems.push(format!("line {number} isn't a \"key = value\" line.  Ignored."));
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        parsed.seen.insert(key.clone());

        let Some(setting) = file.setting(&key) else {
            parsed.problems.push(format!("line {number}: There is no setting called {key}.  Ignored."));
            continue;
        };
        match check(setting, value) {
            Ok(()) => {
                if let Some(earlier) = set_on.get(setting.key) {
                    parsed.problems.push(format!("line {number}: {} was already set on line {earlier}.  \
                        This one wins.", setting.key));
                }
                set_on.insert(setting.key, number);
                parsed.values.insert(setting.key, value.to_string());
            }
            Err(problem) => parsed.problems.push(format!("line {number}: {problem}  Ignored.")),
        }
    }

    parsed
}

/// Whether `value` will do for `setting`, and if not, why not, in words
/// that say what would.
pub fn check(setting: &Setting, value: &str) -> Result<(), String> {
    let key = setting.key;
    match setting.kind {
        Kind::Text => {
            if value.is_empty() {
                return Err(format!("{key} is empty."));
            }
        }
        Kind::Secret => {}
        Kind::Password => {
            if value.is_empty() {
                return Err(format!("{key} is empty."));
            }
        }
        Kind::Folder => {
            if value.is_empty() {
                return Err(format!("{key} is empty, and it needs a folder, like logs or /var/log/opus."));
            }
        }
        // 0 would mean "any port the OS likes", and nobody would know
        // where to point the browser.
        Kind::Port => match value.parse::<u16>() {
            Ok(port) if port > 0 => {}
            _ => return Err(format!("{key} is \"{value}\", and it needs a port from 1 to 65535, like 9996.")),
        },
        Kind::Number { low, high } => match value.parse::<u64>() {
            Ok(number) if (low..=high).contains(&number) => {}
            _ => return Err(format!("{key} has to be a number from {low} to {high}, and \"{value}\" isn't.")),
        },
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// The line under a file's own comment that says which reboot it waits on.
fn reboot_text(reboot: Reboot) -> &'static str {
    match reboot {
        Reboot::Soft => "\
Every setting in this file needs a SOFT REBOOT to take: STOP SERVER and
START SERVER (or RESTART SERVER) on the web admin's Server tab.  The
server reads it again every time it starts.  Edit it by hand with the
server stopped, or through the web admin, which holds the change in a
.wait4server file beside it until the server stops.",
        Reboot::Hard => "\
Every setting in this file needs a HARD REBOOT to take: Conductor, the
whole program, shut down and run again.  Edit it by hand while Conductor
isn't running, or through the web admin, which holds the change in a
.wait4server file beside it until Conductor shuts down.",
    }
}

/// The comment at the top of a fresh file: the file's own words, the
/// reboot rule, and what happens to a line that can't be read.
fn header(file: &ConfigFile) -> String {
    format!("{}\n#\n{}\n#\n{}\n",
            comment(file.about),
            comment(reboot_text(file.reboot)),
            comment("A line Conductor can't make sense of is logged and skipped, and that\n\
                     setting keeps its default.  A setting that isn't in the file at all is\n\
                     added to the end with its default the next time the file is read."))
}

/// `text` with `# ` in front of every line.
fn comment(text: &str) -> String {
    text.lines().map(|line| format!("# {line}")).collect::<Vec<_>>().join("\n")
}

/// One setting as a block: its comment, then the line.  The value is the
/// one in `values`, or the default when it isn't there.
fn block(setting: &Setting, values: &Values) -> String {
    let value = values.get(setting.key).map_or(setting.default, String::as_str);
    format!("{}\n{} = {value}\n", comment(setting.about), setting.key)
}

/// The whole text of a file holding `values`, comments and all.  A key
/// missing from `values` is written with its default.
pub fn file_text(file: &ConfigFile, values: &Values) -> String {
    let mut text = header(file);
    for setting in file.settings {
        text.push('\n');
        text.push_str(&block(setting, values));
    }
    text
}

/// The text to add to the end of a file that only has the keys in `seen`,
/// or `None` if it has them all.  Whatever is in the file already is left
/// alone; only the missing settings are added, with their defaults.
pub fn missing_text(file: &ConfigFile, seen: &HashSet<String>) -> Option<String> {
    let missing: Vec<&Setting> = file.settings.iter().filter(|setting| !seen.contains(setting.key)).collect();
    if missing.is_empty() {
        return None;
    }

    let defaults = defaults(file);
    let mut text = "\n# Conductor added the settings below with their defaults, because it knows\n\
                    # them and they weren't in this file.\n".to_string();
    for setting in missing {
        text.push('\n');
        text.push_str(&block(setting, &defaults));
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::files::{FILES, GLOBALS, POSTGRES};

    /// Every file, with every setting changed from its default, so a line
    /// that silently failed to parse couldn't still "match".
    fn filled_in(file: &ConfigFile) -> Values {
        file.settings.iter().map(|setting| {
            let value = match setting.kind {
                Kind::Text => format!("other-{}", setting.key),
                Kind::Secret => "p@ss = word".to_string(),
                Kind::Password => "p@ss word".to_string(),
                Kind::Folder => "/tmp/somewhere else/logs".to_string(),
                Kind::Port => "12345".to_string(),
                Kind::Number { high, .. } => high.to_string(),
            };
            (setting.key, value)
        }).collect()
    }

    #[test]
    fn what_we_write_we_can_read() {
        for file in FILES {
            let written = filled_in(file);
            let parsed = parse(file, &file_text(file, &written));
            assert!(parsed.problems.is_empty(), "{}: complaints: {:?}", file.name, parsed.problems);
            assert_eq!(parsed.values, written, "{}", file.name);
            assert!(missing_text(file, &parsed.seen).is_none(), "{}", file.name);
        }
    }

    #[test]
    fn the_default_file_reads_clean() {
        for file in FILES {
            let parsed = parse(file, &file_text(file, &defaults(file)));
            assert!(parsed.problems.is_empty(), "{}: {:?}", file.name, parsed.problems);
            assert_eq!(parsed.values, defaults(file), "{}", file.name);
        }
    }

    #[test]
    fn every_line_of_the_written_file_fits_in_78() {
        for file in FILES {
            for line in file_text(file, &defaults(file)).lines() {
                if line.starts_with('#') {
                    assert!(line.len() <= 78, "{}: too wide: {line}", file.name);
                }
            }
        }
    }

    #[test]
    fn a_bad_value_keeps_the_key_out() {
        let parsed = parse(&GLOBALS, "scribe_log_dir =\n");
        assert_eq!(parsed.problems.len(), 1);
        assert!(parsed.problems[0].starts_with("line 1:"));
        assert!(parsed.values.is_empty());
        // The key was there, so it doesn't get added again.
        assert!(parsed.seen.contains("scribe_log_dir"));
    }

    #[test]
    fn a_port_has_to_be_a_real_one() {
        for bad in ["0", "65536", "-1", "port", ""] {
            let parsed = parse(&GLOBALS, &format!("wgui_port = {bad}\n"));
            assert_eq!(parsed.problems.len(), 1, "{bad} should be a complaint");
            assert!(parsed.values.is_empty());
        }
        let parsed = parse(&GLOBALS, "wgui_port = 8080\n");
        assert!(parsed.problems.is_empty());
        assert_eq!(parsed.values["wgui_port"], "8080");
    }

    #[test]
    fn a_number_has_its_range() {
        for line in ["port = 0", "port = 65536", "port = five", "slow_job_ms = 0", "query_time_limit_seconds = -1",
                     "query_time_limit_seconds = 3601"] {
            let parsed = parse(&POSTGRES, line);
            assert_eq!(parsed.problems.len(), 1, "{line}");
            assert!(parsed.values.is_empty(), "{line}");
        }
        let parsed = parse(&POSTGRES, "query_time_limit_seconds = 0\nslow_job_ms = 600000\n");
        assert!(parsed.problems.is_empty());
    }

    #[test]
    fn an_unknown_key_is_a_complaint() {
        let parsed = parse(&POSTGRES, "# a comment\n\npasword = typo\n");
        assert_eq!(parsed.problems.len(), 1);
        assert!(parsed.problems[0].starts_with("line 3:"));
        assert!(parsed.problems[0].contains("pasword"));
        assert!(parsed.values.is_empty());
    }

    #[test]
    fn the_later_one_wins() {
        let parsed = parse(&GLOBALS, "scribe_log_dir = first\nscribe_log_dir = second\n");
        assert_eq!(parsed.problems.len(), 1);
        assert!(parsed.problems[0].starts_with("line 2:"));
        assert_eq!(parsed.values["scribe_log_dir"], "second");
    }

    #[test]
    fn a_bad_line_does_not_spoil_the_good_ones() {
        // Also: keys in any case, spaces around both halves, a Windows line
        // ending, and an `=` inside a value.
        let parsed = parse(&GLOBALS, "this line is broken\n  SCRIBE_LOG_DIR =  /tmp/a=b  \r\n");
        assert_eq!(parsed.problems.len(), 1);
        assert!(parsed.problems[0].starts_with("line 1"));
        assert_eq!(parsed.values["scribe_log_dir"], "/tmp/a=b");
    }

    #[test]
    fn a_broken_line_does_not_show_itself() {
        // A password line with the `=` forgotten must not end up in the log.
        let parsed = parse(&POSTGRES, "password hunter2\n");
        assert_eq!(parsed.problems.len(), 1);
        assert!(!parsed.problems[0].contains("hunter2"));
    }

    #[test]
    fn an_empty_password_is_allowed() {
        let parsed = parse(&POSTGRES, "password =\n");
        assert!(parsed.problems.is_empty());
        assert_eq!(parsed.values["password"], "");
    }

    #[test]
    fn missing_settings_get_added_and_read_back() {
        // An older file, from before the time limit and slow jobs existed.
        let old = "address = localhost\nport = 5432\ndatabase = opusdb\nusername = opus_game\npassword = x\n";
        let parsed = parse(&POSTGRES, old);
        let added = missing_text(&POSTGRES, &parsed.seen).expect("the newer settings are missing");
        assert!(added.contains("query_time_limit_seconds = 10"));
        assert!(!added.contains("address"));

        let read_back = parse(&POSTGRES, &format!("{old}{added}"));
        assert!(read_back.problems.is_empty(), "complaints: {:?}", read_back.problems);
        assert!(missing_text(&POSTGRES, &read_back.seen).is_none());
        assert_eq!(read_back.values["password"], "x");
        assert_eq!(read_back.values["slow_job_ms"], "250");
    }
}
