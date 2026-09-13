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
//!
//! **[`Fleet::delete_branch`] is a person's act, not Fleet's.** It takes the
//! branch a reclaim kept only against the tip that person was shown.

use std::fmt;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, WorktreeSpec};
use adapters::{BranchGone, BranchRefused, Reclaimed, UnmergedWork};
use api::Refusal;
use core_model::{Component, Envelope, FieldValue, JobId, JobStatus, Level};

use crate::adrift::Adrift;
use crate::budget::budgeted_for;
use crate::daemon::Fleet;

/// Why a person's `delete_branch` was refused. Nothing was touched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Undeletable {
    NotTerminal {
        status: JobStatus,
    },
    CheckoutOnDisk {
        path: String,
    },
    Absent {
        branch: String,
    },
    /// **The guard.** The branch moved since the person looked at it.
    TipMoved {
        branch: String,
        asked: String,
        found: String,
    },
}

impl fmt::Display for Undeletable {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Undeletable::NotTerminal { status } => write!(
                out,
                "the Job is {} and may still need it. `kill_job` ends a Job still in flight",
                status.as_wire()
            ),
            Undeletable::CheckoutOnDisk { path } => write!(
                out,
                "its checkout is still at {path}. Reclaim the worktree first, then delete the branch"
            ),
            Undeletable::Absent { branch } => {
                write!(out, "{branch} is already gone, so there is nothing to delete")
            }
            Undeletable::TipMoved {
                branch,
                asked,
                found,
            } => write!(
                out,
                "{branch} now stands at {found}, not {asked}. Look at it again before deleting it"
            ),
        }
    }
}

/// The branch a person deleted, and the tip it is recoverable from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletedBranch {
    pub branch: String,
    pub tip: String,
}

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
    /// **The record survives, marked rather than untouched.** The Job is
    /// still on the Board afterwards with everything it recorded — its log,
    /// its Checks, its Judgments, its workflow results — and
    /// [`forget_job`](Fleet::forget_job) is still what takes the row. What
    /// does change is the one thing that is a resource and not a record: the
    /// stale Drone pid `retain_job` clears, and the `reclaimed_at` stamp a
    /// Board reads to tell this Job apart from one whose disk still stands.
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
        let served = self.served_by(&job)?;
        let spec = WorktreeSpec::for_job(served.root(), &job.handle()).map_err(|cause| {
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
        let reclaimed = adapters::reclaim(&spec, served.manifest().base(), UnmergedWork::Keep)
            .map_err(|cause| Adrift::NotReclaimed {
                job: job_id.clone(),
                cause,
            })?;
        // The stale half `armada clean` already clears — a Drone's pid row
        // naming a worktree that just went — and the mark that tells a later
        // read this Job's disk is back. Stamped after the worktree succeeds,
        // for `armada clean`'s reason: touching the row first and the
        // worktree failing to go would leave a row nothing can derive a
        // worktree for.
        self.store()
            .lock()
            .await
            .retain_job(job_id, &self.now())
            .map_err(Adrift::Writing)?;
        Ok(reclaimed)
    }

    /// Delete this terminal Job's branch, unmerged commits and all, once its
    /// checkout is gone and only while the branch stands at `tip`.
    ///
    /// **`UnmergedWork::Delete`, which the sweep and `reclaim_worktree` never
    /// use.** A person confirmed against this tip; Fleet deciding alone did not.
    pub async fn delete_branch(&self, job_id: &JobId, tip: &str) -> Result<DeletedBranch, Adrift> {
        let job = self.load(job_id).await?;
        let refused = |why| Adrift::BranchNotDeletable {
            job: job_id.clone(),
            why,
        };
        if !job.status().is_terminal() {
            return Err(refused(Undeletable::NotTerminal {
                status: job.status(),
            }));
        }
        let served = self.served_by(&job)?;
        let spec = WorktreeSpec::for_job(served.root(), &job.handle()).map_err(|cause| {
            Adrift::Unworkable {
                job: job_id.clone(),
                cause,
            }
        })?;
        let failed = |why| Adrift::BranchNotDeleted {
            job: job_id.clone(),
            why,
        };
        let gone = adapters::delete_branch(&spec, tip).map_err(|no| match no {
            BranchRefused::CheckoutOnDisk { path } => refused(Undeletable::CheckoutOnDisk { path }),
            BranchRefused::Absent { branch } => refused(Undeletable::Absent { branch }),
            BranchRefused::TipMoved {
                branch,
                asked,
                found,
            } => refused(Undeletable::TipMoved {
                branch,
                asked,
                found,
            }),
            BranchRefused::Unreadable(cause) => {
                failed(format!("{} would not open: {}", cause.repo, cause.why))
            }
        })?;
        let BranchGone::Deleted { branch, tip } = gone else {
            return Err(failed(crate::holding::said_of_branch(&gone)));
        };
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a person deleted this job's branch",
        )
        .in_job(job_id.as_ulid().clone())
        .with_field("branch", FieldValue::Str(branch.clone()))
        .with_field("tip", FieldValue::Str(tip.clone()));
        self.noted_in_the_log(job_id, &envelope);
        Ok(DeletedBranch { branch, tip })
    }

    /// `Commands::reclaim_worktree`. **Nothing is redacted**: every field is
    /// about a directory and a branch this Job derived.
    pub(crate) async fn reclaim_answered(
        self: Arc<Self>,
        job_id: ipc::JobId,
    ) -> Result<ipc::WorktreeReclaimed, Refusal> {
        let id = job_id.to_domain();
        let gave_back = budgeted_for(self.command_budget(), job_id, {
            let fleet = Arc::clone(&self);
            let id = id.clone();
            async move { Fleet::reclaim_worktree(&fleet, &id).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        Ok(crate::wire::reclaimed(&id, gave_back))
    }

    /// `Commands::delete_branch`, redacting nothing for the reason above.
    pub(crate) async fn delete_branch_answered(
        self: Arc<Self>,
        job_id: ipc::JobId,
        asked: ipc::DeleteBranch,
    ) -> Result<ipc::BranchDeleted, Refusal> {
        let deleted = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            let id = job_id.to_domain();
            async move { Fleet::delete_branch(&fleet, &id, &asked.tip).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        Ok(ipc::BranchDeleted {
            job_id,
            branch: deleted.branch,
            tip: deleted.tip,
        })
    }
}
