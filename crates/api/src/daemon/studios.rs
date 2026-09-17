//! Everything a client asks of a Studio. `#1285`.
//!
//! **A sixth surface, for [`Conversations`](super::Conversations)' reason.** A
//! Studio is one record with its own reads and writes, and it answers for a
//! repository rather than a Job, so none of it belongs beside `get_job`.
//!
//! **`within` is the door's scope and nothing else's.** Bridge names a Studio
//! by id alone; an agent's door session is answered only about the Manifest it
//! stands in, and a Studio of another repository is refused for it the way a
//! Job is.

use std::future::Future;

use ipc::{
    AddStudioNode, CreateStudio, DecideStudioEdge, ManifestId, MoveStudioNode, ProposeStudioEdge,
    RemoveStudioNode, RenameStudio, Studio, StudioDeleted, StudioId, StudioList,
};

use crate::daemon::{Redirector, Refusal};

pub trait Studios: Send + Sync + 'static {
    /// `list_studios` — `manifest_id` absent is the first repository served.
    fn list_studios(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<StudioList, Refusal>> + Send;

    /// `get_studio`. [`Refusal::NoSuchJob`]'s 404 where no Studio is `studio_id`.
    fn get_studio(
        &self,
        studio_id: StudioId,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `create_studio`, in the repository `manifest_id` names.
    fn create_studio(
        &self,
        create: CreateStudio,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `rename_studio`.
    fn rename_studio(
        &self,
        studio_id: StudioId,
        rename: RenameStudio,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `delete_studio`.
    fn delete_studio(
        &self,
        studio_id: StudioId,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<StudioDeleted, Refusal>> + Send;

    /// `add_studio_node`. **`by` is placed by the transport**, never claimed by
    /// the body: a node Helm adds is held to the kinds that start proposed.
    fn add_studio_node(
        &self,
        studio_id: StudioId,
        add: AddStudioNode,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `move_studio_node`.
    fn move_studio_node(
        &self,
        studio_id: StudioId,
        moving: MoveStudioNode,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `remove_studio_node`.
    fn remove_studio_node(
        &self,
        studio_id: StudioId,
        removing: RemoveStudioNode,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `propose_studio_edge`. Lands proposed, whoever sent it.
    fn propose_studio_edge(
        &self,
        studio_id: StudioId,
        proposal: ProposeStudioEdge,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `decide_studio_edge`. [`Refusal::IllegalMove`] where the edge is not
    /// proposed.
    fn decide_studio_edge(
        &self,
        studio_id: StudioId,
        decision: DecideStudioEdge,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;
}
