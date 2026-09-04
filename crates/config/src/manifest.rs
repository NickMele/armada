//! `armada.yml`, in the slice M1 reads.
//!
//! **These keys, and nothing else.** `version`, `id`, `base`; `run`, `when`
//! and `requires` under `checks.<name>`; `run` and `destructive` under
//! `commands.<name>`; `setup.requires`; and `quiet_after_seconds` and
//! `poke_limit` under `drone`, `#414`'s — the first section here that is a dial
//! rather than a registry, spelled as a step spells it and named for the reason
//! `docs/contracts/configuration.md` gives. `fleet::Liveness::at` orders its
//! tiers and nothing else does. Every other section the concept page describes
//! is refused: permissions, secrets, ports, skills, budget, dispatch freeze,
//! auto-merge.
//!
//! **A key nothing reads is worse than a key that is not there.** A file
//! carrying `budget: 40` that no code consumes reads to its author as a budget
//! that is set. Refusing it keeps every deferred section additive rather than a
//! migration. [`Manifest::version`] refuses no number for the same reason.
//!
//! `checks.<name>.when` is a list of `core_model::PathPattern`s checked at
//! load, so one this parser cannot read is a refusal beside every other in the
//! file rather than a Check that quietly stops running. **Absent means always.**
//!
//! **Both `requires` keys name Commands this file declares, resolved at load,
//! and share every refusal** — see [`named_commands`]. `setup`'s code word is
//! *preparation*, because `armada::setup` runs nothing and means something
//! else; one word over two meanings is a second vocabulary.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use core_model::{Covers, ManifestId, PathPattern, Prerequisite, Ulid};
use serde_yaml_ng::Value;

use crate::error::{Fault, LoadError, Refusal};
use crate::live::{Cell, Patience, Reloads};
use crate::yaml::{self, Table};

/// The keys M1 reads at the top level of an `armada.yml`.
const TOP_LEVEL: &[&str] = &[
    "version", "id", "base", "checks", "commands", "setup", "drone",
];
/// The keys M1 reads inside `checks.<name>`.
const CHECK_KEYS: &[&str] = &["run", "when", "requires"];
/// The keys M1 reads inside `commands.<name>`.
const COMMAND_KEYS: &[&str] = &["run", "destructive"];
/// The keys M1 reads inside `setup`.
const SETUP_KEYS: &[&str] = &["requires"];
/// The keys M1 reads inside `drone`. **Spelled as a workflow step spells
/// them** — `crates/config/src/workflow.rs`'s `STEP_KEYS` carries the same two
/// words, because they are the same two values one tier up.
const DRONE_KEYS: &[&str] = &["quiet_after_seconds", "poke_limit"];

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
    requires: Vec<Prerequisite>,
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

    /// The Commands that run before this Check, **in the order the file names
    /// them**, already resolved to their command lines.
    ///
    /// Empty where the file declares no `requires`, which is every Manifest
    /// written before the key existed. `requires: []` is refused rather than
    /// read as empty, for `when`'s reason — a list with nothing in it is a key
    /// to delete.
    pub fn requires(&self) -> &[Prerequisite] {
        &self.requires
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
    /// The two `lifetime = "Live"` keys, behind a cell every clone shares.
    /// See [`crate::live`] for why these and not the whole file.
    live: Cell,
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

    /// Read it, and hand back the one handle that may re-read it.
    ///
    /// **The only way to get a [`Reloads`].** A Manifest from
    /// [`load`](Manifest::load) or [`parse`](Manifest::parse) is one nothing
    /// can move, which is what keeps the writer with whoever opened the file
    /// rather than with everyone holding the value.
    pub fn reloadable(path: &Path) -> Result<(Manifest, Reloads), LoadError> {
        let manifest = Manifest::load(path)?;
        let reloads = Reloads::of(path.to_path_buf(), manifest.live.clone(), &manifest);
        Ok((manifest, reloads))
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

    /// How long a Drone working in this repository may say nothing before
    /// Fleet pokes it, in seconds. **`None` where the file declares no
    /// `drone.quiet_after_seconds`**, which is the repository deferring to what
    /// Fleet is running with rather than a number invented here.
    ///
    /// **The middle of three tiers and never the answer on its own.** A step
    /// that names its own beats this; this beats the composition root's
    /// constant. `fleet::Liveness::at` is where the order is written and it is
    /// written nowhere else — a default spelled here would be a second place
    /// that number lives, and the two would drift.
    ///
    /// **Zero is refused where it is written**, by [`yaml::positive`]: a
    /// repository whose Drones are quiet the instant they are spawned is a
    /// sentence nobody means. [`poke_limit`](Manifest::poke_limit) disagrees
    /// about zero, and is right to.
    /// **Read through the live cell, so an `armada.yml` saved under a running
    /// Fleet is answered here.** `#430`. The read is one uncontended lock over
    /// two integers and happens at a step boundary, never inside a step.
    pub fn quiet_after_seconds(&self) -> Option<u32> {
        self.live.read().quiet_after_seconds
    }

    /// How many nudges a quiet Drone gets in this repository before the Job
    /// escalates as `stalled`. **`None` where the file declares no
    /// `drone.poke_limit`**, and resolved independently of
    /// [`quiet_after_seconds`](Manifest::quiet_after_seconds): a repository
    /// that wants to be waited on longer does not thereby want to be asked
    /// more often, which is why `crates/config/settings.toml` holds two rows
    /// rather than one pair.
    ///
    /// **`0` is a value and not an absence**, by [`yaml::counted`]. It says a
    /// Drone in this repository gets no nudge at all and the first silence past
    /// the threshold escalates — legitimate where a poke costs a model run and
    /// buys nothing.
    /// Read through the live cell, for
    /// [`quiet_after_seconds`](Manifest::quiet_after_seconds)'s reason.
    pub fn poke_limit(&self) -> Option<u32> {
        self.live.read().poke_limit
    }

    /// Both live keys at once, for the one caller that adopts them together.
    pub(crate) fn patience(&self) -> Patience {
        self.live.read()
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

    let drafted = match top.optional("checks") {
        Some(value) => registry(value, "checks", CHECK_KEYS, out, check_entry),
        None => BTreeMap::new(),
    };
    let commands = match top.optional("commands") {
        Some(value) => registry(value, "commands", COMMAND_KEYS, out, command_entry),
        None => BTreeMap::new(),
    };
    // Both `requires` keys are resolved after `commands`, because every entry
    // is resolved against it. Nothing about the order of the file matters —
    // `Table` reads by name — only that the registry is built before it is
    // consulted.
    let declares: BTreeSet<String> = drafted.keys().cloned().collect();
    let checks = required_by(drafted, &declares, &commands, out);
    let prepared_by = match top.optional("setup") {
        Some(value) => preparation(value, &declares, &commands, out),
        None => Vec::new(),
    };
    let (quiet_after_seconds, poke_limit) = match top.optional("drone") {
        Some(value) => patience(value, out),
        None => (None, None),
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
        live: Cell::holding(Patience {
            quiet_after_seconds,
            poke_limit,
        }),
    })
}

/// `drone:`, the repository's own patience with a Drone that goes quiet.
///
/// **Both keys optional and the section refused when it holds neither.** A
/// `drone:` with nothing under it says nothing, exactly as `setup:` with
/// nothing under it does, and [`Table::close`] reports no fault for an empty
/// table — so the emptiness has to be asked about here or it is not asked about
/// at all.
///
/// **The two zeros disagree, and each key is right about its own.** This is
/// `crates/config/src/workflow.rs`'s split arriving one file up: a
/// `quiet_after_seconds: 0` pokes a Drone on its first turn and escalates it on
/// its third, which nobody means, and a `poke_limit: 0` says the first silence
/// past the threshold escalates, which somebody might. Both readings are the
/// step tier's and are unchanged by being written here — one value written in
/// two places that read it differently is the defect this whole chain exists to
/// avoid.
///
/// A refused value reads as absent from here, which is safe for the reason the
/// workflow parser gives: the refusal is already in `out`, and a file with any
/// refusal in it does not load at all.
fn patience(value: &Value, out: &mut Vec<Refusal>) -> (Option<u32>, Option<u32>) {
    let Some(mut table) = Table::open("drone", value, out) else {
        return (None, None);
    };
    if table.is_empty() {
        out.push(Refusal::new("drone", Fault::Empty));
        return (None, None);
    }
    let quiet_key = table.at("quiet_after_seconds");
    let quiet_after_seconds = table
        .optional("quiet_after_seconds")
        .and_then(|value| yaml::positive(&quiet_key, value, out));
    let poke_key = table.at("poke_limit");
    let poke_limit = table
        .optional("poke_limit")
        .and_then(|value| yaml::counted(&poke_key, value, out));
    table.close(DRONE_KEYS, out);
    (quiet_after_seconds, poke_limit)
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
    declares: &BTreeSet<String>,
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
    named_commands(texts(items, out), declares, commands, out)
        .into_iter()
        .map(|(name, run)| Preparation { name, run })
        .collect()
}

/// `checks.<name>.requires`, resolved for every Check in the file.
///
/// **A second pass, because a Check may name a Command declared below it.** The
/// two registries are read independently and joined here, so a file's order is
/// never something an author has to think about — the same reason
/// `setup.requires` is resolved after `commands` rather than during it.
///
/// **A name that resolves to nothing fails the Manifest, not the Job.** That is
/// the whole point of resolving here: a Check requiring a Command nobody
/// declared would otherwise be found at a gate, by a Drone, with a worktree
/// already checked out and a retry budget already being spent.
fn required_by(
    drafted: BTreeMap<String, DraftCheck>,
    declares: &BTreeSet<String>,
    commands: &BTreeMap<String, Command>,
    out: &mut Vec<Refusal>,
) -> BTreeMap<String, Check> {
    let mut built = BTreeMap::new();
    for (name, draft) in drafted {
        let requires = draft
            .requires
            .map(|items| named_commands(items, declares, commands, out))
            .unwrap_or_default()
            .into_iter()
            .map(|(name, run)| Prerequisite::resolved(name, run))
            .collect();
        built.insert(
            name,
            Check {
                run: draft.run,
                when: draft.when,
                requires,
            },
        );
    }
    built
}

/// A list of Command names, resolved against the Commands registry.
///
/// **One function for `setup.requires` and for `checks.<name>.requires`**,
/// because the two ask the same question of the same registry and every refusal
/// they can raise is the same refusal. Two copies would drift, and the one that
/// drifted would be the one nobody ran.
///
/// Every entry is refused on its own and the walk continues, so a file with two
/// bad names is one edit. The pairs come back in the order the file wrote them:
/// `[migrate, seed]` is a sequence somebody wrote, and sorting it would run the
/// second before what it depends on.
fn named_commands(
    items: Vec<(String, String)>,
    declares: &BTreeSet<String>,
    commands: &BTreeMap<String, Command>,
    out: &mut Vec<Refusal>,
) -> Vec<(String, String)> {
    let mut built: Vec<(String, String)> = Vec::with_capacity(items.len());
    for (key, name) in items {
        if let Some(first_at) = built.iter().position(|(had, _)| *had == name) {
            out.push(Refusal::new(key, Fault::RequiredTwice { first_at }));
            continue;
        }
        match commands.get(&name) {
            // A destructive Command is withheld from a Drone by
            // `fleet::spawning`, for the reason that makes this a refusal: the
            // flag means *somebody approves before this runs*, and neither
            // reader of this key has anybody to ask — preparation runs before
            // any Drone exists, and a prerequisite runs inside a gate the Drone
            // is already waiting on.
            Some(command) if command.is_destructive() => out.push(Refusal::new(
                key,
                Fault::RequiresSomethingDestructive { value: name },
            )),
            Some(command) => built.push((name, command.run().to_string())),
            None => out.push(Refusal::new(
                key,
                Fault::NotADeclaredCommand {
                    is_a_check: declares.contains(&name),
                    value: name,
                    declared: commands.keys().cloned().collect(),
                },
            )),
        }
    }
    built
}

/// Every item of a list read as a string, keeping the key that names its
/// position. An item that is not a string is refused and dropped, so one pass
/// still reports the rest of the list.
///
/// Separate from [`named_commands`] because a Check's `requires` is read while
/// the Checks registry is being walked and resolved once the Commands registry
/// exists, and the borrow of the document does not survive between the two.
fn texts(items: Vec<(String, &Value)>, out: &mut Vec<Refusal>) -> Vec<(String, String)> {
    items
        .into_iter()
        .filter_map(|(key, item)| yaml::text(&key, item, out).map(|name| (key, name)))
        .collect()
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

/// A Check whose `requires` entries have been read and not yet resolved.
///
/// **The Commands registry does not exist yet** when a Check is parsed — a file
/// may declare `commands` after `checks`, and an author should never have to
/// think about which. So each name is taken here with the key that locates it,
/// and [`required_by`] resolves them once both registries are built.
struct DraftCheck {
    run: String,
    when: Option<Covers>,
    /// `None` where the file declares no `requires`, which is a Check nothing
    /// runs before. Distinct from an empty list, which [`yaml::list`] refuses
    /// outright: `requires: []` is a key to delete rather than a list to read.
    requires: Option<Vec<(String, String)>>,
}

fn check_entry(
    at: &str,
    value: &Value,
    known: &'static [&'static str],
    out: &mut Vec<Refusal>,
) -> Option<DraftCheck> {
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
    // **A `requires` the parser could not read leaves the Check with none, and
    // the refusal is already in `out`.** No Manifest carrying one loads at all,
    // so there is no path on which a Check with a silently emptied `requires`
    // reaches a gate — the distinction `when` has to draw does not arise here.
    let requires = table
        .optional("requires")
        .map(|value| yaml::list(&table.at("requires"), value, out).unwrap_or_default())
        .map(|items| texts(items, out));
    table.close(known, out);
    Some(DraftCheck {
        run: run?,
        when: when.ok()?,
        requires,
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
