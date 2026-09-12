//! What the repository's own build files declare, read for drift and for
//! nothing else.
//!
//! Three answers: the `scripts` a `package.json` names, the `[alias]` table in
//! `.cargo/config.toml`, and the packages a Cargo workspace holds. Each is a
//! file read. **Nothing here spawns** — `cargo metadata` would answer the third
//! question better and costs a process per open, which is the one price drift
//! may not pay.
//!
//! # No parser, and the failure is an answer
//!
//! Decoding untyped JSON is refused by the gate outside `store` and `ipc`, and
//! `fleet` is neither — so `package.json` is read by a scan that knows JSON strings and
//! brace depth and nothing more, the way `xtask` reads `operations.toml`. What
//! makes that safe rather than merely allowed is where it fails: every reader
//! answers [`None`] where it could not read the file with confidence, and a
//! `None` becomes *not followed* on the row. A reader that is wrong about a file
//! produces an honest row, never a false `gone`.
//!
//! **An absent file and an unreadable one are different answers.** No
//! `.cargo/config.toml` is a repository declaring no aliases, which is a fact;
//! a `package.json` that does not scan is nothing at all.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// One checkout's build files, each read at most once per drift read.
///
/// **Lazy and cached**, because sixteen lines naming the root `package.json`
/// four times should read it once — and a line that names no cargo word should
/// not pay for walking the workspace.
pub(crate) struct Repository<'a> {
    checkout: &'a Path,
    scripts: BTreeMap<String, Option<BTreeSet<String>>>,
    aliases: Option<Option<BTreeMap<String, String>>>,
    members: Option<Option<BTreeSet<String>>>,
}

impl<'a> Repository<'a> {
    pub(crate) fn at(checkout: &'a Path) -> Repository<'a> {
        Repository {
            checkout,
            scripts: BTreeMap::new(),
            aliases: None,
            members: None,
        }
    }

    pub(crate) fn checkout(&self) -> &Path {
        self.checkout
    }

    /// The script names in `<dir>/package.json`. `dir` is relative to the
    /// checkout and empty for the root. **`None` where the file is absent or
    /// does not scan** — the caller tells the two apart by looking for it.
    pub(crate) fn scripts(&mut self, dir: &str) -> Option<&BTreeSet<String>> {
        let checkout = self.checkout;
        self.scripts
            .entry(dir.to_string())
            .or_insert_with(|| {
                let text = std::fs::read_to_string(checkout.join(dir).join("package.json"));
                scripts(&text.ok()?)
            })
            .as_ref()
    }

    /// Every `[alias]` in `.cargo/config.toml`, with what it expands to.
    /// **Empty where there is no such file**; `None` only where one is there
    /// and could not be read.
    pub(crate) fn aliases(&mut self) -> Option<&BTreeMap<String, String>> {
        let checkout = self.checkout;
        self.aliases
            .get_or_insert_with(|| {
                // `config` without the extension is the name cargo still reads,
                // and a repository that has not renamed it is not wrong.
                for name in [".cargo/config.toml", ".cargo/config"] {
                    let path = checkout.join(name);
                    if path.exists() {
                        return aliases(&std::fs::read_to_string(path).ok()?);
                    }
                }
                Some(BTreeMap::new())
            })
            .as_ref()
    }

    /// The package names this workspace holds. `None` where there is no root
    /// `Cargo.toml`, or its members could not all be resolved.
    pub(crate) fn members(&mut self) -> Option<&BTreeSet<String>> {
        let checkout = self.checkout;
        self.members
            .get_or_insert_with(|| members(checkout))
            .as_ref()
    }
}

/// The keys of the top-level `"scripts"` object in a `package.json`.
///
/// A scan, not a parse: strings with their escapes, and the depth of every
/// `{` and `[`. **Unbalanced, or not an object at the top, is `None`.** No
/// `"scripts"` key at all is an empty set, because that is a package declaring
/// no scripts.
pub(crate) fn scripts(text: &str) -> Option<BTreeSet<String>> {
    // Each open container: whether it is an object, and whether the next
    // string in it is a key.
    let mut open: Vec<(bool, bool)> = Vec::new();
    let mut last_top_key = String::new();
    let mut inside_scripts: Option<usize> = None;
    let mut found = BTreeSet::new();
    let mut saw_root = false;
    let mut chars = text.chars();

    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {}
            '{' | '[' => {
                if open.is_empty() {
                    if c != '{' || saw_root {
                        return None;
                    }
                    saw_root = true;
                }
                if c == '{' && open.len() == 1 && last_top_key == "scripts" {
                    inside_scripts = Some(2);
                }
                open.push((c == '{', c == '{'));
            }
            '}' | ']' => {
                let (object, _) = open.pop()?;
                if object != (c == '}') {
                    return None;
                }
                if inside_scripts == Some(open.len() + 1) {
                    inside_scripts = None;
                }
            }
            ',' => {
                let top = open.last_mut()?;
                top.1 = top.0;
            }
            ':' => {
                open.last_mut()?.1 = false;
            }
            '"' => {
                let string = string(&mut chars)?;
                let depth = open.len();
                let top = open.last()?;
                if top.0 && top.1 {
                    if depth == 1 {
                        last_top_key = string.clone();
                    }
                    if inside_scripts == Some(depth) {
                        found.insert(string);
                    }
                }
            }
            _ => {
                if open.is_empty() {
                    return None;
                }
            }
        }
    }
    match open.is_empty() && saw_root {
        true => Some(found),
        false => None,
    }
}

/// The rest of one JSON string, its opening quote already taken. Escapes are
/// decoded where they are one character; a `\u` is kept as written, because a
/// script name spelled with one is a name this read would rather not follow
/// than misspell.
fn string(chars: &mut std::str::Chars<'_>) -> Option<String> {
    let mut out = String::new();
    loop {
        match chars.next()? {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'u' => out.push_str("\\u"),
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
}

/// The `[alias]` table of a cargo config, each name with its expansion as one
/// line. Both TOML forms cargo accepts — a string, or an array of strings.
/// **`None` where an entry is in a shape this does not read**, rather than
/// dropping the entry and answering as though it were not declared.
pub(crate) fn aliases(text: &str) -> Option<BTreeMap<String, String>> {
    let mut found = BTreeMap::new();
    let mut in_alias = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_alias = line == "[alias]";
            continue;
        }
        if !in_alias {
            continue;
        }
        let (name, value) = line.split_once('=')?;
        let name = name.trim().trim_matches('"');
        let value = value.trim();
        let expansion = match value.chars().next()? {
            '"' | '\'' => value[1..].split(['"', '\'']).next()?.to_string(),
            '[' => quoted(value)?.join(" "),
            _ => return None,
        };
        found.insert(name.to_string(), expansion);
    }
    Some(found)
}

/// Every quoted string in a one-line TOML array. `None` where the array does
/// not close on the line, which is a shape this read does not follow.
fn quoted(value: &str) -> Option<Vec<String>> {
    let inner = value.strip_prefix('[')?.split(']').next()?;
    if !value.contains(']') {
        return None;
    }
    Some(
        inner
            .split(',')
            .map(|one| one.trim().trim_matches(['"', '\'']).to_string())
            .filter(|one| !one.is_empty())
            .collect(),
    )
}

/// The package names in the workspace rooted at `checkout`, and the root's own
/// where it is a package too.
///
/// `members` entries are directories, or a directory ending `/*`. **Any other
/// glob makes the whole answer `None`**: a member list this read could only
/// partly expand would call a real package missing.
fn members(checkout: &Path) -> Option<BTreeSet<String>> {
    let root = std::fs::read_to_string(checkout.join("Cargo.toml")).ok()?;
    let mut names = BTreeSet::new();
    if let Some(name) = package_name(&root) {
        names.insert(name);
    }
    for member in member_entries(&root)? {
        let dirs = match member.strip_suffix("/*") {
            Some(parent) => std::fs::read_dir(checkout.join(parent))
                .ok()?
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.join("Cargo.toml").is_file())
                .collect(),
            None if member.contains(['*', '?', '[']) => return None,
            None => vec![checkout.join(&member)],
        };
        for dir in dirs {
            let text = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
            names.insert(package_name(&text)?);
        }
    }
    Some(names)
}

/// `name` under `[package]`, where the file has one.
fn package_name(text: &str) -> Option<String> {
    let mut in_package = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package {
            if let Some(value) = line.strip_prefix("name") {
                let value = value.trim_start().strip_prefix('=')?.trim();
                return Some(value.trim_matches(['"', '\'']).to_string());
            }
        }
    }
    None
}

/// The `members` array under `[workspace]`, which may span lines. An empty
/// list where the file declares no workspace — a single package is a real
/// answer, carried by [`package_name`].
fn member_entries(text: &str) -> Option<Vec<String>> {
    let mut in_workspace = false;
    let mut collecting: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(so_far) = collecting.as_mut() {
            so_far.push_str(trimmed);
            if trimmed.contains(']') {
                return quoted(so_far);
            }
            continue;
        }
        if trimmed.starts_with('[') {
            in_workspace = trimmed == "[workspace]";
            continue;
        }
        if in_workspace {
            if let Some(value) = trimmed.strip_prefix("members") {
                let value = value.trim_start().strip_prefix('=')?.trim();
                if value.contains(']') {
                    return quoted(value);
                }
                collecting = Some(value.to_string());
            }
        }
    }
    match collecting {
        Some(_) => None,
        None => Some(Vec::new()),
    }
}
