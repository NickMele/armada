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

mod edge;
mod finding;
mod note;
mod promotion;

use alloc::string::String;

pub use edge::{EdgeRefused, StudioEdge, ToItself};
pub use finding::{
    FrozenFinding, GatheringFinding, NotScoutable, ScoutCheckout, ScoutEnded, ScoutLook,
    ScoutOutcome, Scouted, StudioFinding,
};
pub use note::{CaptureBounds, CaptureElement, CaptureFrame, CaptureWindow, StudioCapture};
pub use promotion::{ContradictionOutcome, NotRewritable, Rewritten};

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
    /// Who gave it the name it has. `None` on an untitled Studio, and on one
    /// named before who named it was kept.
    pub named_by: Option<StudioAuthor>,
    pub created_at: Timestamp,
    /// The last write to the Studio or anything on it.
    pub touched_at: Timestamp,
}

/// A Studio and everything on it, as one read answers it, oldest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioGraph {
    pub studio: Studio,
    pub nodes: alloc::vec::Vec<StudioNode>,
    pub edges: alloc::vec::Vec<StudioEdge>,
}

/// Where a person left a node, in whole canvas units: an integer reads back
/// exactly through SQLite, JSON and a JavaScript number alike.
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
    /// Who put something on a Studio. **Helm's act is kept apart from a
    /// person's on the record itself**, not only on the stream —
    /// `docs/concepts/helm.md`, *Audit trail*.
    StudioAuthor {
        Person => "person",
        Helm => "helm",
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

    /// A kind a person may put on a Studio by hand — `#1364`, decided with the
    /// owner. A Note typed here, a Link pasted, a Sketch placed.
    ///
    /// **Every other kind is made by the act that earns it**, and adding one by
    /// hand would be a claim nothing stands behind: a Finding comes from a
    /// scout, a Run from a run, a Cluster or a Deferral from promotion, an
    /// Issue draft from writing up, a Job from dispatch, a Contradiction from
    /// two sources read in. `docs/concepts/studio.md`, *Promotion*.
    pub fn added_by_hand(&self) -> bool {
        matches!(
            self,
            StudioNodeKind::Note | StudioNodeKind::Link | StudioNodeKind::Sketch
        )
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

/// What a Run node keeps of its run once the run's own retention has swept it:
/// the result, and the log's last lines. `#1289`.
///
/// **Only ever present on a run that is gone.** While the run's record is
/// still on disk the node is a reference and nothing else, and its state is
/// read off the run — so a node carrying one of these is saying that what is
/// here is all there is, which is what *partial* means on a Studio.
///
/// **Taken before the sweep, never after.** A tail read after the directory
/// was removed is no tail at all, and the node would point at nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioRunKept {
    /// The entry the Manifest declared, by name.
    pub name: String,
    /// The command as it ran.
    pub command: String,
    /// `None` where the run was killed before it exited.
    pub exit_code: Option<i32>,
    /// What the entry declared a pass to be, so the node keeps its colour.
    pub expect_exit_code: i64,
    /// Whether a person stopped it.
    pub stopped: bool,
    pub duration_ms: u64,
    /// The log's last lines, oldest first, bounded by the Studio's own bound
    /// rather than by whatever the command printed.
    pub lines: alloc::vec::Vec<String>,
    /// How many lines the log held in all, whether or not they are here.
    pub total_lines: u32,
    /// Whether [`lines`](StudioRunKept::lines) is the whole log rather than
    /// its tail.
    pub whole: bool,
}

/// What a node holds, one variant per kind.
///
/// **The smallest each kind needs to be drawn and read.** A later step adds
/// what it builds beside these, never in place of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StudioNodeContent {
    /// A reference to the run, never its status — and what was kept of it
    /// once retention swept the run away. `#1289`.
    Run {
        run_id: String,
        kept: Option<StudioRunKept>,
    },
    /// What a person pointed at and said, fixed at capture. `capture` is
    /// where they pointed — absent on a Note added before `#1290`, and on one
    /// typed rather than pointed.
    Note {
        said: String,
        capture: Option<StudioCapture>,
    },
    /// Notes a person accepted as one thing.
    Cluster { title: String },
    /// What a scout was asked, and what it read.
    Finding(StudioFinding),
    /// Two sources that disagree, and the answer where a person settled it
    /// here. `answer` is absent until then, and on the three outcomes that
    /// record what was decided somewhere else.
    Contradiction {
        first: String,
        second: String,
        answer: Option<String>,
    },
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
            StudioNodeContent::Finding(_) => StudioNodeKind::Finding,
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
            StudioNodeContent::Run { run_id, .. } => &[("run_id", run_id)],
            StudioNodeContent::Note { said, .. } => &[("said", said)],
            StudioNodeContent::Cluster { title } => &[("title", title)],
            StudioNodeContent::Finding(finding) => &[("asked", finding.ask())],
            StudioNodeContent::Contradiction { first, second, .. } => {
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

    /// The run this node references, where it is a Run node whose run is still
    /// the thing to read. `None` on every other kind, **and on a Run that has
    /// already kept its tail**: what it references is gone.
    pub fn run_still_read(&self) -> Option<&str> {
        match self {
            StudioNodeContent::Run { run_id, kept: None } => Some(run_id),
            _ => None,
        }
    }

    /// This content with what was kept of its run written into it.
    ///
    /// **The only method here that makes new content**, and the reason a Note
    /// stays fixed at capture: it takes a Run node whose run is about to be
    /// swept and no other, so nothing can reach a node's words through it, and
    /// a tail already kept is never written over.
    pub fn keeping(&self, kept: StudioRunKept) -> Option<StudioNodeContent> {
        let run_id = self.run_still_read()?;
        Some(StudioNodeContent::Run {
            run_id: String::from(run_id),
            kept: Some(kept),
        })
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
    /// `None` only on a node added before who added it was kept.
    added_by: Option<StudioAuthor>,
}

/// A stored state its kind does not hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateDoesNotFit {
    pub kind: StudioNodeKind,
    pub state: Option<StudioNodeState>,
}

impl StudioNode {
    /// A node just added by `by`: in its kind's first state, or none.
    pub fn added(
        id: StudioNodeId,
        content: StudioNodeContent,
        position: StudioPosition,
        created_at: Timestamp,
        by: StudioAuthor,
    ) -> StudioNode {
        let state = content.kind().states().first().copied();
        StudioNode {
            id,
            content,
            state,
            position,
            created_at,
            added_by: Some(by),
        }
    }

    /// A node read back, refused where its state does not fit its kind, or a
    /// Finding's content does not fit its state.
    pub fn recorded(
        id: StudioNodeId,
        content: StudioNodeContent,
        state: Option<StudioNodeState>,
        position: StudioPosition,
        created_at: Timestamp,
        added_by: Option<StudioAuthor>,
    ) -> Result<StudioNode, StateDoesNotFit> {
        let kind = content.kind();
        let finding_fits = match &content {
            StudioNodeContent::Finding(finding) => finding.fits(state),
            _ => true,
        };
        if !kind.admits(state) || !finding_fits {
            return Err(StateDoesNotFit { kind, state });
        }
        Ok(StudioNode {
            id,
            content,
            state,
            position,
            created_at,
            added_by,
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
    pub fn added_by(&self) -> Option<StudioAuthor> {
        self.added_by
    }
}

#[cfg(test)]
mod tests;
