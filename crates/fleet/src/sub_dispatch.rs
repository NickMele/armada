//! One Job asking for another to exist, and everything that has to be true
//! first.
//!
//! **The one place `Job::create_sub_dispatched` is reached.** It enters a Job
//! at `queued` — *already approved as part of its parent* — the one exemption
//! from `docs/concepts/fleet.md`'s rule that every Job-level dispatch is
//! approved explicitly. Below keeps it exactly as wide as it was.
//!
//! # Three things authorise the call, and none of them is a flag
//!
//! | | What makes it unspeakable rather than checked |
//! |---|---|
//! | The caller is a top-level Job | [`Origin::top_level`] answers `None` for a sub-dispatched one, and [`Dispatching`] cannot be built without the `Some` |
//! | The caller is on a step that dispatches | The step's frozen workflow says so, and the same field put the tool on the Drone's allowlist |
//! | The child waits only on its own siblings | An `after` id is looked up in this parent's own children and nowhere else |
//!
//! **Recursion is refused by the first of those and not bounded by a number.**
//! There is no value of [`Dispatching::at`] that produces a grandchild, so
//! depth two is a `None` rather than a limit that could drift out of step with
//! a configuration key. A person who wants a child epic dispatches it.
//!
//! **Nothing here admits anything.** `crate::turning` calls `admit_next` every
//! turn, and starting a child from inside a tool call would take the roster
//! lock on a path already inside a Drone's request — `crate::slots`' order,
//! backwards.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, DispatchOrigin, Job, JobId, JobStatus, Origin, ResolvedStep, StepId, StepState,
    TopLevelOrigin,
};
use ipc::mcp::DispatchJob;

use crate::adrift::Adrift;
use crate::crossing::Dispatched;
use crate::daemon::Fleet;
use crate::drafting::StatedBy;

/// Proof that this Job, on this step, may create Jobs.
///
/// **There is no constructor but [`Dispatching::at`]** and its fields are
/// private, so holding one is the whole of the authorisation — a caller cannot
/// assemble the permission out of a Job id and a step id it happens to have.
///
/// It carries the parent's [`TopLevelOrigin`] rather than checking it and
/// throwing it away, which is what makes "a sub-dispatched Job dispatches"
/// unrepresentable rather than refused: `Origin::top_level` returns `None` for
/// exactly that Job, and there is nothing else to build this from.
pub(crate) struct Dispatching<'a> {
    parent: &'a Job,
    step: StepId,
    /// Read at construction and kept. Nothing reads it back — its job is to be
    /// unobtainable for a child, which is done by the time it is stored.
    #[allow(dead_code)]
    top_level: TopLevelOrigin,
}

impl<'a> Dispatching<'a> {
    /// Whether this Job, where it currently stands, may create Jobs.
    ///
    /// `None` on three counts, and the caller says which: a Job that is itself
    /// sub-dispatched, a Job standing on no step, and a Job whose step's frozen
    /// workflow gave it no dispatching role.
    pub(crate) fn at(parent: &'a Job) -> Option<Dispatching<'a>> {
        let top_level = parent.origin().top_level()?;
        let step = parent.current_step_id()?.clone();
        parent
            .workflow()
            .step(&step)?
            .may_dispatch_jobs()
            .then_some(Dispatching {
                parent,
                step,
                top_level,
            })
    }

    /// Where a child records that it came from here: the Job **and the step**,
    /// which is what `DispatchOrigin` carries and what a later step reads back.
    fn origin(&self) -> DispatchOrigin {
        DispatchOrigin {
            job_id: self.parent.id().clone(),
            step_id: self.step.clone(),
        }
    }
}

/// Why a Job a Drone asked for was not created.
///
/// **None of these is a gate failure and none of them moves the parent.** The
/// call was refused, the Drone is told why in words it can act on, and it may
/// call again — which is the shape every Drone-facing refusal has.
#[derive(Debug)]
pub enum NotDispatched {
    /// The caller is not on a step that creates Jobs — because it is a
    /// sub-dispatched Job, because it stands on no step, or because its step's
    /// frozen workflow gave it no role.
    NotItsToDispatch { job: JobId },
    /// An `after` names a Job this parent did not create.
    NotASibling { named: String },
    /// Anything underneath: the title, the workflow name, the write.
    Adrift(Adrift),
}

impl std::fmt::Display for NotDispatched {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // It says what is true rather than what to try instead, because
            // there is nothing to try: a Drone here cannot reach the tool by
            // any argument, and telling it to "call again later" would make it
            // spend turns finding that out.
            NotDispatched::NotItsToDispatch { .. } => out.write_str(
                "creating Jobs is not part of the task you are on. Only the dispatching \
                 part of a task that decomposes work may do it, and a task that was \
                 itself created that way may not do it at all — report what you have \
                 instead",
            ),
            NotDispatched::NotASibling { named } => write!(
                out,
                "`{named}` is not one of the Jobs you created for this task, so nothing \
                 can be made to wait for it. Name only the ids this tool gave you back",
            ),
            NotDispatched::Adrift(cause) => write!(out, "{cause}"),
        }
    }
}

impl std::error::Error for NotDispatched {}

impl From<Adrift> for NotDispatched {
    fn from(cause: Adrift) -> NotDispatched {
        NotDispatched::Adrift(cause)
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
    /// Create one child of the Job that asked, and answer with its id.
    ///
    /// **The parent is the Job whose Drone holds the connection**, resolved by
    /// `crate::peer` before this is reached. There is no parameter here through
    /// which one could be named.
    pub(crate) async fn sub_dispatch(
        &self,
        caller: &JobId,
        asked: &DispatchJob,
    ) -> Result<JobId, NotDispatched> {
        let parent = self.load(caller).await?;
        let dispatching =
            Dispatching::at(&parent).ok_or_else(|| NotDispatched::NotItsToDispatch {
                job: caller.clone(),
            })?;
        // Before anything is drafted: an edge points at a record, and a record
        // that is not this parent's own child is one this Drone has no standing
        // to sequence. `crate::coupling` refuses an edge to a Job that does not
        // exist; this refuses one to a Job that does and is somebody else's.
        let siblings = self.children_of(caller).await?;
        let mut waits_on = Vec::with_capacity(asked.after.len());
        for named in &asked.after {
            let peer = siblings
                .iter()
                .find(|child| child.id().as_str() == named)
                .ok_or_else(|| NotDispatched::NotASibling {
                    named: named.clone(),
                })?;
            waits_on.push(ipc::DependencyEdge {
                direction: ipc::DependencyDirection::from(
                    core_model::DependencyDirection::DependsOn,
                ),
                peer: ipc::JobId::from(peer.id()),
            });
        }
        let at = self.now();
        // **The same draft a person's proposal goes through.** Every refusal
        // `crate::drafting` makes — a blank title, a workflow this repository
        // does not hold, a Manifest that is not this one — is made here too,
        // which is why a Drone cannot create a Job a person could not have.
        let (new, _) = self.drafted(
            proposal(ipc::ManifestId::from(self.manifest().id()), asked, waits_on),
            StatedBy::TheSplit {
                parent: caller.clone(),
            },
            &at,
        )?;
        let child = Job::create_sub_dispatched(new, dispatching.origin(), at.clone());
        self.store()
            .lock()
            .await
            .insert_job(&child, &at)
            .map_err(Adrift::Writing)?;
        // After the write, for `Fleet::proposed_job`'s reason: a client told
        // about a row the store then refused would hold a Job that is not
        // there. **The actor is Fleet** — a person approved the split, and the
        // record already says so at the gate the plan went through; what
        // happened here is Fleet acting on that.
        self.publish(ipc::Event::JobCreated(ipc::JobCreated {
            job: ipc::JobSummary::from(&child),
            actor: Actor::Fleet.into(),
            at: (&at).into(),
        }));
        Ok(child.id().clone())
    }

    /// Every Job this one dispatched, in whatever state each is in.
    ///
    /// **Read off `dispatched_by` and never off an edge.** A dependency edge
    /// sequences peers; this is provenance, and `core_model::DispatchOrigin`
    /// says the two are different relations. A parent's children are therefore
    /// discoverable from the children, which is the direction that survives a
    /// child being created after the parent's row was last written.
    pub(crate) async fn children_of(&self, parent: &JobId) -> Result<Vec<Job>, Adrift> {
        let (loaded, _) = self.every_job().await?;
        Ok(loaded
            .jobs
            .into_iter()
            .filter(|job| {
                job.origin() == Origin::SubDispatched
                    && job.dispatched_by().is_some_and(|by| &by.job_id == parent)
            })
            .collect())
    }

    /// Whether the step that just advanced dispatched Jobs that are still
    /// going.
    ///
    /// **Asked about the step that advanced, never about the step ahead.**
    /// Whatever follows a step that created Jobs is work about those Jobs, so
    /// the wait belongs to the step that made them.
    ///
    /// A marker on the step ahead would have worked too, and it is the version
    /// that does not survive: the workflow this is for is a loop — plan,
    /// dispatch, assess, round again — so the step after a dispatch is
    /// sometimes the one the loop returns to and sometimes the one it exits by,
    /// and neither could carry a marker meaning *wait here*. Asked backwards,
    /// nothing downstream has to be labelled at all.
    ///
    /// `false` on every step of every workflow that creates nothing, and on a
    /// dispatch whose children have all finished already — that one goes
    /// straight on, which is the case where nothing waited.
    pub(crate) async fn dispatched_and_waits(
        &self,
        job: &Job,
        step: &StepId,
    ) -> Result<bool, Adrift> {
        let dispatched = job
            .workflow()
            .step(step)
            .is_some_and(ResolvedStep::may_dispatch_jobs);
        Ok(dispatched && !self.children_all_settled(job.id()).await?)
    }

    /// The block the Drone after a dispatch opens with: every Job this one
    /// created, and where each ended.
    ///
    /// **`None` where it dispatched none**, which renders nothing rather than
    /// a sentence saying there were none. A parent that got past a dispatching
    /// step having created nothing is a real answer, and one its own evidence
    /// should say rather than one this block should.
    pub(crate) async fn dispatched_jobs(
        &self,
        parent: &JobId,
    ) -> Result<Option<Dispatched>, Adrift> {
        let children = self.children_of(parent).await?;
        let listed: Vec<(String, String, &'static str)> = children
            .iter()
            .map(|child| {
                (
                    child.id().as_str().to_string(),
                    child.title().as_str().to_string(),
                    child.status().as_wire(),
                )
            })
            .collect();
        Ok(Dispatched::of(&listed))
    }

    /// Whether every Job this one dispatched has stopped, whatever it stopped
    /// at.
    ///
    /// **Terminal, not successful.** The step after a dispatch reports what
    /// happened, and a child that failed is exactly the thing a person needs
    /// that report for — so this is a weaker test than `crate::admitting`'s
    /// `clear_to_run`, which asks whether an upstream landed. A parent held
    /// until its children *succeeded* would never report on the ones that did
    /// not.
    ///
    /// A parent that dispatched nothing has nothing outstanding, which is
    /// `true` and is the honest answer: a plan of no Jobs is a plan whose
    /// next step has nothing to wait for.
    pub(crate) async fn children_all_settled(&self, parent: &JobId) -> Result<bool, Adrift> {
        Ok(self
            .children_of(parent)
            .await?
            .iter()
            .all(|child| child.status().is_terminal()))
    }
}

/// Every sub-dispatched Job on the board, as the parent it names and where it
/// got to.
///
/// **The board reduced to the two facts the wait needs**, so that a caller
/// already holding the board pays nothing to ask about its children and
/// [`waiting_on_children`] needs no read of its own.
pub(crate) fn children_standing(board: &[Job]) -> Vec<(JobId, JobStatus)> {
    board
        .iter()
        .filter(|job| job.origin() == Origin::SubDispatched)
        .filter_map(|job| {
            job.dispatched_by()
                .map(|by| (by.job_id.clone(), job.status()))
        })
        .collect()
}

/// Whether this Job is waiting for Jobs it dispatched.
///
/// **The predicate admission is held by, and the one a Board row is labelled
/// from.** One answer, for `clear_to_run`'s reason: a Board saying a Job is
/// blocked while Fleet is starting it is worse than a Board saying nothing.
///
/// It asks whether the step this Job *last finished* created Jobs, which is
/// the shape a re-queued parent is in: its dispatching step is `advanced` and
/// the step after it has not been entered.
///
/// **Terminal, not successful**, for [`Fleet::children_all_settled`]'s reason:
/// what comes after a dispatch is work about what happened, and a child that
/// failed is the thing that work is most needed for.
pub(crate) fn waiting_on_children(job: &Job, children: &[(JobId, JobStatus)]) -> bool {
    if !dispatched_and_stands_before_the_next(job) {
        return false;
    }
    children
        .iter()
        .any(|(parent, status)| parent == job.id() && !status.is_terminal())
}

/// Whether this Job's current step created Jobs and has finished.
///
/// **Both halves.** A Job standing on a dispatching step that is still
/// `running` has a Drone on it and is not waiting for anything; a Job whose
/// dispatching step has advanced has given its Drone up and is.
fn dispatched_and_stands_before_the_next(job: &Job) -> bool {
    let Some(standing) = job.current_step_id() else {
        return false;
    };
    if job.step(standing).map(|row| row.state()) != Some(StepState::Advanced) {
        return false;
    }
    job.workflow()
        .step(standing)
        .is_some_and(ResolvedStep::may_dispatch_jobs)
}

/// What the child would have been proposed as, if a person had typed it.
///
/// A free function because it decides nothing and reads nothing — the Manifest
/// id is a parameter, which is why this takes no `Fleet` and needs none of its
/// bounds. Five of `ProposeJob`'s fields are fixed here rather than asked for:
/// see the dispatch tool's own module for why a Drone chooses none of them.
fn proposal(
    owner_manifest_id: ipc::ManifestId,
    asked: &DispatchJob,
    dependencies: Vec<ipc::DependencyEdge>,
) -> ipc::ProposeJob {
    ipc::ProposeJob {
        title: asked.title.clone(),
        workflow_id: ipc::WorkflowId::carried(asked.workflow.clone()),
        owner_manifest_id,
        // **Overwritten by the constructor, and stated anyway.** `drafted`
        // hands this back and `create_sub_dispatched` writes
        // `Origin::SubDispatched` over it; `TopLevelOrigin` has no variant that
        // could name what this is, which is the type saying the same thing.
        origin: ipc::TopLevelOrigin::from(core_model::TopLevelOrigin::AutoDetected),
        urgency: ipc::Urgency::from(core_model::Urgency::Normal),
        // Undetermined, as it is for every other proposal: the child's own
        // scope step settles what it writes, and a parent guessing paths for a
        // repository its child has not opened yet would be a second source for
        // an answer settled later with better information.
        atomic: false,
        write_targets: None,
        dependencies,
        // Fleet's, not the parent's. A Drone choosing what its children are
        // spawned as is a Drone choosing what they cost.
        model: None,
        acceptance_criteria: asked
            .acceptance_criteria
            .iter()
            .map(|text| ipc::ProposedCriterion {
                text: text.clone(),
                // **The Judge, because a criterion a parent wrote is prose and
                // nothing mechanical reads it.** `Check` would claim a
                // mechanical Check answers it, and `Attested` would claim a
                // person did.
                source: ipc::CriterionSource::from(core_model::CriterionSource::Judge),
            })
            .collect(),
        subject: None,
        // The brief. It rides on `facts` because that is where a proposal's own
        // description rides — the child's Drone is briefed from it, and a child
        // briefed from a title alone is one the decomposition was thrown away
        // for.
        facts: asked.brief.clone(),
        attachments: Vec::new(),
    }
}
