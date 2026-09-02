//! The two verification tiers, and the gate between them.
//!
//! Mechanical Checks are pure functions over facts that already exist — an exit
//! code, a file's presence, whether a diff is empty. **Armada does not parse:**
//! which lines of a test run were the failure is a Judge's question, answered by
//! reading the diff, never a runner's output. The Judge is a veto and not a
//! grant — it fires on mechanical triggers, is blind to how the step went, and
//! [`Verdict::but_for`] has no arm producing an advance.
//!
//! **A Drone claiming completion in prose advances nothing, and there is no
//! path from text to a transition.** Not a check anywhere here — the types:
//!
//! | To reach | You need | Which you can only get from |
//! |---|---|---|
//! | [`Verdict::Advance`] | [`decide`] | An [`Accepted`] and a [`Ran`] |
//! | [`Accepted`] | [`Accepted::of`] | A [`Submission`] |
//! | [`Submission`] | [`Submission::submitted`] | The fields of the tool call |
//! | [`Ran`] | [`Ran::of`] | One observation per check the step declared |
//!
//! Nothing in that chain accepts a message, a turn, a transcript or a claim.
//! [`submission`](mod@submission) holds the fields that do not exist, and
//! [`product`](mod@product) the one step type whose writing *is* the
//! deliverable. **The raw diff-computation adapter method is exposed only
//! here**, because two places deciding whether files changed outside their
//! declared scope is two answers — [`scope`](mod@scope) is the one place.

mod answered;
mod converging;
mod drift;
mod gaming;
mod gate;
mod judge;
mod located;
mod mechanical;
mod outcome;
mod product;
mod quoted;
mod request;
mod scope;
mod submission;

#[cfg(test)]
mod tests;

pub use answered::{Answered, Printed};
pub use converging::{Convergence, ConvergenceBrief, NotConverging};
pub use drift::{drift_criterion, DECLARED_PLAN_DRIFT};
pub use gaming::{in_the_diff, judged_patterns, Baseline, Flagged, GamingBrief};
pub use gate::{decide, Accepted, NotWhatTheStepAsked, Verdict};
pub use judge::{field, Brief, Refusals, Unreadable};
pub use mechanical::{
    Artifact, CheckFailed, ChecksOutstanding, Exit, NeverRan, Observed, Ran, EVIDENCE_SCOPE,
};
pub use outcome::{OutcomeTurn, TheBaseMoved, Verified};
pub use product::{
    Delivered, NothingToJudge, Product, Reference, TooBigToJudge, Written, A_DELIVERABLE,
};
pub use request::Request;
pub use scope::{drifted, InScope, OutsideScope};
pub use submission::{Claimed, NotASubmission, NotClaimed, ShownBy, Submission};
