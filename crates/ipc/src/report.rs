//! What a person says went wrong, and the Job's own record attached to it.
//!
//! **The record is the evidence and the sentence is the finding.** Everything
//! else this crate carries answers *did this fail* — a status, a trigger, a
//! verdict, a Check run — and all of it is silent on the question that decides
//! whether the verification tier earns its place: *did it fail correctly*. A
//! Job that failed perfectly by its own lights and was wrong looks, on every
//! field of [`JobDetail`](crate::JobDetail), exactly like one that failed
//! rightly. [`FileReport::said`] is what closes that, and it is the only part
//! of a report that does not already exist somewhere.
//!
//! **A closed set sits beside the sentence because a count may not read
//! prose.** The first override in this repository carries the reason `probe`,
//! sent to find out whether the route was served, and `job_events` is
//! append-only so it says that for ever. A required non-blank field accepts
//! that as happily as a considered one. [`Claim`] is what a count reads; the
//! sentence stays what a person reads, weak and visible where it is weak.
//!
//! **Two of the three claims are not symmetric.** A wrong refusal stops work
//! that was right, loudly, and somebody notices. A wrong pass lets wrong work
//! through and nothing surfaces it at all. One `judge_was_wrong` would average
//! two counts that mean different things.

use serde::{Deserialize, Serialize};

use crate::ids::{CriterionId, Instant, JobId, StepId};

/// A filed report's identifier, minted by Fleet like every other record's.
///
/// Not a [`wire_id`](crate::ids), because there is no `core_model::ReportId`
/// under it: a report is not part of the Job machine and nothing in the domain
/// carries one. It is a ULID all the same, so filing order is the sort.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReportId(String);

impl ReportId {
    /// Carry an id Fleet minted. Nothing here mints one.
    pub fn carried(value: impl Into<String>) -> Self {
        ReportId(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What the person says the machine got wrong.
///
/// **The one vocabulary in this crate that is not a spelling of a domain
/// value**, and the module doc is why: nothing in `core-model` decides this,
/// because it is not a state anything is in — it is what a person concluded
/// after reading work a machine had already ruled on. So the `match` below is
/// this crate being the authority rather than restating one, which is the
/// distinction `lib.rs` draws about second vocabularies.
///
/// `store` keeps the wire spelling as the text it arrived as and never reads it
/// back into an enum: nothing there branches on it, and the count groups on the
/// column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Claim {
    /// The Judge refused work that was right. **The loud failure** — it stops
    /// work, so somebody is there to disagree.
    WronglyRefused,
    /// Something wrong went through. **The quiet failure**, and the one nothing
    /// in the system surfaces on its own.
    WronglyPassed,
    /// Armada itself did the wrong thing, with no verdict involved: a Judge
    /// handed an empty patch, a dry run that reported work it had not done.
    /// Neither of the two above, and the reason this is a report rather than a
    /// mark on a verdict.
    ArmadaMisbehaved,
}

impl Claim {
    /// Every variant, in the order a picker offers them.
    pub const ALL: &'static [Claim] = &[
        Claim::WronglyRefused,
        Claim::WronglyPassed,
        Claim::ArmadaMisbehaved,
    ];

    /// The wire value, which is also the value stored and grouped on.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Claim::WronglyRefused => "wrongly_refused",
            Claim::WronglyPassed => "wrongly_passed",
            Claim::ArmadaMisbehaved => "armada_misbehaved",
        }
    }

    /// Read a stored value back. `None` where nothing spells it.
    pub fn from_wire(value: &str) -> Option<Claim> {
        Claim::ALL
            .iter()
            .copied()
            .find(|claim| claim.as_wire() == value)
    }

    /// Whether this claim disputes a verdict the Judge gave.
    ///
    /// **What the calibration counts, and the only question anything asks of a
    /// claim.** A method rather than a `!= ArmadaMisbehaved` at each call site,
    /// so a fourth value added here cannot default into a rate about the Judge.
    pub fn disputes_a_verdict(&self) -> bool {
        matches!(self, Claim::WronglyRefused | Claim::WronglyPassed)
    }
}

/// Which half of the machine filed it.
///
/// **One store, and this is the column** — v1 settled it and the reason holds:
/// a person triaging on a Monday morning does not care which half of the
/// machine noticed, and a second store would be a second id space, a second
/// listing and a second promotion path.
///
/// Only [`ReportOrigin::Human`] is written today. Nothing in this build files a
/// report on Armada's behalf, and the variant that says so is here rather than
/// in a later migration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportOrigin {
    /// A person pressed the button and wrote the sentence.
    Human,
    /// Armada noticed something about itself. Nothing writes this yet.
    Armada,
}

impl ReportOrigin {
    pub const ALL: &'static [ReportOrigin] = &[ReportOrigin::Human, ReportOrigin::Armada];

    pub fn as_wire(&self) -> &'static str {
        match self {
            ReportOrigin::Human => "human",
            ReportOrigin::Armada => "armada",
        }
    }

    pub fn from_wire(value: &str) -> Option<ReportOrigin> {
        ReportOrigin::ALL
            .iter()
            .copied()
            .find(|origin| origin.as_wire() == value)
    }
}

/// What a person sends to file a report on a Job.
///
/// **The Job is the path segment and not a field here**, like every other act
/// on a Job. What crosses is the part Fleet cannot know: the claim, the
/// sentence, and which verdict is being disputed where one is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileReport {
    /// What the person says was wrong. A closed set, so the count never has to
    /// read prose.
    pub claim: Claim,
    /// **Why they think it was wrong, in their own words. Required.**
    ///
    /// Fleet answers 422 on a blank one, for the reason `override_verdict`
    /// does: a report with the record attached and nothing said is the bundle
    /// without the finding, and the record was already there before anybody
    /// pressed anything.
    pub said: String,
    /// The step whose verdict is disputed. Absent where the report is about the
    /// Job rather than about one criterion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<StepId>,
    /// The criterion whose verdict is disputed — **the scope #117 asks for**,
    /// because a Judge that was right twice and wrong once in one gate is
    /// exactly the shape a calibration count exists to make legible.
    ///
    /// Sent with `step_id` and never without it: a criterion id is unique
    /// inside a step, and a criterion naming no step names two verdicts on a
    /// retried Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criterion_id: Option<CriterionId>,
}

/// One report, as it was filed and as it reads afterwards.
///
/// **It is readable without the Job.** `record` is the Job's own evidence
/// rendered at filing time and stored, rather than a join to rows `armada
/// clean` takes away — so a report about a Job that has since been cleaned up
/// is still a report somebody can act on, which is the case it is most often
/// needed in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub id: ReportId,
    pub filed_at: Instant,
    pub origin: ReportOrigin,
    pub claim: Claim,
    /// The Job it is about. **May name a Job that no longer exists**, which is
    /// the point: the id is how a person joins back to a history where there
    /// still is one, and `record` is what does not depend on it.
    pub job_id: JobId,
    /// What the Job was called. Copied at filing time for the same reason.
    pub job_title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<StepId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criterion_id: Option<CriterionId>,
    /// The person's own words. The finding.
    pub said: String,
    /// The Job's record as it stood when the report was filed, rendered as the
    /// body of an issue. The evidence, and what a Drone would be given as facts
    /// — **a Drone cannot reach the issue tracker**, so the bundle travels either
    /// way and this is it, solved once.
    pub record: String,
}

/// Every report, and the counts they are read beside.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportList {
    /// Newest first. **The bodies travel with the list**, rather than on a
    /// second route per report: a report is the rare artifact — one per verdict
    /// somebody disagreed with — and the body is the payload the rest is
    /// context for, so a route that existed to defer it would defer the part
    /// every reader wants.
    pub reports: Vec<Report>,
    pub calibration: Calibration,
}

/// What is known about whether the Judge has been right.
///
/// **Counts, and deliberately not a rate.** Dividing disputes by refusals would
/// produce a number whose denominator counts every Job nobody read, and #117
/// names that failure directly: an unread Job is not a pass. What these four
/// say together is how often the Judge refuses and how often a person who
/// actually read the work said it was wrong to — and the gap between them is
/// unmeasured rather than divided away.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calibration {
    /// Every criterion the Judge has answered `not_met`, over every Job in the
    /// store, on every attempt.
    pub refusals_recorded: u32,
    /// Reports claiming [`Claim::WronglyRefused`].
    pub refusals_disputed: u32,
    /// Reports claiming [`Claim::WronglyPassed`]. **Not the other half of the
    /// same number** — a wrong pass is refused by nothing, so it has no
    /// recorded population to be counted against.
    pub passes_disputed: u32,
    /// Every report filed, whatever it claims.
    pub reports_filed: u32,
}
