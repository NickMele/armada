//! Every legal edge of the Job status machine, what may accompany it, and the
//! one error a move can produce.
//!
//! # The edges, from the registry
//!
//! [`EDGES`] is `domain/job-transitions.toml`, transcribed — one entry per
//! `[[transitions]]` row, in that file's order. The registry's `on` prose is
//! not carried: it says what fires an edge, which is Fleet's to know, and a
//! second copy of it here could only drift from the file that owns it. What
//! *is* carried is `escalation_trigger`, because it constrains what a caller
//! may pass — and a constraint the type system can hold is worth more than a
//! sentence.
//!
//! A transcription is a copy, and a copy drifts. The gate's `the transition
//! registry and the edge table name the same edges` is what holds it: every row
//! is an entry and every entry is a row, matched on `from` and `to` — which the
//! registry's own header calls the whole identity of an edge — with the trigger
//! compared as a value inside a matched pair. No count is written here or
//! asserted anywhere. A count is a second claim about the set that nothing
//! keeps true, and the one that stood here agreed with the registry while an
//! entry could have named the wrong status.
//!
//! # The destination and its reason are one value
//!
//! [`Target`] fuses them. Four statuses store a qualifying reason and eight do
//! not, so a `(status, reason)` pair has twelve legal shapes out of a much
//! larger product — and every illegal one would need a runtime check. Fused,
//! `Escalated` cannot be entered without a trigger, `Killed` cannot be given
//! one, and neither mistake reaches a check because neither compiles.

use alloc::vec::Vec;
use core::fmt;

use crate::job::escalation::EscalationTrigger;
use crate::job::ids::CriterionId;
use crate::job::status::JobStatus;

/// One legal edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub from: JobStatus,
    pub to: JobStatus,
    /// The named trigger this edge belongs to, where the registry gives it one.
    ///
    /// An edge that declares a trigger accepts that trigger and no other.
    /// `running -> escalated` declares none and is the default edge: a trigger
    /// with no edge of its own fires it.
    pub escalation_trigger: Option<EscalationTrigger>,
}

use JobStatus::*;

/// The edges of `domain/job-transitions.toml`, in that file's order. Nothing
/// else in this crate decides what is legal.
pub static EDGES: &[Edge] = &[
    edge(AwaitingApproval, Killed),
    edge(AwaitingApproval, Queued),
    edge(AwaitingApproval, Rejected),
    edge(AwaitingAttestation, CompletedFailed),
    edge(AwaitingAttestation, CompletedSuccess),
    edge(AwaitingAttestation, Killed),
    edge(AwaitingAttestation, Piloted),
    edge(AwaitingReview, AwaitingAttestation),
    edge(AwaitingReview, CompletedSuccess),
    triggered(AwaitingReview, Escalated, EscalationTrigger::Interrupted),
    edge(AwaitingReview, Killed),
    edge(AwaitingReview, Piloted),
    edge(AwaitingReview, Rejected),
    edge(AwaitingReview, Running),
    edge(Escalated, CompletedFailed),
    edge(Escalated, Killed),
    edge(Escalated, Piloted),
    edge(Escalated, Running),
    edge(Piloted, CompletedSuccess),
    edge(Piloted, Killed),
    edge(Piloted, Running),
    edge(Piloted, Superseded),
    edge(Queued, AwaitingApproval),
    triggered(Queued, Escalated, EscalationTrigger::DependencyFailed),
    edge(Queued, Killed),
    edge(Queued, Running),
    edge(Running, AwaitingApproval),
    edge(Running, AwaitingAttestation),
    edge(Running, AwaitingReview),
    edge(Running, CompletedFailed),
    edge(Running, CompletedSuccess),
    edge(Running, Escalated),
    edge(Running, Killed),
    edge(Running, Piloted),
];

const fn edge(from: JobStatus, to: JobStatus) -> Edge {
    Edge {
        from,
        to,
        escalation_trigger: None,
    }
}

const fn triggered(from: JobStatus, to: JobStatus, trigger: EscalationTrigger) -> Edge {
    Edge {
        from,
        to,
        escalation_trigger: Some(trigger),
    }
}

/// Why a person is being asked to take the Job over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PilotReason {
    TakeOver,
    RestartStep,
    Assist,
}

impl PilotReason {
    pub const ALL: &'static [PilotReason] = &[
        PilotReason::TakeOver,
        PilotReason::RestartStep,
        PilotReason::Assist,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            PilotReason::TakeOver => "take_over",
            PilotReason::RestartStep => "restart_step",
            PilotReason::Assist => "assist",
        }
    }

    /// Read a stored value back. `None` where it is not one of the three
    /// `piloted` names as its `reason_vocabulary`.
    pub fn from_wire(value: &str) -> Option<PilotReason> {
        PilotReason::ALL
            .iter()
            .copied()
            .find(|r| r.as_wire() == value)
    }
}

/// The criteria a Job owes outside Armada. **Never empty.**
///
/// A reference, not an enum: the registry is explicit that the reason for
/// `awaiting_attestation` is the `criterion_id`s owed, and that no per-criterion
/// variant exists because the enum binds closed sets. Non-emptiness is
/// structural — a Job cannot wait on an attestation it does not owe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CriteriaOwed {
    first: CriterionId,
    rest: Vec<CriterionId>,
}

impl CriteriaOwed {
    pub fn owing(first: CriterionId, rest: Vec<CriterionId>) -> Self {
        CriteriaOwed { first, rest }
    }

    /// One criterion, which is the common case.
    pub fn one(id: CriterionId) -> Self {
        CriteriaOwed {
            first: id,
            rest: Vec::new(),
        }
    }

    pub fn ids(&self) -> impl Iterator<Item = &CriterionId> {
        core::iter::once(&self.first).chain(self.rest.iter())
    }

    pub fn len(&self) -> usize {
        1 + self.rest.len()
    }

    /// Always false. Present so that a reader looking for the usual pair finds
    /// the answer rather than assuming there is a way to make one empty.
    pub fn is_empty(&self) -> bool {
        false
    }
}

/// Where a Job is going, carrying whatever that destination stores.
///
/// Twelve variants for twelve statuses. The four that the registry says store a
/// reason take it as a payload; the eight whose `reason_storage` is `None` — or
/// `Derived`, in `Queued`'s case — take nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    AwaitingApproval,
    /// The readiness reason is computed from `dependencies` and live headroom
    /// at read time and never stored: a held port span never self-clears, so a
    /// stored value would go stale the moment headroom frees. There is
    /// therefore nothing to pass.
    Queued,
    Running,
    AwaitingReview,
    Escalated(EscalationTrigger),
    Piloted(PilotReason),
    AwaitingAttestation(CriteriaOwed),
    CompletedSuccess,
    CompletedFailed,
    Rejected,
    Superseded,
    Killed,
}

impl Target {
    /// The status this target arrives at.
    pub fn status(&self) -> JobStatus {
        match self {
            Target::AwaitingApproval => JobStatus::AwaitingApproval,
            Target::Queued => JobStatus::Queued,
            Target::Running => JobStatus::Running,
            Target::AwaitingReview => JobStatus::AwaitingReview,
            Target::Escalated(_) => JobStatus::Escalated,
            Target::Piloted(_) => JobStatus::Piloted,
            Target::AwaitingAttestation(_) => JobStatus::AwaitingAttestation,
            Target::CompletedSuccess => JobStatus::CompletedSuccess,
            Target::CompletedFailed => JobStatus::CompletedFailed,
            Target::Rejected => JobStatus::Rejected,
            Target::Superseded => JobStatus::Superseded,
            Target::Killed => JobStatus::Killed,
        }
    }

    /// The reason as `job_events` will store it.
    pub fn reason(&self) -> TransitionReason {
        match self {
            Target::Queued => TransitionReason::DerivedAtRead,
            Target::Escalated(trigger) => TransitionReason::Escalation(*trigger),
            Target::Piloted(reason) => TransitionReason::Pilot(*reason),
            Target::AwaitingAttestation(owed) => TransitionReason::Attestation(owed.clone()),
            _ => TransitionReason::Unqualified,
        }
    }
}

/// The qualifying reason a transition carries, as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionReason {
    /// The destination's `reason_storage` is `None`. Eight statuses.
    Unqualified,
    /// `queued`'s readiness reason, which is recomputed at read time from
    /// `dependencies` and live headroom rather than stored.
    DerivedAtRead,
    Escalation(EscalationTrigger),
    Pilot(PilotReason),
    Attestation(CriteriaOwed),
}

impl TransitionReason {
    /// The wire value where the reason is a closed set, `None` where it is not
    /// one — an attestation debt is a list of references, and a derived reason
    /// is not stored at all.
    pub fn as_wire(&self) -> Option<&'static str> {
        match self {
            TransitionReason::Unqualified | TransitionReason::DerivedAtRead => None,
            TransitionReason::Escalation(trigger) => Some(trigger.as_wire()),
            TransitionReason::Pilot(reason) => Some(reason.as_wire()),
            TransitionReason::Attestation(_) => None,
        }
    }
}

/// A move the machine refuses. The typed leaf error of this crate.
///
/// Structured fields, never a formatted string: the statuses that failed, not
/// only the kind of failure. Nothing here is `Box<dyn Error>` — a leaf error
/// wraps into `ArmadaError` at a crate boundary, and that wrapper is not built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IllegalTransition {
    /// The Job is over. No terminal has an outbound edge, so nothing leaves it
    /// — not to another terminal, and not back.
    FromTerminal { from: JobStatus, to: JobStatus },
    /// Both statuses exist and the edge between them does not. A move from a
    /// status to itself lands here, which is correct: the registry names no
    /// self-edge, and a transition that changes nothing still writes an event.
    NoSuchEdge { from: JobStatus, to: JobStatus },
    /// The edge exists and belongs to a named trigger, and a different one was
    /// given. `awaiting_review -> escalated` is `interrupted`'s edge and no
    /// other trigger's: the liveness clock is suspended at a human gate, so
    /// `stalled` cannot fire there.
    WrongTrigger {
        from: JobStatus,
        to: JobStatus,
        expected: EscalationTrigger,
        given: EscalationTrigger,
    },
}

impl fmt::Display for IllegalTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IllegalTransition::FromTerminal { from, to } => write!(
                f,
                "{} is terminal and has no edge to {}",
                from.as_wire(),
                to.as_wire()
            ),
            IllegalTransition::NoSuchEdge { from, to } => {
                write!(f, "no edge {} -> {}", from.as_wire(), to.as_wire())
            }
            IllegalTransition::WrongTrigger {
                from,
                to,
                expected,
                given,
            } => write!(
                f,
                "{} -> {} is {}'s edge, and {} was given",
                from.as_wire(),
                to.as_wire(),
                expected.as_wire(),
                given.as_wire()
            ),
        }
    }
}

impl core::error::Error for IllegalTransition {}

/// Whether the machine admits this move, and why not if it does not.
pub(crate) fn admits(from: JobStatus, to: &Target) -> Result<(), IllegalTransition> {
    let arriving = to.status();
    if from.is_terminal() {
        return Err(IllegalTransition::FromTerminal { from, to: arriving });
    }
    let Some(edge) = EDGES.iter().find(|e| e.from == from && e.to == arriving) else {
        return Err(IllegalTransition::NoSuchEdge { from, to: arriving });
    };
    match (edge.escalation_trigger, to) {
        (Some(expected), Target::Escalated(given)) if expected != *given => {
            Err(IllegalTransition::WrongTrigger {
                from,
                to: arriving,
                expected,
                given: *given,
            })
        }
        _ => Ok(()),
    }
}
