//! Asking what a worktree and a branch hold, without touching either.
//!
//! **These are the five readings a sweep decides on**, so every one of them is
//! taken against real git. The case the file exists for is
//! [`an_uncommitted_file_is_seen_even_though_the_branch_is_not_ahead`]: work
//! that was written and never committed leaves a branch level with its base, so
//! every other reading here says it is safe to delete.

use adapter_traits::{Vcs, WorktreeSpec};
use git2::BranchType;

use super::repo::TempRepo;
use crate::reclaim::{standing, BranchStanding, WorktreeStanding};
use crate::worktree::GitVcs;

const JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2A3B4";

fn spec_for(repo: &TempRepo, job: &str) -> WorktreeSpec {
    WorktreeSpec::for_job(&repo.root_str(), job).expect("a legal spec")
}

/// Move `main` to the Job branch's tip, which is what merging it does when
/// nothing else has landed.
fn fast_forward_main_onto(repo: &TempRepo, spec: &WorktreeSpec) {
    let opened = repo.open();
    let tip = opened
        .find_branch(&spec.branch(), BranchType::Local)
        .expect("the branch")
        .get()
        .target()
        .expect("a tip");
    opened
        .find_reference("refs/heads/main")
        .expect("main")
        .set_target(tip, "merged")
        .expect("main moved");
}

fn commit_in(spec: &WorktreeSpec, name: &str, message: &str) {
    let at = spec.worktree_path();
    std::fs::write(std::path::Path::new(&at).join(name), "what a drone wrote\n")
        .expect("a file to commit");
    for args in [
        vec!["add", name],
        vec![
            "-c",
            "user.name=armada",
            "-c",
            "user.email=armada@example.invalid",
            "commit",
            "--quiet",
            "-m",
            message,
        ],
    ] {
        let run = std::process::Command::new("git")
            .arg("-C")
            .arg(&at)
            .args(&args)
            .output()
            .expect("git on PATH");
        assert!(run.status.success(), "git {args:?}: {run:?}");
    }
}

/// The whole of what makes a worktree provably safe, from git's side: nothing
/// uncommitted in it, and nothing on the branch the base cannot reach.
#[test]
fn a_clean_checkout_on_a_merged_branch_stands_clean_and_merged() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");

    let stands = standing(&spec, Some("main")).expect("the repository opens");

    assert_eq!(stands.worktree, WorktreeStanding::Clean);
    assert!(
        matches!(stands.branch, BranchStanding::Merged { .. }),
        "the branch was cut from main and has added nothing: {:?}",
        stands.branch
    );
    assert!(!stands.empty_handed(), "there is disk here to give back");
}

/// **The reading the other four cannot make.** A file written and never
/// committed leaves the branch exactly level with its base, so merged-ness says
/// the worktree is disposable and it is the only copy of somebody's work.
#[test]
fn an_uncommitted_file_is_seen_even_though_the_branch_is_not_ahead() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");
    std::fs::write(
        std::path::Path::new(&spec.worktree_path()).join("half-done.rs"),
        "fn main() {}\n",
    )
    .expect("a file nobody committed");

    let stands = standing(&spec, Some("main")).expect("the repository opens");

    assert_eq!(
        stands.worktree,
        WorktreeStanding::Dirty {
            files: vec!["half-done.rs".to_string()]
        },
        "the file is named, because what is lost is the decision"
    );
    assert!(
        matches!(stands.branch, BranchStanding::Merged { .. }),
        "and the branch still reads as merged, which is the trap: {:?}",
        stands.branch
    );
}

/// **Why the reading is the command line and not libgit2.** `tests/repo.rs`
/// recorded the divergence: libgit2 reads a subdirectory holding a `.git` as a
/// repository of its own and reports nothing, where `git status --porcelain`
/// reports `?? nested/`. A clone somebody has not pushed is exactly the thing a
/// sweep must not delete on a "clean" reading.
#[test]
fn a_nested_repository_nobody_has_pushed_is_not_a_clean_worktree() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");
    let nested = std::path::Path::new(&spec.worktree_path()).join("nested");
    std::fs::create_dir_all(&nested).expect("a directory");
    git2::Repository::init(&nested).expect("a repository of its own");

    let stands = standing(&spec, Some("main")).expect("the repository opens");

    let WorktreeStanding::Dirty { files } = &stands.worktree else {
        panic!("the checkout holds a repository nobody has read: {stands:?}");
    };
    assert_eq!(files, &vec!["nested/".to_string()]);
}

/// The branch's own commits are counted and the base is named, so a person can
/// be told what they would be losing rather than how much disk they would get.
#[test]
fn a_branch_the_base_cannot_reach_says_how_many_and_against_what() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");
    commit_in(&spec, "work.txt", "work nobody has taken");
    commit_in(&spec, "more.txt", "and more of it");

    let stands = standing(&spec, Some("main")).expect("the repository opens");

    let BranchStanding::Ahead { base, commits, .. } = &stands.branch else {
        panic!("two commits main has never seen: {:?}", stands.branch);
    };
    assert_eq!(base, "main");
    assert_eq!(*commits, 2);
    assert_eq!(
        stands.worktree,
        WorktreeStanding::Clean,
        "committed work leaves a clean tree — the two readings are independent"
    );
}

/// Merging is what makes a worktree disposable, and this is the transition
/// asserted rather than assumed: the same worktree, before and after.
#[test]
fn merging_the_branch_is_what_turns_ahead_into_merged() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");
    commit_in(&spec, "work.txt", "work somebody will take");
    assert!(matches!(
        standing(&spec, Some("main")).expect("read").branch,
        BranchStanding::Ahead { .. }
    ));

    fast_forward_main_onto(&repo, &spec);

    assert!(
        matches!(
            standing(&spec, Some("main")).expect("read").branch,
            BranchStanding::Merged { .. }
        ),
        "main reaches every commit on it now"
    );
}

/// A lock is a person saying not yet, and it outranks what is in the tree.
#[test]
fn a_locked_worktree_says_so_rather_than_saying_what_is_in_it() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");
    repo.open()
        .find_worktree(JOB)
        .expect("the registration")
        .lock(Some("mid-bisect"))
        .expect("locked");

    let stands = standing(&spec, Some("main")).expect("the repository opens");

    assert_eq!(
        stands.worktree,
        WorktreeStanding::Locked {
            reason: "mid-bisect".to_string()
        }
    );
}

/// A Job an earlier sweep already gave back has nothing left to give, and says
/// so in one call — which is what keeps it out of every later sweep's report.
#[test]
fn a_job_with_no_worktree_and_no_branch_is_empty_handed() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);

    let stands = standing(&spec, Some("main")).expect("the repository opens");

    assert_eq!(stands.worktree, WorktreeStanding::Absent);
    assert_eq!(stands.branch, BranchStanding::Absent);
    assert!(stands.empty_handed());
}

/// **Reading is not acting.** Everything above would be worthless if asking
/// were what took the disk.
#[test]
fn asking_removes_nothing() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");
    commit_in(&spec, "work.txt", "work nobody has taken");

    standing(&spec, Some("main")).expect("the repository opens");

    assert!(std::path::Path::new(&spec.worktree_path()).is_dir());
    assert!(repo
        .open()
        .find_branch(&spec.branch(), BranchType::Local)
        .is_ok());
}

/// A root git will not open is a refusal naming the repository, not an empty
/// answer. "Nothing to reclaim" and "nothing was looked at" mean opposite
/// things about the disk.
#[test]
fn a_repository_that_will_not_open_is_refused_and_names_itself() {
    let repo = TempRepo::empty();
    let spec = spec_for(&repo, JOB);
    std::fs::remove_dir_all(repo.root().join(".git")).expect("no repository here now");

    let refused = standing(&spec, Some("main")).expect_err("not a repository");

    assert_eq!(refused.repo, repo.root_str());
}
