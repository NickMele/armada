//! What a read-in put on a Studio, written in one transaction. `#1293`.
//!
//! **One write, because a node without its edge is a node from nowhere.** A
//! read-in makes several nodes at once, each `produced` by the Link, and any
//! of them landing without the others would leave a board nobody can walk back.
//!
//! **The Link is rewritten only in its `named` line.** Its address is what
//! `#1379` dispatches from, so nothing here can reach it: the update writes
//! the content this module builds from the address already stored.

use core_model::{
    StudioAuthor, StudioEdge, StudioEdgeId, StudioId, StudioNode, StudioNodeContent, StudioNodeId,
    StudioNodeKind, Timestamp,
};

use super::{content, database, edge_kept, node_kept, touched, StudioError};
use crate::open::Store;

impl Store {
    /// Keep what a read-in made: each node with a `produced` edge from `link`,
    /// every relation it proposed, and the Link's own `named` line where the
    /// read learned one.
    pub fn keep_read_in(
        &mut self,
        studio_id: &StudioId,
        link: &StudioNodeId,
        named: Option<&str>,
        made: &[(StudioNode, StudioEdgeId)],
        edges: &[StudioEdge],
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let tx = self.writing()?;
        touched(&tx, studio_id, at)?;
        if let Some(named) = named {
            let address = tx
                .query_row(
                    "SELECT content FROM studio_nodes WHERE studio_id = ?1 AND id = ?2 \
                     AND kind = ?3",
                    (
                        studio_id.as_str(),
                        link.as_str(),
                        StudioNodeKind::Link.as_wire(),
                    ),
                    |row| row.get::<_, String>(0),
                )
                .map_err(database("reading the Link a read-in was on"))?;
            let StudioNodeContent::Link { address, said, .. } =
                content::read(StudioNodeKind::Link.as_wire(), &address).map_err(|why| {
                    StudioError::Unreadable {
                        table: "studio_nodes",
                        id: link.as_str().to_string(),
                        why: super::Unreadable::Content(why),
                    }
                })?
            else {
                return Err(StudioError::NoSuchNode {
                    node_id: link.as_str().to_string(),
                });
            };
            // **The person's own line is carried over untouched.** A read-in
            // that dropped it would be an agent deleting a person's words.
            let written = content::written(&StudioNodeContent::Link {
                address,
                said,
                named: Some(named.to_string()),
            });
            tx.execute(
                "UPDATE studio_nodes SET content = ?3 WHERE studio_id = ?1 AND id = ?2",
                (studio_id.as_str(), link.as_str(), written),
            )
            .map_err(database("naming the Link a read-in was on"))?;
        }
        for (node, edge) in made {
            node_kept(&tx, studio_id, node)?;
            let by = node.added_by().unwrap_or(StudioAuthor::Person);
            let produced = StudioEdge::produced(
                edge.clone(),
                link.clone(),
                node.id().clone(),
                at.clone(),
                by,
            )
            // Only reachable by reading a Link in onto itself, which no node
            // this function was handed is.
            .map_err(|_| StudioError::NoSuchNode {
                node_id: link.as_str().to_string(),
            })?;
            edge_kept(&tx, studio_id, &produced)?;
        }
        for edge in edges {
            edge_kept(&tx, studio_id, edge)?;
        }
        tx.commit().map_err(database("keeping what a read-in made"))
    }
}
