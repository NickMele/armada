//! Naming one Check, and what that costs. #1456.
//!
//! **The subject is what a name changes and what it must not.** It changes
//! which Checks run and whether the allowance is consulted. It does not change
//! the bar: the gate runs every Check whole at submission whatever was asked
//! here, which is what lets an ask be free in the first place.

use std::sync::Arc;

use ipc::mcp::ChecksAsk;
use testkit::{Gate, Sketch};

use crate::dry_run::NotRun;
use crate::tests::dry_run::{a_fleet_checking, started, Held};
use crate::tests::tmp::TempDir;
use crate::tests::tools::{asked_by_the_one, checked_by_the_one};
use config::ResolvedWorkflow;

/// One fast Check and one slow one, so a run that did not filter is a run that
/// took five seconds rather than a run with an extra row.
fn fast_and_slow() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[
            Gate::Check {
                name: "fast",
                run: "/usr/bin/true",
                expect_exit_code: 0,
                when: &[],
            },
            Gate::Check {
                name: "slow",
                run: "/bin/sleep 5",
                expect_exit_code: 0,
                when: &[],
            },
        ],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

#[tokio::test]
async fn naming_one_check_runs_that_one_and_no_other() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        fast_and_slow(),
        Arc::new(Held::started()),
        3,
    ));
    started(&fleet, &home).await;

    let report = asked_by_the_one(&fleet, ChecksAsk::just("fast"))
        .await
        .expect("a report");
    let named: Vec<&str> = report.ran.iter().map(|ran| ran.name.as_str()).collect();
    assert_eq!(
        named,
        ["fast"],
        "a named ask ran something the Drone did not name"
    );
}

#[tokio::test]
async fn a_name_the_part_does_not_gate_on_is_refused_and_told_the_names_it_has() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        fast_and_slow(),
        Arc::new(Held::started()),
        3,
    ));
    started(&fleet, &home).await;

    let refused = asked_by_the_one(&fleet, ChecksAsk::just("typecheck"))
        .await
        .expect_err("a name it does not have");
    let NotRun::NoSuchCheck { check, declared } = &refused else {
        panic!("{refused:?}");
    };
    assert_eq!(check, "typecheck");
    assert!(
        declared.iter().any(|name| name == "fast") && declared.iter().any(|name| name == "slow"),
        "the refusal named {declared:?}, which a Drone cannot correct against"
    );
}

/// **The whole point of #1456.** A Drone working one suite asks about it as
/// often as it is useful, and still holds every whole-gate rehearsal it
/// started with — which is what it would not spend before, and why it ran the
/// commands by hand instead.
#[tokio::test]
async fn naming_a_check_never_spends_the_allowance() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        fast_and_slow(),
        Arc::new(Held::started()),
        1,
    ));
    started(&fleet, &home).await;

    for asked in 0..4 {
        asked_by_the_one(&fleet, ChecksAsk::just("fast"))
            .await
            .unwrap_or_else(|why| panic!("named ask {asked} was refused: {why:?}"));
    }

    checked_by_the_one(&fleet)
        .await
        .expect("the one whole-gate run this step was allowed");
    let refused = checked_by_the_one(&fleet)
        .await
        .expect_err("the second whole-gate run is refused");
    assert!(
        matches!(refused, NotRun::Spent { allowed: 1 }),
        "{refused:?}"
    );
}
