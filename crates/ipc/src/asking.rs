//! A Judge criterion that refused and a person is being asked about, rather
//! than the step stopping over it. `docs/concepts/judge.md`'s asking design.
//!
//! # It is not a status, and neither registry is touched
//!
//! Same argument as [`crate::CommandInFlight`]'s: the step is `awaiting_human`
//! and the Job `awaiting_review` — the states a `human_always` review gate
//! already reaches — so a question rides beside them rather than adding a
//! seventh state or status. What tells the two apart on a screen is this
//! field being present, not a new value either registry would have to grow.
//!
//! # One question at a time
//!
//! Only one criterion is ever asked about per pass over a step — Fleet asks
//! about the first ask-eligible refusal and records the rest — so `job_id`
//! alone is enough to hold it, and this type carries no id of its own beyond
//! [`JudgeQuestion::criterion_id`].

use serde::{Deserialize, Serialize};

use crate::ids::{CriterionId, Instant, StepId};

/// The question a person is being asked, right now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgeQuestion {
    /// Which step this question is about. A Job runs one step at a time.
    pub step_id: StepId,
    /// The refused criterion. **The join to [`crate::StepDetail::judged`]**,
    /// where the same id names the row this question is asking about.
    pub criterion_id: CriterionId,
    /// The plain question the criterion asked. Neither `expected`, `produced`
    /// nor `consequence` restate it, so a person reading the card needs this
    /// to know what was actually being checked.
    pub question: String,
    /// What should be seen, returned or recorded if the work is right.
    pub expected: String,
    /// What will be seen instead.
    pub produced: String,
    /// What that difference does to whoever consumes it. The field a person
    /// triages on.
    pub consequence: String,
    /// When the refusal was raised, by Fleet's clock.
    pub asked_at: Instant,
    /// Where the whole brief this verdict answers was written, relative to
    /// the repository root. `None` where the brief was not kept — see
    /// [`core_model::Judgment::brief_path`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brief_path: Option<String>,
}

/// A person's answer to a [`JudgeQuestion`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgeAnswer {
    /// The refusal stands. The step fails exactly as it does where the
    /// criterion is marked `refuse`.
    Agree,
    /// The step advances. The next Job's gate asks about this criterion
    /// again.
    DisagreeOnce,
    /// The step advances, and this repository stops being asked about this
    /// criterion again — `docs/concepts/judge.md`'s `#191` closure.
    DisagreeAlways,
}

/// The request half of `answer_judge`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgeAnswered {
    pub answer: JudgeAnswer,
    /// Never required. Rides along for whoever reads the record later; Fleet
    /// asks nothing further of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
