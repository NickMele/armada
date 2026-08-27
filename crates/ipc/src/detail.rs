//! One Job, whole. The answer to `get_job`.
//!
//! [`JobDetail::job`] nests the same [`JobSummary`] the Board row is built
//! from, so the two cannot disagree and a field added to the summary reaches
//! the detail with no second line of code. What is added here is what a list
//! leaves behind.
//!
//! # `facts` crosses here and not on the summary, and that is a decision
//!
//! The summary redacts it as the likeliest place a secret lands, which is an
//! argument about a Board drawn for every Job at once. A detail view is one Job
//! somebody opened, and the brief is most of what the screen is for —
//! `get_job` is named in the summary's own redaction table as where the Job is
//! returned in full.
//!
//! # Absent, never present-and-null
//!
//! Every optional field is skipped when it has no value, the rule
//! `docs/concepts/log-envelope.md` states for its own envelope: a client that
//! receives `branch: null` cannot tell "no worktree yet" from "Fleet forgot".
//! Evidence and a pointer to `.armada/logs/` are absent because nothing
//! produces either, and a field that is always empty reads as working.

use serde::{Deserialize, Serialize};

use crate::checks::{CheckRun, DeclaredCheck};
use crate::enums::{CriterionSource, DependencyDirection, JudgeVerdict, StepState};
use crate::ids::{CriterionId, Instant, JobId, StepId};
use crate::job::{JobSummary, Subject};

/// What Fleet knows about one step beyond its `job_steps` row.
///
/// **The declaration is the workflow's and the result is the store's.** The
/// frozen `job_steps` rows carry neither, and a column for either would be a
/// second authority for a fact that already has one.
///
/// Built by Fleet, which is the only side holding both the workflow and the
/// store. It is not a wire type and is never serialised.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepFacts {
    pub step_id: StepId,
    /// What the workflow calls this step. `None` where Fleet cannot say, on
    /// the same grounds as `declares`.
    pub label: Option<String>,
    /// The Checks the workflow declares for this step.
    ///
    /// **`None` is "Fleet cannot say", not "none declared"** — the Job named a
    /// workflow this Fleet does not hold, or holds one that no longer declares
    /// the step. Empty is the ungated step, which is the common case.
    pub declares: Option<Vec<DeclaredCheck>>,
    /// What each declared Check did, in the step's order. Empty until the gate
    /// has run them.
    pub ran: Vec<CheckRun>,
    /// What the Judge answered, in the order asked. Empty on a step that asks
    /// nothing, which is most of them.
    pub judged: Vec<Judged>,
}

/// One Job, whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDetail {
    /// The Board row, unchanged. Carries id, title, status, when the Job was
    /// created, its branch, reason, origin, urgency, model, current step, and
    /// `redispatched_from`.
    pub job: JobSummary,
    /// When the Job was created. **Also on [`JobSummary`], and kept here
    /// anyway**: removing a field an old peer already reads is a major bump,
    /// and both are built from the one record so they cannot disagree. The next
    /// major bump is where this goes.
    pub created_at: Instant,
    /// The branch the Job's worktree is on. On [`JobSummary`] too, and kept
    /// here for the same reason `created_at` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// One entry per step of the frozen WorkflowDef, in the order they were
    /// written. The count is the list's length; nothing states it separately.
    pub steps: Vec<StepDetail>,
    /// The requester's words, with the id a Judge citation references.
    pub acceptance_criteria: Vec<Criterion>,
    /// Context the Job was given. Absent where none was, rather than `""`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facts: Option<String>,
    /// **Null is not empty.** Absent is scope not yet determined; present and
    /// empty is determined to write nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_targets: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<Subject>,
    /// The DAG edges this Job sits on. Empty until something writes one.
    pub dependencies: Vec<Dependency>,
}

impl JobDetail {
    /// A Job in full, plus the reason its last recorded transition carried and
    /// what Fleet knows about its steps.
    ///
    /// Both are arguments for the same reason [`JobSummary::of`] takes one:
    /// neither is a field of `core_model::Job`. The reason is in the log; a
    /// step's Checks are the workflow's and the store's.
    pub fn of(
        job: &core_model::Job,
        reason: Option<&core_model::TransitionReason>,
        steps: &[StepFacts],
    ) -> JobDetail {
        JobDetail {
            job: JobSummary::of(job, reason),
            created_at: job.created_at().into(),
            branch: job.branch().map(|branch| branch.as_str().to_string()),
            steps: job
                .steps()
                .iter()
                .map(|step| StepDetail::of(step, facts_for(steps, step.step_id())))
                .collect(),
            acceptance_criteria: job
                .acceptance_criteria()
                .iter()
                .map(Criterion::from)
                .collect(),
            facts: Some(job.facts().as_str())
                .filter(|text| !text.is_empty())
                .map(str::to_string),
            write_targets: job.write_targets().map(|targets| {
                targets
                    .paths()
                    .iter()
                    .map(|path| path.as_str().to_string())
                    .collect()
            }),
            subject: job.subject().map(Subject::from),
            dependencies: job.dependencies().iter().map(Dependency::from).collect(),
        }
    }
}

/// One `job_steps` row: which step, where in the order, and where it got to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepDetail {
    pub step_id: StepId,
    /// What a person reads — `Plan the change`, not `plan`.
    ///
    /// **Never absent and never blank**: where the workflow declares no label,
    /// or Fleet cannot say which workflow this is, the id stands in. A client
    /// that had to choose would make that choice differently in each place it
    /// draws a step, and the id is already on the row above.
    pub label: String,
    /// Position in the frozen WorkflowDef, so a rail draws past, current and
    /// future without reading the workflow.
    pub ordinal: u32,
    pub state: StepState,
    /// The Checks this step declares, in the order the workflow declares them.
    ///
    /// **Empty means the step is ungated; absent means Fleet cannot say.**
    /// Those are different sentences and a reader must not have to guess which
    /// one a gap is — which is the whole reason this field exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<DeclaredCheck>>,
    /// What each declared Check did, in the same order. Empty until the gate
    /// has run them, which is not the same as declaring none.
    pub check_runs: Vec<CheckRun>,
    /// Absent until a gate has ruled on the step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_verdict: Option<Verdict>,
    /// Every criterion the Judge answered on this step, in the order asked.
    ///
    /// **This is where a refusal's citation arrives**, and it is the whole
    /// reason a refusal escalates rather than ending the Job: the trigger says
    /// the gate stopped, and only these say what was wrong with the work.
    /// Empty on a step that asks nothing and on a step the Judge never reached.
    pub judged: Vec<Judged>,
    /// When the step was entered. Stamped at creation and moved on entering
    /// `running`, so `entered_at` to `updated_at` is how long the step took.
    pub entered_at: Instant,
    pub updated_at: Instant,
}

impl StepDetail {
    fn of(step: &core_model::JobStep, facts: Option<&StepFacts>) -> StepDetail {
        StepDetail {
            step_id: step.step_id().into(),
            label: facts
                .and_then(|facts| facts.label.clone())
                .filter(|label| !label.trim().is_empty())
                .unwrap_or_else(|| step.step_id().as_str().to_string()),
            ordinal: step.ordinal(),
            state: step.state().into(),
            checks: facts.and_then(|facts| facts.declares.clone()),
            check_runs: facts.map(|facts| facts.ran.clone()).unwrap_or_default(),
            last_verdict: step.last_verdict().map(Verdict::of),
            judged: facts.map(|facts| facts.judged.clone()).unwrap_or_default(),
            entered_at: step.entered_at().into(),
            updated_at: step.updated_at().into(),
        }
    }
}

/// One criterion the Judge answered, as a person reads it.
///
/// **A refusal owes three lines and a no-objection owes none**, which is why
/// the three below are optional rather than blank: there is nothing to cite
/// where nothing was refused, and an empty string would read as a citation
/// somebody lost.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Judged {
    /// Which criterion was asked. What a citation points at, and what stays
    /// meaningful at any panel size.
    pub criterion_id: CriterionId,
    pub verdict: JudgeVerdict,
    /// What should be seen if the work were right.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// What is seen instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced: Option<String>,
    /// What that difference does to whoever consumes it. **The line a person
    /// triages on.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consequence: Option<String>,
}

impl From<&core_model::Judgment> for Judged {
    fn from(judgment: &core_model::Judgment) -> Judged {
        Judged {
            criterion_id: (&judgment.criterion_id).into(),
            verdict: judgment.verdict.into(),
            expected: judgment.expected.clone(),
            produced: judgment.produced.clone(),
            consequence: judgment.consequence.clone(),
        }
    }
}

/// What Fleet handed in for one step, where it handed in anything.
///
/// A linear scan, because a workflow has a handful of steps and a map would be
/// a second index over a list that is already in order.
fn facts_for<'a>(steps: &'a [StepFacts], step_id: &core_model::StepId) -> Option<&'a StepFacts> {
    steps
        .iter()
        .find(|facts| facts.step_id.as_str() == step_id.as_str())
}

/// The last ruling against a step.
///
/// Two fields rather than one string, for the reason
/// [`Reason`](crate::Reason) has two: `failed` carries the trigger that
/// failed it, and the other two carry nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    /// `passed`, `failed` or `not_reached`, spelled as the domain spells it.
    pub named: String,
    /// The escalation trigger a failure carried. Absent on the other two.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
}

impl Verdict {
    pub fn of(verdict: core_model::StepVerdict) -> Verdict {
        Verdict {
            named: verdict.as_wire().to_string(),
            trigger: match verdict {
                core_model::StepVerdict::Failed(trigger) => Some(trigger.as_wire().to_string()),
                _ => None,
            },
        }
    }
}

/// One acceptance criterion, with the id a Judge citation references.
///
/// The read side of [`ProposedCriterion`](crate::ProposedCriterion), which
/// carries no id because the id is minted with the Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Criterion {
    pub criterion_id: CriterionId,
    pub text: String,
    pub source: CriterionSource,
}

impl From<&core_model::AcceptanceCriterion> for Criterion {
    fn from(criterion: &core_model::AcceptanceCriterion) -> Criterion {
        Criterion {
            criterion_id: (&criterion.criterion_id).into(),
            text: criterion.text.clone(),
            source: criterion.source.into(),
        }
    }
}

/// One DAG edge, sequencing peer Jobs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub direction: DependencyDirection,
    pub peer: JobId,
}

impl From<&core_model::DependencyEdge> for Dependency {
    fn from(edge: &core_model::DependencyEdge) -> Dependency {
        Dependency {
            direction: edge.direction.into(),
            peer: (&edge.peer).into(),
        }
    }
}
