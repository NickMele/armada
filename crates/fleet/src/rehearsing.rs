//! A person's run of one Manifest entry — Journey 9, *Running one* and
//! *Running one inside a Job* — and what it leaves behind.
//!
//! **A rehearsal, never a verdict.** Nothing here writes Evidence or a
//! `job_step_checks` row, and nothing moves a Job: the run's own directory
//! under `.armada/runs` is its only record ([`records`]).
//!
//! **Off the turn loop**, for `crate::showing_again`'s reason: the run is a
//! task of its own, and one run at a time per owner is [`Rehearsals`]'s.
//!
//! **Keyed on an owner — a Job, or the main checkout.** The act itself is
//! [`shared`]'s and takes a [`Place`](owner::Place), never a `JobId`. What the
//! owners do not share is at the edges — the entrances here and in
//! [`checkout`], and the shapes [`record`] projects.

mod checkout;
mod entries;
mod in_flight;
mod owner;
mod record;
/// `pub(crate)` because a server's log is a run's log one directory over, read
/// and followed the same way — `crate::servers`.
pub(crate) mod records;
mod running;
mod shared;
mod unrehearsable;

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{JobId, JobStatus};

use crate::daemon::Fleet;
use in_flight::Held;
pub(crate) use in_flight::Rehearsals;
use owner::Place;
use record::Record;
pub use unrehearsable::{Unrehearsable, Whose};

/// How long a stop waits for a stopped run's record. The group is ended with
/// `SIGKILL`, so what is left is one tree read and one file write.
const STOPPING: Duration = Duration::from_secs(30);

/// What one viewer of a run's socket is answered with, before either owner's
/// opening message is built from it.
pub(crate) struct Seen {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) live: Option<api::RunWatch>,
    pub(crate) history: Vec<String>,
    pub(crate) skipped: u64,
    pub(crate) read_to: u64,
    pub(crate) unreadable: bool,
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
    // -----------------------------------------------------------------------
    // A Job's half — Journey 9, *Running one inside a Job*
    // -----------------------------------------------------------------------

    /// What the run sheet lists for this Job, and the facts beside it.
    pub(crate) async fn run_sheet(&self, job_id: &JobId) -> Result<ipc::RunSheet, Refusal> {
        let loaded = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let place = Place::of_job(loaded.clone());
        let tree = self.tree_at(&place);
        let (manifest, has_snapshot) = self.manifest_at(&place).await;
        let listed = entries::frozen(&loaded, &manifest, has_snapshot);
        let changed = tree
            .as_ref()
            .map(|tree| self.changed_in(tree))
            .unwrap_or_default();
        let (worktree_differs, worktree_unreadable) = match tree.as_ref().map(|t| self.theirs(t)) {
            None => (false, None),
            Some(Ok(theirs)) => (!listed.same_as(&entries::declared(&theirs)), None),
            Some(Err(why)) => (false, Some(why)),
        };
        let (setup, checks, commands) = listed.sheet(&changed);
        let wired = ipc::JobId::from(job_id);
        Ok(ipc::RunSheet {
            job_id: wired.clone(),
            setup,
            checks,
            commands,
            manifest_edited_at: self.edited_before(loaded.created_at(), tree.as_ref()),
            worktree_on_disk: tree.is_some(),
            worktree_differs,
            worktree_unreadable,
            drone_working: loaded.status() == JobStatus::Running,
            running: self
                .rehearsals()
                .in_flight(&place.owner)
                .map(|out| out.of_job(&wired)),
            servers: self.declared_servers(&crate::servers::Holder::Job(job_id.clone()), &manifest),
        })
    }

    /// Start one run in this Job's worktree, and answer as soon as it is
    /// underway.
    pub(crate) async fn start_rehearsal(
        self: Arc<Self>,
        job_id: &JobId,
        asked: ipc::StartRun,
    ) -> Result<ipc::RunUnderway, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let place = Place::of_job(job);
        let owner = place.owner.clone();
        let (entry, tree) = self
            .entry_at(&place, &asked.name, asked.worktree_version)
            .await
            .map_err(|why| self.refused_run(&owner, why))?;
        let (command, narrowed) = match asked.narrowed {
            false => (entry.run.clone(), false),
            true => self
                .narrowed(&entry, &tree)
                .map_err(|why| self.refused_run(&owner, why))?,
        };
        let underway = Arc::clone(&self)
            .started_at(
                place,
                tree,
                entry,
                command,
                narrowed,
                asked.worktree_version,
            )
            .await
            .map_err(|why| self.refused_run(&owner, why))?;
        Ok(underway.of_job(&ipc::JobId::from(job_id)))
    }

    /// End a run's group, and answer with its record once it is written.
    pub(crate) async fn stop_rehearsal(
        &self,
        job_id: &JobId,
        id: String,
    ) -> Result<ipc::RunRecord, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let place = Place::of_job(job);
        let record = self
            .stopped_at(&place, id)
            .await
            .map_err(|why| self.refused_run(&place.owner, why))?;
        self.only_a_jobs(&place, record)
    }

    /// Put back what one run changed, from the snapshot taken before it.
    pub(crate) async fn undo_rehearsal(
        &self,
        job_id: &JobId,
        id: String,
    ) -> Result<ipc::RunRecord, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let place = Place::of_job(job);
        // **First**: a Drone's work is uncommitted until delivery, and nothing
        // below can tell its edits from the run's.
        if place
            .job
            .as_ref()
            .is_some_and(|job| job.status() == JobStatus::Running)
        {
            return Err(self.refused_run(&place.owner, Unrehearsable::DroneWorking));
        }
        let record = self
            .undone_at(&place, id)
            .await
            .map_err(|why| self.refused_run(&place.owner, why))?;
        self.only_a_jobs(&place, record)
    }

    /// This Job's earlier runs, newest first.
    pub(crate) async fn rehearsal_history(&self, job_id: &JobId) -> Result<ipc::RunList, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let place = Place::of_job(job);
        let (kept, unreadable) = self.history_at(&place);
        Ok(ipc::RunList {
            job_id: ipc::JobId::from(job_id),
            runs: kept.iter().filter_map(Record::of_job).collect(),
            unreadable,
        })
    }

    /// One run's log. **The Job's own runs are the allowlist**: an id that
    /// names none of them reaches no file.
    pub(crate) async fn rehearsal_output(
        &self,
        job_id: &JobId,
        id: String,
    ) -> Result<ipc::RunOutput, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let place = Place::of_job(job);
        self.output_at(&place, &id)
            .ok_or_else(|| self.no_such_run(&place, id))
    }

    /// A run's socket, resolved before it opens: **the subscription first,
    /// then the log as history**, for `observe_job`'s reason. A finished run
    /// opens too, with its whole log and nothing live.
    pub(crate) async fn observe_rehearsal(
        &self,
        job_id: &JobId,
        id: String,
    ) -> Result<api::ObservedRun, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        let place = Place::of_job(job);
        let seen = self
            .observed_at(&place, &id)
            .ok_or_else(|| self.no_such_run(&place, id))?;
        Ok(api::ObservedRun {
            job_id: ipc::JobId::from(job_id),
            id: seen.id,
            name: seen.name,
            path: seen.path,
            live: seen.live,
            history: seen.history,
            skipped: seen.skipped,
            read_to: seen.read_to,
            unreadable: seen.unreadable,
        })
    }
}
