//! A person dismisses a finding the review raised: refused without a reason or on a finding
//! the review did not raise, and otherwise kept, served apart, and handed to the next pass. #907.

use std::sync::Arc;

use api::{Commands, Refusal};
use core_model::{Area, Bucket, Confidence, Dismissal, Finding, JobId, TestsInChange};
use ipc::mcp::{SubmitEvidence, SubmittedReview};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::crossing::Dismissed;
use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const CODE: &str = "src/headroom.rs";
const DIFF: &str = "diff --git a/src/headroom.rs b/src/headroom.rs\n-    Cpu,\n";
const FINDING: &str = "The lock order when saving";

fn a_fleet(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[CODE]).showing(DIFF));
    fittings.starting().workflows = one(testkit::resolved(&[Sketch {
        id: "review",
        label: "Review the change",
        evidence_type: Some("review"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]));
    fittings.judge = Arc::new(FakeJudge::that_fails(
        "no model is asked about a review step",
    ));
    Fleet::assembled(fittings)
}

/// A Job whose review raised one finding for context.
async fn reviewed(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet
        .propose(a_proposal("review the change"))
        .await
        .expect("a proposal");
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.expect("dispatched");
    let review = SubmittedReview {
        says: Confidence::Confident,
        reasons: vec!["Every Check passed.".to_string()],
        areas: vec![Area::of("Admission", "CPU no longer holds a Job", &[CODE])],
        tests: TestsInChange::default(),
        findings: vec![Finding::of(
            Bucket::ForContext,
            FINDING,
            "The author flagged it",
        )],
    };
    fleet
        .record_evidence(
            job.id(),
            &SubmitEvidence {
                claimed: "The review of the CPU change".to_string(),
                shown_by: "the review's areas and findings".to_string(),
                not_claimed: String::new(),
                review: Some(review),
            },
        )
        .await
        .expect("the review accounts for the change");
    job.id().clone()
}

fn dismissing(finding: &str, reason: &str) -> ipc::FindingDismissed {
    ipc::FindingDismissed {
        finding: finding.to_string(),
        reason: reason.to_string(),
    }
}

/// **Refused on a blank reason and on a finding the review did not raise**, and nothing kept.
#[tokio::test]
async fn a_dismissal_without_a_reason_or_of_a_finding_never_raised_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = reviewed(&fleet, &home).await;

    let blank = fleet
        .dismiss_finding(ipc::JobId::from(&job), dismissing(FINDING, "   "))
        .await;
    assert!(matches!(blank, Err(Refusal::Unacceptable(_))), "{blank:?}");

    let never = fleet
        .dismiss_finding(
            ipc::JobId::from(&job),
            dismissing("A finding nobody raised", "It is wrong"),
        )
        .await;
    assert!(matches!(never, Err(Refusal::IllegalMove(_))), "{never:?}");

    let detail = api::Queries::get_job(&fleet, ipc::JobId::from(&job))
        .await
        .expect("the Job reads");
    let confidence = detail.confidence.expect("the review is served");
    assert!(confidence.dismissed.is_empty(), "nothing was dismissed");
}

/// **A dismissed finding leaves the lists and is served apart with its reason.**
#[tokio::test]
async fn a_dismissed_finding_leaves_the_review_and_keeps_its_reason() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = reviewed(&fleet, &home).await;

    fleet
        .dismiss_finding(
            ipc::JobId::from(&job),
            dismissing(FINDING, "Saving takes one lock, so there is no order"),
        )
        .await
        .expect("a finding the review raised, with a reason");

    let detail = api::Queries::get_job(&fleet, ipc::JobId::from(&job))
        .await
        .expect("the Job reads");
    let confidence = detail.confidence.expect("the review is served");
    assert!(
        confidence
            .for_context
            .iter()
            .all(|row| row.finding != FINDING),
        "the dismissed finding left the review"
    );
    assert_eq!(
        confidence.dismissed,
        vec![ipc::DismissedRow {
            finding: FINDING.to_string(),
            reason: "Saving takes one lock, so there is no order".to_string(),
        }]
    );
}

/// The next review pass is told what was ruled out, and nothing is drawn where nothing was.
#[test]
fn a_review_pass_is_handed_each_dismissal_and_its_reason() {
    assert_eq!(Dismissed::of(vec![]), None);
    let block = Dismissed::of(vec![Dismissal {
        finding: FINDING.to_string(),
        reason: "Saving takes one lock".to_string(),
    }])
    .expect("one dismissal draws the block")
    .text();
    assert!(block.starts_with("WHAT A PERSON RULED OUT"), "{block}");
    assert!(block.contains(FINDING), "{block}");
    assert!(block.contains("Saving takes one lock"), "{block}");
}
