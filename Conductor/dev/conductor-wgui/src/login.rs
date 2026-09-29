//! File:       Opus/Conductor/dev/conductor-wgui/src/login.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! Who is at the page.  Two accounts, and no more: `user`, who can open
//! every tab and change nothing, and `admin`, who can do everything.
//! Their passwords are in `wgui.cfg` (Constellations' `WGUI`), read at
//! boot like every hard file.  They're kept as they are, not hashed:
//! Security only runs while the server does, and a login has to work
//! before START SERVER.  Fine while the page only listens on this
//! machine.  Jacob's design, 2026-09-29.
//!
//! A login hands the browser a cookie holding a random token from
//! Fingerprinter, and the token is kept here, in memory, with the role it
//! stands for.  So a login lasts until LOG OUT or until Conductor shuts
//! down, and every start of Conductor means logging in again.  Reloading
//! the page, or closing the browser and coming back, doesn't lose it.
//! The cookie is `HttpOnly` (the page's script can't read it) and
//! `SameSite=Strict` (another site's page can't send it), and the `X-Opus`
//! header checks in `lib.rs` stay as they were: the cookie says who, the
//! header says the click came from our page.
//!
//! A login that fails says the same thing whether the name or the
//! password was wrong.  Nothing in here logs a password.

use std::io;
use std::sync::Mutex;

use conductor_tools::constellations::{self, WGUI};
use conductor_tools::fingerprinter;
use conductor_tools::scribe::{self, Channel};

use crate::http::Request;

/// The cookie's name.
const COOKIE: &str = "opus_session";

/// How long the browser keeps the cookie.  The token behind it dies with
/// Conductor anyway, so this only decides whether closing the browser
/// logs you out while Conductor keeps running: it doesn't.
const COOKIE_DAYS: u64 = 30;

/// What an account is allowed to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    /// Looks and doesn't touch.
    User,
    /// Everything.
    Admin,
}

impl Role {
    /// The account's name, which is also the role's.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Admin => "admin",
        }
    }

    /// Whether this account may change anything: start and stop the
    /// server, ACK a notice, save a setting, shut Conductor down.
    pub(crate) fn can_change(self) -> bool {
        self == Role::Admin
    }
}

/// How a login went.
pub(crate) enum Login {
    /// In.  The token goes to the browser in a cookie.
    In { token: String, role: Role },
    /// Wrong name or wrong password; we don't say which.
    Wrong,
    /// The name and password were right, but the OS wouldn't give
    /// Fingerprinter the random bytes for a token.  Nobody gets in on a
    /// guessable token.
    NoToken(io::Error),
}

/// A login that's live: the token the browser holds, and who it is.
struct Session {
    token: String,
    role: Role,
}

/// Every live login.  One admin with a browser or two, so a list is
/// plenty.
static SESSIONS: Mutex<Vec<Session>> = Mutex::new(Vec::new());

/// Checks `name` and `password` against `wgui.cfg`, and on a match makes
/// a token and remembers it.
pub(crate) fn log_in(name: &str, password: &str) -> Login {
    let role = match name.trim() {
        "user" => Role::User,
        "admin" => Role::Admin,
        _ => {
            scribe::info(Channel::System, "A login to the web admin failed: the name isn't an account.");
            return Login::Wrong;
        }
    };
    let key = match role {
        Role::User => "user_password",
        Role::Admin => "admin_password",
    };
    // A plain compare.  The page only listens on this machine, and the
    // password sits as text in a file on the same machine, so there is
    // nothing a stopwatch on the compare would give away that the file
    // doesn't.
    if password != constellations::value(&WGUI, key) {
        scribe::info(Channel::System, &format!("A login to the web admin as {} failed: wrong password.",
                                              role.name()));
        return Login::Wrong;
    }

    let token = match fingerprinter::new_token() {
        Ok(token) => token,
        Err(e) => {
            scribe::error_with(Channel::System, &e, &format!("The web admin couldn't make a login token for {}.  \
                Nobody can log in until the OS gives out random bytes again.", role.name()));
            return Login::NoToken(e);
        }
    };
    let mut sessions = SESSIONS.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    sessions.push(Session { token: token.clone(), role });
    scribe::info(Channel::System, &format!("{} logged in to the web admin.", role.name()));
    Login::In { token, role }
}

/// Who sent `request`, going by its cookie.  `None` when it has no cookie,
/// or one we don't know (Conductor was run again since, say).
pub(crate) fn role_of(request: &Request) -> Option<Role> {
    let token = token_in(request)?;
    let sessions = SESSIONS.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    sessions.iter().find(|session| session.token == token).map(|session| session.role)
}

/// Forgets the login `request` carries.  True if there was one to forget.
pub(crate) fn log_out(request: &Request) -> bool {
    let Some(token) = token_in(request) else {
        return false;
    };
    let mut sessions = SESSIONS.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(index) = sessions.iter().position(|session| session.token == token) else {
        return false;
    };
    let session = sessions.remove(index);
    scribe::debug(Channel::System, &format!("{} logged out of the web admin.", session.role.name()));
    true
}

/// Our token out of the `Cookie` header, which can carry several cookies
/// as `a=1; b=2`.
fn token_in(request: &Request) -> Option<&str> {
    request.header("cookie")?
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE)
        .map(|(_, token)| token.trim())
}

/// The header line that hands the browser its cookie.
pub(crate) fn cookie_line(token: &str) -> String {
    format!("Set-Cookie: {COOKIE}={token}; Path=/Opus; Max-Age={}; HttpOnly; SameSite=Strict",
            COOKIE_DAYS * 24 * 60 * 60)
}

/// The header line that takes the cookie away again.
pub(crate) fn clear_cookie_line() -> String {
    format!("Set-Cookie: {COOKIE}=; Path=/Opus; Max-Age=0; HttpOnly; SameSite=Strict")
}

/// The name and password out of a login's body, which is two lines in
/// the config files' own format:
///
/// ```text
/// name = admin
/// password = admin
/// ```
///
/// `None` if either line is missing.  The value is trimmed the way a
/// config value is, so a password is what the file would hold.
pub(crate) fn fields(body: &str) -> Option<(String, String)> {
    let mut name = None;
    let mut password = None;
    for line in body.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim().to_ascii_lowercase().as_str() {
            "name" => name = Some(value.trim().to_string()),
            "password" => password = Some(value.trim().to_string()),
            _ => {}
        }
    }
    Some((name?, password?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_cookie(cookie: &str) -> Request {
        Request {
            method: "GET".to_string(),
            path: "/Opus/status".to_string(),
            query: String::new(),
            headers: vec![("cookie".to_string(), cookie.to_string())],
            body: String::new(),
        }
    }

    // Nothing in the tests loads wgui.cfg, so the passwords are the
    // defaults: user and admin.
    #[test]
    fn the_right_password_gets_a_token_and_the_wrong_one_doesnt() {
        let Login::In { token, role } = log_in("admin", "admin") else {
            panic!("admin / admin is the default");
        };
        assert_eq!(role, Role::Admin);
        assert_eq!(token.len(), 64);
        assert!(role.can_change());

        assert!(matches!(log_in("admin", "wrong"), Login::Wrong));
        assert!(matches!(log_in("nobody", "admin"), Login::Wrong));
        assert!(matches!(log_in("", ""), Login::Wrong));
        assert!(matches!(log_in("user", "admin"), Login::Wrong));

        let Login::In { role, .. } = log_in(" user ", "user") else {
            panic!("user / user is the default, and the name is trimmed");
        };
        assert_eq!(role, Role::User);
        assert!(!role.can_change());
    }

    #[test]
    fn the_cookie_says_who_until_they_log_out() {
        let Login::In { token, .. } = log_in("user", "user") else {
            panic!("user / user is the default");
        };
        let request = with_cookie(&format!("other=1; {COOKIE}={token}; more=2"));
        assert_eq!(role_of(&request), Some(Role::User));

        assert_eq!(role_of(&with_cookie(&format!("{COOKIE}=notatoken"))), None);
        assert_eq!(role_of(&with_cookie("other=1")), None);
        assert_eq!(role_of(&Request { headers: Vec::new(), ..with_cookie("") }), None);

        assert!(log_out(&request));
        assert_eq!(role_of(&request), None);
        assert!(!log_out(&request));
    }

    #[test]
    fn the_cookie_lines_are_shaped_right() {
        let line = cookie_line("abc");
        assert!(line.starts_with("Set-Cookie: opus_session=abc; Path=/Opus; Max-Age=2592000;"));
        assert!(line.ends_with("HttpOnly; SameSite=Strict"));
        assert!(clear_cookie_line().contains("opus_session=; Path=/Opus; Max-Age=0;"));
    }

    #[test]
    fn a_login_body_comes_apart() {
        assert_eq!(fields("name = admin\npassword = p@ss = word\n"),
                   Some(("admin".to_string(), "p@ss = word".to_string())));
        assert_eq!(fields("Password=x\r\nNAME=y"), Some(("y".to_string(), "x".to_string())));
        assert_eq!(fields("name = admin\n"), None);
        assert_eq!(fields(""), None);
    }
}
