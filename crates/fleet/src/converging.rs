//! Catching a Drone that is working and not getting anywhere.
//!
//! # Four stages, and the trigger is the fourth
//!
//! | Stage | What | Costs |
//! |---|---|---|
//! | Tripwire | tool calls, wall-clock, the plan against the live diff | nothing |
//! | The look | one Judge call, **only** once a tripwire fired | one call |
//! | The directive | "stop and report your current state now" | a turn |
//! | `thrashing` | **when that also fails** | the escalation |
//!
//! `escalation-triggers.toml`: *"active but not converging, **and the forced
//! report also failed**"*. A Drone that thrashes and then reports when told to
//! has not thrashed by that definition. Escalating at stage one would make
//! `thrashing` mean "took a while"; at stage two it would spend a call and
//! ignore what it said.
//!
//! **Stage four asks about silence, not about the finding** — [`NoReport`] is
//! the reason, and a Drone still writing inside its plan re-arms the grace.
//!
//! [`Chain`] holds where a step stands and is cleared when the step changes, so
//! a tripwire that stays tripped — drift does — buys no second call.
//! **Nothing here kills a Drone.** `docs/concepts/helm.md` holds a thrashing
//! Drone with its worktree intact, leaving a redirect available; one waiting on
//! an answer is not thrashing at all — `crate::questioning`.
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, CheckOutcome, Component, Envelope, EscalationTrigger, FieldValue, JobId, JobStatus,
    Level, RepoPath, StepCheck, StepId, StepLevelTrigger, StepTarget, Target, Timestamp,
};
use verification::{Convergence, NotConverging};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::judging;
use crate::session::{LiveSession, Occasion};
use crate::transcript;
use crate::working::Working;

/// What a step is expected to cost before any of this looks at it.
///
/// **Three numbers and nothing measured any of them.** They are one type with
/// one constructor and no `Default` for [`CheckBudget`]'s reason: a threshold
/// invented at a call site is a threshold nobody can find. The composition root
/// names them, once, and says there that they are provisional.
///
/// [`CheckBudget`]: crate::CheckBudget
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepNorms {
    calls: u32,
    wall_clock: Duration,
    report_grace: Duration,
}

impl StepNorms {
    pub const fn of(calls: u32, wall_clock: Duration, report_grace: Duration) -> StepNorms {
        StepNorms {
            calls,
            wall_clock,
            report_grace,
        }
    }

    /// How many of the Drone's own tool calls one step is expected to take.
    ///
    /// Calls rather than the harness's turns, and the substitution is measured
    /// rather than assumed — [`Progress::calls`](crate::Progress::calls) says
    /// what the two cost against each other and why the harness's number could
    /// not be read mid-step at all.
    pub fn calls(&self) -> u32 {
        self.calls
    }

    /// How long one step is expected to take.
    pub fn wall_clock(&self) -> Duration {
        self.wall_clock
    }

    /// How long the Drone has to come to rest after it is told to report.
    ///
    /// Bounded rather than instant because an injected turn is consumed when
    /// the current tool call returns — spike 4 measured 33s against a
    /// forty-second command — so a Drone inside a long command has not refused
    /// to answer, it has not been asked yet.
    pub fn report_grace(&self) -> Duration {
        self.report_grace
    }
}

/// Which free detector fired. **Never a reason on its own** — it decides when
/// the look is worth paying for and nothing else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tripwire {
    /// The Drone's own tool-call count passed the step norm.
    ToolCalls { taken: u32 },
    /// The step has been running longer than the ceiling.
    WallClock { after: Duration },
    /// Work appeared outside the declared plan. **Not a failure** —
    /// `docs/concepts/judge.md` keeps drift a signal, because legitimate
    /// investigation sometimes moves the work.
    OffPlan { paths: Vec<RepoPath> },
}

impl Tripwire {
    /// The word a log line and a reader use for it.
    pub fn named(&self) -> &'static str {
        match self {
            Tripwire::ToolCalls { .. } => "tool_calls",
            Tripwire::WallClock { .. } => "wall_clock",
            Tripwire::OffPlan { .. } => "off_plan",
        }
    }
}

/// What the chain did to one step this turn. Ordinarily nothing at all.
#[derive(Debug)]
pub struct Wandering {
    pub job: JobId,
    pub step: StepId,
    pub stage: Stage,
}

/// How far the chain got.
#[derive(Debug)]
pub enum Stage {
    /// A tripwire fired and the look found the work converging, or the drift
    /// justified. **The chain stops here**, having cost one call.
    StillConverging {
        tripped: Tripwire,
        found: Convergence,
    },
    /// The look found thrashing and the Drone has been told to stop and report.
    /// **Nothing has escalated** — this is the chance the trigger's own wording
    /// requires it to be given.
    ///
    /// `asked_at` is the instant the look was taken, which is the stamp
    /// everything downstream quotes the finding with.
    AskedToReport {
        tripped: Tripwire,
        why: NotConverging,
        asked_at: Timestamp,
    },
    /// The grace is spent, the report has not come, and the declared plan has
    /// gained work since the look. **Nothing is stopped and nothing is
    /// recorded**: the deadline re-arms from here, so a Drone that then goes
    /// still is cut on the next window.
    StillWriting { since: Vec<RepoPath> },
    /// The report did not arrive inside the grace and nothing moved. The step
    /// is stopped and the Job is escalated as `thrashing`.
    Escalated { quiet: NoReport },
    /// The look could not be made. **Nothing escalates**: a machine that cannot
    /// answer must not produce a verdict, in either direction.
    CouldNotLook {
        tripped: Tripwire,
        cause: judging::CallFailed,
    },
}

/// `thrashing`, narrowed to what a step may be stopped with.
///
/// `Some` for as long as `escalation-triggers.toml` types the row step-level,
/// which is what lets it reach a step's `last_verdict` and makes restarting
/// that step later a coherent act. Threaded as an `Option` rather than
/// unwrapped, so a registry change reads as the chain going quiet in one place
/// instead of as a panic in the daemon.
pub(crate) fn stops_the_step() -> Option<StepLevelTrigger> {
    StepLevelTrigger::of(EscalationTrigger::Thrashing)
}

/// What the Drone is told at stage three.
///
/// **There is no constructor taking a string**, so nothing can put arbitrary
/// text down the pipe under this name; the only way to make one is from a
/// finding. The wording is `docs/contracts/agent-prompt.md` section 4a's draft
/// and is marked there as drafted rather than sanctioned.
///
/// It carries `expected` and `produced` and never `consequence` — the same
/// field selection the refusal reprompt makes, and for the same reason: the
/// third field is written for the person deciding what to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportNow(String);

impl ReportNow {
    pub fn about(why: &NotConverging) -> ReportNow {
        ReportNow(format!(
            "Stop and report your current state now.\n\n  \
             Expected   {}\n  Produced   {}\n\n\
             Submit what you have. Partial work with an accurate Not claimed is \
             worth more than carrying on.",
            why.expected(),
            why.produced()
        ))
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

/// What the step that stopped is written down as.
///
/// Not a Manifest Check and not a `mechanical_checks` entry, for the reason
/// [`verification::EVIDENCE_SCOPE`] is neither: it is named by the thing that
/// asked for it.
pub const FORCED_REPORT: &str = "forced_report";

/// Why the step stopped, as the row a person reads.
///
/// **The proximate cause, which reached no record the Job carried.** It was in
/// one line of Fleet's log — *the forced report did not arrive* — while the
/// step's only row was the mid-step finding under its own name, outcome
/// `failed`. So the record said the work was going nowhere, dated nothing, and
/// a person had to reconstruct the actual sentence from a log file.
///
/// The finding is still shown, because "what was it doing" is the next question
/// after "why did it stop" — quoted, and stamped by
/// [`NotConverging::as_of`] with the instant the look ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoReport {
    grace: Duration,
    quoted: String,
}

impl NoReport {
    /// **There is no constructor that omits the finding or the instant**, so a
    /// row under this name cannot be written without saying what the look found
    /// and when it looked.
    fn after(grace: Duration, asked_at: &Timestamp, why: &NotConverging) -> NoReport {
        NoReport {
            grace,
            quoted: why.as_of(asked_at),
        }
    }

    pub fn recorded(&self) -> StepCheck {
        StepCheck {
            name: FORCED_REPORT.to_string(),
            outcome: CheckOutcome::Failed,
            expected: Some(format!(
                "a report from the Drone within {}s of being told to stop and report",
                self.grace.as_secs()
            )),
            produced: Some(format!(
                "no report arrived and nothing was submitted, so the step was \
                 stopped for the silence. Before it, {}",
                self.quoted
            )),
            output_path: None,
        }
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Carry the step being worked one stage along the chain, where anything
    /// moved it.
    ///
    /// **Cold on an ordinary turn.** Every tripwire is read off the slot, so a
    /// step inside its norms reaches no store, no worktree and no model.
    pub(crate) async fn watch_convergence(
        &self,
        working: &mut Option<Working>,
    ) -> Result<Option<Wandering>, Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(None);
        };
        let (job, step, _) = at_work.standing();
        match at_work.chain() {
            // Already answered for. A step that stopped is a person's, and a
            // step whose look was spent does not get a second one — that is
            // what keeps the tier cold when a tripwire stays tripped.
            Chain::Stopped | Chain::Looked => Ok(None),
            Chain::Reporting {
                asked_at,
                why,
                in_plan,
            } => {
                let (asked_at, why, in_plan) = (asked_at.clone(), why.clone(), in_plan.clone());
                self.after_the_directive(working, &job, &step, &asked_at, why, &in_plan)
                    .await
            }
            Chain::Working => self.first_look(working, &job, &step).await,
        }
    }

    /// Stage one, then stage two, then stage three — in the one turn, because
    /// each follows from the last with nothing to wait for in between.
    async fn first_look(
        &self,
        working: &mut Option<Working>,
        job: &JobId,
        step: &StepId,
    ) -> Result<Option<Wandering>, Adrift> {
        // At the gate, or waiting on a person — neither is thrashing, and both
        // are cheaper than the tripwires. `crate::questioning` says why.
        if self.evidence_waiting_for(&job) > 0 {
            return Ok(None);
        }
        let at_work = working.as_ref().expect("the slot was read as full");
        if crate::questioning::waiting_on_an_answer(at_work) {
            return Ok(None);
        }
        let Some(tripped) = self.tripped(at_work) else {
            return Ok(None);
        };
        let (_, _, worktree) = at_work.standing();
        let record = self.load(job).await?;
        if record.status() != JobStatus::Running {
            return Ok(None);
        }
        let Some(declared_step) = record.workflow().step(step) else {
            return Ok(None);
        };
        // A patch that will not read spends nothing and marks nothing: there is
        // no work product to look at, so there is no look to have had.
        let Ok(patch) = self.work().patch(&worktree) else {
            return Ok(None);
        };
        // The declared plan as the finding is about to be made against it. A
        // file inside the plan that is not in this reading appeared *after* the
        // look, which is what makes the citation stale rather than merely old.
        // One reading, on the one turn of a step that reaches this far.
        let in_plan = self.in_plan(at_work);
        let judging = self.judging(job).map_err(|cause| Adrift::NotConfigurable {
            job: job.clone(),
            cause,
        })?;
        let found = judging::converging(
            declared_step,
            &patch,
            at_work.declared(),
            at_work.off_plan(),
            &judging,
        )
        .await;
        // Read before the directive is written, never after: the reply can
        // arrive between the two, and a baseline taken afterwards would count
        // it as having been there all along — which reads as a Drone that
        // never answered.
        let rested_before = working.as_ref().map(Working::rested).unwrap_or_default();
        let stage = match found {
            Err(cause) => Stage::CouldNotLook { tripped, cause },
            Ok(Convergence::Thrashing(why)) => {
                self.told_to_report(working, job, &why).await?;
                Stage::AskedToReport {
                    tripped,
                    why,
                    asked_at: self.now(),
                }
            }
            Ok(found) => Stage::StillConverging { tripped, found },
        };
        // The look is spent whatever came back, including a call that failed:
        // asking again every tick against a Judge that is down would be the
        // schedule this tier does not have.
        if let Some(at_work) = working.as_mut() {
            match &stage {
                Stage::AskedToReport { why, asked_at, .. } => at_work.reporting(
                    asked_at.clone(),
                    rested_before,
                    why.clone(),
                    in_plan.clone(),
                ),
                _ => at_work.looked(),
            }
        }
        self.noted(job, step, &stage);
        Ok(Some(Wandering {
            job: job.clone(),
            step: step.clone(),
            stage,
        }))
    }

    /// Stage four's question: has the Drone gone quiet, and is the grace spent.
    async fn after_the_directive(
        &self,
        working: &mut Option<Working>,
        job: &JobId,
        step: &StepId,
        asked_at: &Timestamp,
        why: NotConverging,
        in_plan: &[RepoPath],
    ) -> Result<Option<Wandering>, Adrift> {
        let at_work = working.as_ref().expect("the slot was read as full");
        // **The whole distinction the trigger turns on.** A Drone that came to
        // rest, or that submitted, did what it was told — and Fleet reads only
        // that it happened, never what it said.
        if at_work.came_to_rest() || self.evidence_waiting_for(&job) > 0 {
            if let Some(at_work) = working.as_mut() {
                at_work.looked();
            }
            return Ok(None);
        }
        if elapsed(asked_at, &self.now()) < self.norms().report_grace() {
            return Ok(None);
        }
        let record = self.load(job).await?;
        if record.status() != JobStatus::Running {
            if let Some(at_work) = working.as_mut() {
                at_work.stopped();
            }
            return Ok(None);
        }
        // **The citation, re-read for nothing.** The finding named an observable
        // that had not moved; a declared path holding work that was not there
        // when the look ran is that observable moving, and a Drone answering it
        // two minutes late is not a Drone that has gone quiet. Paths and not
        // content, because a file saved twice is what thrashing looks like and a
        // file that did not exist at the look is not.
        //
        // Not a second Judge call: this asks whether the reason still holds, not
        // whether the step is converging now, and `git diff --name-only` answers
        // it. The deadline re-arms from here, so going still is still cut.
        if let Some((since, held)) = self.grown_in_plan(working, in_plan) {
            if let Some(at_work) = working.as_mut() {
                at_work.still_reporting(self.now(), held);
            }
            let stage = Stage::StillWriting { since };
            self.noted(job, step, &stage);
            return Ok(Some(Wandering {
                job: job.clone(),
                step: step.clone(),
                stage,
            }));
        }
        // Before the Job moves, and it cannot be after: the inner machine is
        // frozen beneath every status but `running`, so a step stopped after
        // the escalation would be refused and `last_verdict` would stay unwritten.
        let Some(stops) = stops_the_step() else {
            return Ok(None);
        };
        let quiet = NoReport::after(self.norms().report_grace(), asked_at, &why);
        self.store()
            .lock()
            .await
            .record_step_checks(job, step, &[quiet.recorded()], &self.now())
            .map_err(Adrift::Writing)?;
        let record = self
            .move_step(&record, step, StepTarget::Stopped(stops))
            .await?;
        self.move_job(
            &record,
            Target::Escalated(EscalationTrigger::Thrashing),
            Actor::Fleet,
        )
        .await?;
        // **The cap, and the one place Fleet stops a Drone itself.** Everything
        // else escalated is held: the process keeps its session and its
        // worktree so a person can redirect it, and holding costs nothing
        // because a Drone waiting on a person is idle. This one is not waiting.
        // It was told to stop and report, it did neither, and it is spending
        // money on a step it is not converging on — so holding it would be
        // paying to watch. The worktree survives, which is what holding was
        // protecting; what ends is the spending.
        //
        // A failure to kill is not a failure to escalate. The Job has already
        // moved and the step already carries its verdict; a process that will
        // not die is a fact about the machine, and leaving the escalation
        // unwritten because of it would lose the finding as well as the money.
        if let Some(at_work) = working.as_ref() {
            let _ = at_work.session().terminate().await;
        }
        if let Some(at_work) = working.as_mut() {
            at_work.stopped();
        }
        let stage = Stage::Escalated { quiet };
        self.noted(job, step, &stage);
        Ok(Some(Wandering {
            job: job.clone(),
            step: step.clone(),
            stage,
        }))
    }

    /// The free half. **No store, no worktree, no model** — every answer is on
    /// the slot already, which is what lets this run on every turn.
    fn tripped(&self, at_work: &Working) -> Option<Tripwire> {
        let taken = at_work.calls_this_step();
        if taken >= self.norms().calls() {
            return Some(Tripwire::ToolCalls { taken });
        }
        let after = at_work.running_for(&self.now());
        if after >= self.norms().wall_clock() {
            return Some(Tripwire::WallClock { after });
        }
        let off_plan = at_work.off_plan();
        (!off_plan.is_empty()).then(|| Tripwire::OffPlan {
            paths: off_plan.to_vec(),
        })
    }

    /// Which of the declared plan's paths the worktree is holding work at.
    ///
    /// Empty where nothing was declared — there is no plan for work to be
    /// inside — and empty where the worktree would not read. A reading nobody
    /// could take spares a Drone once rather than cutting one over it, which is
    /// the direction an unknown answer has to fail in here.
    fn in_plan(&self, at_work: &Working) -> Vec<RepoPath> {
        let Some(declared) = at_work.declared() else {
            return Vec::new();
        };
        let (_, _, worktree) = at_work.standing();
        let Ok(changed) = self.work().changed_files(&worktree) else {
            return Vec::new();
        };
        changed
            .paths()
            .into_iter()
            .filter(|path| declared.covers(path))
            .map(RepoPath::new)
            .collect()
    }

    /// What the declared plan has gained since the look, and the whole reading
    /// it was found in. `None` where it has gained nothing.
    ///
    /// The reading comes back with the answer so that the re-armed deadline is
    /// measured from what is there now: growth has to be fresh growth each
    /// window, or one new file would buy the rest of the step.
    fn grown_in_plan(
        &self,
        working: &Option<Working>,
        was: &[RepoPath],
    ) -> Option<(Vec<RepoPath>, Vec<RepoPath>)> {
        let held = self.in_plan(working.as_ref()?);
        let since: Vec<RepoPath> = held
            .iter()
            .filter(|path| !was.contains(path))
            .cloned()
            .collect();
        (!since.is_empty()).then_some((since, held))
    }

    /// Say it, into the session the slot is holding.
    async fn told_to_report(
        &self,
        working: &Option<Working>,
        job: &JobId,
        why: &NotConverging,
    ) -> Result<(), Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(());
        };
        let directive = ReportNow::about(why);
        at_work.instructed(Occasion::Report, directive.text());
        at_work
            .session()
            .interrupt(&directive)
            .await
            .map_err(|cause| Adrift::NotTold {
                job: job.clone(),
                cause,
            })
    }

    /// Write the stage into the Job's log. **Fields, never an interpolated
    /// message**, so a query can find every step the chain touched.
    fn noted(&self, job: &JobId, step: &StepId, stage: &Stage) {
        let (level, said, found) = match stage {
            Stage::StillConverging { found, .. } => (
                Level::Info,
                "the mid-step look found the step converging",
                match found {
                    Convergence::JustifiedDrift => "justified_drift",
                    _ => "converging",
                },
            ),
            Stage::AskedToReport { .. } => (
                Level::Warn,
                "the step is not converging and the Drone was told to report",
                "thrashing",
            ),
            Stage::StillWriting { .. } => (
                Level::Info,
                "the forced report has not arrived and the declared plan is still growing",
                "still_writing",
            ),
            // **`no_report` and not `thrashing`.** The finding was made two
            // minutes earlier and nothing re-checked it; what this line saw is
            // silence, and the log said the word the record says instead.
            Stage::Escalated { .. } => {
                (Level::Warn, "the forced report did not arrive", "no_report")
            }
            Stage::CouldNotLook { .. } => (
                Level::Warn,
                "the mid-step look could not be made",
                "unanswered",
            ),
        };
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("found", FieldValue::Str(found.to_string()));
        if let Some(tripped) = tripwire_of(stage) {
            envelope = envelope.with_field("tripped", FieldValue::Str(tripped.named().to_string()));
        }
        // A log line that will not write does not stop the Job: the stage is
        // on the slot, and the escalation is a transition of its own.
        let _ = transcript::note(&self.host().repo_root, job, &envelope);
    }
}

/// Which tripwire a stage was reached through, where one was.
fn tripwire_of(stage: &Stage) -> Option<&Tripwire> {
    match stage {
        Stage::StillConverging { tripped, .. }
        | Stage::AskedToReport { tripped, .. }
        | Stage::CouldNotLook { tripped, .. } => Some(tripped),
        Stage::StillWriting { .. } | Stage::Escalated { .. } => None,
    }
}

/// How long between two readings of the injected clock.
///
/// Zero where either will not read as an instant. **Never a negative
/// duration**: a clock that went backwards is a machine that is wrong, and
/// reading it as an expired grace would escalate a Job over it.
pub(crate) fn elapsed(from: &Timestamp, to: &Timestamp) -> Duration {
    match (from.epoch_millis(), to.epoch_millis()) {
        (Some(from), Some(to)) if to > from => Duration::from_millis((to - from) as u64),
        _ => Duration::ZERO,
    }
}

/// Where one step stands in the chain. Cleared when the step changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Chain {
    /// Nothing has fired.
    Working,
    /// The one look this step gets has been spent.
    Looked,
    /// The Drone was told to stop and report, and has not yet.
    ///
    /// `in_plan` is the declared plan as the look found it. What stage four asks
    /// is whether it has grown since — see
    /// [`Fleet::grown_in_plan`](crate::daemon::Fleet).
    Reporting {
        asked_at: Timestamp,
        why: NotConverging,
        in_plan: Vec<RepoPath>,
    },
    /// The step stopped and the Job escalated.
    Stopped,
}
