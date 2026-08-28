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

use crate::checks::{DeclaredCheck, DeclaredJudge};
use crate::enums::AdvanceGate;
use crate::ids::{ManifestId, StepId, WorkflowId};

/// One step of a workflow, as the definition declares it.
///
/// **This is the one place a step's Checks are read from**, and `get_job`
/// answers from the same values rather than a second copy on the Job. A
/// declaration stored twice is a registry that drifts; the Job's frozen rows
/// carry where a step *got to*, which is a different fact with a different
/// authority.
///
/// **The same declarations [`StepDetail`](crate::StepDetail) carries** — the
/// Checks, what the step asks the Judge, and what it takes to advance — in the
/// same shapes. What a person approves before a dispatch and what the rail
/// shows during one are one sentence read at two moments, so a second
/// vocabulary for the earlier one would be two spellings of one declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_id: StepId,
    /// What a person reads. **Never absent and never blank** — the id stands in
    /// where the definition declares no label, for the reason
    /// [`StepDetail::label`](crate::StepDetail::label) does the same.
    pub label: String,
    /// The Checks the step declares, in the order it declares them.
    ///
    /// **Empty is the sentence "this step declares no check."** A WorkflowDef
    /// may write `mechanical_checks: []` or leave the key out and the two mean
    /// the same thing, so the wire spells it one way — always a list, never
    /// absent — and a reader never has to infer an ungated step from a gap.
    pub checks: Vec<DeclaredCheck>,
    /// What the step asks of the Judge, in the order it asks it. Counts and
    /// panel sizes; **the questions themselves do not cross**, which is
    /// [`DeclaredJudge`]'s own rule and not a second one here.
    ///
    /// **Empty is "the Judge is never called on this step"**, which is most
    /// steps — and it is a sentence rather than a gap for the reason `checks`
    /// is. Absent has no meaning on this shape: a workflow Fleet is serving is
    /// a workflow Fleet holds, so there is no "cannot say" to spell, which is
    /// what the same field on [`StepDetail`](crate::StepDetail) is optional for.
    ///
    /// **An inert entry does not cross**, on the same ground the rail's does:
    /// the domain spells a disabled judge check and an absent one identically,
    /// so an entry that asks nothing and looks for nothing would make a step
    /// preview as judged when no Judge will be called.
    pub judge_checks: Vec<DeclaredJudge>,
    /// What it takes to advance past this step.
    ///
    /// **This is the whole of what a preview was missing.** A workflow that
    /// will stop and wait for a person at `handoff` is what somebody is
    /// agreeing to when they approve the dispatch, and it previewed as a step
    /// with nothing on it — the same defect the rail had, one moment earlier
    /// and on the surface where the decision is actually taken.
    pub advance_gate: AdvanceGate,
}

/// One workflow Fleet holds, as a picker offers it.
///
/// The steps are what each one declares rather than the whole definition: a
/// composer needs the count and the words to say "4 steps, ending at close",
/// and what each step will gate on before anybody approves it. The evidence
/// types are still `get_job`'s business.
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
