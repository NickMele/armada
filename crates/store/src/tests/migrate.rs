//! The second migration, and what it does to a Job the first one wrote.
//!
//! `MIGRATIONS` was built so a second entry could exist and had never had one.
//! This is the test of that claim, and of the answer V2 gives to the question a
//! second entry forces: **a Job written before the column existed is named
//! after itself**, visibly, one row at a time.
//!
//! A version 1 file is built here the way the world would produce one — by
//! running the first migration and nothing else, then writing a row through raw
//! SQL. Nothing in this crate's API can produce a Job without a title, which is
//! why a test about one has to go around it.

use core_model::{Actor, Target};
use rusqlite::Connection;

use crate::schema::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::{RowError, Store, KNOWN_SCHEMA_VERSION};

/// A file at version 1, with `ids` Jobs on it and no title column anywhere.
fn version_one(dir: &TempDir, ids: &[&str]) {
    let conn = Connection::open(dir.db()).expect("a file to put version 1 in");
    conn.execute_batch(MIGRATIONS[0])
        .expect("the first migration is the whole of version 1");
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '1')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 1");
    for id in ids {
        conn.execute(
            "INSERT INTO jobs (
                 job_id, status, workflow_id, owner_manifest_id, origin, urgency, atomic,
                 model, acceptance_criteria, dependencies, facts, scope_revisions,
                 write_targets_known, created_at
             ) VALUES (?1, 'awaiting_approval', '01WORKFLOW', '01OWNERMANIFEST', 'manual',
                       'normal', 0, 'a-model-name', '[]', '[]', '', '[]', 0,
                       '2026-08-26T09:00:00.000Z')",
            (id,),
        )
        .expect("a Job as version 1 wrote them");
    }
}

#[test]
fn a_version_one_file_is_brought_forward_rather_than_refused() {
    let dir = TempDir::new();
    version_one(&dir, &["01OLDONE"]);

    let store = Store::open(&dir.db()).expect("a version 1 file opens and is migrated");
    assert_eq!(recorded_version(&store), KNOWN_SCHEMA_VERSION.to_string());
    store
        .load_job(&job_id("01OLDONE"))
        .expect("and the Job that was on it is still readable");
}

/// The property the backfill exists for. A constant would have been shorter and
/// would leave a store where every Job is called the same thing, which reads
/// like twelve Jobs somebody named badly rather than twelve Jobs nobody named.
#[test]
fn every_migrated_job_gets_a_name_of_its_own_that_says_it_was_never_typed() {
    let dir = TempDir::new();
    version_one(&dir, &["01OLDONE", "01OLDTWO", "01OLDTHREE"]);

    let store = Store::open(&dir.db()).expect("migrated");
    let mut titles = Vec::new();
    for id in ["01OLDONE", "01OLDTWO", "01OLDTHREE"] {
        let job = store.load_job(&job_id(id)).expect("loads");
        assert_eq!(job.title().as_str(), format!("Untitled job {id}"));
        titles.push(job.title().clone());
    }
    titles.sort();
    titles.dedup();
    assert_eq!(titles.len(), 3, "no two migrated Jobs share a name");
}

/// Version 1 wrote no titles, so nothing it wrote may come back looking typed.
#[test]
fn a_migrated_title_is_never_blank_and_never_the_column_default() {
    let dir = TempDir::new();
    version_one(&dir, &["01OLDONE"]);
    let store = Store::open(&dir.db()).expect("migrated");

    let stored: String = store
        .conn
        .query_row("SELECT title FROM jobs", [], |row| row.get(0))
        .expect("the column");
    assert!(
        !stored.trim().is_empty(),
        "the DEFAULT '' is never the value"
    );
}

/// A Job written after the migration keeps the title it was given, and does not
/// pick up the backfill. Migrating twice must not rename anything.
#[test]
fn reopening_a_migrated_file_does_not_rename_the_jobs_on_it() {
    let dir = TempDir::new();
    version_one(&dir, &["01OLDONE"]);
    let mut store = Store::open(&dir.db()).expect("migrated");
    store
        .insert_job(&top_level("01NEWONE"), &created_at())
        .expect("a Job written after the migration");
    drop(store);

    let store = open(&dir);
    assert_eq!(
        store
            .load_job(&job_id("01OLDONE"))
            .expect("loads")
            .title()
            .as_str(),
        "Untitled job 01OLDONE"
    );
    assert_eq!(
        store
            .load_job(&job_id("01NEWONE"))
            .expect("loads")
            .title()
            .as_str(),
        "fix the off-by-one in the log reader"
    );
}

// ------------------------------------------- the column that may not be blank

/// The `ALTER` had to carry `DEFAULT ''` because SQLite will not add a
/// `NOT NULL` column without one, and `''` is the one value a title may not be.
/// The trigger is what closes that, and this is what says so.
#[test]
fn the_database_refuses_a_job_written_without_a_title() {
    let dir = TempDir::new();
    let store = open(&dir);
    let refused = store.conn.execute(
        "INSERT INTO jobs (
             job_id, status, workflow_id, owner_manifest_id, origin, urgency, atomic,
             model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES ('01BLANK', 'awaiting_approval', '01WF', '01MF', 'manual', 'normal', 0,
                   'a-model', '[]', '[]', '', '[]', 0, '2026-08-26T09:00:00.000Z')",
        [],
    );
    assert!(
        refused.is_err(),
        "the column default is not a title, and the trigger says so"
    );
}

#[test]
fn the_database_refuses_a_title_being_blanked_out() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01NAMED"), &created_at())
        .expect("stored");
    assert!(store
        .conn
        .execute("UPDATE jobs SET title = '   '", [])
        .is_err());
    // And the ordinary update the store does make still goes through.
    let job = store.load_job(&job_id("01NAMED")).expect("loads");
    let moved = job
        .transition(Target::Queued, Actor::Human, created_at())
        .expect("a legal move");
    store.record_transition(&moved).expect("recorded");
}

/// Reached the way it would be reached in the world — by something outside this
/// crate having written the row. Refused by name rather than read back as
/// "Untitled", which would hide that it ever happened.
#[test]
fn a_row_whose_title_is_blank_is_named_rather_than_substituted() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01SCRIBBLED"), &created_at())
        .expect("stored");
    store
        .conn
        .execute_batch(
            "DROP TRIGGER jobs_are_never_left_without_a_title;
             UPDATE jobs SET title = '';",
        )
        .expect("scribbled on");

    match store.load_job(&job_id("01SCRIBBLED")) {
        Err(crate::LoadJobError::Unreadable(RowError::MalformedColumn { column, .. })) => {
            assert_eq!(column, "title")
        }
        other => panic!("expected a refusal, found {other:?}"),
    }
}

fn recorded_version(store: &Store) -> String {
    store
        .conn
        .query_row(
            "SELECT value FROM armada_meta WHERE key = ?1",
            (SCHEMA_VERSION_KEY,),
            |row| row.get(0),
        )
        .expect("a version")
}
