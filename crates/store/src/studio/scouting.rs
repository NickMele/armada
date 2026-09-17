//! The one rewrite a Studio's nodes allow: a Finding moved by its scout.
//! `#1292`.
//!
//! **Takes [`Scouted`], which only `core_model`'s scout transitions make**, so
//! no call here can hand a Note new words. The row is matched on its kind as
//! well as its id, so even a mistaken id cannot land on another kind.

use core_model::{GatheringFinding, Scouted, StudioId, StudioNodeKind, Timestamp, Ulid};

use super::{database, touched, StudioError};
use crate::open::Store;

impl Store {
    /// Keep a Finding's state and content as its scout left them.
    /// [`StudioError::NoSuchNode`] where a person removed it meanwhile.
    pub fn keep_scouted(
        &mut self,
        studio_id: &StudioId,
        scouted: &impl Scouted,
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let node = scouted.scouted();
        let tx = self.writing()?;
        touched(&tx, studio_id, at)?;
        let kept = tx
            .execute(
                "UPDATE studio_nodes SET state = ?3, content = ?4 \
                 WHERE studio_id = ?1 AND id = ?2 AND kind = ?5",
                (
                    studio_id.as_str(),
                    node.id().as_str(),
                    node.state().map(|state| state.as_wire()),
                    super::content::written(node.content()),
                    StudioNodeKind::Finding.as_wire(),
                ),
            )
            .map_err(database("keeping what a scout found"))?;
        if kept == 0 {
            return Err(StudioError::NoSuchNode {
                node_id: node.id().as_str().to_string(),
            });
        }
        tx.commit().map_err(database("keeping what a scout found"))
    }

    /// Every Finding recorded Gathering, on every Studio. **Read at start**:
    /// a scout's process does not outlive the Fleet reading it, so each of
    /// these is one nobody is reading for any more.
    pub fn gathering_findings(&self) -> Result<Vec<(StudioId, GatheringFinding)>, StudioError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT DISTINCT studio_id FROM studio_nodes \
                 WHERE kind = 'finding' AND state = 'gathering' ORDER BY studio_id",
            )
            .map_err(database("finding the Findings still gathering"))?;
        let studios = asking
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(database("finding the Findings still gathering"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database("finding the Findings still gathering"))?;
        let mut gathering = Vec::new();
        for studio in studios {
            let studio_id = StudioId::carried(Ulid::carried(studio));
            for node in self.nodes_on(&studio_id)? {
                if let Some(finding) = GatheringFinding::of(node) {
                    gathering.push((studio_id.clone(), finding));
                }
            }
        }
        Ok(gathering)
    }
}
