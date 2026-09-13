//! Whose tree a run is in: one Job's worktree, or the main checkout.
//!
//! **One run at a time is per owner, not per repository.** A Job's run and the
//! checkout's are in different trees and share no build directory, so neither
//! locks the other out; two runs in one tree do, which is what [`Owner`] keys
//! [`Rehearsals`](super::in_flight::Rehearsals) on.
//!
//! **The checkout's runs are kept under `main`.** A Job handle always begins
//! with the Job's own number — `crates/core-model/src/job/handle.rs` emits
//! `{number}` or `{number}-{slug}` and nothing else — so no Job can ever be
//! called `main`, and the two cannot collide under `.armada/runs/`. It is the
//! name `crate::servers` already keeps the main checkout's servers under.

use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree, WorktreeSpec};
use core_model::{Job, JobId};

use super::unrehearsable::Whose;
use crate::daemon::Fleet;

/// The directory the main checkout's runs are kept under.
pub(crate) const CHECKOUT_HANDLE: &str = "main";

/// The key one run at a time is held per.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Owner {
    Job(JobId),
    /// The main checkout of the repository at this root.
    Checkout(String),
}

/// A tree a run happens in, as a path and — where there is one — as what
/// `WorkProduct` reads a diff from.
pub(crate) struct Tree {
    pub(crate) path: PathBuf,
    /// **Absent in the main checkout.** A diff there is against nothing in
    /// particular: there is no branch a Job is working on and no base to
    /// measure from, so nothing narrows and nothing lists what changed
    /// *before* the run. What the run itself changed is the snapshot's answer,
    /// and that is taken the same way in both trees.
    pub(crate) worktree: Option<Worktree>,
    /// Where commands run, relative to `path`: a workspace's own directory in
    /// a Verify of its file. The snapshot and Undo stay on `path`.
    pub(crate) within: Option<PathBuf>,
}

impl Tree {
    /// The directory a command runs in.
    pub(crate) fn commands_in(&self) -> PathBuf {
        match &self.within {
            Some(dir) => self.path.join(dir),
            None => self.path.clone(),
        }
    }
}

/// An owner resolved: what a run of its needs that the owner alone decides.
pub(crate) struct Place {
    pub(crate) owner: Owner,
    /// The Job, where one owns this. `None` is the main checkout.
    pub(crate) job: Option<Job>,
    /// The directory under `.armada/runs/` this owner's runs are kept in.
    pub(crate) handle: String,
    /// The repository the tree is in.
    pub(crate) served: crate::repositories::Served,
}

impl Place {
    pub(crate) fn of_job(job: Job, served: crate::repositories::Served) -> Place {
        Place {
            owner: Owner::Job(job.id().clone()),
            handle: job.handle(),
            job: Some(job),
            served,
        }
    }

    pub(crate) fn of_checkout(served: crate::repositories::Served) -> Place {
        Place {
            owner: Owner::Checkout(served.root().to_string()),
            job: None,
            handle: String::from(CHECKOUT_HANDLE),
            served,
        }
    }

    /// The Job's id, where a Job owns this — what a refusal is said about.
    pub(crate) fn job_id(&self) -> Option<&JobId> {
        self.job.as_ref().map(Job::id)
    }

    /// Which owner a refusal from here is written about.
    pub(crate) fn whose(&self) -> Whose {
        match self.job {
            Some(_) => Whose::Job,
            None => Whose::Checkout,
        }
    }
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
    /// The tree this owner's runs happen in, or `None` where a Job's worktree
    /// is no longer on disk. **The main checkout is always there**: it is the
    /// repository Fleet is running on, and Fleet answering at all is the
    /// proof.
    pub(crate) fn tree_at(&self, place: &Place) -> Option<Tree> {
        let Some(job) = place.job.as_ref() else {
            return Some(Tree {
                path: PathBuf::from(place.served.root()),
                worktree: None,
                within: None,
            });
        };
        let spec = WorktreeSpec::for_job(place.served.root(), &job.handle()).ok()?;
        let path = PathBuf::from(spec.worktree_path());
        path.is_dir().then(|| Tree {
            // Measured from the Manifest's base like every other reading of a
            // Job's work, so the narrowing the sheet offers names the files
            // the diff beside it draws.
            worktree: Some(self.based(
                &place.served,
                Worktree::at(spec.worktree_path(), spec.branch()),
            )),
            path,
            within: None,
        })
    }

    /// The Manifest this owner's entries are read from, and whether the Job
    /// froze it. **The main checkout froze nothing** and reads the Manifest
    /// Fleet holds — `crate::servers::hold_server`'s own split.
    pub(crate) async fn manifest_at(&self, place: &Place) -> (config::Manifest, bool) {
        match place.job.as_ref() {
            Some(job) => self.effective_manifest_in(&place.served, job).await,
            None => (place.served.manifest().clone(), false),
        }
    }

    /// A Job's place, in the repository it was created in.
    pub(crate) fn job_place(&self, job: Job) -> Result<Place, api::Refusal> {
        let served = self.served_by(&job).map_err(|why| self.refusal(why))?;
        Ok(Place::of_job(job, served))
    }
}
