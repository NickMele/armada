//! A review handed in through the evidence tool: checked against the change, refused by name
//! where it does not account for it, and kept and served on the Job where it does. #903.

use std::sync::Arc;

use core_model::{
    Area, Bucket, ChangedTest, Confidence, Finding, JobId, Proves, TestChange, TestsInChange,
    Untested,
};
use ipc::mcp::{SubmitEvidence, SubmittedReview};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};
use verification::ReviewRefused;

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::NotSubmitted;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const REMOVED: &str = "a_loaded_machine_holds_a_job_back_too";
const CODE: &str = "src/headroom.rs";
const TEST_FILE: &str = "src/tests/headroom.rs";
const DIFF: &str = "diff --git a/src/headroom.rs b/src/headroom.rs\n\
                    -    Cpu,\n\
                    diff --git a/src/tests/headroom.rs b/src/tests/headroom.rs\n\
                    -async fn a_loaded_machine_holds_a_job_back_too() {\n";

fn review_step() -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "review",
        label: "Review the change",
        evidence_type: Some("review"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn a_fleet(home: &TempDir) -> Fixture {
    let mut fittings = fittings(
        home,
        FakeWorkProduct::changed(&[CODE, TEST_FILE]).showing(DIFF),
    );
    fittings.starting().workflows = one(review_step());
    fittings.judge = Arc::new(FakeJudge::that_fails(
        "no model is asked about a review step",
    ));
    Fleet::assembled(fittings)
}

async fn settled(fleet: &Fixture) {
    let mut steady = 0;
    let mut last = usize::MAX;
    for _ in 0..400 {
        let seen = fleet
            .the_only_slot()
            .await
            .lock()
            .await
            .as_ref()
            .map_or(0, |at| at.turned());
        steady = if seen == last { steady + 1 } else { 0 };
        if steady > 20 {
            return;
        }
        last = seen;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

async fn started(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet
        .propose(a_proposal("review the change"))
        .await
        .expect("a proposal");
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.expect("dispatched");
    settled(fleet).await;
    job.id().clone()
}

async fn detail(fleet: &Fixture, job: &JobId) -> ipc::JobDetail {
    api::Queries::get_job(fleet, ipc::JobId::from(job))
        .await
        .expect("the Job reads")
}

/// A review naming `files` in its one area, and a removed test given no reason.
fn a_review(files: &[&str]) -> SubmittedReview {
    SubmittedReview {
        says: Confidence::Confident,
        reasons: vec!["Every Check passed.".to_string()],
        areas: vec![Area::of("Admission", "CPU no longer holds a Job", files)],
        tests: TestsInChange {
            proves: vec![Proves::of(
                "Admission",
                "A saturated CPU holds nothing back",
                1,
            )],
            changed: vec![ChangedTest {
                name: REMOVED.to_string(),
                change: TestChange::Removed { replaced_by: None },
                why: None,
            }],
            untested: vec![Untested::of("The status bar entry", "No test opens it")],
        },
        findings: vec![Finding::of(
            Bucket::NeedsYou,
            "A busy CPU no longer delays a Job",
            "People may rely on the old behaviour",
        )],
    }
}

fn handed_in(review: Option<SubmittedReview>) -> SubmitEvidence {
    SubmitEvidence {
        claimed: "The review of the CPU change".to_string(),
        shown_by: "the review's areas, tests and findings".to_string(),
        not_claimed: String::new(),
        review,
    }
}

/// **Refused in the tool's own reply, every fault named**, and nothing kept.
#[tokio::test]
async fn a_review_that_leaves_a_changed_file_out_is_refused_by_name() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = started(&fleet, &home).await;

    let refused = fleet
        .record_evidence(&job, &handed_in(Some(a_review(&[CODE]))))
        .await
        .expect_err("a review that does not account for the change");
    let NotSubmitted::ReviewRefused(faults) = refused else {
        panic!("refused as a review that does not account for the change, got {refused:?}");
    };
    assert_eq!(
        faults,
        vec![ReviewRefused::FileInNoArea {
            path: TEST_FILE.to_string()
        }]
    );
    assert_eq!(
        detail(&fleet, &job).await.confidence,
        None,
        "nothing was kept"
    );
}

/// A step that asks for a review is not handed a submission without one.
#[tokio::test]
async fn a_review_step_refuses_a_submission_with_no_review() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = started(&fleet, &home).await;

    let refused = fleet
        .record_evidence(&job, &handed_in(None))
        .await
        .expect_err("a review step with no review");
    assert!(
        matches!(refused, NotSubmitted::NoReview { .. }),
        "{refused:?}"
    );
}

/// **An accepted review is kept and reaches the Job**, with the removed test named.
#[tokio::test]
async fn an_accepted_review_is_kept_and_served_on_the_job() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = started(&fleet, &home).await;

    fleet
        .record_evidence(&job, &handed_in(Some(a_review(&[CODE, TEST_FILE]))))
        .await
        .expect("every changed file is in an area and the removed test is in the diff");

    let confidence = detail(&fleet, &job)
        .await
        .confidence
        .expect("the accepted review is served");
    assert_eq!(confidence.says.as_wire(), "confident");
    let tests = confidence.tests.expect("the change touches tests");
    assert_eq!(
        tests.opened_because,
        Some(ipc::OpenedBecause::TestRemoved {
            name: REMOVED.to_string()
        })
    );
    assert!(
        confidence
            .needs_you
            .iter()
            .any(|finding| finding.finding.contains(REMOVED)),
        "a test removed with no reason needs the person"
    );
}
