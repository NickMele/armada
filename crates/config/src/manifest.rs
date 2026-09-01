//! `armada.yml`, in the slice M1 reads.
//!
//! **Seven keys, and nothing else.** `version`, `id`, `base`,
//! `checks.<name>.run`, `checks.<name>.when`, `commands.<name>.run`,
//! `commands.<name>.destructive` and `setup.requires`. Every other section the
//! Manifest concept page describes — permissions, secrets, ports, skills,
//! budget, dispatch freeze, auto-merge — is a key this parser refuses.
//!
//! **A key nothing reads is worse than a key that is not there.** A file
//! carrying `budget: 40` that no code consumes reads to its author as a budget
//! that is set. Refusing it means the section arrives with the code that
//! honours it, and every deferred section stays additive rather than a
//! migration. [`Manifest::version`] refuses no number for the same reason.
//!
//! `checks.<name>.when` is a list of path patterns in
//! `core_model::PathPattern`'s dialect, checked at load, so a pattern this
//! parser cannot read is a refusal beside every other refusal in the file
//! rather than a Check that quietly stops running. **Absent means always.**
//!
//! `setup.requires` names Commands the same file declares, each resolved to its
//! `run` string **at load** — a name nothing declares is a refusal here rather
//! than a worktree nothing prepares and a Check that fails for a reason nobody
//! can connect to it. **Its code word is *preparation***, because
//! `armada::setup` runs nothing and means something else — one word over two
//! meanings is the second vocabulary this workspace refuses elsewhere.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use core_model::{Covers, ManifestId, PathPattern, Ulid};
use serde_yaml_ng::Value;

use crate::error::{Fault, LoadError, Refusal};
use crate::yaml::{self, Table};

/// The keys M1 reads at the top level of an `armada.yml`.
const TOP_LEVEL: &[&str] = &["version", "id", "base", "checks", "commands", "setup"];
/// The keys M1 reads inside `checks.<name>`.
const CHECK_KEYS: &[&str] = &["run", "when"];
/// The keys M1 reads inside `commands.<name>`.
const COMMAND_KEYS: &[&str] = &["run", "destructive"];
/// The keys M1 reads inside `setup`.
const SETUP_KEYS: &[&str] = &["requires"];

/// A command a change must pass to land or to advance a step.
///
/// **Armada records how to invoke a tool and never what the tool means.** There
/// is no field here for what the command produces, which tests it runs or how
/// its output should be read: a Check is a command and an exit code, and
/// anything needing the output understood is a Judge question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    run: String,
    when: Option<Covers>,
}

impl Check {
    /// The command line, verbatim as the repo wrote it.
    pub fn run(&self) -> &str {
        &self.run
    }

    /// Which paths this Check covers. **`None` where the file declares no
    /// `when`, and that means always** — never "covers nothing".
    ///
    /// An `Option` rather than an empty [`Covers`] because the two would be one
    /// value with opposite meanings, and [`Covers::of`] has no way to build an
    /// empty one for exactly that reason.
    pub fn when(&self) -> Option<&Covers> {
        self.when.as_ref()
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

/// A Command that has to run in a worktree before any step does, resolved.
///
/// **Name and command line together, because the two answer different
/// questions and a failure needs both.** The name is what the file wrote and
/// what a person edits; the `run` string is what was executed. A failure
/// reporting only the second reads as an install that broke on its own, which
/// is the mystery this whole key exists to end.
///
/// There is no way to build one but by resolving a `setup.requires` entry
/// against a declared Command, so a caller holding one is holding a name the
/// Manifest declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preparation {
    name: String,
    run: String,
}

impl Preparation {
    /// The Command's name, as `setup.requires` wrote it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The command line, taken from `commands.<name>.run` at load.
    pub fn run(&self) -> &str {
        &self.run
    }
}

/// One workspace's `armada.yml`, parsed and validated.
///
/// Carries the path it was read from, so a refusal downstream — a workflow step
/// naming a Check that is not here — can name the file that was missing it
/// without the caller having to thread a path alongside.
///
/// **Two registries, sharing no names.** Checks gate advancement and Commands
/// do not, and a Check may name a Command as a prerequisite, which is why they
/// are separate registries rather than one list with a flag. A name in both is
/// refused at load, because a reference to it resolves to two different things
/// with two different meanings and nothing in the file says which was meant.
#[derive(Debug, Clone)]
pub struct Manifest {
    path: PathBuf,
    id: ManifestId,
    version: u32,
    base: Option<String>,
    checks: BTreeMap<String, Check>,
    commands: BTreeMap<String, Command>,
    prepared_by: Vec<Preparation>,
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

    /// **Required, and a positive whole number, and nothing more.** Refusing
    /// anything but `1` would be a compatibility policy this milestone step
    /// does not state, and one written into a parser is one nobody can find.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// The branch a Job's work merges into, where the file names one.
    ///
    /// **Optional, and absent is not a default.** A repository that does not
    /// say has its base inferred from what git already knows, which is a
    /// reading rather than an answer — so the two cases stay distinguishable
    /// all the way to the line a person reads.
    pub fn base(&self) -> Option<&str> {
        self.base.as_deref()
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

    /// The Commands that have to run in a fresh worktree before the first
    /// Drone, **in the order `setup.requires` names them**.
    ///
    /// Order is the file's and is kept: `[install, generate]` is a sequence
    /// somebody wrote, and sorting it would run the second before what it
    /// depends on.
    ///
    /// **Already resolved, so there is nothing to look up and nothing to
    /// miss.** Empty where the file requires none; `setup.requires: []` is
    /// refused rather than read as none, for `when`'s reason — a list with
    /// nothing in it is a key to delete.
    pub fn prepared_by(&self) -> &[Preparation] {
        &self.prepared_by
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

    let base = top
        .optional("base")
        .and_then(|value| yaml::text("base", value, out));

    let checks = match top.optional("checks") {
        Some(value) => registry(value, "checks", CHECK_KEYS, out, check_entry),
        None => BTreeMap::new(),
    };
    let commands = match top.optional("commands") {
        Some(value) => registry(value, "commands", COMMAND_KEYS, out, command_entry),
        None => BTreeMap::new(),
    };
    // After `commands`, because every entry is resolved against it. Nothing
    // about the order of the file matters — `Table` reads by name — only that
    // the registry is built before it is consulted.
    let prepared_by = match top.optional("setup") {
        Some(value) => preparation(value, &checks, &commands, out),
        None => Vec::new(),
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
        base,
        checks,
        commands,
        prepared_by,
    })
}

/// `setup.requires`, resolved against the Commands the same file declares.
///
/// **The refusal that had to come with the key.** `crates/config/tests/shipped.rs`
/// asks the same question of a step naming an undeclared Check, and it exists
/// because one such step passed every test and failed at dispatch with a Drone
/// already spawned. A `setup.requires` naming nothing would fail later still:
/// the worktree is never prepared, and what a person sees is whichever Check
/// needed what was not installed.
///
/// Every entry is refused on its own and the walk continues, so a file with two
/// bad names is one edit.
fn preparation(
    value: &Value,
    checks: &BTreeMap<String, Check>,
    commands: &BTreeMap<String, Command>,
    out: &mut Vec<Refusal>,
) -> Vec<Preparation> {
    let Some(mut table) = Table::open("setup", value, out) else {
        return Vec::new();
    };
    // `requires` is required, because `setup:` with nothing under it says
    // nothing and `close` would report no fault for it.
    let items = table
        .required("requires", out)
        .and_then(|value| yaml::list(&table.at("requires"), value, out));
    table.close(SETUP_KEYS, out);
    let Some(items) = items else {
        return Vec::new();
    };

    let mut built: Vec<Preparation> = Vec::with_capacity(items.len());
    for (key, item) in items {
        let Some(name) = yaml::text(&key, item, out) else {
            continue;
        };
        if let Some(first_at) = built.iter().position(|had| had.name == name) {
            out.push(Refusal::new(key, Fault::RequiredTwice { first_at }));
            continue;
        }
        match commands.get(&name) {
            // A destructive Command is withheld from a Drone by
            // `fleet::spawning`, for the reason that makes this a refusal:
            // the flag means *somebody approves before this runs*, and
            // preparation runs before any Drone exists with nobody to ask.
            Some(command) if command.is_destructive() => out.push(Refusal::new(
                key,
                Fault::PreparedBySomethingDestructive { value: name },
            )),
            Some(command) => built.push(Preparation {
                name,
                run: command.run().to_string(),
            }),
            None => {
                let is_a_check = checks.contains_key(&name);
                out.push(Refusal::new(
                    key,
                    Fault::NotADeclaredCommand {
                        value: name,
                        is_a_check,
                        declared: commands.keys().cloned().collect(),
                    },
                ));
            }
        }
    }
    built
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
    // **A missing `when` and an unreadable `when` are not the same answer.**
    // The first is a Check that always runs; the second is a file that does not
    // load. So a fault inside the list refuses the Check rather than falling
    // back to the always case, which would be the parser deciding on the
    // author's behalf that a Check they meant to scope is unscoped.
    let when = match table.optional("when") {
        None => Ok(None),
        Some(value) => covers(&table.at("when"), value, out),
    };
    table.close(known, out);
    Some(Check {
        run: run?,
        when: when.ok()?,
    })
}

/// `checks.<name>.when`, as a non-empty list of readable patterns.
///
/// `Err(())` where something in the list was refused, so the caller can tell it
/// from a `when` that was simply not written — the two mean opposite things and
/// an `Option` alone cannot carry both.
fn covers(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Result<Option<Covers>, ()> {
    // An empty list is refused by `yaml::list`, which is the answer this key
    // wants: `when: []` is a Check that can never run, and a Check that can
    // never run is one to delete rather than one to write.
    let Some(items) = yaml::list(at, value, out) else {
        return Err(());
    };
    let mut patterns = Vec::with_capacity(items.len());
    let mut refused = false;
    for (key, item) in items {
        let Some(written) = yaml::text(&key, item, out) else {
            refused = true;
            continue;
        };
        match PathPattern::parse(&written) {
            Ok(pattern) => patterns.push(pattern),
            Err(why) => {
                refused = true;
                out.push(Refusal::new(
                    key,
                    Fault::NotAPathPattern {
                        value: written,
                        why,
                    },
                ));
            }
        }
    }
    match refused {
        true => Err(()),
        false => Ok(Covers::of(patterns)),
    }
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
