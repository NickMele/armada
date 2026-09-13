//! A Helm conversation's session: absent until kept, still there after a
//! reopen, one per conversation, and gone once forgotten.

use std::time::Duration;

use core_model::Timestamp;

use crate::tests::{open, TempDir};

const A_DAY: Duration = Duration::from_secs(86_400);

fn at(when: &str) -> Timestamp {
    Timestamp::from_rfc3339(when.to_string())
}

#[test]
fn a_conversation_nobody_spoke_in_resumes_nothing() {
    let dir = TempDir::new();
    let store = open(&dir);
    assert_eq!(store.helm_session("armada").expect("reads"), None);
}

#[test]
fn a_kept_session_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .keep_helm_session("armada", "4e0fb413", &at("2026-09-13T09:00:00.000Z"))
        .expect("kept");
    drop(store);

    let store = open(&dir);
    assert_eq!(
        store.helm_session("armada").expect("reads").as_deref(),
        Some("4e0fb413")
    );
}

#[test]
fn each_conversation_keeps_its_own_and_a_second_keep_replaces_the_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .keep_helm_session("armada", "first", &at("2026-09-13T09:00:00.000Z"))
        .expect("kept");
    store
        .keep_helm_session("elsewhere", "theirs", &at("2026-09-13T09:01:00.000Z"))
        .expect("kept");
    store
        .keep_helm_session("armada", "second", &at("2026-09-13T09:02:00.000Z"))
        .expect("kept");

    assert_eq!(
        store.helm_session("armada").expect("reads").as_deref(),
        Some("second")
    );
    assert_eq!(
        store.helm_session("elsewhere").expect("reads").as_deref(),
        Some("theirs")
    );
}

#[test]
fn forgetting_one_conversation_leaves_the_others() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .keep_helm_session("armada", "ours", &at("2026-09-13T09:00:00.000Z"))
        .expect("kept");
    store
        .keep_helm_session("elsewhere", "theirs", &at("2026-09-13T09:00:00.000Z"))
        .expect("kept");

    assert!(store.forget_helm_session("armada").expect("forgotten"));
    assert!(!store.forget_helm_session("armada").expect("nothing left"));
    assert_eq!(store.helm_session("armada").expect("reads"), None);
    assert_eq!(
        store.helm_session("elsewhere").expect("reads").as_deref(),
        Some("theirs")
    );
}

#[test]
fn a_session_older_than_the_window_is_swept_and_a_fresher_one_is_not() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .keep_helm_session("stale", "old", &at("2026-08-01T00:00:00.000Z"))
        .expect("kept");
    store
        .keep_helm_session("fresh", "new", &at("2026-09-12T00:00:00.000Z"))
        .expect("kept");

    let forgotten = store
        .forget_stale_helm_sessions(&at("2026-09-13T00:00:00.000Z"), A_DAY * 30)
        .expect("swept");

    assert_eq!(forgotten, 1);
    assert_eq!(store.helm_session("stale").expect("reads"), None);
    assert_eq!(
        store.helm_session("fresh").expect("reads").as_deref(),
        Some("new")
    );
}

#[test]
fn a_conversation_answered_the_same_reply_it_would_have_been_swept_on_is_kept() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .keep_helm_session("armada", "old", &at("2026-08-01T00:00:00.000Z"))
        .expect("kept");

    // The reply that would be stale enough to sweep is the one that refreshes
    // `kept_at` first — `Fleet::replied`'s order, not this store's.
    store
        .keep_helm_session("armada", "refreshed", &at("2026-09-13T00:00:00.000Z"))
        .expect("kept");
    let forgotten = store
        .forget_stale_helm_sessions(&at("2026-09-13T00:00:01.000Z"), A_DAY * 30)
        .expect("swept");

    assert_eq!(forgotten, 0);
    assert_eq!(
        store.helm_session("armada").expect("reads").as_deref(),
        Some("refreshed")
    );
}
