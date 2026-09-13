//! Whether a repository has stopped new work starting there — the one predicate
//! admission, the step boundary and the Board share.
//!
//! **Most-restrictive-wins across a Job's gating Manifests**: any frozen one holds
//! the Job, `docs/concepts/convoy.md`. **A person's act is never refused for it**
//! (`docs/concepts/fleet.md`): an approval still lands at `queued`, and this is
//! what holds it there.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::Manifest;
use core_model::{Component, Envelope, FieldValue, Job, JobId, Level, ManifestId, StepId};

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
}
