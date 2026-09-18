//! Which Studios drew a Job, and where on each. `#1440`.
//!
//! **Asked by Job id, across every Studio.** A redispatch is a Job's act and
//! knows no Studio, so the question runs the other way round from every other
//! read in this module: not *what is on this Studio* but *which boards drew
//! this Job*. Answered here so the replacement can be laid out beside the one
//! it replaced.
//!
//! **No foreign key and no index, which is [`super`]'s own rule.** A Job
//! node's reference is text so that forgetting a Job leaves the node standing,
//! and this read runs once per redispatch over a table a person fills by hand.

use core_model::{JobId, StudioId, StudioNodeId, StudioPosition, Ulid};

use super::{database, StudioError};
use crate::open::Store;

/// One Job node: the Studio holding it, where it sits, and how many nodes it
/// has already produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobOnStudio {
    pub studio_id: StudioId,
    pub node_id: StudioNodeId,
    pub position: StudioPosition,
    /// The `produced` edges already leaving it.
    ///
    /// **A row number, not a count of replacements.** A Job redispatched twice
    /// produces two nodes from one, and the second is laid out under the first
    /// rather than on top of it — `docs/concepts/studio.md` says nothing on a
    /// Studio is deleted, so two nodes at one position would be a record only
    /// a drag could read.
    pub produced: i64,
}

impl Store {
    /// Every Job node naming this Job, on every Studio, oldest first.
    ///
    /// **Empty is the ordinary answer.** Most Jobs were never dispatched from
    /// a Studio, and a Studio that held one and had the node deleted reads the
    /// same way — which is the reading the record supports, since a delete on
    /// a Studio is a person saying they are done with that node.
    pub fn job_on_studios(&self, job_id: &JobId) -> Result<Vec<JobOnStudio>, StudioError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT node.studio_id, node.id, node.x, node.y, \
                 (SELECT COUNT(*) FROM studio_edges edge \
                  WHERE edge.studio_id = node.studio_id AND edge.from_node = node.id \
                  AND edge.kind = 'produced') \
                 FROM studio_nodes node \
                 WHERE node.kind = 'job' AND json_extract(node.content, '$.job_id') = ?1 \
                 ORDER BY node.created_at, node.id",
            )
            .map_err(database("finding the Studios a Job is on"))?;
        let rows = asking
            .query_map([job_id.as_str()], |row| {
                Ok(JobOnStudio {
                    studio_id: StudioId::carried(Ulid::carried(row.get::<_, String>(0)?)),
                    node_id: StudioNodeId::carried(Ulid::carried(row.get::<_, String>(1)?)),
                    position: StudioPosition {
                        x: row.get(2)?,
                        y: row.get(3)?,
                    },
                    produced: row.get(4)?,
                })
            })
            .map_err(database("finding the Studios a Job is on"))?;
        rows.map(|row| row.map_err(database("reading one Job node")))
            .collect()
    }
}
