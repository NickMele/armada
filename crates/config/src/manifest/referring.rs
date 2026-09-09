//! The sections that name what another section declared.
//!
//! **A seam, not a line count.** `setup.requires`, `after_merge.checks` and
//! `checks.<name>.requires` do not define anything — each is a list of names
//! resolved against the Checks and Commands registries the same file declares
//! somewhere else, and each fails the Manifest rather than the Job when a name
//! resolves to nothing. That is one job, done four ways, and it was in the
//! middle of the file that reads every other section.
//!
//! **Resolved in a second pass, so a file's order is never something an author
//! has to think about.** A Check may name a Command declared below it; both
//! registries are read independently and joined here.
//!
//! # Why a miss is refused at load
//!
//! A name that resolves to nothing would otherwise be found at a gate, by a
//! Drone, with a worktree already checked out and a retry budget already being
//! spent — and what a person would see is not the missing name but whichever
//! Check needed what was never installed. `crates/config/tests/shipped.rs` asks
//! the same question of a workflow step, and it exists because one such step
//! passed every test and failed at dispatch.
//!
//! Every entry is refused on its own and the walk continues, so a file with two
//! bad names is one edit.

use std::collections::{BTreeMap, BTreeSet};

use core_model::{Prerequisite, ResolvedCheck};
use serde_yaml_ng::Value;

use super::declared::{Check, Command, Preparation};
use super::{texts, DraftCheck, AFTER_MERGE_KEYS, SETUP_KEYS};
use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

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
pub(super) fn preparation(
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
pub(super) fn after_merge(
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
pub(super) fn required_by(
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
