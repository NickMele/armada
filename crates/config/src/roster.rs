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
//! frozen onto the Job, the Job is approved, and it fails at the spawn — where
//! `fleet::spawning` turns a `SpawnConfigRefused` into an interrupt. That costs
//! a worktree and every step that ran before the bad one, and reports the Job
//! as `Interrupted`, which names the wrong cause.

use core_model::ModelName;

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
