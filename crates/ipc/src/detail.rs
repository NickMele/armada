//! One Job, whole. The answer to `get_job`.
//!
//! [`JobDetail::job`] nests the same [`JobSummary`] the Board row is built
//! from, so the two cannot disagree and a field added to the summary reaches
//! the detail for free. What is added here is what a list leaves behind.
//!
//! **A step is its own module.** What a step says about itself needs no Job,
//! so [`step`] holds [`StepDetail`] and the rows under it; the Job is here.
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
//! Evidence is absent because nothing produces it and an always-empty field
//! reads as working; the log, because `#437` serves it as a stream of its own.

mod step;

use serde::{Deserialize, Serialize};

use crate::asking::JudgeQuestion;
use crate::commanding::{CommandAnswer, CommandInFlight, WhenBlocked};
use crate::enums::{CriterionSource, DependencyDirection, Recourse};
use crate::ids::{CriterionId, Instant, JobId, StepId};
use crate::job::{JobSummary, Subject};
use crate::overlap::ScopeOverlap;
use crate::waiting::{QuestionInFlight, RedirectInFlight, RedirectWaiting};
use crate::work::JobFootprint;

pub use step::{JudgeInFlight, StepDetail, StepFacts, Verdict};

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
    /// **Present from the step that delivers, not from the Job finishing.** A
    /// workflow's delivering step commits, pushes and opens the pull request
    /// when the step is *entered* and then holds for a person —
    /// `crates/fleet/src/landing.rs`, `#520` — so a Job sitting at that gate,
    /// long before any terminal status, already has this. **Absent is three
    /// different facts and the surface must say which.** A Job that has not
    /// reached a delivering step has none of it. A Job on a workflow with no
    /// delivering step has none of it either, by design. A Job that finished
    /// before Fleet wrote this down has none of it either — and that was most
    /// of the Jobs on this machine, because the result was once assembled and
    /// dropped rather than stored.
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
    /// Whether a person can ask this Job to show its work again, and every
    /// time somebody did. **Since 10.1**, and absent from a Fleet older than
    /// that — which a reader draws as no control at all rather than a refusal.
    ///
    /// **Filled after [`JobDetail::of`] rather than handed to it**, because it
    /// is read off the worktree and the Manifest as well as the record, and the
    /// constructor takes only what the record and the workflow say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_again: Option<crate::showing::ShowAgain>,
    /// What this Job does when its Drone reaches for a command it was not
    /// given. **Since 10.7**, and absent from a Fleet older than that — which a
    /// reader draws as no setting at all rather than as the default.
    ///
    /// **Filled after [`JobDetail::of`] rather than handed to it**, as
    /// `show_again` is. `of` already takes thirteen positional arguments, and
    /// another is where one lands in the wrong slot silently — the defect
    /// `tests::detail_of` exists for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when_blocked: Option<WhenBlocked>,
    /// The command this Job's Drone is waiting on a person to allow, right now.
    ///
    /// **Absent is the ordinary case**, and it is every Job at
    /// [`WhenBlocked::RefuseAndHold`]: nothing waits there, and a refused
    /// command is answered on [`Stuck::refused`] instead. Filled after
    /// [`JobDetail::of`], like `when_blocked`. See [`CommandInFlight`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_waiting: Option<CommandInFlight>,
    /// The Judge question a person is being asked about, right now. **Since
    /// 11.1**, and absent from a Fleet older than that. Filled after
    /// `JobDetail::of`, like `command_waiting`. See `JudgeQuestion`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_question: Option<JudgeQuestion>,
    /// The commands a person allowed for this Job, oldest first. **Since
    /// 11.0.**
    ///
    /// **Empty is a Job nobody allowed anything on**, and a detail from before
    /// the field reads the same way rather than failing. A command allowed
    /// with [`Reach::Repository`](crate::Reach::Repository) is listed here too:
    /// the Job holds it whichever way it was allowed. Filled after
    /// [`JobDetail::of`], like `when_blocked`.
    #[serde(default)]
    pub allowed_commands: Vec<crate::AllowedCommandRow>,
    /// The model a person chose for this Job's later steps. **Since 11.0.**
    ///
    /// **Absent is no choice**, and each step runs on the model its workflow
    /// gives it. Present, the next step's Drone is spawned on it; the step
    /// running when it was chosen keeps its own. `set_model` moves it. Filled
    /// after [`JobDetail::of`], like `when_blocked`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_override: Option<String>,
    /// The review Fleet composed at this Job's gate — the same text a pull
    /// request carries, where this Job has one. **Since 10.11**, and absent
    /// from a Fleet older than that, which a reader draws as no review at all
    /// rather than a Job that changed nothing.
    ///
    /// **One builder.** `crates/fleet/src/review.rs` composes this and the
    /// pull request's Markdown body from the one reading of the record; the
    /// review area draws these sections instead of assembling its own copy of
    /// them — `#665`.
    ///
    /// **Absent is a Job that has not reached a gate yet**, not an empty
    /// review: a Job still running, or one that finished with no `human_always`
    /// step at all, carries nothing here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<JobReview>,
}

/// The review Fleet composed, in the four parts it is made of. No heading
/// crosses — a heading is how a surface draws a section, not what the section
/// is, and the pull request's own Markdown adds its headings back at render
/// time from the same four parts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobReview {
    /// The brief, in the requester's own words.
    pub why: String,
    /// What the Job's worktree changed, as far as a diff can say it.
    pub outcome: String,
    /// What nothing checked, and what the base carries that this Job did not
    /// write, where there was a base to ask.
    pub risks: String,
    /// Every step and every Check that ran against it, with its outcome.
    pub evidence: String,
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
    /// Whether a Drone is standing on this Job that Fleet cannot hear.
    ///
    /// **The other fact no surface can compute**, and the one that decides what
    /// a restart is about to do. A Job whose agent outlived the Fleet that held
    /// its pipes is running and unreachable at once: `stopped_by` reads
    /// whatever escalated it, the Board shows a step in flight, and nothing on
    /// the wire said which of those two a person was looking at. So Bridge
    /// described restarting as taking over from a Drone that had gone, on the
    /// one Job where it has not.
    ///
    /// **False is every ordinary stuck Job** — including one whose Drone really
    /// is gone, which is the case the sentence used to be written for. Both are
    /// real and the restart does the right thing in each; this is what lets a
    /// surface say which one it is looking at.
    ///
    /// It is a fact and not an act. Whether the restart is on offer is still
    /// `recourse`, decided here.
    #[serde(default)]
    pub drone_unheard: bool,
    /// What the Drone reached for and was refused, oldest first.
    ///
    /// **The trigger's evidence, and nothing anywhere carried it.**
    /// `blocked_by_policy` named a policy and no surface named what it stopped,
    /// so a person was told to widen an allowlist without being told what to
    /// widen it to. The tool and the command sat on two transcript rows joined
    /// by a call id, and nothing made that join.
    ///
    /// **Empty is a Drone that was refused nothing**, which is most of them,
    /// and it is not the same as [`refusals`](Stuck::refusals) being zero on a
    /// Job whose transcripts were reclaimed — that reads empty here too, and
    /// the count is what says so.
    ///
    /// It rides on every trigger and not only on `blocked_by_policy`. A Drone
    /// refused a command it needed goes on to escalate as `stalled` or `silent`
    /// just as often, and evidence withheld until the classification named it
    /// would be missing from exactly the Jobs the classification got wrong.
    #[serde(default)]
    pub refused: Vec<Refusal>,
    /// How many calls were refused altogether, counting the ones
    /// [`refused`](Stuck::refused) left out.
    ///
    /// **A size rather than a flag**, which is `CallArguments::length`'s
    /// reason: a surface can say *showing 50 of 137* instead of reporting that
    /// something was taken away. Equal to `refused.len()` on every Job whose
    /// refusals all fit, which is the ordinary case.
    #[serde(default)]
    pub refusals: u64,
    /// What the gate said it could not read — *the Judge did not answer inside
    /// its budget* — where that is why the stopped step stopped.
    ///
    /// **Present only where [`stopped_by`](Stuck::stopped_by) is
    /// `gate_undecided`.** A gate that could not read what it needed carries no
    /// verdict, so nothing else on this type says why the step is stuck —
    /// `stopped_by` alone names the trigger and not the cause, which was
    /// reaching Fleet's own log and nothing on the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undecided: Option<String>,
}

/// One call the Drone reached for and was refused.
///
/// **[`detail`](Refusal::detail) is the field a person reads.** The harness
/// usually sends no reason — the observed `permission_denied` line carried an
/// empty `decision_reason` — so a row drawn from
/// [`because`](Refusal::because) alone draws nothing, which is the whole of
/// what was wrong.
///
/// **The whole argument stays in the file**, exactly as [`Shown`](crate::Shown)
/// leaves it: a heredoc is a whole file and this crosses on every open of a
/// stopped Job. What crosses instead is that the line was cut and how long the
/// argument was — enough for a surface to say so, which is what a person about
/// to paste a command into an allowlist needs. `get_call` is the operation that
/// serves the rest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refusal {
    /// The tool that was reached for, in the harness's own spelling.
    ///
    /// A string and not a mirrored enum, for [`Stuck::stopped_by`]'s reason:
    /// the set is the harness's and no registry here declares it.
    pub tool: String,
    /// The call id the transcript rows carried.
    ///
    /// **What makes a cut detail openable**: `get_call` serves the whole
    /// argument by this id, so a refused heredoc shown to its bound is not a
    /// dead end.
    pub call: String,
    /// The argument as the transcript recorded it — the command, the path.
    ///
    /// Bounded where it was built, in `adapter_traits::CallDetail`, so a row
    /// carries one line of it. **Empty is a tool whose arguments that
    /// vocabulary has no name for**, and never an invented one.
    pub detail: String,
    /// Whether [`detail`](Refusal::detail) is less than what the Drone sent.
    ///
    /// **Said rather than implied**, `Saw::Called::truncated`'s reason: a
    /// command can legitimately end in an ellipsis. Without it a surface draws
    /// a cut command as the whole one, and the command is what somebody copies
    /// into an allowlist.
    #[serde(default)]
    pub truncated: bool,
    /// How many characters the argument had, before anything was cut.
    ///
    /// **A size rather than only a flag**, which is
    /// [`CallArguments::length`](crate::CallArguments)'s reason: a surface says
    /// *showing 200 of 14,320 characters* instead of reporting that something
    /// was taken away.
    ///
    /// **`None` is a transcript row written before the file recorded the
    /// size.** A surface holding `truncated: true` and no length has what there
    /// is and no way to say how much is missing, and says that rather than
    /// inventing a size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
    /// The harness's own wording, where it gave one.
    ///
    /// **Usually empty, and empty is the honest answer.** Nothing fills it in
    /// from the trigger: a reason Armada wrote would read as the harness's.
    pub because: String,
    /// What a person may answer about this command. **Since 10.7.**
    ///
    /// **Empty is nothing a person can allow here**, and it is also every row
    /// from an older Fleet, which offered nothing. [`withheld`] says why where
    /// Fleet knows.
    ///
    /// [`withheld`]: Refusal::withheld
    #[serde(default)]
    pub offers: Vec<CommandAnswer>,
    /// Why this command cannot be allowed from here — declared destructive, or
    /// a push. **Since 10.7.** Absent where it can be.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withheld: Option<String>,
}

impl From<&core_model::Refusal> for Refusal {
    fn from(refusal: &core_model::Refusal) -> Refusal {
        Refusal {
            tool: refusal.tool.clone(),
            call: refusal.call.clone(),
            detail: refusal.detail.clone(),
            truncated: refusal.truncated,
            length: refusal.length,
            because: refusal.because.clone(),
            // Fleet fills both after the classification: whether a command may
            // be allowed is the Manifest's and the policy's answer, and a
            // transcript row knows neither.
            offers: Vec::new(),
            withheld: None,
        }
    }
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
            drone_unheard: stuck.standing().drone == core_model::DroneStanding::Unheard,
            refused: stuck.refused().kept().iter().map(Refusal::from).collect(),
            refusals: stuck.refused().in_all(),
            undecided: stuck.undecided().map(str::to_string),
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
        budget_hold: Option<core_model::BudgetHold>,
        resumption: Option<core_model::Resumption>,
        steps: &[StepFacts],
        footprint: Option<JobFootprint>,
        redirecting: Option<RedirectInFlight>,
        asking: Option<QuestionInFlight>,
        stuck: Option<&core_model::Stuck>,
        write_scope_overlaps: Option<Vec<ScopeOverlap>>,
        delivery: Option<JobDelivery>,
        spend: Option<JobSpend>,
        review: Option<JobReview>,
    ) -> JobDetail {
        JobDetail {
            // **`asking` from the question this call was already handed**,
            // never a second argument: one fact, one source, and a detail whose
            // row said `false` while its own `asking` held a question would be
            // two answers to one question in one message.
            job: JobSummary::of(
                job,
                reason,
                queued_reason,
                budget_hold,
                asking.is_some(),
                resumption,
            ),
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
            show_again: None,
            when_blocked: None,
            command_waiting: None,
            judge_question: None,
            allowed_commands: Vec::new(),
            model_override: None,
            review,
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

/// What a Job's branch came to. **Present once the Job has entered its
/// delivering step**, which for most workflows is before the Job finishes —
/// a Job holding at `awaiting_review` with its pull request open already has
/// this.
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
    /// Live facts about it, off Fleet's own rotation rather than this call.
    ///
    /// **Never fetched here.** `get_job` is read on every open of a Job, and a
    /// forge call spent on every one of those would be a process per page
    /// load with no bound. Fleet already asks the forge about every open pull
    /// request on a fixed rotation, to notice a merge — `crate::noticing` in
    /// `fleet` — and this is that same reading, cached rather than fetched
    /// twice.
    ///
    /// **Absent is two different facts, told apart by
    /// [`landed`](JobDelivery::landed).** Where `landed` is also absent,
    /// Fleet's rotation has not reached this pull request yet; it will, within
    /// one rotation of every open pull request this Fleet is holding. Where
    /// `landed` is present, the pull request has settled and Fleet has
    /// stopped asking about it — read `landed` instead of looking for this to
    /// reappear. **Present always means `landed` is absent**: a reading taken
    /// while the pull request was still open, stale by at most one rotation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request_detail: Option<PullRequestDetail>,
    /// What became of that pull request. **Absent is unasked or still open**,
    /// which is one absence because neither is news: a pull request sits open
    /// until somebody decides, so a client that saw "still open" as a value
    /// would be drawing the fact that nothing has happened.
    ///
    /// Present only beside [`pull_request`](JobDelivery::pull_request) — there
    /// is nothing to have settled without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landed: Option<Settled>,
    /// Why [`commit`](JobDelivery::commit) never reached the branch's remote,
    /// where it did not. Since protocol 11.2, `#691`.
    ///
    /// **Absent is not "unknown"** — it is a push that went out, a repository
    /// with no remote to fail against, or a Job that has not reached a
    /// delivering step at all. Present is the one fact this exists for: the
    /// commit stands in the worktree and [`pull_request`](JobDelivery::pull_request),
    /// where one is already open, still shows what it carried before this
    /// attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unpushed: Option<String>,
}

/// What Fleet's rotation last read live off an open pull request.
///
/// **Everything here is a snapshot, not a subscription.** Nothing pushes an
/// update when one of these changes; a client that wants a fresher one reopens
/// the Job after Fleet's rotation has had time to come back around, the same
/// bound `docs/practices/protocol.md`'s noticing sweep already accepts for a
/// merge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestDetail {
    /// The forge's own number for it, where the forge answered one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<u64>,
    /// Its title, as the forge holds it right now — not the title Armada
    /// opened it with, because a person may have edited it since.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Whether the forge can merge it into its base as it stands.
    ///
    /// **`None` is not "conflicting".** It is the forge declining to say —
    /// still computing it, or a forge that would not answer at all — and a
    /// client that read it as a conflict would be reporting a fact nobody
    /// found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mergeable: Option<bool>,
    /// Every review that has landed a verdict, oldest first.
    ///
    /// **Comments are not here.** `get_remarks` serves everything anybody
    /// wrote on the pull request, and a review's note would cross twice if it
    /// were carried on both. Empty is a pull request nobody has approved or
    /// asked changes on, which is the ordinary case for one just opened.
    pub reviews: Vec<ReviewedBy>,
    /// What the last attempt to keep this pull request's branch current
    /// against a moved base came to. `#663`.
    ///
    /// **Absent is a branch that has never needed to move**, which is most of
    /// a pull request's life — this is not written until the base it merges
    /// into moves under it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
}

/// What the last attempt to keep a pull request's branch current against a
/// moved base came to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Currency {
    /// The base's tip this was last attempted against.
    pub rebased_onto: String,
    pub rebased_at: Instant,
    /// **Present, and never empty, exactly where the attempt conflicted and
    /// the branch was left exactly as it was.** Absent is a clean rebase —
    /// the branch is current and nothing is owed to a person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflict_files: Vec<String>,
}

impl Currency {
    /// Whether a person can be offered the Drone to resolve this.
    pub fn conflicted(&self) -> bool {
        !self.conflict_files.is_empty()
    }
}

/// One reviewer's verdict on a pull request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewedBy {
    /// The login of whoever reviewed it, as the forge spells it.
    pub by: String,
    /// One word from the forge's own vocabulary: `approved` or
    /// `changes_requested`. A string rather than a mirrored enum, for
    /// [`Stuck::stopped_by`]'s reason — the forge's own set is not this
    /// registry's to widen, and it is not every state a review can be in,
    /// only the two a person acts on.
    pub verdict: String,
}

/// The two ends a pull request comes to, and the whole of the set.
///
/// **Neither open nor unknown is here**, and that is what makes this closed: a
/// pull request that has not settled is the absence of this value, so no
/// variant means "no news" and no client has to tell one kind of nothing from
/// another. What the forge can say beyond these two, Armada does not record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Settled {
    /// Somebody merged it. The answer to *did this land*.
    Merged,
    /// It was closed and never merged — published, and turned down.
    ClosedUnmerged,
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
    /// How many of those named no price, and so are counted in `drones` and in
    /// none of the figures above.
    ///
    /// **`cost_micros` is a floor while this is non-zero**, and a surface that
    /// draws the total without it says a Job spent less than it did. Cost
    /// reaches Armada on the terminating line of a session, so a Drone
    /// signalled mid-run leaves none — Job `01M21BKVPW002DC0ATD1X9T0VF` had two
    /// such Drones over 277 and 299 seconds and read as $5.28 against a $5 cap.
    ///
    /// **Absent on a row written before Fleet could tell the two apart.**
    /// `serde(default)` reads that as nought, which is what such a row means:
    /// nobody was counting, so nothing is claimed.
    #[serde(default)]
    pub unpriced: u64,
    /// How many Drones this is the sum of. **Zero is not the same as a cost of
    /// zero**: it is a Job nothing has run for yet, and a surface that folded
    /// the two would say a Job was free when nobody had tried it.
    pub drones: u64,
}
