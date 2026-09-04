//! A Drone arriving on a step and leaving it: the column, the log row, and the
//! two events Bridge folds.
//!
//! Kept out of [`dispatch`](mod@crate::dispatch) because it is a different
//! subject — that file is what happens to a Job in the slot, and this is the
//! one field whose content is whether a process exists.
//!
//! # Arriving answers, and leaving answers about the fault and not the fact
//!
//! A spawn that cannot be recorded fails the dispatch: `assigned_drone` is what
//! the liveness clock and every surface read to know a process exists, and a
//! Drone running against a record that does not say so is the lie this event
//! was added to remove.
//!
//! A departure used to be swallowed whole, on the grounds that every caller is
//! already ending the Drone for a reason of its own. Two of the three things it
//! swallowed were not that: a store that would not take the write, and — now
//! that the pointer is per step — a caller naming a step the Job does not have.
//! Both are faults and both now return. **A step that holds no Drone is still
//! not one**: `kill_drone` reaches it on a Job whose Drone has already gone,
//! and it answers `Ok`.
//!
//! #140 is what needs that distinction. It records an exit and then spawns onto
//! the next step, and a spawn refuses over a live pointer — so an exit that
//! silently failed would become a boundary that could not put a Drone on it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, DroneAssigned, DroneId, Envelope, FieldValue, IllegalDroneMove, Job, JobId,
    Level, StepId,
};
use store::DroneProcess;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::process::{holder_of, Holder};

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
    /// Record that a Drone is on a step of the Job, and say so on the stream.
    pub(crate) async fn drone_arrived(
        &self,
        job: &Job,
        step: &StepId,
        drone: DroneId,
    ) -> Result<(), Adrift> {
        let moved = job
            .drone_spawned(step, drone, Actor::Fleet, self.now())
            .map_err(Adrift::IllegalDroneMove)?;
        self.record_drone(&moved).await?;
        self.publish(ipc::Event::DroneSpawned(ipc::DroneSpawned::of(
            &moved.event,
            ipc::JobSummary::from(&moved.job),
            moved.job.branch().map(|branch| branch.as_str().to_string()),
        )));
        Ok(())
    }

    /// Record the operating-system process the Drone is running as.
    ///
    /// **The third place a spawn is written down, and the only one a restart
    /// can read.** The step's `assigned_drone` says a Drone exists and
    /// `crate::peer`'s index says which pid it is; both are needed while Fleet
    /// is up, and only this survives Fleet going away. `crate::adopting` is the
    /// reader.
    ///
    /// **A pid and when it started, because a pid alone is not an identity.**
    /// The reading is taken here, once, immediately after the process exists —
    /// not at the adoption, where the answer would be whatever holds the number
    /// by then.
    ///
    /// **A probe that will not run does not fail the spawn.** What is lost is
    /// the ability to adopt this Drone if Fleet restarts, which is a recovery
    /// Fleet did not have at all until now; failing a live dispatch over it
    /// would trade a working Job for a hypothetical one. The Job's log says so,
    /// because a Drone that silently cannot be adopted is exactly the kind of
    /// absence nobody finds later.
    pub(crate) async fn drone_process_recorded(
        &self,
        job: &Job,
        step: &StepId,
        drone: &DroneId,
        pid: u32,
    ) -> Result<(), Adrift> {
        let started_at = match holder_of(pid) {
            Ok(Holder::Held(at)) => at,
            // Held by nothing, a heartbeat after the spawn returned. The
            // process is already gone and the reap will say so; there is
            // nothing to adopt and nothing to write.
            Ok(Holder::Vacant) => {
                self.noted_unadoptable(job, step, pid, "the process was already gone");
                return Ok(());
            }
            Err(probe) => {
                self.noted_unadoptable(job, step, pid, &probe.to_string());
                return Ok(());
            }
        };
        self.store()
            .lock()
            .await
            .record_drone_process(&DroneProcess {
                job_id: job.id().clone(),
                step_id: step.clone(),
                drone_id: drone.clone(),
                pid,
                started_at: started_at.as_str().to_string(),
                spawned_at: self.now(),
            })
            .map_err(Adrift::Writing)?;
        Ok(())
    }

    /// This Drone cannot be adopted if Fleet restarts, and here is why.
    ///
    /// **Fields rather than an interpolated sentence**, so a query finds every
    /// Drone that started without one rather than a person grepping prose.
    fn noted_unadoptable(&self, job: &Job, step: &StepId, pid: u32, because: &str) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the Drone started and its process could not be recorded, so a Fleet restart \
             will not be able to adopt it",
        )
        .in_job(job.id().as_ulid().clone())
        .at_step(step.as_str())
        .with_field("pid", FieldValue::Int(i64::from(pid)))
        .with_field("because", FieldValue::Str(because.to_string()));
        let _ = crate::transcript::note(&self.host().repo_root, job.id(), &envelope);
    }

    /// Record that the Drone on a step is gone.
    ///
    /// **A step holding no Drone is `Ok` and not a fault.** `kill_drone` on a
    /// Job whose Drone has already exited reaches it, and so does every caller
    /// that ends a Drone the settle path has already recorded the exit of.
    /// Everything else returns: a step the Job does not have is a caller's
    /// mistake, and a write that would not land is the record and the process
    /// list disagreeing, which is the fault this event exists to make
    /// impossible.
    pub(crate) async fn drone_left(&self, job_id: &JobId, step: &StepId) -> Result<(), Adrift> {
        // **First, and unconditionally.** A pid left in the index is a
        // connection that would still attribute to this Job, and every road out
        // of here — including the two that answer `Ok` without writing anything
        // — is a road where the Drone has gone.
        //
        // The stored process goes with it, for the same reason one step further
        // out: a row that outlived its Drone is a pid a later Fleet would probe
        // and adopt, and the number by then names whatever took it.
        self.drone_gone(job_id);
        self.store()
            .lock()
            .await
            .forget_drone_process(job_id)
            .map_err(Adrift::Writing)?;
        let job = self.load(job_id).await?;
        let moved = match job.drone_exited(step, Actor::Fleet, self.now()) {
            Ok(moved) => moved,
            Err(IllegalDroneMove::NoneAssigned { .. }) => return Ok(()),
            Err(refused) => return Err(Adrift::IllegalDroneMove(refused)),
        };
        self.record_drone(&moved).await?;
        self.publish(ipc::Event::DroneExited(ipc::DroneExited::of(
            &moved.event,
            ipc::JobSummary::from(&moved.job),
        )));
        Ok(())
    }

    async fn record_drone(&self, moved: &DroneAssigned) -> Result<(), Adrift> {
        self.store()
            .lock()
            .await
            .record_drone_move(moved)
            .map_err(Adrift::Writing)?;
        Ok(())
    }
}

/// Every step of a Job whose row still names a Drone.
///
/// **The ids and not the rows**, because every caller ends the Drones it finds
/// and each departure reloads the Job — a borrow of the record would not
/// survive the first one.
///
/// Ordinarily one. A Job holding two would be a Fleet that put a Drone on a
/// second step without ending the first, which nothing does; iterating is what
/// makes the record's answer the answer, rather than this file's assumption
/// about it.
pub(crate) fn steps_holding_a_drone(job: &Job) -> Vec<StepId> {
    job.steps_holding_a_drone()
        .map(|(step, _)| step.clone())
        .collect()
}
