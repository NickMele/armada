//! `armada.yml`, in the slice M1 reads.
//!
//! # Five keys, and nothing else
//!
//! `version`, `id`, `checks.<name>.run`, `commands.<name>.run` and
//! `commands.<name>.destructive`. The Manifest concept page describes eight
//! sections — permissions, secrets, ports, skills, budget, dispatch freeze,
//! setup requirements, auto-merge policy — and every one of them is a key this
//! parser refuses, on purpose.
//!
//! **A key nothing reads is worse than a key that is not there.** A file
//! carrying `budget: 40` that no code consumes reads to its author as a budget
//! that is set. Refusing it means the section arrives with the code that
//! honours it, and every deferred section stays additive rather than becoming
//! a migration.
//!
//! # Two registries, sharing no names
//!
//! Checks gate advancement and Commands do not; a Check may name a Command as
//! a prerequisite, which is the reason they are separate registries rather than
//! one list with a flag. A name in both is refused at load, because a reference
//! to that name resolves to two different things with two different meanings
//! and nothing in the file says which was meant.
//!
//! # What is deliberately not enforced here
//!
//! `version` is required and must be a positive whole number, and no particular
//! value is demanded. Refusing anything but `1` would be a compatibility policy
//! this milestone step does not state, and one written into a parser is one
//! nobody can find later.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use core_model::{ManifestId, Ulid};
use serde_yaml_ng::Value;

use crate::error::{Fault, LoadError, Refusal};
use crate::yaml::{self, Table};

/// The keys M1 reads at the top level of an `armada.yml`.
const TOP_LEVEL: &[&str] = &["version", "id", "checks", "commands"];
/// The keys M1 reads inside `checks.<name>`.
const CHECK_KEYS: &[&str] = &["run"];
/// The keys M1 reads inside `commands.<name>`.
const COMMAND_KEYS: &[&str] = &["run", "destructive"];

/// A command a change must pass to land or to advance a step.
///
/// **Armada records how to invoke a tool and never what the tool means.** There
/// is no field here for what the command produces, which tests it runs or how
/// its output should be read: a Check is a command and an exit code, and
/// anything needing the output understood is a Judge question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    run: String,
}

impl Check {
    /// The command line, verbatim as the repo wrote it.
    pub fn run(&self) -> &str {
        &self.run
    }
}

/// A command available to run against the repo, gating nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    run: String,
    destructive: bool,
}

impl Command {
    pub fn run(&self) -> &str {
        &self.run
    }

    /// Whether a **Drone** invoking this pauses for approval. It does not gate
    /// a person invoking it by hand, who is already the one triggering it.
    ///
    /// Absent means `false`. The common case is a command that is not
    /// destructive, and a required flag on every entry would be noise on the
    /// many to catch the few.
    pub fn is_destructive(&self) -> bool {
        self.destructive
    }
}

/// One workspace's `armada.yml`, parsed and validated.
///
/// Carries the path it was read from, so a refusal downstream — a workflow step
/// naming a Check that is not here — can name the file that was missing it
/// without the caller having to thread a path alongside.
#[derive(Debug, Clone)]
pub struct Manifest {
    path: PathBuf,
    id: ManifestId,
    version: u32,
    checks: BTreeMap<String, Check>,
    commands: BTreeMap<String, Command>,
}

impl Manifest {
    /// Read and validate an `armada.yml`.
    pub fn load(path: &Path) -> Result<Manifest, LoadError> {
        let text = std::fs::read_to_string(path).map_err(|cause| LoadError::Unreadable {
            path: path.to_path_buf(),
            cause,
        })?;
        Manifest::parse(path, &text)
    }

    /// Validate an `armada.yml` already in hand.
    ///
    /// Separate from [`Manifest::load`] so the tests exercise the parser
    /// without a filesystem, and so nothing about a refusal depends on a file
    /// existing. `path` is carried into the refusal either way.
    pub fn parse(path: &Path, text: &str) -> Result<Manifest, LoadError> {
        let root: Value = serde_yaml_ng::from_str(text).map_err(|cause| LoadError::NotYaml {
            path: path.to_path_buf(),
            cause,
        })?;
        let mut out = Vec::new();
        let parsed = read(path, &root, &mut out);
        match parsed {
            Some(manifest) if out.is_empty() => Ok(manifest),
            _ => Err(LoadError::Refused {
                path: path.to_path_buf(),
                refusals: out,
            }),
        }
    }

    /// The file this was read from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The Manifest's own id, explicit in the file so a Workspace can move
    /// without dangling its history.
    pub fn id(&self) -> &ManifestId {
        &self.id
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    /// A Check by name, or [`None`]. The only lookup in the crate that may
    /// miss; [`crate::ResolvedWorkflow`] exists so it happens exactly once, at
    /// load, rather than at the step that needed it.
    pub fn check(&self, name: &str) -> Option<&Check> {
        self.checks.get(name)
    }

    /// A Command by name, or [`None`].
    pub fn command(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }

    /// Every declared Check name, sorted. Handed to a refusal so the message
    /// can name what the file does declare beside what it does not.
    pub fn check_names(&self) -> Vec<String> {
        self.checks.keys().cloned().collect()
    }

    /// Every declared Command name, sorted.
    pub fn command_names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }
}

/// The walk. Returns [`None`] only where nothing could be assembled at all;
/// every other fault is in `out` and the walk continues, so one pass reports
/// the whole file.
fn read(path: &Path, root: &Value, out: &mut Vec<Refusal>) -> Option<Manifest> {
    let mut top = Table::open("", root, out)?;

    let version = top
        .required("version", out)
        .and_then(|value| yaml::positive("version", value, out));
    let id = top
        .required("id", out)
        .and_then(|value| yaml::text("id", value, out));

    let checks = match top.optional("checks") {
        Some(value) => registry(value, "checks", CHECK_KEYS, out, check_entry),
        None => BTreeMap::new(),
    };
    let commands = match top.optional("commands") {
        Some(value) => registry(value, "commands", COMMAND_KEYS, out, command_entry),
        None => BTreeMap::new(),
    };
    top.close(TOP_LEVEL, out);

    // Sibling maps sharing no keys. Reported against `commands`, because the
    // Checks registry is the one that gates and is the one to keep.
    for name in commands.keys() {
        if checks.contains_key(name) {
            out.push(Refusal::new(
                format!("commands.{name}"),
                Fault::DeclaredInBothRegistries,
            ));
        }
    }

    Some(Manifest {
        path: path.to_path_buf(),
        id: ManifestId::carried(Ulid::carried(id?)),
        version: version?,
        checks,
        commands,
    })
}

/// An open-ended map of author-chosen names to entries of one shape.
fn registry<T>(
    value: &Value,
    key: &'static str,
    known: &'static [&'static str],
    out: &mut Vec<Refusal>,
    entry: fn(&str, &Value, &'static [&'static str], &mut Vec<Refusal>) -> Option<T>,
) -> BTreeMap<String, T> {
    let mut built = BTreeMap::new();
    let Some(table) = Table::open(key, value, out) else {
        return built;
    };
    for (name, item) in table.into_entries() {
        let at = format!("{key}.{name}");
        if let Some(parsed) = entry(&at, item, known, out) {
            built.insert(name, parsed);
        }
    }
    built
}

fn check_entry(
    at: &str,
    value: &Value,
    known: &'static [&'static str],
    out: &mut Vec<Refusal>,
) -> Option<Check> {
    let mut table = Table::open(at, value, out)?;
    let run = table
        .required("run", out)
        .and_then(|value| yaml::text(&table.at("run"), value, out));
    table.close(known, out);
    Some(Check { run: run? })
}

fn command_entry(
    at: &str,
    value: &Value,
    known: &'static [&'static str],
    out: &mut Vec<Refusal>,
) -> Option<Command> {
    let mut table = Table::open(at, value, out)?;
    let run = table
        .required("run", out)
        .and_then(|value| yaml::text(&table.at("run"), value, out));
    let destructive = table
        .optional("destructive")
        .and_then(|value| yaml::flag(&table.at("destructive"), value, out));
    table.close(known, out);
    Some(Command {
        run: run?,
        destructive: destructive.unwrap_or(false),
    })
}
