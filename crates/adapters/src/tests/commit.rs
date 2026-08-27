//! What git does with a finished Job's work, asked of a real repository.
//!
//! These need git's own opinion: what `add -A` picks up, whether a deletion
//! reaches the index, whether the branch a linked worktree has checked out is
//! what moves. Every crate above this one fakes the whole trait.

use adapter_traits::{CommitTime, Committed, Vcs, Worktree, WorktreeSpec};
use git2::Repository;

use crate::tests::repo::TempRepo;
use crate::worktree::GitVcs;

const JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2A3B4";
/// 2026-08-26T09:00:00Z. Handed in, so the commit a test reads back is one it
/// wrote the instant of down.
const NINE: CommitTime = CommitTime::seconds_since_epoch(1_787_734_800);

fn worktree_for(repo: &TempRepo) -> Worktree {
    let spec = WorktreeSpec::for_job(&repo.root_str(), JOB).expect("a legal spec");
    GitVcs::new().create_worktree(&spec).expect("a worktree")
}

fn wrote(worktree: &Worktree, relative: &str, contents: &str) {
    std::fs::write(format!("{}/{relative}", worktree.path()), contents).expect("the file");
}

/// The commit at the tip of the worktree's branch.
fn tip(worktree: &Worktree) -> git2::Oid {
    Repository::open(worktree.path())
        .expect("the worktree")
        .head()
        .and_then(|head| head.peel_to_commit())
        .expect("a commit")
        .id()
}

#[test]
fn the_work_lands_on_the_job_s_own_branch() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    let started_at = tip(&worktree);
    wrote(&worktree, "answer.txt", "42");

    let made = GitVcs::new()
        .commit_all(&worktree, "a subject\n\na body\n", NINE)
        .expect("a commit");

    let Committed::Made { commit } = made else {
        panic!("a changed worktree has something to commit: {made:?}");
    };
    assert_eq!(commit, tip(&worktree).to_string(), "the branch moved to it");
    assert_ne!(tip(&worktree), started_at);

    let repo = Repository::open(worktree.path()).expect("the worktree");
    let head = repo.head().expect("a head");
    assert_eq!(
        head.shorthand(),
        Some(format!("armada/{JOB}").as_str()),
        "the Job's branch, not the repository's"
    );
    let commit = repo.find_commit(tip(&worktree)).expect("the commit");
    assert_eq!(commit.message(), Some("a subject\n\na body\n"));
    assert_eq!(commit.author().name(), Some("Armada Fleet"));
    assert_eq!(
        commit.author().email(),
        Some("fleet@armada.invalid"),
        "not whoever is at the keyboard"
    );
    assert_eq!(commit.time().seconds(), NINE.seconds());
}

/// The `facts_note` shape: nothing changed, so nothing is recorded — and not an
/// empty commit, which would land on the branch a person merges.
#[test]
fn a_worktree_that_changed_nothing_is_answered_rather_than_committed() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    let started_at = tip(&worktree);

    let made = GitVcs::new()
        .commit_all(&worktree, "nothing to say", NINE)
        .expect("a reading");

    assert_eq!(made, Committed::NothingToCommit);
    assert_eq!(tip(&worktree), started_at, "the branch did not move");
}

/// A file the Drone wrote and never staged is work, and a commit missing it is
/// a commit of nothing.
#[test]
fn an_untracked_file_is_committed() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    std::fs::create_dir_all(format!("{}/new", worktree.path())).expect("the directory");
    wrote(&worktree, "new/deeper.txt", "written this step");

    GitVcs::new()
        .commit_all(&worktree, "the work", NINE)
        .expect("a commit");

    assert!(committed_paths(&worktree).contains(&"new/deeper.txt".to_string()));
}

/// A deletion is a change, and a commit that dropped it does not build.
#[test]
fn a_deleted_file_is_committed_as_a_deletion() {
    let repo = TempRepo::with_a_commit();
    repo.write("doomed.txt", "here for now");
    repo.commit_everything("something to delete");
    let worktree = worktree_for(&repo);
    std::fs::remove_file(format!("{}/doomed.txt", worktree.path())).expect("removed");

    GitVcs::new()
        .commit_all(&worktree, "the work", NINE)
        .expect("a commit");

    assert!(
        !committed_paths(&worktree).contains(&"doomed.txt".to_string()),
        "the deletion reached the index"
    );
}

/// `.gitignore` says what is not part of the repository, and a commit that
/// swept it in would carry a build directory onto a branch somebody merges.
#[test]
fn an_ignored_file_is_left_out() {
    let repo = TempRepo::with_a_commit();
    repo.write(".gitignore", "ignored.txt\n");
    repo.commit_everything("an ignore rule");
    let worktree = worktree_for(&repo);
    wrote(&worktree, "ignored.txt", "not part of the repository");
    wrote(&worktree, "kept.txt", "part of it");

    GitVcs::new()
        .commit_all(&worktree, "the work", NINE)
        .expect("a commit");

    let paths = committed_paths(&worktree);
    assert!(paths.contains(&"kept.txt".to_string()));
    assert!(!paths.contains(&"ignored.txt".to_string()));
}

/// A second commit over an unchanged worktree answers nothing, which is what
/// stops a retry stacking empty commits.
#[test]
fn committing_twice_over_the_same_work_makes_one_commit() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    wrote(&worktree, "answer.txt", "42");

    let vcs = GitVcs::new();
    vcs.commit_all(&worktree, "the work", NINE)
        .expect("a commit");
    let landed = tip(&worktree);
    let again = vcs
        .commit_all(&worktree, "the work", NINE)
        .expect("a reading");

    assert_eq!(again, Committed::NothingToCommit);
    assert_eq!(tip(&worktree), landed);
}

/// Every path in the worktree's branch tip, so a test can say what is in a
/// commit without reading a diff.
fn committed_paths(worktree: &Worktree) -> Vec<String> {
    let repo = Repository::open(worktree.path()).expect("the worktree");
    let tree = repo
        .find_commit(tip(worktree))
        .and_then(|commit| commit.tree())
        .expect("the tree");
    let mut paths = Vec::new();
    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            paths.push(format!("{dir}{}", entry.name().unwrap_or_default()));
        }
        git2::TreeWalkResult::Ok
    })
    .expect("a walk");
    paths
}
