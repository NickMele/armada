//! What a caller scoped to one repository is listed: the Drones, worktrees and
//! servers of the Jobs its Manifest owns. Absent a Manifest, every one. `#987`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{DroneList, JobId, ManifestId, ServerList, WorktreesHeld};

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
    /// The Manifest named, checked served; a list is never narrowed to one
    /// Fleet does not hold.
    fn scoped(&self, within: Option<ManifestId>) -> Result<Option<ManifestId>, Refusal> {
        match within {
            None => Ok(None),
            Some(named) => Ok(Some(ManifestId::from(
                self.served_named(Some(&named))?.manifest().id(),
            ))),
        }
    }

    fn owns(&self, within: Option<&ManifestId>, job: &JobId) -> bool {
        within.is_none_or(|named| {
            self.names().owner_of(&job.to_domain()).as_deref() == Some(named.as_str())
        })
    }

    pub(crate) async fn drones_within(
        &self,
        within: Option<ManifestId>,
    ) -> Result<DroneList, Refusal> {
        let within = self.scoped(within)?;
        let mut listed = self.drone_list().await?;
        listed
            .drones
            .retain(|drone| self.owns(within.as_ref(), &drone.job_id));
        Ok(listed)
    }

    pub(crate) fn servers_within(&self, within: Option<ManifestId>) -> Result<ServerList, Refusal> {
        let within = self.scoped(within)?;
        let mut listed = self.server_list();
        listed
            .servers
            .retain(|server| match (&within, &server.manifest_id) {
                (None, _) => true,
                (Some(named), Some(owner)) => owner == named,
                (Some(_), None) => server
                    .job_id
                    .as_ref()
                    .is_some_and(|job| self.owns(within.as_ref(), job)),
            });
        Ok(listed)
    }

    pub(crate) fn worktrees_within(
        &self,
        mut held: WorktreesHeld,
        within: Option<ManifestId>,
    ) -> Result<WorktreesHeld, Refusal> {
        let within = self.scoped(within)?;
        held.worktrees
            .retain(|one| self.owns(within.as_ref(), &one.job_id));
        Ok(held)
    }

    /// Every Job a served Manifest owns, by id.
    pub(crate) fn jobs_owned_by(&self, manifest_id: ManifestId) -> Result<Vec<JobId>, Refusal> {
        let within = self.scoped(Some(manifest_id))?;
        let named = within.as_ref().map(ManifestId::as_str).unwrap_or_default();
        Ok(self
            .names()
            .owned_by(named)
            .iter()
            .map(JobId::from)
            .collect())
    }
}
