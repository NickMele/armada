//! How a Job meets a blocked command, and what a person allowed it to run.
//!
//! The migration is tested from the version before it, the way `migrate` tests
//! every other: a file built by the earlier migrations alone, with a Job
//! written through raw SQL because nothing in this crate's API writes one
//! without the column.

use core_model::{Actor, AllowedCommand, Reach, WhenBlocked};
use rusqlite::Connection;

use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{at, created_at, job_id, open, top_level, TempDir};
use crate::{LoadJobError, Store, WriteError, KNOWN_SCHEMA_VERSION};

fn allowed(run: &str, reach: Reach, instant: &str) -> AllowedCommand {
    AllowedCommand {
        run: run.to_string(),
        reach,
        allowed_at: at(instant),
        by: Actor::Human,
    }
}

fn a_job(store: &mut Store, id: &str) {
    store
        .insert_job(&top_level(id), &created_at())
        .expect("the job is stored");
}

/// A file at version 43, with one Job on it and no `when_blocked` column.
fn version_forty_three(dir: &TempDir, id: &str) {
    let conn = Connection::open(dir.db()).expect("a file to put version 43 in");
    for migration in &MIGRATIONS[..43] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '43')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 43");
    conn.execute(
        "INSERT INTO jobs (
             job_id, title, status, workflow_id, owner_manifest_id, origin, urgency,
             atomic, model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES (?1, 'a job from before it could be asked', 'running', '01WORKFLOW',
                   '01OWNERMANIFEST', 'manual', 'normal', 0, 'a-model-name', '[]', '[]',
                   '', '[]', 0, '2026-08-26T09:00:00.000Z')",
        (id,),
    )
    .expect("a Job as version 43 wrote it");
}

#[test]
fn a_job_on_a_fresh_store_refuses_and_holds_and_has_nothing_allowed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01FRESH");

    assert_eq!(
        store.when_blocked(&job_id("01FRESH")).expect("reads"),
        WhenBlocked::RefuseAndHold
    );
    assert!(store
        .allowed_commands(&job_id("01FRESH"))
        .expect("reads")
        .is_empty());
}

/// The backfill is the column's `DEFAULT`, and the table arrives usable.
#[test]
fn a_job_written_at_the_previous_version_refuses_and_holds() {
    let dir = TempDir::new();
    version_forty_three(&dir, "01BEFOREASKING");
    let mut store = Store::open(&dir.db()).expect("a version 43 file opens and is migrated");
    let recorded: String = store
        .conn
        .query_row(
            "SELECT value FROM armada_meta WHERE key = ?1",
            (SCHEMA_VERSION_KEY,),
            |row| row.get(0),
        )
        .expect("a version");
    assert_eq!(recorded, KNOWN_SCHEMA_VERSION.to_string());

    let old = job_id("01BEFOREASKING");
    assert_eq!(
        store.when_blocked(&old).expect("reads"),
        WhenBlocked::RefuseAndHold
    );
    assert!(store.allowed_commands(&old).expect("reads").is_empty());
    store
        .allow_command(
            &old,
            &allowed("cargo test", Reach::Job, "2026-08-26T10:00:00.000Z"),
        )
        .expect("the table exists for a Job the migration found");
    assert_eq!(store.allowed_commands(&old).expect("reads").len(), 1);
}

#[test]
fn a_setting_reads_back_as_set_and_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01SET");
    let id = job_id("01SET");

    store
        .set_when_blocked(&id, WhenBlocked::AskMe)
        .expect("set");
    assert_eq!(store.when_blocked(&id).expect("reads"), WhenBlocked::AskMe);
    drop(store);

    let mut store = open(&dir);
    assert_eq!(store.when_blocked(&id).expect("reads"), WhenBlocked::AskMe);
    store
        .set_when_blocked(&id, WhenBlocked::RefuseAndHold)
        .expect("set back");
    assert_eq!(
        store.when_blocked(&id).expect("reads"),
        WhenBlocked::RefuseAndHold
    );
}

/// Every setting survives the column, so a later one is not read back as
/// unreadable by the enum that wrote it.
#[test]
fn every_setting_round_trips_through_the_column() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01EVERY");
    let id = job_id("01EVERY");

    for setting in WhenBlocked::ALL {
        store.set_when_blocked(&id, *setting).expect("set");
        assert_eq!(store.when_blocked(&id).expect("reads"), *setting);
    }
}

#[test]
fn a_job_that_does_not_exist_is_named_rather_than_defaulted() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let nobody = job_id("01NOBODY");

    assert!(matches!(
        store.when_blocked(&nobody),
        Err(LoadJobError::NoSuchJob { .. })
    ));
    assert!(matches!(
        store.set_when_blocked(&nobody, WhenBlocked::AskMe),
        Err(WriteError::NoSuchJob { .. })
    ));
    assert!(matches!(
        store.allow_command(
            &nobody,
            &allowed("cargo test", Reach::Job, "2026-08-26T10:00:00.000Z")
        ),
        Err(WriteError::NoSuchJob { .. })
    ));
}

#[test]
fn allowing_one_command_twice_is_one_row_as_first_allowed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01TWICE");
    let id = job_id("01TWICE");

    let first = allowed("cargo test", Reach::Job, "2026-08-26T10:00:00.000Z");
    store.allow_command(&id, &first).expect("allowed");
    store
        .allow_command(
            &id,
            &allowed("cargo test", Reach::Job, "2026-08-26T10:05:00.000Z"),
        )
        .expect("allowed again");

    assert_eq!(store.allowed_commands(&id).expect("reads"), vec![first]);
}

/// Made permanent in the repository is a wider allow, and a later allow for
/// the Job alone does not take it back.
#[test]
fn a_repository_allow_raises_a_job_allow_and_nothing_lowers_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01RAISED");
    let id = job_id("01RAISED");

    store
        .allow_command(
            &id,
            &allowed("cargo test", Reach::Job, "2026-08-26T10:00:00.000Z"),
        )
        .expect("allowed for the Job");
    store
        .allow_command(
            &id,
            &allowed("cargo test", Reach::Repository, "2026-08-26T10:05:00.000Z"),
        )
        .expect("made permanent");
    store
        .allow_command(
            &id,
            &allowed("cargo test", Reach::Job, "2026-08-26T10:09:00.000Z"),
        )
        .expect("allowed for the Job again");

    assert_eq!(
        store.allowed_commands(&id).expect("reads"),
        vec![allowed(
            "cargo test",
            Reach::Repository,
            "2026-08-26T10:00:00.000Z"
        )],
        "one row, at repository reach, still dated by the first allow"
    );
}

#[test]
fn allowed_commands_come_back_oldest_first_whoever_allowed_them() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01ORDERED");
    let id = job_id("01ORDERED");

    let later = allowed("cargo build", Reach::Repository, "2026-08-26T10:05:00.000Z");
    let earlier = AllowedCommand {
        by: Actor::Fleet,
        ..allowed("cargo fmt", Reach::Job, "2026-08-26T10:01:00.000Z")
    };
    store.allow_command(&id, &later).expect("allowed");
    store.allow_command(&id, &earlier).expect("allowed");

    assert_eq!(
        store.allowed_commands(&id).expect("reads"),
        vec![earlier, later]
    );
}

/// One row goes, by its exact text, and the Job's other allows and its
/// neighbour's stay. A second take-back finds nothing and says so.
#[test]
fn taking_back_an_allow_removes_that_row_and_no_other() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01TAKEN");
    a_job(&mut store, "01KEPT");
    let (id, kept) = (job_id("01TAKEN"), job_id("01KEPT"));
    for run in ["cargo test", "cargo build"] {
        store
            .allow_command(&id, &allowed(run, Reach::Job, "2026-08-26T10:00:00.000Z"))
            .expect("allowed");
    }
    store
        .allow_command(
            &kept,
            &allowed("cargo test", Reach::Repository, "2026-08-26T10:00:00.000Z"),
        )
        .expect("allowed");

    assert!(!store.remove_allowed_command(&id, "cargo").expect("reads"));
    assert!(store
        .remove_allowed_command(&id, "cargo test")
        .expect("removed"));

    let left: Vec<String> = store
        .allowed_commands(&id)
        .expect("reads")
        .into_iter()
        .map(|allow| allow.run)
        .collect();
    assert_eq!(left, vec!["cargo build"]);
    assert_eq!(store.allowed_commands(&kept).expect("reads").len(), 1);
    assert!(
        !store
            .remove_allowed_command(&id, "cargo test")
            .expect("reads"),
        "already gone"
    );
    assert!(!store
        .remove_allowed_command(&job_id("01NOBODY"), "cargo test")
        .expect("no Job, no row"));
}

#[test]
fn forgetting_a_job_takes_what_was_allowed_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01FORGOTTEN");
    a_job(&mut store, "01KEPT");
    for id in ["01FORGOTTEN", "01KEPT"] {
        for run in ["cargo test", "cargo build"] {
            store
                .allow_command(
                    &job_id(id),
                    &allowed(run, Reach::Job, "2026-08-26T10:00:00.000Z"),
                )
                .expect("allowed");
        }
    }

    let gone = store.forget_job(&job_id("01FORGOTTEN")).expect("forgotten");

    assert_eq!(gone.allowed_commands, 2);
    assert_eq!(gone.other, 0, "counted by name, not in the lump");
    assert!(store
        .allowed_commands(&job_id("01FORGOTTEN"))
        .expect("reads")
        .is_empty());
    assert_eq!(
        store
            .allowed_commands(&job_id("01KEPT"))
            .expect("reads")
            .len(),
        2,
        "the neighbour's allows are not the forgotten Job's"
    );
}
