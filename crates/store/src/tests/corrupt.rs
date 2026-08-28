//! An empty store and a damaged one are different events, and this is where
//! that is proved.
//!
//! Every case below asserts the same shape: **the open is refused, by name.**
//! None of them asserts a fallback, because there is none. Starting empty over
//! a database that exists is how a person loses work and is told nothing.
//!
//! Row-level damage is here too, and it behaves differently on purpose: one
//! Job that will not read back is not a reason to refuse every other Job, so
//! those surface at load rather than at open.

use core_model::{Actor, JobStatus, StepId, StepState, Target};
use rusqlite::Connection;

use crate::schema::SCHEMA_VERSION_KEY;
use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::{OpenError, RowError, Store, KNOWN_SCHEMA_VERSION};

// ------------------------------------------------------------ the empty case

#[test]
fn nothing_at_the_path_is_a_first_run() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store
        .load_all_jobs()
        .expect("an empty store")
        .jobs
        .is_empty());
    assert_eq!(version(&dir), KNOWN_SCHEMA_VERSION.to_string());
}

#[test]
fn opening_twice_migrates_once() {
    let dir = TempDir::new();
    drop(open(&dir));
    drop(open(&dir));
    assert_eq!(version(&dir), KNOWN_SCHEMA_VERSION.to_string());
}

#[test]
fn the_store_is_in_wal_mode() {
    let dir = TempDir::new();
    let store = open(&dir);
    let mode: String = store
        .conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("a journal mode");
    assert_eq!(mode.to_lowercase(), "wal");
}

// ------------------------------------------------------- the file-level cases

#[test]
fn a_database_that_is_not_an_armada_store_is_refused() {
    let dir = TempDir::new();
    let conn = Connection::open(dir.db()).expect("some other database");
    conn.execute_batch("CREATE TABLE somebody_elses (id INTEGER PRIMARY KEY);")
        .expect("written");
    drop(conn);

    match Store::open(&dir.db()) {
        Err(OpenError::NotAnArmadaStore { tables, .. }) => assert_eq!(tables, 1),
        other => panic!("expected a refusal, found {other:?}"),
    }
}

#[test]
fn a_schema_written_by_a_newer_armada_is_refused() {
    let dir = TempDir::new();
    drop(open(&dir));
    set_version(&dir, &(KNOWN_SCHEMA_VERSION + 7).to_string());

    match Store::open(&dir.db()) {
        Err(OpenError::SchemaVersionFromTheFuture { found, known, .. }) => {
            assert_eq!(found, KNOWN_SCHEMA_VERSION + 7);
            assert_eq!(known, KNOWN_SCHEMA_VERSION);
        }
        other => panic!("expected a refusal, found {other:?}"),
    }
}

#[test]
fn a_version_that_is_not_a_number_is_refused() {
    let dir = TempDir::new();
    drop(open(&dir));
    set_version(&dir, "whenever");

    match Store::open(&dir.db()) {
        Err(OpenError::SchemaVersionUnreadable { found, .. }) => {
            assert_eq!(found.as_deref(), Some("whenever"))
        }
        other => panic!("expected a refusal, found {other:?}"),
    }
}

#[test]
fn a_marker_table_with_no_version_is_refused_rather_than_assumed_fresh() {
    let dir = TempDir::new();
    drop(open(&dir));
    let conn = Connection::open(dir.db()).expect("open");
    conn.execute("DELETE FROM armada_meta", [])
        .expect("emptied");
    drop(conn);

    match Store::open(&dir.db()) {
        Err(OpenError::SchemaVersionUnreadable { found: None, .. }) => {}
        other => panic!("expected a refusal, found {other:?}"),
    }
}

/// An event whose Job is gone. Foreign keys are on, so this cannot be reached
/// through this crate — it is reached the way it would happen in the world, by
/// something else having edited the file.
#[test]
fn a_log_entry_pointing_at_a_job_that_is_gone_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01ORPHANED");
    store.insert_job(&job, &created_at()).expect("stored");
    let moved = job
        .transition(Target::Queued, Actor::Human, created_at())
        .expect("a legal move");
    store.record_transition(&moved).expect("recorded");
    drop(store);

    let conn = Connection::open(dir.db()).expect("open");
    conn.pragma_update(None, "foreign_keys", "OFF")
        .expect("off");
    conn.execute_batch(
        "DELETE FROM job_steps; DELETE FROM job_write_targets;
         DELETE FROM job_manifests; DELETE FROM job_attachments; DELETE FROM jobs;",
    )
    .expect("the job row removed out from under its log");
    drop(conn);

    match Store::open(&dir.db()) {
        Err(OpenError::DanglingReferences { rows, .. }) => assert_eq!(rows, 1),
        other => panic!("expected a refusal, found {other:?}"),
    }
}

/// Scribbling over a page. Which check catches it depends on where the damage
/// lands, so the assertion is the one that matters: **it does not open, and it
/// does not open empty.**
#[test]
fn a_damaged_file_does_not_open_empty() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for id in ["01A", "01B", "01C"] {
        store.insert_job(&top_level(id), &created_at()).expect("ok");
    }
    store
        .conn
        .pragma_update(None, "wal_checkpoint", "TRUNCATE")
        .expect("flushed to the main file");
    drop(store);

    let mut bytes = std::fs::read(dir.db()).expect("the file");
    let page = 4096;
    assert!(
        bytes.len() > page * 2,
        "there is more than one page to damage"
    );
    for byte in bytes.iter_mut().skip(page).take(page) {
        *byte = 0x5A;
    }
    std::fs::write(dir.db(), &bytes).expect("scribbled on");

    // `integrity_check` is what catches this one in practice; a scribble
    // elsewhere in the file would come back as a SQLite fault instead. Either
    // way it is refused, which is the property, so the assertion is that and
    // not the variant.
    assert!(
        Store::open(&dir.db()).is_err(),
        "a damaged file is refused, not opened as though it were new"
    );
}

// -------------------------------------------------------- the row-level cases

#[test]
fn an_event_that_does_not_leave_where_the_log_had_reached_is_named() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01GAP");
    // `running -> awaiting_review` is legal, but the log has only reached
    // `awaiting_approval`. An event is missing, or two are out of order.
    append_event(
        &store,
        "01GAP",
        "running",
        "awaiting_review",
        "unqualified",
        None,
    );

    match store.load_job(&job_id("01GAP")) {
        Err(crate::LoadJobError::Unreadable(RowError::EventDiscontinuity {
            folded,
            recorded,
            ..
        })) => {
            assert_eq!(folded, JobStatus::AwaitingApproval);
            assert_eq!(recorded, JobStatus::Running);
        }
        other => panic!("expected a discontinuity, found {other:?}"),
    }
}

#[test]
fn a_history_the_machine_would_not_admit_is_named() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01ILLEGAL");
    // There is no `awaiting_approval -> awaiting_review` edge.
    append_event(
        &store,
        "01ILLEGAL",
        "awaiting_approval",
        "awaiting_review",
        "unqualified",
        None,
    );

    match store.load_job(&job_id("01ILLEGAL")) {
        Err(crate::LoadJobError::Unreadable(RowError::IllegalRecordedTransition { .. })) => {}
        other => panic!("expected an illegal transition, found {other:?}"),
    }
}

#[test]
fn an_escalation_with_no_trigger_is_named() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01NOTRIGGER");
    append_event(
        &store,
        "01NOTRIGGER",
        "awaiting_approval",
        "queued",
        "derived_at_read",
        None,
    );
    append_event(
        &store,
        "01NOTRIGGER",
        "queued",
        "running",
        "unqualified",
        None,
    );
    append_event(
        &store,
        "01NOTRIGGER",
        "running",
        "escalated",
        "unqualified",
        None,
    );

    match store.load_job(&job_id("01NOTRIGGER")) {
        Err(crate::LoadJobError::Unreadable(RowError::ReasonDoesNotFitStatus {
            status,
            reason_kind,
            ..
        })) => {
            assert_eq!(status, JobStatus::Escalated);
            assert_eq!(reason_kind, "unqualified");
        }
        other => panic!("expected a reason mismatch, found {other:?}"),
    }
}

/// `job_steps.state` is a cache of the fold, in the sense `jobs.status` is.
///
/// **This was a refusal until a step move became a row in the log**, and had to
/// be: the fold could not put a moved step back, so returning the row as
/// `not_started` would have lost a move. The log can now say where a step got
/// to, so a scribbled column is simply not the authority any more.
#[test]
fn a_scribbled_step_state_loses_to_the_log() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01SCRIBBLED");
    store
        .conn
        .execute(
            "UPDATE job_steps SET state = 'advanced' WHERE step_id = 'fix'",
            [],
        )
        .expect("scribbled on");

    let job = store.load_job(&job_id("01SCRIBBLED")).expect("it folds");
    assert_eq!(
        job.step(&StepId::new("fix")).expect("the row").state(),
        StepState::NotStarted,
        "no event moved it, so it never moved"
    );
}

/// A value in the cached column that is not one of the six is still refused,
/// even though the fold does not need it. A row spelling a state this build
/// does not have was written by something that did not share the enum.
#[test]
fn a_step_state_column_this_build_cannot_spell_is_refused() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01MISSPELT");
    store
        .conn
        .execute("UPDATE job_steps SET state = 'advancing'", [])
        .expect("scribbled on");

    match store.load_job(&job_id("01MISSPELT")) {
        Err(crate::LoadJobError::Unreadable(RowError::UnknownEnumValue {
            table, column, ..
        })) => {
            assert_eq!((table, column), ("job_steps", "state"));
        }
        other => panic!("expected a refusal, found {other:?}"),
    }
}

/// The one state M1 cannot reach has no `StepTarget`, so a logged move into it
/// is a machine this build does not have. Refused, never folded as something
/// else.
///
/// `retrying` was the second until it had a budget to be inside. It now has a
/// target and two edges, and the row below is what a hand-back writes — which
/// is why this one is `awaiting_human`, still unreachable because a step at a
/// human gate stays `running`.
#[test]
fn a_logged_step_state_nothing_reaches_is_named_rather_than_folded() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01ATTHEGATE");
    store
        .conn
        .execute(
            "INSERT INTO job_events (
                 kind, job_id, status_from, status_to, reason_kind, reason_value,
                 step_id, state_from, state_to, actor, at
             ) VALUES ('step_transition', '01ATTHEGATE', 'awaiting_approval',
                 'awaiting_approval', 'unqualified', NULL, 'fix', 'not_started',
                 'awaiting_human', 'fleet', '2026-08-26T10:00:00.000Z')",
            [],
        )
        .expect("a machine that does not exist, writing");

    match store.load_job(&job_id("01ATTHEGATE")) {
        Err(crate::LoadJobError::Unreadable(RowError::StepStateNotReachable {
            step_id,
            state,
            ..
        })) => {
            assert_eq!(step_id.as_str(), "fix");
            assert_eq!(state, StepState::AwaitingHuman);
        }
        other => panic!("expected a refusal, found {other:?}"),
    }
}

/// The other side of the same rule. A hand-back carries the failure it is
/// answering, and the shape trigger refuses one written without it — so a
/// `retrying` row cannot reach the log unable to say what it is for.
#[test]
fn a_hand_back_written_without_the_failure_it_answers_is_refused_at_the_insert() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01RETRYING");
    let written = store.conn.execute(
        "INSERT INTO job_events (
             kind, job_id, status_from, status_to, reason_kind, reason_value,
             step_id, state_from, state_to, actor, at
         ) VALUES ('step_transition', '01RETRYING', 'running', 'running',
             'unqualified', NULL, 'fix', 'running', 'retrying', 'fleet',
             '2026-08-26T10:00:00.000Z')",
        [],
    );
    assert!(
        written.is_err(),
        "an unqualified hand-back would fold as a step being reattempted with \
         nothing saying what for"
    );
}

/// `last_verdict` admits step-level triggers only, and the shape trigger cannot
/// check a level. So the narrowing is paid on the way in, and a stop logged
/// with a Job-level trigger is refused rather than folded.
#[test]
fn a_stop_logged_with_a_job_level_trigger_is_refused() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01FANOUT");
    store
        .conn
        .execute(
            "INSERT INTO job_events (
                 kind, job_id, status_from, status_to, reason_kind, reason_value,
                 step_id, state_from, state_to, actor, at
             ) VALUES ('step_transition', '01FANOUT', 'awaiting_approval',
                 'awaiting_approval', 'escalation', 'fan_out', 'fix', 'running',
                 'stopped', 'fleet', '2026-08-26T10:00:00.000Z')",
            [],
        )
        .expect("the shape trigger admits the row; the level is not its business");

    match store.load_job(&job_id("01FANOUT")) {
        Err(crate::LoadJobError::Unreadable(RowError::MalformedColumn {
            table,
            column,
            detail,
        })) => {
            assert_eq!((table, column), ("job_events", "reason_value"));
            assert!(detail.contains("fan_out"), "{detail}");
        }
        other => panic!("expected a refusal, found {other:?}"),
    }
}

/// A step cannot stop without saying why. The schema holds it, so a row that
/// would fold into a stopped step with no verdict never lands.
#[test]
fn a_stop_with_no_trigger_is_refused_by_the_database() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01SILENTSTOP");
    let refused = store.conn.execute(
        "INSERT INTO job_events (
             kind, job_id, status_from, status_to, reason_kind, reason_value,
             step_id, state_from, state_to, actor, at
         ) VALUES ('step_transition', '01SILENTSTOP', 'awaiting_approval',
             'awaiting_approval', 'unqualified', NULL, 'fix', 'running',
             'stopped', 'fleet', '2026-08-26T10:00:00.000Z')",
        [],
    );
    assert!(refused.is_err(), "a stop that says nothing is not a shape");
}

/// The shape trigger, which is the schema holding a rule rather than a
/// convention. A step move with no `step_id` is not a row this log admits.
#[test]
fn a_log_row_that_is_neither_shape_whole_is_refused_by_the_database() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01HALFSHAPE");
    let refused = store.conn.execute(
        "INSERT INTO job_events (
             kind, job_id, status_from, status_to, reason_kind, reason_value,
             step_id, state_from, state_to, actor, at
         ) VALUES ('step_transition', '01HALFSHAPE', 'awaiting_approval',
             'awaiting_approval', 'unqualified', NULL, NULL, 'not_started',
             'running', 'fleet', '2026-08-26T10:00:00.000Z')",
        [],
    );
    assert!(
        refused.is_err(),
        "a step move with no step is not a shape this log holds"
    );
}

/// The column is a cache and the log is the authority, so a disagreement is
/// repaired rather than reported as damage — and the repair is reported, so it
/// is not silent either.
#[test]
fn a_cached_status_that_disagrees_with_the_log_is_corrected_and_reported() {
    let dir = TempDir::new();
    let mut store = seeded(&dir, "01STALE");
    let job = store.load_job(&job_id("01STALE")).expect("loads");
    let moved = job
        .transition(Target::Queued, Actor::Human, created_at())
        .expect("a legal move");
    store.record_transition(&moved).expect("recorded");
    store
        .conn
        .execute("UPDATE jobs SET status = 'killed'", [])
        .expect("a torn write, simulated");

    let loaded = store.load_all_jobs().expect("the log wins");
    assert_eq!(loaded.jobs[0].status(), JobStatus::Queued);
    assert_eq!(loaded.repaired.len(), 1);
    assert_eq!(loaded.repaired[0].cached, JobStatus::Killed);
    assert_eq!(loaded.repaired[0].folded, JobStatus::Queued);

    let cached: String = store
        .conn
        .query_row("SELECT status FROM jobs", [], |row| row.get(0))
        .expect("the column");
    assert_eq!(cached, "queued", "and the column is put right");
    assert!(
        store
            .load_all_jobs()
            .expect("clean now")
            .repaired
            .is_empty(),
        "a repair is not reported twice"
    );
}

#[test]
fn a_dispatch_origin_a_top_level_job_cannot_hold_is_named() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01CONFLICT");
    store
        .conn
        .execute(
            "UPDATE jobs SET dispatched_by_job_id = '01PARENT', dispatched_by_step_id = 'plan'",
            [],
        )
        .expect("scribbled on");

    match store.load_job(&job_id("01CONFLICT")) {
        Err(crate::LoadJobError::Unreadable(RowError::ColumnNotReconstructable {
            column, ..
        })) => assert_eq!(column, "dispatched_by"),
        other => panic!("expected a refusal, found {other:?}"),
    }
}

// ------------------------------------------------------------------- fixtures

fn seeded(dir: &TempDir, id: &str) -> Store {
    let mut store = open(dir);
    store
        .insert_job(&top_level(id), &created_at())
        .expect("stored");
    store
}

/// Append straight at the table. Nothing in this crate's API can write an event
/// the machine did not produce, which is why a test about a damaged log has to
/// go around it.
fn append_event(
    store: &Store,
    job: &str,
    from: &str,
    to: &str,
    reason_kind: &str,
    reason_value: Option<&str>,
) {
    store
        .conn
        .execute(
            "INSERT INTO job_events (job_id, status_from, status_to, reason_kind,
                                     reason_value, actor, at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'fleet', '2026-08-26T11:00:00.000Z')",
            rusqlite::params![job, from, to, reason_kind, reason_value],
        )
        .expect("appended");
}

fn version(dir: &TempDir) -> String {
    let conn = Connection::open(dir.db()).expect("open");
    conn.query_row(
        "SELECT value FROM armada_meta WHERE key = ?1",
        (SCHEMA_VERSION_KEY,),
        |row| row.get(0),
    )
    .expect("a version")
}

fn set_version(dir: &TempDir, value: &str) {
    let conn = Connection::open(dir.db()).expect("open");
    conn.execute(
        "UPDATE armada_meta SET value = ?2 WHERE key = ?1",
        (SCHEMA_VERSION_KEY, value),
    )
    .expect("set");
}
