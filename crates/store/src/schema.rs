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
//! Two triggers refuse `UPDATE` on it, and `DELETE` while the Job it belongs to
//! still exists — see [`V4`], which narrowed the second. A convention that
//! events are never edited is a convention some later query breaks; a trigger
//! is the same rule where breaking it is an error and not a diff nobody
//! reviewed.
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
pub const MIGRATIONS: &[&str] = &[V1, V2, V3, V4, V5, V6, V7, V8, V9, V10, V11, V12];

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

/// Version 3 — a step move is a row in the same log, not a log of its own.
///
/// # Why a kind on `job_events` and not a `job_step_events` table
///
/// **Because the two have to be replayed in one order.** `job-fields.toml` says
/// the inner machine "advances only while the Job is `running` or
/// `awaiting_review`", so replaying a step move means knowing which status the
/// Job stood in when it happened. Two tables means two `AUTOINCREMENT`
/// sequences, and two sequences cannot be interleaved: the fold would have to
/// take the step row's word for the status, which is a column nothing can
/// check. One log in one order means the status a step moved under is
/// `status_from` on its own row, and the fold checks it the same way it checks
/// every other row — against where it has got to.
///
/// A separate table was the tidier schema and would have cost the one property
/// this whole log exists for.
///
/// # What a step row puts in the status columns
///
/// The Job's status, in both. They are `NOT NULL` and a step move is honestly
/// described by them: the Job did not move. That is also what makes the row
/// checkable. `SQLite` cannot drop a `NOT NULL` without rebuilding the table,
/// and rebuilding an append-only log means copying it out and dropping the
/// original past its own triggers — so weakening them was never the cheaper
/// option either.
///
/// # There is no backfill, and the refusal being removed is why
///
/// V2 had to name every existing Job per row, because a constant would have
/// lost a distinction. V3 has no such row to fill: `read.rs` refused a
/// `job_steps` row that was not `not_started` and refused a non-null
/// `current_step_id`, so every step in every existing store is at the state
/// creation wrote and no move has been lost. The `DEFAULT` on `kind` is
/// therefore not a guess about old rows — it is the only thing an old row could
/// be, because nothing could write it anything else.
///
/// # The shape trigger
///
/// One table holding two shapes is one table that can hold a third by accident.
/// The trigger refuses an insert that is neither shape whole: a job transition
/// carrying step columns, a step move missing one, a step move claiming the Job
/// changed status, or a `kind` nobody declared. Same argument as the
/// append-only triggers — a rule the database holds is not a rule a later query
/// can quietly break.
const V3: &str = r#"
ALTER TABLE job_events ADD COLUMN kind TEXT NOT NULL DEFAULT 'job_transition';
ALTER TABLE job_events ADD COLUMN step_id TEXT;
ALTER TABLE job_events ADD COLUMN state_from TEXT;
ALTER TABLE job_events ADD COLUMN state_to TEXT;

CREATE TRIGGER job_events_hold_one_whole_shape
BEFORE INSERT ON job_events
WHEN NOT (
    (NEW.kind = 'job_transition'
        AND NEW.step_id IS NULL
        AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL
        AND NEW.status_from <> NEW.status_to)
 OR (NEW.kind = 'step_transition'
        AND NEW.step_id IS NOT NULL
        AND NEW.state_from IS NOT NULL
        AND NEW.state_to IS NOT NULL
        AND NEW.status_from = NEW.status_to
        AND NEW.reason_kind = 'unqualified'
        AND NEW.reason_value IS NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'a job_events row is one whole shape or the other: a job transition with no step columns, or a step move beneath an unchanged status');
END;
"#;

/// Version 4 — a Job can be forgotten, and only whole.
///
/// # Why the append-only trigger had to move rather than stand
///
/// `armada clean` deletes a repository's Jobs. Every route to that runs into
/// V1's trigger, which refuses any `DELETE` on `job_events` — so the store as
/// built could not forget a Job at all, only accumulate them.
///
/// The rule worth keeping is not "no row is ever deleted". It is **no
/// transition is ever removed from a Job's history**, which is what makes the
/// fold authoritative. So the trigger now fires only while the Job row is still
/// there: an event cannot be deleted out from under a live Job, and the only
/// deletion that succeeds is one that has already forgotten the Job itself.
/// Forgetting is therefore whole or nothing, and there is still no way to
/// spell "remove that one transition".
///
/// `store::forget` does it in one transaction with `defer_foreign_keys`, so the
/// window in which a Job row is gone and its events are not never reaches disk.
const V4: &str = r#"
DROP TRIGGER job_events_are_never_removed;

CREATE TRIGGER job_events_are_never_removed_from_a_job_that_exists
BEFORE DELETE ON job_events
WHEN EXISTS (SELECT 1 FROM jobs WHERE jobs.job_id = OLD.job_id)
BEGIN
    SELECT RAISE(ABORT, 'job_events is append-only: a transition is never removed from a Job that exists');
END;
"#;

/// Version 5 — the branch a Job's worktree is on.
///
/// **The backfill is bounded by the log, not applied to every row.** Only a Job
/// whose log holds a move into `running` ever had a worktree, and every build
/// that made one named the branch `armada/<job_id>` — so for those rows the
/// value is observed rather than assumed. A Job that never ran keeps null,
/// because writing a branch for a worktree that does not exist is the formula
/// this column replaces.
///
/// It also corrects V1's note on `created_at`: that column is a field of the
/// record now, and a whole-Job elapsed is read from it.
const V5: &str = r#"
ALTER TABLE jobs ADD COLUMN branch TEXT;

UPDATE jobs SET branch = 'armada/' || job_id
WHERE branch IS NULL
  AND EXISTS (
      SELECT 1 FROM job_events
      WHERE job_events.job_id = jobs.job_id
        AND job_events.kind = 'job_transition'
        AND job_events.status_to = 'running'
  );

CREATE TRIGGER jobs_are_never_given_a_blank_branch
BEFORE UPDATE ON jobs
WHEN NEW.branch IS NOT NULL AND trim(NEW.branch) = ''
BEGIN
    SELECT RAISE(ABORT, 'a branch is what a person checks out, and blank is not one');
END;
"#;

/// Version 6 — what each of a step's declared Checks did.
///
/// # Its own table, and not `job_events`
///
/// A Check running is not a transition. It is the evidence a transition was
/// derived from, the way `jobs.branch` is a fact about a worktree rather than a
/// move — and V3's shape trigger admits two shapes on purpose. A third would
/// weaken the one rule that makes the log foldable.
///
/// # A pass is a row
///
/// Writing only failures cannot tell a step whose checks all passed from a step
/// whose checks were never run. `outcome` is one of the five in
/// `domain/check-outcomes.toml` and `passed` is one of them.
///
/// # No backfill, and nothing to refuse
///
/// Nothing recorded a Check result before this table existed, so every step in
/// every existing store has none — which is exactly what zero rows says.
const V6: &str = r#"
-- One row per Check a step declared, in the order the step declares them.
-- `name` is the Manifest Check's name, or the built-in's kind where it names
-- none. `expected` and `produced` are the sentences the failure carried and are
-- both null on a pass, where the outcome is the whole sentence.
CREATE TABLE job_step_checks (
    job_id   TEXT NOT NULL REFERENCES jobs(job_id),
    step_id  TEXT NOT NULL,
    ordinal  INTEGER NOT NULL,
    name     TEXT NOT NULL,
    outcome  TEXT NOT NULL,
    expected TEXT,
    produced TEXT,
    ran_at   TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, ordinal)
) STRICT;
"#;

/// Version 7 — the workflow a Job froze, where its Checks printed, and the
/// Drone that worked it.
///
/// Four columns, one migration, because three concurrent ones would race.
///
/// | Column | What it holds |
/// |---|---|
/// | `jobs.workflow` | The whole WorkflowDef the Job froze at creation, as JSON. Fleet reads this and never the file, so editing `.armada/workflows/` changes the next Job rather than a running one |
/// | `job_step_checks.output_path` | Where that Check's stdout and stderr were written, relative to the repository root. The reference, never the bytes |
/// | `job_events.drone_id` | Which Drone the row is about. Set on the two drone rows and null on every other |
/// | `job_events.kind` (values) | Admits `drone_spawned` and `drone_exited`, which the V3 shape trigger refused |
///
/// # `workflow` is null on every existing row, and that is a refusal
///
/// V2 named a titleless Job after itself because a per-row value existed to
/// compute. There is none here: nothing in a pre-V7 store records which
/// definition a Job followed, and a step backfilled as declaring no Check is
/// the one wrong answer — it reads as an ungated step rather than as a gap.
/// V5's rule applies, which is to backfill only what is observed, and nothing
/// is. `read.rs` refuses such a row, and `load_all_jobs` carries it out beside
/// the Jobs that did load rather than shortening the list.
///
/// # The shape trigger is replaced rather than extended
///
/// `SQLite` cannot alter a trigger, so admitting a third and fourth kind means
/// dropping V3's and writing the whole condition again. The rule is unchanged
/// in kind: one row is one whole shape, and a drone row is a `drone_id` beneath
/// an unchanged status with no step columns and no reason.
const V7: &str = r#"
ALTER TABLE jobs ADD COLUMN workflow TEXT;
ALTER TABLE job_step_checks ADD COLUMN output_path TEXT;
ALTER TABLE job_events ADD COLUMN drone_id TEXT;

DROP TRIGGER job_events_hold_one_whole_shape;

CREATE TRIGGER job_events_hold_one_whole_shape
BEFORE INSERT ON job_events
WHEN NOT (
    (NEW.kind = 'job_transition'
        AND NEW.step_id IS NULL
        AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL
        AND NEW.drone_id IS NULL
        AND NEW.status_from <> NEW.status_to)
 OR (NEW.kind = 'step_transition'
        AND NEW.step_id IS NOT NULL
        AND NEW.state_from IS NOT NULL
        AND NEW.state_to IS NOT NULL
        AND NEW.drone_id IS NULL
        AND NEW.status_from = NEW.status_to
        AND NEW.reason_kind = 'unqualified'
        AND NEW.reason_value IS NULL)
 OR (NEW.kind IN ('drone_spawned', 'drone_exited')
        AND NEW.step_id IS NULL
        AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL
        AND NEW.drone_id IS NOT NULL
        AND NEW.status_from = NEW.status_to
        AND NEW.reason_kind = 'unqualified'
        AND NEW.reason_value IS NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'a job_events row is one whole shape: a job transition with no step or drone columns, a step move beneath an unchanged status, or a drone arriving or leaving beneath one');
END;
"#;

/// Version 8 — what the Judge said about a step.
///
/// A table of its own rather than more columns on `job_step_checks`: a Check is
/// a command and an exit code, a judgment is a criterion and a verdict, and
/// folding them would make "which tier refused this" a nullable column nobody
/// could rely on.
///
/// `verdict` is one of the two in `criterion_verdict_judge` — `met` and
/// `not_met`. There is no third, and no `source` column: every row here is the
/// Judge's, and a person attesting a criterion is a different act with a
/// different writer.
///
/// # Nothing to backfill
///
/// No Judge ran before this table existed, so every step in every existing
/// store has none — which is what zero rows says.
const V8: &str = r#"
-- One row per criterion a step's Judge answered, in the order asked. `expected`,
-- `produced` and `consequence` are the three named fields a refusal owes and are
-- all null on `met`, where nothing is being refused on.
CREATE TABLE job_step_judgments (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    ordinal     INTEGER NOT NULL,
    criterion   TEXT NOT NULL,
    verdict     TEXT NOT NULL,
    expected    TEXT,
    produced    TEXT,
    consequence TEXT,
    judged_at   TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, ordinal)
) STRICT;
"#;

/// Version 9 — files a person attached to the brief.
///
/// # Its own table, like `job_write_targets`
///
/// Written once, at Job creation, from paths a proposal staged before the Job
/// existed. There is no column this could be folded onto and no event that
/// describes it — an attachment is not a transition, it is a fact about what
/// the Job was created carrying, the same shape `job_write_targets` already
/// has for declared paths. `job_id` has no `ordinal`: order does not matter to
/// a Drone opening a file by name, where it does for a write target's list.
///
/// # Nothing to backfill
///
/// No Job carried an attachment before this table existed, so every Job in
/// every existing store has none — which is what zero rows says, the same
/// refusal V6 and V8 both make.
const V9: &str = r#"
CREATE TABLE job_attachments (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    filename    TEXT NOT NULL,
    mime_type   TEXT NOT NULL,
    byte_size   INTEGER NOT NULL,
    storage_ref TEXT NOT NULL,
    created_at  TEXT NOT NULL
) STRICT;
"#;

/// Version 10 — the evidence a step's gate accepted.
///
/// `record.rs` has always said evidence is `store`'s to hold and never a column
/// on the Job row. This is that table, built when the first reader appeared:
/// the gaming check's `baseline_ref` names an earlier step's evidence, and a
/// baseline held only in the daemon's memory would vanish on restart and take
/// the check quietly with it.
///
/// One row per step, replaced on resubmission — the current evidence is what a
/// later step is judged against, and a superseded submission is not a baseline.
///
/// # Nothing to backfill
///
/// No step's evidence was written down before this table existed, which is what
/// zero rows says — the same refusal V6, V8 and V9 make.
const V10: &str = r#"
CREATE TABLE job_step_evidence (
    job_id        TEXT NOT NULL REFERENCES jobs(job_id),
    step_id       TEXT NOT NULL,
    evidence_type TEXT NOT NULL,
    claimed       TEXT NOT NULL,
    shown_by      TEXT NOT NULL,
    not_claimed   TEXT NOT NULL,
    recorded_at   TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id)
) STRICT;
"#;

/// Version 11 — a step move may carry why the step stopped.
///
/// V3 pinned every step row to `reason_kind = 'unqualified'`, which held while
/// `advanced` was the only destination that said anything. `running -> stopped`
/// says why, and `job_steps.last_verdict` is a cache of the fold, so a stop
/// written without its trigger rebuilds as a stopped step nobody can ask why
/// about.
///
/// The reason is bound to `state_to = 'stopped'` both ways, which is
/// `StepTarget::arriving_at` said in SQL. The trigger is rewritten whole
/// because `SQLite` cannot alter one. **Nothing to backfill**: no step could
/// stop before this, so every existing row satisfies the unqualified arm.
const V11: &str = r#"
DROP TRIGGER job_events_hold_one_whole_shape;

CREATE TRIGGER job_events_hold_one_whole_shape
BEFORE INSERT ON job_events
WHEN NOT (
    (NEW.kind = 'job_transition'
        AND NEW.step_id IS NULL
        AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL
        AND NEW.drone_id IS NULL
        AND NEW.status_from <> NEW.status_to)
 OR (NEW.kind = 'step_transition'
        AND NEW.step_id IS NOT NULL
        AND NEW.state_from IS NOT NULL
        AND NEW.state_to IS NOT NULL
        AND NEW.drone_id IS NULL
        AND NEW.status_from = NEW.status_to
        AND ((NEW.state_to <> 'stopped'
                AND NEW.reason_kind = 'unqualified'
                AND NEW.reason_value IS NULL)
          OR (NEW.state_to = 'stopped'
                AND NEW.reason_kind = 'escalation'
                AND NEW.reason_value IS NOT NULL)))
 OR (NEW.kind IN ('drone_spawned', 'drone_exited')
        AND NEW.step_id IS NULL
        AND NEW.state_from IS NULL
        AND NEW.state_to IS NULL
        AND NEW.drone_id IS NOT NULL
        AND NEW.status_from = NEW.status_to
        AND NEW.reason_kind = 'unqualified'
        AND NEW.reason_value IS NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'a job_events row is one whole shape: a job transition with no step or drone columns, a step move beneath an unchanged status carrying a trigger only where it stops the step, or a drone arriving or leaving beneath one');
END;
"#;

/// Version 12 — which gaming patterns a step's evidence tripped.
///
/// `evidence_suspect` on its own says the evidence is not to be trusted and not
/// what about it, which is the whole content of the finding. Shaped like
/// `job_step_judgments`, and replaced whole per step for its reason: two passes
/// interleaved would read as one that found twice as much.
///
/// **Nothing to backfill** — no flag was written down before this table
/// existed, which is what zero rows says, the same refusal V6, V8, V9 and V10
/// make.
const V12: &str = r#"
-- One row per pattern flagged, in the order the check answered them. `cited`
-- names the file, line or assertion the flag is about and is the whole value of
-- the row; a flag that cited nothing would be unactionable.
CREATE TABLE job_step_gaming_flags (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    step_id    TEXT NOT NULL,
    ordinal    INTEGER NOT NULL,
    pattern    TEXT NOT NULL,
    cited      TEXT NOT NULL,
    flagged_at TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, ordinal)
) STRICT;
"#;
