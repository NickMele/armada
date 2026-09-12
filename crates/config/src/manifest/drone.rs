//! `drone:`, what a repository says about how a Drone works here.
//!
//! **The one section of `armada.yml` that is a dial rather than a registry**:
//! every other section in [`crate::manifest`] is a map of named things resolved
//! against another map, and this one is four values with four separate
//! fallbacks. `#414` added the first two.
//!
//! **Named for what it configures, not for who enforces it.** Fleet enforces
//! all four and there is no `fleet:` section —
//! `docs/contracts/configuration.md` gives the rule.
//!
//! **The cap says `per_job` because this section does not.** A Drone belongs to
//! one step, so a four-step Job is four Drones, and a key read as per-Drone
//! would be four times the ceiling anybody set. The other three keys are the
//! Drone's; this one is the Job's.

use core_model::RepoPath;
use serde_yaml_ng::Value;

use crate::error::{Fault, Refusal};
use crate::live::Dials;
use crate::yaml::{self, Table};

/// The keys M1 reads inside `drone`. **The first three are spelled as a
/// workflow step spells them** — `crate::workflow::step`'s `STEP_KEYS` and
/// `crate::scope`'s `SCOPE_KEYS` carry the same three words, because they are
/// the same three values one tier up. Neither cap has a step tier: a step is
/// not what a person approved and is not what a cap is set against.
const DRONE_KEYS: &[&str] = &[
    "quiet_after_seconds",
    "poke_limit",
    "cost_cap_micros_per_job",
    "turn_cap_per_job",
    "exclude_paths",
];

/// What the section came to. **Two halves with different lifetimes**: the
/// dials are live and `crate::live` moves them under a running Fleet, and the
/// fences were resolved into every workflow at boot and cannot move without a
/// restart.
pub(super) struct Drone {
    pub(super) dials: Dials,
    pub(super) exclude_paths: Vec<RepoPath>,
}

impl Drone {
    /// A repository that wrote no `drone:` at all: four absences, each
    /// deferring to the tier above it on its own.
    pub(super) fn unstated() -> Drone {
        Drone {
            dials: Dials::default(),
            exclude_paths: Vec::new(),
        }
    }
}

/// Read the section.
///
/// **Every key optional, and the section refused when it holds none.** A
/// `drone:` with nothing under it says nothing, exactly as `setup:` with
/// nothing under it does, and [`Table::close`] reports no fault for an empty
/// table — so the emptiness is asked about here or not at all.
///
/// **The zeros disagree, and each key is right about its own.** This is
/// `crate::workflow::step`'s split arriving one file up: a
/// `quiet_after_seconds: 0` pokes a Drone on its first turn and escalates it on
/// its third, which nobody means, and a `poke_limit: 0` says the first silence
/// past the threshold escalates, which somebody might. Both readings are the
/// step tier's and are unchanged by being written here — one value written in
/// two places that read it differently is the defect this chain avoids.
///
/// **Both caps read `0` as a sentence too**, so [`yaml::counted`] and not
/// [`yaml::positive`]: each holds a repository's Jobs, not the Fleet.
///
/// **An absent list is never an empty one**, and [`yaml::list`] refuses the
/// empty one for every list in this file. So absence has exactly one reading:
/// this repository defers to the tier above it.
///
/// A refused value reads as absent from here, which is safe for the reason the
/// workflow parser gives: the refusal is already in `out`, and a file with any
/// refusal in it does not load at all.
pub(super) fn read(value: &Value, out: &mut Vec<Refusal>) -> Drone {
    let Some(mut table) = Table::open("drone", value, out) else {
        return Drone::unstated();
    };
    if table.is_empty() {
        out.push(Refusal::new("drone", Fault::Empty));
        return Drone::unstated();
    }
    let quiet_key = table.at("quiet_after_seconds");
    let quiet_after_seconds = table
        .optional("quiet_after_seconds")
        .and_then(|value| yaml::positive(&quiet_key, value, out));
    let poke_key = table.at("poke_limit");
    let poke_limit = table
        .optional("poke_limit")
        .and_then(|value| yaml::counted(&poke_key, value, out));
    // `counted` is `u32`, which caps this key at $4,294.96 — see
    // `crate::live::Dials`, where the width is argued.
    let cap_key = table.at("cost_cap_micros_per_job");
    let cost_cap_micros = table
        .optional("cost_cap_micros_per_job")
        .and_then(|value| yaml::counted(&cap_key, value, out));
    // `counted` is `u32` here too, and the ceiling it imposes is not a
    // constraint on anything: a per-Job turn cap of four billion is a Job
    // nothing stops.
    let turns_key = table.at("turn_cap_per_job");
    let turn_cap = table
        .optional("turn_cap_per_job")
        .and_then(|value| yaml::counted(&turns_key, value, out));
    // Read exactly as `evidence_scope.exclude_paths` is read one tier down,
    // through the same helper and with the same absence of validation: two
    // readings of one word is the split this key's spelling exists to avoid.
    let excluded_key = table.at("exclude_paths");
    let exclude_paths = table
        .optional("exclude_paths")
        .and_then(|value| yaml::list(&excluded_key, value, out))
        .map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| yaml::text(at, item, out))
                .map(RepoPath::new)
                .collect::<Vec<RepoPath>>()
        })
        .unwrap_or_default();
    table.close(DRONE_KEYS, out);
    Drone {
        dials: Dials {
            quiet_after_seconds,
            poke_limit,
            cost_cap_micros,
            turn_cap,
        },
        exclude_paths,
    }
}
