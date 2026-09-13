//! The `review` object `submit_evidence` takes, read into the parts Fleet checks. #903.

use crate::mcp::{read, Incoming, NotAnArgument, ReviewArgument};

/// **A review arrives in `core_model`'s parts**, the same ones Fleet checks and keeps.
#[test]
fn a_review_reads_into_the_parts_fleet_checks() {
    let called = read(
        br#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"submit_evidence",
            "arguments":{"claimed":"c","shown_by":"s","not_claimed":"","review":{
              "says":"confident","reasons":["Every Check passed."],
              "areas":[{"name":"Admission","what":"CPU no longer holds a Job","files":["src/headroom.rs"]}],
              "tests":{"proves":[{"area":"Admission","what":"A saturated CPU holds nothing back","tests":2}],
                       "changed":[{"name":"a_loaded_machine_holds_a_job_back_too","change":"removed"}],
                       "untested":[]},
              "findings":[{"bucket":"needs_you","finding":"A busy CPU no longer delays a Job","why":"People may rely on it"}]}}}}"#,
    );
    let Incoming::Submit { submission, .. } = called else {
        panic!("a submission, got {called:?}");
    };
    let review = submission.review.expect("the review was read");
    assert_eq!(review.says, core_model::Confidence::Confident);
    assert_eq!(review.areas[0].files(), ["src/headroom.rs".to_string()]);
    assert_eq!(review.tests.proves[0].tests(), 2);
    assert!(review.tests.changed[0].unexplained(), "no why was given");
    assert_eq!(review.findings[0].bucket(), core_model::Bucket::NeedsYou);
}

/// A submission with no `review` still reads, as every step that asks for none sends.
#[test]
fn a_submission_without_a_review_reads_as_before() {
    let called = read(
        br#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"submit_evidence",
            "arguments":{"claimed":"c","shown_by":"s","not_claimed":""}}}"#,
    );
    let Incoming::Submit { submission, .. } = called else {
        panic!("a submission, got {called:?}");
    };
    assert_eq!(submission.review, None);
}

/// A verdict word the tool does not have is refused by name, never guessed at.
#[test]
fn a_verdict_it_does_not_know_is_refused_by_name() {
    let called = read(
        br#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"submit_evidence",
            "arguments":{"claimed":"c","shown_by":"s","not_claimed":"","review":{
              "says":"probably","reasons":[],"areas":[],
              "tests":{"proves":[],"changed":[],"untested":[]},"findings":[]}}}}"#,
    );
    assert!(matches!(
        called,
        Incoming::NotASubmission {
            why: NotAnArgument::Reviewing(ReviewArgument::NoSuchVerdict { ref named }),
            ..
        } if named == "probably"
    ));
}
