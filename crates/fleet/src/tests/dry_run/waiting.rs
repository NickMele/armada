//! A Drone that ends its turn on `run_checks` is waiting for the report, not
//! finished. #1020: the call answers at once, so ending the turn is the natural
//! next move, and reaping the Drone there would kill the run it is waiting on.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::DroneEvent;
use core_model::JobStatus;
use testkit::FakeHarness;

use crate::tests::dry_run::{
    a_fleet_driven, called, checks_in, one_step, started, the_one_drone, transcript, Fixture, Held,
    A_CHECK_RUN_HAS_LONG_ENOUGH,
};
use crate::tests::tmp::TempDir;

/// A Drone that makes a call, ends its run once `go` exists, and stays.
fn a_drone_that_ends_when(go: &Path) -> FakeHarness {
    let script = format!(
        "echo BUSY; while [ ! -e '{}' ]; do sleep 0.05; done; echo ENDED; sleep 30",
        go.display()
    );
    FakeHarness::running("/bin/sh", &["-c", script.as_str()])
        .reading("BUSY", called())
        .reading(
            "ENDED",
            vec![DroneEvent::Ended {
                turns: 3,
                cost_micros: 1_000,
                refusals: 0,
            }],
        )
}

async fn slot_reads(fleet: &Fixture, question: impl Fn(&crate::working::Working) -> bool) -> bool {
    fleet
        .the_only_slot()
        .await
        .lock()
        .await
        .as_ref()
        .is_some_and(question)
}

#[tokio::test]
async fn a_drone_at_rest_on_its_own_checks_is_kept_and_told_the_report() {
    let home = TempDir::new();
    let go = home.path().join("end-the-run");
    let fleet = Arc::new(a_fleet_driven(
        &home,
        one_step("/bin/sleep 4"),
        Arc::new(Held::started()),
        3,
        &["src/parse.rs"],
        a_drone_that_ends_when(&go),
    ));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");

    let underway = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    std::fs::write(&go, "").expect("the Drone is told to end its run");
    let mut resting = false;
    for _ in 0..600 {
        if slot_reads(&fleet, |at_work| at_work.at_rest()).await {
            resting = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(resting, "the Drone's run never ended");
    assert!(
        slot_reads(&fleet, |at_work| at_work.is_checking()).await,
        "the Checks finished before the vigil could see the Drone at rest"
    );

    fleet.turn().await.expect("a vigil turn");
    assert_eq!(
        fleet.load(&job).await.expect("the Job").status(),
        JobStatus::Running,
        "a Drone waiting on its Checks was taken for finished"
    );
    assert!(
        slot_reads(&fleet, |_| true).await,
        "a Drone waiting on its Checks was stood down"
    );

    let finished = underway.finished().await;
    assert!(
        matches!(finished, Some(Ok(_))),
        "the run was cut short: {finished:?}"
    );
    let said = transcript(&fleet, &home, &job, &drone)
        .await
        .until(A_CHECK_RUN_HAS_LONG_ENOUGH, |said| {
            !checks_in(said).is_empty()
        })
        .await
        .unwrap_or_else(|stood| stood);
    let told = checks_in(&said);
    assert_eq!(told.len(), 1, "the Drone was not sent its report: {told:?}");
    assert!(
        slot_reads(&fleet, |at_work| !at_work.at_rest()).await,
        "the report did not wake the Drone"
    );
}
