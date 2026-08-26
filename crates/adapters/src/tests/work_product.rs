//! What the diff says a Job has produced, asked of a real repository.
//!
//! These need git's own opinion — what a merge base is, whether a commit inside
//! a linked worktree is visible from the repository it belongs to, what
//! libgit2 counts as untracked. A fake here would assert this crate's guess at
//! each of those, and the guess about untracked directories has already been
//! wrong once (see [`repo::TempRepo::status_via_the_library`]).

use adapter_traits::{Vcs, WorkProduct, WorktreeSpec};

use crate::tests::repo::TempRepo;
use crate::worktree::GitVcs;

const JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2A3B4";

/// A repository with one commit and a Job worktree cut from it.
fn worktree_for(repo: &TempRepo) -> adapter_traits::Worktree {
    let spec = WorktreeSpec::for_job(&repo.root_str(), JOB).expect("a legal spec");
    GitVcs::new().create_worktree(&spec).expect("a worktree")
}

#[test]
fn a_fresh_worktree_has_produced_nothing() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);

    let changed = GitVcs::new().changed_files(&worktree).expect("a reading");
    assert!(
        changed.is_empty(),
        "a worktree nobody has worked in reported {:?}",
        changed.paths()
    );
}

#[test]
fn an_uncommitted_edit_counts() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    std::fs::write(format!("{}/answer.txt", worktree.path()), "42").expect("the file");

    let changed = GitVcs::new().changed_files(&worktree).expect("a reading");
    assert_eq!(changed.paths(), ["answer.txt"]);
}

/// The case a diff against the worktree's own HEAD gets wrong: a Drone that
/// committed its work has produced the most, and would read as having produced
/// nothing.
#[test]
fn work_the_drone_committed_still_counts() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    std::fs::write(format!("{}/answer.txt", worktree.path()), "42").expect("the file");
    commit_in(worktree.path(), "the drone's work");

    let changed = GitVcs::new().changed_files(&worktree).expect("a reading");
    assert_eq!(changed.paths(), ["answer.txt"]);
}

#[test]
fn a_deleted_file_counts() {
    let repo = TempRepo::with_a_commit();
    repo.write("doomed.txt", "here for now");
    repo.commit_everything("a file to delete");
    let worktree = worktree_for(&repo);
    std::fs::remove_file(format!("{}/doomed.txt", worktree.path())).expect("the removal");

    let changed = GitVcs::new().changed_files(&worktree).expect("a reading");
    assert_eq!(changed.paths(), ["doomed.txt"]);
}

/// The main line moving on must not make a Job look busier than it is. The base
/// is the merge base, so a commit made on the repository's own branch after the
/// worktree was cut is not the Job's work.
#[test]
fn a_commit_on_the_main_line_afterwards_is_not_this_job_s_work() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    repo.write("somebody-else.txt", "not the job's");
    repo.commit_everything("work outside the Job");

    let changed = GitVcs::new().changed_files(&worktree).expect("a reading");
    assert!(
        changed.is_empty(),
        "the Job was credited with {:?}",
        changed.paths()
    );
}

#[test]
fn a_worktree_that_is_not_there_is_an_error_and_not_an_empty_diff() {
    let missing = adapter_traits::Worktree::at("/armada-no-such-worktree", "armada/nope");
    let read = GitVcs::new().changed_files(&missing);
    assert!(
        read.is_err(),
        "an unreadable worktree answered {:?}",
        read.map(|changed| changed.paths().to_vec())
    );
}

/// Commit whatever is in a worktree, from inside that worktree.
fn commit_in(path: &str, message: &str) {
    let repo = git2::Repository::open(path).expect("the worktree repository");
    let mut index = repo.index().expect("the index");
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .expect("staged everything");
    index.write().expect("the index written");
    let tree = repo
        .find_tree(index.write_tree().expect("a tree"))
        .expect("the tree");
    let who = git2::Signature::now("armada", "armada@example.invalid").expect("a signature");
    let parent = repo
        .head()
        .and_then(|head| head.peel_to_commit())
        .expect("a parent commit");
    repo.commit(Some("HEAD"), &who, &who, message, &tree, &[&parent])
        .expect("a commit");
}
