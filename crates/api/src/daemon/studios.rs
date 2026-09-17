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
use std::sync::Arc;

use ipc::{
    AddStudioNode, AskScout, CreateStudio, DecideStudioEdge, ManifestId, MoveStudioNode,
    ProposeStudioEdge, RemoveStudioNode, RenameStudio, StartScout, StartStudioRun, StopScout,
    Studio, StudioDeleted, StudioId, StudioList, StudioRunStarted,
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

    /// `rename_studio`. `by` is the transport's word, as on
    /// [`add_studio_node`](Studios::add_studio_node): Helm's naming is
    /// published as its own act.
    fn rename_studio(
        &self,
        studio_id: StudioId,
        rename: RenameStudio,
        by: Redirector,
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

    /// `propose_studio_edge`. Lands proposed, whoever sent it; `by` says whose
    /// act to publish it as.
    fn propose_studio_edge(
        &self,
        studio_id: StudioId,
        proposal: ProposeStudioEdge,
        by: Redirector,
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

    /// `ask_scout` — a person's ask, made a Finding and started, answered with
    /// the Studio once it is Gathering. **The scout's reading is a task of its
    /// own**, so what it reads arrives on `studio.changed`.
    fn ask_scout(
        self: Arc<Self>,
        studio_id: StudioId,
        ask: AskScout,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `start_scout` — a Proposed Finding started. [`Refusal::IllegalMove`]
    /// where it is not Proposed.
    fn start_scout(
        self: Arc<Self>,
        studio_id: StudioId,
        start: StartScout,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `stop_scout` — the stop on a Gathering Finding. Answered with the Studio
    /// as it stands; the Finding freezes on the stream once the scout ends.
    fn stop_scout(
        &self,
        studio_id: StudioId,
        stop: StopScout,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `start_studio_run` — run one Manifest entry in the checkout of the
    /// repository this Studio belongs to, and put a Run node on it. `#1289`.
    ///
    /// **By `Arc`, for `Commands::start_checkout_run`'s reason**: it is that
    /// run, started in the working tree as it is on disk, and the task that
    /// runs it outlives the request.
    ///
    /// **The Studio names the repository**, so this call takes none: a Studio
    /// belongs to one, and a run started from it runs there.
    ///
    /// `by` says whose act to record the node as, the way
    /// [`add_studio_node`](Studios::add_studio_node) does. Every refusal
    /// `start_checkout_run` makes is made here first, and no node is written
    /// for a run that was refused.
    fn start_studio_run(
        self: std::sync::Arc<Self>,
        studio_id: StudioId,
        run: StartStudioRun,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<StudioRunStarted, Refusal>> + Send;
}
