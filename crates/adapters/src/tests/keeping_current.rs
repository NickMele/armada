//! Keeping a pull request's branch current against a moved base, asked of a
//! real repository. `#663`.
//!
//! # The remote is a bare repository beside the temp directory
//!
//! `crate::tests::delivery`'s own reason: a push under test needs somewhere to
//! land that is not a real account, and a bare repository on the filesystem is
//! exactly what `git push` treats a remote as.

use std::path::Path;

use adapter_traits::{CommitTime, Delivery, KeptCurrent, Vcs, Worktree, WorktreeSpec};

use crate::tests::repo::TempRepo;
use crate::worktree::GitVcs;
use crate::UnmergedWork;

const JOB: &str = "01K9CURRENCY0000000000001";

fn spec_for(repo: &TempRepo) -> WorktreeSpec {
    WorktreeSpec::for_job(&repo.root_str(), JOB).expect("a legal spec")
}

/// A Job's worktree, committed and pushed — the shape a pull request already
/// exists over by the time anything here runs.
fn a_delivered_worktree(repo: &TempRepo) -> Worktree {
    let spec = spec_for(repo);
    let worktree = GitVcs::new().create_worktree(&spec).expect("a worktree");
    std::fs::write(format!("{}/work.txt", worktree.path()), "the job's work").expect("the file");
    GitVcs::new()
        .commit_all(
            &worktree,
            "the job's work",
            CommitTime::seconds_since_epoch(1_787_734_800),
        )
        .expect("a commit");
    GitVcs::new().push(&worktree).expect("the first push");
    worktree
}

fn remote_tip(bare: &Path, branch: &str) -> String {
    std::process::Command::new("git")
        .args(["-C", &bare.to_string_lossy(), "rev-parse", branch])
        .output()
        .map(|run| String::from_utf8_lossy(&run.stdout).trim().to_string())
        .expect("git on PATH")
}

// -------------------------------------------------------------- base_tip

#[test]
fn base_tip_reads_the_local_branch_without_a_worktree() {
    let repo = TempRepo::with_a_commit();
    assert_eq!(
        GitVcs::new().base_tip(&repo.root_str(), "main"),
        Some(repo.head_str())
    );
}

#[test]
fn base_tip_of_a_branch_nobody_has_is_none() {
    let repo = TempRepo::with_a_commit();
    assert_eq!(GitVcs::new().base_tip(&repo.root_str(), "nowhere"), None);
}

// ---------------------------------------------------------- no branch

#[test]
fn a_branch_this_pull_request_derived_that_is_gone_answers_no_branch() {
    let repo = TempRepo::with_a_commit();
    let outcome = GitVcs::new().kept_current(&repo.root_str(), "01NOBRANCH00000000000001", "main");
    assert_eq!(outcome, KeptCurrent::NoBranch);
}

// --------------------------------------------------------- clean rebase

/// The ordinary case: the Job's own worktree is still there, because its
/// branch is unmerged, which is true for as long as its pull request is open.
#[test]
fn a_behind_branch_is_rebased_and_pushed_in_the_jobs_own_worktree() {
    let repo = TempRepo::with_a_commit();
    let bare = repo.with_a_bare_remote();
    repo.git(&["push", "--set-upstream", "origin", "main"]);
    let worktree = a_delivered_worktree(&repo);

    repo.commit_one("elsewhere.txt", "moved on", "something else landed");
    let onto = repo.git(&["rev-parse", "main"]);

    let outcome = GitVcs::new().kept_current(&repo.root_str(), JOB, "main");
    assert_eq!(
        outcome,
        KeptCurrent::Rebased {
            onto: onto.clone(),
            commits: 1
        }
    );
    assert!(
        Path::new(&format!("{}/elsewhere.txt", worktree.path())).exists(),
        "the worktree carries what moved on the base"
    );
    assert_eq!(
        remote_tip(&bare, &format!("armada/{JOB}")),
        repo.git(&["rev-parse", &format!("armada/{JOB}")]),
        "the rebase reached the remote"
    );
}

/// **The finished-step shape.** A Job's worktree very often does not survive
/// its own pull request — a person may reclaim it by hand while the branch
/// sits unmerged — and this is answered exactly the same way: a scratch
/// checkout attached to the same branch, torn down again once the rebase is
/// decided.
#[test]
fn a_behind_branch_is_rebased_through_a_scratch_worktree_where_its_own_is_gone() {
    let repo = TempRepo::with_a_commit();
    let bare = repo.with_a_bare_remote();
    repo.git(&["push", "--set-upstream", "origin", "main"]);
    let worktree = a_delivered_worktree(&repo);
    let spec = spec_for(&repo);

    crate::reclaim(&spec, None, UnmergedWork::Keep).expect("the disk is given back");
    assert!(
        !Path::new(worktree.path()).exists(),
        "the ordinary worktree is gone, the way #663 found it"
    );

    repo.commit_one("elsewhere.txt", "moved on", "something else landed");

    let outcome = GitVcs::new().kept_current(&repo.root_str(), JOB, "main");
    assert!(
        matches!(outcome, KeptCurrent::Rebased { commits: 1, .. }),
        "{outcome:?}"
    );
    assert!(
        !Path::new(&spec.worktree_path()).exists(),
        "the scratch checkout does not survive the call"
    );
    assert_eq!(
        remote_tip(&bare, &format!("armada/{JOB}")),
        repo.git(&["rev-parse", &format!("armada/{JOB}")]),
        "the rebase reached the remote even without a worktree to start from"
    );
}

// ------------------------------------------------------------- conflict

/// **The whole of the decision `#663` records.** A conflict is not left for
/// anybody to find markers in — the branch, on the remote and locally, is
/// exactly the branch it was before the attempt.
#[test]
fn a_conflicting_rebase_leaves_the_branch_exactly_as_it_was_and_pushes_nothing() {
    let repo = TempRepo::with_a_commit();
    let bare = repo.with_a_bare_remote();
    repo.git(&["push", "--set-upstream", "origin", "main"]);
    repo.write("shared.txt", "the original\n");
    repo.commit_everything("a file both sides touch");

    let spec = spec_for(&repo);
    let worktree = GitVcs::new().create_worktree(&spec).expect("a worktree");
    std::fs::write(
        format!("{}/shared.txt", worktree.path()),
        "what the Job wrote\n",
    )
    .expect("the file");
    GitVcs::new()
        .commit_all(
            &worktree,
            "the job's work",
            CommitTime::seconds_since_epoch(1_787_734_800),
        )
        .expect("a commit");
    GitVcs::new().push(&worktree).expect("the first push");
    let tip_before = repo.git(&["rev-parse", &format!("armada/{JOB}")]);
    let remote_before = remote_tip(&bare, &format!("armada/{JOB}"));

    repo.commit_one(
        "shared.txt",
        "what somebody else merged\n",
        "somebody else got there first",
    );

    let outcome = GitVcs::new().kept_current(&repo.root_str(), JOB, "main");
    let KeptCurrent::Conflicted { files, .. } = &outcome else {
        panic!("two edits to one file conflict: {outcome:?}");
    };
    assert_eq!(files, &["shared.txt"]);

    assert_eq!(
        repo.git(&["rev-parse", &format!("armada/{JOB}")]),
        tip_before,
        "the local branch is exactly where it was"
    );
    assert_eq!(
        remote_tip(&bare, &format!("armada/{JOB}")),
        remote_before,
        "nothing reached the remote"
    );
    assert_eq!(
        std::fs::read_to_string(format!("{}/shared.txt", worktree.path())).expect("the file"),
        "what the Job wrote\n",
        "the worktree is back to what the Job left, with no conflict markers in it"
    );
}

/// The same undoing, where the rebase had to attach a scratch worktree first.
#[test]
fn a_conflicting_rebase_through_a_scratch_worktree_leaves_the_branch_untouched() {
    let repo = TempRepo::with_a_commit();
    let bare = repo.with_a_bare_remote();
    repo.git(&["push", "--set-upstream", "origin", "main"]);
    repo.write("shared.txt", "the original\n");
    repo.commit_everything("a file both sides touch");

    let spec = spec_for(&repo);
    let worktree = GitVcs::new().create_worktree(&spec).expect("a worktree");
    std::fs::write(
        format!("{}/shared.txt", worktree.path()),
        "what the Job wrote\n",
    )
    .expect("the file");
    GitVcs::new()
        .commit_all(
            &worktree,
            "the job's work",
            CommitTime::seconds_since_epoch(1_787_734_800),
        )
        .expect("a commit");
    GitVcs::new().push(&worktree).expect("the first push");
    let tip_before = repo.git(&["rev-parse", &format!("armada/{JOB}")]);
    let remote_before = remote_tip(&bare, &format!("armada/{JOB}"));

    crate::reclaim(&spec, None, UnmergedWork::Keep).expect("the disk is given back");
    repo.commit_one(
        "shared.txt",
        "what somebody else merged\n",
        "somebody else got there first",
    );

    let outcome = GitVcs::new().kept_current(&repo.root_str(), JOB, "main");
    assert!(
        matches!(&outcome, KeptCurrent::Conflicted { files, .. } if files == &["shared.txt".to_string()]),
        "{outcome:?}"
    );
    assert_eq!(
        repo.git(&["rev-parse", &format!("armada/{JOB}")]),
        tip_before
    );
    assert_eq!(remote_tip(&bare, &format!("armada/{JOB}")), remote_before);
    assert!(
        !Path::new(&spec.worktree_path()).exists(),
        "the scratch checkout is gone whether it conflicted or not"
    );
}

// ------------------------------------------------ the reapplied stash

/// **The untested branch `#1097` found the bug in.** The Job's own commits
/// replay onto the moved base cleanly, but a change still sitting uncommitted
/// in its worktree does not come back cleanly once the base is reached — the
/// autostash's own reapply conflicts, not the rebase's replay. The branch
/// still goes back to exactly what it was, and the worktree ends holding the
/// change plainly, with no conflict markers a later commit could pick up.
#[test]
fn an_autostash_that_wont_reapply_leaves_the_branch_and_the_worktree_as_they_were() {
    let repo = TempRepo::with_a_commit();
    let bare = repo.with_a_bare_remote();
    repo.git(&["push", "--set-upstream", "origin", "main"]);
    repo.write("shared.txt", "the original\n");
    repo.commit_everything("a file both sides touch");

    let spec = spec_for(&repo);
    let worktree = GitVcs::new().create_worktree(&spec).expect("a worktree");
    // The Job's committed work, on a file the base never touches — so the
    // rebase's own replay is clean, and only the reapply is left to conflict.
    std::fs::write(format!("{}/job.txt", worktree.path()), "the job's work").expect("the file");
    GitVcs::new()
        .commit_all(
            &worktree,
            "the job's work",
            CommitTime::seconds_since_epoch(1_787_734_800),
        )
        .expect("a commit");
    GitVcs::new().push(&worktree).expect("the first push");
    let tip_before = repo.git(&["rev-parse", &format!("armada/{JOB}")]);
    let remote_before = remote_tip(&bare, &format!("armada/{JOB}"));

    // Left uncommitted in the same worktree — the shape a Job's own worktree
    // can be in before its last turn lands.
    std::fs::write(
        format!("{}/shared.txt", worktree.path()),
        "still in progress\n",
    )
    .expect("the file");

    repo.commit_one(
        "shared.txt",
        "what somebody else merged\n",
        "somebody else got there first",
    );

    let outcome = GitVcs::new().kept_current(&repo.root_str(), JOB, "main");
    let KeptCurrent::Conflicted { files, .. } = &outcome else {
        panic!("the uncommitted change conflicts with what moved on the base: {outcome:?}");
    };
    assert_eq!(files, &["shared.txt"]);

    assert_eq!(
        repo.git(&["rev-parse", &format!("armada/{JOB}")]),
        tip_before,
        "the local branch is exactly where it was"
    );
    assert_eq!(
        remote_tip(&bare, &format!("armada/{JOB}")),
        remote_before,
        "nothing reached the remote"
    );
    assert_eq!(
        std::fs::read_to_string(format!("{}/shared.txt", worktree.path())).expect("the file"),
        "still in progress\n",
        "the worktree carries its own change back plainly, with no conflict markers in it"
    );
    assert!(
        !repo
            .git(&["stash", "list", "--format=%gs"])
            .contains(&format!("armada-kept-current:armada/{JOB}")),
        "put back clean, so nothing is left stranded in the stash"
    );
}

/// **The requirement `#1097` names by name.** Another worktree's own entry
/// sits on the shared stash list, above and below this call's own — a bare
/// `git stash pop` would have taken one of them. Found by its tag instead,
/// this call's own change comes back and the other worktree's is untouched.
#[test]
fn keeping_current_never_takes_another_worktrees_stash_entry() {
    let repo = TempRepo::with_a_commit();
    let bare = repo.with_a_bare_remote();
    repo.git(&["push", "--set-upstream", "origin", "main"]);
    let worktree = a_delivered_worktree(&repo);

    // Another worktree's own change, on the shared list first.
    repo.write("foreign.txt", "another worktree's own change");
    repo.git(&["stash", "push", "-u", "-m", "somebody else's, unrelated"]);

    // Left uncommitted in the Job's own worktree — what this call must set
    // aside and put back, landing its own entry on top of the foreign one.
    std::fs::write(
        format!("{}/still-writing.txt", worktree.path()),
        "not committed yet",
    )
    .expect("the file");

    repo.commit_one("elsewhere.txt", "moved on", "something else landed");
    let onto = repo.git(&["rev-parse", "main"]);

    let outcome = GitVcs::new().kept_current(&repo.root_str(), JOB, "main");
    assert_eq!(outcome, KeptCurrent::Rebased { onto, commits: 1 });
    assert_eq!(
        std::fs::read_to_string(format!("{}/still-writing.txt", worktree.path()))
            .expect("the file"),
        "not committed yet",
        "this call's own change came back"
    );
    assert_eq!(
        remote_tip(&bare, &format!("armada/{JOB}")),
        repo.git(&["rev-parse", &format!("armada/{JOB}")]),
        "the rebase reached the remote"
    );
    let stash_list = repo.git(&["stash", "list", "--format=%gs"]);
    assert!(
        stash_list.contains("somebody else's, unrelated"),
        "the other worktree's entry is untouched: {stash_list}"
    );
    assert!(
        !stash_list.contains(&format!("armada-kept-current:armada/{JOB}")),
        "this call's own entry was dropped once applied clean: {stash_list}"
    );
}
