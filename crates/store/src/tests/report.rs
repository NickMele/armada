//! A report survives the Job it is about, which is the whole claim.
//!
//! Every other record beneath a Job is deliberately taken away with it —
//! `forget.rs` is the test that it is, and the bug it was written against was
//! rows that would *not* go. This is the one table pointed the other way, and
//! the two tests here are the two halves of that: the Job goes and the report
//! stays, and the report is still whole afterwards rather than a row with an
//! id in it.

use core_model::{Actor, CriterionId, StepId, Target};

use crate::schema::tables_pointing_at_a_job;
use crate::tests::{at, job_id, open, top_level, TempDir};
use crate::{Report, Store};

fn a_job_with_a_history(store: &mut Store, id: &str) {
    let job = top_level(id);
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
    let moved = job
        .transition(Target::Queued, Actor::Human, at("2026-08-26T10:00:00.000Z"))
        .expect("approval is a legal move");
    store
        .record_transition(&moved)
        .expect("the transition is recorded");
}

/// What the button files: a criterion the person says was judged wrongly, their
/// sentence, and the Job's record rendered at that moment.
fn about(job: &str) -> Report {
    Report {
        report_id: "01REPORT0000000000000001".to_string(),
        filed_at: at("2026-08-28T21:00:00.000Z"),
        origin: "human".to_string(),
        claim: "wrongly_refused".to_string(),
        job_id: job_id(job),
        job_title: "clean up terminal jobs from the bridge list".to_string(),
        step_id: Some(StepId::new("implement")),
        criterion_id: Some(CriterionId::new("no_behaviour_beyond_scope")),
        said: "the judge quoted a sentence that is in no scope note and in no \
               submission, and refused on it"
            .to_string(),
        record: "## Every move it made\n- stopped, gate_failure\n".to_string(),
    }
}

#[test]
fn a_report_outlives_the_job_it_is_about() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01WRONGLYREFUSED");
    store
        .record_report(&about("01WRONGLYREFUSED"))
        .expect("the report is filed");

    let gone = store
        .forget_job(&job_id("01WRONGLYREFUSED"))
        .expect("the job is forgotten");

    assert!(gone.existed);
    assert_eq!(
        gone.other, 0,
        "the report is not a row beneath the job, so forgetting it removes none"
    );
    let kept = store.reports().expect("the reports read");
    assert_eq!(kept, vec![about("01WRONGLYREFUSED")]);
    assert_eq!(
        kept[0].said,
        about("01WRONGLYREFUSED").said,
        "the finding survives, not only the row"
    );
}

/// The structural half of the test above: nothing in the schema joins a report
/// to a Job, so the delete derived from the catalog cannot reach one.
#[test]
fn no_foreign_key_carries_a_report_into_a_forgotten_job() {
    let dir = TempDir::new();
    let store = open(&dir);

    let tables = tables_pointing_at_a_job(&store.conn).expect("the catalog answers");

    assert!(
        !tables.iter().any(|table| table == "reports"),
        "reports points at jobs, so `armada clean` would take the reports with it: {tables:?}"
    );
}

/// Reading is not editing. A second report about the same Job is a second
/// thing a person said, and neither one replaces the other.
#[test]
fn two_reports_about_one_job_are_two_records_newest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01TWICE");
    store.record_report(&about("01TWICE")).expect("the first");
    let mut second = about("01TWICE");
    second.report_id = "01REPORT0000000000000002".to_string();
    second.claim = "armada_misbehaved".to_string();
    second.said = "and the diff it judged was empty".to_string();
    store.record_report(&second).expect("the second");

    let filed = store.reports().expect("the reports read");

    assert_eq!(filed.len(), 2);
    assert_eq!(filed[0].report_id, second.report_id, "newest first");
    assert_eq!(
        store.reports_by_claim().expect("the claims count"),
        vec![
            ("armada_misbehaved".to_string(), 1),
            ("wrongly_refused".to_string(), 1)
        ],
        "the count groups on the closed set, never on the sentence"
    );
}

/// A report about the whole Job carries no criterion, and comes back carrying
/// none — the scope is a pair or it is absent.
#[test]
fn a_report_with_no_criterion_scope_round_trips_as_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job_with_a_history(&mut store, "01WHOLEJOB");
    let mut whole = about("01WHOLEJOB");
    whole.step_id = None;
    whole.criterion_id = None;
    store.record_report(&whole).expect("the report is filed");

    let filed = store.reports().expect("the reports read");

    assert_eq!(filed, vec![whole]);
}

/// The count the calibration reads. Every refusal, over every Job — and it is
/// the refusals, not the passes: `met` is the Judge declining to object.
#[test]
fn refusals_are_counted_and_passes_are_not() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = crate::tests::attempt::on_its_first_run(&mut store, "01JUDGED");
    crate::tests::attempt::record_a_whole_run(
        &mut store,
        job.id().as_str(),
        "the tests still fail",
        "2026-08-26T10:03:00.000Z",
    );

    let refused = store.refusals_recorded().expect("the count reads");

    assert_eq!(refused, 1, "the fixture's one `not_met`");
}
