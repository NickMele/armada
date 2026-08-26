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

use core_model::{Actor, JobStatus, Target};
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
         DELETE FROM job_manifests; DELETE FROM jobs;",
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

/// Nothing advances a step yet. A row that says one has was written by a
/// machine that does not exist, and reading it back as `not_started` would be
/// the data loss this whole step exists to prevent.
#[test]
fn a_step_the_rebuild_cannot_put_back_is_named_rather_than_reset() {
    let dir = TempDir::new();
    let store = seeded(&dir, "01ADVANCED");
    store
        .conn
        .execute(
            "UPDATE job_steps SET state = 'advanced' WHERE step_id = 'fix'",
            [],
        )
        .expect("scribbled on");

    match store.load_job(&job_id("01ADVANCED")) {
        Err(crate::LoadJobError::Unreadable(RowError::StepStateNotReconstructable {
            step_id,
            state,
            ..
        })) => {
            assert_eq!(step_id.as_str(), "fix");
            assert_eq!(state, "advanced");
        }
        other => panic!("expected a refusal, found {other:?}"),
    }
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
