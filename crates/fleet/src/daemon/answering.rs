//! What Fleet answers when it is asked: the boot read, the two acts a person
//! takes on a Job, and the reads every surface makes between them.
//!
//! **Two acts.** A Job is proposed and a Job is approved. Everything else that
//! moves a Job moves it from inside — from a turn, a Check, or a Drone's own
//! submission — through the modules around this one. **Approval stays a
//! person's alone** — the primary autonomy control, and Fleet is not allowed
//! to be recorded as having approved anything. **Proposal is a person's or
//! Helm's**, `#943`: the door lets a Helm session draft a Job too, and
//! [`Fleet::propose`] records which asked.
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
    Actor, IllegalTransition, Job, JobId, JobStatus, ProposalId, StepId, Target, TransitionReason,
};
use store::{LoadAllError, LoadJobError, Loaded, Moved};

use super::Fleet;
use crate::adrift::Adrift;
use crate::drafting::StatedBy;
use crate::drone::{aftermath, Aftermath, Ending};
use crate::drone_moves::steps_holding_a_drone;
use crate::peer::{attributed, held_within, Drones, NotACaller};
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
        // First of all: the servers a crashed Fleet left running are ended, so
        // the ports they held are free before the probe below. See
        // `crate::servers::left`.
        for repository in self.repositories().every() {
            self.reaped_left_servers(repository.records_root()).await;
        }
        // Before anything else: a main-checkout claim a crashed Fleet left
        // behind is re-probed before this process trusts it. See
        // `crate::ports::Fleet::reconciled_main_checkout_ports`.
        self.reconciled_main_checkout_ports().await;
        // A scout does not outlive the Fleet reading it. `crate::scouting`.
        self.scouts_left_gathering().await;
        // Before any Studio is read: the Links this build reads as an Issue, a
        // Pull request or an Epic. `crate::recognising`.
        let recognised = self.links_recognised().await;
        let (loaded, unreadable) = self.every_job().await?;
        // Before anything else reads a path: every Job's name, and the rename
        // of what an older Fleet wrote under a ULID. See
        // [`mod@crate::naming`] and `crate::transcript::migrating`.
        self.names().learn_all(&loaded.jobs);
        let mut reconciled = Reconciled {
            repaired: loaded.repaired.len(),
            unreadable,
            recognised,
            ..Reconciled::default()
        };
        // **Each repository over its own Jobs.** A Job whose repository is not
        // served is left as it stands, and reconciled when that one is added.
        for served in self.repositories().served() {
            self.reconciled_jobs(&served, &loaded.jobs, &mut reconciled)
                .await?;
        }
        reconciled.admitted = self.admit_next().await?;
        Ok(reconciled)
    }

    /// The same cleanup for one repository added after Fleet started: the
    /// servers it left and the Jobs the store already holds for it.
    pub(crate) async fn reconciled_in(
        &self,
        served: &crate::repositories::Served,
    ) -> Result<Reconciled, Adrift> {
        self.reaped_left_servers(served.records_root()).await;
        let (loaded, _) = self.every_job().await?;
        self.names().learn_all(&loaded.jobs);
        let mut reconciled = Reconciled::default();
        self.reconciled_jobs(served, &loaded.jobs, &mut reconciled)
            .await?;
        reconciled.admitted = self.admit_next().await?;
        Ok(reconciled)
    }

    /// One repository's Jobs, out of every Job the boot read found.
    async fn reconciled_jobs(
        &self,
        served: &crate::repositories::Served,
        every: &[Job],
        reconciled: &mut Reconciled,
    ) -> Result<(), Adrift> {
        let mut jobs: Vec<Job> = every
            .iter()
            .filter(|job| served.owns(job.owner_manifest_id().as_str()))
            .cloned()
            .collect();
        let rekeyed = crate::transcript::rekeyed(served.records_root(), &jobs).await;
        reconciled.rekeyed.logs += rekeyed.logs;
        reconciled.rekeyed.transcripts += rekeyed.transcripts;
        reconciled.rekeyed.refused.extend(rekeyed.refused);
        // Before the Drone-adoption pass below reads these same rows: see
        // `crate::mending`'s own header for why the order matters.
        reconciled
            .mended
            .extend(self.mended_wedged_reviews(&mut jobs).await?);
        // Before the Drone-adoption pass below, for the same reason: a
        // ruling here can move a Job or leave a stale Drone pointer, and the
        // pass right after has to see the result. #796.
        reconciled
            .recovered_evidence
            .extend(self.ruled_on_pending_evidence(&mut jobs).await?);
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
        let held: Vec<(Job, Vec<StepId>)> = jobs
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
            // A Job already `escalated` — `stalled` over a Drone this Fleet
            // has just found gone — reads `Aftermath::AlreadyStopped` below
            // and takes neither arm: the pointer is cleared above and the
            // step stays `running`. `#1034`: a person's restart is what stops
            // it, under `drone_gone`, once `Stuck::of` offers one.
            if let Aftermath::JobMoves(target) =
                aftermath(job.status(), &Ending::Vanished, self.left(job.id()))
            {
                // **The step, then the Job — `Fleet::kill_drone`'s order.** A
                // restart that finds the Drone gone used to move the Job
                // straight to `escalated` and leave its step `running`
                // underneath, which is `#792`: the inner machine freezes the
                // moment the Job leaves `running`, so the step has to stop
                // first or not at all.
                let job = self.stopped_at_rest(&job).await?;
                self.move_job(&job, target, Actor::Fleet).await?;
                reconciled.interrupted.push(job.id().clone());
            }
        }
        Ok(())
    }

    /// A Job drafted onto the approval gate. **The gate is unchanged** — what
    /// comes back is at `awaiting_approval`, not a running Job.
    /// **And it publishes.** A Job created while a client was connected never
    /// reached it: `ipc::Event` carried one kind, `job.state_changed`, and
    /// creating a Job is not a state change — so nothing was published and
    /// nothing woke Bridge. `job.created` is the kind that says a row appeared,
    /// and it carries the row whole so a Board inserts it rather than re-reading.
    ///
    /// **A person's**, and every one of the hundreds of fixtures across this
    /// crate that call it directly is a person's too — `propose_as`'s doc
    /// carries the reason this stays the one-argument call the whole test
    /// suite already spells rather than growing a parameter nothing in it
    /// wants.
    pub async fn propose(&self, proposal: ipc::ProposeJob) -> Result<Job, Adrift> {
        self.propose_as(proposal, api::Redirector::Person).await
    }

    /// [`Fleet::propose`], naming who asked. **`Commands::propose_job`'s own
    /// call** — the door lets a Helm session draft a Job too (`#941`'s reach),
    /// and what `job.created` and entry zero's `approved_by` record has to
    /// say so rather than assume a person. `#943`.
    ///
    /// A second method rather than a parameter on `propose`, because every
    /// caller inside this crate but this one is a fixture proposing as a
    /// person, and a parameter the whole suite would have to spell `Person`
    /// at every call is a parameter nothing there wants.
    pub async fn propose_as(
        &self,
        proposal: ipc::ProposeJob,
        by: api::Redirector,
    ) -> Result<Job, Adrift> {
        // Hand entry, which is the override rather than the path. Entry zero
        // records that the caller stated this scope and not the call, which
        // is what makes the call evaluable against the decisions people made.
        // A person or a Helm session drafting a Job by hand read nothing and
        // split nothing.
        let stated = match by {
            api::Redirector::Person => StatedBy::APerson,
            api::Redirector::Helm => StatedBy::AHelmSession,
        };
        let actor = match by {
            api::Redirector::Person => Actor::Human,
            api::Redirector::Helm => Actor::Helm,
        };
        self.proposed_job(proposal, stated, None, actor).await
    }

    /// The same creation, with who stated the scope carried through to entry
    /// zero. **The only difference between the two dispatch paths**, which is
    /// why they share everything below it.
    /// `minted_by` is the reading this Job came out of, where a proposer read
    /// one request. **A parameter and not a field of `ProposeJob`**: the wire
    /// shape is what a caller drafts, and a caller claiming membership of a
    /// reading it did not make would be claiming a sibling it does not have.
    /// Fleet mints the id and Fleet is the only thing that may write it.
    ///
    /// `by` is who `job.created` is published against — the caller, never
    /// `stated`: the proposer path states `TheProposer` regardless of who
    /// asked it to read a request, because the scope decision is Fleet's
    /// proposer model's either way, but the Job it minted is still that
    /// caller's draft. `#943`.
    pub(crate) async fn proposed_job(
        &self,
        proposal: ipc::ProposeJob,
        stated: StatedBy,
        minted_by: Option<ProposalId>,
        by: Actor,
    ) -> Result<Job, Adrift> {
        let at = self.now();
        // Before `drafted`, which is sync and cannot read the board: an edge is
        // a pointer, and a peer that does not exist is the one shape a cycle
        // needs. `coupling::peers_held` carries why.
        self.peers_held(&proposal.dependencies).await?;
        // **Allocated and written under one lock**, which is what makes the
        // number an allocation rather than a guess: two proposals reading
        // before either wrote would be handed the same number, and the unique
        // index would refuse the second. Holding the lock across both is what
        // stops that happening at all.
        let mut store = self.store.lock().await;
        let number = store
            .next_job_number(
                self.the_repository_named(&proposal.owner_manifest_id)?
                    .manifest()
                    .id(),
            )
            .map_err(Adrift::Reading)?;
        let (new, origin) = self.drafted(proposal, stated, &at, minted_by, number)?;
        let job = Job::create_top_level(new, origin, at.clone());
        store.insert_job(&job, &at).map_err(Adrift::Writing)?;
        self.learn_the_name(&job);
        self.manifest_snapshotted(&mut store, &job).await;
        drop(store);
        // After the write, never before: a client told about a row the store
        // then refused would hold a Job that does not exist, and a resync would
        // silently remove it.
        self.publish(ipc::Event::JobCreated(ipc::JobCreated {
            job: ipc::JobSummary::from(&job),
            actor: by.into(),
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
    ///
    /// **Every read takes the Job's name**, which is what keeps
    /// [`mod@crate::naming`] total for a Job created by a Fleet that has since
    /// been restarted with an older store.
    pub async fn load(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let job = self
            .store
            .lock()
            .await
            .load_job(job_id)
            .map_err(Adrift::Reading)?;
        self.learn_the_name(&job);
        Ok(job)
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

    /// The instant this Job first reached `running` — [`JobSummary`](ipc::JobSummary)'s
    /// `started_at`. Absent until the Job's first Drone starts, and unmoved by
    /// a later return to `awaiting_approval` or `queued`: `crate::wire::job_started_at`
    /// keeps only the first arrival.
    pub async fn job_started_at(
        &self,
        job_id: &JobId,
    ) -> Result<Option<core_model::Timestamp>, Adrift> {
        let store = self.store.lock().await;
        crate::wire::job_started_at(&store, job_id).map_err(Adrift::Reading)
    }

    /// The instant this Job arrived at a terminal status —
    /// [`JobSummary`](ipc::JobSummary)'s `ended_at`. Absent until the Job is
    /// over. Overview 28 (#1092).
    pub async fn job_ended_at(
        &self,
        job_id: &JobId,
    ) -> Result<Option<core_model::Timestamp>, Adrift> {
        let store = self.store.lock().await;
        crate::wire::job_ended_at(&store, job_id).map_err(Adrift::Reading)
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

    /// Whether a Helm session this Fleet is hosting holds this call's
    /// connection: [`caller_of`](Fleet::caller_of)'s port pair, matched across
    /// every process a Helm host is running and what each started. `#941`.
    pub(crate) fn helm_holds(&self, caller: &api::Caller) -> bool {
        held_within(
            caller,
            self.host.port,
            &self.helm.host().running(),
            self.peers.as_ref(),
        )
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
