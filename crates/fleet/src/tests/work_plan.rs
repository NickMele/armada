//! A Job's plan, served: whole on `get_job` and counted on the Board's row, off
//! the one store reading. Absent-where-none is `ipc`'s own test.

use core_model::{Approach, NewTask, PlanChange, StepId, TaskId, TaskUpdate, Timestamp};
use ipc::{JobDetail, JobList, RunId};
use testkit::FakeWorkProduct;

use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet, a_proposal, worktree_directory};
use crate::tests::detail::get;
use crate::tests::tmp::TempDir;

#[tokio::test]
async fn a_recorded_plan_is_served_whole_on_the_detail_and_counted_on_the_row() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("serve the plan back"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &job_id).await.expect("released to run");

    let step = StepId::new("implement");
    let at = Timestamp::from_rfc3339("2026-09-13T10:00:00.000Z");
    let mut store = fleet.store().lock().await;
    store
        .change_plan(
            &job_id,
            &PlanChange::Recorded {
                approach: Approach::new("Bound the reader").expect("an approach"),
                tasks: vec![
                    NewTask::new("Stop at the end", "", &[], "").expect("a title"),
                    NewTask::new("Cover it", "", &[], "").expect("a title"),
                ],
            },
            store::PlanHand::Step(&step),
            &at,
        )
        .expect("recorded");
    store
        .change_plan(
            &job_id,
            &PlanChange::Updated {
                task: TaskId::read("T1").expect("an id"),
                to: TaskUpdate::Done,
                shown: None,
            },
            store::PlanHand::Step(&step),
            &at,
        )
        .expect("updated");
    drop(store);

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (_, body) = get(&app, &format!("/jobs/{}", job_id.as_str())).await;
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");
    let plan = detail.work_plan.expect("the plan is on the detail");
    assert_eq!(plan.approach, "Bound the reader");
    assert_eq!(
        plan.tasks
            .iter()
            .map(|task| (task.id.as_str(), task.state.as_wire()))
            .collect::<Vec<_>>(),
        [("T1", "done"), ("T2", "open")]
    );
    let counted = detail.job.tasks.expect("the nested row counts them too");
    assert_eq!((counted.done, counted.open), (1, 1));

    let (_, body) = get(&app, "/jobs").await;
    let list: JobList = ipc::decode("the Board", &body).expect("a JobList");
    let row = list
        .jobs
        .iter()
        .find(|row| row.id.as_str() == job_id.as_str())
        .expect("the Job is a row");
    assert_eq!(
        row.tasks,
        Some(counted),
        "the Board reads what the detail reads"
    );
}
