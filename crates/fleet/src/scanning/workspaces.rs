//! Which directories are workspaces, found two ways in one pass.
//!
//! **By walking**, for a directory holding a manifest of its own — v1's first
//! Scan read the root alone and was blind on every monorepo. **By the
//! repository's own patterns**, which cite the entry naming each directory and
//! reach deeper than the walk goes.
//!
//! `.gitignore` is not read; `crate::files` names the same gap.

use std::collections::{BTreeMap, BTreeSet};

use ipc::{NotRead, WorkspaceGlob, WorkspaceGlobs};

use super::{at, not_read, Read, Tree};
use crate::drifting::declared;

/// How far below the root the walk descends. At four the things found stop
/// being packages — a vendored copy, a fixture, an example inside a library.
const DEPTH: usize = 3;

/// Directories never walked into: dependencies, build output, caches.
const SKIP: &[&str] = &[
    "__pycache__",
    "_build",
    "bower_components",
    "build",
    "deps",
    "dist",
    "node_modules",
    "site-packages",
    "target",
    "vendor",
    "venv",
];

/// The files that make a directory a package of its own.
pub(super) const MANIFESTS: &[(&str, &str)] = &[
    ("package.json", "node"),
    ("Cargo.toml", "cargo"),
    ("pyproject.toml", "python"),
];

/// Files making a directory a workspace this read does not follow — listed so
/// the workspace is there and says so, rather than reading as nothing.
pub(super) const NOT_FOLLOWED: &[&str] = &[
    "GNUmakefile",
    "Gemfile",
    "Justfile",
    "Makefile",
    "build.gradle",
    "build.gradle.kts",
    "composer.json",
    "deno.json",
    "go.mod",
    "go.work",
    "justfile",
    "makefile",
    "mix.exs",
    "pom.xml",
    "setup.py",
];

/// Every workspace directory, with its file names and the pattern entries
/// naming it, and what was not read on the way.
pub(super) struct Found {
    pub(super) dirs: BTreeMap<String, (BTreeSet<String>, Vec<WorkspaceGlob>)>,
    pub(super) not_read: Vec<NotRead>,
}

pub(super) fn discover(tree: &impl Tree) -> Found {
    let mut found = Found {
        dirs: BTreeMap::new(),
        not_read: Vec::new(),
    };
    match tree.entries("") {
        Ok(entries) => {
            let names = entries.iter().filter(|e| !e.is_dir).map(|e| e.name.clone());
            found
                .dirs
                .insert(String::new(), (names.collect(), Vec::new()));
            for entry in entries.iter().filter(|e| e.is_dir) {
                if entry.name.starts_with('.') {
                    hidden_yaml(tree, &entry.name, &mut found.not_read);
                } else if !SKIP.contains(&entry.name.as_str()) {
                    walk(tree, &entry.name, 1, &mut found);
                }
            }
        }
        Err(why) => {
            let why = format!("the checkout would not list: {why}");
            found.not_read.push(not_read(".".to_string(), why));
            found
                .dirs
                .insert(String::new(), (BTreeSet::new(), Vec::new()));
        }
    }
    patterns(tree, &mut found);
    found
}

fn walk(tree: &impl Tree, dir: &str, depth: usize, found: &mut Found) {
    let entries = match tree.entries(dir) {
        Ok(entries) => entries,
        Err(why) => {
            let why = format!("would not list: {why}");
            return found.not_read.push(not_read(dir.to_string(), why));
        }
    };
    let names = file_names(&entries);
    if is_workspace(&names) {
        found
            .dirs
            .entry(dir.to_string())
            .or_insert_with(|| (names, Vec::new()));
    }
    if depth >= DEPTH {
        return;
    }
    for entry in entries.iter().filter(|e| e.is_dir) {
        if !entry.name.starts_with('.') && !SKIP.contains(&entry.name.as_str()) {
            walk(tree, &at(dir, &entry.name), depth + 1, found);
        }
    }
}

fn file_names(entries: &[super::Entry]) -> BTreeSet<String> {
    entries
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.name.clone())
        .collect()
}

fn is_workspace(names: &BTreeSet<String>) -> bool {
    MANIFESTS.iter().any(|(name, _)| names.contains(*name))
        || NOT_FOLLOWED.iter().any(|name| names.contains(*name))
}

/// Every YAML file under a hidden root directory, two levels down, reported as
/// not read. A hosted CI's configuration is one of these, and whose it is is
/// the adapters' to know, so none is named.
fn hidden_yaml(tree: &impl Tree, dir: &str, not_read_here: &mut Vec<NotRead>) {
    if matches!(dir, ".git" | ".armada" | ".cargo") {
        return;
    }
    let mut pending = vec![(dir.to_string(), 0)];
    while let Some((dir, depth)) = pending.pop() {
        let Ok(mut entries) = tree.entries(&dir) else {
            continue;
        };
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        for entry in entries {
            let path = at(&dir, &entry.name);
            if entry.is_dir && depth < 1 {
                pending.push((path, depth + 1));
            } else if !entry.is_dir && is_yaml(&entry.name) {
                let why = "YAML under a hidden directory, which no part of Scan reads";
                not_read_here.push(not_read(path, why));
            }
        }
    }
}

pub(super) fn is_yaml(name: &str) -> bool {
    name.ends_with(".yml") || name.ends_with(".yaml")
}

/// The root's workspace patterns, each entry expanded or said to be unread.
fn patterns(tree: &impl Tree, found: &mut Found) {
    let mut listed: Vec<(&str, Option<Vec<String>>)> = Vec::new();
    if let Read::Bytes(bytes) = tree.read("pnpm-workspace.yaml") {
        let text = String::from_utf8_lossy(&bytes);
        listed.push(("pnpm-workspace.yaml", yaml_list(&text, "packages")));
    }
    if let Read::Bytes(bytes) = tree.read("package.json") {
        // One that will not decode is reported where its scripts are lost too.
        if let Ok(decoded) = ipc::decode::<ipc::PackageWorkspaces>("a package.json", &bytes) {
            let entries = match decoded.workspaces {
                None => Vec::new(),
                Some(WorkspaceGlobs::Listed(entries)) => entries,
                Some(WorkspaceGlobs::Nested { packages }) => packages,
            };
            listed.push(("package.json", Some(entries)));
        }
    }
    if let Read::Bytes(bytes) = tree.read("Cargo.toml") {
        let text = String::from_utf8_lossy(&bytes);
        listed.push(("Cargo.toml", declared::member_entries(&text)));
    }

    for (file, entries) in listed {
        let Some(entries) = entries else {
            let why = "its workspace list is in a shape this read does not follow";
            found.not_read.push(not_read(file.to_string(), why));
            continue;
        };
        for entry in entries {
            expand(tree, file, &entry, found);
        }
    }
}

/// A directory entry and one ending `/*` are expanded; anything else is said
/// to be unread, because a list only partly expanded would call a real package
/// missing. The walk may still find what it named.
fn expand(tree: &impl Tree, file: &str, entry: &str, found: &mut Found) {
    let cited = format!("{file}: {entry}");
    let trimmed = entry.trim_start_matches("./").trim_end_matches('/');
    let dirs: Vec<String> = match trimmed.strip_suffix("/*") {
        Some(parent) if !has_pattern(parent) => match tree.entries(parent) {
            Ok(entries) => entries
                .into_iter()
                .filter(|e| e.is_dir)
                .map(|e| at(parent, &e.name))
                .collect(),
            Err(_) => {
                let why = "names a directory this checkout does not have";
                return found.not_read.push(not_read(cited, why));
            }
        },
        _ if has_pattern(trimmed) => {
            let why = "a pattern this read does not expand";
            return found.not_read.push(not_read(cited, why));
        }
        _ => vec![trimmed.to_string()],
    };

    let glob = WorkspaceGlob {
        file: file.to_string(),
        entry: entry.to_string(),
    };
    for dir in dirs {
        let names = tree
            .entries(&dir)
            .map(|entries| file_names(&entries))
            .unwrap_or_default();
        if !is_workspace(&names) {
            if !trimmed.ends_with("/*") {
                let why = "names a directory holding no manifest this read knows";
                found.not_read.push(not_read(cited.clone(), why));
            }
            continue;
        }
        let (_, by) = found.dirs.entry(dir).or_insert_with(|| (names, Vec::new()));
        by.push(glob.clone());
    }
}

fn has_pattern(entry: &str) -> bool {
    entry.contains(['*', '?', '[', '{', '!'])
}

/// A top-level YAML list, as an indented block of `- entry` or `[a, b]` on one
/// line. `None` for any other shape; an absent key is an empty list.
pub(super) fn yaml_list(text: &str, key: &str) -> Option<Vec<String>> {
    let mut lines = text.lines();
    let prefix = format!("{key}:");
    let rest = loop {
        let Some(line) = lines.next() else {
            return Some(Vec::new());
        };
        if let Some(rest) = line.strip_prefix(&prefix) {
            break rest.trim();
        }
    };
    if rest.starts_with('[') {
        return declared::quoted(rest);
    }
    if !rest.is_empty() && !rest.starts_with('#') {
        return None;
    }
    let mut entries = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !line.starts_with([' ', '\t']) {
            break;
        }
        let item = trimmed.strip_prefix("- ")?.trim();
        entries.push(scalar(item)?);
    }
    Some(entries)
}

/// One YAML scalar: quoted, or bare up to a trailing comment.
pub(super) fn scalar(item: &str) -> Option<String> {
    match item.chars().next()? {
        quote @ ('"' | '\'') => Some(item[1..].split(quote).next()?.to_string()),
        _ => Some(item.split(" #").next()?.trim().to_string()),
    }
}
