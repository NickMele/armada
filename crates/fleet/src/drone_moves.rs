//! A Drone arriving on a Job and leaving it: the column, the log row, and the
//! two events Bridge folds.
//!
//! Kept out of [`dispatch`](mod@crate::dispatch) because it is a different
//! subject — that file is what happens to a Job in the slot, and this is the
//! one field whose whole content is whether a process exists.
//!
//! # Arriving answers and leaving does not
//!
//! A spawn that cannot be recorded fails the dispatch: `assigned_drone` is what
//! the liveness clock and every surface read to know a process exists, and a
//! Drone running against a record that does not say so is exactly the lie this
//! event was added to remove. A departure that cannot be recorded is swallowed,
//! because every caller is already ending the Drone for a reason of its own and
//! a failure to write the departure must not turn a Job that ended into an
//! error about bookkeeping.

use adapter_traits::{AgentHarness, Vcs, WorkProduct};
use core_model::{Actor, DroneAssigned, DroneId, Job, JobId};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Record that a Drone is on the Job, and say so on the stream.
    pub(crate) async fn drone_arrived(&self, job: &Job, drone: DroneId) -> Result<(), Adrift> {
        let moved = job
            .drone_spawned(drone, Actor::Fleet, self.now())
            .map_err(Adrift::IllegalDroneMove)?;
        self.record_drone(&moved).await?;
        self.publish(ipc::Event::DroneSpawned(ipc::DroneSpawned::of(
            &moved.event,
            ipc::JobSummary::from(&moved.job),
            moved.job.branch().map(|branch| branch.as_str().to_string()),
        )));
        Ok(())
    }

    /// Record that the Job's Drone is gone.
    ///
    /// The refusal a Job with no Drone raises is the ordinary case here:
    /// `kill_drone` on a Job whose Drone has already exited reaches it.
    pub(crate) async fn drone_left(&self, job_id: &JobId) {
        let Ok(job) = self.load(job_id).await else {
            return;
        };
        let Ok(moved) = job.drone_exited(Actor::Fleet, self.now()) else {
            return;
        };
        if self.record_drone(&moved).await.is_ok() {
            self.publish(ipc::Event::DroneExited(ipc::DroneExited::of(
                &moved.event,
                ipc::JobSummary::from(&moved.job),
            )));
        }
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
