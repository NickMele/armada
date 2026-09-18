//! The way back from a Job to the Studio that dispatched it. `#1362`.
//!
//! **Derived, from the edge that already records it.** Dispatch draws an
//! accepted `produced` edge from the Issue draft to the Job node it made, and
//! read the other way round that edge answers *which Studio produced this
//! Job* — so there is no forward column to be written twice, to disagree with
//! the edge, or to outlive the Studio it names. `crate::lineage`'s argument.
//!
//! **Nothing is the ordinary answer, and a truthful one.** Nodes cascade with
//! their Studio, so a deleted Studio takes the edge and this goes quiet;
//! `jobs.origin` is what still says the Job came off a Studio, and the two
//! together are how Job detail says *that Studio is no longer there* rather
//! than offering a control that opens nothing.
//!
//! No foreign key and no index, [`super`]'s own rule: one read per Job opened.
//!
//! **Not [`super::carrying_on`], which asks a different question.** That one
//! finds every Studio a Job's node stands on and where, so a redispatch can be
//! laid out beside it; this one finds the Studio that *produced* the Job, by
//! its name, and returns one.

use core_model::{JobId, StudioId, StudioName, StudioNodeId, Ulid};
use rusqlite::OptionalExtension;

use super::{database, StudioError};
use crate::open::Store;

/// The Studio a Job was dispatched from, and where on it to land.
///
/// **The node, not just the Studio.** Going back means landing on the part of
/// the graph the Job came from, so opening and selecting are one act.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchedFrom {
    pub studio_id: StudioId,
    /// `None` on a Studio nobody has named. Bridge has one word for that and
    /// this crate does not invent a second.
    pub name: Option<StudioName>,
    pub node_id: StudioNodeId,
}

impl Store {
    /// The Studio that produced this Job, where the record is still there.
    ///
    /// **One, never a list.** A Job reaches the Board from one Issue draft, so
    /// the question has one answer; where a redispatch has carried the Job
    /// onto a second Studio, the oldest node is the one that produced it.
    pub fn dispatched_from(&self, job_id: &JobId) -> Result<Option<DispatchedFrom>, StudioError> {
        self.conn
            .query_row(
                // The edge is the join and not the node alone: a Job node with
                // nothing pointing into it records the Job being *on* a
                // Studio, which is a different claim. `studio_edges` admits
                // `produced` only as `accepted`, so no standing is tested.
                "SELECT studio.id, studio.name, node.id \
                 FROM studio_nodes node \
                 JOIN studios studio ON studio.id = node.studio_id \
                 JOIN studio_edges edge \
                   ON edge.studio_id = node.studio_id AND edge.to_node = node.id \
                 WHERE node.kind = 'job' \
                   AND edge.kind = 'produced' \
                   AND json_extract(node.content, '$.job_id') = ?1 \
                 ORDER BY node.created_at, node.id LIMIT 1",
                [job_id.as_str()],
                |row| {
                    Ok(DispatchedFrom {
                        studio_id: StudioId::carried(Ulid::carried(row.get::<_, String>(0)?)),
                        // Blank reads as unnamed, which `StudioName::named`
                        // already decides for every other reader.
                        name: row
                            .get::<_, Option<String>>(1)?
                            .as_deref()
                            .and_then(StudioName::named),
                        node_id: StudioNodeId::carried(Ulid::carried(row.get::<_, String>(2)?)),
                    })
                },
            )
            .optional()
            .map_err(database("finding the Studio a Job was dispatched from"))
    }
}
