//! File:       Opus/Conductor/dev/conductor-launcher/src/launcher.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The launcher, the admin's menu.  main() gets the tools started, then
//! hands the terminal over to `run()`, and when that returns Conductor
//! shuts down.  So Q is how the server gets stopped.
//!
//! Scribe never prints to the terminal (unless it has lost its file), so
//! the menu has the screen to itself.  L is how the admin reads the log.
//!
//! There is only L and Q for now.  Starting the server, accounts and the
//! config menu come once there is something behind them.

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use conductor_tools::scribe;

/// How much of the log a plain L shows.
const VIEW_LOG_LINES: usize = 25;

/// How far back L reads at a time when it is after the last few lines.  25
/// log lines are about 3 KB, so one chunk usually does it.
const VIEW_LOG_CHUNK_BYTES: u64 = 16 * 1024;

/// One line typed at the menu: the letter, and anything after it, split on
/// spaces.  `L -n 50` is the letter L with the arguments `-n` and `50`.
#[derive(Debug, PartialEq)]
struct Command {
    letter: char,
    args: Vec<String>,
}

/// Runs the menu until the admin picks Q, or the terminal closes (Ctrl-D,
/// or a pipe that ran out).  A closed terminal counts as Q.
pub fn run() {
    match scribe::current_file() {
        Some(path) => say(&format!("The log is {}.", path.display())),
        None => say("There is no log file.  Log lines print here instead."),
    }

    loop {
        show_main_menu();

        let Some(line) = read_line("> ") else {
            break;
        };
        if line.is_empty() {
            continue;
        }

        match parse_command(&line) {
            Some(Command { letter: 'L', args }) => view_log(&args),
            Some(Command { letter: 'Q', args }) if args.is_empty() => break,
            _ => say("That isn't one of the choices."),
        }
    }
}

fn show_main_menu() {
    say("");
    say("Opus Conductor");
    say("");
    say("  L) View the log      L, L -n 50, or L all");
    say("  Q) Shut down");
    say("");
}

/// Splits a typed line into its letter and its arguments.  The letter can
/// be either case.  `None` when the first word isn't a single character, so
/// `list` or `quit` is not an L or a Q.  Control characters (an Escape
/// pressed by accident) are ignored.
fn parse_command(line: &str) -> Option<Command> {
    let cleaned: String = line.chars().filter(|c| !c.is_control()).collect();
    let mut words = cleaned.split_whitespace();

    let first = words.next()?;
    let mut chars = first.chars();
    let letter = chars.next()?;
    if chars.next().is_some() {
        return None;
    }

    Some(Command {
        letter: letter.to_ascii_uppercase(),
        args: words.map(|word| word.to_string()).collect(),
    })
}

// ---------------------------------------------------------------------------
// L) View the log
// ---------------------------------------------------------------------------

/// Works out how much of the log L was asked for.  `Some(n)` is the last n
/// lines and `None` is the whole file.  An `Err` is what to tell the admin.
fn view_log_count(args: &[String]) -> Result<Option<usize>, String> {
    // Rust note: this matches on the shape of the list.  `[]` is no
    // arguments, `[word]` is exactly one, and `[flag, number]` is two.
    match args {
        [] => Ok(Some(VIEW_LOG_LINES)),
        [word] if word.eq_ignore_ascii_case("all") => Ok(None),
        [flag, number] if flag == "-n" => match number.parse::<usize>() {
            Ok(count) if count >= 1 => Ok(Some(count)),
            _ => Err("-n needs a number, 1 or more, like L -n 50.".to_string()),
        },
        [flag] if flag == "-n" => Err("-n needs a number, 1 or more, like L -n 50.".to_string()),
        _ => Err("L takes -n and a number, or all.  Like L -n 50, or L all.".to_string()),
    }
}

/// Shows the end of the log (or all of it), plain, the way it is in the
/// file.
fn view_log(args: &[String]) {
    let count = match view_log_count(args) {
        Ok(count) => count,
        Err(problem) => {
            say(&problem);
            return;
        }
    };

    let Some(path) = scribe::current_file() else {
        say("There is no log file to show.  Log lines have been printing here instead.");
        return;
    };

    let lines = match count {
        Some(count) => last_lines(&path, count, VIEW_LOG_CHUNK_BYTES),
        None => all_lines(&path),
    };

    match lines {
        Ok(lines) if lines.is_empty() => say("The log is empty."),
        Ok(lines) => {
            say("");
            for line in &lines {
                say(line);
            }
        }
        Err(problem) => say(&format!("Can't read the log at {}: {problem}.", path.display())),
    }
}

/// Every line of a file.
// Rust note: `from_utf8_lossy` swaps any bytes that aren't valid UTF-8 for
// a placeholder character instead of failing.  A log that was cut off
// mid-character by a crash still shows.
fn all_lines(path: &Path) -> io::Result<Vec<String>> {
    let bytes = fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).lines().map(|line| line.to_string()).collect())
}

/// The last `count` lines of a file.  A day's log can get big, so this
/// doesn't read the whole thing.  It steps back from the end `chunk_bytes`
/// at a time until it holds more newlines than lines wanted, or it reaches
/// the top.  The first line of what it read is probably only the back half
/// of a line, but with one newline to spare the lines we keep are all
/// whole.
fn last_lines(path: &Path, count: usize, chunk_bytes: u64) -> io::Result<Vec<String>> {
    let mut file = File::open(path)?;
    let mut start = file.metadata()?.len();
    let mut tail: Vec<u8> = Vec::new();

    while start > 0 && newlines_in(&tail) <= count {
        let step = chunk_bytes.min(start);
        start -= step;

        let mut chunk = vec![0u8; step as usize];
        file.seek(SeekFrom::Start(start))?;
        file.read_exact(&mut chunk)?;

        // The new chunk goes in front of what we already had.
        chunk.extend_from_slice(&tail);
        tail = chunk;
    }

    let text = String::from_utf8_lossy(&tail);
    let lines: Vec<&str> = text.lines().collect();
    let skip = lines.len().saturating_sub(count);
    Ok(lines[skip..].iter().map(|line| line.to_string()).collect())
}

fn newlines_in(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&byte| byte == b'\n').count()
}

// ---------------------------------------------------------------------------
// The terminal
// ---------------------------------------------------------------------------

/// One line of the menu.  Like Scribe, this never panics if the terminal
/// has gone away.
fn say(text: &str) {
    let _ = writeln!(io::stdout(), "{text}");
}

/// Shows the prompt and reads one line, with the spaces and the newline
/// trimmed off.  `None` means the terminal closed.
fn read_line(prompt: &str) -> Option<String> {
    let mut stdout = io::stdout();
    let _ = write!(stdout, "{prompt}");
    // Rust note: the prompt has no newline, so it sits in a buffer until
    // something pushes it out.  `flush()` does that before we wait.
    let _ = stdout.flush();

    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(line.trim().to_string()),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| word.to_string()).collect()
    }

    #[test]
    fn commands() {
        assert_eq!(parse_command("l"), Some(Command { letter: 'L', args: vec![] }));
        assert_eq!(parse_command("  L -n 50  "), Some(Command { letter: 'L', args: args(&["-n", "50"]) }));
        assert_eq!(parse_command("q"), Some(Command { letter: 'Q', args: vec![] }));
        // Escape pressed by accident.
        assert_eq!(parse_command("\u{1b}q"), Some(Command { letter: 'Q', args: vec![] }));

        assert_eq!(parse_command(""), None);
        assert_eq!(parse_command("   "), None);
        assert_eq!(parse_command("list"), None);
        assert_eq!(parse_command("quit"), None);
    }

    #[test]
    fn how_much_of_the_log() {
        assert_eq!(view_log_count(&args(&[])), Ok(Some(25)));
        assert_eq!(view_log_count(&args(&["-n", "50"])), Ok(Some(50)));
        assert_eq!(view_log_count(&args(&["all"])), Ok(None));
        assert_eq!(view_log_count(&args(&["ALL"])), Ok(None));

        assert!(view_log_count(&args(&["-n"])).is_err());
        assert!(view_log_count(&args(&["-n", "x"])).is_err());
        assert!(view_log_count(&args(&["-n", "0"])).is_err());
        assert!(view_log_count(&args(&["-n", "-5"])).is_err());
        assert!(view_log_count(&args(&["50"])).is_err());
        assert!(view_log_count(&args(&["-n", "5", "more"])).is_err());
    }

    /// A file of our own in the system temp folder with `line 1` to
    /// `line N` in it.  Not the real log.
    fn numbered_file(name: &str, lines: usize, trailing_newline: bool) -> PathBuf {
        let path = std::env::temp_dir().join(format!("opus_launcher_{}_{}.log", name, std::process::id()));
        let mut text = String::new();
        for number in 1..=lines {
            text.push_str(&format!("line {number}\n"));
        }
        if !trailing_newline {
            text.pop();
        }
        fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn last_lines_across_several_chunks() {
        // A 64 byte chunk is about 8 lines, so 50 lines takes several reads
        // and most of them start partway through a line.
        let path = numbered_file("chunks", 120, true);
        let lines = last_lines(&path, 50, 64).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(lines.len(), 50);
        assert_eq!(lines[0], "line 71");
        assert_eq!(lines[49], "line 120");
    }

    #[test]
    fn last_lines_of_a_short_file() {
        let path = numbered_file("short", 7, true);
        let lines = last_lines(&path, 25, 64).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(lines.len(), 7);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[6], "line 7");
    }

    #[test]
    fn last_lines_without_a_trailing_newline() {
        // A crash can leave the last line unfinished.  It still counts.
        let path = numbered_file("cut", 120, false);
        let lines = last_lines(&path, 50, 64).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(lines.len(), 50);
        assert_eq!(lines[0], "line 71");
        assert_eq!(lines[49], "line 120");
    }

    #[test]
    fn last_lines_of_an_empty_file() {
        let path = numbered_file("empty", 0, true);
        let lines = last_lines(&path, 25, 64).unwrap();
        let _ = fs::remove_file(&path);

        assert!(lines.is_empty());
    }

    #[test]
    fn all_lines_is_the_whole_file() {
        let path = numbered_file("all", 120, true);
        let lines = all_lines(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(lines.len(), 120);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[119], "line 120");
    }
}
