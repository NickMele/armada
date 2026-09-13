//! The saved Fleet limits: absent until saved, whole once saved, and still
//! there after a reopen — which is the only way to prove a restart keeps them.

use crate::tests::{open, TempDir};
use crate::SavedLimits;

#[test]
fn a_store_nobody_saved_into_holds_nothing() {
    let dir = TempDir::new();
    let store = open(&dir);
    assert_eq!(store.saved_limits().expect("reads"), SavedLimits::default());
}

#[test]
fn a_save_survives_a_reopen_and_an_unsaved_field_stays_absent() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let saved = SavedLimits {
        concurrency: Some(4),
        memory_spare_percent: None,
        disk_floor_gib: Some(20),
    };
    store.save_limits(&saved).expect("saved");
    drop(store);

    let store = open(&dir);
    assert_eq!(store.saved_limits().expect("reads"), saved);
}

/// A second save is the one row replaced, not a second row beside it.
#[test]
fn a_second_save_replaces_the_first_whole() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .save_limits(&SavedLimits {
            concurrency: Some(4),
            memory_spare_percent: Some(30),
            disk_floor_gib: Some(20),
        })
        .expect("first");
    let second = SavedLimits {
        concurrency: Some(1),
        memory_spare_percent: None,
        disk_floor_gib: Some(0),
    };
    store.save_limits(&second).expect("second");

    assert_eq!(store.saved_limits().expect("reads"), second);
}
