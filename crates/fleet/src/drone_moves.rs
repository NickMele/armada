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
use core_model::{Actor, DroneAssigned, DroneId, IllegalDroneMove, Job, JobId, StepId};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

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
        self.drone_gone(job_id);
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
