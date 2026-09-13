//! A Helm conversation's session: absent until kept, still there after a
//! reopen, one per conversation, and gone once forgotten.

use core_model::Timestamp;

use crate::tests::{open, TempDir};

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
