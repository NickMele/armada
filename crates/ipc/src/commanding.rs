//! A command a Drone reached for and was not given, and what a person answers.
//!
//! # Two paths, one set of answers
//!
//! A Job set to [`WhenBlocked::AskMe`] holds its Drone inside the permission
//! call and a person answers while it waits. A Job at
//! [`WhenBlocked::RefuseAndHold`] refuses the call at once, stops at
//! `blocked_by_policy`, and a person answers the [`Refusal`](crate::Refusal)
//! row instead. `answer_command` takes both, and the call id says which.
//!
//! # Neither enum wraps a `core-model` value
//!
//! Unlike [`crate::enums`], and for [`Settled`](crate::Settled)'s reason: these
//! are this seam's own closed sets with no registry behind them, so the
//! spelling is declared here once. Bridge matches on both to choose which
//! controls to draw, so a third value in either is a major bump.

use serde::{Deserialize, Serialize};

/// What a Job does when its Drone reaches for a command it was not given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhenBlocked {
    /// Refuse the call and stop the Job at `blocked_by_policy`, for a person to
    /// answer later. **Where every Job starts**, because it asks nobody to be
    /// watching.
    RefuseAndHold,
    /// Hold the Drone inside the call and ask a person now. The step stays
    /// `running` while it waits, and an allow lets it carry on in the same
    /// session.
    AskMe,
}

/// What a person may answer about one refused command.
///
/// **Offered, never assumed.** Each place a person answers carries the subset
/// Fleet will take for that command, and an answer outside it is a 409 — so a
/// command nobody may allow here is offered nothing, and says why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandAnswer {
    /// Run it, and let this Job run it again without asking. Recorded on the
    /// Job and nowhere else.
    AllowForJob,
    /// Everything [`AllowForJob`](CommandAnswer::AllowForJob) does, and the
    /// command written into `armada.yml` under `commands` as its own commit on
    /// the Job's branch — so it reaches every Job after this one once the
    /// branch lands.
    AlwaysAllow,
    /// Do not run it. The Drone is told, and goes on without it.
    Reject,
}
