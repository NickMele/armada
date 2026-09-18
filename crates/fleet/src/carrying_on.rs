//! A redispatch, drawn on the Studios that dispatched it. `#1440`.
//!
//! The replacement arrives as its own node beside the Job it replaced, under
//! the `produced` edge that says the first made the second. Nothing is
//! rewritten and nothing is deleted: the node that was there keeps naming the
//! Job that really ran and really was killed.
//!
//! The rules — drawn on the redispatch rather than on the next read, one node
//! per Job however many times a Job is redispatched, and what a chain reads as
//! — are `docs/concepts/studio.md`, *Nodes*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Component, Envelope, FieldValue, JobId, Level, StudioAuthor, StudioEdgeId, StudioNode,
    StudioNodeContent, StudioNodeId, StudioPosition, Timestamp,
};
use std::collections::BTreeSet;
use store::JobOnStudio;

use crate::daemon::Fleet;
use crate::reading_in::{ACROSS, DOWN};

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
    /// Draw `replacement` beside `replaced` on every Studio holding the Job it
    /// replaced, and publish each Studio as changed.
    ///
    /// **Nothing here can fail a redispatch.** Both Jobs are already written
    /// by the time this runs, so a Studio that would not take the node is
    /// noted in the replacement's log rather than telling a person their
    /// redispatch failed while the Job runs.
    ///
    /// **The guard is on the replacement, never on the original**, so no Job
    /// is ever drawn twice. Ordinarily there is no Studio at all, and this is
    /// two reads and no write.
    pub(crate) async fn carried_on_studios(
        &self,
        replaced: &JobId,
        replacement: &JobId,
        at: &Timestamp,
    ) {
        let (holding, already) = {
            let store = self.store().lock().await;
            (
                store.job_on_studios(replaced),
                store.job_on_studios(replacement),
            )
        };
        let (holding, already) = match (holding, already) {
            (Ok(holding), Ok(already)) => (holding, already),
            (Err(why), _) | (_, Err(why)) => {
                return self.studio_not_drawn(replacement, &why.to_string());
            }
        };
        let drawn: BTreeSet<_> = already.iter().map(|on| on.studio_id.clone()).collect();
        for on in holding.iter().filter(|on| !drawn.contains(&on.studio_id)) {
            self.replacement_drawn(on, replacement, at).await;
        }
    }

    /// One Studio's copy of the replacement: the node, and the `produced` edge
    /// from the node it replaced.
    async fn replacement_drawn(&self, on: &JobOnStudio, replacement: &JobId, at: &Timestamp) {
        let node = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            StudioNodeContent::Job {
                job_id: replacement.clone(),
            },
            // One column right, where a read-in puts what it produced. A
            // second replacement of one Job takes the row under the first.
            StudioPosition {
                x: on.position.x + ACROSS,
                y: on.position.y + DOWN * on.produced,
            },
            at.clone(),
            // A person: a redispatch is a person's press, and `redispatch_job`
            // carries no other actor. `crate::redispatch`.
            StudioAuthor::Person,
        );
        let edge = StudioEdgeId::carried(self.mint().ulid());
        let written = self
            .written(&ipc::StudioId::from(&on.studio_id), None, |store, id| {
                store.add_studio_node_produced_by(id, &node, &[(&on.node_id, edge.clone())], at)
            })
            .await;
        if let Err(why) = written {
            self.studio_not_drawn(replacement, &format!("{why:?}"));
        }
    }

    /// Where a Studio that could not take the replacement says so. Warn, in
    /// the replacement's own log: the Job runs either way, and the Studio is
    /// what is now incomplete.
    fn studio_not_drawn(&self, replacement: &JobId, cause: &str) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "a Studio holding the Job this one replaced did not take the replacement; \
             that Studio still shows only the Job that stopped",
        )
        .in_job(replacement.as_ulid().clone())
        .with_field("cause", FieldValue::Str(cause.to_string()));
        self.noted_in_the_log(replacement, &envelope);
    }
}
