//! **The test this step exists for.**
//!
//! A Job is created, driven through eight transitions, and every in-memory copy
//! is dropped along with the connection that wrote them. The file is reopened
//! and the same `Job` comes back — folded from `job_events`, never from the
//! status column.
//!
//! An events table that is written and never read is wrong by the time
//! something needs it, and nothing says so in the meantime. This is what says
//! so.

use core_model::{
    Actor, CriteriaOwed, CriterionId, EscalationTrigger, Job, JobStatus, PilotReason, Target,
    Timestamp, TransitionReason,
};

use crate::tests::{at, job_id, open, top_level, TempDir};
use crate::{LoadJobError, Moved, RecordedEvent, Store};

/// The reason off a Job row, or a panic — a step row carries none, and every
/// row this file writes is a Job transition.
fn reason(event: &RecordedEvent) -> TransitionReason {
    match event.moved() {
        Moved::Job { reason, .. } => reason.clone(),
        Moved::Step { .. } => panic!("this history moves no step"),
    }
}

/// The eight moves, with who caused each and when. Four of them carry a reason
/// the destination stores, which is every kind of reason there is.
fn history() -> Vec<(Target, Actor, Timestamp)> {
    vec![
        (Target::Queued, Actor::Human, at("2026-08-26T10:00:00.000Z")),
        (
            Target::Running,
            Actor::Fleet,
            at("2026-08-26T10:01:00.000Z"),
        ),
        (
            Target::AwaitingReview,
            Actor::Fleet,
            at("2026-08-26T10:02:00.000Z"),
        ),
        (
            Target::Escalated(EscalationTrigger::Interrupted),
            Actor::Fleet,
            at("2026-08-26T10:03:00.000Z"),
        ),
        (
            Target::Piloted(PilotReason::TakeOver),
            Actor::Human,
            at("2026-08-26T10:04:00.000Z"),
        ),
        (
            Target::Running,
            Actor::Human,
            at("2026-08-26T10:05:00.000Z"),
        ),
        (
            Target::AwaitingAttestation(CriteriaOwed::owing(
                CriterionId::new("c1"),
                vec![CriterionId::new("c2")],
            )),
            Actor::Fleet,
            at("2026-08-26T10:06:00.000Z"),
        ),
        (
            Target::CompletedSuccess,
            Actor::Human,
            at("2026-08-26T10:07:00.000Z"),
        ),
    ]
}

/// Drive the Job through `history`, recording each transition as it happens.
fn drive(store: &mut Store, job: Job) -> Job {
    let mut job = job;
    for (target, actor, when) in history() {
        let moved = job
            .transition(target, actor, when)
            .expect("every move in the history is one the machine admits");
        store
            .record_transition(&moved)
            .expect("the transition is recorded");
        job = moved.job;
    }
    job
}

#[test]
fn a_job_rebuilds_from_its_events_after_the_process_that_wrote_it_is_gone() {
    let dir = TempDir::new();
    let id = job_id("01RECONSTRUCT");

    let expected = {
        let mut store = open(&dir);
        let created = top_level("01RECONSTRUCT");
        store
            .insert_job(&created, &crate::tests::created_at())
            .expect("the job is stored");
        drive(&mut store, created)
        // `store` drops here: the connection closes and every in-memory Job
        // with it. Nothing below has seen the value it is about to assert on.
    };
    assert_eq!(expected.status(), JobStatus::CompletedSuccess);

    let reopened = open(&dir);
    let rebuilt = reopened.load_job(&id).expect("the job rebuilds");

    // Not just the status: the whole record, field for field.
    assert_eq!(rebuilt, expected);
}

/// The title comes back through the fold, not just through the row.
///
/// `a_job_rebuilds_from_its_events_after_the_process_that_wrote_it_is_gone`
/// asserts the whole record and therefore covers this already — it is named
/// separately because the record is the thing that grows, and a field that
/// silently stops surviving a rebuild would take a whole-record assertion down
/// with a message naming no field.
#[test]
fn a_title_survives_the_write_the_drop_the_reopen_and_the_fold() {
    let dir = TempDir::new();
    let id = job_id("01TITLED");

    {
        let mut store = open(&dir);
        let created = top_level("01TITLED");
        store
            .insert_job(&created, &crate::tests::created_at())
            .expect("the job is stored");
        drive(&mut store, created);
    }

    let reopened = open(&dir);
    let rebuilt = reopened.load_job(&id).expect("the job rebuilds");
    assert_eq!(
        rebuilt.title().as_str(),
        "fix the off-by-one in the log reader"
    );
    assert_eq!(
        rebuilt.status(),
        JobStatus::CompletedSuccess,
        "and it is the rebuilt Job, not the created one"
    );
}

#[test]
fn the_log_holds_every_transition_with_its_reason_actor_and_time() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let created = top_level("01LOG");
    store
        .insert_job(&created, &crate::tests::created_at())
        .expect("the job is stored");
    drive(&mut store, created);
    drop(store);

    let reopened = open(&dir);
    let events = reopened
        .events_for(&job_id("01LOG"))
        .expect("the events read back");

    assert_eq!(events.len(), 8, "one row per transition, and no more");
    assert_eq!(events[0].under(), JobStatus::AwaitingApproval);
    assert_eq!(events[0].actor(), Actor::Human);
    assert_eq!(events[0].at(), &at("2026-08-26T10:00:00.000Z"));
    assert_eq!(
        events[0].moved(),
        &Moved::Job {
            to: JobStatus::Queued,
            reason: TransitionReason::DerivedAtRead,
        }
    );

    assert_eq!(
        reason(&events[3]),
        TransitionReason::Escalation(EscalationTrigger::Interrupted),
        "the trigger survives, and it is the only one that edge admits"
    );
    assert_eq!(
        reason(&events[4]),
        TransitionReason::Pilot(PilotReason::TakeOver)
    );
    match reason(&events[6]) {
        TransitionReason::Attestation(owed) => {
            let ids: Vec<&str> = owed.ids().map(CriterionId::as_str).collect();
            assert_eq!(ids, vec!["c1", "c2"], "the criteria owed survive in order");
        }
        other => panic!("expected an attestation debt, found {other:?}"),
    }

    // The keys the store assigned are monotonic, and they are the fold's
    // order — the timestamps are injected and cannot be trusted to be.
    let keys: Vec<i64> = events.iter().map(|event| event.seq()).collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(keys, sorted);
}

#[test]
fn the_fold_starts_from_the_row_because_creation_is_not_a_transition() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let created = top_level("01NOEVENTS");
    store
        .insert_job(&created, &crate::tests::created_at())
        .expect("the job is stored");
    drop(store);

    let reopened = open(&dir);
    assert_eq!(
        reopened
            .events_for(&job_id("01NOEVENTS"))
            .expect("no events")
            .len(),
        0,
        "creation has no `from`, so it writes no event"
    );
    assert_eq!(
        reopened
            .load_job(&job_id("01NOEVENTS"))
            .expect("it rebuilds"),
        created,
        "a Job that has never moved still rebuilds, from its row"
    );
}

#[test]
fn a_job_that_was_never_stored_is_not_an_empty_job() {
    let dir = TempDir::new();
    let store = open(&dir);
    match store.load_job(&job_id("01ABSENT")) {
        Err(LoadJobError::NoSuchJob { job_id }) => assert_eq!(job_id.as_str(), "01ABSENT"),
        other => panic!("expected a refusal naming the id, found {other:?}"),
    }
}
