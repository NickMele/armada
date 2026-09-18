//! A Drone asking for the fix to a test it says is broken on main, and Fleet
//! running just that test against main before anything is drafted. #999.
//!
//! **The call answers at once, and what the run came to is a later turn**, for
//! `crate::dry_run`'s reason (#1020): the checkout of main may never have been
//! built, and a build outlasts what the agent CLI waits on one call.
//!
//! **Three bounds.** A Drone waits on one run at a time, a step asks for at most
//! [`Fixes`], and the checkout of main is shared by every Job on the repository,
//! so one test runs there at a time.
//!
//! **Nothing here passes a gate.** A test that fails on main drafts a Job that
//! waits for a person; the Drone's own step is still decided by its Checks.

mod claims;
mod refusal;
mod repeated;
mod running;
mod waiting;

pub use refusal::NotFixed;
pub(crate) use repeated::Repeat;
pub(crate) use waiting::{failures_said, FixStands};

use std::sync::atomic::Ordering;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Breakage, BreakageClaim, Job, JobId, ManifestId, ResolvedCheck, StepId, Ulid,
};
use ipc::mcp::DraftFix;
use tokio::task::JoinHandle;

use crate::checking::{Going, Stop};
use crate::daemon::Fleet;
use crate::drafting::StatedBy;
use crate::session::{LiveSession, Occasion};

/// How many fixes one step may ask for.
///
/// **A newtype with one constructor and no `Default`**, for [`crate::DryRuns`]'
/// reason: the composition root names it once and says there what it is worth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fixes(u32);

impl Fixes {
    pub const fn of(allowed: u32) -> Fixes {
        Fixes(allowed)
    }

    pub fn allowed(&self) -> u32 {
        self.0
    }
}

/// What the call answered: the fix already claiming the test, or a run started.
#[derive(Debug)]
pub enum FixAnswer {
    AlreadyClaimed(JobId),
    Started(FixRunning),
}

impl FixAnswer {
    /// The receipt's word.
    pub fn word(&self) -> String {
        match self {
            FixAnswer::AlreadyClaimed(job) => format!(
                "already being fixed by {}. Nothing new is drafted; carry on with your part",
                job.as_str()
            ),
            FixAnswer::Started(_) => String::from(
                "started. Fleet is running that test on main, and what it came to arrives as \
                 a later turn, however long it takes. Carry on with your part meanwhile",
            ),
        }
    }
}

/// A run against main Fleet has started and owns. **Dropping this leaves it
/// going.**
#[derive(Debug)]
pub struct FixRunning(JoinHandle<Option<Result<Drafted, NotFixed>>>);

impl FixRunning {
    /// What the Drone was told, or `None` where its step ended before the run.
    pub async fn finished(self) -> Option<Result<Drafted, NotFixed>> {
        self.0.await.ok().flatten()
    }
}

/// The fix Job a failure on main drafted.
#[derive(Debug, PartialEq, Eq)]
pub struct Drafted(pub JobId);

/// The later turn a run against main arrives as, built from the run alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixReported(String);

impl FixReported {
    fn of(test: &str, came_to: &Result<Drafted, NotFixed>) -> FixReported {
        const HEADING: &str = "THE TEST YOU SAID IS BROKEN ON MAIN";
        FixReported(match came_to {
            Ok(Drafted(fix)) => format!(
                "{HEADING}\n\n`{test}` fails on main too, so the failure is not your change. \
                 A fix is drafted as Job {} and waits for a person's approval. Carry on with \
                 your part: your own checks still fail on that test until the fix lands, so \
                 say so in your evidence.",
                fix.as_str()
            ),
            Err(why) => format!("{HEADING}\n\n{why}."),
        })
    }

    /// The turn, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// What a Drone asked about, resolved against its own part.
struct Request {
    record: Job,
    step: StepId,
    repository: ManifestId,
    root: String,
    run: ResolvedCheck,
    expect_exit_code: i64,
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
    /// Start the named test against main or say why not; where it fails there
    /// too, the task drafts the fix and claims the test for it.
    ///
    /// **An already-claimed test answers before anything is spent.**
    pub async fn draft_fix(
        self: &Arc<Self>,
        caller: &JobId,
        fix: DraftFix,
    ) -> Result<FixAnswer, NotFixed> {
        let test = fix.test.trim().to_string();
        let request = self.what_fix_is_asked(caller, &fix, &test).await?;
        let claimed = self
            .store()
            .lock()
            .await
            .breakage_claimed(&request.repository, &fix.check, &test)
            .ok()
            .flatten();
        if let Some(claim) = claimed {
            // Pointed at the fix, and hears when it lands. #1001. The receipt is
            // what tells it now, so nothing is queued.
            if claim.fix != *caller && claim.reported_by != *caller {
                self.point_at(caller, &claim, false).await;
            }
            return Ok(FixAnswer::AlreadyClaimed(claim.fix));
        }
        // One counter with the dry runs, so a run one ended cannot end the other.
        let run = crate::dry_run::RUNS.fetch_add(1, Ordering::Relaxed);
        let (going, stop) = Stop::when_dropped();
        self.fix_begins(caller, &request, run, going).await?;
        let fleet = Arc::clone(self);
        let caller = caller.clone();
        Ok(FixAnswer::Started(FixRunning(tokio::spawn(async move {
            let ran = fleet.run_on_main(&request, &stop).await;
            fleet
                .fix_ends(&caller, &request, run, ran, &fix, &test)
                .await
        }))))
    }

    /// The Check the Drone named on its own part, and the command that runs
    /// just the one test.
    async fn what_fix_is_asked(
        &self,
        caller: &JobId,
        fix: &DraftFix,
        test: &str,
    ) -> Result<Request, NotFixed> {
        let Some(slot) = self.slot_of(caller).await else {
            return Err(NotFixed::NothingIsWorking);
        };
        let (job, step) = {
            let working = slot.lock().await;
            let Some(at_work) = working.as_ref() else {
                return Err(NotFixed::NothingIsWorking);
            };
            let (job, step, _) = at_work.standing();
            (job, step)
        };
        self.requested(job, step, &fix.check, test).await
    }

    /// The same resolution `what_fix_is_asked` does for a Drone's own call,
    /// taken by a Job and a step directly — `fixing::repeated`'s reason: a
    /// repeat Fleet spots itself has both in hand already and no slot to read
    /// them off.
    async fn requested(
        &self,
        job: JobId,
        step: StepId,
        check: &str,
        test: &str,
    ) -> Result<Request, NotFixed> {
        let record = self
            .load(&job)
            .await
            .map_err(|_| NotFixed::NothingIsWorking)?;
        let declared = record
            .workflow()
            .step(&step)
            .ok_or(NotFixed::NothingIsWorking)?;
        let (expect_exit_code, requires, template, places, width) = declared
            .checks()
            .iter()
            .find_map(|c| match c {
                ResolvedCheck::ManifestCheck {
                    name,
                    expect_exit_code,
                    requires,
                    one_test,
                    places,
                    width,
                    ..
                } if name == check => Some((
                    *expect_exit_code,
                    requires.clone(),
                    one_test.clone(),
                    *places,
                    *width,
                )),
                _ => None,
            })
            .ok_or_else(|| NotFixed::NoSuchCheck {
                check: check.to_string(),
            })?;
        let template = template.ok_or_else(|| NotFixed::NoWayToRunOneTest {
            check: check.to_string(),
        })?;
        let command =
            checks_runner::one_test(&template, test).ok_or_else(|| NotFixed::NotOneArgument {
                test: test.to_string(),
            })?;
        let owner = self
            .names()
            .owner_of(&job)
            .ok_or(NotFixed::NothingIsWorking)?;
        let served = self.served_by(&record).map_err(|why| NotFixed::NoMain {
            why: why.to_string(),
        })?;
        Ok(Request {
            repository: ManifestId::carried(Ulid::carried(owner)),
            root: served.root().to_string(),
            run: ResolvedCheck::ManifestCheck {
                name: check.to_string(),
                run: command,
                expect_exit_code,
                when: None,
                requires,
                narrow: None,
                one_test: None,
                runs_at: core_model::RunsAt::Everywhere,
                places,
                // Kept: one test out of a suite still runs under the suite's
                // own runner, which reads the same flag. #1444.
                width,
            },
            expect_exit_code,
            record,
            step,
        })
    }

    /// The marks, taken under the slot lock that reads the count: the step's
    /// allowance, the Drone's one run at a time, and the repository's checkout.
    async fn fix_begins(
        &self,
        caller: &JobId,
        request: &Request,
        run: u64,
        going: Going,
    ) -> Result<(), NotFixed> {
        let Some(slot) = self.slot_of(caller).await else {
            return Err(NotFixed::NothingIsWorking);
        };
        let mut working = slot.lock().await;
        let Some(at_work) = working.as_mut().filter(|at_work| {
            at_work.is(request.record.id()) && at_work.standing().1 == request.step
        }) else {
            return Err(NotFixed::NothingIsWorking);
        };
        if at_work.session().unheard() {
            return Err(NotFixed::Unheard);
        }
        if at_work.is_checking() {
            return Err(NotFixed::AlreadyRunning);
        }
        if self.evidence_waiting_for(caller) > 0 {
            return Err(NotFixed::AlreadySubmitted);
        }
        let allowed = self.fixes().allowed();
        if at_work.fixes() >= allowed {
            return Err(NotFixed::Spent { allowed });
        }
        if !self
            .fixing_on_main()
            .lock()
            .await
            .insert(request.root.clone())
        {
            return Err(NotFixed::MainIsBusy);
        }
        at_work.fixing(self.now(), run, going);
        Ok(())
    }

    /// Take the mark off, draft where main failed too, give the checkout back
    /// and tell the Drone — **only where this is still the run in flight**, so a
    /// step that ended or a submission that stopped the run drafts nothing.
    async fn fix_ends(
        &self,
        caller: &JobId,
        request: &Request,
        run: u64,
        ran: Result<checks_runner::OneTestRan, NotFixed>,
        fix: &DraftFix,
        test: &str,
    ) -> Option<Result<Drafted, NotFixed>> {
        use checks_runner::OneTestRan;
        let came_to = match (self.fix_still_asked(caller, request, run).await, ran) {
            (false, _) => None,
            (true, Err(why)) => Some(Err(why)),
            (true, Ok(OneTestRan::Passed)) => Some(Err(NotFixed::PassesOnMain {
                test: test.to_string(),
            })),
            // Neither a pass nor a failure, so neither answer is honest —
            // #1204. Reported the same way a refusal is, not drafted.
            (true, Ok(OneTestRan::NoMatch)) => Some(Err(NotFixed::NoMatch {
                test: test.to_string(),
            })),
            // Still holding the checkout, so no second fix for this repository
            // drafts meanwhile.
            (true, Ok(OneTestRan::Failed)) => {
                Some(self.drafted_fix(caller, fix, request, test).await)
            }
        };
        self.fixing_on_main().lock().await.remove(&request.root);
        let came_to = came_to?;
        self.told_fix(caller, request, &FixReported::of(test, &came_to))
            .await;
        Some(came_to)
    }

    /// Whether run `run` was still this Job's run in flight, taking it off if so.
    async fn fix_still_asked(&self, caller: &JobId, request: &Request, run: u64) -> bool {
        let now = self.now();
        let Some(slot) = self.slot_of(caller).await else {
            return false;
        };
        let mut working = slot.lock().await;
        working
            .as_mut()
            .filter(|at_work| at_work.is(request.record.id()))
            .is_some_and(|at_work| at_work.checked(now, run))
    }

    /// The later turn, written down before the send, `Fleet::tell`'s order.
    async fn told_fix(&self, caller: &JobId, request: &Request, told: &FixReported) {
        let Some(slot) = self.slot_of(caller).await else {
            return;
        };
        let working = slot.lock().await;
        let Some(at_work) = working
            .as_ref()
            .filter(|at_work| at_work.is(request.record.id()))
        else {
            return;
        };
        at_work.instructed(Occasion::Fix, told.text());
        let _ = at_work.session().fix(told).await;
    }

    /// Draft the fix at the approval gate, then claim the test for it.
    async fn drafted_fix(
        &self,
        caller: &JobId,
        fix: &DraftFix,
        request: &Request,
        test: &str,
    ) -> Result<Drafted, NotFixed> {
        let drafted = self
            .proposed_job(
                proposal(fix, &request.repository),
                StatedBy::TheFix {
                    reported_by: caller.clone(),
                },
                None,
                Actor::Fleet,
            )
            .await
            .map_err(|why| NotFixed::NotDrafted {
                why: why.to_string(),
            })?;
        let claim = BreakageClaim {
            fix: drafted.id().clone(),
            repository: request.repository.clone(),
            breakage: Breakage {
                check: fix.check.clone(),
                test: test.to_string(),
                failure: fix.failure.clone(),
            },
            reported_by: caller.clone(),
        };
        let _ = self
            .store()
            .lock()
            .await
            .claim_breakage(&claim, &self.now());
        Ok(Drafted(drafted.id().clone()))
    }
}

/// The fix, as a person would have proposed it. **`dispatch_job`'s shape**, with
/// the origin that says a Drone drafted it and no edge: a fix waits on nothing.
fn proposal(fix: &DraftFix, repository: &ManifestId) -> ipc::ProposeJob {
    ipc::ProposeJob {
        title: fix.title.clone(),
        workflow_id: ipc::WorkflowId::carried(fix.workflow.clone()),
        owner_manifest_id: ipc::ManifestId::from(repository),
        origin: ipc::TopLevelOrigin::from(core_model::TopLevelOrigin::DroneDrafted),
        urgency: ipc::Urgency::from(core_model::Urgency::Normal),
        atomic: false,
        write_targets: None,
        dependencies: Vec::new(),
        model: None,
        acceptance_criteria: fix
            .acceptance_criteria
            .iter()
            .map(|text| ipc::ProposedCriterion {
                text: text.clone(),
                source: ipc::CriterionSource::from(core_model::CriterionSource::Judge),
            })
            .collect(),
        subject: None,
        facts: fix.brief.clone(),
        attachments: Vec::new(),
    }
}
