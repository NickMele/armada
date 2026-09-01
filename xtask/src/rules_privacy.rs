//! Rule nine: nothing committed here names a person or a machine.
//!
//! Split out of `rules.rs` when the rule grew a second and a third half, so
//! that its negative tests have the home every other rule with tests already
//! has. The gate skips everything under `xtask/`, which is what lets this file
//! and its tests spell out the shapes they forbid.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::{walk, Report};

#[cfg(test)]
mod tests;

/// Credential shapes. Prefixes only — a gate that tries to recognise a secret
/// by entropy fails on both sides.
const SECRETS: &[(&str, &str)] = &[
    ("sk-ant-", "an API key"),
    ("gho_", "a token"),
    ("ghp_", "a token"),
    ("github_pat_", "a token"),
    ("xoxb-", "a token"),
    ("AKIA", "an access key"),
    ("BEGIN RSA PRIVATE KEY", "a private key"),
    ("BEGIN OPENSSH PRIVATE KEY", "a private key"),
];

/// Suffixes that only ever end the name of one machine on one network.
const HOST_SUFFIXES: &[&str] = &[".local", ".lan"];

/// The shortest name the gate will compare against. One or two characters is a
/// word in ordinary prose far more often than it is a person, and a gate that
/// fires on every `ci` is one somebody turns off.
const SHORTEST_NAME: usize = 3;

/// The one committed file that names a person on purpose: Apache-2.0 requires
/// its copyright holder by name, and a licence with the name taken out is not a
/// licence. Only the name half is lifted here — a home path or a hostname in
/// `LICENSE` is still refused.
const NAMES_ITS_OWNER: &str = "LICENSE";

// ------------------------------------------------------------ what a line says

/// One person or one machine that a line names.
enum Leak {
    /// A home directory under a name that is not the convention.
    Home(String),
    /// The name the gate itself is running under.
    RunningAs(String),
    /// A host by its shape.
    Machine(String),
}

impl Leak {
    /// What the gate says about it, with no location — the caller adds that
    /// once per file rather than once per occurrence.
    fn sentence(&self) -> String {
        match self {
            Leak::Home(name) => {
                format!("a home directory naming `{name}` — committed paths use `user`")
            }
            Leak::RunningAs(name) => {
                format!("the name this gate is running as, `{name}` — a capture nobody rewrote")
            }
            Leak::Machine(host) => format!(
                "a machine name, `{host}` — `.local` and `.lan` name one machine on one network"
            ),
        }
    }
}

/// What a username is spelled with, and so what continues a name rather than
/// ending one.
fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// Does `text[start..end]` stand as a word rather than sit inside one?
///
/// A hyphen does **not** hold a word together here, because the mangled
/// project-directory form is exactly a home path with its separators rewritten
/// to hyphens. Treating `-` as a letter would blind the gate to one of the four
/// shapes it exists for.
fn stands_alone(text: &str, start: usize, end: usize) -> bool {
    let joined = |c: char| c.is_alphanumeric() || c == '_';
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !before.is_some_and(joined) && !after.is_some_and(joined)
}

/// Every home directory the line names, in order. **All of them** — reading
/// only the segment after the first `/Users/` is what made thirteen reported
/// lines stand for a hundred and twenty-three occurrences.
fn home_directories(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    for prefix in ["/Users/", "/home/"] {
        for rest in line.split(prefix).skip(1) {
            let name: String = rest.chars().take_while(|c| is_name_char(*c)).collect();
            if !name.is_empty() && name != "user" {
                found.push(name);
            }
        }
    }
    found
}

/// Every occurrence of a name the gate is running under, as a whole word and
/// **byte for byte**.
///
/// Case-sensitivity is the whole discriminator, and it was measured rather than
/// assumed. This half exists for captured tool output, and a capture reproduces
/// the environment exactly — an `lsof` owner column, a `USER=` assignment, a
/// home path, a project directory with its separators mangled. Prose does not:
/// the repository's own GitHub slug carries its owner's account name in a
/// different case, and matching case-insensitively reddens the clone line in
/// `README.md` and the advisory link in `SECURITY.md`, neither of which can be
/// written without it. Widening past the exact bytes turns this from a capture
/// check into the guess at what a username looks like that the rule refuses.
fn names_the_line_carries(line: &str, running_as: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for name in running_as {
        let mut from = 0;
        while let Some(offset) = line[from..].find(name.as_str()) {
            let start = from + offset;
            let end = start + name.len();
            from = end;
            if stands_alone(line, start, end) {
                found.push(name.clone());
            }
        }
    }
    found
}

/// Every machine the line names, by shape rather than by a list.
///
/// Three things have to hold, and each is a false positive that is checked in
/// today or would be tomorrow:
///
/// - A host runs back from the dot, so `~/.local/bin` names none.
/// - The suffix ends the name, so `listener.local_addr()` is a method and
///   `vite.config.local.ts` is a file.
/// - The host carries a hyphen or a capital, or sits behind an `@` or a `/`.
///   A bare lowercase word before `.local` is a struct field far more often
///   than it is a host. **This is the rule's own miss**: an all-lowercase
///   unhyphenated `imac.local` standing on its own is not caught, and nothing
///   else catches it either.
fn machine_names(line: &str) -> Vec<String> {
    let haystack = line.to_ascii_lowercase();
    let mut found = Vec::new();
    for suffix in HOST_SUFFIXES {
        let mut from = 0;
        while let Some(offset) = haystack[from..].find(suffix) {
            let at = from + offset;
            let end = at + suffix.len();
            from = end;

            // What follows has to end the name. A trailing dot is a sentence,
            // but a dot with more name after it is a filename.
            let mut rest = line[end..].chars();
            match rest.next() {
                Some('.') if rest.next().is_some_and(char::is_alphanumeric) => continue,
                Some(c) if is_name_char(c) => continue,
                _ => {}
            }

            let Some(host_start) = line[..at]
                .char_indices()
                .rev()
                .take_while(|(_, c)| c.is_alphanumeric() || *c == '-')
                .map(|(i, _)| i)
                .last()
            else {
                continue;
            };
            let host = &line[host_start..at];
            // `localhost` is every machine, and so it is no machine.
            if host.eq_ignore_ascii_case("localhost") {
                continue;
            }
            let led_by = line[..host_start].chars().next_back();
            let is_a_host = host.contains('-')
                || host.chars().any(|c| c.is_uppercase())
                || matches!(led_by, Some('@') | Some('/'));
            if is_a_host {
                found.push(format!("{host}{suffix}"));
            }
        }
    }
    found
}

/// Every person and every machine one line names.
fn leaks_in(line: &str, running_as: &[String]) -> Vec<Leak> {
    let mut found: Vec<Leak> = home_directories(line).into_iter().map(Leak::Home).collect();
    found.extend(
        names_the_line_carries(line, running_as)
            .into_iter()
            .map(Leak::RunningAs),
    );
    found.extend(machine_names(line).into_iter().map(Leak::Machine));
    found
}

// ------------------------------------------------------ what the gate runs as

/// A name worth comparing against, or nothing.
///
/// Unset reaches here as empty, and `user` is the convention every committed
/// capture is rewritten to — comparing against either turns the gate into
/// noise about itself.
fn usable_name(raw: &str) -> Option<String> {
    let name = raw.trim();
    if name.len() < SHORTEST_NAME || name.eq_ignore_ascii_case("user") {
        return None;
    }
    Some(name.to_string())
}

/// The names this gate is running under: `$USER`, and git's configured
/// `user.name`.
///
/// **This is not a guess at what a username looks like and it is not a list.**
/// It is one specific fact the process already has, which is why it holds the
/// rule's two commitments. What makes it work here is that there is no CI: the
/// gate only ever runs in a checkout on the machine whose name would leak, so
/// the usual objection — that an environment check passes vacuously on a build
/// server — has no target.
///
/// **And here is the weakness.** On a machine belonging to somebody else this
/// half knows no name and finds nothing. It is a second net under the paths
/// convention, never a replacement for it, and the convention is the half that
/// holds for a contributor the gate has never seen. It is also exact rather
/// than fuzzy — see `names_the_line_carries` for why the same name in another
/// case goes unremarked, and why that is the correct trade here.
fn names_the_gate_runs_as(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut raw: Vec<String> = Vec::new();
    if let Ok(user) = std::env::var("USER") {
        raw.push(user);
    }
    if let Ok(out) = Command::new("git")
        .args(["config", "user.name"])
        .current_dir(root)
        .output()
    {
        if out.status.success() {
            raw.push(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }
    for candidate in raw {
        let Some(name) = usable_name(&candidate) else {
            continue;
        };
        if !names.iter().any(|kept| kept.eq_ignore_ascii_case(&name)) {
            names.push(name);
        }
    }
    names
}

// ----------------------------------------------------------- what a file says

/// Everything one file's text names, as findings.
///
/// **One finding per subject, with the count.** A rule that fires forty times
/// on one file for one fact is the shape that let sixty-one comment-block lines
/// hide thirteen privacy lines all night; the honest total belongs in the
/// sentence, not in the number of sentences. Credentials keep reporting per
/// line, because two tokens on two lines are two secrets and not one.
fn findings_in(rel: &str, text: &str, running_as: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let here: &[String] = if rel == NAMES_ITS_OWNER {
        &[]
    } else {
        running_as
    };
    // Subject, the line it was first seen on, and how many times in all.
    let mut seen: Vec<(String, usize, usize)> = Vec::new();

    for (n, line) in text.lines().enumerate() {
        let at = n + 1;
        for leak in leaks_in(line, here) {
            let sentence = leak.sentence();
            match seen.iter_mut().find(|(known, _, _)| *known == sentence) {
                Some(entry) => entry.2 += 1,
                None => seen.push((sentence, at, 1)),
            }
        }
        for (shape, what) in SECRETS {
            if line.contains(shape) {
                out.push(format!("{rel}:{at} — {what}, by its prefix `{shape}`"));
            }
        }
    }

    for (sentence, first, times) in seen {
        if times == 1 {
            out.push(format!("{rel}:{first} — {sentence}"));
        } else {
            out.push(format!(
                "{rel}:{first} — {sentence}. {times} times in this file, first here"
            ));
        }
    }
    out
}

/// Nothing committed here names a person or a machine.
///
/// This repository is public, and **documentation is the larger leak surface,
/// not code**. v1 learned that the expensive way: its privacy gate was written
/// after every reference that had to be scrubbed turned out to live in `docs/`,
/// a README and an agent file, none of which the code-only grep covered.
///
/// The rule is a convention rather than a guess at what a real username looks
/// like: **the only user in a committed path is `user`.** A capture from a real
/// machine gets its home path rewritten to that before it lands, which makes an
/// unrewritten one visible instead of plausible.
///
/// No allowlist. A gate with exemptions is a gate whose exemptions grow.
///
/// Three halves, and each catches what the others cannot:
///
/// | half | what it knows | where it is blind |
/// |---|---|---|
/// | the paths convention | `user` is the only one | a bare name with no path around it |
/// | the name it runs as | `$USER` and git's `user.name`, verbatim | somebody else's machine, and the same name in another case |
/// | a host by its shape | `.local` and `.lan` | a lowercase unhyphenated host standing alone |
///
/// The convention is the durable half. `names_the_gate_runs_as` says why the
/// second one holds the two commitments above, and where it does not reach.
pub fn nothing_names_a_person_or_a_machine(root: &Path) -> Report {
    let mut report = Report::new("nothing committed names a person or a machine");
    let running_as = names_the_gate_runs_as(root);

    walk(root, &mut |path| {
        let Ok(text) = fs::read_to_string(path) else {
            return;
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        // The gate names the shapes it forbids, so it always matches itself.
        if rel.starts_with("xtask/") {
            return;
        }
        for finding in findings_in(&rel, &text, &running_as) {
            report.fail(finding);
        }
    });
    report
}
