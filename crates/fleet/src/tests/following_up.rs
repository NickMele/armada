//! A For context finding becomes a Job that waits on this one, or an issue filed on a person's
//! confirm, and the finding keeps what it became. #906.

use std::sync::Arc;

use api::{Commands, Refusal};
use core_model::{
    Area, Became, Bucket, Confidence, DependencyDirection, DependencyEdge, Finding, FollowUp,
    JobId, TestsInChange,
};
use ipc::mcp::{SubmitEvidence, SubmittedReview};
use testkit::{Delivering, FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const CODE: &str = "src/headroom.rs";
const DIFF: &str = "diff --git a/src/headroom.rs b/src/headroom.rs\n-    Cpu,\n";
const FINDING: &str = "`src/headroom.rs` has grown past what one reader holds";
const NEEDS_YOU: &str = "The lock order when saving";

fn a_fleet(home: &TempDir, delivering: Delivering) -> Fixture {
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
    fittings.vcs = FakeVcs::new().delivering(delivering);
    Fleet::assembled(fittings)
}

/// A Job whose review raised one finding for context and one that needs a person.
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
        findings: vec![
            Finding::of(
                Bucket::ForContext,
                FINDING,
                "The author flagged it for later",
            ),
            Finding::of(Bucket::NeedsYou, NEEDS_YOU, "It can deadlock"),
        ],
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

fn filing(finding: &str, title: &str) -> ipc::IssueFiled {
    ipc::IssueFiled {
        finding: finding.to_string(),
        title: title.to_string(),
        body: "Raised by Armada's review.".to_string(),
    }
}

async fn followed(fleet: &Fixture, job: &JobId) -> Vec<FollowUp> {
    fleet
        .store()
        .lock()
        .await
        .followups(job)
        .expect("follow-ups are readable")
}

/// **The queued Job is created waiting on this one**, and the finding names it.
#[tokio::test]
async fn queueing_a_finding_proposes_a_job_that_waits_on_this_one() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Delivering::default());
    let job = reviewed(&fleet, &home).await;

    fleet
        .queue_after_finding(
            ipc::JobId::from(&job),
            ipc::FindingQueued {
                finding: FINDING.to_string(),
            },
        )
        .await
        .expect("a finding raised for context");

    let followed = followed(&fleet, &job).await;
    let [FollowUp {
        became: Became::Queued { job: queued },
        ..
    }] = followed.as_slice()
    else {
        panic!("one Job queued, got {followed:?}");
    };
    let created = fleet.load(queued).await.expect("the queued Job exists");
    assert_eq!(
        created.dependencies(),
        &[DependencyEdge {
            direction: DependencyDirection::DependsOn,
            peer: job.clone(),
        }]
    );
    assert_eq!(
        created.title().as_str(),
        "src/headroom.rs has grown past what one reader holds"
    );
}

#[tokio::test]
async fn filing_an_issue_names_where_the_forge_filed_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Delivering::default());
    let job = reviewed(&fleet, &home).await;

    fleet
        .file_finding_issue(ipc::JobId::from(&job), filing(FINDING, "Split headroom.rs"))
        .await
        .expect("a confirmed draft");

    assert_eq!(
        followed(&fleet, &job).await,
        vec![FollowUp {
            finding: FINDING.to_string(),
            became: Became::Issue {
                url: "https://forge.invalid/armada/issues/1".to_string(),
            },
        }]
    );
}

/// **Refused on a blank title, on a finding not raised for context, and where the forge
/// refuses**, and nothing is kept.
#[tokio::test]
async fn an_issue_with_no_title_off_context_or_refused_by_the_forge_is_not_kept() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        Delivering {
            filed: Err(adapter_traits::NotFiled {
                said: "issues are disabled".to_string(),
            }),
            ..Delivering::default()
        },
    );
    let job = reviewed(&fleet, &home).await;
    let id = ipc::JobId::from(&job);

    assert!(matches!(
        fleet
            .file_finding_issue(id.clone(), filing(FINDING, "  "))
            .await,
        Err(Refusal::Unacceptable(_))
    ));
    assert!(matches!(
        fleet
            .file_finding_issue(id.clone(), filing(NEEDS_YOU, "Fix the lock"))
            .await,
        Err(Refusal::IllegalMove(_))
    ));
    assert!(matches!(
        fleet
            .file_finding_issue(id, filing(FINDING, "Split headroom.rs"))
            .await,
        Err(Refusal::IllegalMove(_))
    ));
    assert_eq!(followed(&fleet, &job).await, Vec::new());
}

/// **A second press on a queued finding proposes nothing**, even when both presses arrive at once.
#[tokio::test]
async fn a_finding_already_queued_is_refused_and_proposes_no_second_job() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Delivering::default());
    let job = reviewed(&fleet, &home).await;
    let press = || {
        fleet.queue_after_finding(
            ipc::JobId::from(&job),
            ipc::FindingQueued {
                finding: FINDING.to_string(),
            },
        )
    };

    let (first, second) = tokio::join!(press(), press());
    assert!(
        matches!(
            (&first, &second),
            (Ok(_), Err(Refusal::IllegalMove(_))) | (Err(Refusal::IllegalMove(_)), Ok(_))
        ),
        "one press queues a Job and the other is refused"
    );
    assert!(matches!(press().await, Err(Refusal::IllegalMove(_))));
    assert_eq!(followed(&fleet, &job).await.len(), 1);
}
