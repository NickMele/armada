//! A person's review model reads back, clears, and names no Job it was not set on. #903.

use crate::tests::attempt::on_its_first_run;
use crate::tests::{job_id, open, TempDir};

#[test]
fn a_chosen_review_model_reads_back_and_clears() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01REVIEWMODEL");
    let job = job_id("01REVIEWMODEL");
    assert_eq!(
        store.review_model_override(&job).expect("the read answers"),
        None
    );

    store
        .set_review_model_override(&job, Some("a-strong-model"))
        .expect("the choice is kept");
    assert_eq!(
        store.review_model_override(&job).expect("the read answers"),
        Some("a-strong-model".to_string())
    );
    assert_eq!(
        store.model_override(&job).expect("the read answers"),
        None,
        "the Job's own model choice is a different column"
    );

    store
        .set_review_model_override(&job, None)
        .expect("the choice is cleared");
    assert_eq!(
        store.review_model_override(&job).expect("the read answers"),
        None
    );
}

#[test]
fn a_review_model_for_a_job_that_is_not_there_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store
        .set_review_model_override(&job_id("01NOBODY"), Some("a-model"))
        .is_err());
}
