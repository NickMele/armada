//! Fleet's answer to a piece of Evidence: run the step's Checks, decide, and
//! say what follows.
//!
//! # Fleet decides, and the Drone's part is over when it submits
//!
//! Everything that reaches a decision here is derived by Fleet. The Checks are
//! Fleet's own runs in the Job's worktree, the diff is Fleet's own reading of
//! that worktree, and the only thing the Drone contributed is that it called
//! the tool at all — plus, on a `facts_note` step, the note itself. There is no
//! parameter on any function in this module through which a Drone could supply
//! a fact that gates its own step.
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

use adapter_traits::{WorkProduct, Worktree};
use checks_runner::Output;
use core_model::{
    Actor, FrozenWorkflow, IllegalTransition, Job, ResolvedCheck, ResolvedStep, StepCheck, StepId,
    Target, Timestamp, Transitioned,
};
use verification::{
    decide, Accepted, CheckFailed, NotWhatTheStepAsked, Observed, OutcomeTurn, Ran, Submission,
    Verdict,
};

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

/// Where a Job is: which step of its frozen workflow, and the worktree the work
/// is in.
///
/// **There is no constructor taking a step index.** A position comes from a
/// step id the workflow actually declares, so a gate cannot be pointed at a
/// step that is not in the definition the Job froze.
#[derive(Clone, Copy, Debug)]
pub struct AtStep<'a> {
    workflow: &'a FrozenWorkflow,
    at: usize,
    worktree: &'a Worktree,
}

impl<'a> AtStep<'a> {
    /// The first step, where a Job starts. `None` for a workflow with no steps.
    pub fn first(workflow: &'a FrozenWorkflow, worktree: &'a Worktree) -> Option<AtStep<'a>> {
        (!workflow.steps().is_empty()).then_some(AtStep {
            workflow,
            at: 0,
            worktree,
        })
    }

    /// A named step. `None` where the workflow declares no step by that id.
    pub fn named(
        workflow: &'a FrozenWorkflow,
        step: &StepId,
        worktree: &'a Worktree,
    ) -> Option<AtStep<'a>> {
        let at = workflow.steps().iter().position(|s| s.id() == step)?;
        Some(AtStep {
            workflow,
            at,
            worktree,
        })
    }

    /// The step being gated.
    pub fn step(&self) -> &'a ResolvedStep {
        &self.workflow.steps()[self.at]
    }

    /// The step after it, or `None` at the last one.
    pub fn next(&self) -> Option<&'a ResolvedStep> {
        self.workflow.steps().get(self.at + 1)
    }

    /// Where the Job is once this step has advanced. `None` at the last step,
    /// which is the workflow being finished rather than a position.
    pub fn advanced(&self) -> Option<AtStep<'a>> {
        self.next().map(|_| AtStep {
            workflow: self.workflow,
            at: self.at + 1,
            worktree: self.worktree,
        })
    }

    pub fn worktree(&self) -> &'a Worktree {
        self.worktree
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
/// **Only [`Ruling::Advanced`] and [`Ruling::Finished`] are reached through
/// [`Verdict::Advance`]**, and that verdict needs evidence and a full set of
/// passing checks. Nothing else in this enum can be produced by a Drone doing
/// anything at all.
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
    },
    /// The last step passed. The Drone is told, then terminated, and the Job
    /// reaches `completed_success`.
    Finished {
        tell: OutcomeTurn,
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
    },
    /// A Check did not pass. **The Job ends**: no Judge, no retry, no
    /// escalation. The worktree is kept, the output below is readable, and the
    /// Drone is terminated without a turn.
    Failed {
        /// Never empty.
        failures: Vec<CheckFailed>,
        /// Every declared Check with what it did, passes included.
        checks: Vec<StepCheck>,
        output: Vec<CheckOutput>,
    },
    /// The submission was not the kind of work product the step declared.
    /// **Nothing ran and nothing moved** — the Checks are not spent on it, and
    /// the Drone is asked again.
    NotWhatTheStepAsked(NotWhatTheStepAsked),
    /// Fleet could not derive a gating artifact. **Not the Drone's doing**, and
    /// the step neither advanced nor failed: a machine that cannot answer must
    /// not produce a verdict, in either direction.
    ///
    /// Nothing here escalates, because this milestone step has no escalation.
    /// A Job that lands in this state stays `running` and is reached by the
    /// liveness clock rather than by the gate, which is a gap and is named as
    /// one.
    CouldNotDecide {
        artifact: &'static str,
        cause: Box<dyn Error + Send + Sync>,
    },
}

impl Ruling {
    /// Whether the step advanced. **The one question the whole milestone is
    /// about**, and three of the five variants answer no.
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
            | Ruling::Failed { checks, .. } => checks,
            Ruling::NotWhatTheStepAsked(_) | Ruling::CouldNotDecide { .. } => &[],
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
            | Ruling::Failed { output, .. } => output,
            Ruling::NotWhatTheStepAsked(_) | Ruling::CouldNotDecide { .. } => &[],
        }
    }

    /// Whether the Drone's session ends here. True where the Job is over, in
    /// either direction, and false everywhere else.
    pub fn ends_the_drone(&self) -> bool {
        matches!(self, Ruling::Finished { .. } | Ruling::Failed { .. })
    }
}

/// Run the step's Checks and decide.
///
/// The evidence comes first because nothing else happens without it: a Check is
/// not run for a submission the step did not ask for, and no path in this
/// function reaches a verdict without one.
pub async fn rule_on<W>(
    at: AtStep<'_>,
    evidence: &Submission,
    work: &W,
    budget: CheckBudget,
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
            ResolvedCheck::DiffNonempty => match work.changed_files(at.worktree()) {
                Ok(changed) => observed.push(Observed::Diff {
                    changed_files: changed.len(),
                }),
                Err(cause) => {
                    return Ruling::CouldNotDecide {
                        artifact: "the Job's diff",
                        cause: Box::new(cause),
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
            }
        }
    };

    let checks = ran.recorded();
    match decide(accepted, &ran) {
        Verdict::Advance => match at.next() {
            Some(next) => Ruling::Advanced {
                tell: OutcomeTurn::advanced(step, Some(next)),
                checks,
                output,
            },
            None => Ruling::Finished {
                tell: OutcomeTurn::advanced(step, None),
                checks,
                output,
            },
        },
        Verdict::Failed(failures) => Ruling::Failed {
            failures,
            checks,
            output,
        },
    }
}

/// The Job move a ruling implies, or `None` where the Job does not move.
///
/// **The actor is always Fleet.** A Drone is never the actor on a transition
/// its own evidence led to, which is what the recorded event has to say: the
/// evidence was a signal and Fleet made the decision.
pub fn apply(
    job: &Job,
    ruling: &Ruling,
    at: Timestamp,
) -> Option<Result<Transitioned, IllegalTransition>> {
    let target = match ruling {
        Ruling::Finished { .. } => Target::CompletedSuccess,
        Ruling::Failed { .. } => Target::CompletedFailed,
        // A step advancing inside a running Job moves the inner machine and not
        // the outer one, and the other two rulings move nothing at all.
        Ruling::Advanced { .. }
        | Ruling::NotWhatTheStepAsked(_)
        | Ruling::CouldNotDecide { .. } => return None,
    };
    Some(job.transition(target, Actor::Fleet, at))
}
