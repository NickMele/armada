//! `armada.yml`, in the slice M1 reads.
//!
//! **These keys, and nothing else.** `version`, `id`, `base`; `run`,
//! `expect_exit_code`, `when`, `requires` and `narrow` under `checks.<name>`;
//! `run` and `destructive` under `commands.<name>`; `setup.requires`; and the
//! three keys [`drone`] reads, the one section here that is a dial rather than
//! a registry. Every other section the concept page describes is refused:
//! permissions, secrets, ports, skills, budget, dispatch freeze, auto-merge.
//!
//! **A key nothing reads is worse than a key that is not there.** A `budget:
//! 40` nothing consumes reads as a budget that is set, and refusing it keeps
//! every deferred section additive. [`Manifest::version`] refuses no number too.
//!
//! `checks.<name>.when` is a list of `core_model::PathPattern`s checked at
//! load, so one this parser cannot read is a refusal rather than a Check that
//! quietly stops running — **absent means always**, as `expect_exit_code`'s
//! absence means zero and [`Check::expect_exit_code`] says why that key left
//! the workflow step. **Both `requires` keys name Commands this file declares
//! and share every refusal** — [`named_commands`]; `setup`'s code word is
//! *preparation*, because `armada::setup` runs nothing and means something
//! else, and one word over two meanings is a second vocabulary. The three
//! values this file produces live in [`declared`].

mod declared;

pub use declared::{Check, Command, Preparation};

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use core_model::{
    Covers, ManifestId, Narrowing, PathPattern, Prerequisite, RepoPath, ResolvedCheck, Ulid,
};
use serde_yaml_ng::Value;

mod drone;

use crate::error::{Fault, LoadError, Refusal};
use crate::live::{Cell, Patience, Reloads};
use crate::yaml::{self, Table};

/// The keys M1 reads at the top level of an `armada.yml`.
const TOP_LEVEL: &[&str] = &[
    "version",
    "id",
    "base",
    "checks",
    "commands",
    "setup",
    "drone",
    "after_merge",
];
/// The keys M1 reads inside `checks.<name>`. **`expect_exit_code` is spelled
/// here as a workflow step spells it**, for the reason `drone:` below gives
/// about its own two: one value written under two names is a vocabulary split,
/// and this one is moving from the step to here.
const CHECK_KEYS: &[&str] = &["run", "expect_exit_code", "when", "requires", "narrow"];
/// The keys M1 reads inside `checks.<name>.narrow`.
const NARROW_KEYS: &[&str] = &["run", "each", "from", "under", "except"];
/// The keys M1 reads inside `commands.<name>`.
const COMMAND_KEYS: &[&str] = &["run", "destructive"];
/// The keys M1 reads inside `setup`.
const SETUP_KEYS: &[&str] = &["requires"];
/// The keys M1 reads inside `after_merge`. **`checks` and nothing else**, so
/// the section says one thing: which of this repository's Checks are worth
/// running against a tree a merge left behind.
const AFTER_MERGE_KEYS: &[&str] = &["checks"];

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
    /// Every name in `checks`, in the order `armada.yml` wrote them.
    ///
    /// **A second view of one set, never a second set.** It is built from the
    /// same walk as `checks` and holds exactly its keys, so the two cannot
    /// disagree about what is declared — only about what order to read it in,
    /// which is the whole reason both exist.
    checks_as_written: Vec<String>,
    commands: BTreeMap<String, Command>,
    prepared_by: Vec<Preparation>,
    proved_after_a_merge: Vec<ResolvedCheck>,
    /// **Not behind the cell**, because it is not live: every workflow was
    /// resolved against it at daemon start, so a save that moves it is
    /// reported as needing a restart rather than adopted. See [`crate::live`].
    exclude_paths: Vec<RepoPath>,
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

    /// Every declared Check name, **in the order `armada.yml` writes them,
    /// which is the order they run in**.
    ///
    /// **Order is the semantics, and there is no key to state it twice with.**
    /// This is `config`'s existing rule about a workflow's `steps[]` — see
    /// `order_is_the_semantics_and_there_is_no_field_for_it` — applied a file
    /// along, and it had to be kept the moment a step could gate on every Check
    /// without naming one. Until then the workflow file was the only place a
    /// repository could sequence its gate, and the sequence was lost in this
    /// parse without anything saying so.
    ///
    /// **The sequence is worth keeping because it is the author's, not because
    /// it makes a failure arrive sooner.** Nothing about a gate stops early —
    /// `fleet::checking` cancels no Check when one fails, and writes every
    /// result into a slot sized from the declaration — so what a Drone is told,
    /// and when, is the same whatever order the four raced in. What the order
    /// decides is which four start first, and that is a scheduling question
    /// this parser has no opinion about. Losing it to a `BTreeMap` was still a
    /// regression: a repository could sequence its gate and then could not.
    ///
    /// **Not [`check_names`](Manifest::check_names)**, which is the same set
    /// sorted, for a message rather than for a run.
    pub fn checks_as_written(&self) -> &[String] {
        &self.checks_as_written
    }

    /// Every declared Check name, sorted. Handed to a refusal so the message
    /// can name what the file does declare beside what it does not — a list a
    /// person scans for a name they expected wants to be alphabetical, which is
    /// the opposite of what a list a machine runs wants.
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

    /// The Checks this repository asks to be run against the tree a merge left
    /// behind, **already resolved, in the order `after_merge.checks` names
    /// them**. Empty is the default and is opt-in on purpose — see
    /// `docs/concepts/manifest.md`, *Proving what merged*, and `#474`.
    ///
    /// **`when` and `narrow` are dropped and `requires` is refused** — each is
    /// a step's or a Drone's question and there is no step here; the refusal is
    /// [`Fault::PreparesTheRepository`], and [`after_merge`] carries why.
    /// `expect_exit_code` is the Check's own and is kept.
    pub fn proved_after_a_merge(&self) -> &[ResolvedCheck] {
        &self.proved_after_a_merge
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

    /// What a step's work in this repository stays out of, where the step
    /// does not say for itself. **`&[]` where the file declares no
    /// `drone.exclude_paths`**, which is the repository deferring to the
    /// default rather than a list invented here.
    ///
    /// **The middle of three tiers, and the only one a repository writes.**
    /// `crate::resolve` is where the order is written and it is written
    /// nowhere else — a step's list beats this, and this beats the compiled-in
    /// default that file carries. The argument for the order, and for why an
    /// inherited list replaces rather than joins the one above it, is there.
    ///
    /// **Read off the value, not through the live cell**, unlike the two keys
    /// beside it in the same section: this one is frozen at daemon start
    /// because a `ResolvedWorkflow` was built from it there, and a Job's own
    /// record carries the resolved list it ran under.
    pub fn exclude_paths(&self) -> &[RepoPath] {
        &self.exclude_paths
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

    let (drafted, checks_as_written) = match top.optional("checks") {
        Some(value) => registry(value, "checks", CHECK_KEYS, out, check_entry),
        None => (BTreeMap::new(), Vec::new()),
    };
    let (commands, _) = match top.optional("commands") {
        Some(value) => registry(value, "commands", COMMAND_KEYS, out, command_entry),
        None => (BTreeMap::new(), Vec::new()),
    };
    // Both `requires` keys are resolved after `commands`, because every entry
    // is resolved against it. **Which section comes first does not matter** —
    // `Table` reads by name, so an author never has to put `commands:` above
    // `checks:` — only that the registry is built before it is consulted.
    //
    // The order *within* `checks:` does matter, and is kept: see
    // [`Manifest::checks_as_written`]. The two are different questions and this
    // sentence used to answer both, which stopped being true the day a step
    // could gate on every Check without naming one.
    //
    // The Commands registry has no such order to keep. What runs before a Check
    // is sequenced by that Check's own `requires` list, and preparation by
    // `setup.requires` — both of which are arrays, where order is already the
    // semantics.
    let declares: BTreeSet<String> = drafted.keys().cloned().collect();
    let checks = required_by(drafted, &declares, &commands, out);
    let prepared_by = match top.optional("setup") {
        Some(value) => preparation(value, &declares, &commands, out),
        None => Vec::new(),
    };
    let drone = match top.optional("drone") {
        Some(value) => drone::read(value, out),
        None => drone::Drone::unstated(),
    };
    // After `checks` for `setup.requires`' reason, one registry along: every
    // entry resolves against it, and a file's order is never something an
    // author has to think about.
    let proved_after_a_merge = match top.optional("after_merge") {
        Some(value) => after_merge(value, &checks, &commands, out),
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
        checks_as_written,
        commands,
        prepared_by,
        proved_after_a_merge,
        exclude_paths: drone.exclude_paths,
        live: Cell::holding(drone.patience),
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

/// `after_merge:`, the Checks this repository asks to be run against the tree a
/// merge left behind.
///
/// **The one section here that spends a person's machine rather than a Job's.**
/// A Job's Checks run in a worktree Armada cut; these run in the repository
/// somebody is working in, minutes after a merge nobody is waiting on. That is
/// why it is opt-in, why it names Checks one at a time rather than meaning *all
/// of them*, and why a Check with prerequisites is refused. `#474`, and
/// `docs/concepts/manifest.md` — *Proving what merged*.
///
/// **`checks` is required, for `setup.requires`' reason.** Every entry is
/// refused on its own and the walk continues, so two bad names are one edit.
fn after_merge(
    value: &Value,
    checks: &BTreeMap<String, Check>,
    commands: &BTreeMap<String, Command>,
    out: &mut Vec<Refusal>,
) -> Vec<ResolvedCheck> {
    let Some(mut table) = Table::open("after_merge", value, out) else {
        return Vec::new();
    };
    let items = table
        .required("checks", out)
        .and_then(|value| yaml::list(&table.at("checks"), value, out));
    table.close(AFTER_MERGE_KEYS, out);
    let Some(items) = items else {
        return Vec::new();
    };
    let mut built: Vec<ResolvedCheck> = Vec::new();
    for (key, name) in texts(items, out) {
        if let Some(first_at) = built.iter().position(|had| had.label() == name) {
            out.push(Refusal::new(key, Fault::RequiredTwice { first_at }));
            continue;
        }
        match checks.get(&name) {
            Some(check) if !check.requires().is_empty() => out.push(Refusal::new(
                key,
                Fault::PreparesTheRepository {
                    requires: check
                        .requires()
                        .iter()
                        .map(|needed| needed.name().to_string())
                        .collect::<Vec<String>>()
                        .join(", "),
                    value: name,
                },
            )),
            // `when` dropped: it is a step's question — which of a change's
            // paths this Check covers — and after a merge there is no step and
            // no change to ask it of. `narrow` dropped for the same shape of
            // reason one tier along: it is a Drone's question about its own
            // work, and what merged is the whole tree.
            //
            // **`expect_exit_code` is kept, and used to be hard zero here.**
            // It was a step's question while it was a step's key. It is the
            // Check's own now, so a Check that legitimately exits non-zero
            // says so once and every reader agrees — including this one, which
            // would otherwise report the same Check as failing after every
            // merge for the reason it was declared with.
            Some(check) => built.push(ResolvedCheck::ManifestCheck {
                name,
                run: check.run().to_string(),
                expect_exit_code: check.expect_exit_code(),
                when: None,
                requires: Vec::new(),
                narrow: None,
            }),
            None => out.push(Refusal::new(
                key,
                Fault::NotADeclaredCheck {
                    is_a_command: commands.contains_key(&name),
                    value: name,
                    declared: checks.keys().cloned().collect(),
                },
            )),
        }
    }
    built
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
                expect_exit_code: draft.expect_exit_code,
                when: draft.when,
                requires,
                narrow: draft.narrow,
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
/// The entries by name, **and the order the file wrote them in**.
///
/// Two returns rather than one ordered map, because the two are asked
/// different questions and only one caller asks the second. A name is looked up
/// far more often than a registry is walked, and a message listing what is
/// declared reads better sorted — so the map stays a `BTreeMap` and the order
/// travels beside it.
///
/// **Order is the semantics here, exactly as it is for `steps[]`.** `config`'s
/// own `order_is_the_semantics_and_there_is_no_field_for_it` states the rule on
/// a workflow's steps and it holds a file along: `checks:` is what a repository
/// sequences its gate with, and there is no `order` key to state it twice with.
/// It went missing between the parse and the caller until
/// `every_manifest_check` needed it — see [`Manifest::checks_as_written`].
fn registry<T>(
    value: &Value,
    key: &'static str,
    known: &'static [&'static str],
    out: &mut Vec<Refusal>,
    entry: fn(&str, &Value, &'static [&'static str], &mut Vec<Refusal>) -> Option<T>,
) -> (BTreeMap<String, T>, Vec<String>) {
    let mut built = BTreeMap::new();
    let mut written = Vec::new();
    let Some(table) = Table::open(key, value, out) else {
        return (built, written);
    };
    for (name, item) in table.into_entries() {
        let at = format!("{key}.{name}");
        if let Some(parsed) = entry(&at, item, known, out) {
            // First appearance, so the two agree on their contents whatever a
            // duplicated key in the document does to the map.
            if !built.contains_key(&name) {
                written.push(name.clone());
            }
            built.insert(name, parsed);
        }
    }
    (built, written)
}

/// A Check whose `requires` entries have been read and not yet resolved.
///
/// **The Commands registry does not exist yet** when a Check is parsed — a file
/// may declare `commands` after `checks`, and an author should never have to
/// think about which. So each name is taken here with the key that locates it,
/// and [`required_by`] resolves them once both registries are built.
struct DraftCheck {
    run: String,
    expect_exit_code: i64,
    when: Option<Covers>,
    /// `None` where the file declares no `requires`, which is a Check nothing
    /// runs before. Distinct from an empty list, which [`yaml::list`] refuses
    /// outright: `requires: []` is a key to delete rather than a list to read.
    requires: Option<Vec<(String, String)>>,
    narrow: Option<Narrowing>,
}

/// `checks.<name>.narrow`, the second question a Check answers about paths.
///
/// **`run` and `each` are both required, and `run` is a whole command line.**
/// The first Check narrowed here is `cargo nextest run --workspace`, and what
/// has to go for `-p` to mean anything is `--workspace` — so no rule about
/// appending arguments to the declared `run` could have produced the narrowed
/// one. The file writes both.
///
/// **`under` is where the repository declares its own layout**, and it is the
/// whole of the derivation from a path to an argument. `crates` turns
/// `crates/ipc/src/lib.rs` into `ipc`; a `pnpm` repository writes something
/// else. Nothing downstream knows either fact, which is the point — a runner
/// that knew Cargo's layout would be wrong about the first repository that did
/// not use it. `except` then drops values the whole run already excludes, which
/// no `from` pattern could: the dialect refuses a leading `!` by name.
///
/// A refused value leaves the Check with no narrowing, and the refusal is
/// already in `out` — no Manifest carrying one loads at all, which is
/// `requires`' reasoning and not `when`'s: running whole is what a Check
/// without this key does anyway, so there is no silent narrowing to fall into.
fn narrowing(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<Narrowing> {
    let mut table = Table::open(at, value, out)?;
    let run = table
        .required("run", out)
        .and_then(|value| yaml::text(&table.at("run"), value, out));
    let each_key = table.at("each");
    let each = table
        .required("each", out)
        .and_then(|value| yaml::text(&each_key, value, out))
        .and_then(|written| match written.contains("{}") {
            true => Some(written),
            false => {
                out.push(Refusal::new(&each_key, Fault::NothingToSubstitute));
                None
            }
        });
    // Not `when`, and it cannot be folded into it: `format` covers every path
    // and narrows only to the Rust ones. Absent means every changed path feeds
    // it, which is the reading `when` gives absence one key up.
    let from = match table.optional("from") {
        None => Ok(None),
        Some(value) => covers(&table.at("from"), value, out),
    };
    let under = table
        .optional("under")
        .and_then(|value| yaml::text(&table.at("under"), value, out));
    // Values, not paths: `from` filters what feeds the derivation and this
    // filters what it derived to. `except: []` is refused by `yaml::list` for
    // `when`'s reason — a list with nothing in it is a key to delete.
    let except = table
        .optional("except")
        .and_then(|value| yaml::list(&table.at("except"), value, out))
        .map(|items| texts(items, out))
        .unwrap_or_default()
        .into_iter()
        .map(|(_, value)| value)
        .collect();
    table.close(NARROW_KEYS, out);
    Some(Narrowing::declared(run?, each?, from.ok()?, under, except))
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
    // **Absent is zero, and a malformed one is a refusal rather than zero.** A
    // file writing `expect_exit_code: "one"` meant to declare a Check that
    // legitimately fails, and quietly giving it zero would make that Check
    // unpassable while reading as declared. Same split as `retry_limit` one
    // file along, for the same reason.
    let code_key = table.at("expect_exit_code");
    let expect_exit_code = match table.optional("expect_exit_code") {
        None => Some(0),
        Some(value) => yaml::integer(&code_key, value, out),
    };
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
    let narrow = table
        .optional("narrow")
        .and_then(|value| narrowing(&table.at("narrow"), value, out));
    table.close(known, out);
    Some(DraftCheck {
        run: run?,
        expect_exit_code: expect_exit_code?,
        when: when.ok()?,
        requires,
        narrow,
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
