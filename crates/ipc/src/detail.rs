//! One Job, whole. The answer to `get_job`.
//!
//! [`JobDetail::job`] nests the same [`JobSummary`] the Board row is built
//! from, so the two cannot disagree and a field added to the summary reaches
//! the detail for free. What is added here is what a list leaves behind.
//!
//! # `facts` crosses here and not on the summary, and that is a decision
//!
//! The summary redacts it as the likeliest place a secret lands, which is an
//! argument about a Board drawn for every Job at once. A detail view is one Job
//! somebody opened, and the brief is most of what the screen is for —
//! `get_job` is named in the summary's own redaction table as where the Job is
//! returned in full. [`RedirectWaiting`] is the only other free text here, on
//! that same argument and its own.
//!
//! # Absent, never present-and-null
//!
//! Every optional field is skipped when it has no value, the rule
//! `docs/concepts/log-envelope.md` states for its own envelope: a client that
//! receives `branch: null` cannot tell "no worktree yet" from "Fleet forgot".
//! Evidence and a pointer to `.armada/logs/` are absent because nothing
//! produces either, and a field that is always empty reads as working.
//!
//! Two of a step's four declared facts are read off the Job's own frozen
//! workflow rather than handed in — [`StepFacts`] says which, and why.

use serde::{Deserialize, Serialize};

use crate::attempt::StepAttempt;
use crate::checks::{CheckRun, DeclaredCheck, DeclaredJudge};
use crate::enums::{
    AdvanceGate, CriterionSource, DependencyDirection, JudgeVerdict, Recourse, StepState,
};
use crate::ids::{CriterionId, Instant, JobId, StepId};
use crate::job::{JobSummary, Subject};
use crate::overlap::ScopeOverlap;
use crate::waiting::{QuestionInFlight, RedirectInFlight, RedirectWaiting};
use crate::work::JobFootprint;

/// What Fleet knows about one step beyond its `job_steps` row.
///
/// **The declaration is the workflow's and the result is the store's.** The
/// frozen `job_steps` rows carry neither, and a column for either would be a
/// second authority for a fact that already has one.
///
/// Built by Fleet, which is the only side holding both the workflow and the
/// store. It is not a wire type and is never serialised.
///
/// **It carries two of a step's declared facts and not the other two.**
/// [`StepDetail::advance_gate`] and [`StepDetail::judge_checks`] do not come
/// through here: [`JobDetail::of`] is given the `core_model::Job`, a Job
/// carries the workflow it froze, and that is the same value Fleet reads to
/// fill `declares` below. Reaching it directly is one authority rather than
/// two, and a declaration a caller cannot forget to hand in — forgetting is
/// what left a `human_always` step reading as a step with nothing on it. The
/// split is a known cost, and `label` and `declares` follow at the next reason
/// to touch them.
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
    /// What the gaming check flagged, in the order it answered. Empty on a step
    /// that declares none and on a step nothing was found on.
    pub flagged: Vec<Flagged>,
    /// Every run of this step, oldest first, folded from the Job's own log.
    ///
    /// **Not a row and not the workflow's** — the other facts here are one or
    /// the other, and this is the third source Fleet holds: `job_events`, which
    /// is what `store::step_attempt` already counts to key the per-attempt
    /// tables. Empty on a step nothing has entered.
    pub attempts: Vec<StepAttempt>,
    /// The Judge call out on this step **right now**. `None` on every step but
    /// the one Fleet is asking about, and on that one too between calls.
    ///
    /// **Not from the store.** The other five fields here are what was written
    /// down; this one is read out of the live slot the gate writes while it
    /// waits, and it is gone the moment the call comes back. A column for it
    /// would be a record of something that is only ever true now.
    pub judging: Option<JudgeInFlight>,
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
    /// What the worktree held when the Job stopped.
    ///
    /// **Absent on every Job that is still going**, which is not a gap: a Job
    /// with a Drone on it has a live reading, published as `job.files_changed`,
    /// and that is the current one. This is the reading nothing else can give
    /// back — the worktree may since have been reclaimed — and a surface that
    /// showed a record while a Drone was still writing would be showing an
    /// answer to a question nobody had asked yet.
    ///
    /// Absent is also every Job that finished before Fleet wrote these down.
    /// Present with no files is a worktree that was read and held no change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint: Option<JobFootprint>,
    /// What the Job's branch came to: the commit, the push, the pull request.
    ///
    /// **Absent is three different facts and the surface must say which.** A
    /// Job still running has none of it. A Job that finished before Fleet wrote
    /// it down has none of it either — and that is most of the Jobs on this
    /// machine, because the result was assembled and dropped for a long time
    /// before it was ever stored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<JobDelivery>,
    /// What the Job has spent, against what it is allowed to.
    ///
    /// **Both halves or neither**, because either alone is unreadable: a figure
    /// with no ceiling says nothing about whether the Job is near one, and a
    /// ceiling with no figure says nothing about this Job at all. It is the
    /// pair that tells a person which of the two signals held their Job back,
    /// which is why the pair is one field.
    ///
    /// Present on every Job, including one that has spent nothing — that is
    /// what makes a Job which cost nothing legible as such rather than as a Job
    /// Fleet has not measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spend: Option<JobSpend>,
    /// The redirect this Job's Drone has been sent and has not answered yet.
    ///
    /// **Absent is the ordinary case**, and on this field absent is the whole
    /// of the second reading: where a step had stopped, the Job went back to
    /// `running` on the send and there is nothing outstanding, so a redirect
    /// that landed on such a Job leaves nothing here. Present is the `stalled`
    /// shape — the Job is still `escalated`, and it is waiting on the Drone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirecting: Option<RedirectInFlight>,
    /// The note a person wrote at a human gate that no Drone has opened with
    /// yet. **The other half of `redirecting`** — that one is a redirect with a
    /// process to go into, this one is a redirect with none.
    ///
    /// **Absent is the ordinary case, and absent is also delivered.** The
    /// record clears the note the moment a Drone's brief is built from it, so
    /// this field stops being present at the same instant the note stops
    /// waiting. Nothing here can go stale, because nothing here is remembered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect_waiting: Option<RedirectWaiting>,
    /// Other Jobs claiming to write where this one says it will.
    ///
    /// **A fact, never a verdict.** `docs/concepts/fleet.md` — "Surfaced,
    /// never serialised." Nothing here refuses a dispatch, and there is no
    /// field a Bridge could read as one; a person approving anyway is the
    /// ordinary case.
    ///
    /// **Absent is not empty**, the same pair `write_targets` draws. Absent
    /// is a Job that has claimed nothing yet — every Job the proposer drafted,
    /// until its first scope step declares — so there was no comparison to
    /// make. Present and empty is a comparison that ran and found nobody. A
    /// card that showed those two the same way would say "no overlap" about a
    /// Job nothing had looked at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_scope_overlaps: Option<Vec<ScopeOverlap>>,
    /// The question this Job's Drone asked and nobody has answered yet.
    ///
    /// **Absent is the ordinary case.** A Drone that never asked, a Drone whose
    /// question has been answered and a Job with no Drone on it are all this
    /// field absent, and the Job's own status tells them apart — a Job that is
    /// not `running` has nothing waiting on an answer.
    ///
    /// **It is not a status and there is no seventh one.** The Job is `running`
    /// and its step is `running`, exactly as they are while a Judge call is
    /// out, and the question stops being outstanding without either moving. See
    /// [`QuestionInFlight`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asking: Option<QuestionInFlight>,
    /// What kind of stuck this Job is, and what moves it.
    ///
    /// **Absent is "this Job did not stop"**, and it is the whole of the
    /// second reading: a queued, running, reviewing, piloted, superseded or
    /// landed Job carries nothing here, because a classification on one of
    /// those would offer acts against a Job nothing is wrong with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stuck: Option<Stuck>,
}

/// Why a Job stopped, and what moves it.
///
/// # The trigger is the classification, and this is its second half
///
/// The registry already names why a Job stopped and gives each trigger the
/// words a person reads. Nothing here mints a word on top of them:
/// [`stopped_by`](Stuck::stopped_by) is the registry's own spelling. What is
/// added is the sentence's other half — a person was shown `stalled` and left
/// to work out which of five acts applies, and that mapping existed only as
/// refusals, learned by pressing a button and reading the 409.
///
/// # Fleet decides once, so the sentence and the buttons cannot disagree
///
/// Bridge derived this from `status`, `current_step_id` and `assigned_drone`
/// and got four of five refusals right. It could not get the fifth: whether the
/// worktree survives is a `path.is_dir()` and a renderer reads no filesystem,
/// so a restart was offered on a Job that had none.
/// [`worktree_on_disk`](Stuck::worktree_on_disk) is that fact, crossing for the
/// first time.
///
/// **It does not claim the trigger is true.** A Drone whose worktree was
/// deleted escalated as `stalled`, the nearest trigger and the wrong condition.
/// What crosses is the escalation as recorded beside the worktree fact, so the
/// acts are right even where the trigger that produced them is not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stuck {
    /// The escalation trigger, spelled as the registry spells it.
    ///
    /// A string rather than a mirrored enum, for the reason
    /// [`Verdict::trigger`] is one: a closed set restated here would be a
    /// second authority for a list that already has one. **Absent is a Job that
    /// recorded no trigger** — one killed by hand stops no step and its
    /// transition carries no reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_by: Option<String>,
    /// The step that stopped, where a step-level trigger named one.
    ///
    /// **Absent on every Job-level escalation**, which is what makes a restart
    /// incoherent there rather than merely refused: `stalled`, `interrupted`
    /// and `resource_exhausted` name no step to run again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<StepId>,
    /// The acts Fleet will take on this Job **now**, ordered by how much each
    /// takes away.
    ///
    /// **Empty is a dead end and says so**: nothing resumes this Job and
    /// nothing replaces it either. It is not the same as absent, which is a Job
    /// that has not stopped at all.
    pub recourse: Vec<Recourse>,
    /// Whether the Job's worktree is still on disk.
    ///
    /// **The fact that decides between a restart and a redispatch**, and the
    /// one no surface can compute for itself. It rides beside the acts rather
    /// than only inside them so that a screen can say *why* a restart is not
    /// offered instead of only that it is missing.
    pub worktree_on_disk: bool,
}

impl Stuck {
    /// The classification, as Fleet made it.
    ///
    /// Every field is read off the domain value, so nothing here can decide
    /// anything the classification did not.
    pub fn of(stuck: &core_model::Stuck) -> Stuck {
        Stuck {
            stopped_by: stuck.stopped_by().map(|why| why.as_wire().to_string()),
            step_id: stuck.step().map(StepId::from),
            recourse: stuck
                .recourse()
                .iter()
                .copied()
                .map(Recourse::from)
                .collect(),
            worktree_on_disk: stuck.standing().worktree_on_disk,
        }
    }
}

impl JobDetail {
    /// A Job in full, plus the reason its last recorded transition carried and
    /// what Fleet knows about its steps.
    ///
    /// Everything past the Job is an argument for the same reason
    /// [`JobSummary::of`] takes two: none of it is a field of
    /// `core_model::Job`. The reason is in the log, the queued reason is
    /// computed from the board, a step's Checks are the workflow's and the
    /// store's, and the footprint is the store's alone — a Job carries no
    /// record of what it touched.
    pub fn of(
        job: &core_model::Job,
        reason: Option<&core_model::TransitionReason>,
        queued_reason: Option<core_model::QueuedReason>,
        resumption: Option<core_model::Resumption>,
        steps: &[StepFacts],
        footprint: Option<JobFootprint>,
        redirecting: Option<RedirectInFlight>,
        asking: Option<QuestionInFlight>,
        stuck: Option<&core_model::Stuck>,
        write_scope_overlaps: Option<Vec<ScopeOverlap>>,
        delivery: Option<JobDelivery>,
        spend: Option<JobSpend>,
    ) -> JobDetail {
        JobDetail {
            // **`asking` from the question this call was already handed**,
            // never a second argument: one fact, one source, and a detail whose
            // row said `false` while its own `asking` held a question would be
            // two answers to one question in one message.
            job: JobSummary::of(job, reason, queued_reason, asking.is_some(), resumption),
            created_at: job.created_at().into(),
            branch: job.branch().map(|branch| branch.as_str().to_string()),
            delivery,
            spend,
            steps: job
                .steps()
                .iter()
                .map(|step| {
                    StepDetail::of(
                        step,
                        job.workflow().step(step.step_id()),
                        facts_for(steps, step.step_id()),
                    )
                })
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
            footprint,
            redirecting,
            // An argument like `redirecting`, and for the same reason: the
            // question lives on the working slot for as long as it is
            // unanswered and the record carries no column for it. A question is
            // only ever true now.
            asking,
            // Read straight off the record, and deliberately not an argument
            // like the six above it: every one of those is a fact the Job does
            // not carry, and this one is a column on `jobs`. A caller asked to
            // hand it in could hand in a note the record had already cleared.
            redirect_waiting: job.redirect_waiting().map(RedirectWaiting::of),
            // An argument like the six above it, and for their reason: which
            // other Jobs claim these paths is not a field of this Job, and
            // working it out needs every other Job's record.
            write_scope_overlaps,
            stuck: stuck.map(Stuck::of),
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
    /// What this step declares for the semantic tier, in the workflow's order.
    ///
    /// **Empty and absent are the two sentences `checks` has**, for the same
    /// reason: empty is a step the Judge will not look at, absent is Fleet
    /// unable to say. Neither is "nothing will happen here" — read it beside
    /// `advance_gate`, which is what says whether the step stops for a person.
    ///
    /// **An inert entry does not cross.** The domain represents a disabled
    /// judge check and an absent one identically, as an entry with no
    /// criteria, so an entry that asks nothing and looks for nothing would
    /// lengthen this list without a Judge ever being called. What is here
    /// fires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_checks: Option<Vec<DeclaredJudge>>,
    /// What it takes to advance past this step, as the workflow declares it.
    ///
    /// **This is what lets a step say it will stop before it stops.**
    /// `human_always` holds the Job at `awaiting_review` for a person, and a
    /// screen that could not read this drew the commonest halt in the fleet as
    /// a step with nothing on it. Absent on the same grounds as `checks`: the
    /// frozen workflow does not declare the step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advance_gate: Option<AdvanceGate>,
    /// Absent until a gate has ruled on the step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_verdict: Option<Verdict>,
    /// **The step advanced because a person overruled the gate, not because it
    /// passed.**
    ///
    /// A field rather than a rule a client applies, and that is the point: the
    /// fact is already on the wire as `state: advanced` beside
    /// `last_verdict: failed`, and leaving a surface to notice the pair is how
    /// an override comes to read like an ordinary advance. Every screen that
    /// draws a rail would have to spell the same rule, and the first one that
    /// forgot would draw a Judge that had been overruled as a Judge that had
    /// cleared the work.
    ///
    /// What was overruled is on `last_verdict`, which still names the trigger.
    /// The person's reason is in the Job's own log, not here.
    pub overridden: bool,
    /// Every criterion the Judge answered on this step, in the order asked.
    ///
    /// **This is where a refusal's citation arrives**, and it is the whole
    /// reason a refusal escalates rather than ending the Job: the trigger says
    /// the gate stopped, and only these say what was wrong with the work.
    /// Empty on a step that asks nothing and on a step the Judge never reached.
    pub judged: Vec<Judged>,
    /// Every gaming pattern this step's evidence tripped, with what each cites.
    ///
    /// **This is what `evidence_suspect` does not say.** The trigger says the
    /// evidence is not to be trusted; only these say which shape of gaming was
    /// found and where — the same relation `judged` has to a `gate_failure`.
    /// Empty on every step nothing was flagged on, which is nearly all of them.
    pub flagged: Vec<Flagged>,
    /// Every run of this step, oldest first. **`Attempt 1 refused`, `Attempt 2
    /// advanced`** — the rows a rail draws under a step that was worked more
    /// than once.
    ///
    /// It is the only place an earlier run's outcome survives:
    /// [`last_verdict`](StepDetail::last_verdict) and [`state`](StepDetail::state)
    /// are both the latest, so a step that passed on its third try and one that
    /// passed on its first were the same message. Empty on a step nothing has
    /// entered, which is every step of a Job that has not reached it.
    ///
    /// **A count is not carried beside it.** The list's length is the count,
    /// and the pair could disagree — the same argument
    /// [`JobDetail::steps`] makes about its own.
    #[serde(default)]
    pub attempts: Vec<StepAttempt>,
    /// The Judge call out on this step **right now**, where one is.
    ///
    /// **Absent is the ordinary case and it is not a gap.** A step nothing is
    /// asking about carries nothing here, which is what makes the absence as
    /// legible as the presence: a step that is not judging and a step that is
    /// look the same because they *are* the same, and the field is the only
    /// thing that separates them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judging: Option<JudgeInFlight>,
    /// When the step was entered. Stamped at creation and moved on entering
    /// `running`, so `entered_at` to `updated_at` is how long the step took.
    pub entered_at: Instant,
    pub updated_at: Instant,
}

/// One Judge call, while it is still out.
///
/// **The fact this seam had no way to state.** A step waiting on a model call,
/// a step whose Drone is thinking and a step that had quietly become
/// unreachable were the same pixels, so the question "is a Judge running where
/// I cannot see it" was asked twice in one day with a different true answer
/// each time.
///
/// # It is not a `StepState`
///
/// `domain/step-states.toml` declares six, a seventh is a variant the other
/// side matches on — a major bump by this seam's own table — and it would be
/// the wrong fact anyway. A step whose gate is asking is still `running`, and
/// it stops asking without moving. **So this rides beside the state**, and
/// nothing about the six changes.
///
/// # `since`, because a spinner says nothing
///
/// A Board is scanned, and ninety seconds is a different fact from two against
/// a two-minute budget. What crosses is the instant the call went out and the
/// budget it has to answer inside; every surface subtracts for itself.
/// **Nothing ticks** — no second message ages this one, which is what keeps a
/// two-minute call to two messages rather than a hundred and twenty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgeInFlight {
    /// Which of Fleet's four looks is out: `criterion`, `drift`, `gaming` or
    /// `convergence`, spelled as `crates/fleet/src/judging.rs` spells them.
    ///
    /// A string rather than a closed set, for the reason [`Verdict::named`] and
    /// [`DeclaredCheck::kind`] are strings: **no registry declares this set.**
    /// It is decided by the code that makes the calls, and a mirrored enum here
    /// would be a second authority for a list that has exactly one. It is
    /// deliberately not a `core-model` vocabulary — a look is something Fleet
    /// does, not a state anything is in, and nothing is stored under these
    /// names.
    pub look: String,
    /// Which criterion the call is about. **The join to
    /// [`judged`](StepDetail::judged)**, where the same `criterion_id` reappears
    /// once the answer comes back — first asked with no verdict, then answered
    /// with one.
    ///
    /// Absent on `gaming`, which is about a `pattern`, and on `convergence`,
    /// which is about neither. On `drift` it is the one criterion Fleet adds
    /// itself, which is why the id is a name rather than one of the Job's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criterion_id: Option<CriterionId>,
    /// Which gaming pattern the call is about, spelled as `flag_if` spells it.
    /// **The join to [`flagged`](StepDetail::flagged)**, exactly as
    /// `criterion_id` is the join to `judged`. Absent on every other look.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Which model is out. The dial that decides what the wait costs and how
    /// long it is likely to be, and a `String` for
    /// [`JobSummary::model`](crate::JobSummary::model)'s reason — naming a
    /// closed set would put a vendor's vocabulary on the wire.
    pub model: String,
    /// Which call of how many this pass over the step is making. Counted from
    /// one.
    ///
    /// **This is what `panel_size` and a multi-criterion step need.** One step
    /// is several calls, and a surface that says "judging" without saying which
    /// of them is the spinner this pair exists instead of.
    pub call: u32,
    /// How many calls this pass will make in total — criteria × panel size,
    /// plus the drift look where the work drifted.
    ///
    /// The same arithmetic [`DeclaredJudge`] already lets a client do, carried
    /// so that it does not have to and so that the two cannot disagree about
    /// the drift look, which no declaration mentions.
    pub of: u32,
    /// When the call went out.
    pub since: Instant,
    /// How long the call may take before it is a failed call, in milliseconds.
    ///
    /// **Fleet's own budget, not a setting** — `crates/config/settings.toml`
    /// names no Judge latency budget. It crosses so a surface can draw the wait
    /// against its ceiling instead of against nothing, which is the difference
    /// between "this is taking a while" and "this is nearly out of time".
    pub budget_ms: u64,
}

impl StepDetail {
    fn of(
        step: &core_model::JobStep,
        declared: Option<&core_model::ResolvedStep>,
        facts: Option<&StepFacts>,
    ) -> StepDetail {
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
            judge_checks: declared.map(|declared| DeclaredJudge::firing(declared.judge_checks())),
            advance_gate: declared.map(|declared| declared.advance_gate().into()),
            last_verdict: step.last_verdict().map(Verdict::of),
            // The one place the pair is read, so that no surface has to. A step
            // that advanced still carrying a failure is a step a person
            // advanced over the gate's ruling — the ordinary advance writes
            // `passed` and the two cannot be confused.
            overridden: step.state() == core_model::StepState::Advanced
                && matches!(
                    step.last_verdict(),
                    Some(core_model::StepVerdict::Failed(_))
                ),
            judged: facts.map(|facts| facts.judged.clone()).unwrap_or_default(),
            flagged: facts.map(|facts| facts.flagged.clone()).unwrap_or_default(),
            attempts: facts
                .map(|facts| facts.attempts.clone())
                .unwrap_or_default(),
            judging: facts.and_then(|facts| facts.judging.clone()),
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
    /// Where the whole brief this verdict answers was written, relative to the
    /// repository root.
    ///
    /// **A reference, never the question**, the way [`CheckRun::output_path`]
    /// is one: a brief carries the request, the deliverable and the whole
    /// branch diff, and Bridge does not read the filesystem. **Absent where the
    /// brief was not kept**, which is a verdict nobody can re-read against its
    /// input rather than one with an empty question.
    ///
    /// [`CheckRun::output_path`]: crate::CheckRun::output_path
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brief_path: Option<String>,
}

impl From<&core_model::Judgment> for Judged {
    fn from(judgment: &core_model::Judgment) -> Judged {
        Judged {
            criterion_id: (&judgment.criterion_id).into(),
            verdict: judgment.verdict.into(),
            expected: judgment.expected.clone(),
            produced: judgment.produced.clone(),
            consequence: judgment.consequence.clone(),
            brief_path: judgment.brief_path.clone(),
        }
    }
}

/// One gaming pattern found, and what it was found in.
///
/// **Never a verdict**, the property `core_model::GamingFlag` has: a flag says
/// the evidence is suspect and does not say the step failed.
///
/// `pattern` is a string rather than a mirrored enum, for the reason
/// [`Verdict::trigger`] is one: no domain registry declares the set — it comes
/// from what a workflow's `flag_if` names — so an enum here would be a second
/// authority for a list that has none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flagged {
    /// The pattern, spelled as `flag_if` spells it.
    pub pattern: String,
    /// The file, line or assertion the flag is about. **The whole value of the
    /// finding** — an uncited flag is unactionable, exactly as an uncited
    /// refusal is.
    pub cited: String,
}

impl From<&core_model::GamingFlag> for Flagged {
    fn from(flag: &core_model::GamingFlag) -> Flagged {
        Flagged {
            pattern: flag.pattern.as_wire().to_string(),
            cited: flag.cited.clone(),
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

/// What a finished Job's branch came to.
///
/// **Three independent absences.** A commit with no push is a repository that
/// names no remote; a push with no pull request is a machine with nothing that
/// can open one, or a repository with no base to open it against. Neither is a
/// failure, and a client that folded them together would have to say "unknown"
/// about a branch that is sitting on a remote right now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDelivery {
    /// The commit Fleet wrote over the Job's work, by its id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// Where the branch was pushed, as `remote/branch`, or that there was no
    /// remote to push to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pushed: Option<String>,
    /// The address a person clicks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<String>,
}

/// What a Job has spent and what it is allowed to spend.
///
/// **Four numbers and no verdict.** Whether the Job is over is the pair being
/// compared, and a client that was handed a boolean instead could not say by
/// how much or which of the two signals it was — which is the whole of what
/// `queued_reason = over_budget` does not carry.
///
/// `cost_micros` and `cost_cap_micros` are millionths of a dollar. They are
/// integers because a cap compared as a float answers differently on two
/// machines, and **notional**: `total_cost_usd` is what a run would have cost
/// at list price, which is not what a subscription account is billed. It is a
/// runaway detector denominated in dollars rather than an invoice, and a
/// surface that presents it as money owed is presenting a currency nothing
/// here spends.
///
/// `ran_ms` carries no cap beside it on purpose. Wall clock is bounded by
/// `settings.drone-job-timeout`, which is a different row at a different scope
/// and is not enforced yet — so the figure is here to be read and the ceiling
/// is not here to be believed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSpend {
    /// What every Drone of this Job has cost, added up.
    pub cost_micros: u64,
    /// What it may cost before Fleet stops starting Drones on it.
    pub cost_cap_micros: u64,
    /// How many turns every Drone of this Job has taken, added up.
    pub turns: u64,
    /// How many it may take before Fleet stops starting Drones on it.
    pub turn_cap: u64,
    /// How long those Drones ran, in milliseconds. **No cap beside it** — see
    /// this type's note.
    pub ran_ms: u64,
    /// How many Drones this is the sum of. **Zero is not the same as a cost of
    /// zero**: it is a Job nothing has run for yet, and a surface that folded
    /// the two would say a Job was free when nobody had tried it.
    pub drones: u64,
}
