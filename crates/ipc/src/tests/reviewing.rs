//! What a reviewing person is handed, and what it declines to guess at.
//!
//! The absent-never-empty rule bites twice here: a Job with no worktree
//! carries no reading at all, and a submission that drew no boundary carries
//! no `not_claimed`. An empty object and an empty string would each read as a
//! value somebody lost.
//!
//! A note coming the other way is refused on its spelling and not on its
//! emptiness — a decoded request is well-formed, and a value that cannot work
//! is Fleet's 422 rather than the wire's 400.

use core_model::EvidenceType;

use crate::{decode, encode, ChangesRequested, JobDiff, JobEvidence, Submitted, Work};

/// **The reviewing reads keep the absent-never-empty rule, in both places it
/// bites.** A Job with no worktree carries no reading at all, and a submission
/// that drew no boundary carries no `not_claimed` — an empty object and an
/// empty string would each read as a value somebody lost.
#[test]
fn the_reviewing_reads_are_absent_rather_than_empty() {
    let nothing_to_read = JobDiff {
        job_id: crate::JobId::carried("01JOB"),
        work: None,
    };
    let json = encode(&nothing_to_read).expect("a diff is plain data");
    assert!(
        !json.contains("\"work\""),
        "there was no worktree, and absent is not an empty reading: {json}"
    );

    let read_and_empty = JobDiff {
        job_id: crate::JobId::carried("01JOB"),
        work: Some(Work {
            files: Vec::new(),
            plan_declared: false,
            patch: None,
        }),
    };
    let json = encode(&read_and_empty).expect("a diff is plain data");
    assert!(
        json.contains("\"files\":[]"),
        "a worktree that opened and holds no change is a real answer: {json}"
    );
    assert!(
        !json.contains("\"patch\""),
        "and nothing in it is absent rather than blank: {json}"
    );

    let boundless = JobEvidence {
        job_id: crate::JobId::carried("01JOB"),
        steps: vec![Submitted {
            step_id: crate::StepId::carried("repro"),
            evidence_type: EvidenceType::FailingTest.into(),
            claimed: "the reader stops one line early".to_string(),
            shown_by: "a failing test".to_string(),
            not_claimed: None,
        }],
    };
    let json = encode(&boundless).expect("evidence is plain data");
    assert!(
        !json.contains("not_claimed"),
        "legitimately empty on the record is absent on the wire: {json}"
    );
    assert!(
        json.contains("\"evidence_type\":\"failing_test\""),
        "the registry's own spelling, through the domain pair: {json}"
    );
}

/// A note is refused on its spelling, not on its emptiness. **Blank is Fleet's
/// refusal** — a decoded request is well-formed, and a value that cannot work
/// is a 422 rather than a 400.
#[test]
fn a_blank_review_note_decodes_and_is_refused_further_in() {
    let note = decode::<ChangesRequested>("a review note", br#"{"note":"   "}"#)
        .expect("the bytes became a request");
    assert_eq!(note.note, "   ");
}
