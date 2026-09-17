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
    AddStudioNode, AskScout, CaptureStudioNote, CreateStudio, DecideStudioEdge, DeferOnStudio,
    DispatchStudioDraft, EditStudioDraft, EditStudioLink, GroupStudioNodes, ManifestId,
    MoveStudioNode, ProposeStudioEdge, RemoveStudioNode, RenameStudio, SettleContradiction,
    StartScout, StartStudioRun, StopScout, Studio, StudioDeleted, StudioId, StudioList,
    StudioNodeId, StudioRunStarted, WriteUpStudioNode,
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

    /// `get_studio_frame` — the picture one Note kept, as the file itself:
    /// the name Fleet stored it under, and its bytes.
    ///
    /// **The node names it and the record holds the file name**, so nothing a
    /// caller spells reaches a path. A node that keeps none and a file that
    /// will not open are both [`Refusal::Unacceptable`], told apart by code.
    fn get_studio_frame(
        &self,
        studio_id: StudioId,
        node_id: StudioNodeId,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<(String, Vec<u8>), Refusal>> + Send;

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

    /// `capture_studio_note` — a Note fixed at where a person pointed, with
    /// its frame copied into Fleet's own keeping. **No `by`**: the route
    /// reaches no agent, so a capture is a person's.
    fn capture_studio_note(
        &self,
        studio_id: StudioId,
        capture: CaptureStudioNote,
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
    /// `group_studio_nodes` — several nodes accepted as one Cluster or read in
    /// order as one Outline, with a `produced` edge from each. `#1291`.
    fn group_studio_nodes(
        &self,
        studio_id: StudioId,
        group: GroupStudioNodes,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `defer_on_studio` — a Deferral against what it blocks. **A person's
    /// act**, so no `by`: nothing offers this to an agent.
    fn defer_on_studio(
        &self,
        studio_id: StudioId,
        deferring: DeferOnStudio,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `write_up_studio_node` — an Issue draft, and never a filed issue. `by`
    /// says whose act to publish it as, as on
    /// [`add_studio_node`](Studios::add_studio_node).
    fn write_up_studio_node(
        &self,
        studio_id: StudioId,
        writing: WriteUpStudioNode,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `edit_studio_draft` — the title and body a person left on a draft.
    fn edit_studio_draft(
        &self,
        studio_id: StudioId,
        edit: EditStudioDraft,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `edit_studio_link` — the line a person keeps beside a Link's address.
    fn edit_studio_link(
        &self,
        studio_id: StudioId,
        edit: EditStudioLink,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `settle_contradiction` — Not a problem, or Resolved here with the
    /// answer. [`Refusal::IllegalMove`] where it has already ended.
    fn settle_contradiction(
        &self,
        studio_id: StudioId,
        settling: SettleContradiction,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;

    /// `dispatch_studio_draft` — the draft's text through the Job proposer,
    /// answered with the Studio once every Job it became is a node on it.
    /// **Reads the daemon by its `Arc`**, as the scout's asks do: the proposal
    /// is a model call made while the caller waits.
    fn dispatch_studio_draft(
        self: Arc<Self>,
        studio_id: StudioId,
        dispatching: DispatchStudioDraft,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> impl Future<Output = Result<Studio, Refusal>> + Send;
}
