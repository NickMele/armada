//! Commands a person always-allowed for a whole Manifest — `#836`'s table,
//! kept apart from any Job's own [`crate::allowing`] row.

use core_model::{Actor, AllowedCommand, ManifestId, Reach};

use crate::tests::{at, open, ulid, TempDir};

fn manifest_id(value: &str) -> ManifestId {
    ManifestId::carried(ulid(value))
}

fn allowed(run: &str, instant: &str) -> AllowedCommand {
    AllowedCommand {
        run: run.to_string(),
        reach: Reach::Repository,
        allowed_at: at(instant),
        by: Actor::Human,
    }
}

#[test]
fn a_manifest_nobody_allowed_anything_reads_empty() {
    let dir = TempDir::new();
    let store = open(&dir);
    assert!(store
        .repository_allowed_commands(&manifest_id("01NOBODY"))
        .expect("reads")
        .is_empty());
}

#[test]
fn an_allow_reads_back_and_is_scoped_to_its_own_manifest() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let (this, other) = (manifest_id("01THIS"), manifest_id("01OTHER"));

    store
        .allow_repository_command(&this, &allowed("gh issue view", "2026-09-13T09:00:00.000Z"))
        .expect("allowed");

    assert_eq!(
        store.repository_allowed_commands(&this).expect("reads"),
        vec![allowed("gh issue view", "2026-09-13T09:00:00.000Z")]
    );
    assert!(
        store
            .repository_allowed_commands(&other)
            .expect("reads")
            .is_empty(),
        "a neighbour Manifest's allow is not this one's"
    );
}

/// A second allow of the same rule is one row, dated by the first — the same
/// idempotence [`crate::allowing`]'s table gives a Job's own row.
#[test]
fn allowing_one_rule_twice_is_one_row_as_first_allowed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = manifest_id("01TWICE");

    store
        .allow_repository_command(&id, &allowed("cargo test", "2026-09-13T09:00:00.000Z"))
        .expect("allowed");
    store
        .allow_repository_command(&id, &allowed("cargo test", "2026-09-13T09:05:00.000Z"))
        .expect("allowed again");

    assert_eq!(
        store.repository_allowed_commands(&id).expect("reads"),
        vec![allowed("cargo test", "2026-09-13T09:00:00.000Z")]
    );
}

#[test]
fn allowed_rules_come_back_oldest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = manifest_id("01ORDERED");

    let later = allowed("cargo build", "2026-09-13T09:05:00.000Z");
    let earlier = AllowedCommand {
        by: Actor::Fleet,
        ..allowed("cargo fmt", "2026-09-13T09:01:00.000Z")
    };
    store
        .allow_repository_command(&id, &later)
        .expect("allowed");
    store
        .allow_repository_command(&id, &earlier)
        .expect("allowed");

    assert_eq!(
        store.repository_allowed_commands(&id).expect("reads"),
        vec![earlier, later]
    );
}

/// One row goes, by its exact text, and the Manifest's other allow and its
/// neighbour's stay. A second take-back finds nothing and says so.
#[test]
fn taking_back_an_allow_removes_that_row_and_no_other() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let (id, kept) = (manifest_id("01TAKEN"), manifest_id("01KEPT"));
    for run in ["cargo test", "cargo build"] {
        store
            .allow_repository_command(&id, &allowed(run, "2026-09-13T09:00:00.000Z"))
            .expect("allowed");
    }
    store
        .allow_repository_command(&kept, &allowed("cargo test", "2026-09-13T09:00:00.000Z"))
        .expect("allowed");

    assert!(!store
        .remove_repository_allowed_command(&id, "cargo")
        .expect("reads"));
    assert!(store
        .remove_repository_allowed_command(&id, "cargo test")
        .expect("removed"));

    let left: Vec<String> = store
        .repository_allowed_commands(&id)
        .expect("reads")
        .into_iter()
        .map(|allow| allow.run)
        .collect();
    assert_eq!(left, vec!["cargo build"]);
    assert_eq!(
        store
            .repository_allowed_commands(&kept)
            .expect("reads")
            .len(),
        1
    );
    assert!(
        !store
            .remove_repository_allowed_command(&id, "cargo test")
            .expect("reads"),
        "already gone"
    );
}
