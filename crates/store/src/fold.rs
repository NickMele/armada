//! The log, and the Job it folds to.
//!
//! **The half of the step that is easy to skip.** An events table written and
//! never read is wrong by the time something needs it and nothing says so in
//! the meantime, so the read path is here and it is the only way [`Job`]s come
//! out of this crate.
//!
//! **The fold replays the machine; it does not assign the answer.** Each event
//! could set `status` to its own `status_to` in one line. Instead each is turned
//! back into a [`Target`] and put through [`Job::transition`], the same function
//! that produced it — so **a history the machine would not admit fails to
//! fold**, rather than reproducing a Job no legal sequence could have reached.
//!
//! **Both machines, one order.** A step move is a row in the same log and goes
//! back through [`Job::transition_step`] the same way, rebuilding
//! `current_step_id` and every `job_steps` row. It is why the log is one table:
//! the inner machine advances beneath two of the twelve statuses only, so a
//! step move replays only once the fold has replayed the Job up to it, and its
//! `status_from` is checked against the fold rather than believed — which a
//! separately keyed second log could not have offered.
//!
//! **It starts at the `jobs` row, not the log.** Creation has no `from`, so no
//! event describes it and the entry status follows from `origin`:
//! `sub_dispatched` enters at `queued`, the other four at `awaiting_approval`.
//! That is which constructor `core-model` offers, and the rebuild calls it.

use core_model::{
    Actor, CriteriaOwed, DroneId, DronePresence, EscalationTrigger, Job, JobId, JobStatus,
    PilotReason, StepId, StepLevelTrigger, StepState, StepTarget, Target, Timestamp,
    TransitionReason, Ulid,
};
use rusqlite::Row;

use crate::columns;
use crate::error::{fault, RowError};
use crate::open::Store;
use crate::row::{column, enum_value, maybe, string};

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
    pub(crate) under: JobStatus,
    pub(crate) moved: Moved,
    pub(crate) actor: Actor,
    pub(crate) at: Timestamp,
}

/// What the row says moved. The `kind` column, read back into the two shapes
/// the schema admits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Moved {
    /// The Job left [`RecordedEvent::under`] for `to`.
    Job {
        to: JobStatus,
        reason: TransitionReason,
    },
    /// One step moved and the Job did not. Which is why there is no `to`
    /// status here: `status_from` and `status_to` on the row are the same
    /// value, and it is [`RecordedEvent::under`].
    Step {
        step_id: StepId,
        from: StepState,
        to: StepState,
        /// Why the step stopped, on the one move that stops it. The column pair
        /// is the same one a Job row's reason uses.
        why: Option<StepLevelTrigger>,
    },
    /// A Drone arrived on a step of the Job or left it, and neither the Job
    /// nor the step moved. The pointer this folds to is the step's
    /// `assigned_drone`, which has no states of its own — hence a presence
    /// rather than a `from` and a `to`.
    ///
    /// **The step is what the fold keys by.** A drone row without one could be
    /// applied to any step, and a Job that ran four Drones would fold to one
    /// pointer naming the last of them.
    Drone {
        step_id: StepId,
        drone_id: DroneId,
        presence: DronePresence,
    },
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
    /// The status the Job stood in when this happened. For a Job transition
    /// that is the status it left; for a step move it is the status it stayed
    /// in.
    pub fn under(&self) -> JobStatus {
        self.under
    }
    pub fn moved(&self) -> &Moved {
        &self.moved
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
/// Five ways this refuses, and they are different faults:
///
/// - the row does not stand where the fold has got to — an event is missing,
///   or two are out of order;
/// - the reason does not fit the status it arrives at — `escalated` with no
///   trigger, `killed` with one;
/// - a step row does not leave the state the fold has that step in;
/// - a step row arrives at a state no [`StepTarget`] names, which is a machine
///   this build does not have;
/// - either machine does not admit the move at all.
pub(crate) fn replay(created: Job, events: &[RecordedEvent]) -> Result<Job, RowError> {
    let mut job = created;
    for event in events {
        if event.under != job.status() {
            return Err(RowError::EventDiscontinuity {
                job_id: event.job_id.clone(),
                seq: event.seq,
                folded: job.status(),
                recorded: event.under,
            });
        }
        job = match &event.moved {
            Moved::Job { .. } => {
                job.transition(target(event)?, event.actor, event.at.clone())
                    .map_err(|cause| RowError::IllegalRecordedTransition {
                        job_id: event.job_id.clone(),
                        seq: event.seq,
                        cause,
                    })?
                    .job
            }
            Moved::Step {
                step_id,
                from,
                to,
                why,
            } => step(&job, event, step_id, *from, *to, *why)?,
            Moved::Drone {
                step_id,
                drone_id,
                presence,
            } => drone(&job, event, step_id, drone_id, *presence)?,
        };
    }
    Ok(job)
}

/// One step row, put back through the mutator that wrote it.
///
/// The state it says it left is checked against the state the fold has the step
/// in, exactly as a Job row's `status_from` is. Without it a step row could
/// name any edge in the table and the machine would admit it — the continuity,
/// not the edge, is what makes a replay a replay.
fn step(
    job: &Job,
    event: &RecordedEvent,
    step_id: &StepId,
    from: StepState,
    to: StepState,
    why: Option<StepLevelTrigger>,
) -> Result<Job, RowError> {
    let folded = job.step(step_id).map(|row| row.state());
    if folded != Some(from) {
        return Err(RowError::StepEventDiscontinuity {
            job_id: event.job_id.clone(),
            seq: event.seq,
            step_id: step_id.clone(),
            folded,
            recorded: from,
        });
    }
    // `from` as well as `to`, because the two arrivals at `running` are told
    // apart by where they came from and by nothing else: a loop return carries
    // no trigger to distinguish it the way an override does at `advanced`.
    let target =
        StepTarget::arriving_at(from, to, why).ok_or_else(|| RowError::StepStateNotReachable {
            job_id: event.job_id.clone(),
            seq: event.seq,
            step_id: step_id.clone(),
            state: to,
        })?;
    Ok(job
        .transition_step(step_id, target, event.actor, event.at.clone())
        .map_err(|cause| RowError::IllegalRecordedStepTransition {
            job_id: event.job_id.clone(),
            seq: event.seq,
            cause,
        })?
        .job)
}

/// One drone row, put back through the mutator that wrote it.
///
/// A spawn onto a step that already holds a Drone, an exit from one holding
/// none, or either naming a step the Job does not have, is refused here rather
/// than applied — the same continuity the step fold checks, over a pointer
/// instead of a state.
fn drone(
    job: &Job,
    event: &RecordedEvent,
    step_id: &StepId,
    drone_id: &DroneId,
    presence: DronePresence,
) -> Result<Job, RowError> {
    let moved = match presence {
        DronePresence::Spawned => {
            job.drone_spawned(step_id, drone_id.clone(), event.actor, event.at.clone())
        }
        DronePresence::Exited => job.drone_exited(step_id, event.actor, event.at.clone()),
    };
    Ok(moved
        .map_err(|cause| RowError::IllegalRecordedDroneMove {
            job_id: event.job_id.clone(),
            seq: event.seq,
            cause,
        })?
        .job)
}

/// The destination and its reason, fused back into the value `transition`
/// takes.
///
/// [`Target`] fuses the two so that an illegal pair does not compile at a call
/// site. Coming off disk the pair is data and can be anything, so this is where
/// the check the type system does everywhere else has to be paid — once, here,
/// on the way in.
fn target(event: &RecordedEvent) -> Result<Target, RowError> {
    let Moved::Job { to, reason } = &event.moved else {
        // Only a Job row reaches here: the caller matched on the kind first.
        unreachable!("a step row has no status target");
    };
    let fits = |ok: Option<Target>| {
        ok.ok_or_else(|| RowError::ReasonDoesNotFitStatus {
            job_id: event.job_id.clone(),
            seq: event.seq,
            status: *to,
            reason_kind: kind(reason).to_string(),
        })
    };
    match to {
        JobStatus::Escalated => fits(escalation(reason).map(Target::Escalated)),
        JobStatus::Piloted => fits(pilot(reason).map(Target::Piloted)),
        JobStatus::AwaitingAttestation => {
            fits(attestation(reason).map(Target::AwaitingAttestation))
        }
        JobStatus::Queued => {
            fits(matches!(reason, TransitionReason::DerivedAtRead).then_some(Target::Queued))
        }
        other => {
            fits(unqualified(*other).filter(|_| matches!(reason, TransitionReason::Unqualified)))
        }
    }
}

/// The nine destinations whose `reason_storage` is `None`.
fn unqualified(status: JobStatus) -> Option<Target> {
    match status {
        JobStatus::AwaitingApproval => Some(Target::AwaitingApproval),
        JobStatus::Running => Some(Target::Running),
        JobStatus::AwaitingReview => Some(Target::AwaitingReview),
        // What stopped the work is on the stopped step's own `last_verdict`,
        // so the Job's transition carries nothing to fit. See `Target`.
        JobStatus::AwaitingRepair => Some(Target::AwaitingRepair),
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

/// A step row's stored reason, narrowed to what `last_verdict` admits.
///
/// A step move stores either nothing or the trigger that stopped it. A pilot
/// reason or an attestation debt on a step row is a shape the schema's trigger
/// refuses to write, so one arriving here came from outside this crate.
fn stop_reason(reason: TransitionReason) -> Result<Option<StepLevelTrigger>, String> {
    match reason {
        TransitionReason::Unqualified => Ok(None),
        TransitionReason::Escalation(trigger) => StepLevelTrigger::of(trigger).map(Some).ok_or(
            format!("`{}` is not a step-level trigger", trigger.as_wire()),
        ),
        other => Err(format!(
            "`{}` is not a reason a step move stores",
            kind(&other)
        )),
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

// ------------------------------------------------------- reading the rows back

impl Store {
    /// One Job's history, oldest first — **both machines, in one order.**
    ///
    /// Never edited, never removed, and public so that something other than the
    /// fold can read it: a timeline is one query over this, not a join across
    /// two logs.
    pub fn events_for(&self, job_id: &JobId) -> Result<Vec<RecordedEvent>, RowError> {
        let mut statement = self
            .conn
            .prepare(SELECT_EVENTS)
            .map_err(fault("preparing the event read"))
            .map_err(RowError::Database)?;
        let rows = statement
            .query_map((job_id.as_str(),), |row| Ok(event(row)))
            .map_err(fault("reading events"))
            .map_err(RowError::Database)?;
        let mut events = Vec::new();
        for row in rows {
            let row = row
                .map_err(fault("reading an event"))
                .map_err(RowError::Database)?;
            events.push(row?);
        }
        Ok(events)
    }
}

const SELECT_EVENTS: &str = "SELECT seq, job_id, kind, status_from, status_to, reason_kind,
                             reason_value, step_id, state_from, state_to, drone_id, actor, at
                             FROM job_events WHERE job_id = ?1 ORDER BY seq";

/// One log row, of either kind.
///
/// `status_from` is read for both, because both stand beneath a status: a Job
/// transition leaves it, a step move stays in it. That is what lets the fold run
/// one continuity check over the whole log.
fn event(row: &Row<'_>) -> Result<RecordedEvent, RowError> {
    let actor = string(row, "actor")?;
    Ok(RecordedEvent {
        seq: row.get("seq").map_err(column("job_events", "seq"))?,
        job_id: JobId::carried(Ulid::carried(string(row, "job_id")?)),
        under: enum_value(
            JobStatus::from_wire,
            "job_events",
            "status_from",
            &string(row, "status_from")?,
        )?,
        moved: moved(row)?,
        actor: Actor::from_wire(&actor).ok_or(RowError::UnknownEnumValue {
            table: "job_events",
            column: "actor",
            value: actor,
        })?,
        at: Timestamp::from_rfc3339(string(row, "at")?),
    })
}

/// What the row says moved, read from `kind`.
///
/// A column missing for the kind it belongs to is malformed rather than
/// defaulted. The shape trigger in V3 refuses to write such a row, so one
/// arriving here came from outside this crate — the same argument the blank
/// title makes.
fn moved(row: &Row<'_>) -> Result<Moved, RowError> {
    let kind = string(row, "kind")?;
    match kind.as_str() {
        "job_transition" => {
            let reason_kind = string(row, "reason_kind")?;
            let reason_value: Option<String> = maybe(row, "reason_value")?;
            Ok(Moved::Job {
                to: enum_value(
                    JobStatus::from_wire,
                    "job_events",
                    "status_to",
                    &string(row, "status_to")?,
                )?,
                reason: columns::read_reason(&reason_kind, reason_value.as_deref()).map_err(
                    |detail| RowError::MalformedColumn {
                        table: "job_events",
                        column: "reason_value",
                        detail,
                    },
                )?,
            })
        }
        "step_transition" => {
            let reason_kind = string(row, "reason_kind")?;
            let reason_value: Option<String> = maybe(row, "reason_value")?;
            let reason = columns::read_reason(&reason_kind, reason_value.as_deref());
            Ok(Moved::Step {
                step_id: StepId::new(present(row, "step_id")?),
                from: enum_value(
                    StepState::from_wire,
                    "job_events",
                    "state_from",
                    &present(row, "state_from")?,
                )?,
                to: enum_value(
                    StepState::from_wire,
                    "job_events",
                    "state_to",
                    &present(row, "state_to")?,
                )?,
                why: reason
                    .and_then(stop_reason)
                    .map_err(|detail| RowError::MalformedColumn {
                        table: "job_events",
                        column: "reason_value",
                        detail,
                    })?,
            })
        }
        // The `kind` column is the presence, spelled by the domain enum itself,
        // so the trigger's `IN` list and this arm cannot drift apart.
        other if DronePresence::from_wire(other).is_some() => Ok(Moved::Drone {
            step_id: StepId::new(present(row, "step_id")?),
            drone_id: DroneId::carried(Ulid::carried(present(row, "drone_id")?)),
            presence: DronePresence::from_wire(other).expect("just read as a presence"),
        }),
        _ => Err(RowError::UnknownEnumValue {
            table: "job_events",
            column: "kind",
            value: kind,
        }),
    }
}

/// A column the row's own kind requires. Null is malformed, never a default.
fn present(row: &Row<'_>, name: &'static str) -> Result<String, RowError> {
    maybe(row, name)?.ok_or(RowError::MalformedColumn {
        table: "job_events",
        column: name,
        detail: "the row's kind requires it and it is null".to_string(),
    })
}
