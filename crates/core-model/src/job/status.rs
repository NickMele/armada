//! Where a Job is, and where one step of its frozen WorkflowDef is.
//!
//! [`JobStatus`] is the `status` column on `jobs` and the outer half of the
//! two-level machine; [`StepState`] is the `state` column on `job_steps` and
//! the inner half. The outer machine gates whether the inner one moves at all.
//!
//! # The set is the registry's, not this file's
//!
//! The twelve variants are the twelve keys of `domain/job-statuses.toml`,
//! spelled exactly as that file spells them, because the key *is* the wire
//! value — a rule comparing the two needs a set lookup and no mapping in
//! between. Issue #92 is that rule; [`JobStatus::ALL`] and
//! [`JobStatus::as_wire`] are what it will read. The same holds for
//! [`StepState`] against `domain/step-states.toml`.
//!
//! # A status carries no payload
//!
//! `job-fields.toml` types `status` as one enum column and puts the qualifying
//! reason in `job_events`, "with its reason, actor and time". So the reason
//! travels on the transition — see [`Target`](crate::Target) — and never on the
//! status. A status that carried its reason could not be one column, and the
//! four statuses that store a reason would each need a variant per value.

use crate::job::transition::EDGES;

/// Where a Job is. Twelve, from `domain/job-statuses.toml`.
///
/// Ordered as the registry orders its tables, alphabetically by wire value, so
/// that reading the two side by side is a line-for-line comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobStatus {
    /// A person must approve the dispatch before anything runs. The entry
    /// status of a top-level Job.
    AwaitingApproval,
    /// The work landed; a criterion needs an action outside Armada. The reason
    /// is the criterion ids owed — a reference, not an enum.
    AwaitingAttestation,
    /// A human advance gate is open. The Drone keeps its PID, worktree and
    /// session across it, and the liveness clock is suspended.
    AwaitingReview,
    /// Terminal. Retries exhausted, a failed Check at M1, or a person accepting
    /// the failure as the outcome.
    CompletedFailed,
    /// Terminal. Last step advanced, every criterion verified. Immutable once
    /// reached: a merge that later breaks main produces a new Job.
    CompletedSuccess,
    /// Fleet paused the Job and a person must decide. The Drone is alive and
    /// idle, and the worktree and port span are held as-is. The reason is an
    /// [`EscalationTrigger`](crate::EscalationTrigger).
    Escalated,
    /// Terminal. Cleared from the Board — an operator act, carrying no verdict.
    /// Reachable from every non-terminal status.
    Killed,
    /// A person is working it: the Drone is gone, the worktree belongs to the
    /// engineer, and Fleet must not reclaim it. Distinguished from
    /// [`Escalated`](Self::Escalated) by who holds the worktree.
    Piloted,
    /// Approved, waiting for a Drone — on headroom, or on a dependency. Its
    /// reason is computed at read time and never stored, because a held port
    /// span never self-clears. The entry status of a sub-dispatched Job.
    Queued,
    /// Terminal. A verdict on the work — approval denied, or `reject` at a
    /// human gate.
    Rejected,
    /// The work is being done. This does not always imply a Drone: returning
    /// from [`Piloted`](Self::Piloted) for verification leaves `assigned_drone`
    /// null.
    Running,
    /// Terminal. The work landed outside the Job; the record has nothing left
    /// to say. A dependent unblocks and surfaces rather than escalating.
    Superseded,
}

impl JobStatus {
    /// Every variant, in registry order. What a set-comparison rule reads.
    pub const ALL: &'static [JobStatus] = &[
        JobStatus::AwaitingApproval,
        JobStatus::AwaitingAttestation,
        JobStatus::AwaitingReview,
        JobStatus::CompletedFailed,
        JobStatus::CompletedSuccess,
        JobStatus::Escalated,
        JobStatus::Killed,
        JobStatus::Piloted,
        JobStatus::Queued,
        JobStatus::Rejected,
        JobStatus::Running,
        JobStatus::Superseded,
    ];

    /// The wire value, which is also the registry key. Never a display verb —
    /// what a person reads comes from `domain/enum-verbs.toml`, joined by
    /// codegen, so no surface holds a verb list of its own.
    pub fn as_wire(&self) -> &'static str {
        match self {
            JobStatus::AwaitingApproval => "awaiting_approval",
            JobStatus::AwaitingAttestation => "awaiting_attestation",
            JobStatus::AwaitingReview => "awaiting_review",
            JobStatus::CompletedFailed => "completed_failed",
            JobStatus::CompletedSuccess => "completed_success",
            JobStatus::Escalated => "escalated",
            JobStatus::Killed => "killed",
            JobStatus::Piloted => "piloted",
            JobStatus::Queued => "queued",
            JobStatus::Rejected => "rejected",
            JobStatus::Running => "running",
            JobStatus::Superseded => "superseded",
        }
    }

    /// Read a stored column back. `None` where the value is not one of the
    /// twelve, which is a row written by something that did not share this
    /// enum — the caller decides what that means rather than getting a default.
    pub fn from_wire(value: &str) -> Option<JobStatus> {
        JobStatus::ALL
            .iter()
            .copied()
            .find(|s| s.as_wire() == value)
    }

    /// Whether the Job is over here.
    ///
    /// Declared, not derived, because the registry declares it. That it agrees
    /// with the edge table — no terminal has an outbound edge — is a test, in
    /// the spirit of `transitions_out` being carried beside the edges that
    /// imply it.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobStatus::CompletedFailed
                | JobStatus::CompletedSuccess
                | JobStatus::Killed
                | JobStatus::Rejected
                | JobStatus::Superseded
        )
    }

    /// The statuses this one may be left for, in edge-table order.
    pub fn transitions_out(&self) -> impl Iterator<Item = JobStatus> + '_ {
        EDGES.iter().filter(move |e| e.from == *self).map(|e| e.to)
    }

    /// The statuses this one may be arrived at from, in edge-table order.
    pub fn transitions_in(&self) -> impl Iterator<Item = JobStatus> + '_ {
        EDGES.iter().filter(move |e| e.to == *self).map(|e| e.from)
    }
}

/// Every status but `completed_success`, which is guarded against every step
/// state except `advanced`.
///
/// Written out rather than filtered from [`JobStatus::ALL`], because
/// `seen_under` returns a `&'static [JobStatus]` and the gate reads these arms
/// as text. A `const fn` filter would answer correctly and be unreadable by the
/// rule that keeps it honest.
const NOT_UNDER_COMPLETED_SUCCESS: &[JobStatus] = &[
    JobStatus::AwaitingApproval,
    JobStatus::AwaitingAttestation,
    JobStatus::AwaitingReview,
    JobStatus::CompletedFailed,
    JobStatus::Escalated,
    JobStatus::Killed,
    JobStatus::Piloted,
    JobStatus::Queued,
    JobStatus::Rejected,
    JobStatus::Running,
    JobStatus::Superseded,
];

/// Where one step of a Job's frozen WorkflowDef is. Six, from
/// `domain/step-states.toml`.
///
/// Stored explicitly on the `job_steps` row rather than inferred from position
/// relative to the current step: position-inference breaks under a loop
/// workflow, where a step can have advanced and then be re-entered.
///
/// Colour is never stored. There is no `display_state`, no hue — the schema
/// owes a surface the state named precisely enough to map onto a mark, and
/// which mark is the Design System's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StepState {
    /// The step passed its advance gate.
    Advanced,
    /// The step's advance gate is a human gate, and it is open.
    AwaitingHuman,
    /// Written at Job creation for every step of the frozen WorkflowDef.
    NotStarted,
    /// The step failed and is being reattempted inside its retry budget.
    Retrying,
    /// The step is being worked.
    Running,
    /// Retries spent. Neither retrying nor waiting on a person — folding it
    /// into either would make a designed human gate and a dead stop render
    /// alike.
    Stopped,
}

impl StepState {
    /// Every variant, in registry order.
    pub const ALL: &'static [StepState] = &[
        StepState::Advanced,
        StepState::AwaitingHuman,
        StepState::NotStarted,
        StepState::Retrying,
        StepState::Running,
        StepState::Stopped,
    ];

    /// The wire value, which is also the registry key.
    pub fn as_wire(&self) -> &'static str {
        match self {
            StepState::Advanced => "advanced",
            StepState::AwaitingHuman => "awaiting_human",
            StepState::NotStarted => "not_started",
            StepState::Retrying => "retrying",
            StepState::Running => "running",
            StepState::Stopped => "stopped",
        }
    }

    /// Read a stored column back. `None` where the value is not one of the six.
    pub fn from_wire(value: &str) -> Option<StepState> {
        StepState::ALL
            .iter()
            .copied()
            .find(|s| s.as_wire() == value)
    }

    /// The Job statuses a step in this state is observed beneath, as
    /// `domain/step-states.toml` declares them. A hand transcription, checked
    /// against that file by the gate.
    ///
    /// **Only `advanced` answers with every status, and that is the machine.**
    /// A status change looks at a step only where the edge carries a
    /// [`Guard`](crate::Guard); across every unguarded edge a frozen step is
    /// carried holding what it held, which is why three of these answer with
    /// almost all of them. Four answered far more narrowly until issue #184,
    /// and `escalated` was the one that made it concrete: `stopped` claimed it
    /// alone, while a Job escalated on `stalled` arrives holding a step that is
    /// `running`.
    ///
    /// **`completed_success` is the exception, and it is the guard.** Every
    /// edge arriving there is guarded on `every_step_advanced`, so no other
    /// state is carried across one and no other state answers with it. That is
    /// issue #189, and it is what a row narrowing on something other than a
    /// widening looks like.
    ///
    /// The two narrowest answers are the two states nothing reaches yet:
    /// `awaiting_human` has no [`StepTarget`](crate::StepTarget) and `retrying`
    /// has no retry budget, so each is where its design puts it rather than
    /// where a walk found it.
    pub fn seen_under(&self) -> &'static [JobStatus] {
        match self {
            StepState::Advanced => JobStatus::ALL,
            StepState::AwaitingHuman => &[JobStatus::AwaitingReview],
            StepState::NotStarted => NOT_UNDER_COMPLETED_SUCCESS,
            StepState::Retrying => &[JobStatus::Running],
            StepState::Running => NOT_UNDER_COMPLETED_SUCCESS,
            StepState::Stopped => NOT_UNDER_COMPLETED_SUCCESS,
        }
    }
}
