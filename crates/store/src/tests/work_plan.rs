//! A Job's plan is its history, and the history outlives the process that kept
//! it. Every run here is reached by transitioning, for `attempt`'s reason.

use core_model::{
    Approach, DropReason, NewTask, PlanAuthor, PlanChange, PlanRefused, TaskId, TaskState,
    TaskUpdate,
};

use crate::tests::attempt::{on_its_first_run, run_it_again, step_id};
use crate::tests::{at, job_id, open, TempDir};
use crate::{PlanHand, PlanNotKept, Store};

fn recorded(titles: &[&str]) -> PlanChange {
    PlanChange::Recorded {
        approach: Approach::new("Bound the reader, then cover it").expect("an approach"),
        tasks: titles
            .iter()
            .map(|title| NewTask::new(title, "a detail").expect("a title"))
            .collect(),
    }
}

fn updated(id: &str, to: TaskUpdate) -> PlanChange {
    PlanChange::Updated {
        task: TaskId::read(id).expect("a task id"),
        to,
    }
}

fn kept(store: &mut Store, id: &str, change: PlanChange, by: PlanHand<'_>) {
    store
        .change_plan(&job_id(id), &change, by, &at("2026-08-26T10:05:00.000Z"))
        .expect("the change is kept");
}

#[test]
fn a_plan_and_every_change_to_it_survive_the_process_that_kept_them() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01WORKPLAN");
    let step = step_id();
    kept(
        &mut store,
        "01WORKPLAN",
        recorded(&["one", "two", "three"]),
        PlanHand::Step(&step),
    );
    kept(
        &mut store,
        "01WORKPLAN",
        updated("T1", TaskUpdate::Done),
        PlanHand::Step(&step),
    );
    let reason = DropReason::new("the writer never had the bug").expect("a reason");
    kept(
        &mut store,
        "01WORKPLAN",
        updated("T3", TaskUpdate::Dropped(reason)),
        PlanHand::Person,
    );
    drop(store);

    let reopened = open(&dir);
    let plan = reopened
        .work_plan(&job_id("01WORKPLAN"))
        .expect("the plan reads")
        .expect("a plan was recorded");
    assert_eq!(
        plan.tasks()
            .iter()
            .map(|task| task.state())
            .collect::<Vec<_>>(),
        [TaskState::Done, TaskState::Open, TaskState::Dropped]
    );
    assert_eq!(
        plan.tasks()[2].reason(),
        Some("the writer never had the bug")
    );
    assert_eq!(plan.tasks()[0].detail(), "a detail");
    assert!(matches!(
        plan.recorded_by(),
        PlanAuthor::Step { step_id, attempt } if step_id.as_str() == "fix" && attempt.number() == 1
    ));
    let history = reopened
        .plan_history(&job_id("01WORKPLAN"))
        .expect("the history reads");
    assert_eq!(history.len(), 3, "every change is a row of its own");
    assert_eq!(history[2].by, PlanAuthor::Person, "and says who made it");
}

#[test]
fn nothing_recorded_reads_as_no_plan() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01NOPLAN");
    assert_eq!(store.work_plan(&job_id("01NOPLAN")).expect("reads"), None);
}

#[test]
fn a_change_the_plan_cannot_take_writes_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01REFUSED");
    let step = step_id();
    let refused = store.change_plan(
        &job_id("01REFUSED"),
        &updated("T1", TaskUpdate::Done),
        PlanHand::Step(&step),
        &at("2026-08-26T10:05:00.000Z"),
    );
    assert!(matches!(
        refused,
        Err(PlanNotKept::Refused(PlanRefused::NoPlan))
    ));
    assert!(store
        .plan_history(&job_id("01REFUSED"))
        .expect("reads")
        .is_empty());

    kept(
        &mut store,
        "01REFUSED",
        recorded(&["one"]),
        PlanHand::Step(&step),
    );
    let unknown = store.change_plan(
        &job_id("01REFUSED"),
        &updated("T9", TaskUpdate::Done),
        PlanHand::Step(&step),
        &at("2026-08-26T10:06:00.000Z"),
    );
    assert!(matches!(
        unknown,
        Err(PlanNotKept::Refused(PlanRefused::NoSuchTask { .. }))
    ));
    assert_eq!(
        store
            .plan_history(&job_id("01REFUSED"))
            .expect("reads")
            .len(),
        1
    );
}

/// The recording step's retry replaces the plan, and is filed under its run.
#[test]
fn a_second_run_that_records_again_replaces_the_plan_under_its_own_run() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = on_its_first_run(&mut store, "01RETRIED");
    let step = step_id();
    kept(
        &mut store,
        "01RETRIED",
        recorded(&["one", "two"]),
        PlanHand::Step(&step),
    );
    kept(
        &mut store,
        "01RETRIED",
        updated("T1", TaskUpdate::Done),
        PlanHand::Step(&step),
    );
    run_it_again(
        &mut store,
        &job,
        "2026-08-26T10:07:00.000Z",
        "2026-08-26T10:08:00.000Z",
    );
    kept(
        &mut store,
        "01RETRIED",
        recorded(&["another"]),
        PlanHand::Step(&step),
    );

    let plan = store
        .work_plan(&job_id("01RETRIED"))
        .expect("reads")
        .expect("a plan");
    assert_eq!(plan.tasks().len(), 1);
    assert_eq!(plan.tasks()[0].state(), TaskState::Open);
    assert!(matches!(
        plan.recorded_by(),
        PlanAuthor::Step { attempt, .. } if attempt.number() == 2
    ));
}

#[test]
fn the_file_refuses_an_edit_a_removal_and_a_drop_with_no_reason() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01GUARDED");
    let step = step_id();
    kept(
        &mut store,
        "01GUARDED",
        recorded(&["one"]),
        PlanHand::Step(&step),
    );

    for statement in [
        "UPDATE job_work_plan_changes SET at = 'later'",
        "DELETE FROM job_work_plan_changes",
        "UPDATE job_work_plan_tasks SET title = 'another'",
        "DELETE FROM job_work_plan_tasks",
        "INSERT INTO job_work_plan_changes (job_id, seq, change, at, task_id, state, reason) \
         VALUES ('01GUARDED', 9, 'updated', 'now', 1, 'dropped', '  ')",
    ] {
        assert!(
            store.conn.execute(statement, []).is_err(),
            "the file took: {statement}"
        );
    }
}

#[test]
fn forgetting_a_job_counts_its_plan() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01FORGOTPLAN");
    let step = step_id();
    kept(
        &mut store,
        "01FORGOTPLAN",
        recorded(&["one", "two"]),
        PlanHand::Step(&step),
    );
    kept(
        &mut store,
        "01FORGOTPLAN",
        updated("T2", TaskUpdate::Working),
        PlanHand::Step(&step),
    );

    let gone = store
        .forget_job(&job_id("01FORGOTPLAN"))
        .expect("the job is forgotten");
    assert_eq!((gone.plan_changes, gone.plan_tasks, gone.other), (2, 2, 0));
}
