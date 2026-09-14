//! `setup.seed`: the build directories a new worktree starts from, and the
//! Commands that fill them in the base checkout. #1064.
//!
//! **Declared by the repository, because only it knows its build.** `target`
//! here, `node_modules/.cache` or `build` elsewhere; a repository that writes no
//! `seed` gets none, whatever language it is in.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde_yaml_ng::Value;

use super::declared::{Command, Preparation};
use super::referring::named_commands;
use super::texts;
use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

/// The keys read inside `setup.seed`.
pub(super) const SEED_KEYS: &[&str] = &["paths", "warm"];

/// What a new worktree is seeded with, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seed {
    pub(super) paths: Vec<String>,
    pub(super) warm: Vec<Preparation>,
}

impl Seed {
    /// Repository-relative directories, as written, with no trailing `/`.
    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    /// The Commands that fill [`paths`](Seed::paths) in the base checkout, in
    /// the order `warm` names them.
    pub fn warmed_by(&self) -> &[Preparation] {
        &self.warm
    }
}

/// Why a `setup.seed.paths` entry names nothing a worktree can be seeded with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadSeedPath {
    /// It starts at the filesystem root.
    Absolute,
    /// It climbs out of the checkout with `..`.
    Escapes,
    /// It holds `*` or `?`.
    Globbed,
    /// It is `.` or empty, which is the whole checkout.
    TheWholeTree,
    /// It is inside `.git` or `.armada`, which are git's and Armada's own.
    NotTheRepositorys,
    /// An earlier entry names the same directory.
    Twice,
}

impl fmt::Display for BadSeedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BadSeedPath::Absolute => write!(f, "it starts at the filesystem root"),
            BadSeedPath::Escapes => write!(f, "it climbs out of the checkout"),
            BadSeedPath::Globbed => write!(f, "it is a pattern, and a seed names directories"),
            BadSeedPath::TheWholeTree => write!(f, "it names the whole checkout"),
            BadSeedPath::NotTheRepositorys => {
                write!(f, "it is inside `.git` or `.armada`, which no build writes")
            }
            BadSeedPath::Twice => write!(f, "an earlier entry names the same directory"),
        }
    }
}

/// `setup.seed`, with `warm` resolved against the Commands the file declares.
///
/// **Both keys required.** A seed with no `warm` is a directory nothing fills,
/// and one with no `paths` is a build whose output nothing reads.
pub(super) fn read(
    value: &Value,
    declares: &BTreeSet<String>,
    commands: &BTreeMap<String, Command>,
    serves: &BTreeSet<String>,
    out: &mut Vec<Refusal>,
) -> Option<Seed> {
    let mut table = Table::open("setup.seed", value, out)?;
    let paths = table
        .required("paths", out)
        .and_then(|value| yaml::list(&table.at("paths"), value, out))
        .map(|items| texts(items, out))
        .map(|items| seed_paths(items, out));
    let warm = table
        .required("warm", out)
        .and_then(|value| yaml::list(&table.at("warm"), value, out))
        .map(|items| named_commands(texts(items, out), declares, commands, serves, out));
    table.close(SEED_KEYS, out);
    Some(Seed {
        paths: paths?,
        warm: warm?
            .into_iter()
            .map(|(name, run)| Preparation { name, run })
            .collect(),
    })
}

fn seed_paths(items: Vec<(String, String)>, out: &mut Vec<Refusal>) -> Vec<String> {
    let mut kept: Vec<String> = Vec::with_capacity(items.len());
    for (key, written) in items {
        let trimmed = written.trim_end_matches('/').to_string();
        let why = match refused(&trimmed) {
            None if kept.contains(&trimmed) => Some(BadSeedPath::Twice),
            other => other,
        };
        match why {
            None => kept.push(trimmed),
            Some(why) => out.push(Refusal::new(
                key,
                Fault::NotASeedPath {
                    value: written,
                    why,
                },
            )),
        }
    }
    kept
}

fn refused(path: &str) -> Option<BadSeedPath> {
    if path.starts_with('/') {
        return Some(BadSeedPath::Absolute);
    }
    if path.contains(['*', '?']) {
        return Some(BadSeedPath::Globbed);
    }
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    if parts.contains(&"..") {
        return Some(BadSeedPath::Escapes);
    }
    match parts.iter().find(|part| **part != ".") {
        None => Some(BadSeedPath::TheWholeTree),
        Some(&".git") | Some(&".armada") => Some(BadSeedPath::NotTheRepositorys),
        Some(_) => None,
    }
}
