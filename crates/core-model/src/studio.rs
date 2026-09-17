//! A Studio: one repository's graph of what a stretch of work produced.
//! `docs/concepts/studio.md` is the authority on every set here.
//!
//! **The kind is the content's variant, never a second field**, so a node
//! cannot say it is a Note and carry a Link's address. A state is checked
//! against its kind the same way, on the way in and on the way back out.
//!
//! **No method rewrites content.** A Note is fixed at capture; a node moves,
//! and that is the only change a node's own methods offer.
//!
//! **A Run or a Job node holds a reference and no status.** Its state is read
//! off the run or the Job, so neither kind admits a state here at all.

use alloc::string::String;

use crate::envelope::{Timestamp, Ulid};
use crate::job::{id_newtype, JobId, ManifestId};

id_newtype! {
    /// A Studio's own key.
    StudioId
}
id_newtype! {
    /// One node on a Studio.
    StudioNodeId
}
id_newtype! {
    /// One edge on a Studio.
    StudioEdgeId
}

/// A Studio's name. **Never blank**: an untitled Studio has no name at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioName(String);

impl StudioName {
    /// `None` where the text is blank, trimmed of its surrounding space.
    pub fn named(text: &str) -> Option<StudioName> {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| StudioName(String::from(trimmed)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A Studio, without its graph.
///
/// **Belongs to one repository, by Manifest id** — the key Helm's conversation
/// is kept under. Nothing expires one: it is kept until a person deletes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Studio {
    pub id: StudioId,
    pub manifest_id: ManifestId,
    pub name: Option<StudioName>,
    pub created_at: Timestamp,
    /// The last write to the Studio or anything on it.
    pub touched_at: Timestamp,
}

/// A Studio and everything on it, as one read answers it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioGraph {
    pub studio: Studio,
    /// Oldest first.
    pub nodes: alloc::vec::Vec<StudioNode>,
    /// Oldest first.
    pub edges: alloc::vec::Vec<StudioEdge>,
}

/// Where a person left a node. **Whole canvas units**: placement by hand needs
/// no fraction, and an integer reads back exactly through SQLite, JSON and a
/// JavaScript number alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StudioPosition {
    pub x: i64,
    pub y: i64,
}

/// Declare a closed set spelled on the wire, with `ALL`, `as_wire` and
/// `from_wire` written once.
macro_rules! spelled {
    ($(#[$meta:meta])* $name:ident { $($(#[$v:meta])* $variant:ident => $wire:literal,)+ }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $($(#[$v])* $variant,)+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant,)+];

            pub fn as_wire(&self) -> &'static str {
                match self {
                    $($name::$variant => $wire,)+
                }
            }

            pub fn from_wire(value: &str) -> Option<$name> {
                match value {
                    $($wire => Some($name::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

spelled! {
    /// What a node is. `studio.md`, *Nodes*.
    StudioNodeKind {
        Run => "run",
        Note => "note",
        Cluster => "cluster",
        Finding => "finding",
        Contradiction => "contradiction",
        Sketch => "sketch",
        Link => "link",
        Deferral => "deferral",
        Outline => "outline",
        IssueDraft => "issue_draft",
        Job => "job",
    }
}

spelled! {
    /// Where a node stands, for the kinds that have states. A Contradiction's
    /// four outcomes are its states after `Reported`.
    StudioNodeState {
        Proposed => "proposed",
        Gathering => "gathering",
        Frozen => "frozen",
        Reported => "reported",
        IssueDraft => "issue_draft",
        Deferral => "deferral",
        NotAProblem => "not_a_problem",
        ResolvedHere => "resolved_here",
        Open => "open",
        Answered => "answered",
        Draft => "draft",
    }
}

spelled! {
    /// What an edge says. `studio.md`, *Edges*.
    StudioEdgeKind {
        /// The first node made the second. Drawn by the Studio, always.
        Produced => "produced",
        SameAs => "same_as",
        Blocks => "blocks",
        Answers => "answers",
    }
}

spelled! {
    /// A relation Helm, a scout or a person may propose: every edge kind but
    /// `Produced`, which no call can propose because this has no such variant.
    StudioRelation {
        SameAs => "same_as",
        Blocks => "blocks",
        Answers => "answers",
    }
}

spelled! {
    /// Whether a person has accepted an edge. A proposed edge is drawn dashed.
    StudioEdgeStanding {
        Proposed => "proposed",
        Accepted => "accepted",
    }
}

impl StudioNodeKind {
    /// The states a node of this kind may hold, the first being where it
    /// starts. Empty for a kind with none, and for Run and Job, whose state is
    /// the run's or the Job's own.
    pub fn states(&self) -> &'static [StudioNodeState] {
        use StudioNodeState as S;
        match self {
            StudioNodeKind::Run
            | StudioNodeKind::Note
            | StudioNodeKind::Cluster
            | StudioNodeKind::Link
            | StudioNodeKind::Job => &[],
            StudioNodeKind::Finding => &[S::Proposed, S::Gathering, S::Frozen],
            StudioNodeKind::Contradiction => &[
                S::Reported,
                S::IssueDraft,
                S::Deferral,
                S::NotAProblem,
                S::ResolvedHere,
            ],
            StudioNodeKind::Sketch => &[S::Frozen],
            StudioNodeKind::Deferral => &[S::Open, S::Answered],
            StudioNodeKind::Outline => &[S::Draft, S::Frozen],
            StudioNodeKind::IssueDraft => &[S::Draft],
        }
    }

    /// Whether `state` is one this kind holds. `None` fits only a kind with no
    /// states, so a Finding never reads back stateless.
    pub fn admits(&self, state: Option<StudioNodeState>) -> bool {
        match state {
            None => self.states().is_empty(),
            Some(state) => self.states().contains(&state),
        }
    }

    /// A kind that starts `Proposed`: the only nodes Helm adds unasked.
    pub fn starts_proposed(&self) -> bool {
        self.states().first() == Some(&StudioNodeState::Proposed)
    }
}

impl From<StudioRelation> for StudioEdgeKind {
    fn from(relation: StudioRelation) -> StudioEdgeKind {
        match relation {
            StudioRelation::SameAs => StudioEdgeKind::SameAs,
            StudioRelation::Blocks => StudioEdgeKind::Blocks,
            StudioRelation::Answers => StudioEdgeKind::Answers,
        }
    }
}

/// What a node holds, one variant per kind.
///
/// **The smallest each kind needs to be drawn and read.** A later step adds
/// what it builds beside these, never in place of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StudioNodeContent {
    /// A reference to the run, never its status or its log.
    Run { run_id: String },
    /// What a person pointed at and said, fixed at capture.
    Note { said: String },
    /// Notes a person accepted as one thing.
    Cluster { title: String },
    /// What was asked of a scout.
    Finding { asked: String },
    /// Two sources that disagree.
    Contradiction { first: String, second: String },
    /// A diagram or mockup, as text.
    Sketch { body: String },
    /// A board, document, issue, page or session, kept as its address.
    Link { address: String },
    /// Something a person put off.
    Deferral { what: String },
    /// An ordered reading of the nodes feeding it.
    Outline { body: String },
    /// An issue's title and body, never filed by Armada.
    IssueDraft { title: String, body: String },
    /// A reference to a dispatched Job, never its status.
    Job { job_id: JobId },
}

impl StudioNodeContent {
    pub fn kind(&self) -> StudioNodeKind {
        match self {
            StudioNodeContent::Run { .. } => StudioNodeKind::Run,
            StudioNodeContent::Note { .. } => StudioNodeKind::Note,
            StudioNodeContent::Cluster { .. } => StudioNodeKind::Cluster,
            StudioNodeContent::Finding { .. } => StudioNodeKind::Finding,
            StudioNodeContent::Contradiction { .. } => StudioNodeKind::Contradiction,
            StudioNodeContent::Sketch { .. } => StudioNodeKind::Sketch,
            StudioNodeContent::Link { .. } => StudioNodeKind::Link,
            StudioNodeContent::Deferral { .. } => StudioNodeKind::Deferral,
            StudioNodeContent::Outline { .. } => StudioNodeKind::Outline,
            StudioNodeContent::IssueDraft { .. } => StudioNodeKind::IssueDraft,
            StudioNodeContent::Job { .. } => StudioNodeKind::Job,
        }
    }

    /// The first field left blank, by name, or `None` where every one is said.
    pub fn blank(&self) -> Option<&'static str> {
        let fields: &[(&'static str, &str)] = match self {
            StudioNodeContent::Run { run_id } => &[("run_id", run_id)],
            StudioNodeContent::Note { said } => &[("said", said)],
            StudioNodeContent::Cluster { title } => &[("title", title)],
            StudioNodeContent::Finding { asked } => &[("asked", asked)],
            StudioNodeContent::Contradiction { first, second } => {
                &[("first", first), ("second", second)]
            }
            StudioNodeContent::Sketch { body } | StudioNodeContent::Outline { body } => {
                &[("body", body)]
            }
            StudioNodeContent::Link { address } => &[("address", address)],
            StudioNodeContent::Deferral { what } => &[("what", what)],
            StudioNodeContent::IssueDraft { title, body } => &[("title", title), ("body", body)],
            StudioNodeContent::Job { job_id } => &[("job_id", job_id.as_str())],
        };
        fields
            .iter()
            .find(|(_, text)| text.trim().is_empty())
            .map(|(name, _)| *name)
    }
}

/// A node, on one Studio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioNode {
    id: StudioNodeId,
    content: StudioNodeContent,
    state: Option<StudioNodeState>,
    position: StudioPosition,
    created_at: Timestamp,
}

/// A stored state its kind does not hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateDoesNotFit {
    pub kind: StudioNodeKind,
    pub state: Option<StudioNodeState>,
}

impl StudioNode {
    /// A node just added: in its kind's first state, or none.
    pub fn added(
        id: StudioNodeId,
        content: StudioNodeContent,
        position: StudioPosition,
        created_at: Timestamp,
    ) -> StudioNode {
        let state = content.kind().states().first().copied();
        StudioNode {
            id,
            content,
            state,
            position,
            created_at,
        }
    }

    /// A node read back, refused where its state does not fit its kind.
    pub fn recorded(
        id: StudioNodeId,
        content: StudioNodeContent,
        state: Option<StudioNodeState>,
        position: StudioPosition,
        created_at: Timestamp,
    ) -> Result<StudioNode, StateDoesNotFit> {
        let kind = content.kind();
        if !kind.admits(state) {
            return Err(StateDoesNotFit { kind, state });
        }
        Ok(StudioNode {
            id,
            content,
            state,
            position,
            created_at,
        })
    }

    /// The same node where a person left it. **The one change a node offers.**
    pub fn moved(&self, to: StudioPosition) -> StudioNode {
        StudioNode {
            position: to,
            ..self.clone()
        }
    }

    pub fn id(&self) -> &StudioNodeId {
        &self.id
    }
    pub fn kind(&self) -> StudioNodeKind {
        self.content.kind()
    }
    pub fn content(&self) -> &StudioNodeContent {
        &self.content
    }
    pub fn state(&self) -> Option<StudioNodeState> {
        self.state
    }
    pub fn position(&self) -> StudioPosition {
        self.position
    }
    pub fn created_at(&self) -> &Timestamp {
        &self.created_at
    }
}

/// An edge between two nodes on one Studio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioEdge {
    id: StudioEdgeId,
    from: StudioNodeId,
    to: StudioNodeId,
    kind: StudioEdgeKind,
    standing: StudioEdgeStanding,
    created_at: Timestamp,
}

/// Why an edge cannot be made or read back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EdgeRefused {
    /// Both ends are one node.
    ToItself { node: StudioNodeId },
    /// A `Produced` edge that is not accepted. The Studio draws those, so none
    /// is ever proposed.
    ProducedUnaccepted,
}

impl StudioEdge {
    /// A relation proposed, by whoever proposed it. Only a person accepts one.
    pub fn proposed(
        id: StudioEdgeId,
        from: StudioNodeId,
        to: StudioNodeId,
        relation: StudioRelation,
        created_at: Timestamp,
    ) -> Result<StudioEdge, EdgeRefused> {
        StudioEdge::recorded(
            id,
            from,
            to,
            relation.into(),
            StudioEdgeStanding::Proposed,
            created_at,
        )
    }

    /// The Studio's own record that `from` made `to`, accepted as drawn.
    pub fn produced(
        id: StudioEdgeId,
        from: StudioNodeId,
        to: StudioNodeId,
        created_at: Timestamp,
    ) -> Result<StudioEdge, EdgeRefused> {
        StudioEdge::recorded(
            id,
            from,
            to,
            StudioEdgeKind::Produced,
            StudioEdgeStanding::Accepted,
            created_at,
        )
    }

    /// An edge read back.
    pub fn recorded(
        id: StudioEdgeId,
        from: StudioNodeId,
        to: StudioNodeId,
        kind: StudioEdgeKind,
        standing: StudioEdgeStanding,
        created_at: Timestamp,
    ) -> Result<StudioEdge, EdgeRefused> {
        if from == to {
            return Err(EdgeRefused::ToItself { node: from });
        }
        if kind == StudioEdgeKind::Produced && standing != StudioEdgeStanding::Accepted {
            return Err(EdgeRefused::ProducedUnaccepted);
        }
        Ok(StudioEdge {
            id,
            from,
            to,
            kind,
            standing,
            created_at,
        })
    }

    pub fn id(&self) -> &StudioEdgeId {
        &self.id
    }
    pub fn from(&self) -> &StudioNodeId {
        &self.from
    }
    pub fn to(&self) -> &StudioNodeId {
        &self.to
    }
    pub fn kind(&self) -> StudioEdgeKind {
        self.kind
    }
    pub fn standing(&self) -> StudioEdgeStanding {
        self.standing
    }
    pub fn created_at(&self) -> &Timestamp {
        &self.created_at
    }
}

#[cfg(test)]
mod tests;
