//! Worktree creation, against real git.

use adapter_traits::{Vcs, WorktreeSpec};
use git2::BranchType;

use super::repo::TempRepo;
use crate::error::CreateWorktreeError;
use crate::worktree::GitVcs;

/// A plausible Job id: the shape a ULID has, which is what the record holds.
const JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2A3B4";
const OTHER_JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2C5D6";

fn spec_for(repo: &TempRepo, job: &str) -> WorktreeSpec {
    WorktreeSpec::for_job(&repo.root_str(), job).expect("a legal spec")
}

// ---------------------------------------------------------------- creation

#[test]
fn a_worktree_lands_where_the_architecture_says_and_on_its_own_branch() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);

    let made = GitVcs::new().create_worktree(&spec).expect("a worktree");

    assert_eq!(
        made.path(),
        repo.root()
            .join(".armada/worktrees")
            .join(JOB)
            .to_string_lossy()
    );
    assert_eq!(made.branch(), format!("armada/{JOB}"));
    assert!(std::path::Path::new(made.path()).join(".git").exists());

    // The worktree's own HEAD, not the outer checkout's.
    let inside = git2::Repository::open(made.path()).expect("the worktree's repository");
    assert_eq!(
        inside.head().expect("a head").shorthand(),
        Some(format!("armada/{JOB}").as_str())
    );
}

#[test]
fn the_parent_directory_is_created_when_it_is_not_there() {
    let repo = TempRepo::with_a_commit();
    assert!(!repo.root().join(".armada").exists());

    GitVcs::new()
        .create_worktree(&spec_for(&repo, JOB))
        .expect("a worktree");

    assert!(repo.root().join(".armada/worktrees").is_dir());
}

// -------------------------------------------------------------- collisions

#[test]
fn an_existing_branch_is_refused_rather_than_reused() {
    let repo = TempRepo::with_a_commit();
    let open = repo.open();
    let head = open.head().unwrap().peel_to_commit().unwrap();
    open.branch(&format!("armada/{JOB}"), &head, false)
        .expect("a branch somebody else made");

    let refused = GitVcs::new()
        .create_worktree(&spec_for(&repo, JOB))
        .expect_err("the branch is taken");

    assert!(
        matches!(refused, CreateWorktreeError::BranchExists { ref branch, .. } if branch == &format!("armada/{JOB}"))
    );
    // **Nothing was created.** The probe runs before anything lands, which is
    // the whole difference from v1, where the refusal arrived from a half-done
    // `git worktree add`.
    assert!(!repo.root().join(".armada").exists());
}

#[test]
fn a_refused_branch_names_the_branch_and_both_ways_out() {
    let repo = TempRepo::with_a_commit();
    let open = repo.open();
    let head = open.head().unwrap().peel_to_commit().unwrap();
    open.branch(&format!("armada/{JOB}"), &head, false).unwrap();

    let said = GitVcs::new()
        .create_worktree(&spec_for(&repo, JOB))
        .expect_err("the branch is taken")
        .to_string();

    assert!(said.contains(&format!("armada/{JOB}")), "{said}");
    assert!(said.contains("new id"), "{said}");
    assert!(said.contains("git branch -D"), "{said}");
}

#[test]
fn a_collision_and_a_disk_error_are_told_apart_without_reading_prose() {
    let repo = TempRepo::with_a_commit();
    let open = repo.open();
    let head = open.head().unwrap().peel_to_commit().unwrap();
    open.branch(&format!("armada/{JOB}"), &head, false).unwrap();

    let collision = GitVcs::new()
        .create_worktree(&spec_for(&repo, JOB))
        .expect_err("the branch is taken");
    assert!(collision.is_a_collision());

    let missing = WorktreeSpec::for_job("/no/such/repository", JOB).unwrap();
    let fault = GitVcs::new()
        .create_worktree(&missing)
        .expect_err("there is no repository there");
    assert!(!fault.is_a_collision());
    // The cause is carried, not formatted into the message.
    assert!(std::error::Error::source(&fault).is_some());
}

#[test]
fn a_directory_git_did_not_put_there_is_refused_before_a_branch_is_made() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    std::fs::create_dir_all(spec.worktree_path()).unwrap();
    std::fs::write(format!("{}/notes.txt", spec.worktree_path()), "mine").unwrap();

    let refused = GitVcs::new()
        .create_worktree(&spec)
        .expect_err("something is already there");

    assert!(matches!(
        refused,
        CreateWorktreeError::PathOccupied { entries: 1, .. }
    ));
    assert!(repo
        .open()
        .find_branch(&spec.branch(), BranchType::Local)
        .is_err());
}

#[test]
fn a_live_worktree_for_the_same_job_is_refused() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");

    let refused = GitVcs::new()
        .create_worktree(&spec)
        .expect_err("it is already there");

    // The branch is what is met first, and that is the more useful sentence.
    assert!(refused.is_a_collision());
}

// ------------------------------------------------- the stale registration

/// The case M1 creates for itself: **there is no cleanup, so a person removes a
/// worktree with `rm -rf`** — and git's administrative record under
/// `.git/worktrees/<name>` outlives the directory.
#[test]
fn a_stale_registration_does_not_block_a_new_worktree_at_the_same_path() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");

    hand_delete(&repo, &spec);

    // The record is still there. This is the precondition, and asserting it is
    // what stops this test passing for the wrong reason.
    assert!(repo.open().find_worktree(spec.registration_name()).is_ok());

    let again = GitVcs::new()
        .create_worktree(&spec)
        .expect("the stale record was cleared");
    assert_eq!(again.branch(), format!("armada/{JOB}"));
    assert!(std::path::Path::new(again.path()).join(".git").exists());
}

/// The same state, with git asked directly. **Without the prune this fails**,
/// which is what makes the test above worth having.
#[test]
fn git_by_itself_refuses_the_same_path_while_the_record_is_there() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");
    hand_delete(&repo, &spec);

    let open = repo.open();
    let head = open.head().unwrap().peel_to_commit().unwrap();
    let branch = open.branch(&spec.branch(), &head, false).unwrap();
    let reference = branch.into_reference();
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&reference));

    let refused = open.worktree(
        spec.registration_name(),
        std::path::Path::new(&spec.worktree_path()),
        Some(&options),
    );
    assert!(
        refused.is_err(),
        "git accepted the path with a stale record still registered — the \
         prune in create_worktree would then be testing nothing"
    );
}

/// What a person's hand cleanup leaves behind.
///
/// The directory goes, and the branch reference is deleted outright — git will
/// not delete the branch for them while the record still claims it is checked
/// out, which is the same trap from the other end.
fn hand_delete(repo: &TempRepo, spec: &WorktreeSpec) {
    std::fs::remove_dir_all(spec.worktree_path()).expect("the directory removed by hand");
    repo.open()
        .find_reference(&format!("refs/heads/{}", spec.branch()))
        .expect("the branch")
        .delete()
        .expect("the branch removed by hand");
}

// ---------------------------------------------------------------- survival

/// **Nothing here removes a worktree**, and the trait offers no method that
/// could. Two Jobs' worktrees coexist and the first is untouched by the second.
#[test]
fn a_worktree_survives_everything_that_happens_after_it() {
    let repo = TempRepo::with_a_commit();
    let vcs = GitVcs::new();

    let first = vcs
        .create_worktree(&spec_for(&repo, JOB))
        .expect("a worktree");
    let second = vcs
        .create_worktree(&spec_for(&repo, OTHER_JOB))
        .expect("a second worktree");

    // A failure for a third Job, after both exist, changes neither.
    let taken = spec_for(&repo, JOB);
    assert!(vcs.create_worktree(&taken).is_err());

    for made in [&first, &second] {
        assert!(std::path::Path::new(made.path()).join(".git").exists());
    }
    let mut names = repo
        .open()
        .worktrees()
        .expect("the registrations")
        .iter()
        .flatten()
        .map(str::to_string)
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(names, vec![JOB.to_string(), OTHER_JOB.to_string()]);
}

// --------------------------------------------------- the nesting, and its cost

/// The mitigation the architecture accepts for putting worktrees inside the
/// repo: `.armada/` is added to `.gitignore` during Manifest setup. v1 never
/// faced this, so there is no v1 evidence that it is sufficient.
#[test]
fn the_outer_checkout_does_not_see_an_ignored_worktree() {
    let repo = TempRepo::with_a_commit();
    repo.write(".gitignore", ".armada/\n");
    repo.commit_everything("ignore armada's own directory");
    assert!(repo.status().is_empty(), "the repository starts clean");

    GitVcs::new()
        .create_worktree(&spec_for(&repo, JOB))
        .expect("a worktree");

    assert_eq!(repo.status(), Vec::<String>::new());
}

/// The other half, stated rather than assumed: **without the ignore the outer
/// checkout does see it.** The cost is real, and this is what it looks like.
///
/// It also pins the disagreement between the command line and the library. The
/// library reports nothing here, so a guard written against the library's
/// untracked set would have concluded the nesting cost does not exist.
#[test]
fn without_the_ignore_the_outer_checkout_sees_the_worktree() {
    let repo = TempRepo::with_a_commit();
    GitVcs::new()
        .create_worktree(&spec_for(&repo, JOB))
        .expect("a worktree");

    assert!(
        repo.status().iter().any(|path| path.starts_with(".armada")),
        "status was {:?}",
        repo.status()
    );
    assert_eq!(
        repo.status_via_the_library(),
        Vec::<String>::new(),
        "the library has started reporting nested worktrees — a guard written \
         against its untracked set can now be trusted for this, and could not \
         before"
    );
}

// ------------------------------------------------------ the repository itself

#[test]
fn a_repository_with_no_commit_is_refused_by_name() {
    let repo = TempRepo::empty();

    let refused = GitVcs::new()
        .create_worktree(&spec_for(&repo, JOB))
        .expect_err("there is nothing to branch from");

    assert!(matches!(
        refused,
        CreateWorktreeError::NoCommitToBranchFrom { .. }
    ));
}

#[test]
fn a_path_that_is_not_a_repository_is_refused_by_name() {
    let refused = GitVcs::new()
        .create_worktree(&WorktreeSpec::for_job("/no/such/repository", JOB).unwrap())
        .expect_err("there is no repository there");

    assert!(matches!(
        refused,
        CreateWorktreeError::RepoUnreadable { .. }
    ));
}
