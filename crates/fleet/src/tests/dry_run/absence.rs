//! What a dry run did not decide, and did not cost.
//!
//! # The Checks are real commands and one of them changes its mind
//!
//! `/bin/test ! -e <marker>` passes while a file is not there and fails once it
//! is. That is the only way to write the case that matters: a dry run in which
//! everything passed, followed by a gate in which the same Check does not — and
//! a step that ends `completed_failed` regardless of what the Drone was told a
//! moment earlier.
//!
//! The second case is the other half of the same absence. A Drone waiting on a
//! Check that Fleet is running is spending none of its own budget, so the clock
//! is pushed while the run is in flight and both tripwires are asked what they
//! saw.

use std::sync::Arc;
use std::time::Duration;

use core_model::{JobStatus, StepId};

use crate::tests::dry_run::{
    a_fleet_checking, ask, one_step, router, started, submit, wait_until_checking, Held,
    QUIET_AFTER, WALL_CLOCK,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::checked_by_the_one;

/// The Check command that passes until `marker` exists.
///
/// `/bin/test` rather than a shell, because `checks_runner` splits a `run`
/// string on whitespace and executes the program directly — which is the whole
/// reason a Manifest Check cannot pipe.
fn passes_until(marker: &std::path::Path) -> String {
    format!("/bin/test ! -e {}", marker.display())
}

/// **The case the whole design turns on.** Every Check passes in the dry run,
/// the world then changes underneath it, and the gate reaches its own verdict
/// on its own run — so the pass the Drone was shown satisfied nothing.
#[tokio::test]
async fn a_dry_run_that_passed_does_not_satisfy_the_gate() {
    let home = TempDir::new();
    let marker = home.path().join("broken");
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step(&passes_until(&marker)),
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    let job = started(&fleet, &home).await;

    let said = ask(&app).await;
    assert!(
        said.text.contains("PASSED") && !said.text.contains("FAILED"),
        "every check passed in the dry run: {}",
        said.text
    );
    let record = fleet.load(&job).await.expect("the Job");
    assert_eq!(
        record.status(),
        JobStatus::Running,
        "a dry run moved the Job"
    );
    assert_eq!(
        record
            .step(&StepId::new("implement"))
            .expect("the step")
            .state(),
        core_model::StepState::Running,
        "a dry run moved the step"
    );
    assert!(
        fleet
            .store()
            .lock()
            .await
            .step_checks(&job)
            .expect("the rows read")
            .is_empty(),
        "a dry run wrote a Check row, which is the record claiming a run that \
         decided something"
    );

    // The same Check, now failing. Nothing the Drone did caused this and that
    // is the point: what the gate rules on is its own run and never the one
    // the Drone was shown.
    std::fs::write(&marker, "").expect("the marker writes");
    submit(&app).await;
    fleet.turn().await.expect("the gate runs");
    assert_eq!(
        fleet.load(&job).await.expect("the Job").status(),
        JobStatus::AwaitingRepair,
        "the gate re-ran the checks and reached its own verdict"
    );
}

/// **A Drone waiting on a Check Fleet is running is not silent and is not
/// thrashing.** The clock is pushed a thousand times past the silence threshold
/// while the run is in flight, and the vigil says nothing — then the step's own
/// wall clock is read afterwards and the time is not on it.
///
/// Without the suspension this is the tripwire firing on the honest case, which
/// is exactly what offering a Drone a `cargo build` would otherwise create.
#[tokio::test]
async fn the_clocks_do_not_count_the_time_a_check_takes() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/bin/sleep 2"),
        Arc::clone(&clock),
        3,
    ));
    let job = started(&fleet, &home).await;

    let running = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        async move { checked_by_the_one(&fleet).await }
    });
    wait_until_checking(&fleet).await;

    // Far past the silence threshold, the pokes and the wall clock together.
    clock.on(QUIET_AFTER.as_secs() * 1_000);
    for _ in 0..8 {
        let turned = fleet.turn().await.expect("a turn");
        assert!(
            turned.quiet().is_none(),
            "the vigil counted a Check Fleet was running against the Drone: {:?}",
            turned.quiet()
        );
        assert!(
            turned.wandering().is_none(),
            "the thrashing chain fired while Fleet was answering the Drone: {:?}",
            turned.wandering()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        fleet.load(&job).await.expect("the Job").status(),
        JobStatus::Running,
        "the Job escalated while Fleet was running its own checks"
    );

    let report = running.await.expect("the run finished").expect("a report");
    assert_eq!(report.ran.len(), 2);

    // And the time is given back rather than merely not read: a suspension that
    // only skipped the check would leave the wall clock over its ceiling for
    // the rest of the step.
    let now = fleet.now();
    let running_for = fleet
        .the_only_slot()
        .await
        .lock()
        .await
        .as_ref()
        .expect("a Drone is working")
        .running_for(&now);
    assert!(
        running_for < WALL_CLOCK,
        "the step's wall clock kept the {} seconds Fleet spent on its own \
         checks: {running_for:?}",
        QUIET_AFTER.as_secs() * 1_000
    );
}
