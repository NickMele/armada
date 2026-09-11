//! A run's snapshot, against a real repository: what git sees is the subject.

use std::path::Path;

use adapter_traits::Change;
use git2::{Repository, Signature, Time};

use crate::snapshot::{forget, last_touched, snapshot, undo, NotUndone, SnapshotError, PREFIX};
use crate::tests::repo::TempRepo;

const AT: i64 = 1_790_000_000;

fn write(root: &Path, path: &str, text: &str) {
    let target = root.join(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).expect("a directory");
    }
    std::fs::write(target, text).expect("a file");
}

fn read(root: &Path, path: &str) -> Option<String> {
    std::fs::read_to_string(root.join(path)).ok()
}

/// A repository whose one commit holds `src/lib.rs`, with a Drone's work on
/// top of it: `src/lib.rs` edited and `notes.md` new, neither committed.
fn with_work_on_it() -> TempRepo {
    let repo = TempRepo::empty();
    write(repo.root(), "src/lib.rs", "committed\n");
    repo.commit_everything("the first commit");
    write(repo.root(), "src/lib.rs", "the drone's\n");
    write(repo.root(), "notes.md", "the drone's notes\n");
    repo
}

fn paths(changed: &[adapter_traits::ChangedFile]) -> Vec<(String, Change)> {
    changed
        .iter()
        .map(|file| (file.path().to_string(), file.change()))
        .collect()
}

/// **The claim.** A run's changes are named, and undoing them puts back the
/// Drone's uncommitted work exactly — not the commit the branch started from.
#[test]
fn undo_puts_back_the_work_that_was_there_and_not_the_last_commit() {
    let repo = with_work_on_it();
    let root = repo.root();
    let git = Repository::open(root).expect("the repository");
    let head = git.head().expect("a head").target();

    let taken = snapshot(root, "01RUN", AT).expect("a snapshot");
    write(root, "src/lib.rs", "formatted\n");
    write(root, "generated.rs", "made by the run\n");
    std::fs::remove_file(root.join("notes.md")).expect("the run deleted a file");
    let settled = taken.settle(AT + 5).expect("settled");

    assert_eq!(settled.reference, format!("{PREFIX}01RUN"));
    assert_eq!(
        paths(&settled.changed),
        vec![
            ("generated.rs".to_string(), Change::Added),
            ("notes.md".to_string(), Change::Deleted),
            ("src/lib.rs".to_string(), Change::Modified),
        ]
    );

    let restored = undo(root, &settled.reference).expect("undone");
    assert_eq!(restored.len(), 3);
    assert_eq!(read(root, "src/lib.rs").as_deref(), Some("the drone's\n"));
    assert_eq!(read(root, "notes.md").as_deref(), Some("the drone's notes\n"));
    assert_eq!(read(root, "generated.rs"), None);
    assert_eq!(
        git.head().expect("a head").target(),
        head,
        "nothing was committed onto the branch"
    );
    let statuses = git.statuses(None).expect("a status");
    assert!(
        statuses.iter().all(|entry| !entry.status().is_index_new()),
        "nothing was staged in the worktree's own index"
    );
}

/// A later edit to a path the run changed is somebody's work, and Undo will
/// not write the older version over it.
#[test]
fn undo_refuses_a_path_that_moved_after_the_run() {
    let repo = with_work_on_it();
    let root = repo.root();
    let taken = snapshot(root, "01RUN", AT).expect("a snapshot");
    write(root, "src/lib.rs", "formatted\n");
    let settled = taken.settle(AT + 5).expect("settled");
    write(root, "src/lib.rs", "edited again after\n");

    let refused = undo(root, &settled.reference);
    let Err(NotUndone::Moved { paths }) = refused else {
        panic!("undone over a later edit: {refused:?}");
    };
    assert_eq!(paths, vec!["src/lib.rs".to_string()]);
    assert_eq!(
        read(root, "src/lib.rs").as_deref(),
        Some("edited again after\n"),
        "nothing was written"
    );
}

#[test]
fn a_run_that_writes_nothing_changed_nothing() {
    let repo = with_work_on_it();
    let taken = snapshot(repo.root(), "01RUN", AT).expect("a snapshot");
    let settled = taken.settle(AT + 1).expect("settled");
    assert!(settled.changed.is_empty());
}

/// A name off a record on disk reaches only a ref this module wrote.
#[test]
fn a_reference_outside_the_namespace_is_no_snapshot() {
    let repo = with_work_on_it();
    let refused = undo(repo.root(), "refs/heads/main");
    assert!(matches!(
        refused,
        Err(NotUndone::Unreadable(SnapshotError::NoSuchSnapshot { .. }))
    ));
    assert!(snapshot(repo.root(), "../main", AT).is_err());
}

#[test]
fn a_forgotten_snapshot_cannot_be_undone() {
    let repo = with_work_on_it();
    let taken = snapshot(repo.root(), "01RUN", AT).expect("a snapshot");
    let settled = taken.settle(AT).expect("settled");
    forget(repo.root(), &settled.reference).expect("forgotten");
    assert!(matches!(
        undo(repo.root(), &settled.reference),
        Err(NotUndone::Unreadable(SnapshotError::NoSuchSnapshot { .. }))
    ));
    forget(repo.root(), &settled.reference).expect("forgetting twice is not a failure");
}

fn commit_at(git: &Repository, path: &str, text: &str, seconds: i64) {
    let root = git.workdir().expect("a working directory").to_path_buf();
    write(&root, path, text);
    let mut index = git.index().expect("an index");
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .expect("staged");
    index.write().expect("written");
    let tree = git
        .find_tree(index.write_tree().expect("a tree"))
        .expect("the tree");
    let who = Signature::new("a person", "a@person.invalid", &Time::new(seconds, 0))
        .expect("a signature");
    let parent = git.head().and_then(|head| head.peel_to_commit()).ok();
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    git.commit(Some("HEAD"), &who, &who, "a commit", &tree, &parents)
        .expect("committed");
}

/// The run sheet's header: the edit before the Job froze the file, not the
/// latest one, and not a commit that touched something else.
#[test]
fn the_last_edit_is_the_newest_one_at_or_before_the_moment_asked() {
    let repo = TempRepo::empty();
    let git = Repository::open(repo.root()).expect("the repository");
    commit_at(&git, "armada.yml", "version: 1\n", 1_000);
    commit_at(&git, "armada.yml", "version: 1\n# edited\n", 2_000);
    commit_at(&git, "README", "unrelated\n", 2_500);
    commit_at(&git, "armada.yml", "version: 1\n# after\n", 3_000);

    let found = |at| last_touched(repo.root(), "armada.yml", at).expect("a walk");
    assert_eq!(found(2_600), Some(2_000));
    assert_eq!(found(3_000), Some(3_000));
    assert_eq!(found(999), None);
    assert_eq!(
        last_touched(repo.root(), "missing.yml", 5_000).expect("a walk"),
        None
    );
}
