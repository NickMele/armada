//! The migrations after the first, and what each does to a Job an earlier one
//! wrote.
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
//!
//! **V3 backfills nothing**, and every log row an earlier version could hold is
//! a status transition because there was no other kind to write. **V7 backfills
//! nothing either**, and there the absence is fatal to the row: a pre-V7 Job
//! cannot say what workflow it froze, so it is refused by name while the file
//! around it still migrates.

use core_model::{Actor, JobStatus, Target, TransitionReason};
use rusqlite::Connection;

use crate::schema::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::{Moved, RowError, Store, KNOWN_SCHEMA_VERSION};

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
    assert_eq!(
        titles(&store),
        vec!["Untitled job 01OLDONE".to_string()],
        "and every column a later version added was filled in"
    );
}

/// The one migration with nothing honest to backfill, and what it costs.
///
/// A Job written before V7 cannot say which WorkflowDef it followed, and the
/// step it names cannot be shown to declare anything — writing "no Checks"
/// would read as an ungated step. So it is named rather than guessed at, and
/// [`Store::load_all_jobs`] hands the refusal back beside the Jobs that loaded
/// instead of shortening the list.
#[test]
fn a_job_written_before_the_workflow_was_frozen_is_named_rather_than_guessed_at() {
    let dir = TempDir::new();
    version_one(&dir, &["01OLDONE"]);
    let mut store = Store::open(&dir.db()).expect("migrated");
    store
        .insert_job(&top_level("01NEWONE"), &created_at())
        .expect("a Job written after the migration");

    match store.load_job(&job_id("01OLDONE")) {
        Err(crate::LoadJobError::Unreadable(RowError::WorkflowNotFrozen { job_id })) => {
            assert_eq!(job_id.as_str(), "01OLDONE")
        }
        other => panic!("expected a named refusal, found {other:?}"),
    }

    match store.load_all_jobs() {
        Err(crate::LoadAllError::SomeJobsUnreadable { loaded, failed }) => {
            assert_eq!(loaded.jobs.len(), 1, "the readable Job still comes back");
            assert_eq!(loaded.jobs[0].id().as_str(), "01NEWONE");
            assert_eq!(failed.len(), 1, "and the unreadable one is carried out");
            // The seam `armada clean` removes it through: no fold, no rebuild,
            // just the two columns every migration keeps.
            let named = failed[0].row.as_ref().expect("the row still names itself");
            assert_eq!(named.job_id.as_str(), "01OLDONE");
            assert_eq!(named.owner_manifest_id.as_str(), "01OWNERMANIFEST");
        }
        other => panic!("expected a partial failure, found {other:?}"),
    }
}

/// The property the backfill exists for. A constant would have been shorter and
/// would leave a store where every Job is called the same thing, which reads
/// like twelve Jobs somebody named badly rather than twelve Jobs nobody named.
#[test]
fn every_migrated_job_gets_a_name_of_its_own_that_says_it_was_never_typed() {
    let dir = TempDir::new();
    version_one(&dir, &["01OLDONE", "01OLDTWO", "01OLDTHREE"]);

    let store = Store::open(&dir.db()).expect("migrated");
    let mut named = titles(&store);
    assert_eq!(
        named,
        vec![
            "Untitled job 01OLDONE".to_string(),
            "Untitled job 01OLDTHREE".to_string(),
            "Untitled job 01OLDTWO".to_string(),
        ]
    );
    named.dedup();
    assert_eq!(named.len(), 3, "no two migrated Jobs share a name");
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
        titles(&store),
        vec![
            "fix the off-by-one in the log reader".to_string(),
            "Untitled job 01OLDONE".to_string(),
        ],
        "the backfilled name and the typed one both survive a second migration"
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

/// Every `title` column, in `job_id` order. Read straight off the row because
/// the Jobs these tests write predate the frozen workflow and do not rebuild —
/// which is the point of the test above, and not of these.
fn titles(store: &Store) -> Vec<String> {
    let mut statement = store
        .conn
        .prepare("SELECT title FROM jobs ORDER BY job_id")
        .expect("the column");
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("titles");
    rows.map(|title| title.expect("a title")).collect()
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

// -------------------------------------------------------------- version three

/// A file at version 2, with one Job and one recorded transition on it — a log
/// row from before there was a second kind of row.
fn version_two(dir: &TempDir, id: &str) {
    let conn = Connection::open(dir.db()).expect("a file to put version 2 in");
    conn.execute_batch(MIGRATIONS[0])
        .expect("the first migration");
    conn.execute_batch(MIGRATIONS[1])
        .expect("the second migration");
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '2')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 2");
    conn.execute(
        "INSERT INTO jobs (
             job_id, title, status, workflow_id, owner_manifest_id, origin, urgency, atomic,
             model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES (?1, 'a job from before the step log', 'queued', '01WORKFLOW',
                   '01OWNERMANIFEST', 'manual', 'normal', 0, 'a-model-name', '[]', '[]', '',
                   '[]', 0, '2026-08-26T09:00:00.000Z')",
        (id,),
    )
    .expect("a Job as version 2 wrote them");
    conn.execute(
        "INSERT INTO job_events (
             job_id, status_from, status_to, reason_kind, reason_value, actor, at
         ) VALUES (?1, 'awaiting_approval', 'queued', 'derived_at_read', NULL, 'human',
                   '2026-08-26T09:30:00.000Z')",
        (id,),
    )
    .expect("a transition as version 2 wrote them");
    conn.execute(
        "INSERT INTO job_steps (job_id, step_id, ordinal, state, last_verdict, entered_at,
                                updated_at)
         VALUES (?1, 'reproduce', 0, 'not_started', NULL, '2026-08-26T09:00:00.000Z',
                 '2026-08-26T09:00:00.000Z')",
        (id,),
    )
    .expect("a step row as version 2 wrote them");
}

/// A log row written before `kind` existed is a status transition, and folds as
/// one. The column's `DEFAULT` is the whole of the backfill because there is no
/// distinction among old rows to lose — V2's per-row backfill answered a
/// question V3 does not have.
#[test]
fn a_log_row_from_before_the_step_log_is_still_a_status_transition() {
    let dir = TempDir::new();
    version_two(&dir, "01BEFORESTEPS");

    let store = Store::open(&dir.db()).expect("a version 2 file opens and is migrated");
    assert_eq!(recorded_version(&store), KNOWN_SCHEMA_VERSION.to_string());

    let events = store
        .events_for(&job_id("01BEFORESTEPS"))
        .expect("the log reads back");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].moved(),
        &Moved::Job {
            to: JobStatus::Queued,
            reason: TransitionReason::DerivedAtRead,
        }
    );

    // The Job itself does not rebuild — it was written before V7 froze a
    // workflow onto the row — so what is asserted here is the log, which is
    // what V3 changed. The fold over it is `reconstruct`'s subject.
    assert!(matches!(
        store.load_job(&job_id("01BEFORESTEPS")),
        Err(crate::LoadJobError::Unreadable(
            RowError::WorkflowNotFrozen { .. }
        ))
    ));
    let cursor: Option<String> = store
        .conn
        .query_row("SELECT current_step_id FROM jobs", [], |row| row.get(0))
        .expect("the cursor column");
    assert_eq!(
        cursor, None,
        "no step move was ever recorded, so the cursor was never set"
    );
}

/// The migration adds columns and a trigger. It must not touch a row.
#[test]
fn version_three_leaves_every_existing_row_exactly_as_it_found_it() {
    let dir = TempDir::new();
    version_two(&dir, "01UNTOUCHED");
    let before = row_digest(&dir);

    let store = Store::open(&dir.db()).expect("migrated");
    drop(store);

    assert_eq!(
        row_digest(&dir),
        before,
        "V3 rewrote a row it should not have"
    );
}

/// Every value V3 could have overwritten, as one string.
fn row_digest(dir: &TempDir) -> String {
    let conn = Connection::open(dir.db()).expect("the file");
    let jobs: String = conn
        .query_row(
            "SELECT title || '|' || status || '|' || ifnull(current_step_id, '-') FROM jobs",
            [],
            |row| row.get(0),
        )
        .expect("the job row");
    let steps: String = conn
        .query_row(
            "SELECT state || '|' || ifnull(last_verdict, '-') FROM job_steps",
            [],
            |row| row.get(0),
        )
        .expect("the step row");
    let events: String = conn
        .query_row(
            "SELECT status_from || '|' || status_to || '|' || reason_kind FROM job_events",
            [],
            |row| row.get(0),
        )
        .expect("the event row");
    format!("{jobs}//{steps}//{events}")
}

// --------------------------------------------------------------- version five

/// A file at version 4, with one Job that reached `running` and one that never
/// left the gate. Neither has a branch column to hold anything.
fn version_four(dir: &TempDir, ran: &str, never_ran: &str) {
    let conn = Connection::open(dir.db()).expect("a file to put version 4 in");
    for migration in &MIGRATIONS[..4] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '4')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 4");
    for (id, status) in [(ran, "running"), (never_ran, "awaiting_approval")] {
        conn.execute(
            "INSERT INTO jobs (
                 job_id, title, status, workflow_id, owner_manifest_id, origin, urgency,
                 atomic, model, acceptance_criteria, dependencies, facts, scope_revisions,
                 write_targets_known, created_at
             ) VALUES (?1, 'a job from before the branch column', ?2, '01WORKFLOW',
                       '01OWNERMANIFEST', 'manual', 'normal', 0, 'a-model-name', '[]', '[]',
                       '', '[]', 0, '2026-08-26T09:00:00.000Z')",
            (id, status),
        )
        .expect("a Job as version 4 wrote them");
    }
    conn.execute(
        "INSERT INTO job_events (
             kind, job_id, status_from, status_to, reason_kind, reason_value, actor, at
         ) VALUES ('job_transition', ?1, 'queued', 'running', 'unqualified', NULL, 'fleet',
                   '2026-08-26T09:30:00.000Z')",
        (ran,),
    )
    .expect("the move that proves a worktree was made");
}

/// **The backfill is what the log says, not what every row could be told.** A
/// Job that reached `running` had a worktree, and every build that made one
/// named the branch after the Job; a Job that never ran has no branch and is
/// left with none rather than given one it never had.
#[test]
fn version_five_names_the_branch_of_a_job_that_ran_and_no_other() {
    let dir = TempDir::new();
    version_four(&dir, "01ITRAN", "01ITNEVERRAN");

    let store = Store::open(&dir.db()).expect("a version 4 file opens and is migrated");
    assert_eq!(recorded_version(&store), KNOWN_SCHEMA_VERSION.to_string());

    assert_eq!(
        branch_of(&store, "01ITRAN").as_deref(),
        Some("armada/01ITRAN")
    );
    assert_eq!(
        branch_of(&store, "01ITNEVERRAN"),
        None,
        "no worktree was ever made, so there is nothing true to write"
    );
}

fn branch_of(store: &Store, job_id: &str) -> Option<String> {
    store
        .conn
        .query_row(
            "SELECT branch FROM jobs WHERE job_id = ?1",
            (job_id,),
            |row| row.get::<_, Option<String>>(0),
        )
        .expect("the Job is there")
}

// ------------------------------------------------------------ version thirteen

/// A file at version 12, with one Job carrying a row in each of the four
/// per-step tables. None of them has an `attempt` column to hold anything.
fn version_twelve(dir: &TempDir, id: &str) {
    let conn = Connection::open(dir.db()).expect("a file to put version 12 in");
    for migration in &MIGRATIONS[..12] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '12')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 12");
    conn.execute(
        "INSERT INTO jobs (
             job_id, title, status, workflow_id, owner_manifest_id, origin, urgency,
             atomic, model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES (?1, 'a job from before the attempt column', 'running', '01WORKFLOW',
                   '01OWNERMANIFEST', 'manual', 'normal', 0, 'a-model-name', '[]', '[]',
                   '', '[]', 0, '2026-08-26T09:00:00.000Z')",
        (id,),
    )
    .expect("a Job as version 12 wrote it");
    for statement in [
        "INSERT INTO job_step_checks (job_id, step_id, ordinal, name, outcome, ran_at)
         VALUES (?1, 'fix', 0, 'build', 'passed', '2026-08-26T09:30:00.000Z')",
        "INSERT INTO job_step_judgments (job_id, step_id, ordinal, criterion, verdict, judged_at)
         VALUES (?1, 'fix', 0, 'c1', 'met', '2026-08-26T09:31:00.000Z')",
        "INSERT INTO job_step_gaming_flags (job_id, step_id, ordinal, pattern, cited, flagged_at)
         VALUES (?1, 'fix', 0, 'assertion_weakened', 'src/read.rs:88',
                 '2026-08-26T09:32:00.000Z')",
        "INSERT INTO job_step_evidence (
             job_id, step_id, evidence_type, claimed, shown_by, not_claimed, recorded_at
         ) VALUES (?1, 'fix', 'diff', 'the fix', 'the patch', 'nothing else',
                   '2026-08-26T09:33:00.000Z')",
    ] {
        conn.execute(statement, (id,))
            .expect("a per-step row as version 12 wrote it");
    }
}

/// **The one migration that rebuilds a table, and every row has to come out the
/// other side.**
///
/// `SQLite` cannot widen a primary key, so the four per-step tables are
/// recreated and copied rather than altered. That is the migration shape most
/// able to lose rows silently, and there is real data in a store on disk — so
/// the test is that each row is still there and is called attempt one.
///
/// One is not a guess about an old row, unlike V2's title. Nothing before this
/// migration could write a second run's rows, because every writer deleted the
/// first run's; so one is the only thing an existing row can be.
#[test]
fn version_thirteen_carries_every_existing_row_across_as_the_first_attempt() {
    let dir = TempDir::new();
    version_twelve(&dir, "01BEFOREATTEMPTS");

    let store = Store::open(&dir.db()).expect("a version 12 file opens and is migrated");
    assert_eq!(recorded_version(&store), KNOWN_SCHEMA_VERSION.to_string());

    for (table, column, value) in [
        ("job_step_checks", "name", "build"),
        ("job_step_judgments", "criterion", "c1"),
        ("job_step_gaming_flags", "cited", "src/read.rs:88"),
        ("job_step_evidence", "claimed", "the fix"),
    ] {
        let found: (i64, String) = store
            .conn
            .query_row(
                &format!("SELECT attempt, {column} FROM {table} WHERE job_id = ?1"),
                ("01BEFOREATTEMPTS",),
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or_else(|cause| panic!("the row in {table} survived: {cause}"));
        assert_eq!(found, (1, value.to_string()), "{table} kept its row");
    }
}

/// The rule the rebuilt tables hold from underneath. Zero is not an attempt,
/// and `Attempt` cannot hold one — so a row written by something that did not
/// share the type is refused rather than read as a first run.
#[test]
fn an_attempt_of_zero_is_refused_by_the_database_itself() {
    let dir = TempDir::new();
    version_twelve(&dir, "01BEFOREATTEMPTS");
    let store = Store::open(&dir.db()).expect("migrated");

    let refused = store.conn.execute(
        "INSERT INTO job_step_judgments (
             job_id, step_id, attempt, ordinal, criterion, verdict, judged_at
         ) VALUES ('01BEFOREATTEMPTS', 'fix', 0, 1, 'c1', 'met', '2026-08-26T09:34:00.000Z')",
        [],
    );
    assert!(refused.is_err(), "a run numbered zero never happened");
}
