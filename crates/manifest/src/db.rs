//! `~/.armada/manifest.db` — machine-global, SQLite (PLAN.md §4.3).
//!
//! The only cross-workspace state, and the only thing that survives a workspace
//! directory being deleted. **SQLite rather than a JSON file because of
//! leases**: a ten-minute `armada manifest check` renews a heartbeat every few seconds,
//! and rewriting a whole document under an `O_EXCL` lockfile, five workspaces
//! at a time, for the whole of that run, is the wrong shape for that write
//! pattern — and it is exactly where the registry-corruption risk lives.
//!
//! **Connection setup is not optional**, and each of the three is a measured
//! failure rather than a convention:
//!
//! - `journal_mode = WAL` is a property of the *file*, not a driver default —
//!   and switching a fresh database into it is the one statement `busy_timeout`
//!   does **not** cover, so it carries its own wait (see [`set_wal`]).
//! - `busy_timeout` is set explicitly, and *first*; relying on a driver's is how
//!   it changes under you, and setting it after another statement leaves that
//!   statement running at a timeout of zero.
//! - **every transaction that may write is `BEGIN IMMEDIATE`.** A DEFERRED
//!   transaction that reads and then writes fails after **0.0 ms** with
//!   `SQLITE_BUSY_SNAPSHOT`, which `busy_timeout` cannot rescue because the
//!   reader's snapshot is already stale. That is the lease pattern exactly: the
//!   obvious lease code works in every test with one writer and fails
//!   nondeterministically under the contention the design exists to handle.

use armada_core::error::{ArmadaError, ErrClass};
use armada_core::id::{ProjectId, WorkspaceId};
use armada_core::lease::{Holder, LeaseId, LeaseKind};
use armada_core::ports::{choose_block, PortBlock};
use armada_core::registry::{LeaseRow, OwnedKind, OwnedRow, WorkspaceRow};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The schema version this binary writes and understands.
pub const USER_VERSION: i64 = 2;

/// How long any one statement waits for a database another Armada is writing.
const BUSY_TIMEOUT_MS: u64 = 5_000;

/// The key the namespace UUID is stored under.
const NAMESPACE_KEY: &str = "namespace";

/// What [`Db::peek_stats`] found: a row count per table, and how much of the
/// ownership store is reclaimable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stats {
    /// `(table name, row count)`, in the order `sqlite_master` names them —
    /// derived, so a table `migrate` adds shows up here without this struct
    /// changing.
    pub tables: Vec<(String, usize)>,
    /// `owned` rows whose workspace's directory is already gone.
    pub reclaimable_owned: usize,
    /// `workspaces` rows in the same state.
    pub reclaimable_workspaces: usize,
}

/// The machine-global store.
#[derive(Debug)]
pub struct Db {
    conn: Connection,
    path: PathBuf,
    /// A handle on the same inode, opened with the connection.
    ///
    /// It exists for one measurement: **`fstat` on a process's own open handle
    /// returns `st_nlink == 0` once the file is unlinked.** Deleting `manifest.db`
    /// while a process holds it open under WAL lets that process keep reading
    /// and writing a consistent world through the unlinked inode, while the
    /// next process creates a fresh file at the same path and hands out a port
    /// block the first one already holds. **Neither errors.** The holder can
    /// detect it; the newcomer provably cannot — to it, the unlinked case is
    /// byte-identical to a genuine fresh install.
    handle: std::fs::File,
}

/// What claiming a port block did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimOutcome {
    /// The workspace is registered. `None` when it needs no block — a
    /// registration rather than a claim, which is still what makes the
    /// workspace reclaimable after its directory is gone.
    Claimed(Option<PortBlock>),
    /// This workspace was already registered. **Claims are idempotent by
    /// workspace id** — `armada manifest init` twice is one block, not two.
    AlreadyHeld(Option<PortBlock>),
    /// Another workspace took the block Armada chose between the read and the
    /// write. The caller re-decides; that is what the claim loop's `Attempt`
    /// action means.
    Lost,
    /// The machine has no free block of this size left.
    Exhausted,
}

/// What acquiring a lease did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquireOutcome {
    /// The row is ours.
    Granted,
    /// Someone else has it. The caller decides whether that holder is *live*
    /// or cold — the coldness rule is pure and lives in the core.
    Held(LeaseRow),
}

/// What an all-or-nothing slot claim came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotOutcome {
    /// Every slot asked for, with the keys the store chose.
    Granted(Vec<LeaseId>),
    /// Not enough were free. `held` is one of the holders, for the `WAITING`
    /// row; `None` when every slot is free and the machine is simply smaller
    /// than the request, which the caller has already clamped for.
    Short {
        /// One holder, so the payload can name a workspace.
        held: Option<LeaseRow>,
        /// How many were free at that instant — this run's own view, which is
        /// the number PLAN.md §3.1's `waiting_on.available` reports.
        free: u32,
    },
}

impl Db {
    /// Open, creating the database and its schema if this is a first run.
    pub fn open(armada_home: &Path) -> Result<Self, ArmadaError> {
        std::fs::create_dir_all(armada_home).map_err(|e| {
            environment(
                armada_home.display().to_string(),
                format!("cannot create {}: {e}", armada_home.display()),
            )
        })?;
        let path = armada_home.join("manifest.db");
        let conn = Connection::open(&path).map_err(|e| map_sqlite(&path, e))?;

        // `busy_timeout` goes first, because a timeout set second does nothing
        // for the statement that ran before it.
        conn.pragma_update(None, "busy_timeout", BUSY_TIMEOUT_MS as i64)
            .map_err(|e| map_sqlite(&path, e))?;
        set_wal(&conn, &path)?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|e| map_sqlite(&path, e))?;

        let handle = std::fs::File::open(&path).map_err(|e| {
            environment(
                path.display().to_string(),
                format!("cannot open manifest.db: {e}"),
            )
        })?;

        let mut db = Db { conn, path, handle };
        db.migrate()?;
        Ok(db)
    }

    /// Create or upgrade the schema.
    ///
    /// **The compatibility rule is deliberately one-directional.**
    /// `~/.armada/manifest.db` is machine-global and long-lived, so a machine running
    /// two Armada versions — one repo pinned, one fresh — is normal rather
    /// than exotic. An older binary meeting a higher `user_version` fails
    /// `environment` and says which version wrote it; a newer binary meeting a
    /// lower one migrates it forward in a single `BEGIN IMMEDIATE`, additively.
    /// Schema changes are additive for the whole 0.x line: new column, never a
    /// dropped or retyped one.
    ///
    /// **`user_version` is the flag that says the whole of creation is done, so
    /// everything creation does commits with it.** The namespace used to be
    /// written by a second statement after the transaction, and a sibling `Armada
    /// init` starting in the same millisecond then read `user_version = 1`,
    /// returned here early, and asked for a namespace that was not there yet —
    /// `manifest.db: Query returned no rows`, from a database that was merely
    /// half a heartbeat young.
    fn migrate(&mut self) -> Result<(), ArmadaError> {
        let version: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|e| map_sqlite(&self.path, e))?;

        if version > USER_VERSION {
            return Err(ArmadaError {
                class: ErrClass::Environment,
                r#where: self.path.display().to_string(),
                message: format!(
                    "{} was written by a newer Armada (schema {version}; this one understands \
                     {USER_VERSION})",
                    self.path.display()
                ),
                next_action: Some(
                    "upgrade Armada, or point $HOME at a different store".to_string(),
                ),
            });
        }
        if version == USER_VERSION {
            return Ok(());
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (
                 key   TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS workspaces (
                 id         TEXT PRIMARY KEY,
                 path       TEXT NOT NULL,
                 project    TEXT,
                 port_from  INTEGER NOT NULL,
                 port_to    INTEGER NOT NULL,
                 claimed_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS owned (
                 workspace      TEXT NOT NULL,
                 kind           TEXT NOT NULL,
                 \"ref\"          TEXT NOT NULL,
                 boot_id        TEXT,
                 pid_started_at TEXT,
                 component      TEXT,
                 PRIMARY KEY (workspace, kind, \"ref\")
             );
             CREATE TABLE IF NOT EXISTS leases (
                 workspace      TEXT,
                 kind           TEXT NOT NULL,
                 key            TEXT NOT NULL,
                 heartbeat_mono INTEGER NOT NULL,
                 boot_id        TEXT NOT NULL,
                 pid            INTEGER NOT NULL,
                 pid_started_at TEXT,
                 PRIMARY KEY (kind, key)
             );",
        )
        .map_err(|e| map_sqlite(&self.path, e))?;

        // **Schema 2, added the additive way the rule above states.** A store
        // created fresh gets the column from the `CREATE` above; one already at
        // schema 1 gets it here, and the duplicate-column error the fresh path
        // raises is the "already correct" answer rather than a failure. The
        // column is nullable, so every row written before it existed reads back
        // as `None` — which is exactly what it means: Armada did not record
        // which component this handle belongs to, so no selector can claim it.
        if let Err(e) = tx.execute_batch("ALTER TABLE owned ADD COLUMN component TEXT") {
            let already = e.to_string().contains("duplicate column");
            if !already {
                return Err(map_sqlite(&self.path, e));
            }
        }

        // The namespace is written once, at creation, and never again. It
        // scopes the whole reaping mechanism to one filesystem view: without
        // it, path-based reaping is actively dangerous the moment two Armada
        // installations share a Docker daemon, which is the ordinary
        // devcontainer setup (PLAN.md §2.3.1).
        tx.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES (?1, ?2)",
            (NAMESPACE_KEY, random_uuid()?),
        )
        .map_err(|e| map_sqlite(&self.path, e))?;

        // Last inside the transaction, and committed with it: this is the
        // announcement that the two above have happened.
        tx.pragma_update(None, "user_version", USER_VERSION)
            .map_err(|e| map_sqlite(&self.path, e))?;
        tx.commit().map_err(|e| map_sqlite(&self.path, e))
    }

    /// Read the namespace out of a database file, best-effort, swallowing
    /// every failure.
    ///
    /// For the rebuild path only. `armada manifest clean --orphaned --force-rebuild`
    /// exists because `manifest.db` cannot be read, so it must not *need* this to
    /// work — but a database can be unreadable in ways that still leave `meta`
    /// legible, and carrying the old namespace across keeps every resource
    /// already stamped with it reapable. Failing to read it costs a namespace,
    /// not the recovery.
    pub fn peek_namespace(path: &Path) -> Option<String> {
        let conn =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
        conn.query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [NAMESPACE_KEY],
            |row| row.get(0),
        )
        .ok()
    }

    /// A row count per table, plus how much of `owned` and `workspaces` is
    /// reclaimable — read without writing anything, best-effort like
    /// [`Db::peek_namespace`], and for the same reason: `armada doctor` promises
    /// never to write to `~/.armada/`, and [`Db::open`] can migrate the schema.
    ///
    /// **Presence is not health.** A store that exists and holds four thousand
    /// dead rows is a different answer from one that merely exists, and this is
    /// what lets `doctor` tell the two apart (`docs/reserved/009-smaller-things-raised-in-use.md`
    /// item 1).
    ///
    /// **The table list comes from `sqlite_master`, never from a name written
    /// here.** A table added to [`Db::migrate`] and never added to a hand-kept
    /// list is the exact defect item 5 of the same document fixed for
    /// `armada --help`: a report that stopped tracking what it describes.
    ///
    /// **"Reclaimable" is [`armada_core::reap::registry_pass`]'s rule and no
    /// other** — a `workspaces` row whose path is `PathStat::Missing`, and an
    /// `owned` row pointing at one. Doctor asking a second question about
    /// staleness is how a doctor row and a `clean --orphaned` disagree about
    /// what is dead.
    pub fn peek_stats(path: &Path) -> Option<Stats> {
        let conn =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;

        let names: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' ORDER BY name",
            )
            .ok()?
            .query_map([], |row| row.get(0))
            .ok()?
            .collect::<Result<_, _>>()
            .ok()?;

        let tables: Vec<(String, usize)> = names
            .into_iter()
            .map(|name| {
                let count: i64 = conn
                    .query_row(&format!("SELECT COUNT(*) FROM \"{name}\""), [], |row| {
                        row.get(0)
                    })
                    .unwrap_or(0);
                (name, count as usize)
            })
            .collect();

        let workspaces: Vec<(String, PathBuf)> = conn
            .prepare("SELECT id, path FROM workspaces")
            .ok()?
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    PathBuf::from(row.get::<_, String>(1)?),
                ))
            })
            .ok()?
            .collect::<Result<_, _>>()
            .ok()?;

        let rows: Vec<(
            armada_core::registry::WorkspaceRow,
            armada_core::reap::PathStat,
        )> = workspaces
            .into_iter()
            .map(|(id, path)| {
                let stat = crate::fs::stat(&path);
                (
                    armada_core::registry::WorkspaceRow {
                        id: armada_core::id::WorkspaceId::from_stored(id),
                        path,
                        project: None,
                        ports: None,
                        claimed_at: String::new(),
                    },
                    stat,
                )
            })
            .collect();
        let dead: std::collections::BTreeSet<String> = armada_core::reap::registry_pass(&rows)
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect();

        let owners: Vec<String> = conn
            .prepare("SELECT workspace FROM owned")
            .ok()?
            .query_map([], |row| row.get(0))
            .ok()?
            .collect::<Result<_, _>>()
            .ok()?;

        Some(Stats {
            tables,
            reclaimable_workspaces: dead.len(),
            reclaimable_owned: owners.iter().filter(|w| dead.contains(*w)).count(),
        })
    }

    /// Carry a namespace recovered from a replaced database into this one.
    pub fn adopt_namespace(&mut self, namespace: &str) -> Result<(), ArmadaError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;
        tx.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            (NAMESPACE_KEY, namespace),
        )
        .map_err(|e| map_sqlite(&self.path, e))?;
        tx.commit().map_err(|e| map_sqlite(&self.path, e))
    }

    /// This installation's namespace.
    pub fn namespace(&self) -> Result<String, ArmadaError> {
        self.conn
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                [NAMESPACE_KEY],
                |row| row.get(0),
            )
            .map_err(|e| map_sqlite(&self.path, e))
    }

    /// Whether the file Armada is writing still has a name.
    ///
    /// **A long-running verb re-checks this on each loop iteration and ends
    /// `environment` when it goes false**, which stops the divergence at the
    /// process that can actually see it. The newcomer proceeding as a fresh
    /// install is then correct rather than merely unavoidable: by the time it
    /// matters, the holder has stopped.
    pub fn still_linked(&self) -> bool {
        use std::os::unix::fs::MetadataExt;
        self.handle
            .metadata()
            .map(|m| m.nlink() > 0)
            .unwrap_or(false)
    }

    /// Every registered workspace.
    pub fn workspaces(&self) -> Result<Vec<WorkspaceRow>, ArmadaError> {
        let mut statement = self
            .conn
            .prepare("SELECT id, path, project, port_from, port_to, claimed_at FROM workspaces")
            .map_err(|e| map_sqlite(&self.path, e))?;
        let rows = statement
            .query_map([], |row| {
                Ok(WorkspaceRow {
                    id: WorkspaceId::from_stored(row.get::<_, String>(0)?),
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    project: row.get::<_, Option<String>>(2)?.map(ProjectId::from_stored),
                    ports: block_of(row.get::<_, i64>(3)?, row.get::<_, i64>(4)?),
                    claimed_at: row.get(5)?,
                })
            })
            .map_err(|e| map_sqlite(&self.path, e))?;
        rows.collect::<Result<_, _>>()
            .map_err(|e| map_sqlite(&self.path, e))
    }

    /// Register this workspace, claiming a port block if it needs one.
    ///
    /// The choice and the write happen inside **one** `BEGIN IMMEDIATE`, which
    /// is what makes "two directories claim non-overlapping blocks
    /// concurrently" true rather than usually true.
    ///
    /// **`size: None` registers without claiming**, for a workspace whose
    /// components declare no `ports:` ([`armada_core::ports::needs_block`]).
    /// The row is still written: it is what makes the workspace reclaimable
    /// after its directory is gone, and that has nothing to do with ports.
    ///
    /// **`base` is the machine's `port_base`**, travelling with `size` because
    /// the two describe one pool and neither is this layer's to decide. The
    /// store is machine-global while the base is per-machine-configuration, so
    /// a row written from a different floor is data here and not a default.
    pub fn claim_block(
        &mut self,
        workspace: &WorkspaceId,
        path: &Path,
        project: Option<&ProjectId>,
        size: Option<u16>,
        base: u16,
        claimed_at: &str,
    ) -> Result<ClaimOutcome, ArmadaError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;

        let held: Option<(i64, i64)> = tx
            .query_row(
                "SELECT port_from, port_to FROM workspaces WHERE id = ?1",
                [workspace.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| map_sqlite(&self.path, e))?;

        if let Some((from, to)) = held {
            // Idempotent by workspace id, but the path and project are
            // refreshed: a checkout that moved keeps its block and gets a
            // correct path label, which pass 2 of the reap stats.
            tx.execute(
                "UPDATE workspaces SET path = ?2, project = ?3 WHERE id = ?1",
                (
                    workspace.as_str(),
                    path.display().to_string(),
                    project.map(|p| p.as_str().to_string()),
                ),
            )
            .map_err(|e| map_sqlite(&self.path, e))?;
            tx.commit().map_err(|e| map_sqlite(&self.path, e))?;
            return Ok(ClaimOutcome::AlreadyHeld(block_of(from, to)));
        }

        let block = match size {
            None => None,
            Some(size) => {
                let taken: Vec<PortBlock> = {
                    let mut statement = tx
                        .prepare("SELECT port_from, port_to FROM workspaces")
                        .map_err(|e| map_sqlite(&self.path, e))?;
                    let rows = statement
                        .query_map([], |row| {
                            Ok(block_of(row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
                        })
                        .map_err(|e| map_sqlite(&self.path, e))?;
                    // A workspace holding no block reserves nothing, so it is
                    // not something a chooser has to route around.
                    rows.collect::<Result<Vec<_>, _>>()
                        .map_err(|e| map_sqlite(&self.path, e))?
                        .into_iter()
                        .flatten()
                        .collect()
                };

                let Some(block) = choose_block(&taken, size, base) else {
                    tx.rollback().map_err(|e| map_sqlite(&self.path, e))?;
                    return Ok(ClaimOutcome::Exhausted);
                };
                Some(block)
            }
        };

        let (from, to) = stored(block);
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO workspaces (id, path, project, port_from, port_to, claimed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                workspace.as_str(),
                path.display().to_string(),
                project.map(|p| p.as_str().to_string()),
                from,
                to,
                claimed_at,
            ),
        );
        let inserted = inserted.map_err(|e| map_sqlite(&self.path, e))?;
        tx.commit().map_err(|e| map_sqlite(&self.path, e))?;

        Ok(if inserted == 1 {
            ClaimOutcome::Claimed(block)
        } else {
            ClaimOutcome::Lost
        })
    }

    /// Drop a workspace row and everything it owns, **except the leases the
    /// caller is itself holding**.
    ///
    /// `keep` is not a convenience. `clean` tears a workspace down while
    /// holding that workspace's own run lease, and the whole ordering rests on
    /// that lease being held throughout and released last: a blanket
    /// `DELETE FROM leases WHERE workspace = ?1` frees it half way through, and
    /// a concurrent `armada manifest up` then starts services into a workspace being torn
    /// down — the exact failure `clean`'s step 0 is written against.
    pub fn release_workspace(
        &mut self,
        workspace: &WorkspaceId,
        keep: &[LeaseId],
    ) -> Result<(), ArmadaError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;
        tx.execute(
            "DELETE FROM owned WHERE workspace = ?1",
            [workspace.as_str()],
        )
        .map_err(|e| map_sqlite(&self.path, e))?;

        let doomed: Vec<(String, String)> = {
            let mut statement = tx
                .prepare("SELECT kind, key FROM leases WHERE workspace = ?1")
                .map_err(|e| map_sqlite(&self.path, e))?;
            let rows = statement
                .query_map([workspace.as_str()], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| map_sqlite(&self.path, e))?;
            rows.collect::<Result<_, _>>()
                .map_err(|e| map_sqlite(&self.path, e))?
        };
        for (kind, key) in doomed {
            if keep
                .iter()
                .any(|held| held.kind.to_string() == kind && held.key == key)
            {
                continue;
            }
            tx.execute(
                "DELETE FROM leases WHERE kind = ?1 AND key = ?2",
                (&kind, &key),
            )
            .map_err(|e| map_sqlite(&self.path, e))?;
        }

        tx.execute("DELETE FROM workspaces WHERE id = ?1", [workspace.as_str()])
            .map_err(|e| map_sqlite(&self.path, e))?;
        tx.commit().map_err(|e| map_sqlite(&self.path, e))
    }

    /// Everything one workspace owns, or everything on the machine.
    pub fn owned(&self, workspace: Option<&WorkspaceId>) -> Result<Vec<OwnedRow>, ArmadaError> {
        let sql = "SELECT workspace, kind, \"ref\", boot_id, pid_started_at, component FROM owned";
        let mut statement = self
            .conn
            .prepare(&match workspace {
                Some(_) => format!("{sql} WHERE workspace = ?1"),
                None => sql.to_string(),
            })
            .map_err(|e| map_sqlite(&self.path, e))?;

        let map = |row: &rusqlite::Row<'_>| {
            let kind: String = row.get(1)?;
            Ok((
                OwnedKind::parse(&kind),
                OwnedRow {
                    workspace: WorkspaceId::from_stored(row.get::<_, String>(0)?),
                    kind: OwnedKind::Container,
                    reference: row.get(2)?,
                    boot_id: row.get(3)?,
                    pid_started_at: row.get(4)?,
                    component: row.get(5)?,
                },
            ))
        };

        let rows: Vec<(Option<OwnedKind>, OwnedRow)> = match workspace {
            Some(workspace) => statement
                .query_map([workspace.as_str()], map)
                .map_err(|e| map_sqlite(&self.path, e))?
                .collect::<Result<_, _>>()
                .map_err(|e| map_sqlite(&self.path, e))?,
            None => statement
                .query_map([], map)
                .map_err(|e| map_sqlite(&self.path, e))?
                .collect::<Result<_, _>>()
                .map_err(|e| map_sqlite(&self.path, e))?,
        };

        // A row whose kind this binary does not recognise was written by a
        // newer Armada. Skipping it is the forward-compatible answer: Armada never
        // acts on something it cannot name, and never deletes it either.
        Ok(rows
            .into_iter()
            .filter_map(|(kind, row)| kind.map(|kind| OwnedRow { kind, ..row }))
            .collect())
    }

    /// Record something this workspace owns.
    ///
    /// **`up` records before it spawns, and this is the opposite of `clean`'s
    /// order.** Both follow one rule — *the failure mode must be a stale row,
    /// never an untracked resource* — and it inverts because one direction is
    /// creating and the other destroying. Spawn-then-record leaks a pgid if
    /// Armada dies in between; record-then-spawn leaves a row pointing at
    /// nothing, which the next `init` reaps for free.
    pub fn record_owned(&mut self, row: &OwnedRow) -> Result<(), ArmadaError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;
        tx.execute(
            "INSERT OR REPLACE INTO owned
                 (workspace, kind, \"ref\", boot_id, pid_started_at, component)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                row.workspace.as_str(),
                row.kind.to_string(),
                &row.reference,
                &row.boot_id,
                &row.pid_started_at,
                &row.component,
            ),
        )
        .map_err(|e| map_sqlite(&self.path, e))?;
        tx.commit().map_err(|e| map_sqlite(&self.path, e))
    }

    /// Forget something, after it has actually been removed.
    pub fn delete_owned(
        &mut self,
        workspace: &WorkspaceId,
        kind: OwnedKind,
        reference: &str,
    ) -> Result<(), ArmadaError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;
        tx.execute(
            "DELETE FROM owned WHERE workspace = ?1 AND kind = ?2 AND \"ref\" = ?3",
            (workspace.as_str(), kind.to_string(), reference),
        )
        .map_err(|e| map_sqlite(&self.path, e))?;
        tx.commit().map_err(|e| map_sqlite(&self.path, e))
    }

    /// Drop every `owns.release:` record for a workspace, so `init` can
    /// re-record the current declarations rather than accumulating old ones.
    pub fn clear_kind(
        &mut self,
        workspace: &WorkspaceId,
        kind: OwnedKind,
    ) -> Result<(), ArmadaError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;
        tx.execute(
            "DELETE FROM owned WHERE workspace = ?1 AND kind = ?2",
            (workspace.as_str(), kind.to_string()),
        )
        .map_err(|e| map_sqlite(&self.path, e))?;
        tx.commit().map_err(|e| map_sqlite(&self.path, e))
    }

    /// Every held lease.
    pub fn leases(&self) -> Result<Vec<LeaseRow>, ArmadaError> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT workspace, kind, key, heartbeat_mono, boot_id, pid, pid_started_at
                 FROM leases",
            )
            .map_err(|e| map_sqlite(&self.path, e))?;
        let rows = statement
            .query_map([], |row| {
                Ok(LeaseRow {
                    workspace: row
                        .get::<_, Option<String>>(0)?
                        .map(WorkspaceId::from_stored),
                    kind: parse_kind(&row.get::<_, String>(1)?),
                    key: row.get(2)?,
                    heartbeat_mono: row.get::<_, i64>(3)? as u64,
                    boot_id: row.get(4)?,
                    pid: row.get::<_, i64>(5)? as i32,
                    pid_started_at: row.get(6)?,
                })
            })
            .map_err(|e| map_sqlite(&self.path, e))?;
        rows.collect::<Result<_, _>>()
            .map_err(|e| map_sqlite(&self.path, e))
    }

    /// One attempt at a lease, inside one `BEGIN IMMEDIATE`.
    pub fn try_acquire(
        &mut self,
        lease: &LeaseId,
        heartbeat_mono: u64,
        boot_id: &str,
        pid: i32,
        pid_started_at: Option<&str>,
    ) -> Result<AcquireOutcome, ArmadaError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;

        let existing: Option<LeaseRow> = tx
            .query_row(
                "SELECT workspace, kind, key, heartbeat_mono, boot_id, pid, pid_started_at
                 FROM leases WHERE kind = ?1 AND key = ?2",
                (lease.kind.to_string(), &lease.key),
                |row| {
                    Ok(LeaseRow {
                        workspace: row
                            .get::<_, Option<String>>(0)?
                            .map(WorkspaceId::from_stored),
                        kind: parse_kind(&row.get::<_, String>(1)?),
                        key: row.get(2)?,
                        heartbeat_mono: row.get::<_, i64>(3)? as u64,
                        boot_id: row.get(4)?,
                        pid: row.get::<_, i64>(5)? as i32,
                        pid_started_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(|e| map_sqlite(&self.path, e))?;

        if let Some(row) = existing {
            tx.rollback().map_err(|e| map_sqlite(&self.path, e))?;
            return Ok(AcquireOutcome::Held(row));
        }

        tx.execute(
            "INSERT INTO leases
             (workspace, kind, key, heartbeat_mono, boot_id, pid, pid_started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                lease.workspace.as_ref().map(|w| w.as_str().to_string()),
                lease.kind.to_string(),
                &lease.key,
                heartbeat_mono as i64,
                boot_id,
                pid as i64,
                pid_started_at,
            ),
        )
        .map_err(|e| map_sqlite(&self.path, e))?;
        tx.commit().map_err(|e| map_sqlite(&self.path, e))?;
        Ok(AcquireOutcome::Granted)
    }

    /// Take `count` CPU slots, **all of them or none**, in one transaction.
    ///
    /// **The identity of a slot is the store's to choose, and that is the whole
    /// point of this method.** `lease::acquisition_order` numbers a check's
    /// slots `0..cost`, which is the right answer for *ordering* and the wrong
    /// one for *naming*: two checks each asking for slot `0` deadlock the
    /// moment the second one blocks on the first — measured, and it hung
    /// `armada manifest check` on its first real run. Slots are a counted budget, so what
    /// a check needs is `cost` free ones and never a particular one.
    ///
    /// **All or nothing, because a partial claim deadlocks under contention.**
    /// Taking three of four and waiting for the fourth means two runs can hold
    /// half the machine each and neither can proceed — the classic counted-
    /// semaphore failure, and one no acquisition *order* can fix, because the
    /// resources within the class are interchangeable. One `BEGIN IMMEDIATE`
    /// makes the count and the insert a single decision, which is exactly the
    /// shape PLAN.md §4.3 requires of anything that reads and then writes.
    ///
    /// **A check costing more than the machine takes the whole machine.** The
    /// reducer already admits it alone (`Budget::admits`); refusing it here
    /// would leave it waiting forever for slots that do not exist, which is the
    /// hang that rule exists to prevent.
    ///
    /// A slot whose holder has gone cold is reclaimed inside the same
    /// transaction rather than reported: the claim loop reclaims by
    /// `(kind, key)` and a slot has no fixed key to name, so the only place the
    /// two can be made atomic is here.
    #[allow(clippy::too_many_arguments)]
    pub fn try_acquire_slots(
        &mut self,
        workspace: &WorkspaceId,
        count: u32,
        total: u32,
        heartbeat_mono: u64,
        boot_id: &str,
        pid: i32,
        pid_started_at: Option<&str>,
    ) -> Result<SlotOutcome, ArmadaError> {
        let want = count.min(total).max(1);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;

        let held: Vec<LeaseRow> = {
            let mut statement = tx
                .prepare(
                    "SELECT workspace, kind, key, heartbeat_mono, boot_id, pid, pid_started_at
                     FROM leases WHERE kind = ?1",
                )
                .map_err(|e| map_sqlite(&self.path, e))?;
            let rows = statement
                .query_map((LeaseKind::CpuSlot.to_string(),), |row| {
                    Ok(LeaseRow {
                        workspace: row
                            .get::<_, Option<String>>(0)?
                            .map(WorkspaceId::from_stored),
                        kind: parse_kind(&row.get::<_, String>(1)?),
                        key: row.get(2)?,
                        heartbeat_mono: row.get::<_, i64>(3)? as u64,
                        boot_id: row.get(4)?,
                        pid: row.get::<_, i64>(5)? as i32,
                        pid_started_at: row.get(6)?,
                    })
                })
                .map_err(|e| map_sqlite(&self.path, e))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| map_sqlite(&self.path, e))?
        };

        let mut live: Vec<&LeaseRow> = Vec::new();
        for row in &held {
            if armada_core::lease::is_cold(
                row.heartbeat_mono,
                heartbeat_mono,
                row.boot_id == boot_id,
            ) {
                tx.execute(
                    "DELETE FROM leases WHERE kind = ?1 AND key = ?2",
                    (LeaseKind::CpuSlot.to_string(), &row.key),
                )
                .map_err(|e| map_sqlite(&self.path, e))?;
            } else {
                live.push(row);
            }
        }

        let taken: std::collections::BTreeSet<&str> =
            live.iter().map(|row| row.key.as_str()).collect();
        let free: Vec<String> = (0..total)
            .map(|slot| slot.to_string())
            .filter(|slot| !taken.contains(slot.as_str()))
            .collect();

        if (free.len() as u32) < want {
            tx.rollback().map_err(|e| map_sqlite(&self.path, e))?;
            return Ok(SlotOutcome::Short {
                held: live.first().map(|row| (*row).clone()),
                free: free.len() as u32,
            });
        }

        let mut granted = Vec::new();
        for slot in free.into_iter().take(want as usize) {
            tx.execute(
                "INSERT INTO leases
                 (workspace, kind, key, heartbeat_mono, boot_id, pid, pid_started_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    workspace.as_str(),
                    LeaseKind::CpuSlot.to_string(),
                    &slot,
                    heartbeat_mono as i64,
                    boot_id,
                    pid as i64,
                    pid_started_at,
                ),
            )
            .map_err(|e| map_sqlite(&self.path, e))?;
            granted.push(LeaseId {
                workspace: Some(workspace.clone()),
                kind: LeaseKind::CpuSlot,
                key: slot,
            });
        }
        tx.commit().map_err(|e| map_sqlite(&self.path, e))?;
        Ok(SlotOutcome::Granted(granted))
    }

    /// Renew a held lease's heartbeat.
    ///
    /// Called from the shell's event loop, **never from a background timer**: a
    /// background timer keeps ticking while the scheduler is wedged, so the
    /// lease looks healthy forever and you need a TTL to catch it. A
    /// loop-driven heartbeat simply stops.
    pub fn renew(&mut self, lease: &LeaseId, heartbeat_mono: u64) -> Result<(), ArmadaError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;
        tx.execute(
            "UPDATE leases SET heartbeat_mono = ?3 WHERE kind = ?1 AND key = ?2",
            (lease.kind.to_string(), &lease.key, heartbeat_mono as i64),
        )
        .map_err(|e| map_sqlite(&self.path, e))?;
        tx.commit().map_err(|e| map_sqlite(&self.path, e))
    }

    /// Reclaim a dead lease, **only if it is still the row that was observed
    /// cold**.
    ///
    /// Releasing one's own row is unconditional; taking somebody else's cannot
    /// be. The observation and the delete are two statements with a gap between
    /// them, and in that gap the holder — which is alive, merely slow — renews.
    /// An unconditional delete then removes a warm lease and lets two runs
    /// proceed in one workspace with no error anywhere. Returns whether the row
    /// was still the cold one; a `false` means the holder came back and the
    /// caller's next attempt will see it held, which is the correct answer.
    pub fn reclaim_lease(
        &mut self,
        kind: LeaseKind,
        key: &str,
        observed: &LeaseRow,
    ) -> Result<bool, ArmadaError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;
        let deleted = tx
            .execute(
                "DELETE FROM leases
                 WHERE kind = ?1 AND key = ?2 AND heartbeat_mono = ?3 AND boot_id = ?4
                   AND pid = ?5",
                (
                    kind.to_string(),
                    key,
                    observed.heartbeat_mono as i64,
                    &observed.boot_id,
                    observed.pid as i64,
                ),
            )
            .map_err(|e| map_sqlite(&self.path, e))?;
        tx.commit().map_err(|e| map_sqlite(&self.path, e))?;
        Ok(deleted == 1)
    }

    /// Release a lease, or reclaim a dead one — the same statement, and that is
    /// deliberate: reclaiming *is* releasing somebody else's row, and having
    /// one path means the reclaim cannot drift from the release.
    pub fn release_lease(&mut self, kind: LeaseKind, key: &str) -> Result<(), ArmadaError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite(&self.path, e))?;
        tx.execute(
            "DELETE FROM leases WHERE kind = ?1 AND key = ?2",
            (kind.to_string(), key),
        )
        .map_err(|e| map_sqlite(&self.path, e))?;
        tx.commit().map_err(|e| map_sqlite(&self.path, e))
    }
}

/// Put the file into WAL, waiting out a sibling Armada doing the same thing.
///
/// **This is the one statement `busy_timeout` does not cover, and it is
/// measured rather than assumed.** Switching a database's journal mode takes a
/// brief exclusive lock, and SQLite acquires that one *without* consulting the
/// busy handler — so a `busy_timeout` of five seconds still loses instantly.
/// Two `armada manifest init`s started together on a machine with no `manifest.db` yet both
/// create the file in rollback mode and both try to convert it, and roughly one
/// run in ten the loser reported `aborted` — "another Armada is writing to
/// manifest.db; retry" — for a database nobody was writing to. That is the
/// concurrent-claim guarantee failing before a claim is even attempted.
///
/// The wait is Armada's own, bounded by the same budget as every other statement,
/// and `SQLITE_BUSY` is the only code it retries: a busy database becomes
/// available, and a corrupt or unopenable one never does.
fn set_wal(conn: &Connection, path: &Path) -> Result<(), ArmadaError> {
    let deadline = std::time::Instant::now() + Duration::from_millis(BUSY_TIMEOUT_MS);
    loop {
        let error = match conn.pragma_update(None, "journal_mode", "WAL") {
            Ok(()) => return Ok(()),
            Err(error) => error,
        };
        let busy = matches!(
            &error,
            rusqlite::Error::SqliteFailure(inner, _) if inner.extended_code == 5
        );
        if !busy || std::time::Instant::now() >= deadline {
            return Err(map_sqlite(path, error));
        }
        // Short enough that the ordinary case — a sibling finishing its own
        // conversion — costs one sleep, and the loop is bounded regardless.
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Describe a lease row's holder, for a `WAITING` report.
pub fn holder_of(row: &LeaseRow, now_mono: u64) -> Holder {
    Holder {
        workspace: row.workspace.clone(),
        pid: row.pid,
        held_ms: now_mono.saturating_sub(row.heartbeat_mono),
    }
}

fn parse_kind(text: &str) -> LeaseKind {
    match text {
        "machine" => LeaseKind::Machine,
        "cpu-slot" => LeaseKind::CpuSlot,
        "exclusive" => LeaseKind::Exclusive,
        // `run` and anything a newer Armada wrote. A lease Armada cannot name is
        // still a lease it must not steal, so an unknown kind is treated as the
        // most conservative one rather than dropped.
        _ => LeaseKind::Run,
    }
}

/// Map a SQLite failure into Armada's vocabulary, **branching on the extended
/// code and never on the message**.
///
/// Two failures both print `database is locked` and mean opposite things:
/// `SQLITE_BUSY` (5) waited the full `busy_timeout` and lost, so it is
/// retryable; `SQLITE_BUSY_SNAPSHOT` (517) failed in microseconds because the
/// reader's snapshot is stale, and no amount of waiting can rescue it — it is a
/// design error in the transaction.
///
/// Three more arrive through the identical error type and mean something else
/// entirely. `SQLITE_FULL` (13), `SQLITE_CANTOPEN` (14) and `SQLITE_CORRUPT`
/// (11) are `environment` — the machine is broken, not the config and not the
/// tool. **Retrying a claim against a full disk is an infinite loop**, and
/// measured, a full disk looks healthy from the lease's point of view: a claim
/// fails with `SQLITE_FULL` while a *smaller* subsequent write still succeeds,
/// so heartbeats keep renewing and nothing gets reclaimed.
/// How "this workspace holds no block" is stored.
///
/// **A sentinel rather than a nullable column, and the reason is the schema
/// rule**: changes are additive for the whole 0.x line — a new column, never a
/// dropped or retyped one — because an older binary reading a `NULL` out of an
/// `INTEGER NOT NULL` column would fail at the `row.get` rather than at the
/// version check. `0` is not a port any workspace can be given whatever the
/// machine's `port_base` is — the floor is configurable, and `0` is below every
/// value of it that a `u16` port can name.
///
/// **It is translated in exactly these two functions and nowhere else.** Every
/// caller above the store sees `Option<PortBlock>`, so nothing outside this file
/// compares a port to zero.
const NO_BLOCK: i64 = 0;

/// A stored pair as the rest of Armada sees it.
fn block_of(from: i64, to: i64) -> Option<PortBlock> {
    (from != NO_BLOCK).then_some(PortBlock {
        from: from as u16,
        to: to as u16,
    })
}

/// A block as the store holds it.
fn stored(block: Option<PortBlock>) -> (i64, i64) {
    match block {
        Some(block) => (block.from as i64, block.to as i64),
        None => (NO_BLOCK, NO_BLOCK),
    }
}

fn map_sqlite(path: &Path, error: rusqlite::Error) -> ArmadaError {
    let extended = match &error {
        rusqlite::Error::SqliteFailure(inner, _) => inner.extended_code,
        _ => 0,
    };

    let (class, next_action) = match extended {
        // SQLITE_BUSY: a genuine queue Armada lost. Retryable, unchanged.
        5 => (
            ErrClass::Aborted,
            Some("another Armada is writing to manifest.db; retry".to_string()),
        ),
        // SQLITE_BUSY_SNAPSHOT: never retryable, and always Armada's bug — it
        // means a transaction read and then wrote without BEGIN IMMEDIATE.
        517 => (ErrClass::ArmadaBug, None),
        11 | 13 | 14 => (
            ErrClass::Environment,
            Some(format!(
                "`armada manifest clean --orphaned --force-rebuild` rebuilds {} from labels alone",
                path.display()
            )),
        ),
        _ => (ErrClass::Environment, None),
    };

    ArmadaError {
        class,
        r#where: path.display().to_string(),
        message: format!("manifest.db: {error}"),
        next_action,
    }
}

fn environment(location: String, message: String) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: location,
        message,
        next_action: None,
    }
}

/// A version-4 UUID from the operating system's entropy.
///
/// `/dev/urandom` rather than a crate: Armada needs sixteen random bytes exactly
/// once in its life, at database creation.
///
/// **A read that fails is `environment` and never a fallback value.** The
/// namespace is what keeps two Armada installations sharing one Docker daemon
/// from reaping each other's resources, and a fixed stand-in makes two
/// installations that both hit the failure compare *equal* — which is the
/// cross-reap the label exists to prevent, arriving silently.
fn random_uuid() -> Result<String, ArmadaError> {
    use std::io::Read;
    let mut bytes = [0u8; 16];
    let mut file = std::fs::File::open("/dev/urandom")
        .map_err(|e| environment("/dev/urandom".to_string(), format!("cannot open it: {e}")))?;
    file.read_exact(&mut bytes)
        .map_err(|e| environment("/dev/urandom".to_string(), format!("cannot read it: {e}")))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::lease::is_cold;
    use rusqlite::ffi;

    fn open() -> (tempfile::TempDir, Db) {
        let home = tempfile::tempdir().unwrap();
        let db = Db::open(home.path()).unwrap();
        (home, db)
    }

    fn ws(id: &str) -> WorkspaceId {
        WorkspaceId::from_stored(id)
    }

    #[test]
    fn a_fresh_database_carries_the_schema_version_and_a_namespace() {
        let (_home, db) = open();
        let version: i64 = db
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, USER_VERSION);
        let namespace = db.namespace().unwrap();
        assert_eq!(namespace.len(), 36, "{namespace}");
        assert_eq!(&namespace[14..15], "4", "version 4 nibble");
    }

    #[test]
    fn a_namespace_can_be_peeked_and_carried_into_a_replacement() {
        let home = tempfile::tempdir().unwrap();
        let original = Db::open(home.path()).unwrap().namespace().unwrap();
        let peeked = Db::peek_namespace(&home.path().join("manifest.db"));
        assert_eq!(peeked.as_deref(), Some(original.as_str()));

        // The replacement adopts it, so every resource already stamped with it
        // stays reapable across the rebuild.
        let fresh = tempfile::tempdir().unwrap();
        let mut replacement = Db::open(fresh.path()).unwrap();
        assert_ne!(replacement.namespace().unwrap(), original);
        replacement.adopt_namespace(&original).unwrap();
        assert_eq!(replacement.namespace().unwrap(), original);
    }

    /// The recovery must not *need* it: a database that cannot be read at all
    /// answers `None` rather than failing the rebuild.
    #[test]
    fn peeking_an_unreadable_database_answers_none_rather_than_failing() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(Db::peek_namespace(&home.path().join("absent.db")), None);

        let junk = home.path().join("junk.db");
        std::fs::write(&junk, b"this is not a database").unwrap();
        assert_eq!(Db::peek_namespace(&junk), None);
    }

    #[test]
    fn the_namespace_survives_reopening_and_never_changes() {
        let home = tempfile::tempdir().unwrap();
        let first = Db::open(home.path()).unwrap().namespace().unwrap();
        let second = Db::open(home.path()).unwrap().namespace().unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn wal_is_actually_on_because_it_is_a_property_of_the_file() {
        let (_home, db) = open();
        let mode: String = db
            .conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
    }

    #[test]
    fn a_database_from_a_newer_armada_is_an_environment_failure_naming_the_version() {
        let home = tempfile::tempdir().unwrap();
        {
            let db = Db::open(home.path()).unwrap();
            db.conn
                .pragma_update(None, "user_version", USER_VERSION + 7)
                .unwrap();
        }
        let err = Db::open(home.path()).unwrap_err();
        assert_eq!(err.class, ErrClass::Environment);
        assert!(err.message.contains(&(USER_VERSION + 7).to_string()));
    }

    #[test]
    fn two_workspaces_get_non_overlapping_blocks() {
        let (_home, mut db) = open();
        let a = db
            .claim_block(
                &ws("aaaaaaaa"),
                Path::new("/a"),
                None,
                Some(10),
                armada_core::ports::PORT_BASE,
                "t",
            )
            .unwrap();
        let b = db
            .claim_block(
                &ws("bbbbbbbb"),
                Path::new("/b"),
                None,
                Some(10),
                armada_core::ports::PORT_BASE,
                "t",
            )
            .unwrap();
        let (ClaimOutcome::Claimed(a), ClaimOutcome::Claimed(b)) = (a, b) else {
            panic!("both claims should have succeeded");
        };
        let (a, b) = (a.expect("a block"), b.expect("a block"));
        assert!(!a.overlaps(&b), "{a:?} overlaps {b:?}");
    }

    #[test]
    fn claiming_twice_is_idempotent_by_workspace_id() {
        let (_home, mut db) = open();
        let first = db
            .claim_block(
                &ws("aaaaaaaa"),
                Path::new("/a"),
                None,
                Some(10),
                armada_core::ports::PORT_BASE,
                "t",
            )
            .unwrap();
        let second = db
            .claim_block(
                &ws("aaaaaaaa"),
                Path::new("/a"),
                None,
                Some(10),
                armada_core::ports::PORT_BASE,
                "t",
            )
            .unwrap();
        match (first, second) {
            (ClaimOutcome::Claimed(a), ClaimOutcome::AlreadyHeld(b)) => assert_eq!(a, b),
            other => panic!("expected claim then already-held, got {other:?}"),
        }
        assert_eq!(db.workspaces().unwrap().len(), 1);
    }

    #[test]
    fn a_released_block_is_reused_by_the_next_claimant() {
        let (_home, mut db) = open();
        let ClaimOutcome::Claimed(first) = db
            .claim_block(
                &ws("aaaaaaaa"),
                Path::new("/a"),
                None,
                Some(10),
                armada_core::ports::PORT_BASE,
                "t",
            )
            .unwrap()
        else {
            panic!()
        };
        db.release_workspace(&ws("aaaaaaaa"), &[]).unwrap();
        let ClaimOutcome::Claimed(second) = db
            .claim_block(
                &ws("bbbbbbbb"),
                Path::new("/b"),
                None,
                Some(10),
                armada_core::ports::PORT_BASE,
                "t",
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(first, second);
    }

    /// **The table list is read off the schema, not hand-typed here.** A fresh
    /// database has every table `migrate` creates, each at zero rows — proof
    /// that a table added there needs no matching edit to `peek_stats`.
    #[test]
    fn peek_stats_names_every_table_the_schema_has() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("manifest.db");
        Db::open(home.path()).unwrap();

        let stats = Db::peek_stats(&path).expect("a fresh database is readable");
        let names: Vec<&str> = stats.tables.iter().map(|(n, _)| n.as_str()).collect();
        for expected in ["meta", "workspaces", "owned", "leases"] {
            assert!(names.contains(&expected), "{names:?} is missing {expected}");
        }
        // `meta` carries the namespace written at creation; every other table
        // is empty on a fresh store.
        for (name, n) in &stats.tables {
            let expected = if name == "meta" { 1 } else { 0 };
            assert_eq!(*n, expected, "{name} has {n} rows: {:?}", stats.tables);
        }
        assert_eq!(stats.reclaimable_workspaces, 0);
        assert_eq!(stats.reclaimable_owned, 0);
    }

    /// **A workspace whose directory is gone is reclaimable, and so is
    /// everything it owns.** This is [`armada_core::reap::registry_pass`]'s own
    /// rule, exercised through `peek_stats` rather than restated: presence is
    /// not health, and this is what lets a reader tell "two workspaces, both
    /// live" from "two workspaces, and neither is".
    #[test]
    fn peek_stats_counts_what_a_missing_workspace_directory_makes_reclaimable() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("manifest.db");
        let mut db = Db::open(home.path()).unwrap();

        let gone = home.path().join("gone");
        db.claim_block(
            &ws("aaaaaaaa"),
            &gone,
            None,
            None,
            armada_core::ports::PORT_BASE,
            "t",
        )
        .unwrap();
        db.record_owned(&OwnedRow {
            workspace: ws("aaaaaaaa"),
            kind: OwnedKind::Container,
            reference: "c1".to_string(),
            boot_id: None,
            pid_started_at: None,
            component: None,
        })
        .unwrap();

        let live = home.path().join("live");
        std::fs::create_dir_all(&live).unwrap();
        db.claim_block(
            &ws("bbbbbbbb"),
            &live,
            None,
            None,
            armada_core::ports::PORT_BASE,
            "t",
        )
        .unwrap();

        let stats = Db::peek_stats(&path).unwrap();
        assert_eq!(stats.reclaimable_workspaces, 1, "{stats:?}");
        assert_eq!(stats.reclaimable_owned, 1, "{stats:?}");
        let workspaces = stats
            .tables
            .iter()
            .find(|(n, _)| n == "workspaces")
            .unwrap()
            .1;
        assert_eq!(workspaces, 2);
    }

    /// A database that cannot be opened at all — the recovery path
    /// [`Db::peek_namespace`] already tolerates — answers `None` rather than
    /// panicking; `store` in `doctor.rs` falls back to `present, could not be
    /// read` on this.
    #[test]
    fn peek_stats_on_an_unreadable_file_answers_none() {
        let home = tempfile::tempdir().unwrap();
        let junk = home.path().join("manifest.db");
        std::fs::write(&junk, b"this is not a database").unwrap();
        assert_eq!(Db::peek_stats(&junk), None);
    }

    #[test]
    fn owned_rows_round_trip_and_an_unknown_kind_is_skipped_rather_than_dropped() {
        let (_home, mut db) = open();
        db.record_owned(&OwnedRow {
            workspace: ws("aaaaaaaa"),
            kind: OwnedKind::Pgid,
            reference: "4212".to_string(),
            boot_id: Some("boot-1".to_string()),
            pid_started_at: Some("whenever".to_string()),
            component: Some("api".to_string()),
        })
        .unwrap();
        // As a newer Armada would have written it.
        db.conn
            .execute(
                "INSERT INTO owned (workspace, kind, \"ref\") VALUES ('aaaaaaaa', 'quantum', 'q1')",
                [],
            )
            .unwrap();

        let rows = db.owned(Some(&ws("aaaaaaaa"))).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, OwnedKind::Pgid);

        let still_there: i64 = db
            .conn
            .query_row("SELECT count(*) FROM owned", [], |r| r.get(0))
            .unwrap();
        assert_eq!(still_there, 2, "Armada never deletes what it cannot name");
    }

    #[test]
    fn a_release_command_is_recorded_as_an_owned_row() {
        let (_home, mut db) = open();
        db.record_owned(&OwnedRow {
            workspace: ws("aaaaaaaa"),
            kind: OwnedKind::Release,
            reference: "psql -c 'DROP DATABASE app_aaaaaaaa'".to_string(),
            boot_id: None,
            pid_started_at: None,
            component: None,
        })
        .unwrap();
        let rows = db.owned(Some(&ws("aaaaaaaa"))).unwrap();
        assert_eq!(rows[0].kind, OwnedKind::Release);
        db.clear_kind(&ws("aaaaaaaa"), OwnedKind::Release).unwrap();
        assert!(db.owned(Some(&ws("aaaaaaaa"))).unwrap().is_empty());
    }

    #[test]
    fn a_second_claimant_sees_the_holder_rather_than_taking_the_lease() {
        let (_home, mut db) = open();
        let lease = LeaseId::run(ws("aaaaaaaa"));
        assert_eq!(
            db.try_acquire(&lease, 1_000, "boot-1", 42, None).unwrap(),
            AcquireOutcome::Granted
        );
        match db.try_acquire(&lease, 2_000, "boot-1", 43, None).unwrap() {
            AcquireOutcome::Held(row) => assert_eq!(row.pid, 42),
            other => panic!("expected Held, got {other:?}"),
        }
    }

    #[test]
    fn a_released_lease_is_available_again() {
        let (_home, mut db) = open();
        let lease = LeaseId::run(ws("aaaaaaaa"));
        db.try_acquire(&lease, 1_000, "boot-1", 42, None).unwrap();
        db.release_lease(LeaseKind::Run, "aaaaaaaa").unwrap();
        assert_eq!(
            db.try_acquire(&lease, 2_000, "boot-1", 43, None).unwrap(),
            AcquireOutcome::Granted
        );
    }

    /// `clean` holds the run lease of the workspace it is dismantling, and the
    /// ordering depends on still holding it after the row is gone.
    #[test]
    fn releasing_a_workspace_keeps_the_lease_its_caller_is_holding() {
        let (_home, mut db) = open();
        let held = LeaseId::run(ws("aaaaaaaa"));
        let slot = LeaseId {
            workspace: Some(ws("aaaaaaaa")),
            kind: LeaseKind::CpuSlot,
            key: "0".to_string(),
        };
        db.try_acquire(&held, 1_000, "boot-1", 42, None).unwrap();
        db.try_acquire(&slot, 1_000, "boot-1", 42, None).unwrap();

        db.release_workspace(&ws("aaaaaaaa"), std::slice::from_ref(&held))
            .unwrap();

        let rows = db.leases().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, LeaseKind::Run);
        assert!(db.workspaces().unwrap().is_empty());
    }

    /// The observation and the delete are two statements, and a holder that was
    /// merely slow renews in the gap between them.
    #[test]
    fn a_reclaim_refuses_a_row_the_holder_renewed_in_the_meantime() {
        let (_home, mut db) = open();
        let lease = LeaseId::run(ws("aaaaaaaa"));
        db.try_acquire(&lease, 1_000, "boot-1", 42, None).unwrap();
        let observed = db.leases().unwrap().remove(0);

        db.renew(&lease, 90_000).unwrap();
        assert!(!db
            .reclaim_lease(LeaseKind::Run, "aaaaaaaa", &observed)
            .unwrap());
        assert_eq!(db.leases().unwrap().len(), 1);

        let observed = db.leases().unwrap().remove(0);
        assert!(db
            .reclaim_lease(LeaseKind::Run, "aaaaaaaa", &observed)
            .unwrap());
        assert!(db.leases().unwrap().is_empty());
    }

    #[test]
    fn a_renewal_moves_the_heartbeat_and_keeps_a_lease_warm() {
        let (_home, mut db) = open();
        let lease = LeaseId::run(ws("aaaaaaaa"));
        db.try_acquire(&lease, 1_000, "boot-1", 42, None).unwrap();
        db.renew(&lease, 90_000).unwrap();
        let row = &db.leases().unwrap()[0];
        assert_eq!(row.heartbeat_mono, 90_000);
        assert!(!is_cold(row.heartbeat_mono, 100_000, true));
    }

    /// The `st_nlink` measurement, reproduced: the holder can see that its
    /// file has been unlinked, and that is the only side of the divergence
    /// that can.
    #[test]
    fn a_holder_notices_when_its_database_is_deleted_under_it() {
        let home = tempfile::tempdir().unwrap();
        let db = Db::open(home.path()).unwrap();
        assert!(db.still_linked());
        std::fs::remove_file(home.path().join("manifest.db")).unwrap();
        assert!(!db.still_linked());
    }

    /// **The class comes from the extended code and never from the message.**
    /// `SQLITE_BUSY` and `SQLITE_BUSY_SNAPSHOT` both print `database is
    /// locked`, and one of them is retryable while the other is Armada's own bug
    /// — so a mapping that read the text would have the claim loop retry a
    /// transaction that can never succeed.
    #[test]
    fn a_sqlite_failure_is_classified_by_its_extended_code() {
        let path = Path::new("/scratch/manifest.db");
        let mapped = |code| {
            map_sqlite(
                path,
                rusqlite::Error::SqliteFailure(ffi::Error::new(code), None),
            )
        };

        // 5, SQLITE_BUSY: a queue Armada lost, and the remedy is to retry.
        let busy = mapped(5);
        assert_eq!(busy.class, ErrClass::Aborted);
        assert!(busy.next_action.unwrap().contains("retry"));

        // 517, SQLITE_BUSY_SNAPSHOT: never retryable, and always Armada's bug.
        let snapshot = mapped(517);
        assert_eq!(snapshot.class, ErrClass::ArmadaBug);
        assert_eq!(snapshot.next_action, None);

        // 11, 13, 14 — corrupt, full, cannot-open: the machine is broken, and
        // the remedy names the database rather than the config.
        for code in [11, 13, 14] {
            let broken = mapped(code);
            assert_eq!(broken.class, ErrClass::Environment, "code {code}");
            assert!(
                broken.next_action.unwrap().contains("/scratch/manifest.db"),
                "code {code}"
            );
        }

        // Anything else, including an error that is not a SQLite failure at
        // all: environment, with no remedy Armada can honestly suggest.
        let other = mapped(1);
        assert_eq!(other.class, ErrClass::Environment);
        assert_eq!(other.next_action, None);
        assert_eq!(
            map_sqlite(path, rusqlite::Error::QueryReturnedNoRows).class,
            ErrClass::Environment
        );
    }

    /// The newcomer's side of the same measurement, recorded because it bounds
    /// what the sentinel can do: to a second process the unlinked case is
    /// byte-identical to a fresh install, so there is no discriminating bit
    /// available to it.
    #[test]
    fn a_newcomer_cannot_tell_an_unlinked_database_from_a_fresh_one() {
        let home = tempfile::tempdir().unwrap();
        let holder = Db::open(home.path()).unwrap();
        std::fs::remove_file(home.path().join("manifest.db")).unwrap();

        let newcomer = Db::open(home.path()).unwrap();
        assert!(newcomer.still_linked());
        assert_ne!(
            newcomer.namespace().unwrap(),
            holder.namespace().unwrap(),
            "it is a different database, and nothing told it so"
        );
    }
}
