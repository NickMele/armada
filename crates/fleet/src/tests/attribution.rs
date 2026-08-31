//! What a finished Job can say about the plan it promised.
//!
//! # The three answers, and the one that used to be missing
//!
//! | What the Job's steps did | What a row says |
//! |---|---|
//! | Named paths | `planned_by` names the steps that promised it, or is empty where none did |
//! | Declared they would touch nothing | `planned_by` is empty on every row — everything is outside a promise of nothing |
//! | Never declared | `planned_by` is **absent**, and no row is marked |
//!
//! The third is why this is not a bool. A record with `outside_plan: false` on
//! every row would say a Job nobody scoped went exactly where it said it would,
//! which is the reading `#127` refused to make possible by carrying no field at
//! all. What replaced the missing field is a name rather than a verdict.
//!
//! # Nothing here opens a worktree
//!
//! The footprint was read at the terminal transition and the plans were written
//! as each step declared. Both are rows by the time a person asks, so the
//! comparison survives `armada clean` taking the directory back.

use std::sync::Arc;

use adapter_traits::Change;
use api::Daemon;
use ipc::mcp::DeclareScope;
use testkit::{FakeWorkProduct, Scoped};

use crate::tests::footprint::{a_fleet_reading, started, three_kinds, Held};
use crate::tests::tmp::TempDir;
use crate::tests::tools::declared_by_the_one;

/// A step that declares its plan at step start and is checked against it.
const DECLARING: Scoped<'static> = Scoped {
    diff_check: true,
    at_step_start: true,
    exclude: &[],
    references: &[],
};

/// Each file's path against the steps that promised it, in wire order.
fn attributed(footprint: &ipc::JobFootprint) -> Vec<(&str, Option<Vec<&str>>)> {
    footprint
        .files
        .iter()
        .map(|file| {
            (
                file.path.as_str(),
                file.planned_by.as_ref().map(|steps| {
                    steps
                        .iter()
                        .map(|step| step.as_str())
                        .collect::<Vec<&str>>()
                }),
            )
        })
        .collect()
}

/// **The capability, and `#177`'s definition of done.** The Drone promised one
/// file and touched three; the record says which two it never promised, long
/// after the slot that held the promise is gone.
#[tokio::test]
async fn a_finished_job_says_which_files_were_outside_what_a_step_promised() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), Some(DECLARING));
    let job = started(&fleet, &home).await;
    declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["src/parse.rs".to_string()],
        },
    )
    .await
    .expect("a declaration");

    fleet.kill_job(&job).await.expect("a terminal status");

    let detail = fleet
        .get_job(ipc::JobId::from(&job))
        .await
        .expect("the Job is served");
    let footprint = detail.footprint.expect("a footprint was recorded");
    assert_eq!(
        attributed(&footprint),
        vec![
            ("src/parse.rs", Some(vec!["implement"])),
            ("src/tokens.rs", Some(Vec::new())),
            ("src/legacy.rs", Some(Vec::new())),
        ],
        "the promised path names its step, and the two that were not are outside every plan"
    );
    assert_eq!(
        footprint
            .plans
            .iter()
            .map(|plan| (plan.step_id.as_str(), plan.attempt, plan.paths.clone()))
            .collect::<Vec<(&str, u32, Vec<String>)>>(),
        vec![("implement", 1, vec!["src/parse.rs".to_string()])],
        "and the promise itself is served beside the record of what was done"
    );
}

/// **A Job nobody scoped is silent, not clean.** No step declared, so no path
/// was measured — and every `planned_by` is absent rather than an empty list a
/// surface would draw as drift.
#[tokio::test]
async fn a_job_whose_steps_never_declared_marks_nothing_at_all() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), None);
    let job = started(&fleet, &home).await;

    fleet.kill_job(&job).await.expect("a terminal status");

    let detail = fleet
        .get_job(ipc::JobId::from(&job))
        .await
        .expect("the Job is served");
    let footprint = detail.footprint.expect("a footprint was recorded");
    assert!(
        footprint.plans.is_empty(),
        "nothing was declared, so there is no promise to serve"
    );
    assert!(
        footprint.files.iter().all(|file| file.planned_by.is_none()),
        "absent on every row: no measurement was made, which is not a measurement of none"
    );
}

/// **A promise to touch nothing is a promise.** Every path is outside it, which
/// is the one case an absent mark and an empty one would otherwise be confused
/// over.
#[tokio::test]
async fn a_step_that_promised_to_touch_nothing_puts_every_path_outside_it() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), Some(DECLARING));
    let job = started(&fleet, &home).await;
    declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: Vec::new(),
        },
    )
    .await
    .expect("a declaration of nothing");

    fleet.kill_job(&job).await.expect("a terminal status");

    let footprint = fleet
        .get_job(ipc::JobId::from(&job))
        .await
        .expect("the Job is served")
        .footprint
        .expect("a footprint was recorded");
    assert_eq!(footprint.plans.len(), 1, "a promise was made");
    assert!(
        footprint.plans[0].paths.is_empty(),
        "and what it promised was nothing"
    );
    assert!(
        footprint
            .files
            .iter()
            .all(|file| file.planned_by.as_deref() == Some(&[])),
        "so every path is outside it, and none is unmeasured"
    );
}

/// **The record keeps the corrected plan, not the outgrown one.** Calling the
/// tool again is the sanctioned answer to drift, and a Job that took it must
/// not be read against the promise it was told to replace.
#[tokio::test]
async fn a_step_that_redeclared_is_read_against_what_it_declared_last() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), Some(DECLARING));
    let job = started(&fleet, &home).await;
    for paths in [vec!["src/parse.rs"], vec!["src/tokens.rs", "src/legacy.rs"]] {
        declared_by_the_one(
            &fleet,
            &DeclareScope {
                context_paths: paths.iter().map(|path| path.to_string()).collect(),
            },
        )
        .await
        .expect("a declaration");
    }

    fleet.kill_job(&job).await.expect("a terminal status");

    let footprint = fleet
        .get_job(ipc::JobId::from(&job))
        .await
        .expect("the Job is served")
        .footprint
        .expect("a footprint was recorded");
    assert_eq!(footprint.plans.len(), 1, "one run declared once, finally");
    assert_eq!(
        attributed(&footprint),
        vec![
            ("src/parse.rs", Some(Vec::new())),
            ("src/tokens.rs", Some(vec!["implement"])),
            ("src/legacy.rs", Some(vec!["implement"])),
        ],
        "the first promise is gone and the path it covered is now outside"
    );
}

/// A worktree that held no change is still measured against the promise, and
/// says nothing drifted because there was nothing to drift.
#[tokio::test]
async fn a_drone_that_changed_nothing_drifts_from_nothing() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(
        &home,
        FakeWorkProduct::changing(&[("src/parse.rs", Change::Modified)]),
        Arc::clone(&clock),
        Some(DECLARING),
    );
    let job = started(&fleet, &home).await;
    declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["src".to_string()],
        },
    )
    .await
    .expect("a declaration");

    fleet.kill_job(&job).await.expect("a terminal status");

    let footprint = fleet
        .get_job(ipc::JobId::from(&job))
        .await
        .expect("the Job is served")
        .footprint
        .expect("a footprint was recorded");
    assert_eq!(
        attributed(&footprint),
        vec![("src/parse.rs", Some(vec!["implement"]))],
        "a declared directory covers what is beneath it"
    );
}
