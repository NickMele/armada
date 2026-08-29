//! The one chokepoint a filed report's text passes through.
//!
//! **Redacted on the way in.** A report is read weeks later, promoted into an
//! issue, and handed to a Drone as facts; scrubbing at read time would leave a
//! window the length of the file's life, and a record that has already written
//! a credential cannot un-write it. So [`Redactor::scrub`] runs before the
//! store write, and nowhere else does a report's text reach the store.
//!
//! **`$HOME` never appears**, as the log envelope already holds: a home
//! directory carries the operator's name, and this is the one record written
//! expressly to be shown to somebody else. `ManifestSummary.path` deliberately
//! keeps its home and does not conflict — that value is two processes agreeing
//! where a file is, and it is never written down.
//!
//! **Only what is named is caught.** A value beside a credential-shaped name,
//! a value after such a flag, a bearer, a private key block. There is no
//! entropy rule: "any long opaque token" would eat the commit hashes and ids a
//! report exists to give a reader and would still miss a short password. And no
//! vendor prefixes — that list is a better detector *and* a list of credential
//! shapes committed to a public repository, which this repository's own guard
//! refuses. So a bare token, named nothing, is not caught.
//!
//! **Redacted from, never with.** The name survives, because that a variable
//! was set is often the diagnostic; a redactor that dropped whole lines makes
//! attachments useless, and a useless attachment gets the guard turned off.

/// What replaces a value that must not be written down.
const REDACTED: &str = "[redacted]";

/// What replaces a home directory. Not the literal string `$HOME`: `~` is what
/// a person reads a path with, and the point is that the path stays useful.
const TILDE: &str = "~";

/// A name whose value is a credential. Matched case-insensitively as a
/// substring, so `AWS_SECRET_ACCESS_KEY` is caught by `secret`.
///
/// **Deliberately narrow, and `auth` is not here.** `auth` catches `author`,
/// and a report about a Job whose commit author was redacted is a report
/// missing the fact somebody filed it to explain. `authorization` is the
/// spelling that means the header.
const CREDENTIAL_NAMES: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "passphrase",
    "credential",
    "apikey",
    "api_key",
    "access_key",
    "private_key",
    "authorization",
];

/// The scrubber a report's every string passes through.
///
/// It holds the home directory rather than reading one: **nothing in this crate
/// reads the environment below Fleet**, and the home Fleet was assembled with
/// is the one a Drone's paths were built from, so it is the one that appears in
/// the text being scrubbed.
#[derive(Clone, Debug)]
pub struct Redactor {
    home: String,
}

impl Redactor {
    /// A redactor that hides this home directory.
    ///
    /// A blank home, or `/`, hides nothing: replacing `/` with `~` would
    /// rewrite every path in the record and lose the paths as well as the home.
    pub fn with_home(home: &str) -> Redactor {
        let home = home.trim_end_matches('/');
        Redactor {
            home: match home.len() > 1 {
                true => home.to_string(),
                false => String::new(),
            },
        }
    }

    /// One string, safe to write down.
    ///
    /// Line by line, because a private key spans lines and a token does not.
    pub fn scrub(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut inside_a_key = false;
        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                out.push('\n');
            }
            if inside_a_key {
                inside_a_key = !line.contains("-----END");
                continue;
            }
            if line.contains("-----BEGIN") && line.contains("PRIVATE KEY") {
                inside_a_key = !line.contains("-----END");
                out.push_str("[redacted private key]");
                continue;
            }
            out.push_str(&self.scrub_line(line));
        }
        out
    }

    /// One line, token by token, keeping the spacing the line had.
    ///
    /// Splitting on whitespace and rejoining would reflow a diff and a check's
    /// output, which are read for their shape as much as their words — so the
    /// separators are kept and only the tokens are looked at.
    fn scrub_line(&self, line: &str) -> String {
        let mut out = String::with_capacity(line.len());
        let mut redact_the_next = false;
        for piece in split_keeping_whitespace(line) {
            if piece.trim().is_empty() {
                out.push_str(piece);
                continue;
            }
            let (kept, next) = self.scrub_token(piece, redact_the_next);
            redact_the_next = next;
            out.push_str(&kept);
        }
        self.without_home(&out)
    }

    /// What this token becomes, and whether the one after it is a value that
    /// must not be written.
    fn scrub_token(&self, token: &str, redact_it: bool) -> (String, bool) {
        if redact_it {
            let (value, trailing) = trailing_punctuation(token);
            if !value.is_empty() {
                return (format!("{REDACTED}{trailing}"), false);
            }
        }
        // `NAME=value`, which is how a credential arrives on a command line and
        // in an environment. The name is kept: that the variable was set is
        // often the whole diagnostic.
        if let Some((name, value)) = token.split_once('=') {
            if credential_named(name) && !value.is_empty() {
                return (format!("{name}={REDACTED}"), false);
            }
        }
        // `"token": "value"` and `token: value` — the same pair with a colon,
        // which is how one arrives in JSON and in a header. A colon at the end
        // of the token means the value is the next one along.
        if let Some((name, value)) = token.split_once(':') {
            if credential_named(name) {
                return match value.trim_matches('"').is_empty() {
                    true => (token.to_string(), true),
                    false => (format!("{name}:{REDACTED}"), false),
                };
            }
        }
        // `--token abc`, and `Bearer abc`. Both name the next token.
        if credential_named(token) && token.starts_with('-') {
            return (token.to_string(), true);
        }
        if token.eq_ignore_ascii_case("bearer") {
            return (token.to_string(), true);
        }
        // Nothing else. A token that names itself is exactly what the prefix
        // list would catch, and the module doc says why there is none.
        (token.to_string(), false)
    }

    /// The home directory, as `~`. Every path in the record goes through here.
    fn without_home(&self, text: &str) -> String {
        match self.home.is_empty() {
            true => text.to_string(),
            false => text.replace(&self.home, TILDE),
        }
    }
}

/// Whether a name says its value is a credential.
fn credential_named(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    CREDENTIAL_NAMES.iter().any(|word| name.contains(word))
}

/// The token without whatever punctuation closes it, and that punctuation.
///
/// `"abc",` is a value in JSON and the comma is not part of it.
fn trailing_punctuation(token: &str) -> (&str, &str) {
    let end = token
        .trim_end_matches(|c: char| matches!(c, ',' | ';' | '"' | '\'' | ')' | ']' | '}' | '.'))
        .len();
    (&token[..end], &token[end..])
}

/// The line, in runs of whitespace and runs of everything else, in order.
fn split_keeping_whitespace(line: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut space = None;
    for (at, ch) in line.char_indices() {
        let is_space = ch.is_whitespace();
        match space {
            Some(was) if was == is_space => {}
            Some(_) => {
                pieces.push(&line[start..at]);
                start = at;
            }
            None => start = at,
        }
        space = Some(is_space);
    }
    if start < line.len() {
        pieces.push(&line[start..]);
    }
    pieces
}
