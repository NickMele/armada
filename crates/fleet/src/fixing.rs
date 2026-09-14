//! A Drone asking for the fix to a test it says is broken on main, and Fleet
//! running just that test against main before anything is drafted. #999.
//!
//! **Two bounds, for `crate::dry_run`'s reasons one directory over.** A Drone
//! waits on one Check run at a time and a step asks for at most [`Fixes`]; and
//! the checkout of main is shared by every Job on the repository, so one run is
//! out there at a time.
//!
//! **Nothing here passes a gate.** A test that fails on main drafts a Job that
//! waits for a person; the Drone's own step is still decided by its Checks.

use std::fmt;
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Breakage, BreakageClaim, Job, JobId, ManifestId, ResolvedCheck, StepId, Ulid,
};
use ipc::mcp::{DraftFix, NotRecorded};
use verification::{Exit, Observed};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::drafting::StatedBy;

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

/// What asking came to: a fix drafted now, or the one already claiming the test.
#[derive(Debug, PartialEq, Eq)]
pub enum Drafted {
    New(JobId),
    AlreadyClaimed(JobId),
}

impl Drafted {
    /// The receipt's word, which names the fix Job either way.
    pub fn word(&self) -> String {
        match self {
            Drafted::New(job) => format!("drafted {}", job.as_str()),
            Drafted::AlreadyClaimed(job) => format!("already being fixed by {}", job.as_str()),
        }
    }
}

/// Why no fix was drafted. **Every one is a tool error the Drone reads**, and
/// none of them stops or advances its step.
#[derive(Debug)]
pub enum NotFixed {
    NothingIsWorking,
    NoSuchCheck { check: String },
    NoWayToRunOneTest { check: String },
    NotOneArgument { test: String },
    AlreadyRunning,
    MainIsBusy,
    Spent { allowed: u32 },
    NoMain { why: String },
    NeverRan,
    PassesOnMain { test: String },
    NotDrafted { why: String },
}

impl fmt::Display for NotFixed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotFixed::NothingIsWorking => out.write_str(
                "no Job is being worked, so there is no Check to say a test is broken under. \
                 Stop — the Job this Drone was started for has already ended",
            ),
            NotFixed::NoSuchCheck { check } => write!(
                out,
                "`{check}` is not one of the Checks on the part you are on. Name the Check \
                 the test failed under, as your part's Checks name it"
            ),
            NotFixed::NoWayToRunOneTest { check } => write!(
                out,
                "`{check}` does not say how to run one test by name, so Fleet cannot run \
                 just that test on main and nothing is drafted. Say in your evidence what \
                 you found"
            ),
            NotFixed::NotOneArgument { test } => write!(
                out,
                "`{test}` cannot be passed as one argument, so it cannot be run by name. \
                 Copy the test's name without quotes"
            ),
            NotFixed::AlreadyRunning => out.write_str(
                "Fleet is already running a Check for you. Wait for that answer, then ask again",
            ),
            NotFixed::MainIsBusy => out.write_str(
                "Fleet is already running a test against main for this repository. Carry on \
                 with your part and ask again in a few minutes",
            ),
            NotFixed::Spent { allowed } => write!(
                out,
                "this part has already asked for {}. No more are drafted from this part — \
                 say in your evidence what you found",
                match allowed {
                    1 => String::from("a fix"),
                    n => format!("{n} fixes"),
                }
            ),
            NotFixed::NoMain { why } => {
                write!(
                    out,
                    "the test could not run on main: {why}. Nothing is drafted"
                )
            }
            NotFixed::NeverRan => out.write_str(
                "the test did not run on main, so nothing about main is known and nothing is \
                 drafted",
            ),
            NotFixed::PassesOnMain { test } => write!(
                out,
                "`{test}` passes on main, so the failure is in your change. Nothing is \
                 drafted; fix it on your branch"
            ),
            NotFixed::NotDrafted { why } => write!(out, "the fix Job could not be drafted: {why}"),
        }
    }
}

impl From<NotFixed> for NotRecorded {
    fn from(why: NotFixed) -> NotRecorded {
        NotRecorded {
            because: why.to_string(),
        }
    }
}

/// What a Drone asked about, resolved against its own part.
struct Asked {
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
    /// Run the named test against main and, where it fails there too, draft the
    /// fix and claim the test for it.
    ///
    /// **An already-claimed test answers before anything is spent.** Every other
    /// road that reaches the run gives the clocks and the checkout back on the
    /// way out, whatever the run came to.
    pub async fn draft_fix(&self, caller: &JobId, fix: &DraftFix) -> Result<Drafted, NotFixed> {
        let test = fix.test.trim();
        let asked = self.what_fix_is_asked(caller, fix, test).await?;
        let claimed = self
            .store()
            .lock()
            .await
            .breakage_claimed(&asked.repository, &fix.check, test)
            .ok()
            .flatten();
        if let Some(claim) = claimed {
            return Ok(Drafted::AlreadyClaimed(claim.fix));
        }
        self.fix_begins(caller, &asked).await?;
        let ran = self.run_on_main(&asked).await;
        self.fix_ends(caller, &asked).await;
        if !ran? {
            return Err(NotFixed::PassesOnMain {
                test: test.to_string(),
            });
        }
        self.drafted_fix(caller, fix, &asked, test).await
    }

    /// Every claimed fix a Job is part of, from either side, for its detail.
    /// A reporter that has been forgotten keeps its id and loses its title.
    pub(crate) async fn breakages_of(
        &self,
        job: &Job,
    ) -> Result<Vec<ipc::ClaimedBreakage>, Adrift> {
        let store = self.store().lock().await;
        let mut claims = store
            .breakages_claimed_by(job.id())
            .map_err(Adrift::Reading)?;
        claims.extend(
            store
                .breakages_reported_by(job.id())
                .map_err(Adrift::Reading)?,
        );
        let title = |id: &JobId| {
            store
                .load_job(id)
                .ok()
                .map(|found| found.title().as_str().to_string())
        };
        Ok(claims
            .into_iter()
            .map(|claim| ipc::ClaimedBreakage {
                fix_title: title(&claim.fix).unwrap_or_default(),
                reported_by_title: title(&claim.reported_by),
                fix: ipc::JobId::from(&claim.fix),
                reported_by: ipc::JobId::from(&claim.reported_by),
                check: claim.breakage.check,
                test: claim.breakage.test,
                failure: claim.breakage.failure,
            })
            .collect())
    }

    /// Give back every breakage a Job that just ended was claiming.
    pub(crate) async fn released_breakages(&self, job: &Job) {
        let _ = self.store().lock().await.release_breakages(job.id());
    }

    /// The Check the Drone named on its own part, and the command that runs
    /// just the one test.
    async fn what_fix_is_asked(
        &self,
        caller: &JobId,
        fix: &DraftFix,
        test: &str,
    ) -> Result<Asked, NotFixed> {
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
        let record = self
            .load(&job)
            .await
            .map_err(|_| NotFixed::NothingIsWorking)?;
        let declared = record
            .workflow()
            .step(&step)
            .ok_or(NotFixed::NothingIsWorking)?;
        let (expect_exit_code, requires, template) = declared
            .checks()
            .iter()
            .find_map(|check| match check {
                ResolvedCheck::ManifestCheck {
                    name,
                    expect_exit_code,
                    requires,
                    one_test,
                    ..
                } if *name == fix.check => {
                    Some((*expect_exit_code, requires.clone(), one_test.clone()))
                }
                _ => None,
            })
            .ok_or_else(|| NotFixed::NoSuchCheck {
                check: fix.check.clone(),
            })?;
        let template = template.ok_or_else(|| NotFixed::NoWayToRunOneTest {
            check: fix.check.clone(),
        })?;
        let command =
            checks_runner::one_test(&template, test).ok_or_else(|| NotFixed::NotOneArgument {
                test: test.to_string(),
            })?;
        let owner = self
            .names()
            .owner_of(caller)
            .ok_or(NotFixed::NothingIsWorking)?;
        let served = self.served_by(&record).map_err(|why| NotFixed::NoMain {
            why: why.to_string(),
        })?;
        Ok(Asked {
            repository: ManifestId::carried(Ulid::carried(owner)),
            root: served.root().to_string(),
            run: ResolvedCheck::ManifestCheck {
                name: fix.check.clone(),
                run: command,
                expect_exit_code,
                when: None,
                requires,
                narrow: None,
                one_test: None,
            },
            expect_exit_code,
            record,
            step,
        })
    }

    /// The marks, taken under the slot lock that reads the count: the step's
    /// allowance, the Drone's one run at a time, and the repository's checkout.
    async fn fix_begins(&self, caller: &JobId, asked: &Asked) -> Result<(), NotFixed> {
        let Some(slot) = self.slot_of(caller).await else {
            return Err(NotFixed::NothingIsWorking);
        };
        let mut working = slot.lock().await;
        let Some(at_work) = working.as_mut() else {
            return Err(NotFixed::NothingIsWorking);
        };
        if at_work.is_checking() {
            return Err(NotFixed::AlreadyRunning);
        }
        let allowed = self.fixes().allowed();
        if at_work.fixes() >= allowed {
            return Err(NotFixed::Spent { allowed });
        }
        if !self
            .fixing_on_main()
            .lock()
            .await
            .insert(asked.root.clone())
        {
            return Err(NotFixed::MainIsBusy);
        }
        at_work.fixing(self.now());
        Ok(())
    }

    /// Whether the test fails on main. `false` is a pass there.
    async fn run_on_main(&self, asked: &Asked) -> Result<bool, NotFixed> {
        // Matched here rather than read off `NoBase::said`, whose sentences are
        // about photographing a before; these say what running a test there met.
        let (served, checkout) = self
            .base_to_show_from(&asked.record)
            .await
            .map_err(|why| NotFixed::NoMain {
                why: match why {
                    crate::basing::NoBase::Unnamed => String::from(
                        "this repository names no base branch, so there is no main to run it against",
                    ),
                    crate::basing::NoBase::NotCheckedOut { why } => {
                        format!("the checkout of main could not be made — {why}")
                    }
                    crate::basing::NoBase::NotPrepared { command, why } => format!(
                        "`setup.requires` did not finish in the checkout of main: `{command}` — {why}"
                    ),
                },
            })?;
        let ports = self.main_checkout_ports(&served).await;
        let env = self.main_checkout_port_env(&served).await;
        let completed = crate::checking::ran(
            std::slice::from_ref(&asked.run),
            &[],
            false,
            false,
            Path::new(checkout.path()),
            self.budget().duration(),
            &self.room(),
            &crate::underway::Announcing::nowhere(),
            &ports,
            &env,
            None,
        )
        .await;
        let exit = completed
            .iter()
            .rev()
            .find_map(|done| match &done.observed {
                Observed::Command(exit) => Some(exit),
                _ => None,
            });
        match exit {
            Some(Exit::Code(code)) if i64::from(*code) == asked.expect_exit_code => Ok(false),
            Some(Exit::NeverRan(_)) | None => Err(NotFixed::NeverRan),
            Some(_) => Ok(true),
        }
    }

    /// Give the checkout and the clocks back. **Guarded on the step**, for
    /// `dry_run_ends`' reason: the gate can advance one while the run is out.
    async fn fix_ends(&self, caller: &JobId, asked: &Asked) {
        self.fixing_on_main().lock().await.remove(&asked.root);
        let now = self.now();
        let Some(slot) = self.slot_of(caller).await else {
            return;
        };
        let mut working = slot.lock().await;
        if let Some(at_work) = working.as_mut() {
            let (job, step, _) = at_work.standing();
            if job == *asked.record.id() && step == asked.step {
                at_work.checked(now);
            }
        }
    }

    /// Draft the fix at the approval gate, then claim the test for it.
    ///
    /// **No race between the two**: the repository's checkout is held for the
    /// whole run, so no second fix for this repository reaches here meanwhile.
    async fn drafted_fix(
        &self,
        caller: &JobId,
        fix: &DraftFix,
        asked: &Asked,
        test: &str,
    ) -> Result<Drafted, NotFixed> {
        let drafted = self
            .proposed_job(
                proposal(fix, &asked.repository),
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
            repository: asked.repository.clone(),
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
        Ok(Drafted::New(drafted.id().clone()))
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
