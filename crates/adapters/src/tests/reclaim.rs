//! Giving a worktree and a branch back, against real git.
//!
//! The case this file exists for is [`an_unrelated_armada_branch_is_left_alone`]:
//! cleaning by hand with a glob over the `armada/` namespace destroyed nine
//! branches that belonged to no Job. Derivation is the fix, and this is what
//! says the fix holds.

use adapter_traits::{CommitTime, Vcs, WorktreeSpec};
use git2::BranchType;

use super::repo::TempRepo;
use crate::reclaim::{reclaim, BranchGone, UnmergedWork, WorktreeGone};
use crate::worktree::GitVcs;

const JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2A3B4";
const OTHER_JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2C5D6";

fn spec_for(repo: &TempRepo, job: &str) -> WorktreeSpec {
    WorktreeSpec::for_job(&repo.root_str(), job).expect("a legal spec")
}

fn branches(repo: &TempRepo) -> Vec<String> {
    let opened = repo.open();
    let mut found: Vec<String> = opened
        .branches(Some(BranchType::Local))
        .expect("the branches")
        .filter_map(|entry| entry.ok())
        .filter_map(|(branch, _)| branch.name().ok().flatten().map(str::to_string))
        .collect();
    found.sort();
    found
}

#[test]
fn a_jobs_worktree_and_branch_both_go() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");

    let given_back = reclaim(&spec, None, UnmergedWork::Keep).expect("the repository opens");

    assert_eq!(
        given_back.worktree,
        WorktreeGone::Removed {
            path: spec.worktree_path()
        }
    );
    assert!(matches!(given_back.branch, BranchGone::Deleted { .. },));
    assert!(!std::path::Path::new(&spec.worktree_path()).exists());
    assert!(!branches(&repo).contains(&format!("armada/{JOB}")));
}

/// **The bug this whole verb was shaped by.** A branch in the `armada/`
/// namespace that no Job derived is not this command's to delete, and the only
/// thing that makes that true is that nothing here takes a pattern.
#[test]
fn an_unrelated_armada_branch_is_left_alone() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");

    // Somebody's own branch, in the same namespace, belonging to no Job.
    {
        let opened = repo.open();
        let head = opened
            .head()
            .and_then(|head| head.peel_to_commit())
            .expect("a commit");
        opened
            .branch("armada/a-branch-somebody-is-using", &head, false)
            .expect("their branch");
    }

    // Everything but the Job's own branch, whatever this machine calls its
    // default one — a fixed `master` here would be asserting a git setting.
    let expected: Vec<String> = branches(&repo)
        .into_iter()
        .filter(|name| name != &spec.branch())
        .collect();

    reclaim(&spec, None, UnmergedWork::Keep).expect("the repository opens");

    assert_eq!(
        branches(&repo),
        expected,
        "only the Job's own branch is the Job's to delete"
    );
    assert!(expected.contains(&"armada/a-branch-somebody-is-using".to_string()));
}

/// A second Job's worktree is untouched by the first being reclaimed.
#[test]
fn another_jobs_worktree_survives() {
    let repo = TempRepo::with_a_commit();
    let mine = spec_for(&repo, JOB);
    let theirs = spec_for(&repo, OTHER_JOB);
    GitVcs::new().create_worktree(&mine).expect("a worktree");
    GitVcs::new().create_worktree(&theirs).expect("a worktree");

    reclaim(&mine, None, UnmergedWork::Keep).expect("the repository opens");

    assert!(std::path::Path::new(&theirs.worktree_path()).is_dir());
    assert!(branches(&repo).contains(&format!("armada/{OTHER_JOB}")));
}

/// The deleted branch's tip comes back, because a branch is recoverable from
/// its SHA and from nothing else.
#[test]
fn the_deleted_branchs_tip_is_reported_so_the_work_is_recoverable() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");
    let expected = repo
        .open()
        .find_branch(&spec.branch(), BranchType::Local)
        .expect("the branch")
        .get()
        .target()
        .expect("a tip")
        .to_string();

    let given_back = reclaim(&spec, None, UnmergedWork::Keep).expect("the repository opens");

    assert_eq!(
        given_back.branch,
        BranchGone::Deleted {
            branch: spec.branch(),
            tip: expected
        }
    );
}

/// The order the module doc argues for, asserted from the failure it prevents:
/// a hand `rm -rf` leaves the record, and reclaim clears it rather than
/// stalling on the branch.
#[test]
fn a_hand_removed_directory_still_leaves_a_record_and_reclaim_clears_it() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");
    std::fs::remove_dir_all(spec.worktree_path()).expect("the checkout removed by hand");

    let given_back = reclaim(&spec, None, UnmergedWork::Keep).expect("the repository opens");

    assert_eq!(
        given_back.worktree,
        WorktreeGone::RecordCleared {
            path: spec.worktree_path()
        }
    );
    assert!(matches!(given_back.branch, BranchGone::Deleted { .. }));
    assert!(
        repo.open().find_worktree(JOB).is_err(),
        "the record is gone"
    );
}

/// Nothing there is not a failure — it is the state the caller asked for.
#[test]
fn a_job_that_never_had_a_worktree_is_answered_rather_than_refused() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);

    let given_back = reclaim(&spec, None, UnmergedWork::Keep).expect("the repository opens");

    assert_eq!(
        given_back.worktree,
        WorktreeGone::Absent {
            path: spec.worktree_path()
        }
    );
    assert_eq!(
        given_back.branch,
        BranchGone::Absent {
            branch: spec.branch()
        }
    );
    assert!(!given_back.faulted());
}

/// A lock is a person saying not yet. The checkout stays, and so does the
/// branch that would otherwise be deleted out from under it.
#[test]
fn a_locked_worktree_is_left_alone_and_says_why() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    GitVcs::new().create_worktree(&spec).expect("a worktree");
    repo.open()
        .find_worktree(JOB)
        .expect("the registration")
        .lock(Some("mid-bisect"))
        .expect("locked");

    let given_back = reclaim(&spec, None, UnmergedWork::Keep).expect("the repository opens");

    assert_eq!(
        given_back.worktree,
        WorktreeGone::Locked {
            path: spec.worktree_path(),
            reason: "mid-bisect".to_string()
        }
    );
    assert!(matches!(given_back.branch, BranchGone::NotDeleted { .. }));
    assert!(std::path::Path::new(&spec.worktree_path()).is_dir());
    assert!(branches(&repo).contains(&spec.branch()));
}

// ------------------------------ a branch nobody has taken the commits from

const NINE: CommitTime = CommitTime::seconds_since_epoch(1_787_734_800);

/// Commit one file on the Job's own branch, the way Fleet does when a Job's
/// last step advances.
fn a_commit_on_the_jobs_branch(spec: &WorktreeSpec) {
    let worktree = GitVcs::new().create_worktree(spec).expect("a worktree");
    std::fs::write(format!("{}/answer.txt", worktree.path()), "42").expect("the file");
    GitVcs::new()
        .commit_all(&worktree, "the work", NINE)
        .expect("a commit");
}

/// **The loss this closes.** Fleet commits a finished Job's work, so the branch
/// is the only copy of it until somebody merges.
#[test]
fn a_branch_holding_commits_main_cannot_reach_is_kept_and_counted() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    a_commit_on_the_jobs_branch(&spec);

    let given_back = reclaim(&spec, None, UnmergedWork::Keep).expect("the repository opens");

    let BranchGone::Kept {
        branch,
        base,
        commits,
        ..
    } = &given_back.branch
    else {
        panic!("one commit is not on main: {:?}", given_back.branch);
    };
    assert_eq!(branch, &spec.branch());
    assert_eq!(base, "main");
    assert_eq!(*commits, 1);
    assert!(branches(&repo).contains(&spec.branch()));
    // The checkout is reproducible and the commit is not, so only one stays.
    assert!(!given_back.faulted());
    assert!(!std::path::Path::new(&spec.worktree_path()).exists());
}

/// Merged is deleted. The commit is on `main`, so the branch is a label.
#[test]
fn a_branch_whose_commits_are_already_on_main_is_deleted() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    a_commit_on_the_jobs_branch(&spec);
    fast_forward_main_onto(&repo, &spec);

    let given_back = reclaim(&spec, None, UnmergedWork::Keep).expect("the repository opens");

    assert!(
        matches!(given_back.branch, BranchGone::Deleted { .. }),
        "{:?}",
        given_back.branch
    );
    assert!(!branches(&repo).contains(&spec.branch()));
}

/// `--force` is the deliberate override, and it says what it costs.
#[test]
fn force_deletes_a_branch_nothing_has_taken() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    a_commit_on_the_jobs_branch(&spec);

    let given_back = reclaim(&spec, None, UnmergedWork::Delete).expect("the repository opens");

    assert!(
        matches!(given_back.branch, BranchGone::Deleted { .. }),
        "{:?}",
        given_back.branch
    );
    assert!(!branches(&repo).contains(&spec.branch()));
}

/// Unanswered is kept. A repository with no `main`, no `master` and no
/// `origin/HEAD` cannot say what a branch would be merged into, and guessing
/// costs a commit.
#[test]
fn a_repository_with_no_base_branch_keeps_the_branch_and_says_why() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    a_commit_on_the_jobs_branch(&spec);
    rename_main_to(&repo, "trunk");

    let given_back = reclaim(&spec, None, UnmergedWork::Keep).expect("the repository opens");

    let BranchGone::KeptUnanswered { why, .. } = &given_back.branch else {
        panic!(
            "no base is not a licence to delete: {:?}",
            given_back.branch
        );
    };
    assert!(why.contains("main"), "it names what it looked for: {why}");
    assert!(branches(&repo).contains(&spec.branch()));
}

/// A repository that names its own default gets its own answer, whatever it is
/// called.
#[test]
fn origin_head_names_the_base_when_the_default_is_not_main() {
    let repo = TempRepo::with_a_commit();
    let spec = spec_for(&repo, JOB);
    a_commit_on_the_jobs_branch(&spec);
    rename_main_to(&repo, "trunk");
    point_origin_head_at(&repo, "trunk");
    fast_forward(&repo, "trunk", &spec);

    let given_back = reclaim(&spec, None, UnmergedWork::Keep).expect("the repository opens");

    assert!(
        matches!(given_back.branch, BranchGone::Deleted { .. }),
        "trunk reaches it, so it is a label: {:?}",
        given_back.branch
    );
}

fn fast_forward_main_onto(repo: &TempRepo, spec: &WorktreeSpec) {
    fast_forward(repo, "main", spec);
}

/// Move `base` to the Job branch's tip, which is what merging it does when
/// nothing else has landed.
fn fast_forward(repo: &TempRepo, base: &str, spec: &WorktreeSpec) {
    let opened = repo.open();
    let tip = opened
        .find_branch(&spec.branch(), BranchType::Local)
        .expect("the branch")
        .get()
        .target()
        .expect("a tip");
    opened
        .find_reference(&format!("refs/heads/{base}"))
        .expect("the base")
        .set_target(tip, "merged")
        .expect("the base moved");
}

fn rename_main_to(repo: &TempRepo, name: &str) {
    repo.open()
        .find_branch("main", BranchType::Local)
        .expect("main")
        .rename(name, false)
        .expect("renamed");
}

fn point_origin_head_at(repo: &TempRepo, branch: &str) {
    let opened = repo.open();
    let tip = opened
        .find_branch(branch, BranchType::Local)
        .expect("the branch")
        .get()
        .target()
        .expect("a tip");
    opened
        .reference(
            &format!("refs/remotes/origin/{branch}"),
            tip,
            true,
            "the remote's copy",
        )
        .expect("a remote-tracking ref");
    opened
        .reference_symbolic(
            "refs/remotes/origin/HEAD",
            &format!("refs/remotes/origin/{branch}"),
            true,
            "the remote's default",
        )
        .expect("origin/HEAD");
}
