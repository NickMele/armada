//! A Finding: what a scout was asked, what it read, and how it ended. `#1292`,
//! `docs/concepts/scout.md`.
//!
//! **The one node whose content changes after it is added, and only its
//! scout changes it.** A Note is fixed because nothing offers a write; a
//! Finding moves through [`GatheringFinding`] and [`FrozenFinding`], which only
//! [`StudioNode::scouting`] makes, so no call can rewrite any other kind.
//!
//! **Its content says which state it is in**, checked on the way back out of
//! the store: a Proposed Finding has read nothing, a Gathering one has
//! recorded its checkout, and a Frozen one says how it ended.

use alloc::string::String;
use alloc::vec::Vec;

use super::{ScoutSourceKind, StudioNode, StudioNodeContent, StudioNodeState};

/// Which state of the repository a scout read: the commit checked out, and
/// whether anything was changed on top of it that is not committed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoutCheckout {
    pub commit: String,
    pub uncommitted: bool,
}

/// How a scout's process ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScoutOutcome {
    /// It finished its turn and answered.
    Answered,
    /// A person stopped it from its node.
    Stopped,
    /// It did not answer, and why, in Fleet's words.
    Failed { why: String },
}

/// A scout's end. **The cost is absent where the agent never reported one**,
/// which a process ended without a turn's end does not — never zero in its
/// place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoutEnded {
    pub outcome: ScoutOutcome,
    pub cost_micros: Option<u64>,
}

/// A source a scout was handed beyond the checkout. `#1293`.
///
/// **Fetched by Fleet and handed over as text**, never reached by the scout
/// itself: its launch denies every tool that could fetch one and
/// `--strict-mcp-config` leaves it no server, which is the confinement spike
/// 017 measured and the reason a scout is safe to run on a person's checkout.
///
/// **`cut` is what did not fit.** A page or a session is bounded before it is
/// handed over, and a Finding that does not say so would claim the scout read
/// a source whole when it read the front of one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoutSource {
    /// The Link's address, as the person pasted it. **Never rewritten**: the
    /// Link keeps its address whatever comes back.
    pub address: String,
    pub kind: ScoutSourceKind,
    /// Characters dropped from the end. `0` where the whole of it was handed
    /// over.
    pub cut: u64,
}

/// One thing a scout looked at: a file it read whole, or a search it ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScoutLook {
    /// A file, relative to the checkout where it is inside it.
    File(String),
    /// A search, as the pattern and where it looked.
    Search(String),
}

/// What a Finding holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioFinding {
    asked: String,
    checkout: Option<ScoutCheckout>,
    /// What it was handed beyond the checkout. Empty on a scout asked about
    /// the code alone, which is every Finding before `#1293`.
    sources: Vec<ScoutSource>,
    read: Vec<String>,
    searched: Vec<String>,
    learned: Option<String>,
    ended: Option<ScoutEnded>,
}

impl StudioFinding {
    /// An ask nobody has started. Proposed.
    pub fn asked(text: &str) -> StudioFinding {
        StudioFinding {
            asked: String::from(text),
            checkout: None,
            sources: Vec::new(),
            read: Vec::new(),
            searched: Vec::new(),
            learned: None,
            ended: None,
        }
    }

    /// A Finding read back, whole. Whether it fits its state is
    /// [`StudioNode::recorded`]'s check.
    pub fn recorded(
        asked: String,
        checkout: Option<ScoutCheckout>,
        sources: Vec<ScoutSource>,
        read: Vec<String>,
        searched: Vec<String>,
        learned: Option<String>,
        ended: Option<ScoutEnded>,
    ) -> StudioFinding {
        StudioFinding {
            asked,
            checkout,
            sources,
            read,
            searched,
            learned,
            ended,
        }
    }

    /// What the scout was asked, verbatim.
    pub fn ask(&self) -> &str {
        &self.asked
    }

    pub fn checkout(&self) -> Option<&ScoutCheckout> {
        self.checkout.as_ref()
    }

    /// Every source it was handed beyond the checkout, in the order handed.
    pub fn sources(&self) -> &[ScoutSource] {
        &self.sources
    }

    /// Every file read, in the order first read.
    pub fn read(&self) -> &[String] {
        &self.read
    }

    /// Every search run, in the order first run.
    pub fn searched(&self) -> &[String] {
        &self.searched
    }

    /// What the scout said last, where it said anything.
    pub fn learned(&self) -> Option<&str> {
        self.learned.as_deref()
    }

    pub fn ended(&self) -> Option<&ScoutEnded> {
        self.ended.as_ref()
    }

    /// Whether this content is what a Finding at `state` holds.
    pub fn fits(&self, state: Option<StudioNodeState>) -> bool {
        let untouched = self.read.is_empty() && self.searched.is_empty() && self.learned.is_none();
        // A source is handed over as the scout starts, so a Proposed Finding
        // has none the way it has no checkout.
        let untouched = untouched && self.sources.is_empty();
        match state {
            Some(StudioNodeState::Proposed) => {
                self.checkout.is_none() && self.ended.is_none() && untouched
            }
            Some(StudioNodeState::Gathering) => self.checkout.is_some() && self.ended.is_none(),
            Some(StudioNodeState::Frozen) => self.checkout.is_some() && self.ended.is_some(),
            _ => false,
        }
    }
}

/// A Proposed Finding cannot be started from the node given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotScoutable {
    /// The node is not a Finding.
    NotAFinding,
    /// It is a Finding, and not Proposed: already gathering, or frozen.
    NotProposed(StudioNodeState),
}

/// A Finding its scout is reading for. Made only by [`StudioNode::scouting`],
/// or from a node already recorded Gathering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatheringFinding {
    node: StudioNode,
}

/// A Finding its scout has finished with, however it ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenFinding {
    node: StudioNode,
}

impl StudioNode {
    /// Start a Proposed Finding: its checkout recorded, and Gathering.
    pub fn scouting(&self, checkout: ScoutCheckout) -> Result<GatheringFinding, NotScoutable> {
        let StudioNodeContent::Finding(finding) = &self.content else {
            return Err(NotScoutable::NotAFinding);
        };
        match self.state {
            Some(StudioNodeState::Proposed) => {}
            Some(other) => return Err(NotScoutable::NotProposed(other)),
            None => return Err(NotScoutable::NotAFinding),
        }
        let mut finding = finding.clone();
        finding.checkout = Some(checkout);
        Ok(GatheringFinding {
            node: StudioNode {
                content: StudioNodeContent::Finding(finding),
                state: Some(StudioNodeState::Gathering),
                ..self.clone()
            },
        })
    }
}

impl StudioNode {
    /// Start a Proposed Finding that is reading sources in as well as the
    /// checkout: what it was handed recorded before it reads a line. `#1293`.
    ///
    /// **Recorded at the start and never after**, because Fleet fetched them:
    /// what a Finding says it was given is what was handed over, not something
    /// read off the scout's own account of its turn.
    pub fn reading_in(
        &self,
        checkout: ScoutCheckout,
        sources: Vec<ScoutSource>,
    ) -> Result<GatheringFinding, NotScoutable> {
        let mut gathering = self.scouting(checkout)?;
        gathering.finding().sources = sources;
        Ok(gathering)
    }
}

impl GatheringFinding {
    /// A node recorded Gathering, taken back up — a scout Fleet lost track of.
    pub fn of(node: StudioNode) -> Option<GatheringFinding> {
        let gathering = matches!(node.content, StudioNodeContent::Finding(_))
            && node.state == Some(StudioNodeState::Gathering);
        gathering.then_some(GatheringFinding { node })
    }

    pub fn node(&self) -> &StudioNode {
        &self.node
    }

    fn finding(&mut self) -> &mut StudioFinding {
        match &mut self.node.content {
            StudioNodeContent::Finding(finding) => finding,
            // Only a Finding is ever wrapped, by both constructors above.
            _ => unreachable!("a GatheringFinding holds a Finding"),
        }
    }

    /// Record one more thing looked at. **`false` where it was already
    /// recorded**, so a file read twice is listed once and not written again.
    pub fn looked(&mut self, look: ScoutLook) -> bool {
        let finding = self.finding();
        let (list, what) = match look {
            ScoutLook::File(path) => (&mut finding.read, path),
            ScoutLook::Search(search) => (&mut finding.searched, search),
        };
        if list.contains(&what) {
            return false;
        }
        list.push(what);
        true
    }

    /// Frozen, with what the scout said last and how it ended.
    pub fn frozen(mut self, learned: Option<String>, ended: ScoutEnded) -> FrozenFinding {
        let finding = self.finding();
        finding.learned = learned.filter(|said| !said.trim().is_empty());
        finding.ended = Some(ended);
        FrozenFinding {
            node: StudioNode {
                state: Some(StudioNodeState::Frozen),
                ..self.node
            },
        }
    }
}

impl FrozenFinding {
    pub fn node(&self) -> &StudioNode {
        &self.node
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::GatheringFinding {}
    impl Sealed for super::FrozenFinding {}
}

/// A Finding its scout moved, which is the one node a store may rewrite.
/// **Sealed**: no other crate can name a node this way.
pub trait Scouted: sealed::Sealed {
    fn scouted(&self) -> &StudioNode;
}

impl Scouted for GatheringFinding {
    fn scouted(&self) -> &StudioNode {
        &self.node
    }
}

impl Scouted for FrozenFinding {
    fn scouted(&self) -> &StudioNode {
        &self.node
    }
}
