//! The shared base checkout, against real git.
//!
//! Every case here is one git has an opinion about — that a second worktree at
//! a checked-out branch is refused, that a detached head has no branch name,
//! that a pruned record frees a path — and none of them can be asserted against
//! a fake.

use std::path::Path;

use adapter_traits::{BaseSpec, Vcs};
use git2::BranchType;

use super::repo::TempRepo;
use crate::worktree::GitVcs;

/// The base as the shipped resolution finds it. **Not the fixture's HEAD
/// spelled out here**: `git init` names the first branch `main` or `master`
/// depending on the machine, and a suite that hard-coded either would pass on
/// one developer's laptop.
fn base_spec(repo: &TempRepo) -> BaseSpec {
    let at = GitVcs::new()
        .base_commit(&repo.root_str(), None)
        .expect("the base resolved")
        .expect("a base to resolve to");
    BaseSpec::at(&repo.root_str(), &at).expect("a legal spec")
}

#[test]
fn the_base_is_checked_out_at_the_commit_and_on_no_branch() {
    let repo = TempRepo::with_a_commit();
    let spec = base_spec(&repo);

    let made = GitVcs::new().base_checkout(&spec).expect("a base checkout");

    assert_eq!(
        made.path(),
        repo.root()
            .join(".armada/bases")
            .join(spec.commit())
            .to_string_lossy()
    );
    let inside = git2::Repository::open(made.path()).expect("the checkout's repository");
    assert!(
        inside.head_detached().expect("a head"),
        "the base checkout is on a branch, and a branch is a line of history"
    );
    assert_eq!(
        inside
            .head()
            .expect("a head")
            .peel_to_commit()
            .expect("a commit")
            .id()
            .to_string(),
        spec.commit()
    );
}

#[test]
fn nothing_is_left_behind_in_the_branch_namespace() {
    let repo = TempRepo::with_a_commit();
    let spec = base_spec(&repo);

    GitVcs::new().base_checkout(&spec).expect("a base checkout");

    let scaffold = format!("armada/base/{}", spec.commit());
    assert!(
        repo.open()
            .find_branch(&scaffold, BranchType::Local)
            .is_err(),
        "the ref the checkout was added at is still a branch"
    );
}

#[test]
fn asking_twice_for_one_commit_answers_one_checkout() {
    let repo = TempRepo::with_a_commit();
    let spec = base_spec(&repo);
    let vcs = GitVcs::new();

    let first = vcs.base_checkout(&spec).expect("a base checkout");
    let second = vcs.base_checkout(&spec).expect("the same base checkout");

    assert_eq!(first.path(), second.path());
    assert_eq!(first.commit(), second.commit());
}

#[test]
fn a_base_that_moved_reaches_a_different_checkout_rather_than_the_old_one() {
    let repo = TempRepo::with_a_commit();
    let before = base_spec(&repo);
    GitVcs::new()
        .base_checkout(&before)
        .expect("a base checkout");

    repo.commit_one("moved.txt", "the base moved", "move the base");
    let after = base_spec(&repo);

    assert_ne!(before.commit(), after.commit());
    assert_ne!(before.path(), after.path());
    let made = GitVcs::new()
        .base_checkout(&after)
        .expect("a second base checkout");
    assert!(Path::new(made.path()).join("moved.txt").exists());
}

#[test]
fn a_checkout_that_has_never_been_prepared_says_so_and_one_marked_ready_says_so() {
    let repo = TempRepo::with_a_commit();
    let spec = base_spec(&repo);
    let vcs = GitVcs::new();

    let fresh = vcs.base_checkout(&spec).expect("a base checkout");
    assert!(!fresh.prepared());

    std::fs::write(spec.ready_marker(), "").expect("the marker");
    let found = vcs.base_checkout(&spec).expect("the same base checkout");
    assert!(found.prepared());
}

#[test]
fn a_dropped_checkout_can_be_made_again_at_the_same_commit() {
    let repo = TempRepo::with_a_commit();
    let spec = base_spec(&repo);
    let vcs = GitVcs::new();
    vcs.base_checkout(&spec).expect("a base checkout");

    vcs.drop_base_checkout(&spec).expect("it comes back");
    assert!(!Path::new(&spec.path()).exists());

    // The record git kept has to have gone with it, or this is the *"missing
    // but already registered working tree"* refusal a hand deletion leaves.
    let again = vcs.base_checkout(&spec).expect("a second base checkout");
    assert!(Path::new(again.path()).join(".git").exists());
}

#[test]
fn dropping_a_checkout_that_is_not_there_is_not_a_failure() {
    let repo = TempRepo::with_a_commit();
    GitVcs::new()
        .drop_base_checkout(&base_spec(&repo))
        .expect("nothing to remove is the answer, not a fault");
}

#[test]
fn a_declared_base_that_names_no_branch_is_refused_by_name() {
    let repo = TempRepo::with_a_commit();
    let refused = GitVcs::new().base_commit(&repo.root_str(), Some("no-such-branch"));
    assert!(matches!(
        refused,
        Err(crate::error::CreateWorktreeError::RefNotFound { .. })
    ));
}

#[test]
fn a_job_worktree_and_a_base_checkout_do_not_share_a_registration() {
    let repo = TempRepo::with_a_commit();
    let spec = base_spec(&repo);
    let vcs = GitVcs::new();

    vcs.base_checkout(&spec).expect("a base checkout");
    let job = adapter_traits::WorktreeSpec::for_job(&repo.root_str(), "01K3Q4R5S6T7V8W9X0Y1Z2A3B4")
        .expect("a legal spec");
    vcs.create_worktree(&job).expect("a Job's worktree");

    assert_ne!(spec.registration_name(), job.registration_name());
}
