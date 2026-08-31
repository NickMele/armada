//! The models a definition may name, handed in rather than known here.
//!
//! # Why this is a parameter and not a constant
//!
//! A model alias is a vendor's spelling, and gate rule six keeps those inside
//! `adapters`. `core_model::ModelName` says so from the other end: it refuses a
//! blank and nothing else, because "naming a closed set here would put a
//! vendor's vocabulary under every crate". So a parser refusing an unknown
//! model against a list it invented would be inventing the vocabulary the
//! boundary exists to contain.
//!
//! What it can do is compare. `armada::model_choices` resolves the roster off
//! the adapter and passes it down. Holding a [`Roster`] is not knowledge of
//! which models exist; it is a caller having said which this machine offers.
//!
//! # Why the refusal is at load and not at spawn
//!
//! `Model::named` refuses only a blank, so a typo travels: it parses, it is
//! frozen onto the Job, the Job is approved, and it fails at the spawn, where
//! `fleet::spawning` turns a `SpawnConfigRefused` into an escalation — costing
//! a worktree and every step before the bad one, which is the whole argument.
//!
//! **What the Job is reported as is no longer part of that argument.** It was
//! `Interrupted` until 2026-08-31 — a process that had never started — and is
//! `not_configurable` now, which points back at this file.

use core_model::ModelName;

use crate::error::{Fault, Refusal};

/// What the caller says this machine can run a Drone as.
///
/// **Empty refuses everything**, and that is deliberate rather than an
/// oversight: a roster with nothing in it is a machine that can spawn no Drone
/// at all, and reading it as "accept anything" would turn the one caller who
/// failed to resolve a roster into the one caller with no check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roster(Vec<ModelName>);

impl Roster {
    /// The roster a caller resolved. Blanks are dropped rather than refused —
    /// a blank is not a model, so it is not one this offers, and the caller
    /// finding that out here would be this type reporting on a list it was
    /// handed rather than on the file it exists to check.
    pub fn of(named: impl IntoIterator<Item = impl AsRef<str>>) -> Roster {
        let mut models: Vec<ModelName> = Vec::new();
        for name in named {
            let Ok(model) = ModelName::new(name.as_ref()) else {
                continue;
            };
            if !models.contains(&model) {
                models.push(model);
            }
        }
        Roster(models)
    }

    /// A roster with nothing in it, which refuses every model a definition
    /// could name.
    ///
    /// **Named rather than a `Default`**, so a call site says out loud that it
    /// offers nothing instead of arriving at it by leaving something out. What
    /// it is for is a definition that names no model at all, where the roster
    /// is never consulted and a list would be a fixture stating something it is
    /// not about.
    pub fn offering_nothing() -> Roster {
        Roster(Vec::new())
    }

    /// Whether a definition may name this.
    pub fn offers(&self, model: &ModelName) -> bool {
        self.0.contains(model)
    }

    /// What it does offer, in the order the caller gave them — which is the
    /// order a picker shows, so a refusal reads back against the list the
    /// person choosing a model already saw.
    pub fn names(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|model| model.as_str().to_string())
            .collect()
    }
}

/// The model named at `at`, refused unless the roster offers it.
///
/// **One reader for both keys.** A step's `model` names what a Drone is spawned
/// as and a `judge_checks[].model` names what a Judge call is read by; they
/// default apart in `adapters` and neither falls back to the other. The legal
/// set is shared because it is one vocabulary — the aliases the vendor's binary
/// takes on `--model`, of which `HeadlessAgent::judge_model` is an entry rather
/// than a constant of its own. Two readers would be two spellings of one rule,
/// and the second is the one that goes stale. The fault names the key it was
/// given, so a reader is pointed at the line they wrote.
///
/// What differs is what a typo costs. A step's buys a worktree and every step
/// before the bad one; a check's buys the whole step, because the Drone has run
/// and the gate that was to rule on the work cannot be called.
///
/// **A blank never reaches here.** `yaml::text` refuses an empty string at the
/// key first, so `model: ""` is [`Fault::Empty`] — a different mistake with a
/// different fix. That refusal stays where it is rather than being restated
/// here; a second would report one blank twice under two names.
pub(crate) fn offered(
    at: &str,
    named: String,
    roster: &Roster,
    out: &mut Vec<Refusal>,
) -> Option<ModelName> {
    let model = ModelName::new(&named).ok()?;
    if roster.offers(&model) {
        return Some(model);
    }
    out.push(Refusal::new(
        at,
        Fault::NoSuchModel {
            value: named,
            roster: roster.names(),
        },
    ));
    None
}
