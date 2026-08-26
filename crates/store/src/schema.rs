//! The tables, and the only thing that changes them.
//!
//! # Why a version exists before anything needs one
//!
//! A schema with no version is a schema nobody can change later: the first
//! migration has to answer "what is already there?" and, with nothing recorded,
//! the honest answer is a guess. [`MIGRATIONS`] is therefore an ordered list,
//! and `armada_meta.schema_version` records how many of them a file has had
//! applied. Adding a migration is appending a `&str`.
//!
//! [`V2`] is the first one appended, and what it had to decide is on it: what a
//! Job written before the column existed is called.
//!
//! The table also serves a second purpose that costs nothing: **its presence is
//! what distinguishes an Armada store from some other database that happens to
//! be at the path.** A file with tables and no `armada_meta` is refused rather
//! than migrated into, which is the difference between opening the wrong file
//! loudly and writing Jobs into it.
//!
//! # `job_events` is append-only in the database, not in the code
//!
//! Two triggers refuse `UPDATE` and `DELETE` on it. A convention that events
//! are never edited is a convention some later query breaks; a trigger is the
//! same rule where breaking it is an error and not a diff nobody reviewed.
//!
//! # `STRICT`, and integer keys
//!
//! Every table is `STRICT`, so a column typed `TEXT` cannot quietly hold an
//! integer. `job_events.seq` is `INTEGER PRIMARY KEY AUTOINCREMENT`: the fold
//! orders by it and never by `at`, because timestamps are injected — two events
//! may legitimately carry the same instant, and a test may hand back an earlier
//! one. `AUTOINCREMENT` rather than a bare rowid so a key is never reused, which
//! matters for a log whose whole value is that entries do not move.

/// The key under which [`MIGRATIONS`]' applied count is recorded.
pub const SCHEMA_VERSION_KEY: &str = "schema_version";

/// How many migrations this build knows. A file recording more than this was
/// written by a newer Armada, and is refused rather than read with the older
/// crate's assumptions.
pub const KNOWN_SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

/// Applied in order. Index `n` takes a file from version `n` to `n + 1`.
///
/// **Nothing is ever edited here.** Changing entry zero changes what an already
/// migrated file is assumed to contain, which is the one thing the version
/// number exists to stop.
pub const MIGRATIONS: &[&str] = &[V1, V2];

/// Version 1 — the Job record, the rows beneath it, and the log.
const V1: &str = r#"
CREATE TABLE armada_meta (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
) STRICT;

-- The Job row. `status` is a cache of the fold over `job_events`; every other
-- column here is the authority for its field, because no event carries it.
CREATE TABLE jobs (
    job_id                TEXT PRIMARY KEY NOT NULL,
    status                TEXT NOT NULL,
    workflow_id           TEXT NOT NULL,
    owner_manifest_id     TEXT NOT NULL,
    origin                TEXT NOT NULL,
    urgency               TEXT NOT NULL,
    atomic                INTEGER NOT NULL,
    model                 TEXT NOT NULL,
    acceptance_criteria   TEXT NOT NULL,
    current_step_id       TEXT,
    assigned_drone        TEXT,
    dependencies          TEXT NOT NULL,
    dispatched_by_job_id  TEXT,
    dispatched_by_step_id TEXT,
    redispatched_from     TEXT,
    subject_kind          TEXT,
    subject_ref           TEXT,
    facts                 TEXT NOT NULL,
    scope_revisions       TEXT NOT NULL,
    -- Null is not empty. `job_write_targets` cannot say which of the two zero
    -- rows means, so the Job row says it.
    write_targets_known   INTEGER NOT NULL,
    -- Not a field of the record. The fold rebuilds a Job through the same
    -- constructor that made it, and that constructor stamps the `job_steps`
    -- rows; no event describes creation, so the instant is kept here.
    created_at            TEXT NOT NULL
) STRICT;

-- One row per step of the frozen WorkflowDef, written at Job creation. No
-- `retry_count` and no `iteration_count`: both arrive with retries, and a
-- counter that exists and never moves reads as a counter that is working.
CREATE TABLE job_steps (
    job_id       TEXT NOT NULL REFERENCES jobs(job_id),
    step_id      TEXT NOT NULL,
    ordinal      INTEGER NOT NULL,
    state        TEXT NOT NULL,
    last_verdict TEXT,
    entered_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id)
) STRICT;

-- One row per declared path, in the order the Job declared them.
CREATE TABLE job_write_targets (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    ordinal INTEGER NOT NULL,
    path    TEXT NOT NULL,
    PRIMARY KEY (job_id, ordinal)
) STRICT;

-- The gating Manifests and what their Checks did.
CREATE TABLE job_manifests (
    job_id         TEXT NOT NULL REFERENCES jobs(job_id),
    ordinal        INTEGER NOT NULL,
    manifest_id    TEXT NOT NULL,
    outcome        TEXT NOT NULL,
    not_run_reason TEXT,
    PRIMARY KEY (job_id, ordinal)
) STRICT;

-- The authority. One row per transition, never edited and never removed.
CREATE TABLE job_events (
    seq          INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id       TEXT NOT NULL REFERENCES jobs(job_id),
    status_from  TEXT NOT NULL,
    status_to    TEXT NOT NULL,
    reason_kind  TEXT NOT NULL,
    reason_value TEXT,
    actor        TEXT NOT NULL,
    at           TEXT NOT NULL
) STRICT;

CREATE INDEX job_events_by_job ON job_events (job_id, seq);

CREATE TRIGGER job_events_are_never_edited
BEFORE UPDATE ON job_events
BEGIN
    SELECT RAISE(ABORT, 'job_events is append-only: a recorded transition is never edited');
END;

CREATE TRIGGER job_events_are_never_removed
BEFORE DELETE ON job_events
BEGIN
    SELECT RAISE(ABORT, 'job_events is append-only: a recorded transition is never removed');
END;
"#;

/// Version 2 — a Job has a title.
///
/// # What happens to a Job written by version 1
///
/// **It is named after itself, visibly.** `Untitled job <job_id>` — computed
/// per row, so no two migrated Jobs come back carrying the same name and
/// nothing on a Board reads as though a person typed it. A single backfill
/// constant would have been shorter and is what this deliberately does not do:
/// a store where every Job is suddenly called "Untitled" has quietly lost a
/// distinction, and looks from the outside like one where somebody typed the
/// same title twelve times.
///
/// **Refusing to open was the other honest option, and is worse here.** There
/// are no production stores, so a refusal protects nothing; what it does is
/// take a file with real Jobs in it and make every one of them unreachable over
/// a column that was never written. `open.rs` refuses a file this build cannot
/// read — and a version 1 Job reads perfectly well, it just has nothing to be
/// called.
///
/// # Why the column carries a `DEFAULT` it must never use
///
/// SQLite will not add a `NOT NULL` column without one. The default is `''`,
/// which is exactly the value a title may not be, so the two triggers close the
/// hole the `ALTER` opens: an insert or an update that would leave a blank
/// title aborts. `insert_job` binds the column on every write and
/// [`Title`](core_model::Title) cannot hold a blank, so nothing in this crate
/// can trip them — which is the point, the same way `job_events`' append-only
/// triggers are.
const V2: &str = r#"
ALTER TABLE jobs ADD COLUMN title TEXT NOT NULL DEFAULT '';

-- Named after itself rather than after a constant. `job_id` is already on the
-- row, so the backfill invents nothing and reads no clock.
UPDATE jobs SET title = 'Untitled job ' || job_id;

CREATE TRIGGER jobs_are_never_written_without_a_title
BEFORE INSERT ON jobs
WHEN trim(NEW.title) = ''
BEGIN
    SELECT RAISE(ABORT, 'a Job has a title, and blank is not one');
END;

CREATE TRIGGER jobs_are_never_left_without_a_title
BEFORE UPDATE ON jobs
WHEN trim(NEW.title) = ''
BEGIN
    SELECT RAISE(ABORT, 'a Job has a title, and blank is not one');
END;
"#;
