//! What Fleet takes from a reviewer, what it refuses, and what it adds.

use config::EvidenceType;

use core_model::{
    Area, Bucket, ChangedTest, Confidence, Finding, Proves, TestChange, TestsInChange,
};

use crate::review::{Review, ReviewRefused};
use crate::submission::{NotASubmission, Submission};

const DIFF: &str = "diff --git a/src/cursor.rs b/src/cursor.rs\n\
                    -fn reads_past_the_end() {\n\
                    +fn stops_at_the_end() {\n";

fn removed(name: &str, why: Option<&str>) -> ChangedTest {
    ChangedTest {
        name: name.to_string(),
        change: TestChange::Removed { replaced_by: None },
        why: why.map(str::to_string),
    }
}

fn a_review(changed: Vec<ChangedTest>, findings: Vec<Finding>) -> Review {
    Review::written(
        Confidence::NotConfident,
        &["The cursor's bound moved and its old test went with it."],
        vec![Area::of("Cursor", "Stops at the end", &["src/cursor.rs"])],
        TestsInChange {
            proves: vec![Proves::of("Cursor", "The bound holds", 1)],
            changed,
            untested: vec![],
        },
        findings,
    )
}

#[test]
fn a_file_the_review_leaves_out_is_refused_by_name() {
    let refused = a_review(vec![], vec![])
        .against(&["src/cursor.rs", "src/reader.rs"], DIFF)
        .expect_err("a changed file in no area");
    assert_eq!(
        refused,
        vec![ReviewRefused::FileInNoArea {
            path: "src/reader.rs".to_string()
        }]
    );
}

#[test]
fn every_fault_comes_back_in_one_pass() {
    let refused = a_review(
        vec![removed("a_test_the_diff_never_touches", Some("flaky"))],
        vec![Finding::of(Bucket::SmallFix, "Rename it", "   ")],
    )
    .against(&["src/cursor.rs", "src/reader.rs"], DIFF)
    .expect_err("three faults");
    assert_eq!(refused.len(), 3, "{refused:?}");
}

#[test]
fn a_removed_test_with_no_reason_needs_the_person_before_anything_else() {
    let accepted = a_review(
        vec![removed("reads_past_the_end", None)],
        vec![Finding::of(
            Bucket::ForContext,
            "The reader is next",
            "It shares the bound",
        )],
    )
    .against(&["src/cursor.rs"], DIFF)
    .expect("an accountable review");
    let first = &accepted.findings()[0];
    assert_eq!(first.bucket(), Bucket::NeedsYou);
    assert!(first.finding().contains("reads_past_the_end"), "{first:?}");
    assert_eq!(accepted.findings().len(), 2);
    assert_eq!(accepted.unexplained_tests().len(), 1);
}

#[test]
fn a_removed_test_with_a_reason_adds_nothing() {
    let accepted = a_review(
        vec![removed("reads_past_the_end", Some("It asserted the bug"))],
        vec![],
    )
    .against(&["src/cursor.rs"], DIFF)
    .expect("an accountable review");
    assert!(accepted.findings().is_empty());
    assert!(accepted.unexplained_tests().is_empty());
}

#[test]
fn a_review_is_handed_in_as_review_evidence_and_keeps_its_parts() {
    let review = a_review(vec![], vec![]);
    let submitted = Submission::reviewed(review.clone()).expect("a review with a reason");
    assert_eq!(submitted.evidence_type(), EvidenceType::Review);
    assert_eq!(submitted.review(), Some(&review));
}

#[test]
fn a_review_giving_no_reason_for_its_verdict_is_not_a_submission() {
    let review = Review::written(
        Confidence::Confident,
        &[],
        vec![],
        TestsInChange::default(),
        vec![],
    );
    assert_eq!(
        Submission::reviewed(review),
        Err(NotASubmission::ClaimedEmpty)
    );
}
