//! Putting a Drone on a worktree: the config, the transcript, the process, the
//! slot.
//!
//! Split from [`dispatch`](mod@crate::dispatch) because three things do it. A
//! dispatch makes the worktree first, a restart finds one already there, and an
//! override onto a Drone that has gone finds one too — and everything after
//! that point is identical, so it is written once, here, and
//! [`Opening`](crate::briefing::Opening) is the parameter that differs.
//!
//! # Every spawn catches the branch up, and this is the one place it happens
//!
//! `crate::delivery` carries the rule. A spawn has no session to inject a turn
//! into, so the rebase runs here and what it came to rides the opening brief —
//! which is why the brief is assembled inside
//! [`put_a_drone_on`](Fleet::put_a_drone_on) rather than handed to it. **A
//! conflict is the Drone's opening work and not a refusal**: refusing the
//! restart instead puts a person at a merge conflict inside a Drone's worktree,
//! which is the one job the Drone is already in the right place to do.
//!
//! # Nothing is inherited from the Drone that went before
//!
//! `docs/concepts/drone.md`: permissions intersect on a respawn, so a widened
//! scope can only produce a narrower toolset than the Drone that asked for it.
//! Every value below is resolved from the Manifest and the Job as they stand
//! now, which is what makes that true by construction rather than by care.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, DroneSpawnConfig, Grant, McpConfig, Model, Prompt, SpawnConfigRefused,
    Toolbelt, Vcs, WorkProduct, Worktree,
};
use core_model::{Component, DroneId, Envelope, EscalationTrigger, Job, JobId, Level, StepId};

use crate::adrift::Adrift;
use crate::briefing::Opening;
use crate::crossing::Redirected;
use crate::daemon::Fleet;
use crate::drone::{self, environment, HostPaths};
use crate::transcript::{Spine, Taps};
use crate::working::Working;

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
    /// Start a Drone against a prepared worktree and take the slot.
    ///
    /// **The Job is already `running` and the step is already `running` when
    /// this is called.** Both machines move before a process exists, because
    /// the inner one is frozen beneath every status but `running` and a step
    /// cannot be entered from underneath a Job that has not moved.
    ///
    /// Every failure leaves the Job `escalated` and returns the cause. A person
    /// decides; Fleet does not retry.
    ///
    /// **A note waiting on the record rides the brief.** `#207`, folded in
    /// here rather than by whichever act reached the spawn: this is the one
    /// funnel, and a caller told to remember it could forget.
    ///
    /// **The branch is caught up first, and the brief is assembled after**, for
    /// the reason this module's header gives. A catch-up that will not run
    /// stops the Job with `no_worktree`: a Drone put on a tree Fleet could not
    /// reconcile starts from a state nobody has read. A rebase that ran and
    /// *conflicted* is not that — it is an answer, and it goes into the brief.
    ///
    /// **Each failure below names who fixes it and none says `interrupted`.**
    /// All five did until 2026-08-31, on Jobs that had never spawned a process
    /// for anyone to find: `no_worktree` for the catch-up, `not_configurable`
    /// for the brief and the spawn config, and `would_not_start` for the
    /// transcript and the harness.
    pub(crate) async fn put_a_drone_on(
        &self,
        job: &Job,
        step: &StepId,
        worktree: Worktree,
        opening: Opening,
        working: &mut Option<Working>,
    ) -> Result<(), Adrift> {
        let job_id = job.id().clone();
        let moved = match self.caught_up_onto(&job_id, &worktree).await {
            Ok(moved) => moved,
            Err(cause) => {
                // `no_worktree`. The tree exists and git would not bring it up
                // to its base, which leaves the same fact behind as a tree
                // that was never made: there is nowhere a Drone can be started
                // that anybody has read the state of. Whoever owns the disk and
                // the repository fixes both.
                self.stopped_before_a_drone(job, EscalationTrigger::NoWorktree)
                    .await?;
                return Err(cause);
            }
        };
        // Asked of the record on every spawn, and answered `None` on almost
        // all of them. It is read before the brief because it is part of the
        // brief, and kept beside it because clearing it needs the same value.
        let waiting = job.redirect_waiting().map(Redirected::of);
        let opening = opening.also_carrying(waiting.clone());
        let brief = match opening.turn(job, job.workflow(), step, moved.as_ref()) {
            Ok(brief) => brief,
            Err(cause) => {
                // `not_configurable`, and so is the refusal below it. The brief
                // is rendered from the Manifest and the Job's own values, so a
                // brief that will not render is a line somebody wrote.
                self.stopped_before_a_drone(job, EscalationTrigger::NotConfigurable)
                    .await?;
                return Err(Adrift::NotConfigurable { job: job_id, cause });
            }
        };
        // Kept before the brief is consumed. **The one turn of Armada's that
        // was rendered and dropped**: `Prompt` goes into the process and the
        // process ends, so what a step opened with survived nowhere at all
        // until this row.
        let opened_with = brief.as_str().to_string();
        // Kept beside the text for the same reason, and it is the half no
        // reader can recover from the text: which lines are block headings.
        let headings = brief.headings().to_vec();
        let config = match self.spawn_config(job, step, &worktree, brief.prompt()) {
            Ok(config) => config,
            Err(cause) => {
                // A model name no roster row carries is the case this is
                // expected to catch, and `crates/config/src/roster.rs` says so.
                // It reported as `interrupted` until 2026-08-31, which
                // presented a typo in `armada.yml` as a crashed process.
                self.stopped_before_a_drone(job, EscalationTrigger::NotConfigurable)
                    .await?;
                return Err(Adrift::NotConfigurable { job: job_id, cause });
            }
        };
        // The record is opened **before** the Drone, so a disk that will not
        // hold it escalates the Job rather than losing a transcript quietly
        // once there is already a process producing one.
        //
        // A second `drone_id` under one `job_id` is what a restart mints, and
        // the log envelope already anticipates it — so the transcript of the
        // Drone that went before is untouched and this one opens beside it.
        let drone = DroneId::carried(self.mint().ulid());
        let recording = match self.recording(&job_id, &drone, step) {
            Ok(recording) => recording,
            Err(cause) => {
                // `would_not_start`. Everything resolved and the machine said
                // no — here the disk, below the harness. Nothing ran, so there
                // is no transcript to go through and the badge says as much.
                self.stopped_before_a_drone(job, EscalationTrigger::WouldNotStart)
                    .await?;
                return Err(Adrift::NoTranscript { job: job_id, cause });
            }
        };
        let started = match drone::start(self.harness().as_ref(), &config).await {
            Ok(started) => started,
            Err(cause) => {
                self.stopped_before_a_drone(job, EscalationTrigger::WouldNotStart)
                    .await?;
                return Err(Adrift::NoDrone {
                    job: job_id,
                    cause: Box::new(cause),
                });
            }
        };
        // After the process exists, never before: `assigned_drone` is presence,
        // and a step claiming a Drone that failed to start is exactly the
        // liveness lie the column is read for.
        //
        // The pid index goes in beside it, for `crate::peer`: the two are one
        // fact — a Drone is on this Job — recorded in the two places that can
        // answer for it, the record for a person and the index for a caller.
        self.drone_at_work(&job_id, started.session.pid());
        self.drone_arrived(job, step, drone.clone()).await?;
        // After the process exists too, and for the same reason: a note
        // cleared over a spawn that then failed is a note nobody was told.
        if waiting.is_some() {
            self.redirect_delivered(job, step).await?;
        }

        *working = Some(Working::holding(
            job_id,
            drone,
            step.clone(),
            worktree,
            started,
            Arc::clone(self.harness()),
            recording,
            self.now(),
        ));
        // The first row of this step's record, written by Armada, before the
        // Drone has said anything. It is written after the slot exists rather
        // than before because the sinks live on it.
        if let Some(at_work) = working.as_ref() {
            at_work.briefed(&opened_with, headings);
        }
        // This step's baseline, read once the slot exists. A Job's first step
        // ordinarily starts on a worktree holding nothing, and reading it
        // rather than assuming so is what makes a redispatch onto a worktree
        // somebody already worked in measure the step and not the branch.
        //
        // **After the catch-up above, which is #131's ordering arriving on the
        // spawn path.** A rebase writes content — a clean one replays the
        // branch onto a base that moved, a conflicted one leaves markers — and
        // a baseline read before it would credit this step with git's output.
        // On a restart that matters twice over: the step is being re-run, so
        // `diff_nonempty` is asking whether *this* attempt wrote something, and
        // a Drone that resolved nothing would pass it on the markers it was
        // handed.
        self.marked(working);
        Ok(())
    }

    /// The note went into the brief this Drone opened with, so nothing is
    /// waiting any more.
    ///
    /// **The log says *delivered*, not that one was written.** That is the
    /// owner's second consequence: a person who left a note at a gate can tell
    /// "you were told" from "nobody was there", and the two lines are written
    /// by two different acts — `request_changes` writes the first and this
    /// writes the second.
    ///
    /// **Cleared once the Drone exists and not before.** The ruling is that a
    /// note survives one boundary and no more; a spawn that failed is not a
    /// boundary that happened, and clearing on the way in would lose the note
    /// to a worktree that would not open.
    ///
    /// A log line that will not write does not undo the delivery, for
    /// `resume::noted_roused`'s reason. The column write does return: a note
    /// that stayed on the record would be delivered a second time, into a
    /// Drone working a part it was never about.
    async fn redirect_delivered(&self, job: &Job, step: &StepId) -> Result<(), Adrift> {
        let delivered = job.redirect_delivered();
        self.store()
            .lock()
            .await
            .record_redirect_waiting(&delivered)
            .map_err(Adrift::Writing)?;
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "the note a person left at the gate was delivered into this Drone's opening brief",
        )
        .in_job(job.id().as_ulid().clone())
        .at_step(step.as_str());
        let _ = crate::transcript::note(&self.host().repo_root, job.id(), &envelope);
        Ok(())
    }

    /// Open this Drone's transcript, and name it in the Job's log.
    ///
    /// The log line is still written: it carries the transcript's path, which
    /// `assigned_drone` does not — the step's column names the Drone and this
    /// names the file its rows are in.
    fn recording(
        &self,
        job: &JobId,
        drone: &DroneId,
        step: &StepId,
    ) -> Result<Taps, std::io::Error> {
        Taps::opening(
            &self.host().repo_root,
            Spine {
                job: job.clone(),
                drone: drone.clone(),
                step: step.clone(),
                run: self.run().clone(),
            },
            Arc::clone(self.clock()),
            self.turns().feeding(&ipc::JobId::from(job)),
        )
    }

    /// Everything one Drone is started with, from the Job and the machine.
    ///
    /// The brief is a parameter rather than assembled here: a Drone starting a
    /// step and one taking a stopped step over are told different things, and
    /// only the caller knows which this is.
    ///
    /// **The model is the step's, not the Job's.** It could not be until a step
    /// was its own process — one session cannot change model partway — and now
    /// that it is, a step that only runs a suite and reports stops paying for
    /// one that reasons. `Job::model_at` is where absent falls back to the
    /// Job's, and this asks it rather than deciding for itself.
    fn spawn_config(
        &self,
        job: &Job,
        step: &StepId,
        worktree: &Worktree,
        brief: Prompt,
    ) -> Result<DroneSpawnConfig, SpawnConfigRefused> {
        Ok(DroneSpawnConfig::spawn_in(
            worktree,
            Model::named(job.model_at(step).as_str())?,
            brief,
            McpConfig::only_these(&self.host().mcp_config)?,
            self.toolbelt(job, step),
            environment(HostPaths {
                path: &self.host().path,
                home: &self.host().home,
                user: &self.host().user,
            })?,
        ))
    }

    /// What the Drone may call: the Evidence tool, its own worktree, each
    /// **non-destructive** command the Manifest declares, and — on one step of
    /// one workflow — the dispatch tool.
    ///
    /// A destructive command is withheld, and that is a decision this file
    /// makes rather than one it inherits: `commands.<name>.destructive` is a
    /// key `config` reads at M1 and nothing consumed until now, and granting
    /// one to an unattended process is the opposite of what the flag is for.
    ///
    /// # Why this takes the step, when it used to take nothing
    ///
    /// **The dispatch grant is per step and cannot be anything else.** The
    /// capability it carries is *other Jobs existing*, and what authorises that
    /// is a person having read the plan the step before it produced. A Job-wide
    /// grant would put the tool in the hands of the Drone writing the plan,
    /// which is the one Drone that must not have it.
    ///
    /// `Dispatching::at` is the same predicate the tool call itself is refused
    /// by, so a Drone's allowlist and Fleet's answer cannot disagree — and both
    /// read the frozen workflow, which is what a person approved.
    fn toolbelt(&self, job: &Job, step: &StepId) -> Toolbelt {
        let mut belt = Toolbelt::evidence_only()
            .and(Grant::ReadTheWorktree)
            .and(Grant::ChangeTheWorktree);
        for name in self.manifest().command_names() {
            match self.manifest().command(&name) {
                Some(command) if !command.is_destructive() => {
                    belt = belt.and(Grant::RunADeclaredCommand(command.run().to_string()));
                }
                _ => {}
            }
        }
        // **Read off the step Fleet is about to put a Drone on, not off the
        // Job's current step.** They are the same on every path that reaches
        // here, and asking the one being spawned onto is what keeps that true
        // if a path ever arrives where they are not.
        if dispatches(job, step) {
            belt = belt.and(Grant::DispatchAJob);
        }
        belt
    }
}

/// Whether a Drone spawned on this step of this Job may create Jobs.
///
/// A free function so that the answer is one expression read in two places —
/// here, where the allowlist is rendered, and `crate::sub_dispatch`, where a
/// call of the tool is refused. It is the same shape `Dispatching::at` asks,
/// minus the parent it borrows, because a toolbelt is built before there is a
/// call to authorise.
fn dispatches(job: &Job, step: &StepId) -> bool {
    job.origin().top_level().is_some()
        && job
            .workflow()
            .step(step)
            .is_some_and(core_model::ResolvedStep::may_dispatch_jobs)
}
