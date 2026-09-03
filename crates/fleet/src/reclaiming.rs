//! Giving one finished Job's worktree and branch back, from inside a running
//! Fleet.
//!
//! # It is not the forget, and not `armada clean`
//!
//! `forget_job` deletes the record and says in its own notes why it does not
//! also take the disk: one call with two unrelated things to fail at is worse
//! than two calls, and a person clearing a Board should not have to think about
//! a directory. This is the act it left out, and neither one's outcome depends
//! on the other.
//!
//! `armada clean` is the same removal from the CLI, and it refuses while Fleet
//! is running — which is exactly when a person wants the space back. What the
//! two share is `adapters::reclaim`, one Job at a time; the CLI's loop over a
//! whole Manifest is not reached from here.
//!
//! # There is no force on this seam
//!
//! `armada clean --force` deletes a branch holding commits the base cannot
//! reach. Nothing here can: a live Fleet must never be the thing that destroys
//! work nobody has taken, so the setting is fixed rather than a parameter, and
//! a branch left standing comes back as part of a successful answer rather than
//! as a failure.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, WorktreeSpec};
use adapters::{Reclaimed, UnmergedWork};
use core_model::JobId;

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
    /// Remove this Job's checkout, then delete the branch it derived.
    ///
    /// **The record is untouched.** The Job is still on the Board afterwards
    /// with everything it recorded; [`forget_job`](Fleet::forget_job) is what
    /// takes the row.
    ///
    /// **Terminal only**, for a forget's reason: there is no disk to reclaim
    /// while a Drone might still write to it.
    ///
    /// **The two halves are answered separately**, because a removed checkout
    /// beside a surviving branch is a real outcome and the ordinary one — see
    /// the module for why the safe setting is not a choice here.
    ///
    /// It runs inline. `adapters::reclaim` is version control and a directory
    /// removal on the calling thread, which is what `create_worktree` on the
    /// dispatch path already is; the seam that would move it off this thread is
    /// the `Vcs` trait, and reclaiming is not on it.
    pub async fn reclaim_worktree(&self, job_id: &JobId) -> Result<Reclaimed, Adrift> {
        let job = self.load(job_id).await?;
        if !job.status().is_terminal() {
            return Err(Adrift::NotReclaimable {
                job: job_id.clone(),
                status: job.status(),
            });
        }
        let spec =
            WorktreeSpec::for_job(&self.host().repo_root, job_id.as_str()).map_err(|cause| {
                Adrift::Unworkable {
                    job: job_id.clone(),
                    cause,
                }
            })?;
        // `base:` in `armada.yml` is the repository's own answer to what this
        // branch would have merged into. Where it declares none, `adapters`
        // falls back to the remote's head and then to `main`/`master` — and
        // where nothing answers at all, the branch is kept unanswered rather
        // than deleted on a guess.
        adapters::reclaim(&spec, self.manifest().base(), UnmergedWork::Keep).map_err(|cause| {
            Adrift::NotReclaimed {
                job: job_id.clone(),
                cause,
            }
        })
    }
}
