//! A Drone asking whether its work passes, and Fleet running the step's Checks
//! to answer. **A signal, and no path to a pass**: nothing here writes a Check
//! row or moves a step, and output goes to `<step>.dry.<n>.log`, never the
//! gate's `<step>.<n>.log`.
//!
//! **The call answers at once, and the report is a later turn** (#1020). A
//! build outlasts what the agent CLI waits on one call, and a run tied to the
//! request died with the connection and left the slot marked. So the run is
//! Fleet's, and the task [`Fleet::run_checks`] starts takes the mark off.
//!
//! | Bound | What it stops |
//! |---|---|
//! | A refusal while one runs | Two builds in one worktree |
//! | [`DryRuns`], per step | Ask, change a line, ask again, for the whole step |
//! | A step that ends mid-run | A report on whatever follows; its Checks stop |

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Footprint, Vcs, WorkProduct, Worktree};
use core_model::{
    Attempt, Component, Envelope, FieldValue, Job, JobId, Level, ResolvedCheck, StepId, TaskCounts,
};
use ipc::mcp::{CheckRan, CheckReport};
use tokio::task::JoinHandle;
use verification::Ran;

use crate::check_output;
use crate::checking::Stop;
use crate::daemon::Fleet;
use crate::session::{LiveSession, Occasion};
use crate::working::Working;

/// How many times one step may ask. **One constructor and no `Default`**, for
/// [`CheckBudget`](crate::CheckBudget)'s reason.
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

/// Why the Checks were not run. **None is a gate failure**, none moves
/// anything, and each reaches the Drone at once as a tool error it can act on.
#[derive(Debug)]
pub enum NotRun {
    /// No Job is being worked, so there is no step whose Checks these would be.
    NothingIsWorking,
    /// The Job stands at a step its frozen workflow does not name: Fleet's fault.
    NoSuchStep { step: StepId },
    /// Refused, not answered empty: a report with no rows reads as a clean run.
    StepHasNoChecks { step: StepId },
    /// Refused, not queued: two builds in one worktree answer about neither.
    AlreadyRunning,
    /// The gate is about to run the same Checks in the same worktree. The other
    /// way round, a submission stops a run already going.
    AlreadySubmitted,
    /// The step has spent its allowance.
    Spent { allowed: u32 },
    /// Fleet holds no pipe into this Drone, adopted after a restart, so the
    /// report could never arrive. Refused before anything is spent.
    Unheard,
    /// A narrowed run of a worktree holding no change, refused before anything is spent.
    NothingChanged,
    /// A reading failed. Refused before anything is spent: every read precedes the mark.
    CouldNotRead { cause: String },
}

impl fmt::Display for NotRun {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotRun::NothingIsWorking => out.write_str(
                "no Job is being worked, so there are no checks to run. Stop — \
                 the Job this Drone was started for has already ended",
            ),
            NotRun::NoSuchStep { step } => write!(
                out,
                "the Job is standing at step `{}`, which its workflow does not \
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
                "the checks are already running for this part. Wait for their \
                 report — it arrives as a later turn, and a second run would be \
                 two builds in one worktree and neither answer would be about \
                 your work",
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
            NotRun::Unheard => out.write_str(
                "Fleet restarted while this part was going and can no longer send \
                 you a later turn, so the checks were not run and nothing has been \
                 spent. Finish the work and submit — the checks are run again then",
            ),
            NotRun::NothingChanged => out.write_str(
                "you asked for the checks against what you have changed, and \
                 your worktree holds no change yet. Nothing was run and nothing \
                 has been spent — do some of the work and ask again, or ask \
                 with `only_what_changed` false for the run the gate will make",
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

/// A run Fleet has started and owns. **Dropping this leaves it going.**
#[derive(Debug)]
pub struct ChecksRunning(JoinHandle<Option<Result<CheckReport, String>>>);

impl ChecksRunning {
    /// What the Drone was told, or `None` where the step ended before the run.
    pub async fn finished(self) -> Option<Result<CheckReport, String>> {
        self.0.await.ok().flatten()
    }
}

/// The later turn a run's report arrives as, built from the run and nothing else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChecksReported(String);

impl ChecksReported {
    fn of(ran: &Result<CheckReport, String>) -> ChecksReported {
        const HEADING: &str = "THE CHECKS YOU ASKED FOR";
        ChecksReported(match ran {
            Ok(report) => {
                format!("{HEADING}\n\nThey have finished. This is what each one did:\n\n{report}")
            }
            Err(cause) => format!(
                "{HEADING}\n\nThey ran, and no report could be made of them: {cause}. \
                 This is a fault in Fleet and not in your work. Ask again, or submit \
                 when the work is done."
            ),
        })
    }

    /// The turn, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// Which run is in flight, so the task ending one cannot end another.
pub(crate) static RUNS: AtomicU64 = AtomicU64::new(0);

/// What one dry run is against: read under the slot lock, held while it is not.
struct Plan {
    record: Job,
    step: StepId,
    worktree: Worktree,
    entered_with: Option<Footprint>,
}

/// What the run reads before anything is spent, so a failed read refuses at once.
struct Readings {
    narrow: bool,
    moved: bool,
    touched: Vec<String>,
    ports: BTreeMap<String, u16>,
    port_env: Vec<(String, String)>,
    tasks: Option<TaskCounts>,
    attempt: Attempt,
    records_root: String,
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
    /// Start the step's Checks for the Drone, or say why not. **Nothing moves.**
    /// Job, step and Checks come from the caller's own slot and frozen workflow;
    /// `only_what_changed` narrows what each Check opens (`#504`). **It returns
    /// once the run has started**, and the report is a later turn.
    pub async fn run_checks(
        self: &Arc<Self>,
        caller: &JobId,
        only_what_changed: bool,
    ) -> Result<ChecksRunning, NotRun> {
        let plan = self.dry_run_looks(caller).await?;
        let read = self.dry_run_reads(&plan, only_what_changed).await?;
        self.dry_run_begins(caller, plan, read).await
    }

    /// The slot's refusals, asked when the call arrives and again at the mark.
    fn dry_run_refused(&self, caller: &JobId, at_work: &Working) -> Option<NotRun> {
        if at_work.session().unheard() {
            return Some(NotRun::Unheard);
        }
        if at_work.is_checking() {
            return Some(NotRun::AlreadyRunning);
        }
        if self.evidence_waiting_for(caller) > 0 {
            return Some(NotRun::AlreadySubmitted);
        }
        let allowed = self.dry_runs().allowed();
        (at_work.dry_runs() >= allowed).then_some(NotRun::Spent { allowed })
    }

    /// Whether there is a run to make, and what it is against.
    async fn dry_run_looks(&self, caller: &JobId) -> Result<Plan, NotRun> {
        let Some(slot) = self.slot_of(caller).await else {
            return Err(NotRun::NothingIsWorking);
        };
        let working = slot.lock().await;
        let Some(at_work) = working.as_ref() else {
            return Err(NotRun::NothingIsWorking);
        };
        if let Some(why) = self.dry_run_refused(caller, at_work) {
            return Err(why);
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
        Ok(Plan {
            record,
            step,
            worktree,
            entered_with,
        })
    }

    /// Every reading the run needs, with no lock held.
    async fn dry_run_reads(&self, plan: &Plan, narrow: bool) -> Result<Readings, NotRun> {
        let unread = |cause: String| NotRun::CouldNotRead { cause };
        let Some(declared) = plan.record.workflow().step(&plan.step) else {
            return Err(NotRun::NoSuchStep {
                step: plan.step.clone(),
            });
        };
        let checks = declared.checks();
        let moved = match checks
            .iter()
            .any(|check| matches!(check, ResolvedCheck::DiffNonempty))
        {
            false => false,
            true => {
                let now = self
                    .work()
                    .footprint(&plan.worktree)
                    .map_err(|cause| unread(cause.to_string()))?;
                plan.entered_with
                    .as_ref()
                    .is_some_and(|before| now.differs_from(before))
            }
        };
        // The gate's own skip, from the same reading; a narrowed run reads paths regardless.
        let touched: Vec<String> =
            match narrow || checks.iter().any(ResolvedCheck::needs_changed_paths) {
                false => Vec::new(),
                true => self
                    .work()
                    .changed_files(&plan.worktree)
                    .map_err(|cause| unread(cause.to_string()))?
                    .paths(),
            };
        if narrow && touched.is_empty() {
            return Err(NotRun::NothingChanged);
        }
        let tasks = self
            .store()
            .lock()
            .await
            .work_plan(plan.record.id())
            .map_err(|cause| unread(cause.to_string()))?
            .map(|recorded| recorded.counts());
        // The attempt keeps a reattempt's dry runs from overwriting earlier ones.
        let attempt = self
            .store()
            .lock()
            .await
            .step_attempt(plan.record.id(), &plan.step)
            .map_err(|cause| unread(cause.to_string()))?;
        let records_root = self
            .served_by(&plan.record)
            .map_err(|cause| unread(cause.to_string()))?
            .records_root()
            .to_string();
        Ok(Readings {
            narrow,
            moved,
            touched,
            ports: self.port_map(&plan.record).await,
            port_env: self.port_env(&plan.record).await,
            tasks,
            attempt,
            records_root,
        })
    }

    /// Put the mark on and start the run, inside the lock that read the count.
    async fn dry_run_begins(
        self: &Arc<Self>,
        caller: &JobId,
        plan: Plan,
        read: Readings,
    ) -> Result<ChecksRunning, NotRun> {
        let Some(slot) = self.slot_of(caller).await else {
            return Err(NotRun::NothingIsWorking);
        };
        let mut working = slot.lock().await;
        let Some(at_work) = working
            .as_mut()
            .filter(|at_work| at_work.is(plan.record.id()) && at_work.standing().1 == plan.step)
        else {
            return Err(NotRun::NothingIsWorking);
        };
        if let Some(why) = self.dry_run_refused(caller, at_work) {
            return Err(why);
        }
        let run = RUNS.fetch_add(1, Ordering::Relaxed);
        let (going, stop) = Stop::when_dropped();
        at_work.checking(self.now(), run, going);
        let fleet = Arc::clone(self);
        let caller = caller.clone();
        Ok(ChecksRunning(tokio::spawn(async move {
            let ran = fleet.dry_run(&plan, &read, &stop).await;
            fleet.dry_run_ends(&caller, &plan, run, ran).await
        })))
    }

    /// Take the mark off, give the clocks back and tell the Drone — **only where
    /// this is still the run in flight**, so a step that ended hears nothing.
    async fn dry_run_ends(
        &self,
        caller: &JobId,
        plan: &Plan,
        run: u64,
        ran: Result<CheckReport, String>,
    ) -> Option<Result<CheckReport, String>> {
        let now = self.now();
        let slot = self.slot_of(caller).await?;
        let mut working = slot.lock().await;
        let at_work = working
            .as_mut()
            .filter(|at_work| at_work.is(plan.record.id()))?;
        if !at_work.checked(now, run) {
            return None;
        }
        let told = ChecksReported::of(&ran);
        // Written down before the send, `Fleet::tell`'s order.
        at_work.instructed(Occasion::Checks, told.text());
        let _ = at_work.session().checks(&told).await;
        drop(working);
        if let Ok(report) = &ran {
            self.noted_dry_run(plan, report);
        }
        Some(ran)
    }

    /// The run itself, with no lock held.
    async fn dry_run(
        &self,
        plan: &Plan,
        read: &Readings,
        stop: &Stop,
    ) -> Result<CheckReport, String> {
        let Some(declared) = plan.record.workflow().step(&plan.step) else {
            return Err(format!(
                "step `{}` is not in the workflow",
                plan.step.as_str()
            ));
        };
        // The same batch the gate runs, so the rows keep the step's own order.
        let mut observed = Vec::with_capacity(declared.checks().len());
        let mut printed = Vec::new();
        let mut took = Vec::with_capacity(declared.checks().len());
        let mut narrowed_to = Vec::with_capacity(declared.checks().len());
        for done in crate::checking::ran(
            declared.checks(),
            &read.touched,
            read.moved,
            read.narrow,
            Path::new(plan.worktree.path()),
            self.budget().duration(),
            &self.room(),
            // Not a gate a person is watching, so no live log beside the gate's.
            &crate::underway::Announcing::nowhere(),
            &read.ports,
            &read.port_env,
            read.tasks,
            stop,
        )
        .await
        {
            observed.push(done.observed);
            took.push(done.took);
            narrowed_to.push(done.narrowed_to);
            if let Some(pair) = done.printed {
                printed.push(pair);
            }
        }
        // Unreachable while `ran` answers one per Check; carried, as a panic ends Fleet.
        let ran = Ran::of(declared, &observed).map_err(|cause| cause.to_string())?;
        let rows = check_output::kept_dry(
            &read.records_root,
            &plan.record.handle(),
            &plan.step,
            read.attempt,
            &ran.recorded(),
            &printed,
        );
        // A failed Check printing a test another Job is fixing points this Job
        // at that fix, as the gate does. #1001.
        let said: Vec<_> = printed
            .iter()
            .map(|(name, output)| (name.as_str(), output))
            .collect();
        let failed = crate::fixing::failures_said(
            rows.iter()
                .filter(|row| !row.outcome.advances())
                .map(|row| row.name.as_str()),
            &said,
        );
        self.pointed_at_fixes(plan.record.id(), &failed).await;
        Ok(CheckReport {
            ran: rows
                .into_iter()
                .enumerate()
                .map(|(at, row)| {
                    // `#737`: the tail rides on a row that did not advance.
                    let output = (!row.outcome.advances())
                        .then(|| printed.iter().find(|(name, _)| *name == row.name))
                        .flatten()
                        .map(|(_, printed)| check_output::excerpt(printed));
                    CheckRan {
                        name: row.name,
                        outcome: row.outcome.into(),
                        detail: row.produced,
                        took: took.get(at).copied().unwrap_or(Duration::ZERO),
                        log: row.output_path,
                        // The command that ran, where it was not the Check's own.
                        narrowed_to: narrowed_to.get(at).cloned().flatten(),
                        output,
                    }
                })
                .collect(),
            narrowed: read.narrow,
        })
    }

    /// Write the run into the Job's log, as fields a query can count.
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
        .with_field("failed", FieldValue::Int(report.failed() as i64))
        .with_field("narrowed", FieldValue::Bool(report.narrowed));
        // A line that will not write fails nothing: the Drone has its answer.
        self.noted_in_the_log(plan.record.id(), &envelope);
    }
}

/// The refusal a Drone reads, beside the refusal, as `crate::questioning` keeps its own.
impl From<NotRun> for ipc::mcp::NotRecorded {
    fn from(why: NotRun) -> ipc::mcp::NotRecorded {
        ipc::mcp::NotRecorded {
            because: why.to_string(),
        }
    }
}
