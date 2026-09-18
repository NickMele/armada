//! What a read-in put on a Studio, written in one transaction. `#1293`.
//!
//! **One write, because a node without its edge is a node from nowhere.** A
//! read-in makes several nodes at once, each `produced` by the Link, and any
//! of them landing without the others would leave a board nobody can walk back.
//!
//! **The node a read-in was on is rewritten only through [`Rewritten`].** Its
//! address is what a Job is dispatched from, so nothing here can reach it: the
//! only content this writes is one of `core_model`'s own rewrites, which is
//! the same rule `rewrite_studio_node` already follows.

use core_model::{
    Rewritten, StudioAuthor, StudioEdge, StudioEdgeId, StudioId, StudioNode, StudioNodeId,
    Timestamp,
};

use super::{content, database, edge_kept, node_kept, touched, StudioError};
use crate::open::Store;

impl Store {
    /// Keep what a read-in made: each node with a `produced` edge from `link`,
    /// every relation it proposed, whatever the read narrowed away, and the
    /// node the read-in was on as the read left it — an Epic's title and how
    /// many of its issues landed, or an Issue's or a Pull request's title and
    /// state. `#1394`, `#1405`.
    ///
    /// **`taken_back` goes in the same transaction as the count that describes
    /// it.** A narrowing that removed its nodes and failed before writing the
    /// Epic would leave the board and what the Epic says about it disagreeing,
    /// with nothing to reconcile them: a read-in is a person's act, not a
    /// reconciler, and nothing runs it again on its own.
    ///
    /// **Which nodes those are is the caller's to decide**, and `#1405` decides
    /// it from the graph: only an Issue this Epic `produced`, with no other
    /// edge, no line of a person's own and still where it was laid out.
    #[allow(clippy::too_many_arguments)]
    pub fn keep_read_in(
        &mut self,
        studio_id: &StudioId,
        link: &StudioNodeId,
        resolved: Option<&Rewritten>,
        made: &[(StudioNode, StudioEdgeId)],
        edges: &[StudioEdge],
        taken_back: &[StudioNodeId],
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let tx = self.writing()?;
        touched(&tx, studio_id, at)?;
        for node in taken_back {
            tx.execute(
                "DELETE FROM studio_nodes WHERE studio_id = ?1 AND id = ?2",
                (studio_id.as_str(), node.as_str()),
            )
            .map_err(database("taking back what a read-in narrowed away"))?;
        }
        if let Some(resolved) = resolved {
            let node = resolved.node();
            let changed = tx
                .execute(
                    "UPDATE studio_nodes SET content = ?3 WHERE studio_id = ?1 AND id = ?2",
                    (
                        studio_id.as_str(),
                        node.id().as_str(),
                        content::written(node.content()),
                    ),
                )
                .map_err(database("writing what a read-in learned about its source"))?;
            if changed == 0 {
                return Err(StudioError::NoSuchNode {
                    node_id: node.id().as_str().to_string(),
                });
            }
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
