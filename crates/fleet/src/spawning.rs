//! Putting a Drone on a worktree: the config, the transcript, the process, the
//! slot.
//!
//! Split from [`dispatch`](mod@crate::dispatch) because two things do it. A
//! dispatch makes the worktree first and a restart finds one already there,
//! and everything after that point is identical — so it is written once, here,
//! and the brief is the parameter that differs.
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
use core_model::{DroneId, Job, JobId, StepId};

use crate::adrift::Adrift;
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
    pub(crate) async fn put_a_drone_on(
        &self,
        job: &Job,
        step: &StepId,
        worktree: Worktree,
        brief: Prompt,
        working: &mut Option<Working>,
    ) -> Result<(), Adrift> {
        let job_id = job.id().clone();
        let config = match self.spawn_config(job, &worktree, brief) {
            Ok(config) => config,
            Err(cause) => {
                self.interrupt(job).await?;
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
                self.interrupt(job).await?;
                return Err(Adrift::NoTranscript { job: job_id, cause });
            }
        };
        let started = match drone::start(self.harness().as_ref(), &config).await {
            Ok(started) => started,
            Err(cause) => {
                self.interrupt(job).await?;
                return Err(Adrift::NoDrone {
                    job: job_id,
                    cause: Box::new(cause),
                });
            }
        };
        // After the process exists, never before: `assigned_drone` is presence,
        // and a Job claiming a Drone that failed to start is exactly the
        // liveness lie the column is read for.
        self.drone_arrived(job, drone.clone()).await?;

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
        Ok(())
    }

    /// Open this Drone's transcript, and name it in the Job's log.
    ///
    /// The log line is still written: it carries the transcript's path, which
    /// `assigned_drone` does not — the column names the Drone and this names
    /// the file its rows are in.
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
    fn spawn_config(
        &self,
        job: &Job,
        worktree: &Worktree,
        brief: Prompt,
    ) -> Result<DroneSpawnConfig, SpawnConfigRefused> {
        Ok(DroneSpawnConfig::spawn_in(
            worktree,
            Model::named(job.model().as_str())?,
            brief,
            McpConfig::only_these(&self.host().mcp_config)?,
            self.toolbelt(),
            environment(HostPaths {
                path: &self.host().path,
                home: &self.host().home,
                user: &self.host().user,
            })?,
        ))
    }

    /// What the Drone may call: the Evidence tool, its own worktree, and each
    /// **non-destructive** command the Manifest declares.
    ///
    /// A destructive command is withheld, and that is a decision this file
    /// makes rather than one it inherits: `commands.<name>.destructive` is a
    /// key `config` reads at M1 and nothing consumed until now, and granting
    /// one to an unattended process is the opposite of what the flag is for.
    fn toolbelt(&self) -> Toolbelt {
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
        belt
    }
}
