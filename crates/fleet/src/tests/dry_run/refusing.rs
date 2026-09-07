//! The calls that get no report, and what each refusal says to do instead.
//!
//! Two are correctness — a second run in a worktree already building, and a run
//! alongside the gate that is about to decide the same question. One is cost,
//! which has to be a count because the clock suspension removed the pressure
//! that would otherwise have limited it. The last two are a step with nothing
//! to run and a call with nothing working, refused rather than answered with an
//! empty report: a report with no rows reads as a run that found nothing wrong,
//! which is the vacuous pass one layer out.

use std::sync::Arc;

use testkit::Sketch;

use crate::dry_run::NotRun;
use crate::tests::dry_run::{
    a_fleet_checking, one_step, router, started, submit, wait_until_checking, Held,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::checked_by_the_one;

/// **The correctness bound.** Two runs at once are two builds in one worktree,
/// contending for one target directory, and neither answer would be about the
/// work — so the second is refused rather than queued.
#[tokio::test]
async fn a_second_run_while_one_is_going_is_refused() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/bin/sleep 2"),
        Arc::new(Held::started()),
        3,
    ));
    started(&fleet, &home).await;

    let running = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        async move { checked_by_the_one(&fleet).await }
    });
    wait_until_checking(&fleet).await;

    let refused = checked_by_the_one(&fleet).await;
    assert!(
        matches!(refused, Err(NotRun::AlreadyRunning)),
        "{refused:?}"
    );
    running.await.expect("the run finished").expect("a report");
}

/// The gate runs these same Checks in this same worktree, so a Drone that has
/// already submitted is told to wait rather than given a second run alongside
/// the one that decides.
#[tokio::test]
async fn a_drone_that_has_already_submitted_is_told_to_wait() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/usr/bin/true"),
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;

    submit(&app).await;
    assert_eq!(fleet.evidence_waiting(), 1, "the gate has not run yet");
    let refused = checked_by_the_one(&fleet)
        .await
        .expect_err("the gate is about to");
    assert!(matches!(refused, NotRun::AlreadySubmitted), "{refused:?}");
    assert!(
        refused.to_string().contains("later turn"),
        "and is told where the answer comes from: {refused}"
    );
}

/// **The cost bound.** The clock suspension removes the pressure that
/// would otherwise have limited this, so a count does — and the refusal says
/// what to do instead, because a Drone told only "no" asks again.
#[tokio::test]
async fn a_step_that_has_spent_its_allowance_is_refused_and_told_why() {
    let home = TempDir::new();
    let fleet = a_fleet_checking(
        &home,
        one_step("/usr/bin/true"),
        Arc::new(Held::started()),
        1,
    );
    started(&fleet, &home).await;

    checked_by_the_one(&fleet).await.expect("the first run");
    let refused = checked_by_the_one(&fleet)
        .await
        .expect_err("the second is refused");
    assert!(
        matches!(refused, NotRun::Spent { allowed: 1 }),
        "{refused:?}"
    );
    let said = refused.to_string();
    assert!(
        said.contains("submit"),
        "a Drone out of runs is told what to do instead: {said}"
    );
}

/// A step declaring no mechanical Check is refused rather than answered with an
/// empty report — a report with no rows reads as a run that found nothing
/// wrong, which is the vacuous pass one layer out.
#[tokio::test]
async fn a_step_with_no_checks_is_refused_rather_than_answered_with_nothing() {
    let home = TempDir::new();
    let unchecked = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let fleet = a_fleet_checking(&home, unchecked, Arc::new(Held::started()), 3);
    started(&fleet, &home).await;

    let refused = checked_by_the_one(&fleet)
        .await
        .expect_err("nothing to run");
    assert!(
        matches!(refused, NotRun::StepHasNoChecks { .. }),
        "{refused:?}"
    );
}

/// A call arriving when no Job is being worked is a tool error the Drone can
/// read, never a queued run against whatever comes next.
#[tokio::test]
async fn a_call_with_nothing_working_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet_checking(
        &home,
        one_step("/usr/bin/true"),
        Arc::new(Held::started()),
        3,
    );

    let refused = checked_by_the_one(&fleet)
        .await
        .expect_err("nothing is working");
    assert!(matches!(refused, NotRun::NothingIsWorking), "{refused:?}");
}
