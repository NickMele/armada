//! The `job_events` row: one per transition, with its reason, actor and time.
//!
//! `job-fields.toml` makes this log the authority and the `status` column a
//! cache of the fold over it — the column stores the fold's result so that
//! every surface that lists, sorts or filters Jobs reads it directly instead of
//! paying for the full history on every Board render. The transition and its
//! event land in one SQLite transaction, and Fleet re-folds non-terminal Jobs
//! at boot and lets the log win.
//!
//! Nothing persists one yet. It is defined here because
//! [`Job::transition`](crate::Job::transition) has nowhere else to put what it
//! knows, and because a shape retrofitted after the writers exist is a rewrite
//! of all of them — the same argument the log envelope was built on.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::envelope::{Actor, FieldValue, Timestamp};
use crate::job::ids::JobId;
use crate::job::status::JobStatus;
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
