//! Promotion: what a person does with what is on a Studio. `#1291`,
//! `docs/concepts/studio.md`, *Promotion*.
//!
//! **Every rewrite of a node a person wrote is here, and nowhere else.** A
//! Contradiction settles, an Issue draft is edited before it is dispatched,
//! and a Link takes the line beside its address (`#1378`). Everything else a
//! rung does is a node added with `Produced` edges, which needs nothing new.
//! [`Rewritten`] is the second [`Scouted`](super::Scouted): only the methods
//! below make one, so the store's rewrite cannot reach a Note.
//!
//! **Nothing here files anything, anywhere.** Writing up makes an Issue draft
//! and dispatching sends its text; a forge is a person's own act afterwards.

use alloc::string::String;

use super::{StudioNode, StudioNodeContent, StudioNodeKind, StudioNodeState};

/// How a Contradiction ended, and a person picks which.
/// `docs/concepts/studio.md`'s outcomes table.
///
/// **Resolved here carries the answer**, because a Contradiction settled with
/// nothing written down is one nobody can reread. The other three record what
/// was decided and where it went — the Issue draft or the Deferral is its own
/// node, on the far end of a `Produced` edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContradictionOutcome {
    /// One side is stale: written up, and the draft is its own node.
    IssueDraft,
    /// A real decision, put off: the Deferral is its own node.
    Deferral,
    /// Both statements hold, in different contexts.
    NotAProblem,
    /// The person settled it, and this is the answer.
    ResolvedHere { answer: String },
}

impl ContradictionOutcome {
    /// The state a Contradiction holds once it ended this way.
    pub fn state(&self) -> StudioNodeState {
        match self {
            ContradictionOutcome::IssueDraft => StudioNodeState::IssueDraft,
            ContradictionOutcome::Deferral => StudioNodeState::Deferral,
            ContradictionOutcome::NotAProblem => StudioNodeState::NotAProblem,
            ContradictionOutcome::ResolvedHere { .. } => StudioNodeState::ResolvedHere,
        }
    }

    /// The answer, on the one outcome that records one.
    pub fn answer(&self) -> Option<&str> {
        match self {
            ContradictionOutcome::ResolvedHere { answer } => Some(answer),
            _ => None,
        }
    }
}

/// A rewrite asked of a node that does not take it: the wrong kind, or a
/// Contradiction that has already ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotRewritable {
    pub kind: StudioNodeKind,
    pub state: Option<StudioNodeState>,
}

/// A node as one of the rewrites below left it. **The store takes this and
/// never a bare node**, which is what keeps every other kind fixed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rewritten(StudioNode);

impl Rewritten {
    pub fn node(&self) -> &StudioNode {
        &self.0
    }
}

impl StudioNodeContent {
    /// The request an Issue draft dispatches as: **its title, a blank line and
    /// its body, whole**, and `None` on every other kind.
    ///
    /// **Nothing is summarised, trimmed or re-fetched.** A write-up is made
    /// from the nodes feeding it, so a lossy hop here would hand a Drone
    /// something other than what the person read and approved. The title is
    /// first because the proposer reads a request as prose and a person writes
    /// the subject at the top.
    pub fn dispatched_as(&self) -> Option<String> {
        match self {
            StudioNodeContent::IssueDraft { title, body } => {
                Some(alloc::format!("{title}\n\n{body}"))
            }
            _ => None,
        }
    }
}

impl StudioNode {
    /// The Issue draft, with the title and body a person edited into it.
    ///
    /// **Editable is the point**: a write-up is a first draft, and what is
    /// dispatched is what the person read and changed. Any other kind is
    /// refused.
    pub fn edited(&self, title: String, body: String) -> Result<Rewritten, NotRewritable> {
        match self.content() {
            StudioNodeContent::IssueDraft { .. } => {
                self.holding(StudioNodeContent::IssueDraft { title, body }, self.state())
            }
            content => Err(NotRewritable {
                kind: content.kind(),
                state: self.state(),
            }),
        }
    }

    /// The node, with the line a person wrote beside its address — `#1378`.
    ///
    /// **The address is not touched.** A Link never stops being its address,
    /// so this writes one field and reads the other back off the node; a blank
    /// line clears it, which is how somebody takes back what they typed.
    ///
    /// **Every kind that keeps an address takes one**, which is the four of
    /// `#1394`: an Issue, a Pull request and an Epic are what a Link's address
    /// turned out to name, and the line beside it is the person's either way.
    pub fn relabelled(&self, said: Option<String>) -> Result<Rewritten, NotRewritable> {
        match self.content().with_said(said) {
            Some(content) => self.holding(content, self.state()),
            None => Err(NotRewritable {
                kind: self.kind(),
                state: self.state(),
            }),
        }
    }

    /// The node, with what the forge says written into it — `#1394`. Refused
    /// on every kind that says nothing about a forge.
    pub fn resolved(&self, facts: &super::ForgeFacts) -> Result<Rewritten, NotRewritable> {
        match self.content().resolved(facts) {
            Some(content) => self.holding(content, self.state()),
            None => Err(NotRewritable {
                kind: self.kind(),
                state: self.state(),
            }),
        }
    }

    /// The Contradiction, ended the way a person picked.
    ///
    /// **A Contradiction ends once.** One already past `Reported` is refused
    /// rather than ended again, so the node keeps the outcome that was acted
    /// on rather than the last one anybody pressed.
    pub fn settled(&self, outcome: &ContradictionOutcome) -> Result<Rewritten, NotRewritable> {
        let refused = || NotRewritable {
            kind: self.kind(),
            state: self.state(),
        };
        let StudioNodeContent::Contradiction { first, second, .. } = self.content() else {
            return Err(refused());
        };
        if self.state() != Some(StudioNodeState::Reported) {
            return Err(refused());
        }
        self.holding(
            StudioNodeContent::Contradiction {
                first: first.clone(),
                second: second.clone(),
                answer: outcome.answer().map(String::from),
            },
            Some(outcome.state()),
        )
    }

    /// The same node, holding new content and a new state. **Built through
    /// [`StudioNode::recorded`]**, so a rewrite is held to the same
    /// kind-and-state rule a row read back off the store is.
    fn holding(
        &self,
        content: StudioNodeContent,
        state: Option<StudioNodeState>,
    ) -> Result<Rewritten, NotRewritable> {
        StudioNode::recorded(
            self.id().clone(),
            content,
            state,
            self.position(),
            self.created_at().clone(),
            self.added_by(),
        )
        .map(Rewritten)
        .map_err(|fault| NotRewritable {
            kind: fault.kind,
            state: fault.state,
        })
    }
}
