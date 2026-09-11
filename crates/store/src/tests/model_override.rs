//! The model a person chose for a Job's later steps, and the migration that
//! makes room for it — tested here rather than in `migrate`, which is at the
//! lines the gate refuses at.

use rusqlite::Connection;

use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::{LoadJobError, Store, WriteError, KNOWN_SCHEMA_VERSION};

#[test]
fn a_fresh_job_has_no_chosen_model() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01FRESH"), &created_at())
        .expect("the job is stored");

    assert_eq!(
        store.model_override(&job_id("01FRESH")).expect("reads"),
        None
    );
}

#[test]
fn a_choice_survives_a_reopen_and_clearing_it_leaves_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01CHOSEN"), &created_at())
        .expect("the job is stored");
    let id = job_id("01CHOSEN");

    store
        .set_model_override(&id, Some("a-cheaper-model"))
        .expect("chosen");
    drop(store);
    let mut store = open(&dir);
    assert_eq!(
        store.model_override(&id).expect("reads"),
        Some("a-cheaper-model".to_string())
    );

    store.set_model_override(&id, None).expect("cleared");
    assert_eq!(store.model_override(&id).expect("reads"), None);
}

#[test]
fn a_job_that_does_not_exist_is_named_rather_than_defaulted() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let nobody = job_id("01NOBODY");

    assert!(matches!(
        store.model_override(&nobody),
        Err(LoadJobError::NoSuchJob { .. })
    ));
    assert!(matches!(
        store.set_model_override(&nobody, Some("a-model")),
        Err(WriteError::NoSuchJob { .. })
    ));
}

/// A file at version 46, with a Job written through raw SQL because nothing in
/// this crate's API writes one without the column.
fn version_forty_six(dir: &TempDir, id: &str) {
    let conn = Connection::open(dir.db()).expect("a file to put version 46 in");
    for migration in &MIGRATIONS[..46] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '46')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 46");
    conn.execute(
        "INSERT INTO jobs (
             job_id, title, status, workflow_id, owner_manifest_id, origin, urgency,
             atomic, model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES (?1, 'a job from before a model could be chosen', 'running', '01WORKFLOW',
                   '01OWNERMANIFEST', 'manual', 'normal', 0, 'a-model-name', '[]', '[]',
                   '', '[]', 0, '2026-09-11T09:00:00.000Z')",
        (id,),
    )
    .expect("a Job as version 46 wrote it");
}

/// The Job the migration found reads as nobody having chosen, and takes a
/// choice once it has run.
#[test]
fn a_job_written_at_the_previous_version_has_no_chosen_model() {
    let dir = TempDir::new();
    version_forty_six(&dir, "01BEFORECHOOSING");
    let mut store = Store::open(&dir.db()).expect("a version 46 file opens and is migrated");
    let recorded: String = store
        .conn
        .query_row(
            "SELECT value FROM armada_meta WHERE key = ?1",
            (SCHEMA_VERSION_KEY,),
            |row| row.get(0),
        )
        .expect("a version");
    assert_eq!(recorded, KNOWN_SCHEMA_VERSION.to_string());

    let old = job_id("01BEFORECHOOSING");
    assert_eq!(store.model_override(&old).expect("reads"), None);
    store
        .set_model_override(&old, Some("a-model"))
        .expect("the column exists for a Job the migration found");
    assert_eq!(
        store.model_override(&old).expect("reads"),
        Some("a-model".to_string())
    );
}
