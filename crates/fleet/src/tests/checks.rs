//! What the gate wrote down, and what a reader can tell from it.
//!
//! `gate` proves the ruling; this proves the record. They are separate subjects
//! because a Job that ended for the right reason and left no trace of why is
//! the failure this file exists to catch — the verdict is in the log, and what
//! it was derived from was not anywhere until now.
//!
//! The Checks are real commands, for the reason `gate`'s comment gives: a
//! hanging Check and a command that is not installed are the operating system's
//! behaviour.

use std::time::Duration;

use adapter_traits::{Footprint, WorkProduct};
use core_model::CheckOutcome;
use ipc::{JobDetail, RunId};
use testkit::FakeWorkProduct;
use verification::{Lifted, Request};

use crate::at_step::AtStep;
use crate::gate::{rule_on, CheckBudget};
use crate::tests::daemon::{a_fleet, a_proposal, worktree_directory};
use crate::tests::detail::get;
use crate::tests::gate::{budget, diff_evidence, judging, note_evidence, workflow, worktree};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use core_model::StepId;

/// The `(name, outcome, produced)` of every check the step declared, in order.
fn recorded(ruling: &crate::gate::Ruling) -> Vec<(String, CheckOutcome, Option<String>)> {
    ruling
        .checks()
        .iter()
        .map(|check| (check.name.clone(), check.outcome, check.produced.clone()))
        .collect()
}

#[tokio::test]
async fn a_check_that_passes_is_written_down_as_a_pass() {
    let workflow = workflow("/usr/bin/true");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await;

    assert_eq!(
        recorded(&ruling),
        vec![
            ("suite".to_string(), CheckOutcome::Passed, None),
            ("diff_nonempty".to_string(), CheckOutcome::Passed, None),
        ],
        "a pass is a row — a step that advanced writing nothing down cannot be \
         told from a step whose checks never ran"
    );
    assert!(
        ruling.checks().iter().all(|check| check.expected.is_none()),
        "nothing was measured against and missed"
    );
}

#[tokio::test]
async fn a_check_that_fails_records_the_code_it_returned() {
    let workflow = workflow("/usr/bin/false");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await;

    let checks = ruling.checks();
    assert_eq!(checks[0].outcome, CheckOutcome::Failed);
    assert_eq!(checks[0].expected.as_deref(), Some("`suite` exits 0"));
    assert_eq!(checks[0].produced.as_deref(), Some("it exited 1"));
    assert_eq!(
        checks[1].outcome,
        CheckOutcome::Passed,
        "the diff check passed and is recorded beside the one that did not"
    );
}

/// A hanging Check and a Check that returned the wrong code are two different
/// things to do about it, and the record keeps them apart.
#[tokio::test]
async fn a_hanging_check_is_recorded_as_timed_out_and_not_as_failed() {
    let workflow = workflow("/bin/sleep 3600");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        CheckBudget::of(Duration::from_millis(300)),
        &judging(),
        &keeping_nowhere(),
    )
    .await;

    let checks = ruling.checks();
    assert_eq!(checks[0].outcome, CheckOutcome::TimedOut);
    assert_ne!(checks[0].outcome, CheckOutcome::Failed);
    assert_eq!(
        checks[0].produced.as_deref(),
        Some("it was still running after 0s")
    );
}

/// The failure a fresh machine actually produces. **Never a vacuous pass**, and
/// never confused with the suite having a real opinion.
#[tokio::test]
async fn a_check_whose_command_does_not_exist_is_recorded_as_never_ran() {
    let workflow = workflow("armada-no-such-program");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await;

    let checks = ruling.checks();
    assert_eq!(checks[0].outcome, CheckOutcome::NeverRan);
    assert_eq!(
        checks[0].produced.as_deref(),
        Some("`armada-no-such-program` is not installed"),
        "which of the four not-passes it was, in words a person can act on"
    );
    assert!(!checks[0].outcome.passed());
}

/// **The defect this repository shipped.** A step that wrote nothing advanced
/// on the files the step before it had committed.
///
/// `diff_nonempty` was decided from `WorkProduct::changed_files`, which reads
/// the branch — everything since the commit it was cut from. So the second step
/// of a workflow inherited the first step's work and passed on it, and every
/// step after that inherited the lot. A Job reached `Write tests` having
/// produced no code at all, credited with a `SCOPE.md` its own Drone had said,
/// in the evidence, that it did not write.
///
/// The step is measured against what the worktree held when *it* began, so the
/// inherited files are not its.
#[tokio::test]
async fn a_step_that_added_nothing_to_what_it_inherited_fails_its_diff_check() {
    let workflow = workflow("/usr/bin/true");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    // A worktree already holding an earlier step's work, which this step does
    // not add to.
    let work = FakeWorkProduct::inherited(&["SCOPE.md"]);
    let entered_with = work.footprint(&worktree).expect("the step's own start");

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&entered_with),
        &[],
        &work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await;

    assert!(
        !ruling.advanced(),
        "the branch is not empty, and this step still produced nothing"
    );
    assert_eq!(
        recorded(&ruling),
        vec![
            ("suite".to_string(), CheckOutcome::Passed, None),
            (
                "diff_nonempty".to_string(),
                CheckOutcome::Failed,
                Some("nothing moved while this step ran".to_string()),
            ),
        ],
        "the Manifest Check passes on an unchanged tree precisely because \
         nothing changed, which is why it can never be the one that catches \
         this — and what is recorded says the step moved nothing, not that the \
         worktree was empty, because it was not"
    );
}

/// The other half of the same rule: a step that *did* move the work advances,
/// and the baseline it is measured from is a real reading rather than an
/// assumption that the worktree started empty.
#[tokio::test]
async fn a_step_that_moved_work_it_inherited_advances() {
    let workflow = workflow("/usr/bin/true");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["SCOPE.md"]);
    let entered_with = work.footprint(&worktree).expect("the step's own start");

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&entered_with),
        &[],
        &work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await;

    assert!(ruling.advanced(), "the step changed the file it inherited");
}

/// A baseline Fleet never managed to read is **not** a worktree that did not
/// move. Nothing is known to have changed, so the step does not advance.
#[tokio::test]
async fn a_step_whose_start_was_never_read_does_not_advance_on_the_doubt() {
    let workflow = workflow("/usr/bin/true");
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        None,
        &[],
        &work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await;

    assert!(
        !ruling.advanced(),
        "an unread baseline fails the check rather than passing it"
    );
}

#[tokio::test]
async fn an_ungated_step_records_nothing_because_there_was_nothing_to_run() {
    let workflow = workflow("/usr/bin/false");
    let worktree = worktree();
    let at_step = AtStep::named(workflow.frozen(), &StepId::new("summarise"), &worktree)
        .expect("the second step");
    let work = FakeWorkProduct::untouched();

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &note_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await;

    assert!(ruling.advanced());
    assert!(
        ruling.checks().is_empty(),
        "no check was declared, so none is recorded — the declaration is what \
         says the step is ungated"
    );
}

/// What `list_workflows` serves is what `get_job` answers from — one reading of
/// a step's declaration, so a composer and a running Job's rail cannot disagree
/// about what a step is gated on.
#[tokio::test]
async fn the_workflow_list_carries_the_checks_each_step_declares() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (_, body) = get(&app, "/workflows").await;
    let workflows: Vec<ipc::WorkflowSummary> =
        ipc::decode("the workflows", &body).expect("a workflow list");

    let steps = &workflows[0].steps;
    assert_eq!(steps[0].step_id.as_str(), "implement");
    assert_eq!(
        steps[0].label, "Implement",
        "a picker offers the word, not the key"
    );
    assert_eq!(steps[0].checks[0].kind, "diff_nonempty");
    assert!(
        steps[0].checks[0].name.is_none(),
        "a built-in names no Check"
    );
    // Ungated, spelled as an empty list. A WorkflowDef may omit
    // `mechanical_checks` or write `[]`, and the wire says it one way.
    assert!(steps[1].checks.is_empty());
}

/// The end of the whole line: a Check runs, its result is written against the
/// step, and `get_job` serves it beside what the step declared.
#[tokio::test]
async fn what_the_gate_found_reaches_the_detail_view() {
    let home = TempDir::new();
    // Nothing changed, so the `implement` step's `diff_nonempty` fails.
    let fleet = a_fleet(&home, FakeWorkProduct::untouched());
    let job = fleet
        .propose(a_proposal("change nothing"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job_id);
    fleet.approve(&job_id).await.expect("released to run");
    submitted_by_the_one(&fleet, crate::tests::daemon::diff_evidence())
        .await
        .expect("the tool took it");
    fleet.turn().await.expect("the gate ruled");

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (_, body) = get(&app, &format!("/jobs/{}", job_id.as_str())).await;
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");

    let implement = &detail.steps[0];
    assert_eq!(
        implement.checks.as_ref().map(Vec::len),
        Some(1),
        "the step declares one check, read from the workflow"
    );
    assert_eq!(implement.checks.as_ref().unwrap()[0].kind, "diff_nonempty");
    assert_eq!(implement.check_runs.len(), 1);
    assert_eq!(implement.check_runs[0].outcome.as_wire(), "failed");
    assert_eq!(
        implement.check_runs[0].produced.as_deref(),
        Some("nothing moved while this step ran")
    );

    let summarise = &detail.steps[1];
    assert_eq!(
        summarise.checks.as_deref(),
        Some(&[][..]),
        "**declares none**, which is a sentence and not a gap"
    );
    assert!(
        summarise.check_runs.is_empty(),
        "nothing ran, which the empty declaration already explains"
    );
}

/// The record survives the process that wrote it. A Job read back by a second
/// Fleet still says which Check failed and how.
#[tokio::test]
async fn the_record_survives_a_fleet_restart() {
    let home = TempDir::new();
    let job_id = {
        let fleet = a_fleet(&home, FakeWorkProduct::untouched());
        let job = fleet
            .propose(a_proposal("change nothing"))
            .await
            .expect("a Job at the gate");
        worktree_directory(&home, job.id());
        fleet.approve(job.id()).await.expect("released to run");
        submitted_by_the_one(&fleet, crate::tests::daemon::diff_evidence())
            .await
            .expect("the tool took it");
        // **The gate refusing is what ends this Drone**, on the turn above:
        // `dispatch` terminates a Drone whose step stopped, and `terminate`
        // waits. So the slot is already empty here and there is nothing for
        // `the_drone_is_gone` to end — which is why this case never flaked the
        // way `#443`'s six did.
        fleet.turn().await.expect("the gate ruled");
        job.id().clone()
    };
    let restarted = a_fleet(&home, FakeWorkProduct::untouched());
    restarted.reconcile().await.expect("a boot read");
    let events = restarted.events();
    let app = api::router(api::Served::by(restarted, RunId::carried("01RUN"), events));

    let (_, body) = get(&app, &format!("/jobs/{}", job_id.as_str())).await;
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");
    assert_eq!(detail.steps[0].check_runs.len(), 1);
    assert_eq!(detail.steps[0].check_runs[0].outcome.as_wire(), "failed");
}
