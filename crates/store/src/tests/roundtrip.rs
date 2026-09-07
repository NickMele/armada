//! Every field the Job row holds, written and read back — and the writes the
//! store refuses.
//!
//! The fixtures fill every `Option` and leave no array empty, because a
//! round-trip over an empty record exercises almost nothing and passes anyway.
//! That holds for the two modules below as much as for this one.
//!
//! **Split by what is being round-tripped**, at the 900 the gate refuses at.
//! The Job row and the log its `assigned_drone` folds out of are here, with the
//! refusals that keep a creation from being an update and the log from being
//! edited; [`steps`] is what a step's run produced, in the tables keyed by
//! step; [`workflow`] is the frozen declaration and the column dialect it is
//! stored in.
mod steps;
mod workflow;

use core_model::{
    Actor, Attachment, DroneId, Job, JobStatus, JobStep, RedirectWaiting, RepoPath, StepId, Target,
    Ulid, WriteTargets,
};

use crate::tests::{created_at, job_id, open, sub_dispatched, top_level, TempDir};
use crate::{LoadAllError, WriteError};

#[test]
fn a_top_level_job_survives_with_every_field_intact() {
    let dir = TempDir::new();
    let stored = top_level("01FULL");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let reopened = open(&dir);
    assert_eq!(reopened.load_job(&job_id("01FULL")).expect("loads"), stored);
}

#[test]
fn a_sub_dispatched_job_keeps_the_step_that_dispatched_it() {
    let dir = TempDir::new();
    let stored = sub_dispatched("01SUB");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let reopened = open(&dir);
    let loaded = reopened.load_job(&job_id("01SUB")).expect("loads");
    assert_eq!(loaded, stored);
    assert_eq!(loaded.status(), JobStatus::Queued, "its entry status");
    assert_eq!(
        loaded.dispatched_by().map(|by| by.step_id.as_str()),
        Some("plan")
    );
}

/// Null is not empty. Zero rows in `job_write_targets` cannot say which of the
/// two a Job means, so the Job row carries the discriminator — and this is what
/// would fail if it stopped doing so.
#[test]
fn undetermined_scope_and_determined_to_write_nothing_stay_apart() {
    let dir = TempDir::new();
    let mut store = open(&dir);

    let undetermined = with_targets("01NULLSCOPE", None);
    let nothing = with_targets("01NOTHING", Some(WriteTargets::nothing()));
    let something = with_targets(
        "01SOMETHING",
        Some(WriteTargets::of(vec![RepoPath::new(
            "crates/store/src/lib.rs",
        )])),
    );
    for job in [&undetermined, &nothing, &something] {
        store.insert_job(job, &created_at()).expect("stored");
    }
    drop(store);

    let reopened = open(&dir);
    assert!(reopened
        .load_job(&job_id("01NULLSCOPE"))
        .expect("loads")
        .write_targets()
        .is_none());
    assert_eq!(
        reopened
            .load_job(&job_id("01NOTHING"))
            .expect("loads")
            .write_targets()
            .map(|targets| targets.paths().len()),
        Some(0)
    );
    assert_eq!(
        reopened
            .load_job(&job_id("01SOMETHING"))
            .expect("loads")
            .write_targets()
            .map(|targets| targets.paths().len()),
        Some(1)
    );
}

fn with_targets(id: &str, targets: Option<WriteTargets>) -> Job {
    let mut new = crate::tests::full_new_job(id);
    new.write_targets = targets;
    Job::create_top_level(new, core_model::TopLevelOrigin::Manual, created_at())
}

/// A file handed to the Job at proposal time survives a close and reopen —
/// its own table, like `job_write_targets`, and read back the same way.
#[test]
fn an_attachment_survives_the_reopen() {
    let dir = TempDir::new();
    let mut new = crate::tests::full_new_job("01ATTACHED");
    new.attachments = vec![Attachment {
        filename: "repro.png".to_string(),
        mime_type: "image/png".to_string(),
        byte_size: 20480,
        storage_ref: "/var/armada/attachments/01ATTACHED/repro.png".to_string(),
    }];
    let stored = Job::create_top_level(new, core_model::TopLevelOrigin::Manual, created_at());

    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let reopened = open(&dir);
    let loaded = reopened.load_job(&job_id("01ATTACHED")).expect("loads");
    assert_eq!(loaded.attachments().len(), 1);
    assert_eq!(loaded.attachments()[0].filename, "repro.png");
    assert_eq!(loaded.attachments()[0].mime_type, "image/png");
    assert_eq!(loaded.attachments()[0].byte_size, 20480);
    assert_eq!(
        loaded.attachments()[0].storage_ref,
        "/var/armada/attachments/01ATTACHED/repro.png"
    );
}

#[test]
fn creation_is_not_an_update() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01TWICE");
    store.insert_job(&job, &created_at()).expect("stored");
    match store.insert_job(&job, &created_at()) {
        Err(WriteError::JobAlreadyExists { job_id }) => assert_eq!(job_id.as_str(), "01TWICE"),
        other => panic!("expected a refusal, found {other:?}"),
    }
}

#[test]
fn a_transition_against_a_job_that_was_never_stored_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let moved = top_level("01GHOST")
        .transition(Target::Queued, Actor::Human, created_at())
        .expect("a legal move");
    match store.record_transition(&moved) {
        Err(WriteError::NoSuchJob { job_id }) => assert_eq!(job_id.as_str(), "01GHOST"),
        other => panic!("expected a refusal, found {other:?}"),
    }
}

/// Never edited and never removed — enforced by the database, not by this
/// crate's discipline. There is no method here that would try either; these go
/// straight at the table to show the trigger is what stops them.
#[test]
fn the_log_refuses_to_be_edited_or_removed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01APPENDONLY");
    store.insert_job(&job, &created_at()).expect("stored");
    let moved = job
        .transition(Target::Queued, Actor::Human, created_at())
        .expect("a legal move");
    store.record_transition(&moved).expect("recorded");

    let edit = store
        .conn
        .execute("UPDATE job_events SET status_to = 'killed'", []);
    assert!(edit.is_err(), "a recorded transition is never edited");

    let remove = store.conn.execute("DELETE FROM job_events", []);
    assert!(remove.is_err(), "a recorded transition is never removed");

    assert_eq!(
        store
            .events_for(&job_id("01APPENDONLY"))
            .expect("still there")
            .len(),
        1
    );
}

#[test]
fn the_boot_read_returns_every_job() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for id in ["01ONE", "01TWO", "01THREE"] {
        store.insert_job(&top_level(id), &created_at()).expect("ok");
    }
    drop(store);

    let mut reopened = open(&dir);
    let loaded = reopened.load_all_jobs().expect("all three rebuild");
    assert_eq!(loaded.jobs.len(), 3);
    assert!(
        loaded.repaired.is_empty(),
        "nothing to repair on a clean file"
    );
}

/// The signature that makes the v1 bug unwritable: a caller cannot end up with
/// a shorter list and no error.
#[test]
fn one_unreadable_job_does_not_hide_and_does_not_take_the_others_down() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01GOOD"), &created_at())
        .expect("ok");
    store
        .insert_job(&top_level("01BAD"), &created_at())
        .expect("ok");
    store
        .conn
        .execute(
            "UPDATE jobs SET status = 'a status nobody has' WHERE job_id = '01BAD'",
            [],
        )
        .expect("scribbled on");

    match store.load_all_jobs() {
        Err(LoadAllError::SomeJobsUnreadable { loaded, failed }) => {
            assert_eq!(loaded.jobs.len(), 1, "the good one still came back");
            assert_eq!(failed.len(), 1, "and the bad one is named, not dropped");
        }
        other => panic!("expected both halves, found {other:?}"),
    }
}

/// `assigned_drone` folds, onto the step the Drone was put on. It was refused
/// on read until a Drone arriving was a row in the log, because a rebuild that
/// cannot put a value back must not quietly drop it.
#[test]
fn a_drone_arriving_and_leaving_folds_back_out_of_the_log() {
    let dir = TempDir::new();
    let stored = top_level("01WITHDRONE");
    let drone = DroneId::carried(Ulid::carried("01DRONE"));
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");

    let queued = stored
        .transition(Target::Queued, Actor::Human, created_at())
        .expect("a legal move");
    store.record_transition(&queued).expect("recorded");
    let running = queued
        .job
        .transition(Target::Running, Actor::Fleet, created_at())
        .expect("a legal move");
    store.record_transition(&running).expect("recorded");
    let step = StepId::new("reproduce");
    let arrived = running
        .job
        .drone_spawned(&step, drone.clone(), Actor::Fleet, created_at())
        .expect("nothing is on that step yet");
    store.record_drone_move(&arrived).expect("recorded");
    drop(store);

    let reopened = open(&dir);
    let loaded = reopened.load_job(&job_id("01WITHDRONE")).expect("loads");
    assert_eq!(
        loaded.assigned_drone(),
        Some(&drone),
        "the column is not read back; this came off the log"
    );
    assert_eq!(
        loaded.step(&step).and_then(JobStep::assigned_drone),
        Some(&drone),
        "and it folded onto the step it was put on, which is what lets a \
         finished Job name every Drone that worked it"
    );

    let mut store = reopened;
    let left = loaded
        .drone_exited(&step, Actor::Fleet, created_at())
        .expect("one is on that step");
    store.record_drone_move(&left).expect("recorded");
    drop(store);

    let reopened = open(&dir);
    assert_eq!(
        reopened
            .load_job(&job_id("01WITHDRONE"))
            .expect("loads")
            .assigned_drone(),
        None,
        "and a Drone that left is null again, which is what suspends the \
         liveness clock"
    );
}

/// A note with nowhere to go survives the process that took it.
///
/// **The column is the field's authority**, in the sense `branch` is: no event
/// carries a person's words, so the rebuild reads this straight back rather
/// than folding it. A Fleet that restarts between a person writing a note and a
/// Drone opening with it is the ordinary case, not an exotic one — a person may
/// take a day over a review.
#[test]
fn a_note_waiting_for_the_next_drone_survives_a_reopen() {
    let dir = TempDir::new();
    let stored = top_level("01NOTE");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");

    let waiting = stored
        .redirect_waits(RedirectWaiting::saying("name the cause, not the symptom").expect("a note"))
        .expect("nothing was waiting");
    store
        .record_redirect_waiting(&waiting)
        .expect("the note is written down");
    drop(store);

    let mut reopened = open(&dir);
    let loaded = reopened.load_job(&job_id("01NOTE")).expect("loads");
    assert_eq!(
        loaded.redirect_waiting().map(RedirectWaiting::text),
        Some("name the cause, not the symptom")
    );
    assert_eq!(loaded, waiting, "and nothing else about the Job moved");

    // Delivering it is the same write with nothing in it, which is why there is
    // one method rather than a setter and a clearer.
    reopened
        .record_redirect_waiting(&loaded.redirect_delivered())
        .expect("the note is cleared");
    drop(reopened);

    let after = open(&dir);
    assert_eq!(
        after.load_job(&job_id("01NOTE")).expect("loads"),
        stored,
        "cleared on delivery, and the record is back where it started"
    );
}

/// A Job holding an undelivered note, reinserted, still holds it. `insert_job`
/// binds every column for this reason — a rebuild that dropped one would lose
/// what a person typed.
#[test]
fn a_reinserted_job_does_not_lose_the_note_it_was_holding() {
    let dir = TempDir::new();
    let waiting = top_level("01REINSERT")
        .redirect_waits(RedirectWaiting::saying("do the writer too").expect("a note"))
        .expect("nothing was waiting");
    let mut store = open(&dir);
    store.insert_job(&waiting, &created_at()).expect("stored");
    drop(store);

    assert_eq!(
        open(&dir)
            .load_job(&job_id("01REINSERT"))
            .expect("loads")
            .redirect_waiting()
            .map(RedirectWaiting::text),
        Some("do the writer too")
    );
}

/// A note nobody could act on is refused on the way out, not carried into a
/// brief. `RedirectWaiting::saying` is what makes the blank unrepresentable, so
/// a row holding one was written by something that did not share the type.
#[test]
fn a_blank_note_in_the_column_is_refused_rather_than_rendered() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01BLANKNOTE"), &created_at())
        .expect("stored");
    store
        .conn
        .execute(
            "UPDATE jobs SET redirect_waiting = '   ' WHERE job_id = ?1",
            (job_id("01BLANKNOTE").as_str(),),
        )
        .expect("the column is written past the type");

    let refused = store.load_job(&job_id("01BLANKNOTE"));
    assert!(
        matches!(
            refused,
            Err(crate::LoadJobError::Unreadable(
                crate::RowError::MalformedColumn {
                    column: "redirect_waiting",
                    ..
                }
            ))
        ),
        "a blank note is a malformed column, not an empty block in a brief: {refused:?}"
    );
}
