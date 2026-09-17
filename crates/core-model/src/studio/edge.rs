//! An edge between two nodes on one Studio. `docs/concepts/studio.md`, *Edges*.
//!
//! **Only a person accepts a relation**: a proposed edge is the only kind any
//! caller builds, and `Produced` is the Studio's own, accepted as drawn.

use super::{
    StudioAuthor, StudioEdgeId, StudioEdgeKind, StudioEdgeStanding, StudioNodeId, StudioRelation,
};
use crate::envelope::Timestamp;

/// An edge between two nodes on one Studio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioEdge {
    id: StudioEdgeId,
    from: StudioNodeId,
    to: StudioNodeId,
    kind: StudioEdgeKind,
    standing: StudioEdgeStanding,
    created_at: Timestamp,
    /// `None` only on an edge kept before who added it was.
    added_by: Option<StudioAuthor>,
}

/// Both ends of an edge are one node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToItself {
    pub node: StudioNodeId,
}

/// Why an edge cannot be read back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EdgeRefused {
    ToItself(ToItself),
    /// A `Produced` edge that is not accepted. The Studio draws those, so none
    /// is ever proposed.
    ProducedUnaccepted,
}

impl StudioEdge {
    /// A relation proposed by `by`. Only a person accepts one.
    pub fn proposed(
        id: StudioEdgeId,
        from: StudioNodeId,
        to: StudioNodeId,
        relation: StudioRelation,
        created_at: Timestamp,
        by: StudioAuthor,
    ) -> Result<StudioEdge, ToItself> {
        StudioEdge::joining(
            id,
            (from, to),
            relation.into(),
            StudioEdgeStanding::Proposed,
            created_at,
            Some(by),
        )
    }

    /// A relation a person drew themselves, accepted as drawn. **Only a
    /// person builds one**: `by` is the transport's word and the only value
    /// any caller passes is [`StudioAuthor::Person`], because accepting a
    /// relation is a person's act — `#1291`'s Deferral against what it blocks
    /// is the one rung that draws one.
    pub fn accepted(
        id: StudioEdgeId,
        from: StudioNodeId,
        to: StudioNodeId,
        relation: StudioRelation,
        created_at: Timestamp,
        by: StudioAuthor,
    ) -> Result<StudioEdge, ToItself> {
        StudioEdge::joining(
            id,
            (from, to),
            relation.into(),
            StudioEdgeStanding::Accepted,
            created_at,
            Some(by),
        )
    }

    /// The Studio's own record that `from` made `to`, accepted as drawn, and
    /// added by whoever added `to`.
    pub fn produced(
        id: StudioEdgeId,
        from: StudioNodeId,
        to: StudioNodeId,
        created_at: Timestamp,
        by: StudioAuthor,
    ) -> Result<StudioEdge, ToItself> {
        StudioEdge::joining(
            id,
            (from, to),
            StudioEdgeKind::Produced,
            StudioEdgeStanding::Accepted,
            created_at,
            Some(by),
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
        added_by: Option<StudioAuthor>,
    ) -> Result<StudioEdge, EdgeRefused> {
        if kind == StudioEdgeKind::Produced && standing != StudioEdgeStanding::Accepted {
            return Err(EdgeRefused::ProducedUnaccepted);
        }
        StudioEdge::joining(id, (from, to), kind, standing, created_at, added_by)
            .map_err(EdgeRefused::ToItself)
    }

    fn joining(
        id: StudioEdgeId,
        (from, to): (StudioNodeId, StudioNodeId),
        kind: StudioEdgeKind,
        standing: StudioEdgeStanding,
        created_at: Timestamp,
        added_by: Option<StudioAuthor>,
    ) -> Result<StudioEdge, ToItself> {
        if from == to {
            return Err(ToItself { node: from });
        }
        Ok(StudioEdge {
            id,
            from,
            to,
            kind,
            standing,
            created_at,
            added_by,
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
    pub fn added_by(&self) -> Option<StudioAuthor> {
        self.added_by
    }
}
