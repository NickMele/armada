//! What Fleet holds, as a proposal may name it: the workflows, the Manifests
//! and the models.
//!
//! # Three shapes, because a composer had none
//!
//! Bridge's create form offered a text field for `workflow_id` and another for
//! `owner_manifest_id`, and the comment above it said why: nothing served
//! either, so the form fell back to whatever ids were already on the Board and
//! a typed one otherwise. A pasted id is an id nothing checked — a proposal
//! naming a workflow that does not exist was accepted, stored, and shown on the
//! board claiming a workflow Fleet had never heard of.
//!
//! Fleet now refuses that at creation. These three exist so a person is not
//! guessing at a value the other end will refuse: a picker offers what Fleet
//! holds, and what Fleet holds is what it will accept.
//!
//! # None of them carries a name a Manifest does not have
//!
//! [`ManifestSummary`] has no `name`, because `armada.yml` has no key for one —
//! `version`, `id`, `checks` and `commands` are the whole schema and every
//! other key is refused. It carries the **repository** it was read from
//! instead, which is a fact rather than an invention. What a Manifest should be
//! called, and where that name would come from, is a schema decision and is
//! reported rather than decided here.

use serde::{Deserialize, Serialize};

use crate::checks::DeclaredCheck;
use crate::ids::{ManifestId, StepId, WorkflowId};

/// One step of a workflow, as the definition declares it.
///
/// **This is the one place a step's Checks are read from**, and `get_job`
/// answers from the same values rather than a second copy on the Job. A
/// declaration stored twice is a registry that drifts; the Job's frozen rows
/// carry where a step *got to*, which is a different fact with a different
/// authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_id: StepId,
    /// The Checks the step declares, in the order it declares them.
    ///
    /// **Empty is the sentence "this step declares no check."** A WorkflowDef
    /// may write `mechanical_checks: []` or leave the key out and the two mean
    /// the same thing, so the wire spells it one way — always a list, never
    /// absent — and a reader never has to infer an ungated step from a gap.
    pub checks: Vec<DeclaredCheck>,
}

/// One workflow Fleet holds, as a picker offers it.
///
/// The steps are their ids and their Checks rather than the whole definition: a
/// composer needs the count and the names to say "4 steps, ending at close",
/// and the labels, the gates and the evidence types are still `get_job`'s
/// business.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowSummary {
    /// What a proposal's `workflow_id` must name. **The only value Fleet
    /// accepts** — anything else is refused at creation.
    pub id: WorkflowId,
    /// What a person reads. Distinct from the id so a rename does not dangle
    /// the Jobs that ran under the old word.
    pub name: String,
    pub version: u32,
    /// The steps, in order. Order is the semantics; there is no ordinal field.
    pub steps: Vec<WorkflowStep>,
    /// The `armada.yml` this workflow's Checks resolved against. Holding a
    /// resolved workflow means every Check its steps name was declared there.
    pub manifest_id: ManifestId,
}

/// One Manifest Fleet holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestSummary {
    /// What a proposal's `owner_manifest_id` must name.
    pub id: ManifestId,
    /// The repository the Manifest was read from — its directory's own name.
    ///
    /// **Not a name the Manifest declares**, because it declares none. This is
    /// the most useful true thing available: a person reading a Job wants to
    /// know which project it runs against, and a ULID does not say.
    pub repository: String,
    /// The absolute path of the `armada.yml`, for the case where two
    /// repositories share a directory name.
    pub path: String,
    pub version: u32,
    /// The Checks it declares, by name. What gates a Job here.
    pub checks: Vec<String>,
}

/// The models a Job may name, and the one it gets when it names none.
///
/// **`crates/config/settings.toml` supplies neither.** Two rows bear on this —
/// `kit-level-allowed-default-models-list` and `default-model-per-job-type` —
/// and neither carries a `default`, unlike the AgentHarness binary row beside
/// them. So the values below come from the adapter, which is the boundary
/// allowed to know a vendor's spellings, and that is a stand-in reported as
/// such rather than the configuration this is supposed to read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelChoices {
    /// Every model a proposal may name, in the order a picker offers them.
    pub models: Vec<String>,
    /// The one a proposal that names none is given. Always a member of
    /// `models`, so a picker can select it without a lookup that can miss.
    pub default: String,
}
