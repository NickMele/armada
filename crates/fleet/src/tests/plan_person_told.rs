//! A person's add and drop, delivered — or not. `#897`.
//!
//! **The transcript file is the record of what Fleet said**, not `heard()`:
//! that answers what the Drone said, and `Occasion` only ever reaches the
//! file, through `Working::instructed`. So these read the file back, the way
//! `crate::tests::transcript` already does for the Drone's own lines.

use std::sync::Arc;
use std::time::Duration;

use core_model::{Approach, JobStatus, NewTask, PlanChange, StepId, Timestamp};
use ipc::{AddTask, DropTask};
use store::PlanHand;
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::transcript::transcript_of;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

fn a_fixture(home: &TempDir) -> Arc<Fixture> {
    Arc::new(Fleet::assembled(fittings(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
    )))
}

async fn a_plan_of_four(fleet: &Arc<Fixture>, job: &core_model::JobId) {
    let mut store = fleet.store().lock().await;
    store
        .change_plan(
            job,
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

/// Wait until the Drone has answered its opening brief, so nothing from the
/// spawn itself is still arriving when a test takes its baseline.
async fn settled(fleet: &Fixture) {
    let mut steady = 0;
    let mut last = usize::MAX;
    for _ in 0..400 {
        let now = {
            let held = fleet.the_only_slot().await;
            let slot = held.lock().await;
            slot.as_ref().map(|at| at.heard().len()).unwrap_or_default()
        };
        steady = if now == last { steady + 1 } else { 0 };
        if steady == 10 && now > 0 {
            return;
        }
        last = now;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the Drone never stopped talking");
}

/// The transcript file's whole text, once it holds at least `how_many` rows.
async fn rows_once_written(
    records_root: &str,
    handle: &str,
    drone: &core_model::DroneId,
    how_many: usize,
) -> String {
    for _ in 0..400 {
        let path = transcript_of(records_root, handle, drone);
        if let Ok(written) = std::fs::read_to_string(&path) {
            if written.lines().count() >= how_many {
                return written;
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the transcript never reached {how_many} rows");
}

#[tokio::test]
async fn a_working_drone_mid_step_is_told_a_persons_add() {
    let home = TempDir::new();
    let fleet = a_fixture(&home);
    let job = fleet
        .propose(a_proposal("cover the writer's bound"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &job_id).await.expect("released to run");
    settled(&fleet).await;
    a_plan_of_four(&fleet, &job_id).await;

    let record = fleet.load(&job_id).await.expect("the Job");
    let handle = record.handle();
    let drone = record.assigned_drone().expect("a live Drone").clone();
    let before = rows_once_written(&home.path().to_string_lossy(), &handle, &drone, 1).await;
    let before_rows = before.lines().count();

    let add = AddTask {
        title: "Add a regression test".to_string(),
        detail: String::new(),
        after: String::new(),
    };
    Fleet::add_task_by_person(Arc::clone(&fleet), ipc::JobId::from(&job_id), add)
        .await
        .expect("a plan with no rule against this add");

    let after = rows_once_written(
        &home.path().to_string_lossy(),
        &handle,
        &drone,
        before_rows + 1,
    )
    .await;
    let new: String = after
        .lines()
        .skip(before_rows)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        new.contains("\"occasion\":\"plan\""),
        "its own Occasion, not a redirect: {new}"
    );
    assert!(new.contains("THE PLAN CHANGED"), "{new}");
    assert!(new.contains("T5"), "{new}");
    assert!(new.contains("Add a regression test"), "{new}");
}

#[tokio::test]
async fn a_working_drone_mid_step_is_told_a_persons_drop() {
    let home = TempDir::new();
    let fleet = a_fixture(&home);
    let job = fleet
        .propose(a_proposal("cover the writer's bound"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &job_id).await.expect("released to run");
    settled(&fleet).await;
    a_plan_of_four(&fleet, &job_id).await;

    let record = fleet.load(&job_id).await.expect("the Job");
    let handle = record.handle();
    let drone = record.assigned_drone().expect("a live Drone").clone();
    let before = rows_once_written(&home.path().to_string_lossy(), &handle, &drone, 1).await;
    let before_rows = before.lines().count();

    let drop = DropTask {
        task: "T3".to_string(),
        reason: "the writer's bound was never inclusive".to_string(),
    };
    Fleet::drop_task_by_person(Arc::clone(&fleet), ipc::JobId::from(&job_id), drop)
        .await
        .expect("a plan with no rule against this drop");

    let after = rows_once_written(
        &home.path().to_string_lossy(),
        &handle,
        &drone,
        before_rows + 1,
    )
    .await;
    let new: String = after
        .lines()
        .skip(before_rows)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        new.contains("\"occasion\":\"plan\""),
        "its own Occasion, not a redirect: {new}"
    );
    assert!(new.contains("THE PLAN CHANGED"), "{new}");
    assert!(new.contains("T3"), "{new}");
    assert!(
        new.contains("the writer's bound was never inclusive"),
        "{new}"
    );
}

/// With no live session — here, a Job never dispatched — nothing is sent and
/// nothing is respawned: the change still keeps, and the Job's status is
/// exactly what it was.
#[tokio::test]
async fn with_no_live_session_nothing_is_sent_and_nothing_respawns() {
    let home = TempDir::new();
    let fleet = a_fixture(&home);
    let job = fleet
        .propose(a_proposal("cover the writer's bound"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    a_plan_of_four(&fleet, &job_id).await;
    assert_eq!(
        fleet.load(&job_id).await.expect("the Job").status(),
        JobStatus::AwaitingApproval,
        "never dispatched, so there is no session to speak into"
    );

    let add = AddTask {
        title: "Add a regression test".to_string(),
        detail: String::new(),
        after: String::new(),
    };
    let plan = Fleet::add_task_by_person(Arc::clone(&fleet), ipc::JobId::from(&job_id), add)
        .await
        .expect("the plan itself has no rule against this add");
    assert_eq!(plan.tasks.len(), 5, "the change keeps regardless");

    assert_eq!(
        fleet.load(&job_id).await.expect("the Job").status(),
        JobStatus::AwaitingApproval,
        "the Job is exactly where it was: nothing respawned to deliver the change"
    );
}
