//! The condition an edge is admitted under, beyond the edge existing.
//!
//! # A guard is handed the step rows, and nothing else
//!
//! [`Guard::refusing`] takes `&[JobStep]`. Not a `&Job`, and emphatically not
//! anything that could read a database: `core-model` has no I/O and
//! `Job::transition` is pure, so a guard that needed a row it was not given
//! could not be written here at all. Narrowing it further than `&Job` is the
//! same move a Drone's VCS handle makes about push — a guard cannot consult the
//! Drone assignment or the dependency list because those are not in scope at
//! the call site, rather than because a rule says not to. A guard that needs
//! another field of the record widens this signature, deliberately and once.
//!
//! # The predicate *is* the declared set
//!
//! [`holds`](Guard::holds) names the step states a guarded edge admits and
//! [`refusing`](Guard::refusing) is written in terms of it. There is no second
//! statement of the condition that could disagree with the first, which matters
//! because the gate reads `holds` to narrow `job-statuses.toml`'s `step_states`
//! rows: a predicate written independently of the set would let the machine
//! refuse one thing while the registry claimed another.
//!
//! `domain/transition-guards.toml` is the authority on the set, this is that
//! file transcribed, and the gate's `a status row declares the step states a
//! Job holds beneath it` compares them.

use crate::job::status::StepState;
use crate::job::step::JobStep;

/// A condition on an edge. One, from `domain/transition-guards.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Guard {
    /// Every step of the frozen WorkflowDef passed its advance gate.
    ///
    /// What `completed_success` has always claimed — "Frozen, every step
    /// advanced" — and could not hold while an edge had no condition.
    EveryStepAdvanced,
    /// No step is still being worked.
    ///
    /// **On `running -> awaiting_repair` alone**, and on no edge into
    /// `completed_failed`: both of those are a person accepting a failure, and
    /// a Job escalated on `stalled` holds a `running` step legitimately. So
    /// `completed_failed`'s `step_states` row does not narrow behind this
    /// guard, and is not meant to; `awaiting_repair`'s does, and is the only
    /// row that narrows against `running`.
    ///
    /// **It moved with its edge in #208**, which replaced
    /// `running -> completed_failed`: the guard belongs to the moment a gate
    /// stops the work, not to the destination that moment used to have.
    ///
    /// It is defence in depth — `fleet::dispatch` stops the step before the Job
    /// moves — and #179 is why it exists anyway: that path was correct at three
    /// of its four rulings for a month.
    NoStepRunning,
}

impl Guard {
    /// Every variant, in registry order. What a set-comparison rule reads.
    pub const ALL: &'static [Guard] = &[Guard::EveryStepAdvanced, Guard::NoStepRunning];

    /// The wire value, which is also the registry key. What a refusal names,
    /// so that a caller reporting one says the condition rather than the edge.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Guard::EveryStepAdvanced => "every_step_advanced",
            Guard::NoStepRunning => "no_step_running",
        }
    }

    /// Read a stored value back. `None` where it is not one of the guards.
    pub fn from_wire(value: &str) -> Option<Guard> {
        Guard::ALL.iter().copied().find(|g| g.as_wire() == value)
    }

    /// The step states a Job may hold and still cross an edge this guards.
    ///
    /// The gate reads these arms and carries only these states across a guarded
    /// edge, which is what lets a destination's `step_states` row narrow to
    /// something a reader learns from.
    pub const fn holds(&self) -> &'static [StepState] {
        match self {
            Guard::EveryStepAdvanced => &[StepState::Advanced],
            // Every state but `running`, written out rather than derived: the
            // gate reads these arms out of the source text, so a set built by
            // filtering `StepState::ALL` would be a set it could not read. The
            // list says what the name says and no more — a `retrying` step is
            // passed through and not rested in, and `awaiting_human` is a state
            // nothing writes yet.
            Guard::NoStepRunning => &[
                StepState::Advanced,
                StepState::AwaitingHuman,
                StepState::NotStarted,
                StepState::Retrying,
                StepState::Stopped,
            ],
        }
    }

    /// The first step row this guard refuses, or `None` where it admits the
    /// move.
    ///
    /// The row and not a bool, so the refusal can name which step is in the way
    /// and what it holds. A person reading "the Job cannot complete" learns
    /// nothing; "step `fix` is running" is the whole finding.
    ///
    /// **The row's `state` is the latest attempt** and there is no other
    /// reading available: a `job_steps` row is overwritten by every move, and
    /// how many runs it took lives in the log — see [`Attempt`](crate::Attempt).
    pub(crate) fn refusing<'a>(&self, steps: &'a [JobStep]) -> Option<&'a JobStep> {
        steps
            .iter()
            .find(|row| !self.holds().contains(&row.state()))
    }
}
