//! A Studio's rows read back, and refused by name where one does not.
//!
//! **Never a short list.** A node or an edge that does not read back fails the
//! whole read, naming its row, because a graph missing an edge draws a
//! different plan than the one a person left.

use core_model::{
    EdgeRefused, ManifestId, StateDoesNotFit, Studio, StudioEdge, StudioEdgeId, StudioEdgeKind,
    StudioEdgeStanding, StudioId, StudioName, StudioNode, StudioNodeId, StudioNodeState,
    StudioPosition, Timestamp, Ulid,
};

use super::content::{self, UnreadableContent};
use super::{database, StudioError};
use crate::open::Store;

/// What was wrong with a row.
#[derive(Debug)]
pub enum Unreadable {
    Content(UnreadableContent),
    UnknownValue { column: &'static str, value: String },
    StateDoesNotFit(StateDoesNotFit),
    Edge(EdgeRefused),
}

impl Store {
    pub(super) fn nodes_on(&self, studio_id: &StudioId) -> Result<Vec<StudioNode>, StudioError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT id, kind, state, content, x, y, created_at FROM studio_nodes \
                 WHERE studio_id = ?1 ORDER BY created_at, id",
            )
            .map_err(database("reading a Studio's nodes"))?;
        let rows = asking
            .query_map([studio_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    StudioPosition {
                        x: row.get(4)?,
                        y: row.get(5)?,
                    },
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(database("reading a Studio's nodes"))?;
        let mut nodes = Vec::new();
        for row in rows {
            let (id, kind, state, stored, position, created_at) =
                row.map_err(database("reading one node"))?;
            let unreadable = |why| StudioError::Unreadable {
                table: "studio_nodes",
                id: id.clone(),
                why,
            };
            let content = content::read(&kind, &stored)
                .map_err(|why| unreadable(Unreadable::Content(why)))?;
            let state = match state {
                None => None,
                Some(value) => Some(StudioNodeState::from_wire(&value).ok_or_else(|| {
                    unreadable(Unreadable::UnknownValue {
                        column: "state",
                        value,
                    })
                })?),
            };
            let node = StudioNode::recorded(
                StudioNodeId::carried(Ulid::carried(id.clone())),
                content,
                state,
                position,
                Timestamp::from_rfc3339(created_at),
            )
            .map_err(|why| unreadable(Unreadable::StateDoesNotFit(why)))?;
            nodes.push(node);
        }
        Ok(nodes)
    }

    pub(super) fn edges_on(&self, studio_id: &StudioId) -> Result<Vec<StudioEdge>, StudioError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT id, from_node, to_node, kind, standing, created_at FROM studio_edges \
                 WHERE studio_id = ?1 ORDER BY created_at, id",
            )
            .map_err(database("reading a Studio's edges"))?;
        let rows = asking
            .query_map([studio_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(database("reading a Studio's edges"))?;
        let mut edges = Vec::new();
        for row in rows {
            let (id, from, to, kind, standing, created_at) =
                row.map_err(database("reading one edge"))?;
            let unreadable = |why| StudioError::Unreadable {
                table: "studio_edges",
                id: id.clone(),
                why,
            };
            let kind = StudioEdgeKind::from_wire(&kind).ok_or_else(|| {
                unreadable(Unreadable::UnknownValue {
                    column: "kind",
                    value: kind.clone(),
                })
            })?;
            let standing = StudioEdgeStanding::from_wire(&standing).ok_or_else(|| {
                unreadable(Unreadable::UnknownValue {
                    column: "standing",
                    value: standing.clone(),
                })
            })?;
            let node = |id: String| StudioNodeId::carried(Ulid::carried(id));
            let edge = StudioEdge::recorded(
                StudioEdgeId::carried(Ulid::carried(id.clone())),
                node(from),
                node(to),
                kind,
                standing,
                Timestamp::from_rfc3339(created_at),
            )
            .map_err(|why| unreadable(Unreadable::Edge(why)))?;
            edges.push(edge);
        }
        Ok(edges)
    }
}

pub(super) fn studio_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Studio> {
    Ok(Studio {
        id: StudioId::carried(Ulid::carried(row.get::<_, String>(0)?)),
        manifest_id: ManifestId::carried(Ulid::carried(row.get::<_, String>(1)?)),
        name: row
            .get::<_, Option<String>>(2)?
            .as_deref()
            .and_then(StudioName::named),
        created_at: Timestamp::from_rfc3339(row.get::<_, String>(3)?),
        touched_at: Timestamp::from_rfc3339(row.get::<_, String>(4)?),
    })
}
