//! The span of ports a Job's worktree holds, a checkout holds, or Fleet's own
//! listener holds.
//!
//! `docs/concepts/fleet.md`, *Ports*: "A claim names a Job, a checkout or
//! Fleet's own listener, so who holds which span is a query rather than an
//! inspection of directories." [`PortClaimant`] is that one-of-three, answered
//! by the type rather than by three nullable columns a caller could leave all
//! set or all empty — the shape a `CHECK` would otherwise have to refuse at
//! the boundary between this crate and whoever calls it.
//!
//! **Sizing, probing and picking a free span are not here.** This module
//! persists a claim once Fleet has decided one; the decision is
//! `fleet::ports`', which reads [`Store::every_port_claim`] to know what is
//! occupied before it probes a candidate.

use core_model::{JobId, Timestamp, Ulid};

use crate::error::{fault, LoadAllError, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::column;

/// Version 45 — the port span a Job's worktree holds, or the main checkout
/// holds. Beside the table it creates, like every migration since
/// [`V17`](crate::report::V17) — `schema.rs` is at the 900 lines the gate
/// refuses at.
///
/// **`job_id` and `main_checkout` are both nullable and the `CHECK` admits
/// exactly one.** [`PortClaimant`] makes the wrong shape unspeakable from
/// Rust; the `CHECK` holds the same rule from underneath, for a row this
/// crate did not write. Same argument as `job_events_hold_one_whole_shape`.
///
/// **One claim per Job and at most one for the main checkout**, each its own
/// partial unique index. `main_checkout` is `1` or absent rather than a plain
/// boolean: a `UNIQUE` index over a column most rows hold `NULL` in already
/// ignores those rows, so the one value the column is ever given is what the
/// index has to make singular.
///
/// **The foreign key on `job_id` is what makes this table one
/// [`tables_pointing_at_a_job`](crate::migrations::tables_pointing_at_a_job)
/// finds**, so `forget_job` takes a Job's claim with it if one was never
/// released — a safety net behind the explicit release
/// `docs/concepts/fleet.md` describes at teardown, not a substitute for it.
///
/// Nothing is backfilled: no claim was recorded before this table existed,
/// which is what zero rows says.
pub(crate) const V45: &str = r#"
CREATE TABLE port_claims (
    job_id        TEXT REFERENCES jobs(job_id),
    main_checkout INTEGER,
    base          INTEGER NOT NULL,
    width         INTEGER NOT NULL,
    claimed_at    TEXT NOT NULL,
    CHECK (base > 0 AND width > 0),
    CHECK ((job_id IS NULL) <> (main_checkout IS NULL)),
    CHECK (main_checkout IS NULL OR main_checkout = 1)
) STRICT;

CREATE UNIQUE INDEX port_claims_by_job ON port_claims (job_id) WHERE job_id IS NOT NULL;
CREATE UNIQUE INDEX port_claims_main_checkout ON port_claims (main_checkout) WHERE main_checkout IS NOT NULL;
"#;

/// Version 50 — Fleet's own listener joins a Job and the main checkout as a
/// third thing that can hold a span.
///
/// **A table rebuild, because SQLite cannot alter a `CHECK` in place.**
/// [`V45`]'s `CHECK ((job_id IS NULL) <> (main_checkout IS NULL))` spells
/// "exactly one of two" and there is no `ALTER TABLE` that widens it, so the
/// table is built beside the old one, the rows are carried across, and the old
/// one is dropped. **The drop comes before the indexes are created**: a rename
/// carries a table's indexes with it under their original names, so
/// `port_claims_by_job` exists on the renamed table until that table is gone.
///
/// **The rows are carried, not backfilled.** Every claim a Job or the main
/// checkout held before this migration is still held after it, at the same
/// base and width — which is what the migration test asserts, because a table
/// rebuild that quietly drops rows is a Job whose Command loses its port.
///
/// `fleet_listener` is `1` or absent, and its own partial unique index makes
/// it singular, exactly as `main_checkout` is: **one Fleet per store.** Two
/// Fleets on one machine are two stores under two homes, and what keeps their
/// ports apart is the bind-and-connect probe rather than a row either can see.
pub(crate) const V50: &str = r#"
ALTER TABLE port_claims RENAME TO port_claims_before_the_fleet_listener;

CREATE TABLE port_claims (
    job_id         TEXT REFERENCES jobs(job_id),
    main_checkout  INTEGER,
    fleet_listener INTEGER,
    base           INTEGER NOT NULL,
    width          INTEGER NOT NULL,
    claimed_at     TEXT NOT NULL,
    CHECK (base > 0 AND width > 0),
    CHECK ((job_id IS NOT NULL) + (main_checkout IS NOT NULL)
           + (fleet_listener IS NOT NULL) = 1),
    CHECK (main_checkout IS NULL OR main_checkout = 1),
    CHECK (fleet_listener IS NULL OR fleet_listener = 1)
) STRICT;

INSERT INTO port_claims (job_id, main_checkout, base, width, claimed_at)
SELECT job_id, main_checkout, base, width, claimed_at
FROM port_claims_before_the_fleet_listener;

DROP TABLE port_claims_before_the_fleet_listener;

CREATE UNIQUE INDEX port_claims_by_job ON port_claims (job_id) WHERE job_id IS NOT NULL;
CREATE UNIQUE INDEX port_claims_main_checkout ON port_claims (main_checkout) WHERE main_checkout IS NOT NULL;
CREATE UNIQUE INDEX port_claims_fleet_listener ON port_claims (fleet_listener) WHERE fleet_listener IS NOT NULL;
"#;

/// Version 83 — `main_checkout` becomes `checkout_path`, because a claim is
/// now held by any checkout of a repository and not only the one at its root.
///
/// **A rename and not a rebuild.** [`V50`] and [`V57`] rebuilt the table
/// because a `CHECK` cannot be altered in place; nothing about the rule
/// changes here, and SQLite carries a `RENAME COLUMN` into the `CHECK`
/// expressions and the index definitions naming it. **Every row keeps its base
/// and width** — a main-checkout claim is a checkout claim whose path is the
/// repository root, so there is nothing to convert.
///
/// The index is dropped and remade because SQLite renames the column inside an
/// index and never the index itself.
///
/// [`V57`]: crate::repositories::V57
pub(crate) const V83: &str = r#"
ALTER TABLE port_claims RENAME COLUMN main_checkout TO checkout_path;

DROP INDEX port_claims_main_checkout;
CREATE UNIQUE INDEX port_claims_checkout_path ON port_claims (checkout_path) WHERE checkout_path IS NOT NULL;
"#;

/// Who a port span belongs to. **Exactly one of the three, and there is no
/// fourth constructor that leaves it unstated.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortClaimant {
    /// A Job's worktree, for the whole of the worktree's lifetime.
    Job(JobId),
    /// A checkout, **by its path**, held for as long as Fleet runs. A
    /// repository's main checkout is the one at its root — what the proof run
    /// after a merge and a server started with no Job draw their ports from —
    /// and a worktree somebody cut by hand is another, so two checkouts of one
    /// repository never resolve `${port.NAME}` to the same number. `#1577`.
    ///
    /// Whether a path *is* a checkout of a served repository is
    /// `fleet::checkouts`', because this crate cannot look.
    ///
    /// **One key here is not a path**: a Verify's workspace span rides under
    /// `workspace:<path>`, which collides with no checkout because every
    /// checkout's key is absolute. **One such key is a convention and two
    /// would be a design** — a second is where this becomes a variant of its
    /// own, and that is another table rebuild.
    Checkout(String),
    /// Fleet's own listener — the one port Bridge connects to, published in
    /// the runtime file. Claimed at startup before the bind and released when
    /// Fleet stops, so two Fleets on one machine never bind the same number.
    ///
    /// **One port, not a span**, and it is the only claimant whose width is
    /// not sized from a Manifest's `ports:`.
    FleetListener,
}

/// A span of contiguous ports, claimed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortClaim {
    pub claimant: PortClaimant,
    /// The lowest port in the span.
    pub base: u16,
    /// How many ports wide the span is.
    pub width: u16,
    pub claimed_at: Timestamp,
}

impl Store {
    /// Record a span as claimed.
    ///
    /// **An insert, never an upsert.** Unlike
    /// [`record_drone_process`](Store::record_drone_process), a second claim
    /// for the same Job is not a respawn replacing the first — Fleet claims
    /// once, at worktree cut, and a second call here is a caller asking for
    /// what it should have read back with [`port_span_for_job`], so it fails
    /// rather than silently moving the Job's ports out from under a Command
    /// already holding the first span's numbers. The main checkout's claim is
    /// the same: claimed once, on first need, and reused after that.
    ///
    /// [`port_span_for_job`]: Store::port_span_for_job
    pub fn claim_port_span(&mut self, claim: &PortClaim) -> Result<(), WriteError> {
        let (job_id, checkout_path, fleet_listener) = match &claim.claimant {
            PortClaimant::Job(job_id) => (Some(job_id.as_str()), None, None),
            PortClaimant::Checkout(path) => (None, Some(path.as_str()), None),
            PortClaimant::FleetListener => (None, None, Some(1_i64)),
        };
        self.conn
            .execute(
                "INSERT INTO port_claims \
                 (job_id, checkout_path, fleet_listener, base, width, claimed_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (
                    job_id,
                    checkout_path,
                    fleet_listener,
                    i64::from(claim.base),
                    i64::from(claim.width),
                    claim.claimed_at.as_str(),
                ),
            )
            .map_err(|why| match (&claim.claimant, &why) {
                (PortClaimant::Job(job_id), rusqlite::Error::SqliteFailure(err, _))
                    if err.extended_code == FOREIGN_KEY_VIOLATION =>
                {
                    WriteError::NoSuchJob {
                        job_id: job_id.clone(),
                    }
                }
                _ => WriteError::Database(fault("claiming a port span")(why)),
            })?;
        Ok(())
    }

    /// The span released. **A claimant with no row is `Ok`**, for
    /// [`forget_drone_process`](Store::forget_drone_process)'s reason: every
    /// road that reaches this is a road where the span is already gone, or
    /// never existed because the repository declared no `ports:` at all.
    pub fn release_port_span(&mut self, claimant: &PortClaimant) -> Result<(), WriteError> {
        let deleted = match claimant {
            PortClaimant::Job(job_id) => self.conn.execute(
                "DELETE FROM port_claims WHERE job_id = ?1",
                (job_id.as_str(),),
            ),
            PortClaimant::Checkout(path) => self.conn.execute(
                "DELETE FROM port_claims WHERE checkout_path = ?1",
                (path.as_str(),),
            ),
            PortClaimant::FleetListener => self.conn.execute(
                "DELETE FROM port_claims WHERE fleet_listener IS NOT NULL",
                (),
            ),
        };
        deleted
            .map_err(fault("releasing a port span"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// The span claimed for one Job, if any was.
    pub fn port_span_for_job(&self, job_id: &JobId) -> Result<Option<PortClaim>, LoadJobError> {
        let found = self
            .conn
            .query_row(
                "SELECT job_id, checkout_path, fleet_listener, base, width, claimed_at \
                 FROM port_claims WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| Ok(read_claim(row)),
            )
            .map(Some)
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(LoadJobError::Unreadable(RowError::Database(fault(
                    "reading a Job's port span",
                )(
                    other
                )))),
            })?;
        found.transpose().map_err(LoadJobError::Unreadable)
    }

    /// The span claimed for the checkout at `path`, if one has been. A
    /// repository's main checkout is the one at its root.
    ///
    /// **Not a [`LoadJobError`]** — there is no Job to name if the read fails,
    /// so a database fault here is [`LoadAllError::Database`], the same
    /// reading [`every_port_claim`](Store::every_port_claim) gives one.
    pub fn port_span_for_checkout(&self, path: &str) -> Result<Option<PortClaim>, LoadAllError> {
        let found = self
            .conn
            .query_row(
                "SELECT job_id, checkout_path, fleet_listener, base, width, claimed_at \
                 FROM port_claims WHERE checkout_path = ?1",
                (path,),
                |row| Ok(read_claim(row)),
            )
            .map(Some)
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(LoadAllError::Database(fault(
                    "reading a checkout's port span",
                )(other))),
            })?;
        found.transpose().map_err(|cause| {
            LoadAllError::Database(fault("reading a checkout's port span")(
                rusqlite::Error::InvalidColumnName(cause.to_string()),
            ))
        })
    }

    /// The port Fleet's own listener claimed, if one has been claimed.
    ///
    /// **Read at startup, before the bind.** A row here that no live Fleet is
    /// behind is what a crash leaves, and it is reused rather than trusted:
    /// `fleet::listener` probes the port before this process binds it, because
    /// the row says only that the last Fleet to hold it believed it was free.
    ///
    /// **Not a [`LoadJobError`]**, for
    /// [`port_span_for_checkout`](Store::port_span_for_checkout)'s reason —
    /// there is no Job to name if the read fails.
    pub fn port_span_for_fleet_listener(&self) -> Result<Option<PortClaim>, LoadAllError> {
        let found = self
            .conn
            .query_row(
                "SELECT job_id, checkout_path, fleet_listener, base, width, claimed_at \
                 FROM port_claims WHERE fleet_listener IS NOT NULL",
                (),
                |row| Ok(read_claim(row)),
            )
            .map(Some)
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(LoadAllError::Database(fault(
                    "reading Fleet's own listener port claim",
                )(other))),
            })?;
        found.transpose().map_err(|cause| {
            LoadAllError::Database(fault("reading Fleet's own listener port claim")(
                rusqlite::Error::InvalidColumnName(cause.to_string()),
            ))
        })
    }

    /// Every span currently claimed, Job spans before the two kinds that have
    /// no Job — a checkout's and Fleet's own listener's — and each in claim
    /// order.
    ///
    /// **What Fleet reads before it picks a candidate span.** One pass rather
    /// than a query per attempt, the same shape as
    /// [`drone_processes`](Store::drone_processes) and for the reconciliation
    /// read's reason: the whole point of asking is to see every occupied
    /// range at once.
    pub fn every_port_claim(&self) -> Result<Vec<PortClaim>, LoadAllError> {
        let mut asked = self
            .conn
            .prepare(
                "SELECT job_id, checkout_path, fleet_listener, base, width, claimed_at \
                 FROM port_claims \
                 ORDER BY job_id IS NULL, job_id, checkout_path, fleet_listener, claimed_at",
            )
            .map_err(fault("preparing the port claim read"))
            .map_err(LoadAllError::Database)?;
        let rows = asked
            .query_map([], |row| Ok(read_claim(row)))
            .map_err(fault("reading every port claim"))
            .map_err(LoadAllError::Database)?;
        let mut held = Vec::new();
        for row in rows {
            let row = row
                .map_err(fault("reading a port claim row"))
                .map_err(LoadAllError::Database)?;
            held.push(row.map_err(|why| {
                LoadAllError::Database(fault("reading a port claim row")(
                    rusqlite::Error::InvalidColumnName(why.to_string()),
                ))
            })?);
        }
        Ok(held)
    }
}

/// One row, narrowed. **The claimant is refused rather than guessed** — a row
/// holding more or fewer than one of `job_id`, `checkout_path` and
/// `fleet_listener` is a row the `CHECK` should have stopped, and reading it
/// as one of them would be this crate deciding the shape the database already
/// refused to hold.
fn read_claim(row: &rusqlite::Row<'_>) -> Result<PortClaim, RowError> {
    let job_id: Option<String> = row.get("job_id").map_err(column(TABLE, "job_id"))?;
    let checkout_path: Option<String> = row
        .get("checkout_path")
        .map_err(column(TABLE, "checkout_path"))?;
    let fleet_listener: Option<i64> = row
        .get("fleet_listener")
        .map_err(column(TABLE, "fleet_listener"))?;
    let claimant = match (job_id, checkout_path, fleet_listener) {
        (Some(job_id), None, None) => PortClaimant::Job(JobId::carried(Ulid::carried(job_id))),
        (None, Some(path), None) => PortClaimant::Checkout(path),
        (None, None, Some(_)) => PortClaimant::FleetListener,
        _ => {
            return Err(RowError::MalformedColumn {
                table: TABLE,
                column: "job_id",
                detail: "a port claim names exactly one of a Job, a checkout or Fleet's own \
                         listener, and this row does not"
                    .to_string(),
            })
        }
    };
    let base: i64 = row.get("base").map_err(column(TABLE, "base"))?;
    let width: i64 = row.get("width").map_err(column(TABLE, "width"))?;
    let claimed_at: String = row.get("claimed_at").map_err(column(TABLE, "claimed_at"))?;
    Ok(PortClaim {
        claimant,
        base: u16::try_from(base).map_err(|_| RowError::MalformedColumn {
            table: TABLE,
            column: "base",
            detail: "a port is in range 1..=65535, and this is not".to_string(),
        })?,
        width: u16::try_from(width).map_err(|_| RowError::MalformedColumn {
            table: TABLE,
            column: "width",
            detail: "a span's width is a small positive count, and this is not".to_string(),
        })?,
        claimed_at: Timestamp::from_rfc3339(claimed_at),
    })
}

/// Named once, because every error above points at the same table.
const TABLE: &str = "port_claims";

/// `SQLITE_CONSTRAINT_FOREIGNKEY`, for [`claim_port_span`](Store::claim_port_span)'s
/// reason — see `job_drone_process`'s own copy in `process.rs` for why it is
/// spelled out rather than read off `rusqlite`'s enum.
const FOREIGN_KEY_VIOLATION: i32 = 787;
