//! What a forge address makes: an Issue, a Pull request or an Epic. `#1394`,
//! `docs/concepts/studio.md`, *Nodes*.
//!
//! **The kind is the concept and the adapter decides it.** Which host is the
//! forge and which of its paths is an issue are `crates/adapters`' to know, so
//! nothing here reads an address. What is here is the shape the three kinds
//! share, and the three narrow ways to reach one.
//!
//! **Three ways in, and no fourth.** [`StudioNodeContent::on_the_forge`] makes
//! one from an address an adapter recognised; `resolved` writes what a read-in
//! learned into one; and [`StudioNode::recognised`] turns a Link already on a
//! Studio into what it turned out to be. None of them can name a kind outside
//! the three, move an address, or reach a Note.

use alloc::string::String;

use super::{trimmed, ForgeState, StudioNode, StudioNodeContent, StudioNodeKind};

/// How much of an Epic is on the Studio, as reading it in left it. `#1394`.
///
/// **Two numbers rather than a sentence.** A read-in is bounded, so an Epic
/// that fits says how many it holds and one that did not says how many of how
/// many — and that is a fact a surface can draw rather than a line it parses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EpicRead {
    /// How many of its issues are on this Studio.
    pub issues: u64,
    /// How many it holds in all, whether or not they fit.
    pub total: u64,
}

/// What the forge says about what an address names, as a read-in learned it.
///
/// **Only what the kind holds is written.** An Epic takes no state and an
/// Issue takes no count, so a field this carries for a kind that has none is
/// dropped rather than kept somewhere it could be read back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ForgeFacts {
    pub title: Option<String>,
    pub state: Option<ForgeState>,
    pub read_in: Option<EpicRead>,
}

/// A Link as the kind its address turned out to name. `#1394`.
///
/// **The second [`Rewritten`]**, and narrower: only
/// [`StudioNode::recognised`] makes one, and it refuses anything that is not a
/// Link becoming one of the three forge kinds at the same address. So the
/// store's one conversion cannot reach a Note, cannot change a node's address
/// and cannot turn an Issue back into a Link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recognised(StudioNode);

impl Recognised {
    pub fn node(&self) -> &StudioNode {
        &self.0
    }
}

impl StudioNodeContent {
    /// A node made from an address an adapter recognised: the kind it decided,
    /// the number it read off the address, and the person's line carried over
    /// from the Link they pasted. `#1394`.
    ///
    /// **`None` for every kind that is not one of the three**, so a caller
    /// cannot mint a Note through this — the kind comes from the adapter's
    /// reading and nothing else may name one.
    pub fn on_the_forge(
        kind: StudioNodeKind,
        address: String,
        number: String,
        said: Option<String>,
    ) -> Option<StudioNodeContent> {
        let said = trimmed(said);
        Some(match kind {
            StudioNodeKind::Issue => StudioNodeContent::Issue {
                address,
                number,
                said,
                title: None,
                state: None,
            },
            StudioNodeKind::PullRequest => StudioNodeContent::PullRequest {
                address,
                number,
                said,
                title: None,
                state: None,
            },
            StudioNodeKind::Epic => StudioNodeContent::Epic {
                address,
                number,
                said,
                title: None,
                read_in: None,
            },
            _ => return None,
        })
    }

    /// The same content with the line beside its address replaced, and `None`
    /// on a kind that keeps no address. **One place normalises a blank line**
    /// — [`StudioNodeContent::link`]'s rule, for the three forge kinds too.
    pub fn with_said(&self, said: Option<String>) -> Option<StudioNodeContent> {
        let address = String::from(self.address()?);
        Some(match self.clone() {
            StudioNodeContent::Link { .. } => StudioNodeContent::link(address, said),
            StudioNodeContent::Issue {
                number,
                title,
                state,
                ..
            } => StudioNodeContent::Issue {
                address,
                number,
                said: trimmed(said),
                title,
                state,
            },
            StudioNodeContent::PullRequest {
                number,
                title,
                state,
                ..
            } => StudioNodeContent::PullRequest {
                address,
                number,
                said: trimmed(said),
                title,
                state,
            },
            StudioNodeContent::Epic {
                number,
                title,
                read_in,
                ..
            } => StudioNodeContent::Epic {
                address,
                number,
                said: trimmed(said),
                title,
                read_in,
            },
            // `address()` answered, so there is no other kind here.
            _ => return None,
        })
    }

    /// The same content with what the forge says written into it, and `None`
    /// on a kind that says nothing about a forge. `#1394`.
    ///
    /// **The address, the number and the person's line are carried over
    /// untouched.** A read-in that rewrote either would be a record of the
    /// reading rather than of the source, and one that rewrote the line would
    /// be an agent deleting a person's words.
    pub fn resolved(&self, facts: &ForgeFacts) -> Option<StudioNodeContent> {
        Some(match self.clone() {
            StudioNodeContent::Issue {
                address,
                number,
                said,
                title,
                state,
            } => StudioNodeContent::Issue {
                address,
                number,
                said,
                title: facts.title.clone().or(title),
                state: facts.state.or(state),
            },
            StudioNodeContent::PullRequest {
                address,
                number,
                said,
                title,
                state,
            } => StudioNodeContent::PullRequest {
                address,
                number,
                said,
                title: facts.title.clone().or(title),
                state: facts.state.or(state),
            },
            StudioNodeContent::Epic {
                address,
                number,
                said,
                title,
                read_in,
            } => StudioNodeContent::Epic {
                address,
                number,
                said,
                title: facts.title.clone().or(title),
                read_in: facts.read_in.or(read_in),
            },
            _ => return None,
        })
    }
}

impl StudioNode {
    /// The same node as the kind its address turned out to name — `#1394`.
    ///
    /// **`None` unless this is a Link and `content` is one of the three forge
    /// kinds at the very same address.** A Link's address is what a Job is
    /// dispatched from, so a conversion that could move one would be a node
    /// pointing somewhere nobody pasted; and a conversion that could reach any
    /// other kind would be the one write on a Studio that rewrites a person's
    /// words.
    ///
    /// **Everything else survives**: the id, the position, when it was made,
    /// who made it, and the line the person typed, which the caller carries
    /// into `content`.
    pub fn recognised(&self, content: StudioNodeContent) -> Option<Recognised> {
        let address = match self.content() {
            StudioNodeContent::Link { address, .. } => address,
            _ => return None,
        };
        let becomes = matches!(
            content.kind(),
            StudioNodeKind::Issue | StudioNodeKind::PullRequest | StudioNodeKind::Epic
        );
        if !becomes || content.address() != Some(address.as_str()) {
            return None;
        }
        Some(Recognised(StudioNode {
            content,
            // Neither kind holds a state, and `recorded` would refuse it.
            state: None,
            ..self.clone()
        }))
    }
}
