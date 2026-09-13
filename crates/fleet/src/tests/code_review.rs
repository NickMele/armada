//! A Code Review Job's review is checked against the pull request its subject names, which
//! the forge is asked for, because on Code Review the change is the Drone's input and not
//! anything in its worktree. #903.

use std::sync::Arc;

use adapter_traits::PullRequestDiff;
use core_model::{Area, Bucket, Confidence, Finding, JobId, TestsInChange};
use ipc::mcp::{SubmitEvidence, SubmittedReview};
use testkit::{Delivering, FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};
use verification::ReviewRefused;

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::NotSubmitted;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const PULL_REQUEST: &str = "https://forge.invalid/someone/else/pull/7";
const FILE: &str = "src/reader.rs";

fn the_pull_request() -> PullRequestDiff {
    PullRequestDiff {
        files: vec![FILE.to_string()],
        patch: format!("diff --git a/{FILE} b/{FILE}\n-    end + 1\n+    end\n"),
    }
}

/// A worktree holding no change, and a forge that shows `diff` for the pull request.
fn a_fleet(home: &TempDir, diff: Option<PullRequestDiff>) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().workflows = one(testkit::resolved(&[Sketch {
        id: "assess",
        label: "Assess",
        evidence_type: Some("review"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]));
    fittings.vcs = FakeVcs::new().delivering(Delivering {
        pull_request_diff: diff,
        ..Delivering::default()
    });
    fittings.judge = Arc::new(FakeJudge::that_fails(
        "no model is asked about a review step",
    ));
    Fleet::assembled(fittings)
}

async fn reviewing_a_pull_request(fleet: &Fixture, home: &TempDir) -> JobId {
    let mut proposal = a_proposal("review someone else's pull request");
    proposal.subject = Some(ipc::Subject {
        kind: "pull_request".to_string(),
        reference: PULL_REQUEST.to_string(),
    });
    let job = fleet.propose(proposal).await.expect("a proposal");
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.expect("dispatched");
    job.id().clone()
}

fn a_review_naming(file: &str) -> SubmitEvidence {
    SubmitEvidence {
        claimed: "The review of the reader's bound".to_string(),
        shown_by: "the review's areas and findings".to_string(),
        not_claimed: String::new(),
        review: Some(SubmittedReview {
            says: Confidence::NotConfident,
            reasons: vec!["The bound moved and nothing tests it.".to_string()],
            areas: vec![Area::of("Reader", "The bound is exclusive now", &[file])],
            tests: TestsInChange::default(),
            findings: vec![Finding::of(
                Bucket::NeedsYou,
                "Nothing tests the new bound",
                "Reading past the end is the bug this fixes",
            )],
        }),
    }
}

/// **Checked against the pull request's own files**, not the empty worktree.
#[tokio::test]
async fn a_code_review_is_checked_against_the_pull_request_its_subject_names() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Some(the_pull_request()));
    let job = reviewing_a_pull_request(&fleet, &home).await;

    let refused = fleet
        .record_evidence(&job, &a_review_naming("src/writer.rs"))
        .await
        .expect_err("the pull request's one file is in no area");
    let NotSubmitted::ReviewRefused(faults) = refused else {
        panic!("refused against the pull request, got {refused:?}");
    };
    assert_eq!(
        faults,
        vec![ReviewRefused::FileInNoArea {
            path: FILE.to_string()
        }]
    );

    fleet
        .record_evidence(&job, &a_review_naming(FILE))
        .await
        .expect("the review accounts for the pull request");
    let detail = api::Queries::get_job(&fleet, ipc::JobId::from(&job))
        .await
        .expect("the Job reads");
    assert!(detail.confidence.is_some(), "the review is served");
}

/// A forge that will not show the pull request refuses the review, by name, rather than
/// passing it against nothing.
#[tokio::test]
async fn a_code_review_the_forge_will_not_show_is_refused_as_unreadable() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, None);
    let job = reviewing_a_pull_request(&fleet, &home).await;

    let refused = fleet
        .record_evidence(&job, &a_review_naming(FILE))
        .await
        .expect_err("no diff to check against");
    assert!(
        matches!(refused, NotSubmitted::ChangeUnreadable { .. }),
        "{refused:?}"
    );
}
