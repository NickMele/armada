//! What a Job's branch came to survives the process that recorded it.
//!
//! **The defect this is written against was not a wrong value, it was no
//! value.** `fleet::delivery` had committed, pushed and opened a pull request
//! for months, and put the result in a map that the next read *drained* — so
//! the Drone's closing turn consumed it and a person opening the Job afterwards
//! was told none had been opened. Everything here is about the record outliving
//! that turn.

use crate::tests::{open, top_level, TempDir};
use adapter_traits::Landing;

use crate::{Delivery, Store};

fn a_job(store: &mut Store, id: &str) {
    let job = top_level(id);
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
}

/// A commit, a push and a pull request are read back exactly as written.
#[test]
fn what_the_branch_came_to_is_read_back() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01DELIVERY000000000000001");
    let came_to = Delivery {
        commit: Some("5375d705cb7713a21a91681c1028166b98a0d6de".to_string()),
        pushed: Some("origin/armada/01DELIVERY000000000000001".to_string()),
        pull_request: Some("https://example.invalid/armada/pull/229".to_string()),
        landed: None,
    };
    store
        .record_delivery(&crate::tests::job_id("01DELIVERY000000000000001"), &came_to)
        .expect("the delivery is recorded");
    let read = store
        .delivery_for(&crate::tests::job_id("01DELIVERY000000000000001"))
        .expect("the delivery is read");
    assert_eq!(read, came_to, "every field comes back as it went in");
}

/// **A commit with no push is not a Job that pushed nothing it can name.** The
/// three fields are independent, and a repository with no remote writes the
/// first and leaves the other two absent — which a surface reads as a different
/// sentence from "nothing was recorded".
#[test]
fn a_commit_with_no_push_keeps_the_other_two_absent() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01DELIVERY000000000000002");
    store
        .record_delivery(
            &crate::tests::job_id("01DELIVERY000000000000002"),
            &Delivery {
                commit: Some("abc123".to_string()),
                pushed: Some("no remote".to_string()),
                pull_request: None,
                landed: None,
            },
        )
        .expect("the delivery is recorded");
    let read = store
        .delivery_for(&crate::tests::job_id("01DELIVERY000000000000002"))
        .expect("the delivery is read");
    assert_eq!(read.commit.as_deref(), Some("abc123"));
    assert_eq!(read.pushed.as_deref(), Some("no remote"));
    assert!(
        read.pull_request.is_none(),
        "no pull request was opened, and none is what comes back"
    );
    assert!(!read.is_empty(), "a commit is something to say");
}

/// **A Job that finished before version 21 reads as nothing to say, not as a
/// branch that came to nothing.** The surface says different sentences for the
/// two, so `is_empty` is what tells them apart and it has to be true here.
#[test]
fn a_job_that_was_never_delivered_has_nothing_to_say() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01DELIVERY000000000000003");
    let read = store
        .delivery_for(&crate::tests::job_id("01DELIVERY000000000000003"))
        .expect("the delivery is read");
    assert!(read.is_empty(), "nothing was recorded, and nothing is read");
}

/// A redispatched Job delivering again must not inherit the last run's URL,
/// which is why the write sets every field including `None`.
#[test]
fn a_second_delivery_clears_what_the_first_one_wrote() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01DELIVERY000000000000004");
    let id = crate::tests::job_id("01DELIVERY000000000000004");
    store
        .record_delivery(
            &id,
            &Delivery {
                commit: Some("first".to_string()),
                pushed: Some("origin/first".to_string()),
                pull_request: Some("https://example.invalid/pull/1".to_string()),
                landed: None,
            },
        )
        .expect("the first delivery is recorded");
    store
        .record_delivery(
            &id,
            &Delivery {
                commit: Some("second".to_string()),
                pushed: Some("origin/second".to_string()),
                pull_request: None,
                landed: None,
            },
        )
        .expect("the second delivery is recorded");
    let read = store.delivery_for(&id).expect("the delivery is read");
    assert_eq!(read.commit.as_deref(), Some("second"));
    assert!(
        read.pull_request.is_none(),
        "the second run opened none, and the first run's URL does not survive it"
    );
}

/// A merge is recorded beside the pull request it happened to, and neither the
/// commit nor the push is disturbed by it. **The two writers are separate for
/// exactly this reason**: a merge is read by a later turn than the one that
/// pushed, and one `UPDATE` for both would have that turn restate the commit.
#[test]
fn a_merge_is_recorded_without_touching_what_the_branch_came_to() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01DELIVERY000000000000006");
    let id = crate::tests::job_id("01DELIVERY000000000000006");
    store
        .record_delivery(
            &id,
            &Delivery {
                commit: Some("abc123".to_string()),
                pushed: Some("origin/armada/01DELIVERY000000000000006".to_string()),
                pull_request: Some("https://example.invalid/armada/pull/337".to_string()),
                landed: None,
            },
        )
        .expect("the delivery is recorded");

    store
        .record_landed(
            &id,
            &Landing::Merged {
                url: String::from("https://example.invalid/armada/pull/337"),
            },
        )
        .expect("the merge is recorded");

    let read = store.delivery_for(&id).expect("the delivery is read");
    assert_eq!(read.commit.as_deref(), Some("abc123"));
    assert!(read.pushed.is_some(), "the push survives the merge");
    assert!(
        matches!(read.landed, Some(Landing::Merged { .. })),
        "the record answers `did this land`"
    );
    assert!(
        store.pull_requests_unsettled().unwrap().is_empty(),
        "a settled pull request leaves the rotation"
    );
}

/// **Still open is not written down**, which is the whole shape of the column:
/// it stores an answer and never the absence of one, so a Job asked about and
/// found unchanged stays in the rotation.
#[test]
fn an_open_or_unknown_answer_is_not_written_down() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01DELIVERY000000000000007");
    let id = crate::tests::job_id("01DELIVERY000000000000007");
    let url = String::from("https://example.invalid/armada/pull/338");
    store
        .record_delivery(
            &id,
            &Delivery {
                commit: Some("abc123".to_string()),
                pushed: Some("origin/one".to_string()),
                pull_request: Some(url.clone()),
                landed: None,
            },
        )
        .expect("the delivery is recorded");

    store
        .record_landed(&id, &Landing::Open { url })
        .expect("an open pull request is not a failure to record");
    store
        .record_landed(&id, &Landing::Unknown)
        .expect("a forge that could not say is not a failure to record");

    assert!(store.delivery_for(&id).unwrap().landed.is_none());
    assert_eq!(
        store.pull_requests_unsettled().unwrap().len(),
        1,
        "the Job is still one to ask about"
    );
}

/// A Job the file does not hold is not an error: the read is spent on a Job
/// that may have been forgotten between the list and the open.
#[test]
fn a_job_that_is_not_there_reads_as_nothing() {
    let dir = TempDir::new();
    let store: Store = open(&dir);
    let read = store
        .delivery_for(&crate::tests::job_id("01DELIVERY000000000000005"))
        .expect("a missing job is not a failure");
    assert!(read.is_empty());
}
