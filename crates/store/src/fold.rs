//! The log, and the Job it folds to.
//!
//! # This is the half of the step that is easy to skip
//!
//! An events table that is written and never read is wrong by the time
//! something needs it, and nothing says so in the meantime — every write looks
//! fine. So the read path is here now, it is the only way [`Job`]s come out of
//! this crate, and a test writes a whole history, drops every in-memory copy,
//! reopens the file and rebuilds the same `Job` from the log.
//!
//! # The fold replays the machine, it does not assign the answer
//!
//! Each event could set `status` to its own `status_to` in one line. Instead
//! each one is turned back into a [`Target`] and put through
//! [`Job::transition`] — the same function that produced it. **A history the
//! machine would not admit therefore fails to fold**, rather than reproducing a
//! Job that no sequence of legal moves could have reached. That is the
//! difference between reading the log and trusting it.
//!
//! # Where the fold starts
//!
//! At the `jobs` row, not at the log. Creation is not a transition — it has no
//! `from` — so no event describes it, and the entry status follows from
//! `origin`: `sub_dispatched` enters at `queued` and the other four enter at
//! `awaiting_approval`. That is not this file's rule; it is which constructor
//! `core-model` offers, and the rebuild calls the same one.

use core_model::{
    Actor, CriteriaOwed, EscalationTrigger, Job, JobId, JobStatus, PilotReason, Target, Timestamp,
    TransitionReason,
};

use crate::error::RowError;

/// One `job_events` row, as stored.
///
/// **Not a [`JobEvent`](core_model::JobEvent).** That type has no public
/// constructor — only `Job::transition` mints one, which is the property that
/// makes a recorded transition trustworthy — and it deliberately carries no id.
/// This carries the key the store assigned, which is the thing a log entry
/// needs and a freshly-minted event does not have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedEvent {
    pub(crate) seq: i64,
    pub(crate) job_id: JobId,
    pub(crate) from: JobStatus,
    pub(crate) to: JobStatus,
    pub(crate) reason: TransitionReason,
    pub(crate) actor: Actor,
    pub(crate) at: Timestamp,
}

impl RecordedEvent {
    /// The key the store assigned. Monotonic, never reused, and what the fold
    /// orders by — never `at`, which is injected and may repeat.
    pub fn seq(&self) -> i64 {
        self.seq
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
    pub fn actor(&self) -> Actor {
        self.actor
    }
    pub fn at(&self) -> &Timestamp {
        &self.at
    }
}

/// Replay every event onto a Job rebuilt at its creation state.
///
/// Three ways this refuses, and they are different faults:
///
/// - the event does not leave where the fold has got to — an event is missing,
///   or two are out of order;
/// - the reason does not fit the status it arrives at — `escalated` with no
///   trigger, `killed` with one;
/// - the machine does not admit the move at all.
pub(crate) fn replay(created: Job, events: &[RecordedEvent]) -> Result<Job, RowError> {
    let mut job = created;
    for event in events {
        if event.from != job.status() {
            return Err(RowError::EventDiscontinuity {
                job_id: event.job_id.clone(),
                seq: event.seq,
                folded: job.status(),
                recorded: event.from,
            });
        }
        let target = target(event)?;
        job = job
            .transition(target, event.actor, event.at.clone())
            .map_err(|cause| RowError::IllegalRecordedTransition {
                job_id: event.job_id.clone(),
                seq: event.seq,
                cause,
            })?
            .job;
    }
    Ok(job)
}

/// The destination and its reason, fused back into the value `transition`
/// takes.
///
/// [`Target`] fuses the two so that an illegal pair does not compile at a call
/// site. Coming off disk the pair is data and can be anything, so this is where
/// the check the type system does everywhere else has to be paid — once, here,
/// on the way in.
fn target(event: &RecordedEvent) -> Result<Target, RowError> {
    let fits = |ok: Option<Target>| {
        ok.ok_or_else(|| RowError::ReasonDoesNotFitStatus {
            job_id: event.job_id.clone(),
            seq: event.seq,
            status: event.to,
            reason_kind: kind(&event.reason).to_string(),
        })
    };
    match event.to {
        JobStatus::Escalated => fits(escalation(&event.reason).map(Target::Escalated)),
        JobStatus::Piloted => fits(pilot(&event.reason).map(Target::Piloted)),
        JobStatus::AwaitingAttestation => {
            fits(attestation(&event.reason).map(Target::AwaitingAttestation))
        }
        JobStatus::Queued => {
            fits(matches!(event.reason, TransitionReason::DerivedAtRead).then_some(Target::Queued))
        }
        other => fits(
            unqualified(other).filter(|_| matches!(event.reason, TransitionReason::Unqualified)),
        ),
    }
}

/// The eight destinations whose `reason_storage` is `None`.
fn unqualified(status: JobStatus) -> Option<Target> {
    match status {
        JobStatus::AwaitingApproval => Some(Target::AwaitingApproval),
        JobStatus::Running => Some(Target::Running),
        JobStatus::AwaitingReview => Some(Target::AwaitingReview),
        JobStatus::CompletedSuccess => Some(Target::CompletedSuccess),
        JobStatus::CompletedFailed => Some(Target::CompletedFailed),
        JobStatus::Rejected => Some(Target::Rejected),
        JobStatus::Superseded => Some(Target::Superseded),
        JobStatus::Killed => Some(Target::Killed),
        // The four that store a reason are matched before this is reached.
        JobStatus::Escalated
        | JobStatus::Piloted
        | JobStatus::AwaitingAttestation
        | JobStatus::Queued => None,
    }
}

fn escalation(reason: &TransitionReason) -> Option<EscalationTrigger> {
    match reason {
        TransitionReason::Escalation(trigger) => Some(*trigger),
        _ => None,
    }
}

fn pilot(reason: &TransitionReason) -> Option<PilotReason> {
    match reason {
        TransitionReason::Pilot(pilot) => Some(*pilot),
        _ => None,
    }
}

fn attestation(reason: &TransitionReason) -> Option<CriteriaOwed> {
    match reason {
        TransitionReason::Attestation(owed) => Some(owed.clone()),
        _ => None,
    }
}

/// What a stored reason is, for a refusal to name. The same five spellings
/// `columns::write_reason` puts in the column.
fn kind(reason: &TransitionReason) -> &'static str {
    match reason {
        TransitionReason::Unqualified => "unqualified",
        TransitionReason::DerivedAtRead => "derived_at_read",
        TransitionReason::Escalation(_) => "escalation",
        TransitionReason::Pilot(_) => "pilot",
        TransitionReason::Attestation(_) => "attestation",
    }
}
