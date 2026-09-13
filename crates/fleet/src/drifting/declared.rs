//! What the repository's own build files declare, read for drift and for
//! nothing else.
//!
//! Three answers: the `scripts` a `package.json` names, the `[alias]` table in
//! `.cargo/config.toml`, and the packages a Cargo workspace holds. Each is a
//! file read. **Nothing here spawns** — `cargo metadata` would answer the third
//! question better and costs a process per open, which is the one price drift
//! may not pay.
//!
//! # `package.json` goes through `ipc`'s codec
//!
//! The gate lets only `store` and `ipc` decode untyped JSON, because a decode
//! failure that is quietly skipped is how v1 lost 21 Jobs. So `package.json`
//! is decoded through [`ipc::decode`] into [`ipc::PackageScripts`], the same
//! door `crate::rehearsing::records` uses for a run record. There is no second
//! parser to disagree with the real one about a nested `"scripts"` key or an
//! escaped quote. The two TOML files are still read line by line, the way
//! `xtask` reads `operations.toml`, and like the decode, each reader's failure is
//! [`None`]. A `None` makes the row *not followed*, never `gone`.
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
                let bytes = std::fs::read(checkout.join(dir).join("package.json"));
                scripts(&bytes.ok()?)
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

/// The script names a `package.json` declares, decoded through `ipc`.
///
/// **`None` where it will not decode**, and the caller turns that into *not
/// followed*. No `"scripts"` key at all decodes to an empty set, since that is a
/// package declaring no scripts.
pub(crate) fn scripts(bytes: &[u8]) -> Option<BTreeSet<String>> {
    Some(script_lines(bytes)?.into_keys().collect())
}

/// Each script with the command it runs — the one decode both drift and
/// `crate::scanning` read a `package.json` through, so the two cannot come to
/// disagree about what a script is.
pub(crate) fn script_lines(bytes: &[u8]) -> Option<BTreeMap<String, String>> {
    let decoded = ipc::decode::<ipc::PackageScripts>("a package.json", bytes).ok()?;
    Some(decoded.scripts)
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
pub(crate) fn quoted(value: &str) -> Option<Vec<String>> {
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
pub(crate) fn member_entries(text: &str) -> Option<Vec<String>> {
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
