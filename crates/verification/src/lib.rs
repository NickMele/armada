//! The two verification tiers, and the gate between them.
//!
//! Mechanical Checks are pure functions over facts that already exist — an exit
//! code, a file's presence, whether a diff is empty. **Armada does not parse**:
//! deciding which lines of a test run were the failure is a Judge's question,
//! answered by reading the diff, never a runner's output.
//!
//! The Judge is a veto and not a grant. It fires on mechanical triggers, is
//! blind to the Drone, and judges whether evidence satisfies the step's intent.
//! It cannot vouch for something an exit code already contradicted, and
//! [`Verdict::but_for`] has no arm that produces an advance.
//!
//! # The property this crate exists to hold
//!
//! **A Drone claiming completion in prose advances nothing, and there is no
//! path from text to a transition.** That is not a check anywhere in this
//! crate; it is the shape of the types:
//!
//! | To reach | You need | Which you can only get from |
//! |---|---|---|
//! | [`Verdict::Advance`] | [`decide`] | An [`Accepted`] and a [`Ran`] |
//! | [`Accepted`] | [`Accepted::of`] | A [`Submission`] |
//! | [`Submission`] | [`Submission::submitted`] | The fields of the tool call |
//! | [`Ran`] | [`Ran::of`] | One observation per check the step declared |
//!
//! Nothing in that chain accepts a message, a turn, a transcript or a claim —
//! and the Judge is not on it, because a Judge answer can only take an advance
//! away.
//!
//! # What the Drone hands over, and what Fleet derives
//!
//! **A Drone hands over prose and nothing else.** Every gating artifact is
//! Fleet's own reading: the diff is in the worktree and the Check result is its
//! own run. So [`Submission`] carries no typed exit code, no file list and no
//! diff — a Drone cannot report a fact that gates its own step, because there
//! is no field to report it in.
//!
//! `shown_by` is not that field. It **names** an artifact, in prose, for a
//! person who will go and look; nothing in this crate reads it, and a Drone
//! writing "exit 0" into it has moved no check. The artifact and the report of
//! the artifact are different objects, and only the first one gates.
//!
//! # Why the diff computation lives behind this crate
//!
//! The raw diff-computation adapter method is exposed **only** here, so exactly
//! one place decides whether files changed outside their declared scope. Two
//! places deciding that is two answers.

mod gate;
mod judge;
mod mechanical;
mod outcome;
mod submission;

#[cfg(test)]
mod tests;

pub use gate::{decide, Accepted, NotWhatTheStepAsked, Verdict};
pub use judge::{Brief, Refusals, Unreadable};
pub use mechanical::{CheckFailed, ChecksOutstanding, Exit, NeverRan, Observed, Ran};
pub use outcome::{OutcomeTurn, TheBaseMoved};
pub use submission::{Claimed, NotASubmission, NotClaimed, ShownBy, Submission};
