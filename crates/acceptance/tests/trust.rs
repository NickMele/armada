//! Trust's claim: **work that passes its Check but is wrong gets caught, and I
//! can see that it was caught rightly, before I take it.** #903.
//!
//! Written before the code it names; the compiler's errors are what #903 provides.
//!
//! | Proved | Not proved |
//! |---|---|
//! | A delivering step asks for a review, and the Job holds there with it | That a Drone writes a good one |
//! | A review not accounting for the change is refused, every fault in one pass | That the refusal reaches the Drone as a turn |
//! | A removed test opens its section, named, with its row marked | That Bridge draws it; the Screens story does |
//! | A removed test given no reason needs the person | That pull request CI is read, which is a forge |
//! | The review survives the wire whole | View #904, What should change #907, CI #905, follow-ups #906 |

// Shared with the other milestones' tests, and none of them uses all of it.
#[allow(dead_code)]
mod bench;

use core_model::{JobStatus, StepState};
use testkit::{FakeJudge, FakeWorkProduct};
use verification::{Bucket, ReviewRefused};

use bench::board::{delivered, on_its_branch, received_detail, step_facts};
use bench::trust::{
    a_review_that_removes_a_test, a_review_wrong_three_ways, reviews_before_merging, submitted,
    the_diff, CHANGED, HANDOFF, REMOVED_TEST,
};
use bench::{a_fix_diff, a_root_cause_note, states, Bench, Run};

const PULL_REQUEST: &str = "https://forge.invalid/armada/pull/1";

async fn worked(bench: &Bench, run: &mut Run, at: usize, submitted: &verification::Submission) {
    let step = bench.step(at);
    let ruling = bench.gate(run, &step, submitted).await;
    bench.settled(run, &step, &ruling);
}

/// Both working steps passed and the delivering step handed in the review.
async fn a_job_at_the_review() -> (Run, Bench, Option<core_model::TransitionReason>) {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&CHANGED),
        reviews_before_merging(),
        FakeJudge::that_fails("a Judge that should never be asked"),
    );
    let mut run = bench.created("leave CPU to macOS");
    on_its_branch(&mut run);
    bench.approved_and_dispatched(&mut run);
    worked(&bench, &mut run, 0, &a_root_cause_note()).await;
    worked(&bench, &mut run, 1, &a_fix_diff()).await;
    worked(
        &bench,
        &mut run,
        2,
        &submitted(a_review_that_removes_a_test()),
    )
    .await;
    let reason = bench.reasons().last().cloned();
    (run, bench, reason)
}

/// One stop, with the pull request already open: the review is read there.
#[tokio::test]
async fn the_job_holds_at_the_step_that_asked_for_the_review() {
    let (run, _, _) = a_job_at_the_review().await;
    assert_eq!(run.job.status(), JobStatus::AwaitingReview);
    assert_eq!(
        states(&run.job),
        [
            ("root_cause", StepState::Advanced),
            ("fix", StepState::Advanced),
            (HANDOFF, StepState::AwaitingHuman)
        ],
    );
    assert_eq!(
        run.job
            .workflow()
            .step(&core_model::StepId::new(HANDOFF))
            .and_then(|step| step.evidence_type()),
        Some(config::EvidenceType::Review),
    );
}

/// Every fault by name in one pass, so a Drone told to write it again is told everything.
#[test]
fn a_review_that_does_not_account_for_the_change_is_refused() {
    let refused = a_review_wrong_three_ways()
        .against(&CHANGED, the_diff())
        .expect_err("a review leaving the change unaccounted for is refused");
    assert_eq!(
        refused,
        vec![
            ReviewRefused::FileInNoArea {
                path: "crates/fleet/src/admitting.rs".to_string()
            },
            ReviewRefused::FileInNoArea {
                path: "crates/fleet/src/tests/headroom.rs".to_string()
            },
            ReviewRefused::TestNotInDiff {
                name: "a_test_nobody_wrote".to_string()
            },
            ReviewRefused::FindingWithNoReason {
                finding: "Rename a variable".to_string()
            },
        ]
    );
}

/// A removed test the reviewer gave no reason for is not theirs to wave through.
#[test]
fn a_removed_test_with_no_reason_needs_the_person() {
    let accepted = a_review_that_removes_a_test()
        .against(&CHANGED, the_diff())
        .expect("every changed file is in an area and every named test is in the diff");
    let needs_you: Vec<&str> = accepted
        .findings()
        .iter()
        .filter(|finding| finding.bucket() == Bucket::NeedsYou)
        .map(|finding| finding.finding())
        .collect();
    assert!(
        needs_you
            .iter()
            .any(|finding| finding.contains(REMOVED_TEST)),
        "{needs_you:?}"
    );
}

/// On the detail a Board receives: the section says why it opened, naming the test.
#[tokio::test]
async fn a_removed_test_opens_its_section_and_names_the_test() {
    let (run, _, reason) = a_job_at_the_review().await;
    let accepted = a_review_that_removes_a_test()
        .against(&CHANGED, the_diff())
        .expect("an accountable review");

    let mut detail = delivered(
        &run.job,
        reason.as_ref(),
        &step_facts(&run.job, &[]),
        ipc::JobDelivery {
            commit: None,
            pushed: None,
            pull_request: Some(String::from(PULL_REQUEST)),
            pull_request_detail: None,
            landed: None,
            unpushed: None,
        },
    );
    detail.confidence = Some(ipc::JobConfidence::of(&accepted.recorded()));
    let received = received_detail(&detail);

    let review = received.confidence.expect("the review reaches the Job");
    assert_eq!(review.says.as_wire(), "confident");
    let tests = review.tests.expect("the change touches tests");
    assert_eq!(
        tests.opened_because,
        Some(ipc::OpenedBecause::TestRemoved {
            name: REMOVED_TEST.to_string()
        }),
    );
    let flagged: Vec<&str> = tests
        .changed
        .iter()
        .filter(|row| row.flagged)
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(flagged, vec![REMOVED_TEST]);
    assert!(review
        .needs_you
        .iter()
        .any(|finding| finding.finding.contains(REMOVED_TEST)));
}
