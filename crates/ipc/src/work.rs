//! The material a person reads before deciding, and what the decision says.
//!
//! `get_evidence` and `get_diff` are what a Job's work *is*; `request_changes`
//! is the one act of the three that carries a body.
//!
//! # Two reads, not one, and neither on [`JobDetail`](crate::JobDetail)
//!
//! `get_job` is fetched on every open of a Job to draw a summary.
//! `adapter-traits`' `WorkProduct` splits the file list from the patch for a
//! measured reason — the bytes are large and most steps ask no semantic
//! question. **This is where the expensive half is finally spent**, on its own
//! route so that nothing else pays for it. Evidence is split from the diff for
//! the same reason one step down: a surface wanting only the claims would
//! otherwise fetch a megabyte to read four lines.
//!
//! # Absent, never present-and-empty
//!
//! `detail.rs` states the rule and this is where it bites hardest. A Job never
//! dispatched has no worktree, so [`JobDiff::work`] is **absent** rather than an
//! empty reading — an empty file list is a Drone that changed nothing, which is
//! a real and different answer.

use serde::{Deserialize, Serialize};

use crate::enums::EvidenceType;
use crate::event::ChangedFile;
use crate::ids::{JobId, StepId};

/// What one Job's worktree holds against the branch it was cut from.
///
/// The answer to `get_diff`, and the expensive read of the pair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDiff {
    /// The Job this is the work of, so an answer can be bound to the question —
    /// the same reason [`JobHistory`](crate::JobHistory) names its Job.
    pub job_id: JobId,
    /// The reading. **Absent where there was no worktree to read**: a Job still
    /// at the approval gate, one never dispatched, or one whose worktree has
    /// been reclaimed. Not the same as a reading that found nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work: Option<Work>,
}

/// One reading of a worktree: which files moved, and what moved inside them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Work {
    /// Every file changed since the branch was cut, in the order the reading
    /// found them. **Empty is a real answer**: the worktree opened and holds no
    /// change, which is what fails a `diff_nonempty` check.
    ///
    /// The same [`ChangedFile`] a `job.files_changed` event carries, so a review
    /// screen and a live footprint are one vocabulary rather than two.
    pub files: Vec<ChangedFile>,
    /// Whether a step has declared a plan for [`ChangedFile::outside_plan`] to
    /// mean anything. **False is "there is no plan", not "nothing drifted"** —
    /// the same distinction `JobFilesChanged::plan_declared` draws, and it is
    /// false on every Job whose Drone is no longer holding the pen, because the
    /// declaration belongs to the Drone that made it.
    pub plan_declared: bool,
    /// The unified diff, as the repository rendered it.
    ///
    /// **Absent where there is nothing in it**, which `files` being empty says
    /// in the same breath. A present-and-blank patch reads as a reading that
    /// broke, and a reading that broke is a refusal rather than a field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<String>,
}

/// Every claim a Job's Drones have submitted, step by step.
///
/// The answer to `get_evidence`, and the cheap read of the pair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobEvidence {
    pub job_id: JobId,
    /// One entry per step that has submitted evidence, in step order.
    ///
    /// **A step that submitted none is absent, not blank.** That is the shape
    /// the store already answers in, and it is what lets a reader tell a step
    /// that claimed nothing from a step that claimed nothing *yet*: the step's
    /// state on `get_job` says which. Empty is a real answer — no step has
    /// submitted anything.
    pub steps: Vec<Submitted>,
}

/// What one step's Drone claimed, and what it offered as the demonstration.
///
/// The three sentences the Agent Copy Contract defines, spelled as
/// `core_model::StepEvidence` spells them so the record and the wire are one
/// vocabulary.
///
/// **There is no `source` field, here or in the record.** A Drone marking its
/// own evidence human-attested has to be impossible on both sides of the write,
/// and a field the wire carries is a field somebody will eventually let a Drone
/// set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Submitted {
    pub step_id: StepId,
    /// What the step's workflow asked the work product to be. Recorded by
    /// Fleet from the frozen step, never named by the Drone.
    pub evidence_type: EvidenceType,
    /// What the work now does, as an observable.
    pub claimed: String,
    /// The artifact demonstrating it.
    pub shown_by: String,
    /// Everything the claim does not assert. **Absent where the submission
    /// drew no boundary**, which the record calls legitimately empty — and an
    /// empty string here would read as a limit somebody forgot to write.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_claimed: Option<String>,
}

/// What a person says when they send the work back.
///
/// A type of its own rather than [`Redirection`](crate::Redirection), which is
/// structurally the same string: a redirect steers a Drone whose step *stopped*
/// and this answers a gate the Drone is *waiting at*. Two acts with one body
/// would be one route that means whichever the caller had in mind.
///
/// Blank is refused at the Fleet boundary rather than here, for
/// [`Redirection`](crate::Redirection)'s reason: a decoded request is
/// well-formed, and a note with nothing in it is a value that cannot work —
/// a 422 and not a 400.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangesRequested {
    /// The person's own words, delivered to the Drone as a turn. **The one
    /// string on this route Fleet does not assemble** — the reviewer read the
    /// diff and the evidence, and what they want changed is not derivable from
    /// either.
    pub note: String,
}

/// What a person says when they overrule a gate that refused the work.
///
/// A type of its own rather than [`ChangesRequested`], which is structurally
/// the same string, for that type's own reason turned around: a review note
/// tells a Drone what to change, and this one goes nowhere near a Drone. **It
/// is written for the record and for whoever later asks how often the Judge was
/// wrong** — a person disagreeing is the strongest signal there is that a
/// criterion is mis-stated, and a count of overrides with no reasons beside it
/// gives the rate and never the cause.
///
/// Blank is refused at the Fleet boundary rather than here, for
/// [`ChangesRequested`]'s reason. It is refused at all because an override that
/// says nothing is how the act this route exists to keep visible becomes the
/// one somebody reaches for to make a gate stop complaining.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overruled {
    /// Why the verdict is wrong, in the person's own words. It is not sent to
    /// the Drone: the Drone did nothing wrong, and is told only that the step
    /// was accepted.
    pub reason: String,
}
