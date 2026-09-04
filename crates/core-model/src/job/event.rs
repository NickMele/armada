//! The `job_events` rows: one per move, with its reason, actor and time.
//!
//! **Two kinds of move, one log.** [`JobEvent`] records a status transition and
//! [`StepEvent`] records a step transition, and they share a sequence because
//! the outer machine gates the inner one: a step move can only be replayed
//! against the status the Job stood in when it happened, and two logs keyed
//! independently cannot be interleaved to say what that was. Every `StepEvent`
//! therefore carries the status it happened *under*, which the fold checks
//! against where it has got to — the same continuity check a `JobEvent` gets.
//!
//! `job-fields.toml` makes this log the authority and the `status` column a
//! cache of the fold over it — the column stores the fold's result so that
//! every surface that lists, sorts or filters Jobs reads it directly instead of
//! paying for the full history on every Board render. The transition and its
//! event land in one SQLite transaction, and Fleet re-folds non-terminal Jobs
//! at boot and lets the log win.
//!
//! It is defined here because
//! [`Job::transition`](crate::Job::transition) has nowhere else to put what it
//! knows, and because a shape retrofitted after the writers exist is a rewrite
//! of all of them — the same argument the log envelope was built on.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::envelope::{Actor, FieldValue, Timestamp};
use crate::job::escalation::StepLevelTrigger;
use crate::job::ids::{JobId, StepId};
use crate::job::status::{JobStatus, StepState};
use crate::job::step_machine::StepTarget;
use crate::job::transition::TransitionReason;

/// One transition, recorded.
///
/// # It has no id of its own
///
/// Nothing in this crate mints one — `Ulid` deliberately has no constructor
/// that does, because Fleet is the sole authority for the ids that name
/// records. Taking one as an argument to `transition` would put id-minting in
/// the signature of the state machine. `store` assigns the key when it writes
/// the row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobEvent {
    job_id: JobId,
    from: JobStatus,
    to: JobStatus,
    reason: TransitionReason,
    actor: Actor,
    at: Timestamp,
}

impl JobEvent {
    pub(crate) fn recorded(
        job_id: JobId,
        from: JobStatus,
        to: JobStatus,
        reason: TransitionReason,
        actor: Actor,
        at: Timestamp,
    ) -> Self {
        JobEvent {
            job_id,
            from,
            to,
            reason,
            actor,
            at,
        }
    }

    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }
    pub fn from(&self) -> JobStatus {
        self.from
    }
    pub fn to(&self) -> JobStatus {
        self.to
    }
    pub fn reason(&self) -> &TransitionReason {
        &self.reason
    }
    /// Who caused it. Three ways, and it cannot be reconstructed afterwards —
    /// a row that did not record who caused it never will.
    pub fn actor(&self) -> Actor {
        self.actor
    }
    pub fn at(&self) -> &Timestamp {
        &self.at
    }

    /// The event as structured log fields.
    ///
    /// [`FieldValue`] rather than a sentence, for the reason the envelope gives:
    /// a type that can hold anything ends up holding a formatted sentence, and
    /// nothing greps `msg`. A caller puts these on an [`Envelope`] with
    /// [`with_field`], and the ids stay fields.
    ///
    /// [`Envelope`]: crate::Envelope
    /// [`with_field`]: crate::Envelope::with_field
    pub fn fields(&self) -> BTreeMap<String, FieldValue> {
        let mut fields = BTreeMap::new();
        fields.insert(
            "job_status_from".to_string(),
            FieldValue::Str(self.from.as_wire().to_string()),
        );
        fields.insert(
            "job_status_to".to_string(),
            FieldValue::Str(self.to.as_wire().to_string()),
        );
        if let Some(reason) = self.reason.as_wire() {
            fields.insert(
                "transition_reason".to_string(),
                FieldValue::Str(reason.to_string()),
            );
        }
        if let TransitionReason::Attestation(owed) = &self.reason {
            let ids: Vec<FieldValue> = owed
                .ids()
                .map(|id| FieldValue::Str(id.as_str().to_string()))
                .collect();
            fields.insert("criteria_owed".to_string(), FieldValue::List(ids));
        }
        fields
    }
}

/// One step move, recorded.
///
/// Minted only by [`Job::transition_step`](crate::Job::transition_step), for
/// the reason [`JobEvent`] is minted only by `Job::transition`: a record of a
/// move that something other than the machine could have written is not
/// evidence of anything. It has no id of its own either — `store` assigns the
/// key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepEvent {
    job_id: JobId,
    step_id: StepId,
    from: StepState,
    to: StepState,
    /// Why the step stopped, on the one move that stops it.
    ///
    /// The log is the authority and `job_steps.last_verdict` is a cache of the
    /// fold over it, so a stop recorded without its trigger would rebuild as a
    /// stopped step nobody could ask why about.
    why: Option<StepLevelTrigger>,
    /// Which step sent the work back, on the one move that is a loop return.
    ///
    /// **`None` on every other move**, and therefore on every row of every
    /// linear workflow. It is here rather than derivable because the emitting
    /// step makes no move of its own on a return: `iteration_count` is its,
    /// and without this the log holds nothing to count against it.
    returned_by: Option<StepId>,
    under: JobStatus,
    actor: Actor,
    at: Timestamp,
}

impl StepEvent {
    /// Takes the whole [`StepTarget`] rather than a state and a reason, for the
    /// reason [`Target`](crate::Target) fuses the two: apart, a destination
    /// could be recorded beside a reason it does not store.
    pub(crate) fn recorded(
        job_id: JobId,
        step_id: StepId,
        from: StepState,
        to: &StepTarget,
        under: JobStatus,
        actor: Actor,
        at: Timestamp,
    ) -> Self {
        StepEvent {
            job_id,
            step_id,
            from,
            to: to.state(),
            why: to.why(),
            returned_by: to.returned_by().cloned(),
            under,
            actor,
            at,
        }
    }

    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }
    pub fn step_id(&self) -> &StepId {
        &self.step_id
    }
    pub fn from(&self) -> StepState {
        self.from
    }
    pub fn to(&self) -> StepState {
        self.to
    }
    /// What qualified the move. `Some` only on a stop, which is the one
    /// destination that stores a reason.
    pub fn why(&self) -> Option<StepLevelTrigger> {
        self.why
    }
    /// Which step routed a verdict back here. `Some` only on a loop return,
    /// which is the one move that stores it.
    pub fn returned_by(&self) -> Option<&StepId> {
        self.returned_by.as_ref()
    }
    /// The Job status this move happened beneath. Always one of
    /// [`ADVANCING_STATUSES`](crate::ADVANCING_STATUSES), and the Job does not
    /// leave it here — a step moving is not the Job moving.
    pub fn under(&self) -> JobStatus {
        self.under
    }
    pub fn actor(&self) -> Actor {
        self.actor
    }
    pub fn at(&self) -> &Timestamp {
        &self.at
    }

    /// The move as structured log fields, for the reason [`JobEvent::fields`]
    /// gives: nothing greps a sentence.
    pub fn fields(&self) -> BTreeMap<String, FieldValue> {
        let mut fields = BTreeMap::new();
        fields.insert(
            "step_id".to_string(),
            FieldValue::Str(self.step_id.as_str().to_string()),
        );
        fields.insert(
            "step_state_from".to_string(),
            FieldValue::Str(self.from.as_wire().to_string()),
        );
        fields.insert(
            "step_state_to".to_string(),
            FieldValue::Str(self.to.as_wire().to_string()),
        );
        fields.insert(
            "job_status".to_string(),
            FieldValue::Str(self.under.as_wire().to_string()),
        );
        if let Some(why) = self.why {
            fields.insert(
                "step_stop_trigger".to_string(),
                FieldValue::Str(why.as_wire().to_string()),
            );
        }
        fields
    }
}
