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

use crate::enums::{StudioEdgeKind, StudioEdgeStanding, StudioNodeState, StudioRelation};
use crate::ids::{Instant, JobId, ManifestId, StudioEdgeId, StudioId, StudioNodeId};

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
}

/// What a node holds, tagged by its kind. `core_model::StudioNodeContent`'s
/// fields, one for one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StudioNodeContent {
    Run { run_id: String },
    Note { said: String },
    Cluster { title: String },
    Finding { asked: String },
    Contradiction { first: String, second: String },
    Sketch { body: String },
    Link { address: String },
    Deferral { what: String },
    Outline { body: String },
    IssueDraft { title: String, body: String },
    Job { job_id: JobId },
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

/// Which of the acts Helm may take on a Studio unasked it took.
/// `fleet::helm::reach::UNASKED`'s calls, one variant each.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "act", rename_all = "snake_case")]
pub enum HelmStudioAct {
    /// A node that starts proposed, by the id it was given.
    AddedNode { node_id: StudioNodeId },
    /// An edge, proposed, by the id it was given.
    ProposedEdge { edge_id: StudioEdgeId },
    /// The Studio named, and what it was named.
    Named { name: String },
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

/// `remove_studio_node`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveStudioNode {
    pub node_id: StudioNodeId,
}

/// `propose_studio_edge`. Lands `proposed`, whoever sends it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposeStudioEdge {
    pub from: StudioNodeId,
    pub to: StudioNodeId,
    pub kind: StudioRelation,
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
    pub fn of(graph: &core_model::StudioGraph) -> Studio {
        let summary = StudioSummary::of(&graph.studio);
        Studio {
            id: summary.id,
            manifest_id: summary.manifest_id,
            name: summary.name,
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
            C::Run { run_id } => StudioNodeContent::Run { run_id },
            C::Note { said } => StudioNodeContent::Note { said },
            C::Cluster { title } => StudioNodeContent::Cluster { title },
            C::Finding { asked } => StudioNodeContent::Finding { asked },
            C::Contradiction { first, second } => {
                StudioNodeContent::Contradiction { first, second }
            }
            C::Sketch { body } => StudioNodeContent::Sketch { body },
            C::Link { address } => StudioNodeContent::Link { address },
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
            StudioNodeContent::Run { run_id } => C::Run { run_id },
            StudioNodeContent::Note { said } => C::Note { said },
            StudioNodeContent::Cluster { title } => C::Cluster { title },
            StudioNodeContent::Finding { asked } => C::Finding { asked },
            StudioNodeContent::Contradiction { first, second } => {
                C::Contradiction { first, second }
            }
            StudioNodeContent::Sketch { body } => C::Sketch { body },
            StudioNodeContent::Link { address } => C::Link { address },
            StudioNodeContent::Deferral { what } => C::Deferral { what },
            StudioNodeContent::Outline { body } => C::Outline { body },
            StudioNodeContent::IssueDraft { title, body } => C::IssueDraft { title, body },
            StudioNodeContent::Job { job_id } => C::Job {
                job_id: job_id.to_domain(),
            },
        }
    }
}
