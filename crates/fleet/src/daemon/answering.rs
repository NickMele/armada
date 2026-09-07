//! What Fleet answers when it is asked: the boot read, the two acts a person
//! takes on a Job, and the reads every surface makes between them.
//!
//! **Two acts, and both of them a person's.** A Job is proposed and a Job is
//! approved. Everything else that moves a Job moves it from inside — from a
//! turn, a Check, or a Drone's own submission — through the modules around this
//! one, so the actor recorded here is `human` and Fleet is not allowed to be
//! recorded as having approved anything.
//!
//! **A read never quietly shortens.** [`Fleet::every_job`] answers with the rows
//! that would not load beside the ones that did: a caller handed a short list
//! with nothing in the signature saying so cannot tell it from a complete one.
//!
//! **Who is calling is a read too, and it asks the transport.** The Drone index
//! is kept here beside [`Fleet::caller_of`], which is its only reason to exist —
//! `crate::peer` places a call by the port pair it arrived on, and this is what
//! it places it against.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, IllegalTransition, Job, JobId, JobStatus, StepId, Target, TransitionReason,
};
use store::{LoadAllError, LoadJobError, Loaded, Moved};

use super::Fleet;
use crate::adrift::Adrift;
use crate::drafting::StatedBy;
use crate::drone::{aftermath, Aftermath, Ending};
use crate::drone_moves::steps_holding_a_drone;
use crate::peer::{attributed, Drones, NotACaller};
use crate::readopting::Recovered;
use crate::reconciled::Reconciled;

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
    /// **The boot read, and the reconciliation.** Nothing runs until this has.
    ///
    /// A Job the store says was `running` **is asked about** rather than
    /// assumed dead. `libc::setsid()` at every spawn is what lets a Drone
    /// outlive the Fleet that started it, so a step whose pointer is still set
    /// may name a process that is still working — and this used to state
    /// `Ending::Vanished` about all of them, because a pid lived only in memory
    /// and there was nothing to ask with. `crate::readopting` is the asking and
    /// `#61` is the subject.
    ///
    /// Where the process is gone, the answer is what it always was: `escalated`,
    /// reason `interrupted`, through `crate::aftermath` rather than restated
    /// here.
    ///
    /// **Never resumed silently, which is a stronger claim than it looks.** A
    /// Drone that is adopted is put back in a slot and the Job carries on, and
    /// the record says so twice over: a line in the Job's log naming the pid,
    /// and a row in the Drone's own transcript saying how long nothing was read
    /// and that nothing will be read from here.
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
            // **Asked before anything is recorded**, because the departure is
            // what clears the pointer and the stored pid, and a Drone that is
            // still there has not departed. A Job that is adopted keeps its
            // pointer, its process row and its status.
            if self.recovered(&job).await? == Recovered::Adopted {
                reconciled.adopted.push(job.id().clone());
                continue;
            }
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

    /// Release a Job to spawn. **It queues; it does not dispatch.**
    ///
    /// The transition is `awaiting_approval -> queued` and the actor is
    /// **human**: this is the primary autonomy control, and Fleet is not
    /// allowed to be recorded as the one that took it.
    ///
    /// **It answers `queued` and no longer `running`, which is `#428`.** This
    /// ran `admit_next` inline, so the whole of [`crate::dispatch`] ran inside
    /// the request that approved the Job — and a client that stopped waiting
    /// took the cold install and the timeout watching it away together.
    /// [`Fleet::admit_next`] holds the rule that came out of it. The wire's
    /// shape is unchanged and the move to `running` follows on the turn that
    /// dispatches, within one `PROVISIONAL_TURN_INTERVAL`.
    pub async fn approve(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        // **The gate is `awaiting_approval` and nothing else, said here rather
        // than left to the machine.** Three other statuses have an edge to
        // `queued` and none of them is an approval: `awaiting_review` and
        // `escalated` are a person's act at the far end of a Job, and
        // `running -> queued` is Fleet standing a dispatching Job's Drone down
        // to wait for its children. Approving any of them would re-queue a Job
        // that is already past the gate — and for the `running` one, strand a
        // live Drone. The machine refused two of the four for as long as their
        // edges did not exist, which is what this replaces.
        if job.status() != JobStatus::AwaitingApproval {
            return Err(Adrift::IllegalMove(IllegalTransition::NoSuchEdge {
                from: job.status(),
                to: JobStatus::Queued,
            }));
        }
        self.move_job(&job, Target::Queued, Actor::Human).await
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
}
