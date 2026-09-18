//! Every Studio a repository keeps, with its nodes and edges. `#1285`.
//!
//! **Kept until a person deletes it.** No foreign key reaches `jobs`, so
//! `forget_job` has nothing to delete here, and no sweep names these tables.
//! A Job node's reference is text for the same reason: a forgotten Job leaves
//! the node that pointed at it standing.
//!
//! **No write offers new content, but a scout's and a sweep's.** A node is
//! added, moved or removed, and a Note is fixed at capture because nothing
//! here could change one; a Finding is rewritten only as `scouting` allows,
//! and [`sweeping`] fills in what a Run node keeps of a run retention is about
//! to take away, reaching no other kind.
//!
//! **Every write touches its Studio**, in the same transaction, so the list a
//! person reads orders by the last thing that happened on each.

mod content;
mod reading;
mod reading_in;
mod scouting;
mod sweeping;

use core_model::{
    ManifestId, Recognised, Rewritten, Studio, StudioAuthor, StudioEdge, StudioEdgeId,
    StudioEdgeKind, StudioEdgeStanding, StudioGraph, StudioId, StudioName, StudioNode,
    StudioNodeId, StudioPosition, Timestamp,
};
use rusqlite::{OptionalExtension, Transaction};

use crate::error::{fault, DatabaseFault};
use crate::open::Store;

pub use content::UnreadableContent;
pub use reading::Unreadable;

use reading::studio_row;

/// Version 77 — Studios, their nodes and their edges.
///
/// **The composite keys are what keep a graph on one Studio.** An edge's ends
/// reference `(studio_id, id)`, so an edge cannot join two Studios' nodes, and
/// removing a node removes every edge on it. The sets in each `CHECK` are
/// `core_model`'s, spelled as `as_wire` spells them.
pub(crate) const V77: &str = r#"
CREATE TABLE studios (
    id          TEXT PRIMARY KEY,
    manifest_id TEXT NOT NULL,
    name        TEXT,
    created_at  TEXT NOT NULL,
    touched_at  TEXT NOT NULL
) STRICT;

CREATE INDEX studios_by_manifest ON studios (manifest_id, touched_at);

CREATE TABLE studio_nodes (
    id         TEXT PRIMARY KEY,
    studio_id  TEXT NOT NULL REFERENCES studios (id) ON DELETE CASCADE,
    kind       TEXT NOT NULL CHECK (kind IN ('run', 'note', 'cluster', 'finding',
               'contradiction', 'sketch', 'link', 'deferral', 'outline', 'issue_draft', 'job')),
    state      TEXT CHECK (state IS NULL OR state IN ('proposed', 'gathering', 'frozen',
               'reported', 'issue_draft', 'deferral', 'not_a_problem', 'resolved_here', 'open',
               'answered', 'draft')),
    content    TEXT NOT NULL,
    x          INTEGER NOT NULL,
    y          INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (studio_id, id)
) STRICT;

CREATE TABLE studio_edges (
    id         TEXT PRIMARY KEY,
    studio_id  TEXT NOT NULL REFERENCES studios (id) ON DELETE CASCADE,
    from_node  TEXT NOT NULL,
    to_node    TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('produced', 'same_as', 'blocks', 'answers')),
    standing   TEXT NOT NULL CHECK (standing IN ('proposed', 'accepted')),
    created_at TEXT NOT NULL,
    FOREIGN KEY (studio_id, from_node) REFERENCES studio_nodes (studio_id, id) ON DELETE CASCADE,
    FOREIGN KEY (studio_id, to_node) REFERENCES studio_nodes (studio_id, id) ON DELETE CASCADE,
    UNIQUE (from_node, to_node, kind),
    CHECK (from_node <> to_node),
    CHECK (kind <> 'produced' OR standing = 'accepted')
) STRICT;
"#;

/// Version 78 — who put each thing on a Studio: a person or Helm. `#1288`.
///
/// **Nullable, and left null on every row before it.** Helm could add a node
/// under V77 too, so a default would claim an author nobody recorded; a null
/// reads back as unrecorded. A name gets one only where the Studio has a name.
pub(crate) const V78: &str = r#"
ALTER TABLE studios ADD COLUMN named_by TEXT
    CHECK (named_by IS NULL OR named_by IN ('person', 'helm'));
ALTER TABLE studio_nodes ADD COLUMN added_by TEXT
    CHECK (added_by IS NULL OR added_by IN ('person', 'helm'));
ALTER TABLE studio_edges ADD COLUMN added_by TEXT
    CHECK (added_by IS NULL OR added_by IN ('person', 'helm'));
"#;

/// Version 79 — an Issue, a Pull request and an Epic are node kinds. `#1394`.
///
/// **Both tables are rebuilt and no `ALTER TABLE` is used.** SQLite cannot
/// widen a `CHECK`, and a rename rewrites `studio_edges`' `REFERENCES
/// studio_nodes` to follow it — measured, an edge written afterwards failed on
/// *no such table: studio_nodes_narrow*, and `PRAGMA legacy_alter_table` did
/// not hold inside the migration's transaction.
///
/// **The edges go first.** Their foreign keys cascade and cannot be turned off
/// inside a transaction, so dropping the nodes under them would take them.
///
/// Every column and every other constraint is V77's and V78's. No row is
/// reclassified — a host is a literal the gate refuses in this crate, so Fleet
/// converts the Links it recognises on boot, `fleet::recognising`.
pub(crate) const V79: &str = r#"
CREATE TABLE studio_nodes_parked AS SELECT * FROM studio_nodes;
CREATE TABLE studio_edges_parked AS SELECT * FROM studio_edges;

DROP TABLE studio_edges;
DROP TABLE studio_nodes;

CREATE TABLE studio_nodes (
    id         TEXT PRIMARY KEY,
    studio_id  TEXT NOT NULL REFERENCES studios (id) ON DELETE CASCADE,
    kind       TEXT NOT NULL CHECK (kind IN ('run', 'note', 'cluster', 'finding',
               'contradiction', 'sketch', 'link', 'issue', 'pull_request', 'epic',
               'deferral', 'outline', 'issue_draft', 'job')),
    state      TEXT CHECK (state IS NULL OR state IN ('proposed', 'gathering', 'frozen',
               'reported', 'issue_draft', 'deferral', 'not_a_problem', 'resolved_here', 'open',
               'answered', 'draft')),
    content    TEXT NOT NULL,
    x          INTEGER NOT NULL,
    y          INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    added_by   TEXT CHECK (added_by IS NULL OR added_by IN ('person', 'helm')),
    UNIQUE (studio_id, id)
) STRICT;

CREATE TABLE studio_edges (
    id         TEXT PRIMARY KEY,
    studio_id  TEXT NOT NULL REFERENCES studios (id) ON DELETE CASCADE,
    from_node  TEXT NOT NULL,
    to_node    TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('produced', 'same_as', 'blocks', 'answers')),
    standing   TEXT NOT NULL CHECK (standing IN ('proposed', 'accepted')),
    created_at TEXT NOT NULL,
    added_by   TEXT CHECK (added_by IS NULL OR added_by IN ('person', 'helm')),
    FOREIGN KEY (studio_id, from_node) REFERENCES studio_nodes (studio_id, id) ON DELETE CASCADE,
    FOREIGN KEY (studio_id, to_node) REFERENCES studio_nodes (studio_id, id) ON DELETE CASCADE,
    UNIQUE (from_node, to_node, kind),
    CHECK (from_node <> to_node),
    CHECK (kind <> 'produced' OR standing = 'accepted')
) STRICT;

INSERT INTO studio_nodes (id, studio_id, kind, state, content, x, y, created_at, added_by)
SELECT id, studio_id, kind, state, content, x, y, created_at, added_by
FROM studio_nodes_parked;

INSERT INTO studio_edges (id, studio_id, from_node, to_node, kind, standing, created_at, added_by)
SELECT id, studio_id, from_node, to_node, kind, standing, created_at, added_by
FROM studio_edges_parked;

DROP TABLE studio_nodes_parked;
DROP TABLE studio_edges_parked;
"#;

/// Why a Studio read or write did not happen.
#[derive(Debug)]
pub enum StudioError {
    Database(DatabaseFault),
    NoSuchStudio {
        studio_id: String,
    },
    /// Named a node that is not on this Studio.
    NoSuchNode {
        node_id: String,
    },
    /// Named an edge that is not on this Studio.
    NoSuchEdge {
        edge_id: String,
    },
    /// The same two nodes already carry an edge of this kind.
    EdgeExists {
        from: String,
        to: String,
        kind: StudioEdgeKind,
    },
    /// Only a proposed edge is accepted or rejected.
    NotProposed {
        edge_id: String,
    },
    /// A row that does not read back as what was written.
    Unreadable {
        table: &'static str,
        id: String,
        why: Unreadable,
    },
}

impl core::fmt::Display for StudioError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StudioError::Database(fault) => write!(out, "{fault}"),
            StudioError::NoSuchStudio { studio_id } => write!(out, "no Studio is `{studio_id}`"),
            StudioError::NoSuchNode { node_id } => {
                write!(out, "no node on this Studio is `{node_id}`")
            }
            StudioError::NoSuchEdge { edge_id } => {
                write!(out, "no edge on this Studio is `{edge_id}`")
            }
            StudioError::EdgeExists { from, to, kind } => write!(
                out,
                "`{from}` and `{to}` already carry a `{}` edge",
                kind.as_wire()
            ),
            StudioError::NotProposed { edge_id } => write!(
                out,
                "edge `{edge_id}` is not proposed, so there is nothing to accept or reject"
            ),
            StudioError::Unreadable { table, id, why } => {
                write!(out, "{table} row `{id}` does not read back: {why:?}")
            }
        }
    }
}

impl std::error::Error for StudioError {}

fn database(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> StudioError {
    move |cause| StudioError::Database(fault(doing)(cause))
}

impl Store {
    /// Keep a new Studio.
    pub fn create_studio(&mut self, studio: &Studio) -> Result<(), StudioError> {
        self.conn
            .execute(
                "INSERT INTO studios (id, manifest_id, name, created_at, touched_at, named_by) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (
                    studio.id.as_str(),
                    studio.manifest_id.as_str(),
                    studio.name.as_ref().map(StudioName::as_str),
                    studio.created_at.as_str(),
                    studio.touched_at.as_str(),
                    studio.named_by.map(|by| by.as_wire()),
                ),
            )
            .map(|_| ())
            .map_err(database("keeping a new Studio"))
    }

    /// Every Studio this repository keeps, the last touched first.
    pub fn studios(&self, manifest_id: &ManifestId) -> Result<Vec<Studio>, StudioError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT id, manifest_id, name, created_at, touched_at, named_by FROM studios \
                 WHERE manifest_id = ?1 ORDER BY touched_at DESC, id DESC",
            )
            .map_err(database("reading a repository's Studios"))?;
        let rows = asking
            .query_map([manifest_id.as_str()], studio_row)
            .map_err(database("reading a repository's Studios"))?;
        rows.map(|row| row.map_err(database("reading one Studio")))
            .collect()
    }

    /// One Studio and everything on it.
    pub fn studio(&self, studio_id: &StudioId) -> Result<StudioGraph, StudioError> {
        let studio = self
            .conn
            .query_row(
                "SELECT id, manifest_id, name, created_at, touched_at, named_by FROM studios \
                 WHERE id = ?1",
                [studio_id.as_str()],
                studio_row,
            )
            .optional()
            .map_err(database("reading a Studio"))?
            .ok_or_else(|| StudioError::NoSuchStudio {
                studio_id: studio_id.as_str().to_string(),
            })?;
        Ok(StudioGraph {
            nodes: self.nodes_on(studio_id)?,
            edges: self.edges_on(studio_id)?,
            studio,
        })
    }

    /// Name a Studio, or name it again, as `by`.
    pub fn rename_studio(
        &mut self,
        studio_id: &StudioId,
        name: &StudioName,
        by: StudioAuthor,
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let tx = self.writing()?;
        tx.execute(
            "UPDATE studios SET name = ?2, named_by = ?3 WHERE id = ?1",
            (studio_id.as_str(), name.as_str(), by.as_wire()),
        )
        .map_err(database("renaming a Studio"))?;
        touched(&tx, studio_id, at)?;
        tx.commit().map_err(database("renaming a Studio"))
    }

    /// Delete a Studio, and every node and edge on it with it.
    pub fn delete_studio(&mut self, studio_id: &StudioId) -> Result<(), StudioError> {
        let deleted = self
            .conn
            .execute("DELETE FROM studios WHERE id = ?1", [studio_id.as_str()])
            .map_err(database("deleting a Studio"))?;
        match deleted {
            0 => Err(StudioError::NoSuchStudio {
                studio_id: studio_id.as_str().to_string(),
            }),
            _ => Ok(()),
        }
    }

    /// Add a node, and the Studio's own `Produced` edge where another node on it
    /// made this one. The edge is built here, so its far end cannot be anything
    /// but the node being added.
    pub fn add_studio_node(
        &mut self,
        studio_id: &StudioId,
        node: &StudioNode,
        produced_by: Option<(&StudioNodeId, StudioEdgeId)>,
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let made_it: Vec<_> = produced_by.into_iter().collect();
        self.add_studio_node_produced_by(studio_id, node, &made_it, at)
    }

    /// Add a node made by **several** nodes already on this Studio, with one
    /// `Produced` edge from each, in the order given. `#1291`: a Cluster is
    /// the Notes a person accepted as one thing, and an Outline is an ordered
    /// reading of what feeds it, so one maker is not enough for either.
    ///
    /// **The edges are built here**, so no caller can point one anywhere but
    /// at the node being added.
    pub fn add_studio_node_produced_by(
        &mut self,
        studio_id: &StudioId,
        node: &StudioNode,
        produced_by: &[(&StudioNodeId, StudioEdgeId)],
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let tx = self.writing()?;
        touched(&tx, studio_id, at)?;
        node_kept(&tx, studio_id, node)?;
        for (from, id) in produced_by {
            let by = node.added_by().unwrap_or(StudioAuthor::Person);
            let edge = StudioEdge::produced(
                id.clone(),
                (*from).clone(),
                node.id().clone(),
                at.clone(),
                by,
            )
            // Only reachable by naming the node being added as its own maker.
            .map_err(|_| StudioError::NoSuchNode {
                node_id: from.as_str().to_string(),
            })?;
            edge_kept(&tx, studio_id, &edge)?;
        }
        tx.commit().map_err(database("adding a node to a Studio"))
    }

    /// Move a node to where a person left it.
    pub fn move_studio_node(
        &mut self,
        studio_id: &StudioId,
        node_id: &StudioNodeId,
        to: StudioPosition,
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let tx = self.writing()?;
        touched(&tx, studio_id, at)?;
        let moved = tx
            .execute(
                "UPDATE studio_nodes SET x = ?3, y = ?4 WHERE studio_id = ?1 AND id = ?2",
                (studio_id.as_str(), node_id.as_str(), to.x, to.y),
            )
            .map_err(database("moving a node"))?;
        if moved == 0 {
            return Err(StudioError::NoSuchNode {
                node_id: node_id.as_str().to_string(),
            });
        }
        tx.commit().map_err(database("moving a node"))
    }

    /// Remove a node, and every edge on it.
    pub fn remove_studio_node(
        &mut self,
        studio_id: &StudioId,
        node_id: &StudioNodeId,
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let tx = self.writing()?;
        touched(&tx, studio_id, at)?;
        let removed = tx
            .execute(
                "DELETE FROM studio_nodes WHERE studio_id = ?1 AND id = ?2",
                (studio_id.as_str(), node_id.as_str()),
            )
            .map_err(database("removing a node"))?;
        if removed == 0 {
            return Err(StudioError::NoSuchNode {
                node_id: node_id.as_str().to_string(),
            });
        }
        tx.commit().map_err(database("removing a node"))
    }

    /// Keep an edge between two nodes already on this Studio.
    pub fn add_studio_edge(
        &mut self,
        studio_id: &StudioId,
        edge: &StudioEdge,
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let tx = self.writing()?;
        touched(&tx, studio_id, at)?;
        edge_kept(&tx, studio_id, edge)?;
        tx.commit().map_err(database("keeping an edge"))
    }

    /// Accept a proposed edge, or reject it, which removes it.
    pub fn decide_studio_edge(
        &mut self,
        studio_id: &StudioId,
        edge_id: &StudioEdgeId,
        accepted: bool,
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let tx = self.writing()?;
        touched(&tx, studio_id, at)?;
        let standing: Option<String> = tx
            .query_row(
                "SELECT standing FROM studio_edges WHERE studio_id = ?1 AND id = ?2",
                (studio_id.as_str(), edge_id.as_str()),
                |row| row.get(0),
            )
            .optional()
            .map_err(database("reading an edge's standing"))?;
        let edge = || edge_id.as_str().to_string();
        match standing.as_deref() {
            None => return Err(StudioError::NoSuchEdge { edge_id: edge() }),
            Some(standing) if standing != StudioEdgeStanding::Proposed.as_wire() => {
                return Err(StudioError::NotProposed { edge_id: edge() })
            }
            Some(_) => {}
        }
        let statement = match accepted {
            true => "UPDATE studio_edges SET standing = 'accepted' WHERE id = ?1",
            false => "DELETE FROM studio_edges WHERE id = ?1",
        };
        tx.execute(statement, [edge_id.as_str()])
            .map_err(database("deciding an edge"))?;
        tx.commit().map_err(database("deciding an edge"))
    }

    /// Keep a node as a promotion left it: an Issue draft a person edited, or
    /// a Contradiction they ended. **Takes [`Rewritten`], which only
    /// `core_model`'s own two transitions make**, the way `keep_scouted` takes
    /// [`Scouted`](core_model::Scouted) — so no call here can hand a Note new
    /// words. The row is matched on its kind as well as its id.
    /// Every Link on every Studio, each with the Studio it is on — what a
    /// conversion has to look at. `#1394`.
    ///
    /// **Links alone.** A row already one of the three forge kinds was
    /// converted or written as one, so nothing here needs to see it.
    pub fn every_link(&self) -> Result<Vec<(StudioId, StudioNode)>, StudioError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT studio_id FROM studio_nodes WHERE kind = ?1 GROUP BY studio_id \
                 ORDER BY studio_id",
            )
            .map_err(database("reading every Studio holding a Link"))?;
        let studios: Vec<String> = asking
            .query_map([core_model::StudioNodeKind::Link.as_wire()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(database("reading every Studio holding a Link"))?
            .collect::<rusqlite::Result<Vec<String>>>()
            .map_err(database("reading one Studio holding a Link"))?;
        let mut links = Vec::new();
        for studio in studios {
            let id = StudioId::carried(core_model::Ulid::carried(studio));
            for node in self.nodes_on(&id)? {
                if node.kind() == core_model::StudioNodeKind::Link {
                    links.push((id.clone(), node));
                }
            }
        }
        Ok(links)
    }

    /// A Link written back as the kind its address turned out to name.
    /// `#1394`.
    ///
    /// **Takes [`Recognised`], which only a Link becoming one of the three
    /// makes**, so the one write on a Studio that changes a node's kind cannot
    /// reach a Note and cannot move an address.
    ///
    /// **The Studio is not touched.** A conversion is nobody's write: it is
    /// what this build reads the record as, and touching every Studio holding
    /// a Link would reorder the list the first time Fleet started.
    pub fn recognise_studio_node(
        &mut self,
        studio_id: &StudioId,
        recognised: &Recognised,
    ) -> Result<(), StudioError> {
        let node = recognised.node();
        self.conn
            .execute(
                "UPDATE studio_nodes SET kind = ?3, content = ?4 \
                 WHERE studio_id = ?1 AND id = ?2 AND kind = ?5",
                (
                    studio_id.as_str(),
                    node.id().as_str(),
                    node.kind().as_wire(),
                    content::written(node.content()),
                    core_model::StudioNodeKind::Link.as_wire(),
                ),
            )
            .map(|_| ())
            .map_err(database("recognising what a Link's address names"))
    }

    pub fn keep_rewritten(
        &mut self,
        studio_id: &StudioId,
        rewritten: &Rewritten,
        at: &Timestamp,
    ) -> Result<(), StudioError> {
        let node = rewritten.node();
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
                    content::written(node.content()),
                    node.kind().as_wire(),
                ),
            )
            .map_err(database("keeping what a promotion wrote"))?;
        if kept == 0 {
            return Err(StudioError::NoSuchNode {
                node_id: node.id().as_str().to_string(),
            });
        }
        tx.commit()
            .map_err(database("keeping what a promotion wrote"))
    }

    fn writing(&mut self) -> Result<Transaction<'_>, StudioError> {
        self.conn
            .transaction()
            .map_err(database("starting a Studio write"))
    }
}

/// Move a Studio's `touched_at`, refusing a Studio that is not there.
fn touched(tx: &Transaction<'_>, studio_id: &StudioId, at: &Timestamp) -> Result<(), StudioError> {
    let touched = tx
        .execute(
            "UPDATE studios SET touched_at = ?2 WHERE id = ?1",
            (studio_id.as_str(), at.as_str()),
        )
        .map_err(database("touching a Studio"))?;
    match touched {
        0 => Err(StudioError::NoSuchStudio {
            studio_id: studio_id.as_str().to_string(),
        }),
        _ => Ok(()),
    }
}

/// Write one edge, naming which end is missing or that it already exists.
/// One node, inserted. **Every write that adds one goes through here**, so a
/// column added to the table is added in one place.
fn node_kept(
    tx: &Transaction<'_>,
    studio_id: &StudioId,
    node: &StudioNode,
) -> Result<(), StudioError> {
    tx.execute(
        "INSERT INTO studio_nodes \
         (id, studio_id, kind, state, content, x, y, created_at, added_by) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        (
            node.id().as_str(),
            studio_id.as_str(),
            node.kind().as_wire(),
            node.state().map(|state| state.as_wire()),
            content::written(node.content()),
            node.position().x,
            node.position().y,
            node.created_at().as_str(),
            node.added_by().map(|by| by.as_wire()),
        ),
    )
    .map(|_| ())
    .map_err(database("adding a node to a Studio"))
}

fn edge_kept(
    tx: &Transaction<'_>,
    studio_id: &StudioId,
    edge: &StudioEdge,
) -> Result<(), StudioError> {
    for end in [edge.from(), edge.to()] {
        let there = tx
            .query_row(
                "SELECT 1 FROM studio_nodes WHERE studio_id = ?1 AND id = ?2",
                (studio_id.as_str(), end.as_str()),
                |_| Ok(()),
            )
            .optional()
            .map_err(database("finding an edge's node"))?;
        if there.is_none() {
            return Err(StudioError::NoSuchNode {
                node_id: end.as_str().to_string(),
            });
        }
    }
    let exists = tx
        .query_row(
            "SELECT 1 FROM studio_edges WHERE from_node = ?1 AND to_node = ?2 AND kind = ?3",
            (
                edge.from().as_str(),
                edge.to().as_str(),
                edge.kind().as_wire(),
            ),
            |_| Ok(()),
        )
        .optional()
        .map_err(database("finding an edge like it"))?;
    if exists.is_some() {
        return Err(StudioError::EdgeExists {
            from: edge.from().as_str().to_string(),
            to: edge.to().as_str().to_string(),
            kind: edge.kind(),
        });
    }
    tx.execute(
        "INSERT INTO studio_edges \
         (id, studio_id, from_node, to_node, kind, standing, created_at, added_by) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            edge.id().as_str(),
            studio_id.as_str(),
            edge.from().as_str(),
            edge.to().as_str(),
            edge.kind().as_wire(),
            edge.standing().as_wire(),
            edge.created_at().as_str(),
            edge.added_by().map(|by| by.as_wire()),
        ),
    )
    .map(|_| ())
    .map_err(database("keeping an edge"))
}
