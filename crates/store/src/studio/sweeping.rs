//! What a Run node keeps when the run it references stops being readable — a
//! checkout run swept, or a server ending. `#1289`, `#1345`.
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
        self.kept_on_nodes(kept, |content| {
            content.checkout_run_still_read() == Some(run_id)
        })
    }

    /// The same write for a **server** the node holds: what it said, onto every
    /// Run node naming that instance. `#1345`.
    ///
    /// **Called as the server ends, not as its directory is swept.** A server's
    /// result is Fleet's memory and nothing else — `crates/fleet`'s `Servers`
    /// is never written down — so the instant it ends is the last moment there
    /// is anything to keep.
    pub fn keep_studio_server(
        &mut self,
        server_id: &str,
        kept: &StudioRunKept,
    ) -> Result<Vec<StudioId>, StudioError> {
        self.kept_on_nodes(kept, |content| {
            content.server_still_read() == Some(server_id)
        })
    }

    fn kept_on_nodes(
        &mut self,
        kept: &StudioRunKept,
        holds: impl Fn(&StudioNodeContent) -> bool,
    ) -> Result<Vec<StudioId>, StudioError> {
        let holders = self.nodes_holding(&holds)?;
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

    /// Every Run node `holds` answers for, as its id, its Studio's id and its
    /// content.
    ///
    /// **Filtered in Rust over the `run` rows**, not by a query into the JSON:
    /// the kind is a column with a `CHECK` over it, so the rows to read are
    /// already few, and the reference is read back through the one decoder the
    /// rest of this module reads content with — which is also what keeps a
    /// server's id from ever matching a run's, since each is asked for by its
    /// own variant rather than by string equality on one field.
    fn nodes_holding(
        &self,
        holds: &impl Fn(&StudioNodeContent) -> bool,
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
            if holds(&content) {
                holding.push((id, studio_id, content));
            }
        }
        Ok(holding)
    }
}
