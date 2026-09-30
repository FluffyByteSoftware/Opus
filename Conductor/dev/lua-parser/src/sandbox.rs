//! File:       Opus/Conductor/dev/lua-parser/src/sandbox.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The locked-down Lua every script runs in.  It gets Lua's `string`,
//! `table`, `math`, `utf8` and `coroutine` libraries and nothing else of
//! Lua's: no `io`, no `os`, no `package` (so no `require`), no `debug`,
//! and the base library's `dofile`, `loadfile`, `load` and `warn` are
//! taken out too, along with `string.dump`.  The first two read files,
//! `load` can be handed raw bytecode (which Lua doesn't check, so a bad
//! bit of it can crash the whole process), and `warn` writes straight to
//! the console past Scribe.  What a script can call from outside Lua is
//! the log and nothing more.  It asks the game, and the game decides --
//! once there's a game to ask.
//!
//! A script also can't run forever or eat the machine's memory.  Past
//! `TIME_LIMIT` it's stopped, past `MEMORY_LIMIT` its next allocation
//! fails, and past `LOG_LINES` its log lines are dropped.  Each of those
//! is a Warn with the file's name, and the server carries on.
//!
//! The one gap I know of: a script that wraps its runaway loop in `pcall`
//! catches the stop, the same as any other error.  It only buys itself a
//! moment, though.  The stop keeps firing every few thousand
//! instructions, and sooner or later one lands outside the `pcall`.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use conductor_tools::scribe::{self, Channel};
use mlua::{Error, Function, HookTriggers, Lua, LuaOptions, StdLib, Value, Variadic, VmState};

/// How long one script can run before it's stopped.  A second is forever
/// for a script that sets up a goblin, and short enough that a script
/// stuck in a loop doesn't hold up START SERVER or STOP SERVER for long.
pub const TIME_LIMIT: Duration = Duration::from_secs(1);

/// How much memory one script's Lua can hold, 64 MB.  A guess for now.
pub const MEMORY_LIMIT: usize = 64 * 1024 * 1024;

/// How many log lines one script can write in a run.  A Warn is a notice
/// on the bell, so a script logging in a loop would bury the bell without
/// this.  The line that says the limit was hit is one more.
pub const LOG_LINES: u32 = 50;

/// How long one log line from a script can be, in characters.  Longer
/// ones are cut.
pub const LINE_CHARS: usize = 1000;

/// How often the time limit is checked, in Lua instructions.  Often enough
/// that a runaway is caught within a hair of the limit, rarely enough that
/// the check costs next to nothing.
const CHECK_EVERY: u32 = 10_000;

/// How loud a script's log line is.
#[derive(Debug, Clone, Copy)]
enum Level {
    Debug,
    Info,
    Warn,
}

/// Runs one script in a fresh, locked-down Lua and drops the Lua when
/// it's done.  `name` is the file as the log shows it
/// (`scripts/hello.lua`).  An error comes back as the Warn to log, naming
/// the file and, where Lua knows it, the line.
pub fn run(name: &str, source: &str) -> Result<(), String> {
    let lua = locked_down(name).map_err(|e| describe(name, &e))?;

    // The clock starts here, so building the Lua doesn't count against
    // the script.
    let started = Instant::now();
    // Rust note: `move` hands the closure its own copy of `started`, so
    // it can outlive this function inside the Lua.  The closure runs every
    // CHECK_EVERY instructions, and an error from it stops the script.
    //
    // It's the global hook, not the plain one, on purpose: mlua's plain
    // hook is for the one Lua thread it's set on, and a runaway loop
    // inside a coroutine would never be stopped.
    let hook = HookTriggers::new().every_nth_instruction(CHECK_EVERY);
    lua.set_global_hook(hook, move |_lua, debug| {
        if started.elapsed() <= TIME_LIMIT {
            return Ok(VmState::Continue);
        }
        let file = debug.source().short_src.map(|file| file.into_owned()).unwrap_or_default();
        let line = debug.current_line().map(|line| line.to_string()).unwrap_or_default();
        Err(Error::runtime(format!("{file}:{line}: still running after {} s, its time limit",
                                   TIME_LIMIT.as_secs())))
    }).map_err(|e| describe(name, &e))?;

    // The `@` tells Lua this is a file name, so its errors read
    // `scripts/hello.lua:3: ...`.
    lua.load(source)
        .set_name(format!("@{name}"))
        .exec()
        .map_err(|e| describe(name, &e))
}

/// A Lua with only the safe libraries, the unsafe bits of those taken
/// out, the memory limit set, and the log put in.
fn locked_down(name: &str) -> mlua::Result<Lua> {
    let libraries = StdLib::STRING | StdLib::TABLE | StdLib::MATH | StdLib::UTF8 | StdLib::COROUTINE;
    let lua = Lua::new_with(libraries, LuaOptions::default())?;
    lua.set_memory_limit(MEMORY_LIMIT)?;

    let globals = lua.globals();
    for unsafe_one in ["dofile", "loadfile", "load", "warn"] {
        globals.set(unsafe_one, Value::Nil)?;
    }
    let string: mlua::Table = globals.get("string")?;
    string.set("dump", Value::Nil)?;

    // Rust note: `Rc<Cell<u32>>` is one counter shared by the five
    // functions below.  `Rc` lets them all hold it, and `Cell` lets them
    // change it through a shared handle.  It never leaves this thread,
    // which is all `Rc` asks.
    let lines = Rc::new(Cell::new(0));
    let log = lua.create_table()?;
    log.set("debug", log_function(&lua, name, Level::Debug, &lines)?)?;
    log.set("info", log_function(&lua, name, Level::Info, &lines)?)?;
    log.set("warn", log_function(&lua, name, Level::Warn, &lines)?)?;
    // Jacob's call: anything a script says is wrong is a Warn in the log,
    // never an Error.  An Error is for Conductor itself.
    log.set("error", log_function(&lua, name, Level::Warn, &lines)?)?;
    globals.set("log", log)?;
    // Lua's own `print` would go to the console and skip the log.
    globals.set("print", log_function(&lua, name, Level::Debug, &lines)?)?;

    Ok(lua)
}

/// One of the log's functions, as Lua sees it.  It takes any number of
/// values, the same as `print`, and writes them with a space between.
fn log_function(lua: &Lua, name: &str, level: Level, lines: &Rc<Cell<u32>>) -> mlua::Result<Function> {
    let name = name.to_string();
    let lines = Rc::clone(lines);
    lua.create_function(move |lua, parts: Variadic<Value>| {
        write_line(lua, &name, level, &lines, &parts);
        Ok(())
    })
}

/// Writes one of a script's log lines to Scribe, unless the script has
/// used up its `LOG_LINES`.
fn write_line(lua: &Lua, name: &str, level: Level, lines: &Cell<u32>, parts: &[Value]) {
    let used = lines.get();
    if used > LOG_LINES {
        return;
    }
    lines.set(used + 1);
    if used == LOG_LINES {
        scribe::warn(Channel::Script, &format!("{name} has written {LOG_LINES} log lines this run, its limit.  \
            The rest are dropped."));
        return;
    }

    // A value that can't be turned into text (a table whose __tostring
    // fails) still gets a line, just not its words.
    let words: Vec<String> = parts.iter()
        .map(|part| part.to_string().unwrap_or_else(|_| "(can't be shown as text)".to_string()))
        .collect();
    let mut text = words.join(" ").replace(['\r', '\n'], " ");
    if text.chars().count() > LINE_CHARS {
        text = text.chars().take(LINE_CHARS).collect();
        text.push_str(" (cut)");
    }

    let message = format!("{}: {text}", where_from(lua, name));
    match level {
        Level::Debug => scribe::debug(Channel::Script, &message),
        Level::Info => scribe::info(Channel::Script, &message),
        Level::Warn => scribe::warn(Channel::Script, &message),
    }
}

/// The file and line that called the log, `scripts/hello.lua, line 3`.
/// Level 0 of the stack is the log function itself, so the script is
/// level 1.  When Lua doesn't know the line (the log was handed to `pcall`
/// and called from there), it's the file's name alone.
fn where_from(lua: &Lua, name: &str) -> String {
    let place = lua.inspect_stack(1, |debug| {
        let line = debug.current_line()?;
        let file = debug.source().short_src?.into_owned();
        Some(format!("{file}, line {line}"))
    });
    place.flatten().unwrap_or_else(|| name.to_string())
}

/// What went wrong, as one line for the log.  Lua's own messages already
/// start with the file and line (`scripts/hello.lua:3: attempt to call a
/// nil value`); the traceback after the first line is dropped.
fn describe(name: &str, error: &Error) -> String {
    match error {
        Error::SyntaxError { message, .. } => format!("{name} didn't load: {}", first_line(message)),
        Error::RuntimeError(message) => format!("{name} stopped with an error: {}", first_line(message)),
        Error::MemoryError(_) => format!("{name} went over its memory limit of {} MB and was stopped.",
                                         MEMORY_LIMIT / (1024 * 1024)),
        // An error that came up through one of our functions (the time
        // limit's check, or the log) is wrapped.  The one inside is the
        // one that matters.
        Error::CallbackError { cause, .. } | Error::WithContext { cause, .. } => describe(name, cause),
        other => format!("{name} stopped with an error: {}", first_line(&other.to_string())),
    }
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_reaches_the_disk_or_the_machine() {
        let script = "assert(io == nil and os == nil and require == nil and package == nil and debug == nil)
                      assert(dofile == nil and loadfile == nil and load == nil and warn == nil)
                      assert(string.dump == nil)";
        assert_eq!(run("scripts/test.lua", script), Ok(()));
    }

    #[test]
    fn the_safe_libraries_are_there() {
        let script = "assert(string.upper('a') == 'A' and math.max(1, 2) == 2 and #table.pack(1, 2) == 2)
                      assert(utf8.char(72) == 'H' and coroutine.create ~= nil)
                      log.info('from a test')";
        assert_eq!(run("scripts/test.lua", script), Ok(()));
    }

    #[test]
    fn a_syntax_error_names_the_file_and_line() {
        let why = run("scripts/test.lua", "local x = 1\nthis is not lua").unwrap_err();
        assert!(why.contains("didn't load") && why.contains("scripts/test.lua:2:"), "{why}");
    }

    #[test]
    fn a_runtime_error_names_the_file_and_line() {
        let why = run("scripts/test.lua", "local x = 1\nnot_a_function()").unwrap_err();
        assert!(why.contains("stopped with an error") && why.contains("scripts/test.lua:2:"), "{why}");
    }

    #[test]
    fn a_runaway_loop_is_stopped() {
        let started = Instant::now();
        let why = run("scripts/test.lua", "while true do end").unwrap_err();
        assert!(why.contains("time limit"), "{why}");
        assert!(started.elapsed() < TIME_LIMIT * 3);
    }

    #[test]
    fn a_runaway_loop_in_a_coroutine_is_stopped() {
        // `resume` catches the coroutine's error and hands it back, so
        // it's thrown again to reach `run()`.
        let script = "local ok, why = coroutine.resume(coroutine.create(function() while true do end end))
                      error(why, 0)";
        let why = run("scripts/test.lua", script).unwrap_err();
        assert!(why.contains("time limit"), "{why}");
    }

    #[test]
    fn a_memory_hog_is_stopped() {
        let script = "local t = {} while true do t[#t + 1] = string.rep('x', 1024 * 1024) end";
        let why = run("scripts/test.lua", script).unwrap_err();
        assert!(why.contains("memory limit"), "{why}");
    }
}
