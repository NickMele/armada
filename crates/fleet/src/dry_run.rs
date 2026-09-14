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
//! | A Check that fails | The rest of the run; the Drone hears the failure at once |
//!
//! **A turn per result, and the report is the last** (#1062). [`landed`] tells
//! each one that lands while others still run.

mod landed;

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
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use verification::Ran;

use crate::check_output;
use crate::checking::Stop;
use crate::underway::{stopped_by, Announcing, Landed};
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

/// What every turn about the Checks a Drone asked for opens with.
const HEADING: &str = "THE CHECKS YOU ASKED FOR";

/// A later turn about a run, built from the run and nothing else: a result
/// that landed, or the report that ends it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChecksReported(String);

impl ChecksReported {
    fn of(ran: &Result<CheckReport, String>) -> ChecksReported {
        ChecksReported(match ran {
            Ok(report) => match report.ran.iter().find_map(|row| row.stopped.as_deref()) {
                None => {
                    format!("{HEADING}\n\nThe run is over. This is what each one did:\n\n{report}")
                }
                Some(failed) => format!(
                    "{HEADING}\n\nThe run is over: `{failed}` did not pass, so the checks still \
                     going were stopped rather than finished. Fix it and ask again. This is \
                     what each one did:\n\n{report}"
                ),
            },
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
    /// What the worktree held before any Check started. **The reading
    /// `crate::reuse::KeptDryRun` is built against**, never one taken once
    /// the run has finished — a Drone may keep editing while its Checks run
    /// (#1020), and a footprint taken then would vouch for content the
    /// Checks never saw.
    footprint: Footprint,
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
        // **Read unconditionally, not only where the step declares
        // `diff_nonempty`, and kept whole rather than folded to a bool.**
        // `crate::reuse` needs this same reading beside whatever the run
        // finds, so the gate can tell later whether the worktree it is
        // looking at is the one this run measured — and it is taken here,
        // before any Check starts, because a Drone may keep editing while
        // they run (#1020). A reading taken at the end would vouch for
        // content the Checks never saw, and the gate would reuse a pass
        // against work that was never tested.
        let footprint = self
            .work()
            .footprint(&plan.worktree)
            .map_err(|cause| unread(cause.to_string()))?;
        let moved = checks
            .iter()
            .any(|check| matches!(check, ResolvedCheck::DiffNonempty))
            && plan
                .entered_with
                .as_ref()
                .is_some_and(|before| footprint.differs_from(before));
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
            footprint,
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
        let (going, stop) = Stop::when_dropped_or_one_fails();
        at_work.checking(self.now(), run, going);
        let fleet = Arc::clone(self);
        let caller = caller.clone();
        Ok(ChecksRunning(tokio::spawn(async move {
            let (heard, hearing) = tokio::sync::mpsc::unbounded_channel();
            // A narrowed run's durations are its narrower commands', not the Checks'.
            let showing = fleet.announcing_dry_run(
                &plan.record,
                &plan.step,
                read.attempt,
                heard,
                !read.narrow,
            );
            let ran = fleet
                .dry_run(&caller, run, &plan, &read, &stop, &showing, hearing)
                .await;
            fleet.dry_run_ends(&caller, &plan, run, ran, showing).await
        })))
    }

    /// Take the mark off, give the clocks back and tell the Drone — **only where
    /// this is still the run in flight**, so a step that ended hears nothing.
    ///
    /// **What the run found is kept here too, on the same guard, and nowhere
    /// else.** `at_work.checked(now, run)` is the one place this run is known
    /// to be the one the slot is still waiting on: a submission, a kill or a
    /// step boundary already cleared it and answers `false`, and keeps nothing.
    /// A run its own first failure stopped does finish, and `KeptDryRun::of`
    /// keeps only what passed, so a Check it stopped is never reused (#1014).
    async fn dry_run_ends(
        &self,
        caller: &JobId,
        plan: &Plan,
        run: u64,
        ran: Result<(CheckReport, crate::reuse::KeptDryRun), String>,
        showing: Announcing,
    ) -> Option<Result<CheckReport, String>> {
        // Whether or not the step still waits: what ran to a code was measured.
        self.kept_timings(&plan.record, showing.timings()).await;
        let now = self.now();
        let slot = self.slot_of(caller).await?;
        let mut working = slot.lock().await;
        let at_work = working
            .as_mut()
            .filter(|at_work| at_work.is(plan.record.id()))?;
        if !at_work.checked(now, run) {
            return None;
        }
        let (ran, kept) = match ran {
            Ok((report, kept)) => (Ok(report), Some(kept)),
            Err(cause) => (Err(cause), None),
        };
        if let Some(kept) = kept {
            at_work.kept_dry_run(kept);
        }
        at_work.show_dry_run(showing);
        let told = ChecksReported::of(&ran);
        // Written down before the send, `Fleet::tell`'s order.
        at_work.instructed(Occasion::Checks, told.text());
        let _ = at_work.session().checks(&told).await;
        drop(working);
        if let Ok(report) = &ran {
            self.noted_dry_run(plan, report);
            self.pointed_at_fixes_in(plan.record.id(), report).await;
        }
        Some(ran)
    }

    /// The run itself, with no lock held.
    ///
    /// **Returns what is kept beside what is told.** The report is the
    /// Drone's; [`crate::reuse::KeptDryRun`] is Fleet's own, and
    /// [`dry_run_ends`](Fleet::dry_run_ends) is what puts it where the gate
    /// can find it — never here, which has no slot to write into.
    #[allow(clippy::too_many_arguments)]
    async fn dry_run(
        &self,
        caller: &JobId,
        run: u64,
        plan: &Plan,
        read: &Readings,
        stop: &Stop,
        showing: &Announcing,
        hearing: UnboundedReceiver<Landed>,
    ) -> Result<(CheckReport, crate::reuse::KeptDryRun), String> {
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
        let mut stopped = Vec::with_capacity(declared.checks().len());
        // Fastest first, by this repository's past runs. #1062.
        let room = self.checks_room_for(&plan.record).await;
        let running = crate::checking::ran(
            declared.checks(),
            &read.touched,
            read.moved,
            read.narrow,
            Path::new(plan.worktree.path()),
            self.budget().duration(),
            &room,
            // Shown as the Drone's own run, with no live log beside the gate's.
            showing,
            &read.ports,
            &read.port_env,
            read.tasks,
            stop,
            // **A dry run never reuses.** It is the reading `crate::reuse`
            // keeps for the gate to trust later; trusting an earlier one of
            // its own here would be a dry run measuring nothing and calling
            // it a measurement.
            None,
            read.attempt,
            None,
        );
        for done in self.heard_while(caller, run, running, hearing).await {
            observed.push(done.observed);
            took.push(done.took);
            narrowed_to.push(done.narrowed_to);
            stopped.push(done.stopped);
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
        // **Kept before the rows are consumed below**, and from the same
        // `rows` and `narrowed_to` the report is about to render — a second
        // reading of either here is a second place they could disagree.
        // `read.footprint` is the reading `dry_run_reads` took before any
        // Check started, never one taken here: the Drone may have kept
        // editing while they ran (#1020), and a footprint taken now would
        // vouch for content the Checks never saw. `KeptDryRun::of` is what
        // decides which of these rows the gate may ever trust; everything
        // not eligible is dropped there, not here.
        let kept = crate::reuse::KeptDryRun::of(
            read.attempt,
            self.now(),
            read.footprint.clone(),
            rows.clone(),
            &narrowed_to,
        );
        let report = CheckReport {
            ran: rows
                .into_iter()
                .enumerate()
                .map(|(at, row)| {
                    let stopped = stopped.get(at).cloned().flatten();
                    // `#737`: the tail rides on a row that did not advance, and
                    // not on one a failure stopped, which has nothing to explain.
                    let output = (stopped.is_none() && !row.outcome.advances())
                        .then(|| printed.iter().find(|(name, _)| *name == row.name))
                        .flatten()
                        .map(|(_, printed)| check_output::excerpt(printed));
                    CheckRan {
                        name: row.name,
                        outcome: row.outcome.into(),
                        detail: stopped.as_deref().map(stopped_by).or(row.produced),
                        took: took.get(at).copied().unwrap_or(Duration::ZERO),
                        log: row.output_path,
                        // The command that ran, where it was not the Check's own.
                        narrowed_to: narrowed_to.get(at).cloned().flatten(),
                        output,
                        stopped,
                    }
                })
                .collect(),
            narrowed: read.narrow,
        };
        Ok((report, kept))
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
