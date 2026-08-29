//! A Drone asking whether its work passes, and Fleet running the step's Checks
//! to answer.
//!
//! **The allowlist is why this exists and is not the fix.** `--allowedTools` is
//! a permission list rather than a toolset, so a Drone granted `cargo fmt` and
//! nothing else has every other invocation denied *silently*, which reads as a
//! tool that does not work. Widening it would run a command the workflow did
//! not freeze. Fleet runs the Checks; a Drone can now ask it to.
//!
//! **A signal, and no path from here to a pass.** Nothing below writes a Check
//! row, records evidence or moves a step, and the gate runs every Check again
//! for itself. Output goes to `<step>.dry.<n>.log` and never to the gate's
//! `<step>.<n>.log`, so no record ends up naming a file no gate wrote.
//!
//! **The clocks suspend while it runs, which is what needs two bounds.** A
//! Drone waiting on Fleet is not silent and is not thrashing — `#58` settled
//! that for evidence at the gate — but they were also the only thing bounding
//! the cost, and `cargo build --workspace` is minutes.
//!
//! | Bound | What it stops |
//! |---|---|
//! | A refusal while one runs | Two builds in one worktree, neither answer about the work |
//! | [`DryRuns`], per step | Ask, change a line, ask again, for as long as the step lasts |
//!
//! **Which Checks is not the call's choice.** It takes no arguments at all.

use std::fmt;
use std::path::Path;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Footprint, Vcs, WorkProduct, Worktree};
use core_model::{Component, Envelope, FieldValue, Job, Level, ResolvedCheck, StepId};
use ipc::mcp::{CheckRan, CheckReport};
use verification::{Observed, Ran};

use crate::check_output;
use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::transcript;

/// How many times one step may ask.
///
/// **A newtype with one constructor and no `Default`**, for [`CheckBudget`]'s
/// reason: a threshold invented at a call site is a threshold nobody can find.
/// The composition root names it once and says there what it is worth.
///
/// [`CheckBudget`]: crate::CheckBudget
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DryRuns(u32);

impl DryRuns {
    pub const fn of(allowed: u32) -> DryRuns {
        DryRuns(allowed)
    }

    pub fn allowed(&self) -> u32 {
        self.0
    }
}

/// Why the Checks were not run.
///
/// **No variant is a gate failure**, and none of them stops or advances
/// anything: the call was aimed at nothing, at a step with no Checks, or at a
/// budget that is spent. Every one reaches the Drone as a tool error it can
/// read and act on.
///
/// Its own type rather than a variant of `Adrift` or a sibling of
/// `NotSubmitted`: those are `crate::adrift`'s, this refuses a different act,
/// and a module that has to be opened to add a refusal is a module two changes
/// collide in.
#[derive(Debug)]
pub enum NotRun {
    /// No Job is being worked, so there is no step whose Checks these would be.
    NothingIsWorking,
    /// The Job is standing at a step its frozen workflow does not name. **A
    /// fault in Fleet, not in the call.**
    NoSuchStep { step: StepId },
    /// The step declares no mechanical Checks. Refused rather than answered
    /// with an empty report: a report with no rows reads as a run that found
    /// nothing wrong.
    StepHasNoChecks { step: StepId },
    /// A run is already in flight for this step. **Refused rather than
    /// queued** — two builds in one worktree contend for one target directory,
    /// and neither result would be about the work.
    AlreadyRunning,
    /// The Drone has already submitted, and the gate is about to run the same
    /// Checks itself.
    ///
    /// **The third bound, and the only one that is not about cost.** The gate
    /// runs in the worktree the dry run would run in, so answering here would
    /// be two builds in one directory the same way a second dry run would — and
    /// the answer the Drone is waiting for is the gate's, which arrives as a
    /// turn. It does not close the case the other way round: a Drone that
    /// submits while a run is in flight is a real ordering and the gate does
    /// not refuse it.
    AlreadySubmitted,
    /// The step has spent its allowance.
    Spent { allowed: u32 },
    /// The worktree could not be read, so `diff_nonempty` has no answer.
    /// **Refused before anything is spent**, which is why the reading is taken
    /// first: a report that guessed at this would tell a Drone its work changed
    /// nothing on the strength of a failed read.
    CouldNotRead { cause: String },
}

impl fmt::Display for NotRun {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotRun::NothingIsWorking => out.write_str(
                "no Job is being worked, so there are no checks to run. Stop — \
                 the task this Drone was started for has already ended",
            ),
            NotRun::NoSuchStep { step } => write!(
                out,
                "the task is standing at step `{}`, which its workflow does not \
                 name. This is a fault in Fleet and not in the call",
                step.as_str()
            ),
            NotRun::StepHasNoChecks { step } => write!(
                out,
                "step `{}` declares no mechanical checks, so there is nothing to \
                 run. Get on with the work and submit when it is done",
                step.as_str()
            ),
            NotRun::AlreadyRunning => out.write_str(
                "the checks are already running for this part. Wait for the call \
                 you have made to come back — a second run would be two builds \
                 in one worktree and neither answer would be about your work",
            ),
            NotRun::AlreadySubmitted => out.write_str(
                "you have submitted, and the checks are about to be run against \
                 your work. Wait — the outcome arrives as a later turn, and \
                 running them again now would be two runs in one worktree",
            ),
            NotRun::Spent { allowed } => write!(
                out,
                "this part has already asked for the checks {allowed} times, \
                 which is all it gets. Finish the work and submit — the checks \
                 are run again then, and that run is the one that decides"
            ),
            NotRun::CouldNotRead { cause } => write!(
                out,
                "your worktree could not be read, so the checks were not run and \
                 nothing has been spent: {cause}. Try again"
            ),
        }
    }
}

impl std::error::Error for NotRun {}

/// What one dry run is against: read under the slot lock, held while the lock
/// is not.
///
/// **A value rather than a borrow**, because the Checks are minutes and the
/// slot lock is read four times a second. A run that held it would stop Fleet
/// turning at all — no gate, no vigil, no live file list — for the length of a
/// `cargo build`.
struct Plan {
    record: Job,
    step: StepId,
    worktree: Worktree,
    /// What the worktree held when the step began, for `diff_nonempty`.
    entered_with: Option<Footprint>,
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
    /// Run the step's Checks and say what each one did. **Nothing moves.**
    ///
    /// The Drone names no Job, no step and no Check; all three are read out of
    /// the working slot and the Job's own frozen workflow, which is
    /// `Fleet::declare_scope`'s binding and for the same reason.
    pub async fn run_checks(&self) -> Result<CheckReport, NotRun> {
        let plan = self.dry_run_begins().await?;
        let ran = self.dry_run(&plan).await;
        // **Before the result is returned, on both roads out.** A run that
        // failed to end would leave the clocks suspended for the rest of the
        // step, which is the tripwires switched off by an error path.
        self.dry_run_ends(&plan).await;
        let report = ran?;
        self.noted_dry_run(&plan, &report);
        Ok(report)
    }

    /// Everything decided under the slot lock: whether there is a run to make,
    /// and what it is against.
    ///
    /// **The mark goes on here**, inside the same lock that read the count, so
    /// two calls arriving together cannot both find the budget unspent.
    async fn dry_run_begins(&self) -> Result<Plan, NotRun> {
        let mut working = self.slot().lock().await;
        let Some(at_work) = working.as_mut() else {
            return Err(NotRun::NothingIsWorking);
        };
        if at_work.is_checking() {
            return Err(NotRun::AlreadyRunning);
        }
        // Cheaper than the rest and true regardless of them: the gate is about
        // to run these same Checks in this same worktree.
        if self.evidence_waiting() > 0 {
            return Err(NotRun::AlreadySubmitted);
        }
        let allowed = self.dry_runs().allowed();
        if at_work.dry_runs() >= allowed {
            return Err(NotRun::Spent { allowed });
        }
        let (job, step, worktree) = at_work.standing();
        let entered_with = at_work.entered_with().cloned();
        let record = self
            .load(&job)
            .await
            .map_err(|_| NotRun::NoSuchStep { step: step.clone() })?;
        let Some(declared) = record.workflow().step(&step) else {
            return Err(NotRun::NoSuchStep { step });
        };
        if declared.checks().is_empty() {
            return Err(NotRun::StepHasNoChecks { step });
        }
        at_work.checking(self.now());
        Ok(Plan {
            record,
            step,
            worktree,
            entered_with,
        })
    }

    /// Take the mark off and give the clocks back the time.
    ///
    /// **Guarded on the step**, because the gate can advance one while a run is
    /// in flight: `Working::now_on` has already cleared the mark for the step
    /// that ended, and crediting the new step with the old one's minutes would
    /// hand it a wall clock it did not earn.
    async fn dry_run_ends(&self, plan: &Plan) {
        let now = self.now();
        let mut working = self.slot().lock().await;
        if let Some(at_work) = working.as_mut() {
            let (job, step, _) = at_work.standing();
            if job == *plan.record.id() && step == plan.step {
                at_work.checked(now);
            }
        }
    }

    /// The run itself, with no lock held.
    async fn dry_run(&self, plan: &Plan) -> Result<CheckReport, NotRun> {
        let Some(declared) = plan.record.workflow().step(&plan.step) else {
            return Err(NotRun::NoSuchStep {
                step: plan.step.clone(),
            });
        };
        // **First, and before anything is spent.** It is the one observation
        // that can fail to be made at all, and a Drone told its work changed
        // nothing on the strength of a failed read has been told something
        // false about its own worktree.
        let moved = match declared
            .checks()
            .iter()
            .any(|check| matches!(check, ResolvedCheck::DiffNonempty))
        {
            false => false,
            true => match self.work().footprint(&plan.worktree) {
                Ok(now) => plan
                    .entered_with
                    .as_ref()
                    .is_some_and(|before| now.differs_from(before)),
                Err(cause) => {
                    return Err(NotRun::CouldNotRead {
                        cause: cause.to_string(),
                    })
                }
            },
        };
        // **The same skip the gate takes, from the same reading.** A dry run
        // that ran a Check the gate will not run would tell a Drone its work
        // failed something no gate is going to ask — and this report's closing
        // sentence promises the opposite: the same Checks, run by Fleet.
        let touched: Vec<String> = match declared
            .checks()
            .iter()
            .any(ResolvedCheck::needs_changed_paths)
        {
            false => Vec::new(),
            true => match self.work().changed_files(&plan.worktree) {
                Ok(changed) => changed.paths(),
                Err(cause) => {
                    return Err(NotRun::CouldNotRead {
                        cause: cause.to_string(),
                    })
                }
            },
        };
        let mut observed = Vec::with_capacity(declared.checks().len());
        let mut printed = Vec::new();
        let mut took = Vec::with_capacity(declared.checks().len());
        for check in declared.checks() {
            let began = self.now();
            if let Some(skip) = crate::gate::not_covered(check, &touched) {
                observed.push(skip);
                took.push(elapsed(&began, &self.now()));
                continue;
            }
            match check {
                ResolvedCheck::ManifestCheck { name, run, .. } => {
                    let attempt = checks_runner::run(
                        run,
                        Path::new(plan.worktree.path()),
                        self.budget().duration(),
                    )
                    .await;
                    observed.push(Observed::Command(attempt.exit));
                    printed.push((name.clone(), attempt.output));
                }
                ResolvedCheck::DiffNonempty => observed.push(Observed::Diff { moved }),
            }
            took.push(elapsed(&began, &self.now()));
        }
        // Unreachable while the loop above emits one observation per check, in
        // order, of the kind that check takes. Carried rather than unwrapped
        // for `gate::rule_on`'s reason: an unreachable `expect` on the Drone's
        // own call path is where a panic takes Fleet down mid-Job.
        let ran = Ran::of(declared, &observed).map_err(|cause| NotRun::CouldNotRead {
            cause: cause.to_string(),
        })?;
        // Which run of this step the output belongs to. A reattempt's dry runs
        // must not overwrite the ones before it, for the reason the gate's own
        // path carries the attempt: `job_step_checks` is keyed by attempt, and
        // a file named without one leaves an earlier row pointing at a later
        // run's output.
        let on = self
            .store()
            .lock()
            .await
            .step_attempt(plan.record.id(), &plan.step)
            .map_err(|cause| NotRun::CouldNotRead {
                cause: cause.to_string(),
            })?;
        let rows = check_output::kept_dry(
            &self.host().repo_root,
            plan.record.id(),
            &plan.step,
            on,
            &ran.recorded(),
            &printed,
        );
        Ok(CheckReport {
            ran: rows
                .into_iter()
                .enumerate()
                .map(|(at, row)| CheckRan {
                    name: row.name,
                    outcome: row.outcome.into(),
                    // The failure's own sentence, kept rather than re-derived —
                    // which lines of a run were the failure is not a question
                    // anything here answers, and the log path beside it is how
                    // a Drone finds out.
                    detail: row.produced,
                    took: took.get(at).copied().unwrap_or(Duration::ZERO),
                    log: row.output_path,
                })
                .collect(),
        })
    }

    /// Write the run into the Job's log. **Fields, never an interpolated
    /// message**, so a query can find every step that asked and how often.
    ///
    /// It is a log line and not an event: a Check running is not a transition,
    /// and one run for the Drone's own information is less of one than the
    /// gate's.
    fn noted_dry_run(&self, plan: &Plan, report: &CheckReport) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "the Drone asked for the step's checks and they were run",
        )
        .in_job(plan.record.id().as_ulid().clone())
        .at_step(plan.step.as_str())
        .with_field("ran", FieldValue::Int(report.ran.len() as i64))
        .with_field("failed", FieldValue::Int(report.failed() as i64));
        // A log line that will not write does not fail the call: the Drone has
        // its answer, and nothing about the Job moved either way.
        let _ = transcript::note(&self.host().repo_root, plan.record.id(), &envelope);
    }
}
