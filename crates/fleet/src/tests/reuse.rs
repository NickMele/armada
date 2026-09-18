//! The gate reusing a passing dry run instead of running a Check again.
//! `#1014`.
//!
//! Every case shares one `FakeWorkProduct`: `footprint()` on a fake built with
//! `moving: false` (`FakeWorkProduct::changed`'s non-moving sibling is not
//! exposed, so [`still`] builds one directly) answers the same reading twice
//! running, and a different one once something is written — which is exactly
//! the property `crate::reuse` is asking the worktree about.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Change, Footprint, WorkProduct};
use core_model::{Attempt, CheckOutcome, ResolvedCheck, Spent, StepCheck, Timestamp};
use ipc::JobDetail;
use testkit::{FakeWorkProduct, Gate, Sketch};
use verification::{Lifted, Request};

use crate::at_step::AtStep;
use crate::gate::{rule_on, CheckBudget, Ruling};
use crate::policy::Policies;
use crate::reuse::KeptDryRun;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::detail::get;
use crate::tests::gate::{diff_evidence, judging, worktree};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tools::submitted_by_the_one;

/// A worktree that answers the same footprint until `wrote` runs.
fn still() -> FakeWorkProduct {
    FakeWorkProduct::untouched()
}

fn row(name: &str, outcome: CheckOutcome) -> StepCheck {
    StepCheck {
        name: name.to_string(),
        outcome,
        expected: None,
        produced: None,
        output_path: None,
        reused_from_dry_run: None,
    }
}

/// A dry run that already answered `checks`, read against `work`'s worktree
/// as it stands right now.
fn kept(work: &FakeWorkProduct, attempt: Attempt, checks: Vec<StepCheck>) -> KeptDryRun {
    let footprint = work.footprint(&worktree()).expect("a fake worktree reads");
    let narrowed_to = vec![None; checks.len()];
    KeptDryRun::of(
        attempt,
        Timestamp::from_rfc3339("2026-09-13T09:00:00Z"),
        footprint,
        checks,
        &narrowed_to,
    )
}

async fn ruled(gates: &[Gate<'_>], work: &FakeWorkProduct, dry_run: Option<&KeptDryRun>) -> Ruling {
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates,
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree)
        .expect("a first step")
        .on_attempt(Attempt::FIRST, Spent::FIRST);
    rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        crate::gate::Began::Unseen,
        &[],
        work,
        CheckBudget::of(Duration::from_secs(5)),
        &crate::places::Room::ignoring_the_machine(crate::places::ChecksAtOnce::of(4)),
        &judging(),
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
        &BTreeMap::new(),
        &[],
        core_model::WhenRefused::default(),
        &[],
        None,
        dry_run,
    )
    .await
}

fn checks() -> Vec<Gate<'static>> {
    vec![Gate::Check {
        name: "build",
        run: "/usr/bin/true",
        expect_exit_code: 0,
        when: &[],
    }]
}

#[tokio::test]
async fn a_passing_dry_run_over_an_unchanged_worktree_is_reused_and_nothing_runs() {
    let work = still();
    let dry_run = kept(
        &work,
        Attempt::FIRST,
        vec![row("build", CheckOutcome::Passed)],
    );
    let ruling = ruled(&checks(), &work, Some(&dry_run)).await;
    let build = ruling
        .checks()
        .iter()
        .find(|check| check.name == "build")
        .expect("build's row");
    assert_eq!(build.outcome, CheckOutcome::Passed);
    assert!(
        build.reused_from_dry_run.is_some(),
        "a Check the dry run already passed, over a worktree that has not \
         moved, is answered from it rather than run again"
    );
}

#[tokio::test]
async fn an_edit_after_the_dry_run_means_every_check_runs_fresh() {
    let work = still();
    let dry_run = kept(
        &work,
        Attempt::FIRST,
        vec![row("build", CheckOutcome::Passed)],
    );
    // The worktree moves between the dry run and the gate.
    work.wrote(&[("src/lib.rs", adapter_traits::Change::Modified)]);
    let ruling = ruled(&checks(), &work, Some(&dry_run)).await;
    let build = ruling
        .checks()
        .iter()
        .find(|check| check.name == "build")
        .expect("build's row");
    assert_eq!(
        build.reused_from_dry_run, None,
        "the worktree the dry run measured is not the worktree the gate is \
         looking at, so nothing from it may be trusted"
    );
}

#[tokio::test]
async fn a_check_that_failed_in_the_dry_run_is_run_again() {
    let work = still();
    // `KeptDryRun::of` already drops a failed Check; this pins that a step
    // still reaches it at the gate rather than reading it as answered.
    let dry_run = kept(
        &work,
        Attempt::FIRST,
        vec![row("build", CheckOutcome::Failed)],
    );
    let ruling = ruled(&checks(), &work, Some(&dry_run)).await;
    let build = ruling
        .checks()
        .iter()
        .find(|check| check.name == "build")
        .expect("build's row");
    assert_eq!(build.reused_from_dry_run, None);
    // `/usr/bin/true` is what actually ran, so the fresh answer is a pass —
    // never the dry run's stale failure.
    assert_eq!(build.outcome, CheckOutcome::Passed);
}

#[tokio::test]
async fn a_dry_run_from_a_different_attempt_is_never_reused() {
    let work = still();
    let dry_run = kept(
        &work,
        Attempt::stored(2).expect("a second attempt"),
        vec![row("build", CheckOutcome::Passed)],
    );
    // `ruled` always asks on the first attempt.
    let ruling = ruled(&checks(), &work, Some(&dry_run)).await;
    let build = ruling
        .checks()
        .iter()
        .find(|check| check.name == "build")
        .expect("build's row");
    assert_eq!(
        build.reused_from_dry_run, None,
        "a dry run kept against a different attempt says nothing about this one"
    );
}

/// **What `#1014`'s review caught.** `checking::ran` used to pair `checks`
/// with a second, caller-built sequence by `Iterator::zip`, which stops at
/// the shorter one without a word — one hand-written test call passed an
/// empty one and ran zero of six declared Checks. `ran` now takes the dry
/// run itself and looks a Check's row up by name, inside the one loop that
/// already walks `checks`; there is no second sequence left for a caller to
/// get the length of wrong. This pins the row count directly, against a dry
/// run built to have nothing to say about either declared Check.
#[tokio::test]
async fn every_declared_check_gets_a_row_whatever_the_dry_run_names() {
    let declared = vec![
        ResolvedCheck::ManifestCheck {
            name: "build".to_string(),
            run: "/usr/bin/true".to_string(),
            expect_exit_code: 0,
            when: None,
            requires: Vec::new(),
            narrow: None,
            one_test: None,
            runs_at: core_model::RunsAt::Everywhere,
            places: std::num::NonZeroU32::MIN,
            width: None,
            runner: None,
        },
        ResolvedCheck::ManifestCheck {
            name: "test".to_string(),
            run: "/usr/bin/true".to_string(),
            expect_exit_code: 0,
            when: None,
            requires: Vec::new(),
            narrow: None,
            one_test: None,
            runs_at: core_model::RunsAt::Everywhere,
            places: std::num::NonZeroU32::MIN,
            width: None,
            runner: None,
        },
    ];
    let footprint = Footprint::nothing();
    let dry_run = KeptDryRun::of(
        Attempt::FIRST,
        Timestamp::from_rfc3339("2026-09-13T09:00:00Z"),
        footprint.clone(),
        vec![row(
            "a_check_neither_of_these_is_named",
            CheckOutcome::Passed,
        )],
        &[None],
    );
    let observed = crate::checking::ran(
        &declared,
        &[],
        false,
        false,
        std::path::Path::new("/"),
        Duration::from_secs(5),
        &crate::places::Room::ignoring_the_machine(crate::places::ChecksAtOnce::of(4)),
        &crate::underway::Announcing::nowhere(),
        &BTreeMap::new(),
        &[],
        None,
        &crate::checking::Stop::never(),
        Some(&dry_run),
        Attempt::FIRST,
        Some(&footprint),
    )
    .await;
    assert_eq!(
        observed.len(),
        declared.len(),
        "every declared Check gets a row, whatever the dry run does or does not name"
    );
}

type Fixture = crate::daemon::Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// One step, one Check that takes real time, so a test can act while it runs
/// — `crate::tests::dry_run::later`'s own shape, for the same reason.
fn a_slow_step() -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::Check {
            name: "suite",
            run: "/bin/sleep 2",
            expect_exit_code: 0,
            when: &[],
        }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// A Fleet dispatched onto one Job of [`a_slow_step`], with a worktree on disk
/// and a Drone in the slot.
async fn dispatched_onto(home: &crate::tests::tmp::TempDir) -> (Arc<Fixture>, core_model::JobId) {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::untouched(),
        testkit::FakeHarness::that_listens(),
    );
    fittings.starting().workflows = one(a_slow_step());
    let fleet = Arc::new(crate::daemon::Fleet::assembled(fittings));
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .expect("a proposed Job");
    worktree_directory(home, &job);
    dispatched(&fleet, job.id()).await.expect("an approved Job");
    (fleet, job.id().clone())
}

/// The gate's own reading of this Job's one step, over the wire — whether
/// `check_runs` names the run the dry run answered, or the gate's own.
async fn gated_check_runs(fleet: &Arc<Fixture>, job: &core_model::JobId) -> Vec<ipc::CheckRun> {
    fleet.turn().await.expect("the gate ruled");
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(fleet),
        ipc::RunId::carried("01RUN"),
        events,
    ));
    let (_, body) = get(&app, &format!("/jobs/{}", job.as_str())).await;
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");
    detail.steps[0].check_runs.clone()
}

/// **An edit made while the dry run is still in flight.** The Drone starts
/// `run_checks`, the worktree moves before the background run finishes, and
/// every Check the run itself observed still passed. #1035 let the two
/// overlap for the first time — `run_checks` answers before the run ends, so
/// the worktree is no longer the Drone's alone while it runs. This is what
/// pins `crate::reuse` against a footprint taken at the end instead of the
/// one `dry_run_reads` takes before any Check starts: that bug would still
/// see the passing report and reuse it.
#[tokio::test]
async fn an_edit_while_the_dry_run_is_in_flight_means_every_check_runs_fresh_at_the_gate() {
    let home = crate::tests::tmp::TempDir::new();
    let (fleet, job) = dispatched_onto(&home).await;

    let underway = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    for _ in 0..400 {
        if fleet
            .the_only_slot()
            .await
            .lock()
            .await
            .as_ref()
            .is_some_and(|at_work| at_work.is_checking())
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    // The edit: the worktree moves while the Checks the Drone is waiting on
    // are still running.
    fleet.work().wrote(&[("src/parse.rs", Change::Modified)]);
    let report = underway
        .finished()
        .await
        .expect("the step did not end while the run was going")
        .expect("a report");
    assert!(
        report
            .ran
            .iter()
            .all(|row| row.outcome.as_wire() == "passed"),
        "the dry run itself saw every Check pass: {:?}",
        report.ran
    );

    submitted_by_the_one(&fleet, crate::tests::daemon::diff_evidence())
        .await
        .expect("the tool took it");
    let check_runs = gated_check_runs(&fleet, &job).await;
    assert_eq!(check_runs.len(), 1);
    assert_eq!(
        check_runs[0].reused_from_dry_run, None,
        "the dry run measured a worktree that moved before the gate looked \
         at it, so nothing from it may be trusted"
    );
}

/// **A submission that stops a dry run mid-run.** The gate runs every Check
/// fresh — never a stale pass off a run that never finished — and no
/// `KeptDryRun` exists to have answered from: `dry_run_ends` only writes one
/// inside the same guard that keeps this run from being reported at all, and
/// a stopped run never reaches it.
#[tokio::test]
async fn a_submission_that_stops_the_dry_run_means_every_check_runs_fresh_at_the_gate() {
    let home = crate::tests::tmp::TempDir::new();
    let (fleet, job) = dispatched_onto(&home).await;

    let underway = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    for _ in 0..400 {
        if fleet
            .the_only_slot()
            .await
            .lock()
            .await
            .as_ref()
            .is_some_and(|at_work| at_work.is_checking())
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    submitted_by_the_one(&fleet, crate::tests::daemon::diff_evidence())
        .await
        .expect("the tool took it");
    assert!(
        underway.finished().await.is_none(),
        "a run the submission stopped was still reported"
    );

    let check_runs = gated_check_runs(&fleet, &job).await;
    assert_eq!(check_runs.len(), 1);
    assert_eq!(
        check_runs[0].reused_from_dry_run, None,
        "a stopped dry run kept nothing the gate could ever have reused"
    );
}
