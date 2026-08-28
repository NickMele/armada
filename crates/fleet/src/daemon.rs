//! The daemon core: what Fleet is made of, the one slot it works in, and the
//! things it can be asked.
//!
//! Two neighbours carry what this file deliberately does not.
//! [`working`](mod@crate::working) is the slot's contents — a type with an
//! invariant of its own, kept away from the logic that moves it, which is the
//! split the 500-line rule exists to prompt. [`dispatch`](mod@crate::dispatch)
//! is what happens to a Job while it is in that slot.
//!
//! # One Job at a time, and the queue is not a list
//!
//! [`Fleet`] holds one [`Working`] slot. A Job approved while another is being
//! worked stays at `queued`, which is a status the registry already has and the
//! store already persists — **so there is no queue object here**, and no
//! ordering held in memory that a restart could lose or that could disagree
//! with the log. `queued` is the queue. Throughput is a later milestone, and
//! what it will add is a second slot, not a scheduler over a structure this one
//! already built.
//!
//! # A refused transition is not survivable
//!
//! Every move goes through `Job::transition` or `Job::transition_step`, and
//! every refusal comes straight back out as `Adrift::IllegalMove` or
//! `Adrift::IllegalStepMove`. There is no arm anywhere in this crate that logs
//! one and continues. A refusal means Fleet asked the machine for something the
//! edge table says cannot happen, which is a bug in Fleet and has to read like
//! one.
//!
//! # Nothing here reads a clock or invents an id
//!
//! [`Clock`] and [`Mint`] are injected. That is what lets an end-to-end test
//! assert on the exact instant and the exact id of every row it produced, and
//! it is the same rule `core-model`, `store` and `config` each state about
//! themselves — the rule needs somewhere for the reading to happen, and this is
//! the composition root's business to hand in.
//!
//! # Locking, and the one order it is taken in
//!
//! Two locks: the working slot and the store. **The slot is always taken
//! first**, and no path takes the store and then reaches for the slot. The gate
//! holds the slot across a Check, which is what one-Job-at-a-time means in
//! practice: nothing else can be dispatched while `cargo test` runs, and
//! nothing wants to be.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, Model, ModelClient, SpawnConfigRefused, Vcs, WorkProduct,
};
use config::{Manifest, ResolvedWorkflow};
use core_model::{
    Actor, Job, JobId, JobStatus, Target, Timestamp, TransitionReason, Ulid, WorkflowId,
};
use store::{LoadAllError, LoadJobError, Loaded, Moved, Store};
use tokio::sync::Mutex;

use crate::adrift::Adrift;
use crate::clock::Clock;
use crate::converging::{StepNorms, Wandering};
use crate::delivery::Delivered;
use crate::drafting::StatedBy;
use crate::drone::{aftermath, environment, Aftermath, Ending, HostPaths};
use crate::evidence::EvidenceInbox;
use crate::gate::{CheckBudget, Ruling};
use crate::judging::{JudgeBudget, Judging};
use crate::mint::Mint;
use crate::proposal::Proposing;
use crate::scope::Drifting;
use crate::working::Working;

/// What Fleet knows about the machine it runs on.
///
/// **Resolved once, by the composition root, and never read from this
/// process.** `crate::drone` gives the reason for the two paths: a Drone that
/// inherited Fleet's own `PATH` would find a different toolchain on two
/// machines, and a different one again after a shell profile changes. The same
/// argument makes every field here an argument rather than a lookup.
///
/// Public fields and no `Default`, so a caller writes each one out.
#[derive(Clone, Debug)]
pub struct Host {
    /// The repository every worktree is added to. Absolute.
    pub repo_root: String,
    /// What a Drone's `PATH` is set to. Fleet's choice, not Fleet's own.
    pub path: String,
    /// The home directory the agent CLI reads its credentials from. **The
    /// confinement's known floor** — see `crate::drone::HostPaths`.
    pub home: String,
    /// Who the operator is. The agent CLI will not authenticate without it,
    /// however readable its credentials are — see `crate::drone::environment`.
    pub user: String,
    /// The strict MCP configuration a Drone is bound to.
    pub mcp_config: String,
    /// Where Fleet keeps its own copy of a Job's attachments, outside every
    /// worktree. `drafted()` writes under `<attachments_dir>/<job_id>/`, and
    /// `dispatch` reads from there to seed the worktree a Drone actually sees.
    pub attachments_dir: String,
}

/// Everything Fleet is assembled from.
///
/// A plain struct with public fields rather than a builder, for the reason
/// `NewJob` gives: there is no `Default`, so a caller writes every field out
/// and cannot forget one, and adding a field is a compile error at the one call
/// site that matters.
pub struct Fittings<H, V, W> {
    pub store: Store,
    pub harness: H,
    pub vcs: V,
    pub work: W,
    pub clock: Arc<dyn Clock>,
    pub mint: Arc<dyn Mint>,
    /// Every workflow a Job may run, keyed by the `workflow_id` its definition
    /// carries. Fleet is pointed at a repository and `.armada/workflows/` may
    /// hold more than one definition — a proposal names which one it wants,
    /// and a name this map does not hold is refused at creation instead of
    /// written onto the record unverified.
    pub workflows: BTreeMap<WorkflowId, ResolvedWorkflow>,
    /// The `armada.yml` that workflow resolved against. Held because a Drone's
    /// toolbelt is built from the commands it declares.
    pub manifest: Manifest,
    pub host: Host,
    pub budget: CheckBudget,
    /// What a step is expected to cost before the thrashing chain looks at it.
    /// See [`StepNorms`] for why it has no default.
    pub norms: StepNorms,
    /// What makes a Judge call. **A pointer rather than a type parameter**: the
    /// seam renders and cannot fail, so nothing about it needs to be generic.
    pub judge: Arc<dyn ModelClient + Send + Sync>,
    /// How long one Judge call may take. See [`JudgeBudget`] for why it has no
    /// default.
    pub judge_budget: JudgeBudget,
    /// What a step naming no model of its own is judged by. **Resolved by the
    /// composition root**, like every other input here — which model is cheap
    /// is a vendor's fact, and nothing below Fleet may spell one.
    pub judge_model: Model,
    /// What a dispatch request is read by. **Its own dial and not the Judge's**
    /// — this call fires on every dispatch rather than on every criterion, so
    /// the two are raised for different reasons and at different prices.
    pub proposer_model: Model,
    /// The models a Job may name, and the one it gets when it names none.
    ///
    /// **Resolved by the composition root, like every other input here.**
    /// Nothing below reads configuration, which is what lets a test plant a
    /// roster and assert on what a proposal with no model was given. Where the
    /// default is blank, a proposal that names no model is refused at creation
    /// rather than at spawn.
    pub models: ipc::ModelChoices,
    pub events: api::Broadcaster,
}

/// What the boot read found and what the reconciliation did about it.
#[derive(Debug, Default)]
pub struct Reconciled {
    /// Jobs the store says were `running` and whose Drone this Fleet does not
    /// have. Every one is now `escalated`, reason `interrupted`.
    pub interrupted: Vec<JobId>,
    /// Rows whose cached status disagreed with the log and were corrected.
    pub repaired: usize,
    /// Rows that would not rebuild at all. **Never dropped** — carried out so a
    /// caller cannot end up holding a short list with nothing saying so.
    pub unreadable: Vec<String>,
    /// The Job dispatched on the way out, where the slot was free and one was
    /// waiting.
    pub admitted: Option<JobId>,
}

/// What one turn of the loop did. Every field is ordinarily empty.
#[derive(Debug, Default)]
pub struct Turned {
    /// The gate's answer to a submission that had landed.
    pub ruled: Option<Ruling>,
    /// What followed from a Drone that had gone.
    pub after: Option<Aftermath>,
    /// The Job admitted because the slot came free.
    pub admitted: Option<JobId>,
    /// What became of a finished Job's branch. Present on the turn that
    /// finished one and empty on every other, like the two fields above it.
    pub delivered: Option<Delivered>,
    /// Work seen outside the step's declared scope, on the turn it was first
    /// seen. **It fails nothing here** — the Drone may declare again, and the
    /// gate reads the footprint for itself when the step ends.
    pub drifting: Option<Drifting>,
    /// How far the thrashing chain got with the step being worked. Empty on
    /// every turn of a step inside its norms, which is nearly all of them.
    pub wandering: Option<Wandering>,
}

/// The daemon core: **the only writer of Job state.**
pub struct Fleet<H, V, W> {
    store: Mutex<Store>,
    harness: Arc<H>,
    vcs: V,
    work: W,
    clock: Arc<dyn Clock>,
    mint: Arc<dyn Mint>,
    workflows: BTreeMap<WorkflowId, ResolvedWorkflow>,
    manifest: Manifest,
    host: Host,
    budget: CheckBudget,
    norms: StepNorms,
    judge: Arc<dyn ModelClient + Send + Sync>,
    judge_budget: JudgeBudget,
    judge_model: Model,
    proposer_model: Model,
    models: ipc::ModelChoices,
    events: api::Broadcaster,
    /// Every Job somebody could be watching. **Minted here, not a fitting** —
    /// nothing outside this crate holds one, because a viewer reaches it
    /// through `api::Daemon::observe_job` rather than through the composition
    /// root.
    turns: api::Turns,
    inbox: EvidenceInbox,
    /// What the last finished Job's delivery came to, waiting for the turn that
    /// reports it. **Drained, not read** — a second turn must not report a
    /// push that happened before it.
    delivered: Mutex<Option<Delivered>>,
    working: Mutex<Option<Working>>,
    /// **This process's** run id, minted once at assembly.
    ///
    /// It names the emitter rather than a record, which is the one id a
    /// process mints for itself — and it is why a Fleet restart is visible as
    /// this value changing rather than as nothing at all.
    run: Ulid,
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
    pub fn assembled(fittings: Fittings<H, V, W>) -> Fleet<H, V, W> {
        let run = fittings.mint.ulid();
        Fleet {
            store: Mutex::new(fittings.store),
            harness: Arc::new(fittings.harness),
            vcs: fittings.vcs,
            work: fittings.work,
            clock: fittings.clock,
            mint: fittings.mint,
            workflows: fittings.workflows,
            manifest: fittings.manifest,
            host: fittings.host,
            budget: fittings.budget,
            norms: fittings.norms,
            judge: fittings.judge,
            judge_budget: fittings.judge_budget,
            judge_model: fittings.judge_model,
            proposer_model: fittings.proposer_model,
            models: fittings.models,
            events: fittings.events,
            turns: api::Turns::new(),
            inbox: EvidenceInbox::new(),
            delivered: Mutex::new(None),
            working: Mutex::new(None),
            run,
        }
    }

    /// **The boot read, and the reconciliation.** Nothing runs until this has.
    ///
    /// A Job the store says was `running` has no Drone, because a Drone is held
    /// in memory by the Fleet that spawned it and this Fleet has just started.
    /// It is `escalated`, reason `interrupted` — the answer `crate::aftermath`
    /// gives for a process that is gone having left nothing, reached through
    /// that function rather than restated here.
    ///
    /// **Never resumed silently.** There is no path in this crate that puts a
    /// Drone back onto a Job that was running when Fleet died.
    pub async fn reconcile(&self) -> Result<Reconciled, Adrift> {
        let (loaded, unreadable) = self.every_job().await?;
        let mut reconciled = Reconciled {
            repaired: loaded.repaired.len(),
            unreadable,
            ..Reconciled::default()
        };
        // Every Job the store says holds a Drone, whatever status it is under.
        // A Drone is spoken to through a pipe the Fleet that spawned it holds,
        // so this Fleet has none of them — and `assigned_drone` is read to
        // decide whether a Job can be redirected at all. A column left saying
        // yes is a redirect that injects a turn into nothing.
        let held: Vec<Job> = loaded
            .jobs
            .iter()
            .filter(|job| job.assigned_drone().is_some() || job.status() == JobStatus::Running)
            .cloned()
            .collect();
        for job in held {
            self.drone_left(job.id()).await;
            let job = self.load(job.id()).await?;
            if let Aftermath::JobMoves(target) =
                aftermath(job.status(), &Ending::Vanished, self.left())
            {
                self.move_job(&job, target, Actor::Fleet).await?;
                reconciled.interrupted.push(job.id().clone());
            }
        }
        let mut working = self.working.lock().await;
        reconciled.admitted = self.admit_next(&mut working).await?;
        Ok(reconciled)
    }

    /// One turn: settle what landed, reap a Drone that is gone, admit the next
    /// approved Job if the slot came free.
    ///
    /// **Not a scheduler.** It runs the three things that can follow from the
    /// world having moved, in the one order they can follow in, over a slot
    /// that holds one Job.
    pub async fn turn(&self) -> Result<Turned, Adrift> {
        let mut working = self.working.lock().await;
        // First, because the reading it takes is the one the drift check needs
        // and a turn must not open the same repository twice. It answers `None`
        // on the turns it declines to read, and the drift check then reads for
        // itself exactly as it did before this existed.
        let footprint = self.watch_footprint(&mut working).await;
        // Before the gate, so a step whose evidence lands this turn has its
        // last live reading taken while its Drone is still the one being
        // watched — and after nothing, because the check reads a worktree and
        // must not run against a slot the gate has just cleared.
        let drifting = self.watch_scope(&mut working, footprint.as_ref()).await;
        // After the drift reading it consumes and before the gate, which is the
        // one place both are true: a step whose evidence lands this turn is at
        // the gate rather than thrashing, and `settle` may clear the slot.
        let wandering = self.watch_convergence(&mut working).await?;
        let ruled = self.settle(&mut working).await?;
        let delivered = self.delivered.lock().await.take();
        let after = self.reap(&mut working).await?;
        let admitted = self.admit_next(&mut working).await?;
        Ok(Turned {
            ruled,
            after,
            admitted,
            delivered,
            drifting,
            wandering,
        })
    }

    /// A Job drafted onto the approval gate. **The gate is unchanged** — what
    /// comes back is at `awaiting_approval`, not a running Job.
    /// **And it publishes.** A Job created while a client was connected never
    /// reached it: `ipc::Event` carried one kind, `job.state_changed`, and
    /// creating a Job is not a state change — so nothing was published and
    /// nothing woke Bridge. `job.created` is the kind that says a row appeared,
    /// and it carries the row whole so a Board inserts it rather than re-reading.
    ///
    /// The actor is **human**. A proposal is a person's act or Helm's; Fleet
    /// creates no Job of its own accord at M1, and the log envelope's actor
    /// vocabulary has no fourth value to distinguish the two with.
    pub async fn propose(&self, proposal: ipc::ProposeJob) -> Result<Job, Adrift> {
        // Hand entry, which is the override rather than the path. Entry zero
        // records that a person stated this scope and not the call, which is
        // what makes the call evaluable against the decisions people made.
        self.proposed_job(proposal, StatedBy::APerson).await
    }

    /// The same creation, with who stated the scope carried through to entry
    /// zero. **The only difference between the two dispatch paths**, which is
    /// why they share everything below it.
    pub(crate) async fn proposed_job(
        &self,
        proposal: ipc::ProposeJob,
        stated: StatedBy,
    ) -> Result<Job, Adrift> {
        let at = self.now();
        let (new, origin) = self.drafted(proposal, stated, &at)?;
        let job = Job::create_top_level(new, origin, at.clone());
        self.store
            .lock()
            .await
            .insert_job(&job, &at)
            .map_err(Adrift::Writing)?;
        // After the write, never before: a client told about a row the store
        // then refused would hold a Job that does not exist, and a resync would
        // silently remove it.
        self.publish(ipc::Event::JobCreated(ipc::JobCreated {
            job: ipc::JobSummary::from(&job),
            actor: Actor::Human.into(),
            at: (&at).into(),
        }));
        Ok(job)
    }

    /// Release a Job to spawn, and dispatch it if the slot is free.
    ///
    /// The transition is `awaiting_approval -> queued` and the actor is
    /// **human**: this is the primary autonomy control, and Fleet is not
    /// allowed to be recorded as the one that took it.
    pub async fn approve(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        self.move_job(&job, Target::Queued, Actor::Human).await?;
        {
            let mut working = self.working.lock().await;
            self.admit_next(&mut working).await?;
        }
        self.load(job_id).await
    }

    /// End the Drone. **The Job survives**, and its worktree is held.
    ///
    /// Where the Job then stands is `crate::aftermath`'s answer and not this
    /// method's: a process that is gone having left no evidence pauses the Job
    /// for a person. That is not terminal, so nothing here ends a Job — and it
    /// is not `running` either, which is the state the milestone refuses.
    pub async fn kill_drone(&self, job_id: &JobId) -> Result<Job, Adrift> {
        {
            let mut working = self.working.lock().await;
            if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
                let ending = Ending::of(
                    &working
                        .as_ref()
                        .expect("the slot was just read as full")
                        .heard(),
                );
                let standing = self.load(job_id).await?.status();
                self.end_the_drone(&mut working).await;
                let job = self.load(job_id).await?;
                if let Aftermath::JobMoves(target) = aftermath(standing, &ending, self.left()) {
                    self.move_job(&job, target, Actor::Human).await?;
                }
            }
        }
        self.load(job_id).await
    }

    /// End the Job at `killed`. Terminal, and carrying no verdict.
    ///
    /// Legal from every non-terminal status, including those with no process
    /// under them — which is why it cannot be spelled as
    /// [`kill_drone`](Fleet::kill_drone).
    pub async fn kill_job(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let mut working = self.working.lock().await;
        if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
            self.end_the_drone(&mut working).await;
        }
        let job = self.load(job_id).await?;
        let killed = self.move_job(&job, Target::Killed, Actor::Human).await?;
        self.admit_next(&mut working).await?;
        Ok(killed)
    }

    /// Every Job, **and every row that would not load**.
    ///
    /// The store refuses to hand back a short list; this refuses to complete
    /// one. Both halves come out together and the caller decides.
    pub async fn every_job(&self) -> Result<(Loaded, Vec<String>), Adrift> {
        match self.store.lock().await.load_all_jobs() {
            Ok(loaded) => Ok((loaded, Vec::new())),
            Err(LoadAllError::SomeJobsUnreadable { loaded, failed }) => Ok((
                loaded,
                failed.iter().map(|refusal| refusal.to_string()).collect(),
            )),
            Err(fault) => Err(Adrift::BootRead(fault)),
        }
    }

    /// One Job, folded from its events. The status column is not read.
    pub async fn load(&self, job_id: &JobId) -> Result<Job, Adrift> {
        self.store
            .lock()
            .await
            .load_job(job_id)
            .map_err(Adrift::Reading)
    }

    /// The qualifying reason the Job's last transition stored, where it stored
    /// one. Read from the log, because the `jobs` row does not carry it and
    /// only a caller holding the log can supply it.
    pub async fn last_reason(&self, job_id: &JobId) -> Result<Option<TransitionReason>, Adrift> {
        let events = self
            .store
            .lock()
            .await
            .events_for(job_id)
            .map_err(|cause| Adrift::Reading(LoadJobError::Unreadable(cause)))?;
        Ok(events.iter().rev().find_map(|event| match event.moved() {
            Moved::Job { reason, .. } => Some(reason.clone()),
            Moved::Step { .. } | Moved::Drone { .. } => None,
        }))
    }

    /// Which Job is being worked, if any.
    pub async fn working_on(&self) -> Option<JobId> {
        self.working
            .lock()
            .await
            .as_ref()
            .map(|at_work| at_work.standing().0)
    }

    /// The stream Fleet publishes transitions on. Cloned for the listener.
    pub fn events(&self) -> api::Broadcaster {
        self.events.clone()
    }

    // The seams, read-only, for `dispatch`. Private accessors rather than
    // `pub(crate)` fields: a field would be assignable from the other module,
    // and Fleet's own configuration is fixed at assembly.

    pub(crate) fn store(&self) -> &Mutex<Store> {
        &self.store
    }
    pub(crate) fn harness(&self) -> &Arc<H> {
        &self.harness
    }
    pub(crate) fn vcs(&self) -> &V {
        &self.vcs
    }
    pub(crate) fn work(&self) -> &W {
        &self.work
    }
    /// One workflow by id, or `None` where this Fleet holds no such definition.
    pub(crate) fn workflow_named(&self, id: &WorkflowId) -> Option<&ResolvedWorkflow> {
        self.workflows.get(id)
    }
    /// Every workflow this Fleet holds, for `serving`'s `list_workflows`.
    pub(crate) fn workflows(&self) -> &BTreeMap<WorkflowId, ResolvedWorkflow> {
        &self.workflows
    }
    pub(crate) fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub(crate) fn host(&self) -> &Host {
        &self.host
    }
    pub(crate) fn budget(&self) -> CheckBudget {
        self.budget
    }
    pub(crate) fn norms(&self) -> StepNorms {
        self.norms
    }

    /// What the gate needs in order to ask the Judge.
    ///
    /// The environment is the Drone's own list, built the same way and by the
    /// same function — a Judge call needs the credential floor for the reason a
    /// Drone does, and a second list here would be a second answer.
    pub(crate) fn judging(&self) -> Result<Judging, SpawnConfigRefused> {
        Ok(Judging {
            client: Arc::clone(&self.judge),
            budget: self.judge_budget,
            default_model: self.judge_model.clone(),
            environment: environment(HostPaths {
                path: &self.host.path,
                user: &self.host.user,
                home: &self.host.home,
            })?,
        })
    }
    /// What the dispatch path needs in order to ask the proposer.
    ///
    /// **The Judge's client and the Judge's budget**, because the call is the
    /// same call: one turn, no toolset, no directory. Only the model differs,
    /// and only because it is a separate dial.
    pub(crate) fn proposing(&self) -> Result<Proposing, SpawnConfigRefused> {
        Ok(Proposing {
            client: Arc::clone(&self.judge),
            budget: self.judge_budget,
            model: self.proposer_model.clone(),
            environment: environment(HostPaths {
                path: &self.host.path,
                user: &self.host.user,
                home: &self.host.home,
            })?,
        })
    }
    /// The models a Job may name. Read at creation and served by
    /// `list_models`; nothing else consults it.
    pub(crate) fn models(&self) -> &ipc::ModelChoices {
        &self.models
    }
    pub(crate) fn mint(&self) -> &Arc<dyn Mint> {
        &self.mint
    }
    /// **The one clock reading in the crate.** Everything below Fleet takes its
    /// instant as an argument, and this is where the argument comes from.
    pub(crate) fn now(&self) -> Timestamp {
        self.clock.now()
    }
    /// The clock itself, for the one thing that outlives a call and still has
    /// to stamp what it writes: a transcript's writer.
    pub(crate) fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }
    pub(crate) fn run(&self) -> &Ulid {
        &self.run
    }
    pub(crate) fn publish(&self, event: ipc::Event) {
        self.events.publish(event);
    }
    /// The per-Job transcript channels. Dispatch opens one, `serving`
    /// subscribes to it.
    pub(crate) fn turns(&self) -> &api::Turns {
        &self.turns
    }
    /// The working slot, for `evidence`.
    ///
    /// Everything the Evidence tool binds to is in it, and taking this lock
    /// once is what makes which-Job, which-step and which-type one decision
    /// rather than three reads a turn can interleave with.
    pub(crate) fn slot(&self) -> &Mutex<Option<Working>> {
        &self.working
    }
    pub(crate) fn inbox(&self) -> &EvidenceInbox {
        &self.inbox
    }
    /// Where a finished Job's delivery is left for the turn that reports it.
    pub(crate) fn delivery_slot(&self) -> &Mutex<Option<Delivered>> {
        &self.delivered
    }
}
