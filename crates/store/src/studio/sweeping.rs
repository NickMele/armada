//! What a Run node keeps when the run it references is swept. `#1289`.
//!
//! **The one write that makes new content**, and it is not on the seam: no
//! request reaches it, and `core_model`'s `keeping` refuses anything but a Run
//! node whose tail is not kept yet — so a Note stays fixed at capture and a
//! result already kept is never written over.
//!
//! **`touched_at` does not move.** Retention passing is not something a person
//! did on the Studio, and reordering the Studios list by it would put a
//! whiteboard nobody has opened in weeks at the top.

use core_model::{StudioId, StudioNodeContent, StudioRunKept, Ulid};

use super::{content, database, StudioError, Unreadable};
use crate::open::Store;

impl Store {
    /// Write what was kept of `run_id` onto every Run node that references it,
    /// and name the Studios that changed, oldest node first.
    ///
    /// **Taken before the sweep, so the caller has to have read the log while
    /// it was still there.** Nothing here opens a file: what it is handed is
    /// what survives.
    ///
    /// A node that already kept a tail is left alone, so a second pass over
    /// the same run is no write at all.
    pub fn keep_studio_run(
        &mut self,
        run_id: &str,
        kept: &StudioRunKept,
    ) -> Result<Vec<StudioId>, StudioError> {
        let holders = self.nodes_holding(run_id)?;
        if holders.is_empty() {
            return Ok(Vec::new());
        }
        let tx = self
            .conn
            .transaction()
            .map_err(database("keeping a swept run on a Studio"))?;
        let mut studios = Vec::new();
        for (node_id, studio_id, content) in holders {
            let Some(kept) = content.keeping(kept.clone()) else {
                continue;
            };
            tx.execute(
                "UPDATE studio_nodes SET content = ?2 WHERE id = ?1",
                (node_id.as_str(), content::written(&kept)),
            )
            .map_err(database("keeping a swept run on a Studio"))?;
            let studio_id = StudioId::carried(Ulid::carried(studio_id));
            if !studios.contains(&studio_id) {
                studios.push(studio_id);
            }
        }
        tx.commit()
            .map_err(database("keeping a swept run on a Studio"))?;
        Ok(studios)
    }

    /// Every Run node that still reads its state off `run_id`, as its id, its
    /// Studio's id and its content.
    ///
    /// **Filtered in Rust over the `run` rows**, not by a query into the JSON:
    /// the kind is a column with a `CHECK` over it, so the rows to read are
    /// already few, and the reference is read back through the one decoder the
    /// rest of this module reads content with.
    fn nodes_holding(
        &self,
        run_id: &str,
    ) -> Result<Vec<(String, String, StudioNodeContent)>, StudioError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT id, studio_id, content FROM studio_nodes WHERE kind = 'run' \
                 ORDER BY created_at, id",
            )
            .map_err(database("reading the Run nodes a sweep would strand"))?;
        let rows = asking
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(database("reading the Run nodes a sweep would strand"))?;
        let mut holding = Vec::new();
        for row in rows {
            let (id, studio_id, stored) = row.map_err(database("reading one Run node"))?;
            let content = content::read("run", &stored).map_err(|why| StudioError::Unreadable {
                table: "studio_nodes",
                id: id.clone(),
                why: Unreadable::Content(why),
            })?;
            if content.run_still_read() == Some(run_id) {
                holding.push((id, studio_id, content));
            }
        }
        Ok(holding)
    }
}
