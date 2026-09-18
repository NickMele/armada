//! A Studio on the wire: the list, one Studio with its graph, and the acts a
//! client asks of one. `#1285`, `docs/concepts/studio.md`.
//!
//! **A node's `kind` is its content's tag**, flat beside its fields, so a
//! client switches on one field and reads the rest by it. The tag is spelled
//! by serde here and by `core_model` there; `crate::tests::studio` holds the
//! two to one spelling per kind.
//!
//! **Answered whole after every write**, the way `save_preferences` is: a
//! client replaces the Studio it holds rather than folding a change into it.

use serde::{Deserialize, Serialize};

use crate::capturing::StudioCapture;
use crate::enums::{
    StudioAuthor, StudioEdgeKind, StudioEdgeStanding, StudioNodeState, StudioRelation,
};
use crate::ids::{Instant, JobId, ManifestId, StudioEdgeId, StudioId, StudioNodeId};
use crate::rehearsal::CheckoutRunUnderway;
use crate::scouting::{ScoutCheckout, ScoutEnded, ScoutSource};
use crate::servers::ServerState;

/// Every Studio one repository keeps, the last touched first — `list_studios`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioList {
    pub studios: Vec<StudioSummary>,
}

/// One Studio as the list names it: what it is called and when it was last
/// touched. **No Workspace**, `studio.md`'s rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioSummary {
    pub id: StudioId,
    pub manifest_id: ManifestId,
    /// Absent on a Studio nobody has named yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub created_at: Instant,
    pub touched_at: Instant,
}

/// One Studio and its whole graph — `get_studio`, the answer to every write
/// on one, and the body of `studio.changed`.
///
/// **`manifest_id` sits at the top level** so the stream's own reading of which
/// repository an event is about finds it without knowing this shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Studio {
    pub id: StudioId,
    pub manifest_id: ManifestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Who gave it its name. Absent on an untitled Studio, and on one named
    /// before who named it was kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named_by: Option<StudioAuthor>,
    pub created_at: Instant,
    pub touched_at: Instant,
    /// Oldest first.
    pub nodes: Vec<StudioNode>,
    /// Oldest first.
    pub edges: Vec<StudioEdge>,
}

/// One node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioNode {
    pub id: StudioNodeId,
    #[serde(flatten)]
    pub content: StudioNodeContent,
    /// Absent on a kind with no states, and always on a Run or a Job, whose
    /// state is read off the run or the Job and never copied here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<StudioNodeState>,
    pub position: StudioPosition,
    pub created_at: Instant,
    /// A person or Helm. Absent only on a node added before it was kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added_by: Option<StudioAuthor>,
}

/// What a node holds, tagged by its kind. `core_model::StudioNodeContent`'s
/// fields, one for one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StudioNodeContent {
    Run {
        /// The run's id, or a server instance's. Which of the two is [`held`].
        ///
        /// [`held`]: StudioNodeContent::Run::held
        run_id: String,
        /// Who holds what `run_id` names. **Absent is a run in the checkout**,
        /// which is every node written before 16.2 — and the reader for each
        /// is a different one, so a client that does not know this field asks
        /// the run reader about a server and is told there is no such run.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        held: Option<StudioRunHeld>,
        /// Absent while it is still there to read. Present is **partial**: the
        /// run's own record is gone and this is all there is.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kept: Option<StudioRunKept>,
    },
    Note {
        said: String,
        /// Where a person pointed, `#1290`. Absent on a Note that was typed
        /// rather than pointed, and on one kept before capture existed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capture: Option<StudioCapture>,
    },
    Cluster {
        title: String,
    },
    /// What a scout was asked, and from its start what it read. `#1292`.
    Finding {
        asked: String,
        /// The commit read, and whether anything uncommitted was on top of
        /// it. Absent until the scout starts.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        checkout: Option<ScoutCheckout>,
        /// Every source Fleet fetched and handed it beyond the checkout, in
        /// the order handed. Empty on a scout asked about the code. `#1293`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        sources: Vec<ScoutSource>,
        /// Every file read, relative to the checkout, in the order first read —
        /// a file a search returned lines of included.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        read: Vec<String>,
        /// Every search run, as its pattern and where it looked.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        searched: Vec<String>,
        /// What the scout said last.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        learned: Option<String>,
        /// How it ended, and what it cost. Absent until it ends.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ended: Option<ScoutEnded>,
    },
    /// Two sources that disagree, and the answer where a person settled it
    /// here. `#1291`.
    Contradiction {
        first: String,
        second: String,
        /// Absent until a person ends it as *Resolved here*, and on the three
        /// outcomes that record what was decided somewhere else.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        answer: Option<String>,
    },
    Sketch {
        body: String,
    },
    Link {
        address: String,
        /// The line a person wrote beside the address, saying why they kept
        /// it — `#1378`. **Additional, never a replacement**: a Link never
        /// stops being its address. Absent on one pasted with nothing typed,
        /// and on every Link kept before the field existed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        said: Option<String>,
        /// What the source calls itself, where a read-in learned it — an
        /// issue's number, title and state on one line. **Beside the address
        /// and never in place of it**, and never over `said`, which is a
        /// person's own. `#1293`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        named: Option<String>,
    },
    /// An issue on a forge — `#1394`. `address` and `number` were read off the
    /// address when the node was made; `title` and `state` were read off the
    /// forge, so both are absent until it has been read in.
    Issue {
        address: String,
        number: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        said: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<ForgeState>,
    },
    /// A pull request on a forge. An Issue's fields, and `state` holds the one
    /// an issue cannot: `merged`.
    PullRequest {
        address: String,
        number: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        said: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<ForgeState>,
    },
    /// What a forge calls a set of issues under one address. **No state and a
    /// count instead**: what matters about an Epic is how much of it is on the
    /// Studio, which is what reading it in answers.
    Epic {
        address: String,
        number: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        said: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        read_in: Option<EpicRead>,
    },
    Deferral {
        what: String,
    },
    Outline {
        body: String,
    },
    IssueDraft {
        title: String,
        body: String,
    },
    Job {
        job_id: JobId,
    },
}

/// Where something on a forge stands — `core_model::ForgeState`, one for one.
/// `#1394`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgeState {
    Open,
    Closed,
    /// A pull request's own end. An issue never holds it.
    Merged,
}

/// Which of an Epic's issues a read-in takes — `core_model::EpicTake`, one for
/// one. `#1405`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpicTake {
    /// Every issue the Epic holds, whatever state it is in.
    Everything,
    /// Only what the forge says is open.
    Open,
}

/// How much of an Epic is on the Studio — `core_model::EpicRead`, one for one.
/// `#1405`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpicRead {
    pub issues: u64,
    pub total: u64,
    /// Absent on an Epic read in before `#1405`, whose answer nothing recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took: Option<EpicTake>,
    /// How many of its issues the answer left out.
    #[serde(default)]
    pub left_out: u64,
    /// How many nodes the read left standing because a person had worked on
    /// them.
    #[serde(default)]
    pub kept: u64,
    /// Where the read-in laid its issues out. Absent on an Epic read in before
    /// `#1405`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laid_out_from: Option<StudioPosition>,
}

/// What a Run node kept of its run, once retention swept the run away —
/// `core_model::StudioRunKept`'s fields, one for one. `#1289`.
///
/// **A node carrying one is partial, and that is what says so.** There is no
/// second flag: while the run is there the node is a reference and its state
/// is read off the run, and this is what is left when it is not.
///
/// **The colour survives with it.** `exit_code`, `expect_exit_code` and
/// `stopped` are the three a run's colour is derived from (`#1304`), so a
/// swept run reads failed on the whiteboard the same as it did while it ran.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioRunKept {
    pub name: String,
    pub command: String,
    /// Absent where the run was killed before it exited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub expect_exit_code: i64,
    pub stopped: bool,
    pub duration_ms: u64,
    /// The log's last lines, oldest first.
    pub lines: Vec<String>,
    /// How many lines the log held in all, whether or not they are here.
    pub total_lines: u32,
    /// Whether `lines` is the whole log rather than its tail.
    pub whole: bool,
}

/// Who holds what a Run node's `run_id` names. `#1345`.
///
/// **Two readers, never one.** A checkout run is read from `.armada/runs` by
/// `observe_checkout_run`; a server is read from Fleet's own memory by
/// `observe_server`, `list_servers` and `stop_server`. Nothing can be asked of
/// both, so which it is rides on the node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudioRunHeld {
    /// A Check or a Command that runs and exits. **The default**, so a node
    /// written before 16.2 reads back as what it is.
    #[default]
    Checkout,
    /// A Command with `serve`, held by Fleet against the main checkout.
    Server,
}

/// `start_studio_server`: start a Command with `serve` in the checkout the
/// Studio's repository stands in, and put a Run node on the Studio for it.
/// `#1345`.
///
/// **`start_studio_run`'s body without a workspace.** A server is declared in
/// the Manifest Fleet holds for the repository, and `start_server` takes a
/// name and no directory for the same reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartStudioServer {
    /// The Command with `serve` the Manifest declares, by name.
    pub name: String,
    /// Where the Run node is placed.
    pub position: StudioPosition,
    /// The node this was started from. The Studio draws the `produced` edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced_by: Option<StudioNodeId>,
}

/// `start_studio_server`'s answer: the server is up or coming up, and the
/// Studio holds a node for it. `#1345`.
///
/// **The instance, live, and never a copy of it.** What the node draws is read
/// off `list_servers` from then on; this is the first read, in the answer, so
/// a client does not have to wait for the next event to draw the node it just
/// made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioServerStarted {
    pub studio: Studio,
    pub node_id: StudioNodeId,
    pub server: ServerState,
    /// **One instance per checkout**, so an ask for a server already serving
    /// is answered with that one and this says so. The node is still made: it
    /// records that this Studio asked for it.
    pub already_up: bool,
}

/// `start_studio_run`: run one Manifest entry in the checkout and put a Run
/// node on the Studio for it.
///
/// **No repository**: the Studio names it, being a repository's own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartStudioRun {
    /// The Check or Command the Manifest declares, by name.
    pub name: String,
    /// A directory below the repository root whose own `armada.yml` declares
    /// `name`, run in that directory. Absent, empty or `.` is the root's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    /// Where the Run node is placed.
    pub position: StudioPosition,
    /// The node this run was started from. The Studio draws the `produced`
    /// edge itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced_by: Option<StudioNodeId>,
}

/// `start_studio_run`'s answer: the run is underway, and the Studio holds a
/// node for it.
///
/// **The Studio whole, as every write on one answers**, and the node's id
/// beside it so a client knows which of them is new without diffing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioRunStarted {
    pub studio: Studio,
    pub node_id: StudioNodeId,
    pub run: CheckoutRunUnderway,
}

/// Where a person left a node, in whole canvas units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioPosition {
    pub x: i64,
    pub y: i64,
}

/// One edge. A `proposed` one is drawn dashed until a person accepts it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioEdge {
    pub id: StudioEdgeId,
    pub from: StudioNodeId,
    pub to: StudioNodeId,
    pub kind: StudioEdgeKind,
    pub standing: StudioEdgeStanding,
    pub created_at: Instant,
    /// A person or Helm. Absent only on an edge kept before it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added_by: Option<StudioAuthor>,
}

/// A Studio that is gone — `delete_studio`'s answer and `studio.deleted`'s body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioDeleted {
    pub id: StudioId,
    pub manifest_id: ManifestId,
}

/// `studio.helm_acted`: one act Helm took on a Studio. **Helm's act as its own
/// event type** — `docs/concepts/helm.md`, *Audit trail*. The write publishes
/// `studio.changed` as any write does, and this besides only where the door
/// placed the call in a Helm session, so a person's act and Helm's are told
/// apart by kind.
///
/// **Ids, not the Studio.** What the Studio holds now is `studio.changed`'s to
/// carry; this says who did which part of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioHelmActed {
    pub studio_id: StudioId,
    pub manifest_id: ManifestId,
    #[serde(flatten)]
    pub act: HelmStudioAct,
    pub at: Instant,
}

/// Which act on a Studio Helm took. **Every act Helm takes, not only the
/// unasked ones** — `docs/concepts/studio.md`, *Helm on a Studio*: the three
/// `fleet::helm::reach::UNASKED` calls, and the two it takes on a person's
/// ask.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "act", rename_all = "snake_case")]
pub enum HelmStudioAct {
    /// A node that starts proposed, by the id it was given.
    AddedNode { node_id: StudioNodeId },
    /// An edge, proposed, by the id it was given.
    ProposedEdge { edge_id: StudioEdgeId },
    /// The Studio named, and what it was named.
    Named { name: String },
    /// A node written up, and the Issue draft it produced. `#1291`.
    WroteUp {
        from: StudioNodeId,
        node_id: StudioNodeId,
    },
    /// A Link read in, and every node it produced. `#1293`.
    ReadIn {
        from: StudioNodeId,
        node_ids: Vec<StudioNodeId>,
    },
    /// An Issue draft dispatched, and every Job node the proposal put on the
    /// Studio. **A list**, because one request can be several Jobs. `#1291`.
    Dispatched {
        from: StudioNodeId,
        node_ids: Vec<StudioNodeId>,
    },
}

/// `create_studio`. **A name is not required**: `studio.md` has Helm name an
/// untitled Studio.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateStudio {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `rename_studio`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameStudio {
    pub name: String,
}

/// `add_studio_node`. **Nothing here sets a state**: a node starts in its
/// kind's first one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddStudioNode {
    #[serde(flatten)]
    pub content: StudioNodeContent,
    pub position: StudioPosition,
    /// The node on this Studio that made this one. The Studio draws the
    /// `produced` edge itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced_by: Option<StudioNodeId>,
}

/// `move_studio_node`. **Position only**: no act on this seam writes a node's
/// content after it is added.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveStudioNode {
    pub node_id: StudioNodeId,
    pub position: StudioPosition,
}

/// `remove_studio_nodes`: everything a person picked, deleted as one write.
/// `#1411`.
///
/// **A list and not a repeated call.** Eighteen calls can stop at the ninth,
/// and what is left is a board a person has to reconcile by reading it; one
/// call is all of them or none. Naming no node is refused rather than taken as
/// a write that does nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveStudioNodes {
    pub node_ids: Vec<StudioNodeId>,
}

/// `propose_studio_edge`. Lands `proposed`, whoever sends it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposeStudioEdge {
    pub from: StudioNodeId,
    pub to: StudioNodeId,
    pub kind: StudioRelation,
}

/// `group_studio_nodes`: several nodes on a Studio accepted as one. **A
/// Cluster or an Outline and nothing else** — every other kind is added by the
/// rung that makes it. `#1291`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupStudioNodes {
    #[serde(flatten)]
    pub content: StudioNodeContent,
    /// The nodes it is made of, **in the order they were given**: an Outline
    /// is an ordered reading, and the `produced` edges are kept in this order.
    pub from: Vec<StudioNodeId>,
    pub position: StudioPosition,
}

/// `defer_on_studio`: something raised on a node, put off. **Only a person
/// defers**, whatever Helm is asked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeferOnStudio {
    /// What is being put off, in the person's words.
    pub what: String,
    /// The node it was raised on. The Studio draws the `produced` edge.
    pub raised_on: StudioNodeId,
    /// What it holds up, where it holds anything up. The Studio draws an
    /// accepted `blocks` edge from the Deferral to it — accepted, because the
    /// person drawing it is the one who accepts a relation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocks: Option<StudioNodeId>,
    pub position: StudioPosition,
}

/// `write_up_studio_node`: a Note, Cluster, Contradiction or Outline written up
/// as an Issue draft. **Never filed anywhere** — an issue on a forge is a
/// person's own act afterwards.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteUpStudioNode {
    pub node_id: StudioNodeId,
    pub title: String,
    pub body: String,
    pub position: StudioPosition,
}

/// `edit_studio_draft`: the title and body of an Issue draft, as a person left
/// them. **The one act that rewrites a node a person wrote**, and it reaches
/// no other kind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditStudioDraft {
    pub node_id: StudioNodeId,
    pub title: String,
    pub body: String,
}

/// `edit_studio_link`: the line a person keeps beside a Link's address, as
/// they left it — `#1378`.
///
/// **The address is not on this request.** A Link never stops being its
/// address, so nothing on this seam can rewrite one; what is editable is the
/// line beside it. A blank `said` clears the line, which is how somebody takes
/// back what they typed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditStudioLink {
    pub node_id: StudioNodeId,
    pub said: String,
}

/// `settle_contradiction`: the two outcomes that write nothing else down.
///
/// **The other two are the rungs that make a node**: *Issue draft* is
/// `write_up_studio_node` on the Contradiction, and *Deferral* is
/// `defer_on_studio` raised on it, each moving the Contradiction itself. So
/// there is one way to make an Issue draft and one way to make a Deferral,
/// rather than two.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettleContradiction {
    pub node_id: StudioNodeId,
    #[serde(flatten)]
    pub outcome: ContradictionSettled,
}

/// Which of the two. `outcome` is the tag the answer hangs off.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ContradictionSettled {
    /// Both statements hold, in different contexts.
    NotAProblem,
    /// The person settled it, and the node records the answer.
    ResolvedHere { answer: String },
}

/// `dispatch_studio_draft`: an Issue draft's text, through the Job proposer, to
/// the ordinary dispatch gate.
///
/// **No field carries a workflow and none carries an issue.** The draft's text
/// is the whole request, exactly as `propose_from_request` carries one, and
/// every Job it becomes stands at `awaiting_approval` like any other.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchStudioDraft {
    pub node_id: StudioNodeId,
    /// Where the first Job node is placed. A split's later Jobs are placed
    /// below it, so a person sees all of them without moving anything.
    pub position: StudioPosition,
}

/// `decide_studio_edge`: accept a proposed edge, or reject it, which removes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecideStudioEdge {
    pub edge_id: StudioEdgeId,
    pub accepted: bool,
}

impl StudioSummary {
    pub fn of(studio: &core_model::Studio) -> StudioSummary {
        StudioSummary {
            id: StudioId::from(&studio.id),
            manifest_id: ManifestId::from(&studio.manifest_id),
            name: studio.name.as_ref().map(|name| name.as_str().to_string()),
            created_at: Instant::from(&studio.created_at),
            touched_at: Instant::from(&studio.touched_at),
        }
    }
}

impl Studio {
    /// The Studio on the wire.
    ///
    /// **Nothing is worked out here any more.** What a node's address names
    /// was read once by `crates/adapters`, when the node was made, and is the
    /// node's own kind — `#1394`. Until then this took a classifier as a
    /// parameter so that no sender could forget to say.
    pub fn of(graph: &core_model::StudioGraph) -> Studio {
        let summary = StudioSummary::of(&graph.studio);
        Studio {
            id: summary.id,
            manifest_id: summary.manifest_id,
            name: summary.name,
            named_by: graph.studio.named_by.map(StudioAuthor::from),
            created_at: summary.created_at,
            touched_at: summary.touched_at,
            nodes: graph.nodes.iter().map(StudioNode::of).collect(),
            edges: graph.edges.iter().map(StudioEdge::of).collect(),
        }
    }
}

impl StudioNode {
    pub fn of(node: &core_model::StudioNode) -> StudioNode {
        StudioNode {
            id: StudioNodeId::from(node.id()),
            content: StudioNodeContent::from(node.content()),
            state: node.state().map(StudioNodeState::from),
            position: StudioPosition::from(node.position()),
            created_at: Instant::from(node.created_at()),
            added_by: node.added_by().map(StudioAuthor::from),
        }
    }
}

impl StudioEdge {
    pub fn of(edge: &core_model::StudioEdge) -> StudioEdge {
        StudioEdge {
            id: StudioEdgeId::from(edge.id()),
            from: StudioNodeId::from(edge.from()),
            to: StudioNodeId::from(edge.to()),
            kind: StudioEdgeKind::from(edge.kind()),
            standing: StudioEdgeStanding::from(edge.standing()),
            created_at: Instant::from(edge.created_at()),
            added_by: edge.added_by().map(StudioAuthor::from),
        }
    }
}

impl StudioRunKept {
    pub fn of(kept: &core_model::StudioRunKept) -> StudioRunKept {
        StudioRunKept {
            name: kept.name.clone(),
            command: kept.command.clone(),
            exit_code: kept.exit_code,
            expect_exit_code: kept.expect_exit_code,
            stopped: kept.stopped,
            duration_ms: kept.duration_ms,
            lines: kept.lines.clone(),
            total_lines: kept.total_lines,
            whole: kept.whole,
        }
    }
}

/// One of the three forge kinds, as the domain holds it. **Built through
/// `on_the_forge` and `resolved`**, so this crate never spells a variant of
/// the domain's own set and a kind that is not one of the three cannot be
/// reached from here at all.
fn forge_content(
    kind: core_model::StudioNodeKind,
    address: String,
    number: String,
    said: Option<String>,
    title: Option<String>,
    state: Option<ForgeState>,
    read_in: Option<EpicRead>,
) -> core_model::StudioNodeContent {
    let made = core_model::StudioNodeContent::on_the_forge(kind, address, number, said)
        .expect("one of the three forge kinds");
    made.resolved(&core_model::ForgeFacts {
        title,
        state: state.map(ForgeState::to_domain),
        read_in: read_in.map(EpicRead::to_domain),
    })
    .expect("one of the three forge kinds")
}

impl From<core_model::ForgeState> for ForgeState {
    fn from(state: core_model::ForgeState) -> ForgeState {
        match state {
            core_model::ForgeState::Open => ForgeState::Open,
            core_model::ForgeState::Closed => ForgeState::Closed,
            core_model::ForgeState::Merged => ForgeState::Merged,
        }
    }
}

impl ForgeState {
    pub fn to_domain(self) -> core_model::ForgeState {
        match self {
            ForgeState::Open => core_model::ForgeState::Open,
            ForgeState::Closed => core_model::ForgeState::Closed,
            ForgeState::Merged => core_model::ForgeState::Merged,
        }
    }
}

impl From<core_model::EpicTake> for EpicTake {
    fn from(take: core_model::EpicTake) -> EpicTake {
        match take {
            core_model::EpicTake::Everything => EpicTake::Everything,
            core_model::EpicTake::Open => EpicTake::Open,
        }
    }
}

impl EpicTake {
    pub fn to_domain(self) -> core_model::EpicTake {
        match self {
            EpicTake::Everything => core_model::EpicTake::Everything,
            EpicTake::Open => core_model::EpicTake::Open,
        }
    }
}

impl From<core_model::EpicRead> for EpicRead {
    fn from(read: core_model::EpicRead) -> EpicRead {
        EpicRead {
            issues: read.issues,
            total: read.total,
            took: read.took.map(EpicTake::from),
            left_out: read.left_out,
            kept: read.kept,
            laid_out_from: read.laid_out_from.map(StudioPosition::from),
        }
    }
}

impl EpicRead {
    pub fn to_domain(self) -> core_model::EpicRead {
        core_model::EpicRead {
            issues: self.issues,
            total: self.total,
            took: self.took.map(EpicTake::to_domain),
            left_out: self.left_out,
            kept: self.kept,
            laid_out_from: self.laid_out_from.map(StudioPosition::to_domain),
        }
    }
}

impl From<core_model::StudioPosition> for StudioPosition {
    fn from(at: core_model::StudioPosition) -> StudioPosition {
        StudioPosition { x: at.x, y: at.y }
    }
}

impl StudioPosition {
    pub fn to_domain(self) -> core_model::StudioPosition {
        core_model::StudioPosition {
            x: self.x,
            y: self.y,
        }
    }
}

impl From<&core_model::StudioNodeContent> for StudioNodeContent {
    fn from(content: &core_model::StudioNodeContent) -> StudioNodeContent {
        use core_model::StudioNodeContent as C;
        match content.clone() {
            C::Run { run, kept } => StudioNodeContent::Run {
                run_id: run.id().to_string(),
                held: match run {
                    core_model::StudioRun::Checkout(_) => None,
                    core_model::StudioRun::Server(_) => Some(StudioRunHeld::Server),
                },
                kept: kept.as_ref().map(StudioRunKept::of),
            },
            C::Note { said, capture } => StudioNodeContent::Note {
                said,
                capture: capture.as_ref().map(StudioCapture::from),
            },
            C::Cluster { title } => StudioNodeContent::Cluster { title },
            C::Finding(finding) => crate::scouting::finding_on_the_wire(&finding),
            C::Contradiction {
                first,
                second,
                answer,
            } => StudioNodeContent::Contradiction {
                first,
                second,
                answer,
            },
            C::Sketch { body } => StudioNodeContent::Sketch { body },
            C::Link {
                address,
                said,
                named,
            } => StudioNodeContent::Link {
                address,
                said,
                named,
            },
            C::Issue {
                address,
                number,
                said,
                title,
                state,
            } => StudioNodeContent::Issue {
                address,
                number,
                said,
                title,
                state: state.map(ForgeState::from),
            },
            C::PullRequest {
                address,
                number,
                said,
                title,
                state,
            } => StudioNodeContent::PullRequest {
                address,
                number,
                said,
                title,
                state: state.map(ForgeState::from),
            },
            C::Epic {
                address,
                number,
                said,
                title,
                read_in,
            } => StudioNodeContent::Epic {
                address,
                number,
                said,
                title,
                read_in: read_in.map(EpicRead::from),
            },
            C::Deferral { what } => StudioNodeContent::Deferral { what },
            C::Outline { body } => StudioNodeContent::Outline { body },
            C::IssueDraft { title, body } => StudioNodeContent::IssueDraft { title, body },
            C::Job { job_id } => StudioNodeContent::Job {
                job_id: JobId::from(&job_id),
            },
        }
    }
}

impl StudioNodeContent {
    /// The content an arriving request names, as the domain holds it.
    pub fn to_domain(&self) -> core_model::StudioNodeContent {
        use core_model::StudioNodeContent as C;
        match self.clone() {
            // **`kept` never decodes into a write.** What a node keeps of a
            // swept run is taken from the run's own record by the sweep, and
            // `add_studio_node` refuses a Run kind outright, so nothing on
            // this seam can name a result the run did not have.
            StudioNodeContent::Run { run_id, held, .. } => C::Run {
                run: match held {
                    None | Some(StudioRunHeld::Checkout) => core_model::StudioRun::Checkout(run_id),
                    Some(StudioRunHeld::Server) => core_model::StudioRun::Server(run_id),
                },
                kept: None,
            },
            StudioNodeContent::Note { said, capture } => C::Note {
                said,
                capture: capture.map(StudioCapture::to_domain),
            },
            StudioNodeContent::Cluster { title } => C::Cluster { title },
            StudioNodeContent::Finding {
                asked,
                checkout,
                sources,
                read,
                searched,
                learned,
                ended,
            } => C::Finding(core_model::StudioFinding::recorded(
                asked,
                checkout.map(ScoutCheckout::to_domain),
                sources.into_iter().map(ScoutSource::to_domain).collect(),
                read,
                searched,
                learned,
                ended.map(ScoutEnded::to_domain),
            )),
            StudioNodeContent::Contradiction {
                first,
                second,
                answer,
            } => C::Contradiction {
                first,
                second,
                answer,
            },
            StudioNodeContent::Sketch { body } => C::Sketch { body },
            // The line is trimmed here, and a blank one is no line at all.
            // **`named` does not decode into a write**: what a source calls
            // itself is a read-in's to record, so a request naming one is
            // dropped the way a Run's `kept` is.
            StudioNodeContent::Link { address, said, .. } => C::link(address, said),
            // **The number does not decode into a write either.** What an
            // address names on a forge is `crates/adapters`' reading, made
            // once when the node was made, so a request naming a kind and a
            // number is refused before it reaches here — `fleet::studios`
            // classifies what arrives. Reconstructing the content keeps this
            // total, and the fields a read-in filled in survive it.
            StudioNodeContent::Issue {
                address,
                number,
                said,
                title,
                state,
            } => forge_content(
                core_model::StudioNodeKind::Issue,
                address,
                number,
                said,
                title,
                state,
                None,
            ),
            StudioNodeContent::PullRequest {
                address,
                number,
                said,
                title,
                state,
            } => forge_content(
                core_model::StudioNodeKind::PullRequest,
                address,
                number,
                said,
                title,
                state,
                None,
            ),
            StudioNodeContent::Epic {
                address,
                number,
                said,
                title,
                read_in,
            } => forge_content(
                core_model::StudioNodeKind::Epic,
                address,
                number,
                said,
                title,
                None,
                read_in,
            ),
            StudioNodeContent::Deferral { what } => C::Deferral { what },
            StudioNodeContent::Outline { body } => C::Outline { body },
            StudioNodeContent::IssueDraft { title, body } => C::IssueDraft { title, body },
            StudioNodeContent::Job { job_id } => C::Job {
                job_id: job_id.to_domain(),
            },
        }
    }
}
