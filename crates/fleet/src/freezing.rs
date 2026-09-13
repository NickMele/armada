//! Whether a repository has stopped new work starting there — the one predicate
//! admission, the step boundary and the Board share.
//!
//! **Most-restrictive-wins across a Job's gating Manifests**: any frozen one holds
//! the Job, `docs/concepts/convoy.md`. **A person's act is never refused for it**
//! (`docs/concepts/fleet.md`): an approval still lands at `queued`, and this is
//! what holds it there. **Nothing lands while it holds** either: delivery waits
//! with the step it enters, and a merge — the sweep's or a person's — waits here.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::Manifest;
use core_model::{Actor, Component, Envelope, FieldValue, Job, JobId, Level, ManifestId, StepId};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The frozen Manifests among `gating`, in the order given. Empty is clear.
pub fn frozen_among<'a>(gating: impl IntoIterator<Item = &'a Manifest>) -> Vec<ManifestId> {
    gating
        .into_iter()
        .filter(|manifest| manifest.frozen())
        .map(|manifest| manifest.id().clone())
        .collect()
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
    /// Which of this Job's gating Manifests are frozen, read live.
    ///
    /// **The owner and every gate Manifest this Fleet serves.** One it does not
    /// serve cannot be read and is not guessed frozen; the paths that need it
    /// refuse the Job on their own.
    pub(crate) fn frozen_by(&self, job: &Job) -> Vec<ManifestId> {
        let mut ids = vec![job.owner_manifest_id().clone()];
        for gate in job.gate_manifests() {
            if !ids.contains(&gate.manifest_id) {
                ids.push(gate.manifest_id.clone());
            }
        }
        let served: Vec<_> = ids
            .iter()
            .filter_map(|id| self.repositories().serving(id.as_str()))
            .collect();
        frozen_among(served.iter().map(|one| one.manifest()))
    }

    /// Whether the Job stands its Drone down at this boundary rather than start
    /// the next step: its children are still going, or a freeze holds it.
    pub(crate) async fn stands_down_after(&self, job: &Job, step: &StepId) -> Result<bool, Adrift> {
        if self.dispatched_and_waits(job, step).await? {
            return Ok(true);
        }
        let frozen = self.frozen_by(job);
        if frozen.is_empty() {
            return Ok(false);
        }
        self.noted_frozen(job.id(), &frozen);
        Ok(true)
    }

    /// The Job's own log is where a person looks for why it stopped moving.
    fn noted_frozen(&self, job: &JobId, frozen: &[ManifestId]) {
        let named: Vec<&str> = frozen.iter().map(ManifestId::as_str).collect();
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a Manifest gating this Job says freeze: true, so its step passed and the next \
             one waits in the queue until the freeze is lifted",
        )
        .in_job(job.as_ulid().clone())
        .with_field("frozen_by", FieldValue::Str(named.join(", ")));
        self.noted_in_the_log(job, &envelope);
    }

    /// A person's merge press at a frozen repository: checked as a press is, written
    /// down, and left for the sweep. `None` where nothing is frozen and the press merges.
    pub(crate) async fn merge_pressed_while_frozen(
        &self,
        job_id: &JobId,
    ) -> Result<Option<Job>, Adrift> {
        let job = self.load(job_id).await?;
        let frozen = self.frozen_by(&job);
        if frozen.is_empty() {
            return Ok(None);
        }
        self.at_the_gate(&job)?;
        let url = self.pull_request_of(job_id).await?;
        self.sweeping()
            .lock()
            .await
            .pressed_while_frozen
            .insert(job_id.clone());
        self.noted_merge_held(
            job_id,
            &frozen,
            &url,
            "a person pressed merge while a Manifest gating this Job is frozen: the press is \
             taken and nothing merges until the freeze is lifted",
        );
        Ok(Some(job))
    }

    /// Whether the sweep leaves this pull request alone: a freeze holds it, or a
    /// press held by one just merged it.
    pub(crate) async fn held_from_the_sweep(&self, job: &Job, url: &str) -> bool {
        let frozen = self.frozen_by(job);
        if !frozen.is_empty() {
            let first = self
                .sweeping()
                .lock()
                .await
                .held_by_freeze
                .insert(url.to_string());
            if first {
                self.noted_merge_held(
                    job.id(),
                    &frozen,
                    url,
                    "a Manifest gating this Job is frozen, so its pull request is not merged \
                     until the freeze is lifted",
                );
            }
            return true;
        }
        let pressed = {
            let mut sweeping = self.sweeping().lock().await;
            sweeping.held_by_freeze.remove(url);
            sweeping.pressed_while_frozen.remove(job.id())
        };
        if !pressed {
            return false;
        }
        // Held, never raised: `merged` has already said any refusal in the Job's log.
        let _ = self
            .merged(
                job.id(),
                Actor::Human,
                "merging a pull request a person pressed for while the repository was \
                 frozen, now that the freeze is lifted",
            )
            .await;
        true
    }

    fn noted_merge_held(&self, job: &JobId, frozen: &[ManifestId], url: &str, said: &'static str) {
        let named: Vec<&str> = frozen.iter().map(ManifestId::as_str).collect();
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.as_ulid().clone())
        .with_field("frozen_by", FieldValue::Str(named.join(", ")))
        .with_field("pull_request", FieldValue::Str(url.to_string()));
        self.noted_in_the_log(job, &envelope);
    }
}
