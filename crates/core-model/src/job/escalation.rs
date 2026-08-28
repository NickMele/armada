//! Why a Job stopped and a person is being asked.
//!
//! The vocabulary the `escalated` status stores its reason from, and the one
//! `last_verdict` draws its `failed(<reason>)` payload from. One variant per key
//! of `domain/escalation-triggers.toml`, spelled as that file spells them — and
//! no count written here, because a count is a second claim about the set that
//! nothing keeps true. The gate's `the domain registries and the enums hold the
//! same set` compares the two both ways, which is the only statement of the
//! size that cannot go stale.
//!
//! # What this module does not decide
//!
//! `silent` is typed `Sub-kind` in the registry and every other row is typed
//! `Trigger`. The registry's README names that as a disagreement that "decides
//! a Rust type" and leaves it open. It is carried here as a variant of the same
//! enum with [`EscalationTrigger::sub_kind_of`] recording what it is a sub-kind
//! *of* — the shape that preserves the question. Collapsing it into a payload
//! on `Stalled` would answer it, and answering it is not this step's to do.

use crate::job::status::JobStatus;

/// Why a Job stopped and asked. The keys of
/// `domain/escalation-triggers.toml`, one variant each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EscalationTrigger {
    /// The Drone was refused a tool or command it needed and finished having
    /// submitted no evidence. It tried and was stopped, where
    /// [`Silent`](Self::Silent) called nothing at all — the remedies are
    /// opposite, so the boundary is the tool call and not the empty result.
    BlockedByPolicy,
    /// A Check hit its own bound. The Check did not fail, it did not finish, so
    /// retrying would reproduce the same hang.
    CheckTimeout,
    /// An upstream Job this one depends on reached a terminal status other than
    /// `completed_success`. `superseded` is the exception.
    DependencyFailed,
    /// Mechanically passed, semantically flagged as likely gamed.
    /// Resubmission under the same instructions would reproduce the gaming, so
    /// the retry flow is the wrong destination.
    EvidenceSuspect,
    /// Evidence exceeded `max_context_size`.
    EvidenceTooLarge,
    /// A Job tripped the sub-dispatch cap or the sub-dispatch rate threshold.
    /// Counts Jobs only — a Judge call is not a sub-dispatch.
    FanOut,
    /// Evidence was submitted, honestly did not pass, and the retry limit is
    /// exhausted. The ordinary failure.
    GateFailure,
    /// Fleet could not read what it needed in order to rule — the Job's diff,
    /// its changed files, the step's patch, or an answer the Judge never gave.
    ///
    /// **The opposite of [`GateFailure`](Self::GateFailure) about the same
    /// step**: there the machinery worked and the work did not clear the bar,
    /// here the machinery is what failed and the criteria were never reached.
    /// A machine that cannot answer must not produce a verdict in either
    /// direction, so the Job stops and names the artifact instead.
    GateUndecided,
    /// The Drone called `escape_hatch` on a Job Fleet had not marked for the
    /// handoff. **The pull is refused and the Job surfaces**: a Drone does not
    /// open a terminal on the operator's machine on its own initiative, but a
    /// Drone reaching for the hatch unbidden has said it is stuck, and that is
    /// the most reliable stuck signal there is.
    ///
    /// Not [`Thrashing`](Self::Thrashing): the call stands in place of
    /// thrashing rather than following it, and nothing mechanical has to fire
    /// first.
    HatchUnbidden,
    /// Crash recovery. A Job marked running has no matching OS process. Set by
    /// Fleet's restart reconciliation.
    Interrupted,
    /// A loop workflow's step hit its `iteration_cap`. Nothing failed — the
    /// loop did not converge, which is why the count that tripped it is
    /// `iteration_count` and never the retry budget.
    LoopCap,
    /// A running Job exhausted CPU or memory. Belongs to the process, not to
    /// any step it happened to be on.
    ResourceExhausted,
    /// The Drone process exited normally having called no tool at all. Declared
    /// a sub-kind of [`Stalled`](Self::Stalled): it pauses the Job identically
    /// and differs only in the recommended action — rephrase and redispatch,
    /// rather than plain redispatch.
    Silent,
    /// Fleet has no signal despite the Job having been active. Detected by the
    /// liveness timer, which runs only while the Job is `running`.
    Stalled,
    /// Active but not converging, and the forced report also failed.
    Thrashing,
}

/// Whether a row is a trigger of its own or a sub-kind of another, as the
/// registry types it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerKind {
    Trigger,
    SubKind,
}

/// Whether a trigger describes one step or the whole Job.
///
/// **Total, and deliberately not an `Option`.** `last_verdict` admits
/// step-level triggers only, so a level that could be absent would be a
/// trigger nothing could check that rule against. A sub-kind takes its
/// parent's, which is the registry's own rule that it pauses the Job exactly
/// as its parent does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerLevel {
    Step,
    Job,
}

/// A trigger the registry types step-level, and the only thing `last_verdict`
/// admits.
///
/// A newtype over [`EscalationTrigger`] rather than a second enum of the seven:
/// the mapping already exists on [`EscalationTrigger::level`], and a copy of it
/// would be the second vocabulary this crate keeps refusing. The narrowing is
/// paid once, here, and every call site downstream is total — a step cannot be
/// stopped with `fan_out` because there is no way to build the argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepLevelTrigger(EscalationTrigger);

impl StepLevelTrigger {
    /// `None` where the registry types the trigger Job-level. A sub-kind reads
    /// its parent's level, so it narrows exactly as its parent does.
    pub fn of(trigger: EscalationTrigger) -> Option<StepLevelTrigger> {
        matches!(trigger.level(), TriggerLevel::Step).then_some(StepLevelTrigger(trigger))
    }

    pub fn trigger(&self) -> EscalationTrigger {
        self.0
    }

    pub fn as_wire(&self) -> &'static str {
        self.0.as_wire()
    }
}

impl EscalationTrigger {
    /// Every variant, in registry order.
    pub const ALL: &'static [EscalationTrigger] = &[
        EscalationTrigger::BlockedByPolicy,
        EscalationTrigger::CheckTimeout,
        EscalationTrigger::DependencyFailed,
        EscalationTrigger::EvidenceSuspect,
        EscalationTrigger::EvidenceTooLarge,
        EscalationTrigger::FanOut,
        EscalationTrigger::GateFailure,
        EscalationTrigger::GateUndecided,
        EscalationTrigger::HatchUnbidden,
        EscalationTrigger::Interrupted,
        EscalationTrigger::LoopCap,
        EscalationTrigger::ResourceExhausted,
        EscalationTrigger::Silent,
        EscalationTrigger::Stalled,
        EscalationTrigger::Thrashing,
    ];

    /// The wire value, which is also the registry key.
    pub fn as_wire(&self) -> &'static str {
        match self {
            EscalationTrigger::BlockedByPolicy => "blocked_by_policy",
            EscalationTrigger::CheckTimeout => "check_timeout",
            EscalationTrigger::DependencyFailed => "dependency_failed",
            EscalationTrigger::EvidenceSuspect => "evidence_suspect",
            EscalationTrigger::EvidenceTooLarge => "evidence_too_large",
            EscalationTrigger::FanOut => "fan_out",
            EscalationTrigger::GateFailure => "gate_failure",
            EscalationTrigger::GateUndecided => "gate_undecided",
            EscalationTrigger::HatchUnbidden => "hatch_unbidden",
            EscalationTrigger::Interrupted => "interrupted",
            EscalationTrigger::LoopCap => "loop_cap",
            EscalationTrigger::ResourceExhausted => "resource_exhausted",
            EscalationTrigger::Silent => "silent",
            EscalationTrigger::Stalled => "stalled",
            EscalationTrigger::Thrashing => "thrashing",
        }
    }

    /// Read a stored value back. `None` where it is not one of them.
    pub fn from_wire(value: &str) -> Option<EscalationTrigger> {
        EscalationTrigger::ALL
            .iter()
            .copied()
            .find(|t| t.as_wire() == value)
    }

    /// `Trigger` or `SubKind`, as the registry types the row.
    pub fn kind(&self) -> TriggerKind {
        match self {
            EscalationTrigger::Silent => TriggerKind::SubKind,
            _ => TriggerKind::Trigger,
        }
    }

    /// The trigger this one is a sub-kind of, where the registry says it is
    /// one. `None` on every row typed `Trigger`.
    pub fn sub_kind_of(&self) -> Option<EscalationTrigger> {
        match self {
            EscalationTrigger::Silent => Some(EscalationTrigger::Stalled),
            _ => None,
        }
    }

    /// Step or Job, as the registry decides it.
    ///
    /// A step-level trigger attaches to a step and can therefore name which
    /// step stopped, which is what `last_verdict` needs and what makes
    /// restarting that step later a coherent act. A Job-level one has no step
    /// to attach to, because no step is the reason.
    pub fn level(&self) -> TriggerLevel {
        match self {
            EscalationTrigger::BlockedByPolicy
            | EscalationTrigger::CheckTimeout
            | EscalationTrigger::EvidenceSuspect
            | EscalationTrigger::EvidenceTooLarge
            | EscalationTrigger::GateFailure
            | EscalationTrigger::GateUndecided
            | EscalationTrigger::LoopCap
            | EscalationTrigger::Thrashing => TriggerLevel::Step,
            EscalationTrigger::DependencyFailed
            | EscalationTrigger::FanOut
            | EscalationTrigger::HatchUnbidden
            | EscalationTrigger::Interrupted
            | EscalationTrigger::ResourceExhausted
            | EscalationTrigger::Stalled => TriggerLevel::Job,
            // A sub-kind has no level of its own. It pauses the Job exactly as
            // its parent does, so it reads its parent's rather than declaring
            // a second answer that could drift from it.
            EscalationTrigger::Silent => EscalationTrigger::Stalled.level(),
        }
    }

    /// The edge this trigger fires where it has one of its own, as
    /// `from -> to`.
    ///
    /// A trigger with none fires `running -> escalated`, which that edge's own
    /// registry row names as the default. This returns `None` for those rather
    /// than filling the default in, because the edge table is where the default
    /// lives and two copies of it would be one too many.
    pub fn declared_edge(&self) -> Option<(JobStatus, JobStatus)> {
        match self {
            EscalationTrigger::DependencyFailed => Some((JobStatus::Queued, JobStatus::Escalated)),
            EscalationTrigger::Interrupted => {
                Some((JobStatus::AwaitingReview, JobStatus::Escalated))
            }
            _ => None,
        }
    }
}
