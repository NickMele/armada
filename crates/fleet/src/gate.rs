//! Fleet's answer to a piece of Evidence: run the step's Checks, decide, and
//! say what follows.
//!
//! # Fleet decides, and the Drone's part is over when it submits
//!
//! Everything that reaches a decision here is derived by Fleet. The Checks are
//! Fleet's own runs in the Job's worktree, the diff is Fleet's own reading of
//! that worktree, and the only thing the Drone contributed is that it called
//! the tool at all. There is no parameter on any function in this module
//! through which a Drone could supply a fact that gates its own step.
//!
//! It held when a Drone said outright that its step had failed and the step
//! advanced: reading prose catches an honest Drone and believes a dishonest
//! one. See [`rule_on`] for what should have caught it.
//!
//! # This runs after the tool call has returned
//!
//! The Evidence tool queues and returns `recorded`; this drains the queue. The
//! separation is not tidiness — a tool call that blocked while `cargo test` ran
//! would time out, and the Drone would be told nothing by a mechanism that had
//! already failed.
//!
//! # The budget is a parameter and there is no default
//!
//! Nothing in `crates/config/settings.toml` names a Check timeout, so there is
//! no value to read and inventing one here would put a threshold somewhere
//! nobody can find it. [`CheckBudget`] therefore has one constructor taking a
//! duration and no `Default`.
//!
//! # What a ruling does not do
//!
//! It does not write anything. [`apply`] turns a ruling into the Job move it
//! implies, and moving a Job is `Job::transition` and the store, in that order,
//! by the caller. Keeping the decision separate from the write is what lets
//! every case below be tested with no database.

use std::error::Error;
use std::path::Path;
use std::time::Duration;

use adapter_traits::{Footprint, WorkProduct};
use checks_runner::Output;
use core_model::{
    Actor, AdvanceGate, DeclaredPaths, EscalationTrigger, IllegalTransition, Job, Judgment,
    ResolvedCheck, StepCheck, StepEvidence, StepId, StepLevelTrigger, Target, Timestamp,
    Transitioned,
};
use verification::{
    decide, Accepted, Baseline, CheckFailed, Flagged, InScope, NotWhatTheStepAsked, Observed,
    OutcomeTurn, OutsideScope, Ran, Refusals, Submission, Verdict,
};

use crate::at_step::AtStep;
use crate::judging::{self, Judging};

/// How long a Check may run before it is a failure.
///
/// A newtype rather than a bare `Duration` so that the argument cannot be
/// confused with any other duration at a call site, and so that the one place
/// the value is decided is visible in a search. **No `Default`**: see this
/// module's comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckBudget(Duration);

impl CheckBudget {
    pub fn of(budget: Duration) -> CheckBudget {
        CheckBudget(budget)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// What one Check printed, kept for a person to read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckOutput {
    pub check: String,
    pub output: Output,
}

/// What Fleet decided, and what follows from it.
///
/// **Only [`Ruling::Advanced`], [`Ruling::Finished`] and
/// [`Ruling::HeldForReview`] are reached through [`Verdict::Advance`]**, and
/// that verdict needs evidence and a full set of passing checks. Nothing else
/// in this enum can be produced by a Drone doing anything at all — and the
/// third of the three advances nothing, so a Drone cannot reach the far side of
/// a human gate by satisfying it.
#[derive(Debug)]
pub enum Ruling {
    /// The step passed. The Drone is told and goes on to the next step. The Job
    /// stays where it is: `running` has no self-edge, and a step advancing is
    /// the inner machine, not the outer one.
    Advanced {
        tell: OutcomeTurn,
        /// Every declared Check with what it did. **Carried on a pass too**,
        /// because a step that advanced having written nothing down cannot be
        /// told from a step whose Checks were never run.
        checks: Vec<StepCheck>,
        /// What each Manifest Check printed, in the step's order.
        output: Vec<CheckOutput>,
        /// Every criterion the Judge answered. Empty on the ordinary step,
        /// which asks nothing and did not drift.
        judged: Vec<Judgment>,
    },
    /// The last step passed. The Drone is told, then terminated, and the Job
    /// reaches `completed_success`.
    Finished {
        tell: OutcomeTurn,
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
        judged: Vec<Judgment>,
    },
    /// Every tier the step declared held, and the step is gated `human_always`.
    /// **The Job reaches `awaiting_review` and the step does not move.**
    ///
    /// It is the only ruling that stops a Job without anything having gone
    /// wrong, which is why it carries no failure of any kind: the checks, the
    /// output and the judgments below are all passes, and they are carried
    /// because they are the material a person opens rather than a record of a
    /// verdict.
    ///
    /// **No turn.** The Drone is not told anything, and is not terminated
    /// either — it waits, holding its context, and the person's answer is the
    /// next turn its session gets: `approve_review` injects one,
    /// `request_changes` injects the note, `reject` ends it. A turn here would
    /// spend a Drone's remaining tool call to say "someone is looking at this".
    ///
    /// The step stays `running` while the Job stands at the gate.
    /// `ADVANCING_STATUSES` admits `awaiting_review`, so the inner machine is
    /// still live there and `approve_review` moves the step before it moves the
    /// Job. `step_machine`'s own comment says what rendering it as
    /// `awaiting_human` instead would cost.
    HeldForReview {
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
        judged: Vec<Judgment>,
    },
    /// A Check did not pass. **The Job ends**, and **the Judge never ran** —
    /// the semantic tier is asked only after the mechanical one holds, so a
    /// failing Check costs nothing. The worktree is kept, the output below is
    /// readable, and the Drone is terminated without a turn.
    Failed {
        /// Never empty.
        failures: Vec<CheckFailed>,
        /// Every declared Check with what it did, passes included.
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
    },
    /// Every Check passed and the Judge refused. **The Job escalates**, and
    /// that is what makes it different from [`Ruling::Failed`]: a Check failing
    /// says the work is broken, a refusal says the work runs and is not what
    /// was asked for — which is "stopped, and needs a person". Ending the Job
    /// would throw the verdict away, because a terminal status has nowhere to
    /// put a citation.
    ///
    /// The citation itself travels on the step, in `job_step_judgments`, and
    /// reaches the wire on [`ipc::StepDetail::judged`]. `Target::Escalated`
    /// carries the trigger and nothing else — see [`apply`].
    Refused {
        /// Never empty.
        refusals: Refusals,
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
        judged: Vec<Judgment>,
    },
    /// Every Check passed, the Judge did not refuse, and the gaming check
    /// flagged the evidence. **The Job escalates as `evidence_suspect`**, which
    /// is a different claim from a refusal: the work satisfies the step as
    /// written, and the way it satisfies it is not to be trusted.
    ///
    /// It is not a gate failure and does not route to the retry flow, because
    /// resubmission under the same instructions would reproduce the same
    /// gaming.
    Suspect {
        /// Never empty.
        flagged: Flagged,
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
        judged: Vec<Judgment>,
    },
    /// The submission was not the kind of work product the step declared.
    /// **Nothing ran and nothing moved** — the Checks are not spent on it, and
    /// the Drone is asked again.
    NotWhatTheStepAsked(NotWhatTheStepAsked),
    /// Fleet could not derive a gating artifact. **Not the Drone's doing**, and
    /// the step neither advanced nor failed: a machine that cannot answer must
    /// not produce a verdict, in either direction.
    ///
    /// **It escalates on `gate_undecided`, and the artifact is the whole
    /// point.** Nothing here used to escalate: a Job that landed in this state
    /// stayed `running` and was reached, if at all, by the liveness clock —
    /// which arrives late and knows nothing about why. So a Job sat on one step
    /// for eight minutes with a one-line log while the person watching it asked
    /// whether a Judge was running invisibly, and the truthful answer was that
    /// the gate had already tried, failed to read something, and told nobody.
    ///
    /// Escalating is not a verdict. The trigger says the gate could not decide
    /// rather than that the work failed, and `crate::settling` writes the
    /// artifact and the cause into the Job's own log, so what stopped it is
    /// readable without opening the record.
    ///
    /// **Nothing is retried.** A worktree momentarily unreadable and a Judge
    /// that cannot be handed a patch arrive here as the same value, and trying
    /// again would fix only the first — the Job stranded on 2026-08-28 was on a
    /// `facts_note` step whose Judge could not be given a patch at all. So the
    /// two are not told apart here; they are named, and handed to a person.
    CouldNotDecide {
        artifact: &'static str,
        cause: Box<dyn Error + Send + Sync>,
        /// Whatever had been established before the reading failed. Carried
        /// because a Judge call that could not be made happens *after* every
        /// Check ran, and those results are real.
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
    },
}

impl Ruling {
    /// Whether the step advanced. **The one question the whole milestone is
    /// about**, and most variants answer no.
    ///
    /// [`HeldForReview`](Ruling::HeldForReview) is the one that answers no
    /// having failed nothing: every tier held and the step still did not move,
    /// because the gate names a person. Folding it in with the two that did
    /// advance would make "the machine is satisfied" and "the step advanced"
    /// one sentence, which is exactly what a human gate separates.
    pub fn advanced(&self) -> bool {
        matches!(self, Ruling::Advanced { .. } | Ruling::Finished { .. })
    }

    /// The turn to inject, where there is one. A failure produces none: the
    /// Job is over and the Drone is terminated rather than told.
    pub fn tell(&self) -> Option<&OutcomeTurn> {
        match self {
            Ruling::Advanced { tell, .. } | Ruling::Finished { tell, .. } => Some(tell),
            _ => None,
        }
    }

    /// Every declared Check with what it did, in the step's order.
    ///
    /// Empty on the two rulings that ran nothing — a submission of the wrong
    /// kind does not spend a Check, and a gate that could not decide has no
    /// full set to report.
    pub fn checks(&self) -> &[StepCheck] {
        match self {
            Ruling::Advanced { checks, .. }
            | Ruling::Finished { checks, .. }
            | Ruling::HeldForReview { checks, .. }
            | Ruling::Failed { checks, .. }
            | Ruling::Refused { checks, .. }
            | Ruling::Suspect { checks, .. }
            | Ruling::CouldNotDecide { checks, .. } => checks,
            Ruling::NotWhatTheStepAsked(_) => &[],
        }
    }

    /// What the gaming check flagged. `None` on every ruling but one — the
    /// evidence is suspect or it is not, and there is no ruling that is both
    /// suspect and something else.
    pub fn flagged(&self) -> Option<&Flagged> {
        match self {
            Ruling::Suspect { flagged, .. } => Some(flagged),
            _ => None,
        }
    }

    /// Every criterion the Judge answered, in the order asked, with the
    /// mandatory drift look last. **Empty on most rulings**, because most steps
    /// ask nothing, stay on plan, and a failing Check never reaches the Judge.
    pub fn judged(&self) -> &[Judgment] {
        match self {
            Ruling::Advanced { judged, .. }
            | Ruling::Finished { judged, .. }
            | Ruling::HeldForReview { judged, .. }
            | Ruling::Refused { judged, .. }
            | Ruling::Suspect { judged, .. } => judged,
            Ruling::Failed { .. }
            | Ruling::NotWhatTheStepAsked(_)
            | Ruling::CouldNotDecide { .. } => &[],
        }
    }

    /// What each Manifest Check printed, in the same order.
    ///
    /// **Carried on a pass as well as a failure.** A step whose Checks all
    /// passed and a step whose Checks were never run are different sentences in
    /// the output too, not only in the record — and `diff_nonempty` runs no
    /// command, so this list is shorter than [`checks`](Ruling::checks).
    pub fn output(&self) -> &[CheckOutput] {
        match self {
            Ruling::Advanced { output, .. }
            | Ruling::Finished { output, .. }
            | Ruling::HeldForReview { output, .. }
            | Ruling::Failed { output, .. }
            | Ruling::Refused { output, .. }
            | Ruling::Suspect { output, .. }
            | Ruling::CouldNotDecide { output, .. } => output,
            Ruling::NotWhatTheStepAsked(_) => &[],
        }
    }

    /// The trigger a ruling stops the step with, and `None` where it stops no
    /// step.
    ///
    /// **The one place any of the three is named**, and what [`apply`] derives
    /// the escalation from — so the step's `last_verdict` and the Job's stored
    /// reason cannot come to disagree about why the same gate stopped.
    ///
    /// [`CouldNotDecide`](Ruling::CouldNotDecide) is here for a reason unlike
    /// the other two. The step is stopped not because it failed but because
    /// only a stopped step is one a person can act on: `crate::resume` finds
    /// the step to redirect or restart by looking for the stopped one, so a
    /// gate that could not decide and left the step `running` would escalate a
    /// Job neither act could reach.
    ///
    /// [`Ruling::Failed`] answers `None`: `job-statuses.toml` gives `stopped`
    /// to `escalated` alone, and `completed_failed`'s step machine is "frozen
    /// at the failed step" with no state named. Stopping a step under a
    /// terminal status would be this file deciding that.
    ///
    /// [`Ruling::HeldForReview`] answers `None` for the opposite reason: its
    /// step is not stopped at all. It is still `running` and still the Job's
    /// cursor, waiting on the person rather than on a trigger, and a
    /// `last_verdict` written there would say the gate failed when it held.
    pub fn stops_the_step(&self) -> Option<StepLevelTrigger> {
        match self {
            Ruling::Refused { .. } => StepLevelTrigger::of(EscalationTrigger::GateFailure),
            Ruling::Suspect { .. } => StepLevelTrigger::of(EscalationTrigger::EvidenceSuspect),
            Ruling::CouldNotDecide { .. } => StepLevelTrigger::of(EscalationTrigger::GateUndecided),
            _ => None,
        }
    }

    /// What the gate could not read, and why, where that is what stopped it.
    /// `None` on every other ruling.
    ///
    /// Handed out rather than rendered here, for
    /// [`stops_the_step`](Ruling::stops_the_step)'s reason turned around: the
    /// trigger says the gate could not decide, and this says what about — which
    /// is the half a person triages on and the half no vocabulary can hold.
    pub fn undecided(&self) -> Option<(&'static str, &(dyn Error + Send + Sync))> {
        match self {
            Ruling::CouldNotDecide {
                artifact, cause, ..
            } => Some((artifact, cause.as_ref())),
            _ => None,
        }
    }

    /// Whether the Drone's session ends here. **True only where the Job is
    /// over.**
    ///
    /// A refusal and a suspect verdict escalate, and `job-statuses.toml` gives
    /// `escalated` the Drone "alive, idle" — so the session stays, holding its
    /// context, and a redirect is a turn injected into it rather than a
    /// respawn. `crate::aftermath` is what stops that idle Drone being reaped
    /// into an `escalated -> escalated` move.
    pub fn ends_the_drone(&self) -> bool {
        matches!(self, Ruling::Finished { .. } | Ruling::Failed { .. })
    }
}

/// Run the step's Checks and decide.
///
/// The evidence comes first because nothing else happens without it: a Check is
/// not run for a submission the step did not ask for, and no path in this
/// function reaches a verdict without one.
///
/// `recorded` is what every step of this Job has submitted so far. It has two
/// readers — the gaming check's baseline and a step's `reference_docs` — and
/// both reach it through [`AtStep::baseline`], which will not answer with
/// anything but a strictly earlier step's.
/// `entered_with` is what the worktree held when this step began — **after the
/// boundary rebase that started it**, which is `crate::dispatch::Fleet::marked`'s
/// to place and not this function's. `diff_nonempty` is decided by comparing it
/// against a second reading taken here. That is what catches the step that
/// advanced having written nothing: the check used to read the whole branch and
/// count an earlier step's file as this step's work.
pub async fn rule_on<W>(
    at: AtStep<'_>,
    evidence: &Submission,
    declared: Option<&DeclaredPaths>,
    entered_with: Option<&Footprint>,
    recorded: &[(StepId, StepEvidence)],
    work: &W,
    budget: CheckBudget,
    judging: &Judging,
) -> Ruling
where
    W: WorkProduct,
    W::Error: Error + Send + Sync + 'static,
{
    let step = at.step();
    let accepted = match Accepted::of(step, evidence) {
        Ok(accepted) => accepted,
        Err(mismatch) => return Ruling::NotWhatTheStepAsked(mismatch),
    };
    let mut observed = Vec::with_capacity(step.checks().len());
    let mut output = Vec::new();
    for check in step.checks() {
        match check {
            ResolvedCheck::ManifestCheck { name, run, .. } => {
                let attempt =
                    checks_runner::run(run, Path::new(at.worktree().path()), budget.duration())
                        .await;
                observed.push(Observed::Command(attempt.exit));
                output.push(CheckOutput {
                    check: name.clone(),
                    output: attempt.output,
                });
            }
            // **Against the step's own start, never the branch's.** A step that
            // wrote nothing used to pass this on the files an earlier step
            // committed; `entered_with` is what the worktree held when this
            // step began, and the difference is what this step did.
            //
            // "Began" includes the boundary rebase, which is why a step that
            // resolves none of a conflict fails here: the markers are in the
            // baseline, so nothing about them differs.
            //
            // A step with no baseline is one Fleet never saw start — a Drone
            // adopted mid-flight, or a slot that lost its reading. The honest
            // answer there is that nothing is known to have moved, which fails
            // the check rather than passing it: an unread baseline must not
            // advance a step, for `Changed::nothing`'s reason.
            ResolvedCheck::DiffNonempty => match work.footprint(at.worktree()) {
                Ok(now) => observed.push(Observed::Diff {
                    moved: entered_with.is_some_and(|before| now.differs_from(before)),
                }),
                Err(cause) => {
                    return Ruling::CouldNotDecide {
                        artifact: "the Job's diff",
                        cause: Box::new(cause),
                        checks: Vec::new(),
                        output,
                    }
                }
            },
        }
    }

    let ran = match Ran::of(step, &observed) {
        Ok(ran) => ran,
        // Unreachable while the loop above emits one observation per check, in
        // order, of the kind that check takes. It is carried rather than
        // unwrapped because an unreachable `expect` in the gate is exactly the
        // place a panic would take Fleet down mid-Job.
        Err(cause) => {
            return Ruling::CouldNotDecide {
                artifact: "the step's checks",
                cause: Box::new(cause),
                checks: Vec::new(),
                output,
            }
        }
    };

    let mut checks = ran.recorded();
    let mechanical = decide(accepted, &ran);
    // The scope tier, and it answers into two tiers rather than one.
    // `docs/concepts/judge.md` gives declared plan drift to the Judge and says
    // it does not fail the step, because legitimate investigation sometimes
    // moves the work. What stays mechanical is the other two: nothing drifted
    // where nothing was declared, and a denylist a model could excuse is not a
    // denylist.
    //
    // This used to fold drift in here too, arguing that a step which did
    // another step's work should never reach a model call. That was true about
    // the cost and wrong about the consequence — the Judge is asked only where
    // the mechanical tier held, so the same line made the mandatory look
    // unreachable.
    //
    // **Cold unless the step declares an evidence scope**, which is what leaves
    // a step without one behaving exactly as it did before.
    let (scope, off_plan) = match step.evidence_scope() {
        None => (None, Vec::new()),
        Some(scope) => match work.changed_files(at.worktree()) {
            Ok(changed) => match InScope::resolved(scope, declared, &changed.paths()) {
                Ok(_) => (None, Vec::new()),
                Err(OutsideScope::Undeclared { changed }) => (None, changed),
                // A variant added to `OutsideScope` lands here and fails the
                // step, which is the safe default: drift is named, not
                // inferred.
                Err(outside) => (Some(CheckFailed::OutOfScope(outside)), Vec::new()),
            },
            Err(cause) => {
                return Ruling::CouldNotDecide {
                    artifact: "the Job's changed files",
                    cause: Box::new(cause),
                    checks,
                    output,
                }
            }
        },
    };
    checks.extend(scope.as_ref().and_then(CheckFailed::recorded));
    let mechanical = mechanical.and_also(scope);
    // **The trigger, and the whole of it.** The Judge is asked where the
    // mechanical tier held and either the step declares a criterion or the step
    // drifted — the second being the one look `judge.md` calls mandatory, which
    // fires on a step that declares no criterion of its own.
    let asked = step.asks_the_judge() || !off_plan.is_empty();
    let (judged, verdict) = match mechanical.advanced() && asked {
        false => (Vec::new(), mechanical),
        true => {
            let patch = match work.patch(at.worktree()) {
                Ok(patch) => patch,
                Err(cause) => {
                    return Ruling::CouldNotDecide {
                        artifact: "the step's patch",
                        cause: Box::new(cause),
                        checks,
                        output,
                    }
                }
            };
            match judging::judged(at, accepted, &patch, &checks, &off_plan, recorded, judging).await
            {
                Ok((judged, refusals)) => (judged, mechanical.but_for(refusals)),
                // A verification that could not run is not a refusal, and it is
                // not a pass. The step neither advances nor fails.
                Err(cause) => {
                    return Ruling::CouldNotDecide {
                        artifact: "the Judge's answer",
                        cause: Box::new(cause),
                        checks,
                        output,
                    }
                }
            }
        }
    };

    // **The gaming check, and the only place it fires.** It runs where the step
    // would otherwise advance, because that is the case it exists for: a
    // Mechanical Check passes gamed evidence by design, and a step already
    // stopped needs no second reason. It cannot take an advance away by
    // failing the step — it routes elsewhere entirely.
    if verdict.advanced() {
        if let Some(suspect) = suspect(at, work, recorded, judging, &checks, &output, &judged).await
        {
            return suspect;
        }
    }

    match verdict {
        // **The advance gate is read here and nowhere earlier**, which is what
        // makes a human gate cost the same as an auto one: every tier the step
        // declared has already run, and what the gate decides is only who acts
        // on the result. A gate read before the tiers would be a step whose
        // Checks a person waits on after deciding.
        //
        // Matched exhaustively rather than through a helper, so a fourth gate
        // is a compile error here rather than a step that quietly advances.
        Verdict::Advance => match step.advance_gate() {
            AdvanceGate::HumanAlways => Ruling::HeldForReview {
                checks,
                output,
                judged,
            },
            AdvanceGate::Auto | AdvanceGate::AutoIfJudgePasses => match at.next() {
                Some(next) => Ruling::Advanced {
                    tell: OutcomeTurn::advanced(step, Some(next)),
                    checks,
                    output,
                    judged,
                },
                None => Ruling::Finished {
                    tell: OutcomeTurn::advanced(step, None),
                    checks,
                    output,
                    judged,
                },
            },
        },
        Verdict::Failed(failures) => Ruling::Failed {
            failures,
            checks,
            output,
        },
        Verdict::Refused(refusals) => Ruling::Refused {
            refusals,
            checks,
            output,
            judged,
        },
    }
}

/// The gaming look, where the step declares one. `None` is nothing flagged.
///
/// Its own function because it is the one part of the gate whose answer is not
/// a [`Verdict`]: it returns a whole [`Ruling`] or nothing, and there is no
/// value it could hand back that `Verdict::but_for` would accept.
async fn suspect<W>(
    at: AtStep<'_>,
    work: &W,
    recorded: &[(StepId, StepEvidence)],
    judging: &Judging,
    checks: &[StepCheck],
    output: &[CheckOutput],
    // `judged` is carried onto the ruling because a step whose Judge cleared
    // it and a step whose Judge never ran are different facts, and a gaming
    // flag must not erase the first.
    judged: &[Judgment],
) -> Option<Ruling>
where
    W: WorkProduct,
    W::Error: Error + Send + Sync + 'static,
{
    let step = at.step();
    if !step.asks_about_gaming() {
        return None;
    }
    let patch = match work.patch(at.worktree()) {
        Ok(patch) => patch,
        Err(cause) => {
            return Some(Ruling::CouldNotDecide {
                artifact: "the step's patch",
                cause: Box::new(cause),
                checks: checks.to_vec(),
                output: output.to_vec(),
            })
        }
    };
    // A step may name a baseline and have none: the named step may not have
    // recorded evidence, and a first step has no earlier step at all. The
    // check still runs, and the brief says so.
    let named = step
        .judge_checks()
        .iter()
        .filter_map(|check| check.gaming())
        .find_map(|gaming| gaming.baseline())
        .and_then(|reference| at.baseline(reference, recorded));
    let baseline = named.map(|(step, evidence)| Baseline::of(step.as_str(), evidence));
    match judging::gaming(step, &patch, baseline, judging).await {
        Ok(None) => None,
        Ok(Some(flagged)) => Some(Ruling::Suspect {
            flagged,
            checks: checks.to_vec(),
            output: output.to_vec(),
            judged: judged.to_vec(),
        }),
        Err(cause) => Some(Ruling::CouldNotDecide {
            artifact: "the gaming check's answer",
            cause: Box::new(cause),
            checks: checks.to_vec(),
            output: output.to_vec(),
        }),
    }
}

/// The Job move a ruling implies, or `None` where the Job does not move.
///
/// **The actor is always Fleet.** A Drone is never the actor on a transition
/// its own evidence led to, which is what the recorded event has to say: the
/// evidence was a signal and Fleet made the decision.
///
/// # A failure and a refusal go to different statuses
///
/// A Check failing is the work being broken and is terminal. A refusal is the
/// work running and not being what was asked for, which is a person's to
/// answer — so it is `escalated`, from which redispatch and Pilot are both
/// reachable and `completed_failed` still is, once a person agrees.
///
/// The trigger comes from [`Ruling::stops_the_step`] and is not spelled here:
/// the step that stopped and the Job that escalated are stating one fact, and
/// two spellings of it could drift. `gate_failure` is what
/// `docs/concepts/judge.md` picks for the step evidence gate in as many words;
/// `evidence_suspect` is the gaming check's alone, because a Judge that refused
/// a criterion accused nobody; `gate_undecided` is neither, because the work was
/// never weighed. All three are step-level, which is what lets each reach
/// `last_verdict`.
pub fn apply(
    job: &Job,
    ruling: &Ruling,
    at: Timestamp,
) -> Option<Result<Transitioned, IllegalTransition>> {
    let target = match ruling {
        Ruling::Finished { .. } => Target::CompletedSuccess,
        Ruling::Failed { .. } => Target::CompletedFailed,
        // The one move in this function that is not an ending. `running ->
        // awaiting_review` is the edge `fleet::reviewing`'s three acts all
        // start from, and Fleet is the actor: the person has not answered yet,
        // they have only been asked.
        Ruling::HeldForReview { .. } => Target::AwaitingReview,
        // **The rulings that escalate are exactly the rulings that stop the
        // step**, which is why this reads the trigger off that answer instead
        // of naming one. `None` covers the three that move nothing: a step
        // advancing inside a running Job moves the inner machine and not the
        // outer one, and the other two move neither.
        _ => Target::Escalated(ruling.stops_the_step()?.trigger()),
    };
    Some(job.transition(target, Actor::Fleet, at))
}
