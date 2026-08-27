//! **The test this milestone step exists for.**
//!
//! A Job is driven through both of its steps — started, advanced, started again
//! — and then every in-memory copy is dropped along with the connection that
//! wrote them. The file is reopened and the Job comes back **on the step it was
//! on**, with the row it advanced still saying so.
//!
//! It is the same shape as `reconstruct`, which proved a Job's status rebuilds,
//! and it is here rather than there because the two prove different halves: the
//! outer machine's history against the inner machine's, folded from one log in
//! one order.
//!
//! # Nothing here constructs a step in a state
//!
//! Every state is reached by transitioning, in the store as in `core-model`. A
//! test that wrote `state = 'advanced'` into the row and reopened would prove
//! that SQLite stores strings.

use core_model::{Actor, Job, JobStatus, StepId, StepState, StepTarget, StepVerdict, Target};

use crate::tests::{at, created_at, job_id, open, top_level, TempDir};
use crate::{Moved, Store};

fn first() -> StepId {
    StepId::new("reproduce")
}

fn second() -> StepId {
    StepId::new("fix")
}

/// A stored Job standing at `running`, which is where the inner machine moves.
/// Two moves to get there, both recorded, because a Job that reached `running`
/// any other way is not one this store could have.
fn running(store: &mut Store, id: &str) -> Job {
    let created = top_level(id);
    store
        .insert_job(&created, &created_at())
        .expect("the job is stored");
    let mut job = created;
    for (target, when) in [
        (Target::Queued, at("2026-08-26T10:00:00.000Z")),
        (Target::Running, at("2026-08-26T10:01:00.000Z")),
    ] {
        let moved = job
            .transition(target, Actor::Fleet, when)
            .expect("a legal move");
        store.record_transition(&moved).expect("recorded");
        job = moved.job;
    }
    job
}

/// Move a step and record it, failing loudly on either half.
fn step(store: &mut Store, job: &Job, step_id: &StepId, to: StepTarget, when: &str) -> Job {
    let moved = job
        .transition_step(step_id, to, Actor::Fleet, at(when))
        .unwrap_or_else(|e| panic!("moving {}: {e}", step_id.as_str()));
    store
        .record_step_transition(&moved)
        .expect("the step move is recorded");
    moved.job
}

/// Start the first step, advance it, start the second. Three step moves on top
/// of two status transitions, interleaved in one log.
fn drive(store: &mut Store, job: Job) -> Job {
    let job = step(
        store,
        &job,
        &first(),
        StepTarget::Running,
        "2026-08-26T10:02:00.000Z",
    );
    let job = step(
        store,
        &job,
        &first(),
        StepTarget::Advanced,
        "2026-08-26T10:03:00.000Z",
    );
    step(
        store,
        &job,
        &second(),
        StepTarget::Running,
        "2026-08-26T10:04:00.000Z",
    )
}

#[test]
fn a_job_comes_back_on_the_step_it_advanced_to_after_the_process_is_gone() {
    let dir = TempDir::new();
    let id = job_id("01CURSOR");

    let expected = {
        let mut store = open(&dir);
        let job = running(&mut store, "01CURSOR");
        drive(&mut store, job)
        // `store` drops here: the connection closes and every in-memory Job
        // with it. Nothing below has seen the value it is about to assert on.
    };
    assert_eq!(expected.current_step_id(), Some(&second()));

    let reopened = open(&dir);
    let rebuilt = reopened.load_job(&id).expect("the job rebuilds");

    // Not just the cursor: the whole record, field for field, including both
    // `job_steps` rows and their verdicts and timestamps.
    assert_eq!(rebuilt, expected);
}

/// Named separately from the whole-record assertion above, because the record
/// is the thing that grows: a cursor that silently stopped surviving would take
/// a whole-record comparison down with a message naming no field.
#[test]
fn the_cursor_and_both_step_rows_survive_the_drop_and_the_reopen() {
    let dir = TempDir::new();
    {
        let mut store = open(&dir);
        let job = running(&mut store, "01STEPROWS");
        drive(&mut store, job);
    }

    let reopened = open(&dir);
    let rebuilt = reopened.load_job(&job_id("01STEPROWS")).expect("it folds");

    assert_eq!(rebuilt.status(), JobStatus::Running, "the Job did not move");
    assert_eq!(rebuilt.current_step_id(), Some(&second()));

    let advanced = rebuilt.step(&first()).expect("the first row");
    assert_eq!(advanced.state(), StepState::Advanced);
    assert_eq!(advanced.last_verdict(), Some(StepVerdict::Passed));
    assert_eq!(
        advanced.entered_at(),
        &at("2026-08-26T10:02:00.000Z"),
        "entered when it started, not when it advanced"
    );
    assert_eq!(advanced.updated_at(), &at("2026-08-26T10:03:00.000Z"));

    let current = rebuilt.current_step().expect("the row the cursor names");
    assert_eq!(current.step_id(), &second());
    assert_eq!(current.state(), StepState::Running);
    assert_eq!(current.last_verdict(), None, "nothing has ruled on it");
}

#[test]
fn the_log_holds_both_machines_in_one_order() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = running(&mut store, "01ONELOG");
    drive(&mut store, job);
    drop(store);

    let reopened = open(&dir);
    let events = reopened
        .events_for(&job_id("01ONELOG"))
        .expect("the log reads back");

    assert_eq!(events.len(), 5, "two status moves and three step moves");
    let kinds: Vec<&str> = events
        .iter()
        .map(|event| match event.moved() {
            Moved::Job { .. } => "job",
            Moved::Step { .. } => "step",
            Moved::Drone { .. } => "drone",
        })
        .collect();
    assert_eq!(kinds, vec!["job", "job", "step", "step", "step"]);

    assert_eq!(
        events[2].moved(),
        &Moved::Step {
            step_id: first(),
            from: StepState::NotStarted,
            to: StepState::Running,
        }
    );
    assert_eq!(
        events[2].under(),
        JobStatus::Running,
        "a step move records the status it happened beneath, and the Job stays there"
    );

    let keys: Vec<i64> = events.iter().map(|event| event.seq()).collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(keys, sorted, "one sequence, both kinds");
}

/// The cached columns and the fold agree because they are written in one
/// transaction. Read straight out of SQL rather than through the fold, which is
/// the only way to see the cache at all.
#[test]
fn the_cached_columns_are_written_with_the_event_that_moved_them() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = running(&mut store, "01CACHE");
    drive(&mut store, job);

    let cursor: String = store
        .conn
        .query_row("SELECT current_step_id FROM jobs", [], |row| row.get(0))
        .expect("the column");
    assert_eq!(cursor, "fix");

    let states: Vec<String> = store
        .conn
        .prepare("SELECT state FROM job_steps ORDER BY ordinal")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get(0))
                .and_then(|rows| rows.collect())
        })
        .expect("the columns");
    assert_eq!(states, vec!["advanced", "running"]);
}

/// A step cannot be driven beneath a status the inner machine is frozen under,
/// so nothing is written and nothing needs undoing.
#[test]
fn a_step_move_the_machine_refuses_never_reaches_the_store() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let created = top_level("01FROZEN");
    store
        .insert_job(&created, &created_at())
        .expect("the job is stored");

    let refused = created.transition_step(
        &first(),
        StepTarget::Running,
        Actor::Fleet,
        at("2026-08-26T10:00:00.000Z"),
    );
    assert!(
        refused.is_err(),
        "awaiting_approval is not a status a step advances beneath"
    );
    assert_eq!(
        store
            .events_for(&job_id("01FROZEN"))
            .expect("the log")
            .len(),
        0,
        "a refusal writes nothing, because there is no event to hand the store"
    );
}
