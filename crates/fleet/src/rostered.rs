//! The Drones Fleet is holding, read without touching a working slot.
//!
//! **A slot is held for the length of a Check.** A read that took one would
//! block behind a gate for minutes, so every fact below comes from the process
//! register, the Job record and the Job's own log.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{DronePresence, Job};
use ipc::{DroneDetail, DroneId, DroneList, DroneSummary};
use store::Moved;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// How many of one Drone's rows `get_drone` answers with.
///
/// **A tail, and it says so.** `backfill::HISTORY` is what a person scrolls
/// through on a socket; this is read into a caller's own memory in one reply,
/// and what it leaves out is counted rather than dropped.
pub(crate) const TURNS: usize = 200;

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
    /// `list_drones` — every Drone Fleet holds a process for.
    pub(crate) async fn drone_list(&self) -> Result<DroneList, Refusal> {
        let mut drones = Vec::new();
        for (job_id, pid) in self.drones_at_work() {
            let job = self.load(&job_id).await.map_err(|why| self.refusal(why))?;
            if let Some(summary) = self.drone_summary(&job, pid).await? {
                drones.push(summary);
            }
        }
        Ok(DroneList { drones })
    }

    /// `get_drone` — one Drone, its declaration and a window of its rows.
    pub(crate) async fn drone_detail(&self, drone_id: DroneId) -> Result<DroneDetail, Refusal> {
        let wanted = drone_id.to_domain();
        for (job_id, pid) in self.drones_at_work() {
            let job = self.load(&job_id).await.map_err(|why| self.refusal(why))?;
            if job.assigned_drone() != Some(&wanted) {
                continue;
            }
            let Some(drone) = self.drone_summary(&job, pid).await? else {
                continue;
            };
            let plan = self
                .store()
                .lock()
                .await
                .step_plans(&job_id)
                .map_err(|why| self.refusal(Adrift::Reading(why)))?
                .into_iter()
                // The step the Drone is on, latest attempt last — a step worked
                // twice declares twice, and the promise in force is the last.
                .filter(|plan| Some(&plan.step_id) == job.current_step_id())
                .next_back();
            let (turns, older) = crate::transcript::one_drones_rows(
                &self.host().records_root,
                &job.handle(),
                &wanted,
                TURNS,
            )
            .await;
            return Ok(DroneDetail {
                drone,
                declared: plan.as_ref().map(|plan| {
                    plan.paths
                        .paths()
                        .iter()
                        .map(|path| path.as_str().to_string())
                        .collect()
                }),
                declared_at: plan.as_ref().map(|plan| (&plan.declared_at).into()),
                turns,
                older,
            });
        }
        // **A refusal, never an empty detail.** A Drone that has exited is not
        // in the register, and an empty answer would draw a dead process as a
        // live one with nothing to say.
        Err(self.refusal(Adrift::NoSuchDrone {
            named: drone_id.as_str().to_string(),
        }))
    }

    /// One row, from the record rather than from the slot.
    ///
    /// `None` where the Job's record names no Drone on a step — a process
    /// register entry whose Job has already been folded past it, which is a
    /// moment rather than a state and is left out rather than half-answered.
    async fn drone_summary(&self, job: &Job, pid: u32) -> Result<Option<DroneSummary>, Refusal> {
        let (Some(drone), Some(step)) = (job.assigned_drone(), job.current_step_id()) else {
            return Ok(None);
        };
        let events = self
            .store()
            .lock()
            .await
            .events_for(job.id())
            .map_err(|cause| {
                self.refusal(Adrift::Reading(store::LoadJobError::Unreadable(cause)))
            })?;
        let arrived = events.iter().rev().find(|event| {
            matches!(
                event.moved(),
                Moved::Drone { drone_id, presence: DronePresence::Spawned, .. } if drone_id == drone
            )
        });
        Ok(Some(DroneSummary {
            drone_id: drone.into(),
            job_id: job.id().into(),
            handle: job.handle(),
            step_id: step.into(),
            worktree: self
                .worktree_of(job)
                .map_err(|why| self.refusal(why))?
                .map(|worktree| worktree.path().to_string()),
            pid,
            since: arrived.map(|event| event.at().into()),
        }))
    }
}
