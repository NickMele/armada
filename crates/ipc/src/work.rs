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
use crate::event::{ChangeKind, ChangedFile};
use crate::ids::{Instant, JobId, StepId};

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

/// What a Job's worktree held when the Job stopped.
///
/// Carried on [`JobDetail`](crate::JobDetail) rather than on its own read, and
/// **that is the one place the argument runs the other way from the module
/// note above.** A history is unbounded and a patch is megabytes, so both were
/// kept off a call made on every open; a footprint is a path and a word per
/// file. Fleet asks for it only where a Job has one, which is only once the Job
/// is over, so an open of a running Job pays nothing at all — and a finished
/// Job's outcome row would otherwise draw its file count from a second read
/// that lands after the row it belongs to.
///
/// **Absent is not empty.** No footprint is a Job nothing recorded — one still
/// running, or one that finished before Fleet wrote these down. One with no
/// files is a worktree that was read and held no change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobFootprint {
    /// Every file, in the order the reading found them.
    pub files: Vec<TouchedFile>,
    /// When the reading was taken. **The instant the Job stopped**, not the
    /// instant anybody asked — which is what makes this a record rather than a
    /// reading, and what lets a surface say so.
    pub recorded_at: Instant,
    /// What each run of each step said its work would be, in the order the
    /// declarations were made.
    ///
    /// **Empty is the whole of "there is nothing to be outside of."** A Job
    /// whose steps never declared, and one that finished before Fleet kept
    /// declarations, both come back with none — and every
    /// [`TouchedFile::planned_by`] is then absent rather than empty, so a
    /// client that never reads this list still cannot mistake an unmeasured
    /// path for one that stayed inside a plan.
    #[serde(default)]
    pub plans: Vec<DeclaredPlan>,
}

/// What one run of one step promised its work would be.
///
/// **The promise, beside the record of what was done.** A footprint is the
/// Job's whole work since the branch was cut and a plan belongs to one step, so
/// the two are carried side by side rather than folded into one mark: naming
/// the steps is what lets a reader say *`implement` promised these paths and
/// the work went outside them* rather than *something drifted*.
///
/// A step that never declared has no entry here. It is silent rather than
/// counted, because a step with no plan promised nothing to be outside of.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredPlan {
    pub step_id: StepId,
    /// Which run of that step declared it, one-based. **A step may be worked
    /// twice and then declares twice**, and without this the two entries would
    /// read as one step contradicting itself.
    pub attempt: u32,
    /// When the declaration was taken, by Fleet's clock.
    pub declared_at: Instant,
    /// The paths the Drone named, in the order it named them. Each covers
    /// itself and everything beneath it, at a segment boundary.
    ///
    /// **Empty is a declaration of nothing**, which every changed path is
    /// outside of — not the same as a step that never declared, which is absent
    /// from the list above.
    pub paths: Vec<String>,
}

/// One file a finished Job touched.
///
/// **Not [`ChangedFile`], and the drift mark is the reason.** A live reading
/// carries `outside_plan` as a `bool`, because the step being watched is the
/// step that declared the plan it is measured against. A record is the Job's
/// whole work since the branch was cut, and the step holding the pen when a Job
/// stops is usually not the step that scoped it — `handoff` finishes what
/// `implement` planned. So one `bool` here would attribute one step's promise
/// to every step's work, and a `false` a client could read as "inside the plan"
/// is worse than no field at all.
///
/// [`planned_by`](TouchedFile::planned_by) is what that field became once the
/// declarations themselves were kept: it names the steps rather than asserting
/// a verdict, and it is absent — not `false`, not empty — where nothing
/// declared anything for it to be measured against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TouchedFile {
    /// Repository-relative, exactly as git spells it.
    pub path: String,
    /// The same vocabulary [`ChangedFile::change`] carries. One set of words
    /// for what happened to a file, whether it is being watched or remembered.
    pub change: ChangeKind,
    /// The steps whose declared plan covers this path, in
    /// [`JobFootprint::plans`]'s order.
    ///
    /// **Three readings, and the absent one is why this is not a `bool`.**
    /// Absent is a Job where no step declared anything: there is no plan for
    /// this path to be inside or outside of, and no measurement was made.
    /// Present and empty is a path outside every plan that was declared — the
    /// drift a finished Job could not state before. Present with steps in it is
    /// a path one of those steps promised.
    ///
    /// A step is named once however many of its runs cover the path: two
    /// attempts of one step both promising a file is one promise kept twice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planned_by: Option<Vec<StepId>>,
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
