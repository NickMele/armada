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
mod forge;
mod note;
mod promotion;

use alloc::string::String;

pub use edge::{EdgeRefused, StudioEdge, ToItself};
pub use finding::{
    FrozenFinding, GatheringFinding, NotScoutable, ScoutCheckout, ScoutEnded, ScoutLook,
    ScoutOutcome, ScoutSource, Scouted, StudioFinding,
};
pub use forge::{EpicRead, ForgeFacts, Recognised};
pub use note::{
    CaptureBounds, CaptureElement, CaptureFrame, CaptureServed, CaptureWindow, StudioCapture,
};
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
        /// An issue on a forge. **The concept, never the vendor** — which
        /// forge served the address is `adapters`' to know, and the node is an
        /// Issue whoever hosts it. `#1394`.
        Issue => "issue",
        /// A pull request on a forge.
        PullRequest => "pull_request",
        /// A milestone, an epic, a wave — whatever a forge calls the thing one
        /// address names that holds a set of issues. **Epic is the node**; the
        /// `epic` workflow is what dispatching one may run, and the Job
        /// proposer decides that.
        Epic => "epic",
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
    /// What a source a scout was handed was read as. `#1293`,
    /// `docs/concepts/scout.md`, *What it may read*.
    ///
    /// **Named for what it is, never for whose it is.** Which host serves an
    /// issue, which agent wrote a session and which daemon keeps a thread are
    /// `adapters`' to know; a Studio records that a scout read an issue.
    ScoutSourceKind {
        /// An issue on the repository's forge.
        Issue => "issue",
        /// A pull request on the repository's forge.
        PullRequest => "pull_request",
        /// A milestone on the forge, read in as its issues. Fleet's own read,
        /// with no scout: a list echoed back by a model is cost spent on a
        /// transcription.
        Milestone => "milestone",
        /// A page on the web, as its headings and text.
        Page => "page",
        /// An agent session started in this repository or one of its worktrees.
        Session => "session",
        /// This repository's own Helm thread.
        Thread => "thread",
    }
}

spelled! {
    /// Where something on a forge stands, as an Issue or a Pull request node
    /// carries it. `#1394`.
    ///
    /// **Three, and each is the concept rather than a forge's spelling.** A
    /// forge that says `OPEN`, `open` or `Opened` is read as the same word by
    /// `adapters`, which is the only crate that sees one.
    ForgeState {
        Open => "open",
        Closed => "closed",
        /// A pull request's own end. An issue never holds it.
        Merged => "merged",
    }
}

spelled! {
    /// Which of an Epic's issues a read-in takes. `#1405`.
    ///
    /// **Asked, never assumed.** A person reading a milestone to plan work
    /// wants the open ones and a person reading one to see what shipped wants
    /// all of them, and a read-in that picked for them fills a Studio with work
    /// that is already done — which is what this replaced.
    EpicTake {
        /// Every issue the Epic holds, whatever state it is in.
        Everything => "everything",
        /// Only what the forge says is open.
        Open => "open",
    }
}

spelled! {
    /// Whether a person has accepted an edge. A proposed edge is drawn dashed.
    StudioEdgeStanding {
        Proposed => "proposed",
        Accepted => "accepted",
    }
}

impl EpicTake {
    /// Whether an issue in this state is one this answer takes.
    ///
    /// **`Open` takes what the forge said is open, and nothing else.** An issue
    /// whose state did not come back is not known to be open, so it is left out
    /// — and left out is a number the Epic says, never a silence.
    pub fn admits(&self, state: Option<ForgeState>) -> bool {
        match self {
            EpicTake::Everything => true,
            EpicTake::Open => state == Some(ForgeState::Open),
        }
    }
}

impl StudioNodeKind {
    /// The states a node of this kind may hold, the first being where it
    /// starts. Empty for a kind with none, and for Run and Job, whose state is
    /// the run's or the Job's own.
    pub fn states(&self) -> &'static [StudioNodeState] {
        use StudioNodeState as S;
        match self {
            // The three forge kinds hold no state of Armada's. Where the
            // *forge* says one stands is a field on the node, read off the
            // forge and never a lifecycle of ours.
            StudioNodeKind::Run
            | StudioNodeKind::Note
            | StudioNodeKind::Cluster
            | StudioNodeKind::Link
            | StudioNodeKind::Issue
            | StudioNodeKind::PullRequest
            | StudioNodeKind::Epic
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
    ///
    /// **An Issue, a Pull request and an Epic are not here either, and a
    /// person still makes one by pasting.** They paste a Link; the adapter
    /// recognises the address and Fleet writes the kind that follows —
    /// `#1394`. So the one act stays *paste an address*, and nothing on the
    /// seam can name a kind an address did not earn.
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

/// What a Run node keeps of its run once the run stops being Fleet's to read:
/// the result, and the log's last lines. `#1289`, `#1345`.
///
/// **Only ever present on a run nobody can read any more.** While it is still
/// readable the node is a reference and nothing else, and its state is read
/// off the run — so a node carrying one of these is saying that what is here
/// is all there is, which is what *partial* means on a Studio.
///
/// **Taken at the last moment it can be read, never after**, and which moment
/// that is depends on who holds it: a checkout run's record is a directory, so
/// before the sweep; a server's is Fleet's memory, so the instant it ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioRunKept {
    /// The entry the Manifest declared, by name.
    pub name: String,
    /// The command as it ran.
    pub command: String,
    /// `None` where the run was killed before it exited.
    pub exit_code: Option<i32>,
    /// What the entry declared a pass to be, so the node keeps its colour. **A
    /// server's colour is not read off it**: one that exits on its own has
    /// failed whatever its code — `docs/concepts/manifest.md`.
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

/// A line a person typed, or `None` where they typed nothing but space.
///
/// **One place decides what a blank line is**, so a node pasted with an empty
/// field and one whose line was cleared afterwards are the same record.
pub(super) fn trimmed(said: Option<String>) -> Option<String> {
    said.map(|line| String::from(line.trim()))
        .filter(|line| !line.is_empty())
}

/// What a Run node holds: a run in the checkout, or a server Fleet is holding.
/// `#1289`, `#1345`.
///
/// **Two ids that are not interchangeable, so they are not one field.** A
/// checkout run's id names a directory under `.armada/runs` that outlives
/// Fleet; a server's names an instance Fleet holds in memory and nothing else
/// does. A single `run_id` would let either be handed to the reader of the
/// other, and the reader would answer *no such run* about a server that is
/// serving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StudioRun {
    /// A Check or a Command that runs and exits, by its run id.
    Checkout(String),
    /// A Command with `serve`, by the id Fleet holds the instance under.
    Server(String),
}

impl StudioRun {
    /// The id, whichever it is. **For writing it down and nothing else** —
    /// which reader it is handed to is the variant's to decide.
    pub fn id(&self) -> &str {
        match self {
            StudioRun::Checkout(id) | StudioRun::Server(id) => id,
        }
    }
}

/// What a node holds, one variant per kind.
///
/// **The smallest each kind needs to be drawn and read.** A later step adds
/// what it builds beside these, never in place of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StudioNodeContent {
    /// A reference to the run, never its status — and what was kept of it
    /// once the run stopped being Fleet's to read. `#1289`, `#1345`.
    Run {
        run: StudioRun,
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
    /// A board, document, issue, page or session, kept as its address, the
    /// line a person wrote beside it, and what the source calls itself.
    ///
    /// **Three fields, and the address is never one of the other two.** A Link
    /// never stops being its address (`#1378`), and `#1379` dispatches a Job
    /// from the address alone.
    ///
    /// `said` is a person's own line. `named` is what a read-in learned the
    /// source calls itself — an issue's number, title and state (`#1293`).
    /// **They are not one field**: a read-in writing over a person's line
    /// would be an agent reorganising a person's work.
    Link {
        address: String,
        said: Option<String>,
        named: Option<String>,
    },
    /// An issue on a forge, made by pasting its address. `#1394`.
    ///
    /// **`address` and `number` are read off the address the moment the node
    /// is made**, by `adapters`, which is the only crate that knows whose
    /// forge it is. **`title` and `state` are read off the forge**, so they
    /// are absent until the node is read in.
    Issue {
        address: String,
        number: String,
        /// The line a person wrote beside it, as on a Link — `#1378`.
        said: Option<String>,
        title: Option<String>,
        state: Option<ForgeState>,
    },
    /// A pull request on a forge. Its fields are an Issue's, and `state` holds
    /// the one an issue cannot: `Merged`.
    PullRequest {
        address: String,
        number: String,
        said: Option<String>,
        title: Option<String>,
        state: Option<ForgeState>,
    },
    /// What a forge calls a set of issues under one address — a milestone, an
    /// epic, a wave. `#1394`.
    ///
    /// **No `state` and a count instead.** What matters about an Epic is how
    /// much of it is on the Studio, which is what reading it in answers; where
    /// it stands is the sum of its issues and is not a field anybody reads.
    Epic {
        address: String,
        number: String,
        said: Option<String>,
        title: Option<String>,
        /// How many of its issues are on this Studio, of how many it holds.
        /// Absent until it is read in.
        read_in: Option<EpicRead>,
    },
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
    /// A Link, with the person's line normalised: trimmed, and absent where
    /// nothing but whitespace was typed.
    ///
    /// **One place decides what a blank line is**, so a Link pasted with an
    /// empty field and one whose line was cleared afterwards are the same
    /// record rather than two shapes a reader has to tell apart.
    pub fn link(address: String, said: Option<String>) -> StudioNodeContent {
        let said = trimmed(said);
        StudioNodeContent::Link {
            address,
            said,
            named: None,
        }
    }

    /// A Link a read-in made, carrying the issue's own address and the line
    /// naming it. **No `said`**: a person has not written one on a node that
    /// did not exist a moment ago. `#1293`.
    pub fn link_named(address: String, named: String) -> StudioNodeContent {
        StudioNodeContent::Link {
            address,
            said: None,
            named: Some(named),
        }
    }

    pub fn kind(&self) -> StudioNodeKind {
        match self {
            StudioNodeContent::Run { .. } => StudioNodeKind::Run,
            StudioNodeContent::Note { .. } => StudioNodeKind::Note,
            StudioNodeContent::Cluster { .. } => StudioNodeKind::Cluster,
            StudioNodeContent::Finding(_) => StudioNodeKind::Finding,
            StudioNodeContent::Contradiction { .. } => StudioNodeKind::Contradiction,
            StudioNodeContent::Sketch { .. } => StudioNodeKind::Sketch,
            StudioNodeContent::Link { .. } => StudioNodeKind::Link,
            StudioNodeContent::Issue { .. } => StudioNodeKind::Issue,
            StudioNodeContent::PullRequest { .. } => StudioNodeKind::PullRequest,
            StudioNodeContent::Epic { .. } => StudioNodeKind::Epic,
            StudioNodeContent::Deferral { .. } => StudioNodeKind::Deferral,
            StudioNodeContent::Outline { .. } => StudioNodeKind::Outline,
            StudioNodeContent::IssueDraft { .. } => StudioNodeKind::IssueDraft,
            StudioNodeContent::Job { .. } => StudioNodeKind::Job,
        }
    }

    /// The first field left blank, by name, or `None` where every one is said.
    pub fn blank(&self) -> Option<&'static str> {
        let fields: &[(&'static str, &str)] = match self {
            StudioNodeContent::Run { run, .. } => &[("run_id", run.id())],
            StudioNodeContent::Note { said, .. } => &[("said", said)],
            StudioNodeContent::Cluster { title } => &[("title", title)],
            StudioNodeContent::Finding(finding) => &[("asked", finding.ask())],
            StudioNodeContent::Contradiction { first, second, .. } => {
                &[("first", first), ("second", second)]
            }
            StudioNodeContent::Sketch { body } | StudioNodeContent::Outline { body } => {
                &[("body", body)]
            }
            // Neither `said` nor `named` is here: a Link with no line and no
            // name is a Link, and both are normalised to absent, never blank.
            StudioNodeContent::Link { address, .. } => &[("address", address)],
            // Neither `title` nor `state` is here: what the forge says is
            // absent until the node is read in, never blank.
            StudioNodeContent::Issue {
                address, number, ..
            }
            | StudioNodeContent::PullRequest {
                address, number, ..
            }
            | StudioNodeContent::Epic {
                address, number, ..
            } => &[("address", address), ("number", number)],
            StudioNodeContent::Deferral { what } => &[("what", what)],
            StudioNodeContent::IssueDraft { title, body } => &[("title", title), ("body", body)],
            StudioNodeContent::Job { job_id } => &[("job_id", job_id.as_str())],
        };
        fields
            .iter()
            .find(|(_, text)| text.trim().is_empty())
            .map(|(name, _)| *name)
    }

    /// The **checkout run** this node references, where it is a Run node whose
    /// run is still the thing to read. `None` on every other kind, on a node
    /// holding a server, **and on a Run that has already kept its tail**: what
    /// it references is gone.
    ///
    /// **A server is not answered here**, which is what stops a run sweep from
    /// matching one: the two id spaces are separate and the readers are
    /// separate, so each has its own question.
    pub fn checkout_run_still_read(&self) -> Option<&str> {
        match self {
            StudioNodeContent::Run {
                run: StudioRun::Checkout(id),
                kept: None,
            } => Some(id),
            _ => None,
        }
    }

    /// The **server instance** this node references, where Fleet is still the
    /// one to read it from. [`checkout_run_still_read`]'s rule, one holder
    /// over. `#1345`.
    ///
    /// [`checkout_run_still_read`]: StudioNodeContent::checkout_run_still_read
    pub fn server_still_read(&self) -> Option<&str> {
        match self {
            StudioNodeContent::Run {
                run: StudioRun::Server(id),
                kept: None,
            } => Some(id),
            _ => None,
        }
    }

    /// The address a node keeps, and `None` on every kind that keeps none —
    /// `#1379`, `#1394`.
    ///
    /// **Four kinds keep one.** A Link is an address nothing recognised; an
    /// Issue, a Pull request and an Epic are an address an adapter did. What
    /// the address names is still not decided here: which host is the forge is
    /// `crates/adapters`' to know, and by the time a node holds one of the
    /// three that question has already been answered once, on the way in.
    pub fn address(&self) -> Option<&str> {
        match self {
            StudioNodeContent::Link { address, .. }
            | StudioNodeContent::Issue { address, .. }
            | StudioNodeContent::PullRequest { address, .. }
            | StudioNodeContent::Epic { address, .. } => Some(address),
            _ => None,
        }
    }

    /// The line a person wrote beside an address, on any kind that keeps one.
    pub fn said(&self) -> Option<&str> {
        match self {
            StudioNodeContent::Link { said, .. }
            | StudioNodeContent::Issue { said, .. }
            | StudioNodeContent::PullRequest { said, .. }
            | StudioNodeContent::Epic { said, .. } => said.as_deref(),
            _ => None,
        }
    }

    /// This content with what was kept of its run written into it.
    ///
    /// **The only method here that makes new content**, and the reason a Note
    /// stays fixed at capture: it takes a Run node whose run is about to stop
    /// being readable and no other, so nothing can reach a node's words
    /// through it, and a tail already kept is never written over.
    ///
    /// **Whichever it holds keeps holding it.** The reference is not rewritten
    /// by keeping a tail: a node that named a server still names it, so the
    /// log that is still on disk is still openable by its id.
    pub fn keeping(&self, kept: StudioRunKept) -> Option<StudioNodeContent> {
        let StudioNodeContent::Run { run, kept: None } = self else {
            return None;
        };
        Some(StudioNodeContent::Run {
            run: run.clone(),
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
