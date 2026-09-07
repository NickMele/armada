//! One step of a Job, as a client reads it and as Fleet hands it in.
//!
//! [`StepDetail`] is the row a rail draws: what the frozen workflow declared
//! for the step, what each gate did on every run of it, what the Judge cited,
//! and what the latest run came to. [`StepFacts`] is the other side of the
//! same seam and never crosses — it is what Fleet assembles, because the
//! declaration is the workflow's and the result is the store's, and the frozen
//! `job_steps` row carries neither.
//!
//! **Two of a step's four declared facts are read rather than handed in.**
//! `advance_gate` and `judge_checks` come off the Job's own frozen workflow
//! inside [`crate::JobDetail::of`], so a caller cannot forget to pass them;
//! [`StepFacts`] says why the split falls where it does.
//!
//! Empty and absent are two different sentences on nearly every list here,
//! which is the rule the parent module states for the detail as a whole.

use serde::{Deserialize, Serialize};

use crate::attempt::StepAttempt;
use crate::checks::{CheckRun, DeclaredCheck, DeclaredJudge};
use crate::enums::{AdvanceGate, StepState};
use crate::ids::{CriterionId, Instant, StepId};
use crate::judged::{Flagged, Judged, KeptDeliverable};

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
///
/// [`JobDetail::of`]: crate::JobDetail::of
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
    /// The copies of this step's deliverable that are on disk, oldest run
    /// first. **The one fact here read off the filesystem** — Fleet rebuilds
    /// each name from the step, the run and the target the frozen workflow
    /// declares, and answers only the names that are there.
    pub deliverables: Vec<KeptDeliverable>,
    /// Every run of this step, oldest first, folded from the Job's own log.
    ///
    /// **Not a row and not the workflow's** — the other facts here are one or
    /// the other, and this is the third source Fleet holds: `job_events`, which
    /// is what `store::step_attempt` already counts to key the per-attempt
    /// tables. Empty on a step nothing has entered.
    pub attempts: Vec<StepAttempt>,
    /// What each closed run of this step came to, oldest first. **Derived from
    /// `attempts` and carried beside it**, rather than recomputed at every
    /// reader: [`StepAttempt::verdicts`] is the one place the `(outcome, why)`
    /// pair becomes a ruling, and a second surface restating that mapping is
    /// how the two come to disagree.
    pub verdicts: Vec<Verdict>,
    /// The Judge call out on this step **right now**. `None` on every step but
    /// the one Fleet is asking about, and on that one too between calls.
    ///
    /// **Not from the store.** The other five fields here are what was written
    /// down; this one is read out of the live slot the gate writes while it
    /// waits, and it is gone the moment the call comes back. A column for it
    /// would be a record of something that is only ever true now.
    pub judging: Option<JudgeInFlight>,
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
    /// What each declared Check did, **every attempt's rows, oldest first**.
    /// Empty until the gate has run them, which is not the same as declaring
    /// none. Join to [`attempts`](StepDetail::attempts) by `attempt` — a step
    /// worked more than once holds one group of rows per run, not the
    /// latest's alone.
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
    /// Every criterion the Judge answered, **every attempt's rows, oldest
    /// first**, in the order asked within each run.
    ///
    /// **This is where a refusal's citation arrives**, and it is the whole
    /// reason a refusal escalates rather than ending the Job: the trigger says
    /// the gate stopped, and only these say what was wrong with the work.
    /// Empty on a step that asks nothing and on a step the Judge never reached.
    /// Join to [`attempts`](StepDetail::attempts) by `attempt`, for
    /// [`check_runs`](StepDetail::check_runs)'s reason.
    pub judged: Vec<Judged>,
    /// Every gaming pattern this step's evidence tripped, with what each cites.
    ///
    /// **This is what `evidence_suspect` does not say.** The trigger says the
    /// evidence is not to be trusted; only these say which shape of gaming was
    /// found and where — the same relation `judged` has to a `gate_failure`.
    /// Empty on every step nothing was flagged on, which is nearly all of them.
    pub flagged: Vec<Flagged>,
    /// The copies of this step's deliverable Fleet kept, oldest run first.
    ///
    /// **The third of the three records a verdict is argued with**, beside
    /// [`CheckRun::output_path`] and [`Judged::brief_path`]: what the Judge
    /// read, what it printed, and what it was asked. One without the others
    /// cannot separate a bad Judge from a bad brief, which is why all three
    /// cross rather than the two that already did.
    ///
    /// **Empty is the ordinary case.** A step that declares no deliverable
    /// keeps none, and so does one whose Judge was never asked — the bytes are
    /// read where the Judge's call is built and nowhere else.
    ///
    /// [`CheckRun::output_path`]: crate::CheckRun::output_path
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deliverables: Vec<KeptDeliverable>,
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
    ///
    /// [`JobDetail::steps`]: crate::JobDetail::steps
    #[serde(default)]
    pub attempts: Vec<StepAttempt>,
    /// What each closed run of this step came to, oldest first. **The nested
    /// reading `attempts` alone cannot give**: `last_verdict` says only the
    /// current ruling and `attempts[].outcome` says only where a run ended,
    /// never `passed` or `failed` in so many words. Empty on a step nothing
    /// has closed a run of yet, same as `attempts`.
    #[serde(default)]
    pub verdicts: Vec<Verdict>,
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
    pub(super) fn of(
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
            last_verdict: step
                .last_verdict()
                .map(|verdict| Verdict::of(latest_closed_attempt(facts), verdict)),
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
            deliverables: facts
                .map(|facts| facts.deliverables.clone())
                .unwrap_or_default(),
            attempts: facts
                .map(|facts| facts.attempts.clone())
                .unwrap_or_default(),
            verdicts: facts
                .map(|facts| facts.verdicts.clone())
                .unwrap_or_default(),
            judging: facts.and_then(|facts| facts.judging.clone()),
            entered_at: step.entered_at().into(),
            updated_at: step.updated_at().into(),
        }
    }
}

/// Which attempt a step's standing verdict belongs to.
///
/// **The latest closed attempt, never the one still open.** Entering
/// `running` leaves the previous ruling standing —
/// `core_model::JobStep::moved_to`'s own rule — so the run that produced
/// `last_verdict` is the last one `verdicts` holds, not the last one
/// `attempts` does. `1` where nothing has closed yet, which cannot arise
/// beside a `last_verdict` that is `Some`.
fn latest_closed_attempt(facts: Option<&StepFacts>) -> u32 {
    facts
        .and_then(|facts| facts.verdicts.last())
        .map(|verdict| verdict.attempt)
        .unwrap_or(1)
}

/// The last ruling against a step.
///
/// Two fields rather than one string, for the reason
/// [`Reason`](crate::Reason) has two: `failed` carries the trigger that
/// failed it, and the other two carry nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    /// Which run of the step this ruling was made on, counted from one. **On
    /// the row rather than implied by position**, for
    /// [`CheckRun::attempt`](crate::CheckRun::attempt)'s reason: this is what
    /// joins [`StepDetail::last_verdict`] and each entry of
    /// [`StepDetail::verdicts`] back to [`StepDetail::attempts`]. The same
    /// ordinal [`StepAttempt::attempt`](crate::StepAttempt) carries.
    pub attempt: u32,
    /// `passed`, `failed` or `not_reached`, spelled as the domain spells it.
    pub named: String,
    /// The escalation trigger a failure carried. Absent on the other two.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
}

impl Verdict {
    pub fn of(attempt: u32, verdict: core_model::StepVerdict) -> Verdict {
        Verdict {
            attempt,
            named: verdict.as_wire().to_string(),
            trigger: match verdict {
                core_model::StepVerdict::Failed(trigger) => Some(trigger.as_wire().to_string()),
                _ => None,
            },
        }
    }
}
