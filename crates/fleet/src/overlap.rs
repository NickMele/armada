//! Which other Jobs claim to write where this one says it will.
//!
//! **Reachable from `get_job` and from nothing on the dispatch path**, which is
//! how "surfaced, never serialised" is held: `admit_next` cannot consult this
//! even by mistake.
//!
//! A Job's claims come from two authors, because one of them is usually absent:
//! `write_targets` is null on every Job the proposer drafted, and what a
//! running Job has instead is its Drones' per-step declarations.
//!
//! `docs/concepts/fleet.md`, Write-scope overlap, holds the rest.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{collisions, Attempt, Job, RepoPath, ScopeClaim, StepId};

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
    /// Every other unfinished Job that claims a path this one claims.
    ///
    /// **`None` is "no comparison was made"**, and it is not the same answer as
    /// an empty list. `None` is this Job having claimed nothing yet, which is
    /// every proposer-drafted Job until its first scope step declares; empty is
    /// a comparison that ran and found nobody. A card that drew those the same
    /// way would tell a person there is no overlap when nothing had looked.
    ///
    /// **Every unfinished Job, not only the running ones.** A Job waiting at
    /// the gate beside this one is about to write the same place, and the pair
    /// is one fact — read off either card, it has to say the same thing. Naming
    /// only the running peers would have made this Job's card and the other
    /// Job's card disagree about whether there is a collision. The other Job's
    /// status travels with it, so a person can see which of the two is already
    /// writing.
    pub(crate) async fn write_scope_overlaps(
        &self,
        job: &Job,
    ) -> Result<Option<Vec<ipc::ScopeOverlap>>, Adrift> {
        let mine = self.scope_claims(job).await?;
        if mine.is_empty() {
            return Ok(None);
        }
        let (loaded, _) = self.every_job().await?;
        let mut found = Vec::new();
        for other in &loaded.jobs {
            if other.id() == job.id() || other.status().is_terminal() {
                continue;
            }
            let theirs = self.scope_claims(other).await?;
            let shared = collisions(&mine, &theirs);
            if shared.is_empty() {
                continue;
            }
            found.push(ipc::ScopeOverlap {
                job_id: ipc::JobId::from(other.id()),
                title: other.title().as_str().to_string(),
                status: ipc::JobStatus::from(other.status()),
                paths: shared
                    .into_iter()
                    .map(|at| ipc::SharedPath {
                        path: at.path.as_str().to_string(),
                        this_step: at.mine.as_ref().map(ipc::StepId::from),
                        other_step: at.theirs.as_ref().map(ipc::StepId::from),
                    })
                    .collect(),
            });
        }
        Ok(Some(found))
    }

    /// Everything one Job has said about where its writes land.
    ///
    /// Two authors and one list: the Job's own `write_targets`, where the
    /// requester supplied one, and the latest run of each step that declared a
    /// plan.
    ///
    /// **Every step, not only the one being worked.** An earlier step's writes
    /// are still sitting unlanded in the same worktree, so a Job that touched
    /// `crates/store` on step one is still the Job somebody else will conflict
    /// with on step three.
    ///
    /// **The latest run of a step replaces the earlier ones.** Calling the
    /// scope tool again is how a Drone corrects its plan, so an earlier run's
    /// paths are a promise that was withdrawn — counting them would name a
    /// collision over a path nobody intends to write any more.
    async fn scope_claims(&self, job: &Job) -> Result<Vec<ScopeClaim>, Adrift> {
        let mut claims = Vec::new();
        if let Some(targets) = job.write_targets() {
            claims.push(ScopeClaim::by_the_job(targets.paths().to_vec()));
        }
        // A Job that has never been dispatched has no rows here, and one that
        // never declares has none either. The read is cheap and unconditional
        // rather than gated on status, because "declared" is not a status.
        let plans = self
            .store()
            .lock()
            .await
            .step_plans(job.id())
            .map_err(Adrift::Reading)?;
        let mut latest: Vec<(StepId, Attempt, Vec<RepoPath>)> = Vec::new();
        for plan in plans {
            match latest.iter_mut().find(|(step, _, _)| *step == plan.step_id) {
                Some(held) if held.1 >= plan.attempt => {}
                Some(held) => {
                    held.1 = plan.attempt;
                    held.2 = plan.paths.paths().to_vec();
                }
                None => latest.push((plan.step_id, plan.attempt, plan.paths.paths().to_vec())),
            }
        }
        for (step, _, paths) in latest {
            claims.push(ScopeClaim::by_a_step(step, paths));
        }
        Ok(claims)
    }
}
