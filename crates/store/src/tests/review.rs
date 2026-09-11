//! The review Fleet composed at a Job's gate survives the process that wrote
//! it, and a second gate overwrites the first rather than layering on top.

use crate::tests::{open, top_level, TempDir};
use crate::{Review, Store};

fn a_job(store: &mut Store, id: &str) {
    let job = top_level(id);
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
}

/// The four parts come back exactly as they went in.
#[test]
fn the_review_composed_at_a_gate_is_read_back() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01REVIEW0000000000000001");
    let id = crate::tests::job_id("01REVIEW0000000000000001");
    let composed = Review {
        why: Some("Add the export button the brief asked for.".to_string()),
        outcome: Some("One file changed:\n\n- `src/export.ts` — added\n".to_string()),
        evidence: Some("**Implement** — advanced\n".to_string()),
        risks: Some("Every line below is something Fleet ran.".to_string()),
    };
    store
        .record_review(&id, &composed)
        .expect("the review is recorded");
    let read = store.review_for(&id).expect("the review is read");
    assert_eq!(read, composed, "every section comes back as it went in");
}

/// A Job that has not reached a gate, or one written before this shipped, has
/// nothing to say — and that is `is_empty`, not four empty strings.
#[test]
fn a_job_with_no_gate_yet_has_nothing_to_say() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01REVIEW0000000000000002");
    let id = crate::tests::job_id("01REVIEW0000000000000002");
    let read = store.review_for(&id).expect("the review is read");
    assert!(read.is_empty(), "nothing was recorded, and nothing is read");
}

/// A Job re-entering an earlier gate on a restart must not read the far
/// gate's words beside this one's, which is why every field is written
/// together.
#[test]
fn a_second_gate_clears_what_the_first_one_wrote() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01REVIEW0000000000000003");
    let id = crate::tests::job_id("01REVIEW0000000000000003");
    store
        .record_review(
            &id,
            &Review {
                why: Some("first gate".to_string()),
                outcome: Some("first outcome".to_string()),
                evidence: Some("first evidence".to_string()),
                risks: Some("first risks".to_string()),
            },
        )
        .expect("the first review is recorded");
    store
        .record_review(
            &id,
            &Review {
                why: Some("second gate".to_string()),
                outcome: Some("second outcome".to_string()),
                evidence: Some("second evidence".to_string()),
                risks: None,
            },
        )
        .expect("the second review is recorded");
    let read = store.review_for(&id).expect("the review is read");
    assert_eq!(read.why.as_deref(), Some("second gate"));
    assert!(
        read.risks.is_none(),
        "the second gate said nothing about risk, and the first gate's words do not survive it"
    );
}

/// A Job the file does not hold is not an error: the read is spent on a Job
/// that may have been forgotten between the list and the open.
#[test]
fn a_job_that_is_not_there_reads_as_nothing() {
    let dir = TempDir::new();
    let store: Store = open(&dir);
    let read = store
        .review_for(&crate::tests::job_id("01REVIEW0000000000000004"))
        .expect("a missing job is not a failure");
    assert!(read.is_empty());
}
