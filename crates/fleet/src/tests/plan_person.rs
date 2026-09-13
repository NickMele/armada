//! A person's `add_task` and `drop_task`, refused by name and kept under
//! `PlanAuthor::Person` when they are not. `#897`.
//!
//! **No Drone is dispatched here.** These are about the change and its
//! refusals, which are decided before delivery is ever asked about — the
//! working Drone that is or is not told is `crate::tests::plan_person_told`.

use std::sync::Arc;
use std::time::Duration;

use api::{Next, Refusal, Subscription};
use core_model::{
    Approach, NewTask, PlanAuthor, PlanChange, StepId, TaskId, TaskUpdate, Timestamp,
};
use ipc::{AddTask, DropTask};
use store::PlanHand;
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::daemon::{a_fleet, a_proposal};
use crate::tests::tmp::TempDir;

/// The `WireError.code` a refusal carries, whichever of the four variants it
/// answers as.
fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

async fn a_job(
    fleet: &Arc<Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>>,
) -> ipc::JobId {
    let job = fleet
        .propose(a_proposal("cover the writer's bound"))
        .await
        .expect("a Job at the gate");
    ipc::JobId::from(job.id())
}

/// Record a four-task plan directly, `store::PlanHand::Step`'s way — the
/// plan step's own act, which every case here starts from.
async fn a_plan_of_four(
    fleet: &Arc<Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>>,
    job_id: &ipc::JobId,
) {
    let job = job_id.to_domain();
    let mut store = fleet.store().lock().await;
    store
        .change_plan(
            &job,
            &PlanChange::Recorded {
                approach: Approach::new("Bound the reader").expect("an approach"),
                tasks: vec![
                    NewTask::new("Stop at the end", "").expect("a title"),
                    NewTask::new("Cover it", "").expect("a title"),
                    NewTask::new("Check the writer too", "").expect("a title"),
                    NewTask::new("Note it in the module", "").expect("a title"),
                ],
            },
            PlanHand::Step(&StepId::new("implement")),
            &Timestamp::from_rfc3339("2026-09-13T10:00:00.000Z"),
        )
        .expect("recorded");
}

fn a_fixture(
    home: &TempDir,
) -> Arc<Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>> {
    Arc::new(a_fleet(home, FakeWorkProduct::changed(&["src/log.rs"])))
}

#[tokio::test]
async fn adding_to_a_job_with_no_plan_is_refused_by_name() {
    let home = TempDir::new();
    let fleet = a_fixture(&home);
    let job_id = a_job(&fleet).await;

    let add = AddTask {
        title: "Add a regression test".to_string(),
        detail: String::new(),
        after: String::new(),
    };
    let refusal = Fleet::add_task_by_person(Arc::clone(&fleet), job_id, add)
        .await
        .expect_err("no plan is recorded yet");
    assert_eq!(code(&refusal), "fleet.no_plan");
}

#[tokio::test]
async fn dropping_from_a_job_with_no_plan_is_refused_by_name() {
    let home = TempDir::new();
    let fleet = a_fixture(&home);
    let job_id = a_job(&fleet).await;

    let drop = DropTask {
        task: "T1".to_string(),
        reason: "not needed".to_string(),
    };
    let refusal = Fleet::drop_task_by_person(Arc::clone(&fleet), job_id, drop)
        .await
        .expect_err("no plan is recorded yet");
    assert_eq!(code(&refusal), "fleet.no_plan");
}

#[tokio::test]
async fn dropping_a_task_the_plan_does_not_hold_is_refused_by_name() {
    let home = TempDir::new();
    let fleet = a_fixture(&home);
    let job_id = a_job(&fleet).await;
    a_plan_of_four(&fleet, &job_id).await;

    let drop = DropTask {
        task: "T9".to_string(),
        reason: "not needed".to_string(),
    };
    let refusal = Fleet::drop_task_by_person(Arc::clone(&fleet), job_id, drop)
        .await
        .expect_err("the plan holds no T9");
    assert_eq!(code(&refusal), "fleet.no_such_task");
}

#[tokio::test]
async fn adding_after_a_task_the_plan_does_not_hold_is_refused_by_name() {
    let home = TempDir::new();
    let fleet = a_fixture(&home);
    let job_id = a_job(&fleet).await;
    a_plan_of_four(&fleet, &job_id).await;

    let add = AddTask {
        title: "Add a regression test".to_string(),
        detail: String::new(),
        after: "T9".to_string(),
    };
    let refusal = Fleet::add_task_by_person(Arc::clone(&fleet), job_id, add)
        .await
        .expect_err("the plan holds no T9 to add after");
    assert_eq!(code(&refusal), "fleet.no_such_task");
}

#[tokio::test]
async fn dropping_a_done_task_is_refused_by_name_and_a_dropped_one_is_too() {
    let home = TempDir::new();
    let fleet = a_fixture(&home);
    let job_id = a_job(&fleet).await;
    a_plan_of_four(&fleet, &job_id).await;

    // T1 done and T2 already dropped, by the step that follows the plan —
    // exactly as `implement`'s own `update_task` would leave them.
    let job = job_id.to_domain();
    {
        let mut store = fleet.store().lock().await;
        store
            .change_plan(
                &job,
                &PlanChange::Updated {
                    task: TaskId::read("T1").expect("an id"),
                    to: TaskUpdate::Done,
                },
                PlanHand::Step(&StepId::new("implement")),
                &Timestamp::from_rfc3339("2026-09-13T10:00:01.000Z"),
            )
            .expect("updated");
        store
            .change_plan(
                &job,
                &PlanChange::Updated {
                    task: TaskId::read("T2").expect("an id"),
                    to: TaskUpdate::Dropped(
                        core_model::DropReason::new("superseded").expect("a reason"),
                    ),
                },
                PlanHand::Step(&StepId::new("implement")),
                &Timestamp::from_rfc3339("2026-09-13T10:00:02.000Z"),
            )
            .expect("dropped");
    }

    let drop_done = DropTask {
        task: "T1".to_string(),
        reason: "not needed after all".to_string(),
    };
    let refusal = Fleet::drop_task_by_person(Arc::clone(&fleet), job_id.clone(), drop_done)
        .await
        .expect_err("T1 is already done");
    assert_eq!(code(&refusal), "fleet.task_already_settled");

    let drop_dropped = DropTask {
        task: "T2".to_string(),
        reason: "still not needed".to_string(),
    };
    let refusal = Fleet::drop_task_by_person(Arc::clone(&fleet), job_id, drop_dropped)
        .await
        .expect_err("T2 is already dropped");
    assert_eq!(code(&refusal), "fleet.task_already_settled");
}

#[tokio::test]
async fn a_blank_title_or_reason_is_refused() {
    let home = TempDir::new();
    let fleet = a_fixture(&home);
    let job_id = a_job(&fleet).await;
    a_plan_of_four(&fleet, &job_id).await;

    let blank_title = AddTask {
        title: "   ".to_string(),
        detail: String::new(),
        after: String::new(),
    };
    Fleet::add_task_by_person(Arc::clone(&fleet), job_id.clone(), blank_title)
        .await
        .expect_err("a blank title makes no task");

    let blank_reason = DropTask {
        task: "T1".to_string(),
        reason: "  ".to_string(),
    };
    Fleet::drop_task_by_person(Arc::clone(&fleet), job_id, blank_reason)
        .await
        .expect_err("a blank reason cannot drop anything");
}

/// A kept add and a kept drop each record `PlanAuthor::Person` — the whole of
/// what tells a person's change from a Drone's on the record — and each
/// publishes `job.plan_changed` with `actor` `human`.
#[tokio::test]
async fn a_kept_change_records_the_person_and_publishes_the_event() {
    let home = TempDir::new();
    let fleet = a_fixture(&home);
    let job_id = a_job(&fleet).await;
    a_plan_of_four(&fleet, &job_id).await;
    let mut watching: Subscription = fleet.events().subscribe();

    let add = AddTask {
        title: "Add a regression test".to_string(),
        detail: String::new(),
        after: String::new(),
    };
    let plan = Fleet::add_task_by_person(Arc::clone(&fleet), job_id.clone(), add)
        .await
        .expect("a plan with no rule against this add");
    assert_eq!(plan.tasks.len(), 5, "the fifth task");
    assert_eq!(plan.tasks[4].id, "T5");

    let job = job_id.to_domain();
    let history = fleet
        .store()
        .lock()
        .await
        .plan_history(&job)
        .expect("the plan's history");
    assert!(
        matches!(history.last(), Some(entry) if entry.by == PlanAuthor::Person),
        "the add is a person's, on the record: {history:?}"
    );

    let published = drained(&mut watching).await;
    assert_eq!(published.len(), 1, "one add, one event");
    assert_eq!(published[0].actor.domain(), core_model::Actor::Human);
    assert_eq!(published[0].job_id, job_id);
}

async fn drained(watching: &mut Subscription) -> Vec<ipc::JobPlanChanged> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) =
        tokio::time::timeout(Duration::from_millis(30), watching.next()).await
    {
        if let ipc::Event::JobPlanChanged(changed) = delivered.event {
            seen.push(changed);
        }
    }
    seen
}
