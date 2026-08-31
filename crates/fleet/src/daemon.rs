//! The daemon core: what Fleet is made of, the one slot it works in, and the
//! things it can be asked.
//!
//! Two neighbours carry what this file deliberately does not.
//! [`working`](mod@crate::working) is the slot's contents — a type with an
//! invariant of its own, kept away from the logic that moves it, which is the
//! split the 500-line rule exists to prompt. [`dispatch`](mod@crate::dispatch)
//! is what happens to a Job while it is in that slot.
//!
//! # N Jobs at a time, and the queue is still not a list
//!
//! [`Fleet`] holds a [`Slots`] roster: one [`Working`] slot per Job being
//! worked, bounded by [`Concurrency`]. A Job approved while the bound is spent
//! stays at `queued`, which is a status the registry already has and the store
//! already persists — **so there is no queue object here**, and no ordering
//! held in memory that a restart could lose or that could disagree with the
//! log. `queued` is the queue, and what `#50` added was the second slot rather
//! than a scheduler over the structure the first one already built.
//!
//! **The bound is on Drones and never on approvals.** Every Job-level dispatch
//! is still approved explicitly and one by one; this decides how many *approved*
//! Jobs run at once.
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
//! Three locks: the roster, one Job's slot, and the store. **They are taken in
//! that order and in no other**, and no path takes a slot and then reaches for
//! the roster. [`crate::slots`] holds the argument.
//!
//! The gate still holds a slot across a Check. What changed is whose: it is the
//! slot of the Job being checked, so a `cargo nextest` that runs for a quarter
//! of an hour holds up that Job's own Drone and nothing else's.
//!
//! A fourth lock is not in the order because nothing else takes it while
//! holding one of the three except in the one direction: [`Fleet::merge_end`]
//! serialises the rebase-and-push tail, so exactly one Job at a time touches
//! the repository every worktree is cut from. See [`crate::delivery`].

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, Model, ModelClient, SpawnConfigRefused, Vcs, WorkProduct,
};
use config::{Manifest, ResolvedWorkflow};
use core_model::{
    Actor, Job, JobId, JobStatus, StepId, Target, Timestamp, TransitionReason, Ulid, WorkflowId,
};
use store::{LoadAllError, LoadJobError, Loaded, Moved, Store};
use tokio::sync::Mutex;

use crate::admitting::Polled;
use crate::adrift::Adrift;
use crate::clock::Clock;
use crate::converging::StepNorms;
use crate::delivery::Delivered;
use crate::drafting::StatedBy;
use crate::drone::{aftermath, environment, Aftermath, Ending, HostPaths};
use crate::drone_moves::steps_holding_a_drone;
use crate::dry_run::DryRuns;
use crate::evidence::EvidenceInbox;
use crate::gate::CheckBudget;
use crate::headroom::{Headroom, Machine, Polling};
use crate::judging::{Aloft, JudgeBudget, Judging, Marking};
use crate::mint::Mint;
use crate::peer::{attributed, Drones, NotACaller, PeerOf};
use crate::proposal::Proposing;
use crate::silence::Liveness;
use crate::slots::{Concurrency, Slot, Slots};

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
    /// The loopback port Fleet is listening on.
    ///
    /// **Held because a connection to it is what names a Drone** — see
    /// `crate::peer`, which matches a caller's port against this one as a pair.
    /// It is the same number `mcp_config` points at, resolved once by the
    /// composition root from the listener it actually bound.
    pub port: u16,
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
    /// How a caller is placed: which process holds the connection a tool call
    /// arrived on. **A seam so a test can plant one** — the shipped answer is
    /// [`peer::Kernel`](crate::peer::Kernel), and a fixture has no sockets to
    /// ask about.
    pub peers: Arc<dyn PeerOf>,
    /// How many Jobs Fleet may work at once. **The
    /// `settings.concurrency-cap` row, enforced** — see [`Concurrency`], which
    /// has no default for the reason none of the four dials above it does.
    pub concurrency: Concurrency,
    /// What the machine has left, asked rather than assumed. **A seam so a
    /// test can plant one**, exactly as `peers` is — the shipped answer is
    /// [`TheMachine`](crate::headroom::TheMachine) and a fixture has no machine
    /// it can hold still.
    pub machine: Arc<dyn Machine>,
    /// How much of the machine must be free before another Drone starts. **The
    /// `settings.cpu-mem-headroom-threshold-for-spawning` row, enforced** — see
    /// [`Headroom`], which has no default for [`Concurrency`]'s reason.
    pub headroom: Headroom,
    /// How stale a machine reading may be. **The
    /// `settings.fleet-health-check-resource-poll-interval` row** — see
    /// [`Polling`] for why it is a freshness bound rather than a second timer.
    pub polling: Polling,
    pub budget: CheckBudget,
    /// What a step is expected to cost before the thrashing chain looks at it.
    /// See [`StepNorms`] for why it has no default.
    pub norms: StepNorms,
    /// How long a Drone may say nothing before Fleet asks, and how many times
    /// it asks. Its own value rather than a fourth `StepNorms` number: what it
    /// bounds is the Drone being there at all, and nothing about it is measured
    /// against a step's work. See [`Liveness`].
    pub liveness: Liveness,
    /// How many times one step may ask Fleet to run its Checks. Its own value
    /// for [`Liveness`]'s reason: what it bounds is money spent answering the
    /// Drone rather than anything about the step's work. See
    /// [`DryRuns`](crate::DryRuns).
    pub dry_runs: DryRuns,
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
    /// The Jobs dispatched on the way out, where the bound had room and they
    /// were waiting. Empty on the ordinary boot.
    pub admitted: Vec<JobId>,
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
    liveness: Liveness,
    dry_runs: DryRuns,
    judge: Arc<dyn ModelClient + Send + Sync>,
    judge_budget: JudgeBudget,
    /// The Judge call that is out right now, or none. **The one piece of Fleet
    /// state that is only ever true for as long as it takes** — it is never
    /// written down, because a record of it would outlive the fact.
    aloft: Aloft,
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
    /// What each finished Job's delivery came to, waiting for the turn that
    /// reports it. **Drained, not read** — a second turn must not report a
    /// push that happened before it.
    ///
    /// **Keyed by Job**, unlike the single value it was: two Jobs can reach
    /// their branch in one turn, and one slot for both would have the second
    /// one's push overwrite the first's before anybody was told about it.
    delivered: Mutex<BTreeMap<JobId, Delivered>>,
    /// Which Jobs are being worked and how many may be. See [`crate::slots`]
    /// for the two locks and the order they are taken in.
    slots: Mutex<Slots>,
    machine: Arc<dyn Machine>,
    headroom: Headroom,
    polling: Polling,
    /// The last machine reading, and when it was taken. **Never written down**
    /// — headroom frees on its own, so a reading that outlived the process
    /// would be a reason that was already wrong when it was read back.
    ///
    /// Taken after the roster and never before it, which is the order
    /// `crate::slots` states. Nothing is held while this one is.
    polled: Mutex<Option<Polled>>,
    /// Which process is working which Job. See [`Drones`], which argues why
    /// this is not the same fact as the pid inside the slot.
    ///
    /// A `std::sync::Mutex` rather than tokio's: it is never held across an
    /// `.await`, and what it guards is a map of a handful of integers.
    drones: std::sync::Mutex<Drones>,
    peers: Arc<dyn PeerOf>,
    /// The rebase-and-push tail, held by one Job at a time.
    ///
    /// **Every worktree is cut from one `.git`**, and whether two of them can
    /// rebase and push into it concurrently is not established — `#50` accepted
    /// the serialisation rather than discovering git's ref lockfiles by way of
    /// a Job dying at its push, unattended. So dispatch and the work run
    /// N-wide and this one part does not.
    ///
    /// **It is at the tail and not at admission.** A lock taken when a Job
    /// starts would be the single working slot again under another name; this
    /// one is taken when a Job's branch is touched and released when it has
    /// been.
    merge_end: Mutex<()>,
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
            liveness: fittings.liveness,
            dry_runs: fittings.dry_runs,
            judge: fittings.judge,
            judge_budget: fittings.judge_budget,
            aloft: Aloft::default(),
            judge_model: fittings.judge_model,
            proposer_model: fittings.proposer_model,
            models: fittings.models,
            events: fittings.events,
            turns: api::Turns::new(),
            inbox: EvidenceInbox::new(),
            delivered: Mutex::new(BTreeMap::new()),
            slots: Mutex::new(Slots::bounded_by(fittings.concurrency)),
            machine: fittings.machine,
            headroom: fittings.headroom,
            polling: fittings.polling,
            polled: Mutex::new(None),
            drones: std::sync::Mutex::new(Drones::default()),
            peers: fittings.peers,
            merge_end: Mutex::new(()),
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
        // Every **step** the store says holds a Drone, whatever status its Job
        // is under. A Drone is spoken to through a pipe the Fleet that spawned
        // it holds, so this Fleet has none of them — and `assigned_drone` is
        // read to decide whether a Job can be redirected at all. A column left
        // saying yes is a redirect that injects a turn into nothing.
        //
        // **Steps and not Jobs**, because the pointer is one per step: a Job
        // whose current step holds nothing may still have an earlier step whose
        // row does, and a walk over Jobs would leave it standing. A `running`
        // Job with no pointer anywhere is still carried here, because a Job
        // marked running by a Fleet that is gone is interrupted whether or not
        // its Drone was ever recorded.
        let held: Vec<(Job, Vec<StepId>)> = loaded
            .jobs
            .iter()
            .map(|job| (job.clone(), steps_holding_a_drone(job)))
            .filter(|(job, steps)| !steps.is_empty() || job.status() == JobStatus::Running)
            .collect();
        for (job, steps) in held {
            for step in steps {
                self.drone_left(job.id(), &step).await?;
            }
            let job = self.load(job.id()).await?;
            if let Aftermath::JobMoves(target) =
                aftermath(job.status(), &Ending::Vanished, self.left(job.id()))
            {
                self.move_job(&job, target, Actor::Fleet).await?;
                reconciled.interrupted.push(job.id().clone());
            }
        }
        reconciled.admitted = self.admit_next().await?;
        Ok(reconciled)
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
        // Before `drafted`, which is sync and cannot read the board: an edge is
        // a pointer, and a peer that does not exist is the one shape a cycle
        // needs. `coupling::peers_held` carries why.
        self.peers_held(&proposal.dependencies).await?;
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
        self.admit_next().await?;
        self.load(job_id).await
    }

    /// End the Drone. **The Job survives**, and its worktree is held.
    ///
    /// Where the Job then stands is `crate::aftermath`'s answer and not this
    /// method's: a process that is gone having left no evidence pauses the Job
    /// for a person. That is not terminal, so nothing here ends a Job — and it
    /// is not `running` either, which is the state the milestone refuses.
    pub async fn kill_drone(&self, job_id: &JobId) -> Result<Job, Adrift> {
        if let Some(slot) = self.slot_of(job_id).await {
            let mut working = slot.lock().await;
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
                if let Aftermath::JobMoves(target) = aftermath(standing, &ending, self.left(job_id))
                {
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
        if let Some(slot) = self.slot_of(job_id).await {
            let mut working = slot.lock().await;
            if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
                self.end_the_drone(&mut working).await;
            }
        }
        // And every step the *record* still names one on, which the slot cannot
        // answer for: a Fleet that died holding a Drone leaves the pointer set,
        // and `redispatch` reaches this on a Job whose process this Fleet never
        // held. Without it a killed Job reads on the Board as one with a Drone
        // still on it. `drone_left` answers `Ok` for a step holding nothing, so
        // the ordinary path pays one load.
        self.every_exit_recorded(job_id).await?;
        let job = self.load(job_id).await?;
        let killed = self.move_job(&job, Target::Killed, Actor::Human).await?;
        self.admit_next().await?;
        Ok(killed)
    }

    /// Delete the Job's whole record. **Real deletion**, through
    /// `Store::forget_job`, and only from a terminal status — a Job still in
    /// flight has no record to erase, only a status to move, and `kill_job` is
    /// the act that ends one that is not there yet.
    ///
    /// **It does not reclaim the worktree or the branch.** `armada clean`
    /// already owns that, on its own retention schedule; folding it in here
    /// would give one call two unrelated things to fail at.
    ///
    /// `Store::forget_job` runs through the same lock every other write
    /// takes — there is no second connection opened for it, which is what
    /// makes this safe to call from inside a live Fleet in the first place.
    pub async fn forget_job(&self, job_id: &JobId) -> Result<(), Adrift> {
        let job = self.load(job_id).await?;
        if !job.status().is_terminal() {
            return Err(Adrift::NotForgettable {
                job: job_id.clone(),
                status: job.status(),
            });
        }
        self.store
            .lock()
            .await
            .forget_job(job_id)
            .map_err(Adrift::Writing)?;
        self.publish(ipc::Event::JobForgotten(ipc::JobForgotten {
            job_id: job_id.into(),
        }));
        Ok(())
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

    /// Which Jobs are being worked. Empty where none is.
    ///
    /// **A list rather than an `Option`**, which is `#50` arriving on the one
    /// method every surface asked the question through. A caller wanting to
    /// know about one Job asks [`Fleet::slot_of`].
    pub async fn working_on(&self) -> Vec<JobId> {
        self.slots.lock().await.working_on()
    }

    /// Every Drone this Fleet is holding, as pid and Job.
    ///
    /// **Its only reader today is `crate::tests::concurrency`**, which needs
    /// the pids to say which Drone is calling from which port. Said out loud
    /// rather than left to be discovered: nothing on the wire carries this, and
    /// a Doctor probe that wanted "which process is working which Job" is the
    /// reader it is waiting for.
    pub fn drones_at_work(&self) -> Vec<(JobId, u32)> {
        self.held_drones().each()
    }

    /// Which Job made this call.
    ///
    /// **The whole of what a Drone tool call is bound to**, and it asks the
    /// transport rather than the body: `crate::peer` matches the caller's port
    /// and Fleet's own listening port as a *pair* against the processes Fleet
    /// spawned, because a local port number alone is not unique on a host and
    /// names the wrong process deterministically. Spike 12 is the measurement.
    ///
    /// [`NotACaller`] is the only failure, and it is a refusal rather than a
    /// guess: a caller Fleet cannot place is one whose work it must not credit
    /// to anybody.
    pub fn caller_of(&self, caller: &api::Caller) -> Result<JobId, NotACaller> {
        attributed(
            caller,
            self.host.port,
            &self.held_drones().each(),
            self.peers.as_ref(),
        )
        .ok_or(NotACaller)
    }

    /// A Drone started on this Job, as this process.
    pub(crate) fn drone_at_work(&self, job: &JobId, pid: u32) {
        self.held_drones().arrived(job, pid);
    }

    /// The Drone on this Job has gone. Called wherever the record's own
    /// `assigned_drone` is cleared, so the index and the record go together.
    pub(crate) fn drone_gone(&self, job: &JobId) {
        self.held_drones().left(job);
    }

    fn held_drones(&self) -> std::sync::MutexGuard<'_, Drones> {
        self.drones
            .lock()
            .expect("the Drone index is not held across a panic")
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
    pub(crate) fn liveness(&self) -> Liveness {
        self.liveness
    }
    pub(crate) fn dry_runs(&self) -> DryRuns {
        self.dry_runs
    }

    /// What the gate needs in order to ask the Judge.
    ///
    /// The environment is the Drone's own list, built the same way and by the
    /// same function — a Judge call needs the credential floor for the reason a
    /// Drone does, and a second list here would be a second answer.
    ///
    /// **The Job is a parameter for one reason**: a call that is out has to be
    /// nameable while it is out, and a wait that cannot say whose it is is a
    /// fact no surface can place. Nothing else here reads it — a Judge call is
    /// still assembled from the step and the workflow, and there is no
    /// arrangement of this argument that could reach the Judge.
    pub(crate) fn judging(&self, job: &JobId) -> Result<Judging, SpawnConfigRefused> {
        Ok(Judging {
            client: Arc::clone(&self.judge),
            budget: self.judge_budget,
            default_model: self.judge_model.clone(),
            environment: environment(HostPaths {
                path: &self.host.path,
                user: &self.host.user,
                home: &self.host.home,
            })?,
            marking: Marking::on(
                job.into(),
                self.aloft.clone(),
                self.events.clone(),
                Arc::clone(&self.clock),
                self.judge_budget,
            ),
        })
    }
    /// The Judge call that is out, for `serving` to put on `get_job`.
    ///
    /// **Read, never written, from here.** The only writer is the guard in
    /// `crate::judging`, which puts the mark up before a call and takes it down
    /// however the call ends.
    pub(crate) fn aloft(&self) -> &Aloft {
        &self.aloft
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
    /// The roster, for `dispatch` and for a turn.
    pub(crate) fn slots(&self) -> &Mutex<Slots> {
        &self.slots
    }

    /// The three dials the headroom half of admission reads. All of them are
    /// `crate::admitting`'s, which is the only caller.
    pub(crate) fn machine(&self) -> &Arc<dyn Machine> {
        &self.machine
    }
    pub(crate) fn headroom(&self) -> &Headroom {
        &self.headroom
    }
    pub(crate) fn polling(&self) -> Polling {
        self.polling
    }
    pub(crate) fn polled(&self) -> &Mutex<Option<Polled>> {
        &self.polled
    }

    /// One Job's working slot, for the three Drone tools and for every act a
    /// person takes on a named Job. `None` is a Job with no Drone.
    ///
    /// Everything a Drone tool binds to is inside the slot, and taking that
    /// lock once is what makes which-Job, which-step and which-type one
    /// decision rather than three reads a turn can interleave with. **Which
    /// slot is `crate::peer`'s answer** — the Drone does not name it.
    pub(crate) async fn slot_of(&self, job: &JobId) -> Option<Slot> {
        self.slots.lock().await.slot_of(job)
    }

    /// One Job's slot, made if it has none.
    ///
    /// **For the acts a person takes on a Job Fleet is not holding**, which
    /// need somewhere to stand a Drone down or to land work through and have no
    /// entry in the roster to do it in. None of them starts a Drone: since #50
    /// only admission does, so an empty slot opened here holds nothing and
    /// `Slots::sweep` forgets it when the caller lets it go.
    pub(crate) async fn slot_for(&self, job: &JobId) -> Slot {
        self.slots.lock().await.opened_for(job)
    }

    /// The one working slot, for a fixture bounded at one.
    ///
    /// **A test's convenience and nothing else**, which is why it is `cfg(test)`
    /// rather than a method a surface could reach: with the bound above one
    /// there is no such thing as *the* slot, and asking for it is the question
    /// `#50` exists to stop anybody asking. A Fleet working nothing answers with
    /// an empty slot, so a caller reads "nothing is working" where it used to.
    #[cfg(test)]
    pub(crate) async fn the_only_slot(&self) -> Slot {
        let mut slots = self.slots.lock().await;
        match slots.working_on().first() {
            Some(job) => slots.opened_for(job),
            None => Arc::new(Mutex::new(None)),
        }
    }

    /// The rebase-and-push tail's lock. See the field.
    pub(crate) fn merge_end(&self) -> &Mutex<()> {
        &self.merge_end
    }
    pub(crate) fn inbox(&self) -> &EvidenceInbox {
        &self.inbox
    }
    /// Leave what a Job's branch came to for the turn that reports it.
    pub(crate) async fn left_delivered(&self, job: &JobId, delivered: Delivered) {
        self.delivered.lock().await.insert(job.clone(), delivered);
    }

    /// Take what this Job's branch came to, where anything is waiting.
    /// **Drained, not read**, so a second turn cannot report a push once.
    pub(crate) async fn take_delivered(&self, job: &JobId) -> Option<Delivered> {
        self.delivered.lock().await.remove(job)
    }
}
