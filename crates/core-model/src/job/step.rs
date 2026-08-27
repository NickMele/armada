//! The `job_steps` row: one per step of the frozen WorkflowDef.
//!
//! **Step state is rows, not a field.** They are written at Job creation, all
//! `not_started`, including for a Job that is never approved — a Job can sit at
//! `awaiting_approval` for days while the workflow file is edited, and writing
//! the rows at creation is what lets "what you approved is what runs" hold
//! through that window. A WorkflowDef edited in the repo mid-Job cannot reach a
//! Job already running against it, because the Job runs against these rows.
//!
//! # No counter columns
//!
//! `retry_count` and `iteration_count` are `job_steps` columns in the registry
//! and both say "arrives with retries, not at M1". They are absent here rather
//! than present and zero, because a counter that exists and never moves reads
//! as a counter that is working.
//!
//! `judge_calls` and `spawned_jobs` are absent for a different reason: the
//! registry types both `Derived — not stored`. They are answered by an index
//! over other rows, and a column would be a denormalisation needing its own
//! source-wins rule.

use alloc::string::String;
use alloc::vec::Vec;

use crate::envelope::Timestamp;
use crate::job::escalation::StepLevelTrigger;
use crate::job::ids::{JobId, StepId};
use crate::job::status::StepState;
use crate::job::step_machine::StepTarget;
use crate::job::workflow::EvidenceType;

/// The last verdict against a step.
///
/// **Three sources spell this differently, and this follows the field row.**
/// `job-fields.toml` gives `passed`, `failed(<escalation_reason>)` and
/// `not_reached`; `enum-verbs.toml`'s `step_verdict` vocabulary gives `pass`
/// and `evidence_suspect` instead. The field row is the authority on the
/// column, so it wins here, and `evidence_suspect` is reachable as
/// `Failed(EvidenceSuspect)`.
///
/// Activity and verdict are separate fields and never one combined enum: a step
/// retrying after an evidence-suspect flag is `Retrying` and
/// `Failed(EvidenceSuspect)` at the same moment, which one enum cannot say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepVerdict {
    Passed,
    /// **[`StepLevelTrigger`] and not [`EscalationTrigger`]**, which is the
    /// registry's own rule that only step-level triggers appear here, held by
    /// the type rather than by a check. Every trigger now carries a level, so
    /// the constraint that was unenforceable is enforceable.
    ///
    /// [`EscalationTrigger`]: crate::EscalationTrigger
    Failed(StepLevelTrigger),
    NotReached,
}

impl StepVerdict {
    pub fn as_wire(&self) -> &'static str {
        match self {
            StepVerdict::Passed => "passed",
            StepVerdict::Failed(_) => "failed",
            StepVerdict::NotReached => "not_reached",
        }
    }
}

/// One step of the frozen WorkflowDef, as Job creation receives it.
///
/// The seed of a row rather than the row itself: it carries what the
/// WorkflowDef knows — which step, and where in the order — and nothing about
/// where the step got to, which is the Job's to write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepSeed {
    pub step_id: StepId,
    /// The step's position in the WorkflowDef at freeze time. A rail renders
    /// past, current and future steps from one query without reading the
    /// WorkflowDef.
    pub ordinal: u32,
}

/// One `job_steps` row.
///
/// Every field is private and there is no setter. Two functions in this file
/// write `state`, and they are the whole inner machine's writing surface:
/// [`written_at_creation`](JobStep::written_at_creation), which takes no state
/// and produces `not_started`, and [`moved_to`](JobStep::moved_to), which is
/// `pub(crate)` and is called from one place —
/// [`Job::transition_step`](crate::Job::transition_step), after
/// `admits_step` has ruled on the move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobStep {
    job_id: JobId,
    step_id: StepId,
    ordinal: u32,
    state: StepState,
    last_verdict: Option<StepVerdict>,
    entered_at: Timestamp,
    updated_at: Timestamp,
}

impl JobStep {
    /// Write the row as Job creation writes it: `not_started`, no verdict, both
    /// timestamps the creation instant.
    ///
    /// This is the only constructor, and it takes no state — there is no way to
    /// mint a row already advanced.
    pub fn written_at_creation(job_id: JobId, seed: StepSeed, at: Timestamp) -> Self {
        JobStep {
            job_id,
            step_id: seed.step_id,
            ordinal: seed.ordinal,
            state: StepState::NotStarted,
            last_verdict: None,
            entered_at: at.clone(),
            updated_at: at,
        }
    }

    /// The row as the move leaves it. **Never called except by
    /// [`Job::transition_step`](crate::Job::transition_step)**, which is what
    /// asks the machine whether the move is admitted; this only writes down
    /// the answer.
    ///
    /// `entered_at` moves only on entering [`Running`](StepTarget::Running),
    /// because the field is "when this step was entered" and a step is entered
    /// when it starts being worked. Advancing writes `updated_at` alone, so the
    /// time a step took stays readable from the pair.
    ///
    /// `last_verdict` follows the destination rather than a caller:
    /// `advanced`'s meaning is "the step passed its advance gate", which is
    /// what `passed` records, and `stopped` is `failed` carrying the trigger
    /// the target already had to name. Entering `running` leaves the previous
    /// verdict standing — the registry is explicit that activity and verdict
    /// are separate fields, so starting work does not erase the last ruling.
    pub(crate) fn moved_to(&self, to: &StepTarget, at: Timestamp) -> JobStep {
        JobStep {
            job_id: self.job_id.clone(),
            step_id: self.step_id.clone(),
            ordinal: self.ordinal,
            state: to.state(),
            last_verdict: match to {
                StepTarget::Running => self.last_verdict,
                StepTarget::Advanced => Some(StepVerdict::Passed),
                StepTarget::Stopped(why) => Some(StepVerdict::Failed(*why)),
            },
            entered_at: match to {
                StepTarget::Running => at.clone(),
                StepTarget::Advanced | StepTarget::Stopped(_) => self.entered_at.clone(),
            },
            updated_at: at,
        }
    }

    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }
    pub fn step_id(&self) -> &StepId {
        &self.step_id
    }
    pub fn ordinal(&self) -> u32 {
        self.ordinal
    }
    pub fn state(&self) -> StepState {
        self.state
    }
    /// `None` until a gate has ruled on the step. The registry types the column
    /// nullable *and* names `not_reached` among its values, which are two ways
    /// to say the same thing; the nullability is followed here and the variant
    /// is kept because it renders.
    pub fn last_verdict(&self) -> Option<StepVerdict> {
        self.last_verdict
    }
    pub fn entered_at(&self) -> &Timestamp {
        &self.entered_at
    }
    pub fn updated_at(&self) -> &Timestamp {
        &self.updated_at
    }
}

/// Write one row per seed, in the order given.
pub(crate) fn rows_at_creation(
    job_id: &JobId,
    seeds: Vec<StepSeed>,
    at: &Timestamp,
) -> Vec<JobStep> {
    seeds
        .into_iter()
        .map(|seed| JobStep::written_at_creation(job_id.clone(), seed, at.clone()))
        .collect()
}

/// One step's evidence, as `store` holds it.
///
/// The record half of `verification`'s `Submission`, the way [`StepCheck`] is
/// the record half of a Check's run: the tool-call type cannot reach `store`,
/// because `config` sits between them and depends on it. The field names are
/// the same three the Agent Copy Contract defines, so the two are one
/// vocabulary rather than two.
///
/// **There is no `source` field here either.** A Drone marking its own evidence
/// human-attested has to be impossible on both sides of the write.
///
/// [`StepCheck`]: crate::StepCheck
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepEvidence {
    pub evidence_type: EvidenceType,
    /// What the work now does, as an observable.
    pub claimed: String,
    /// The artifact demonstrating it.
    pub shown_by: String,
    /// Everything the claim does not assert. Legitimately empty.
    pub not_claimed: String,
}
