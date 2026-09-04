//! Delivery, asked of real repositories.
//!
//! # A bare repository beside the temp directory, never a real remote
//!
//! A push is what these are about, and a test that pushed to a real remote
//! would need a credential and would write to somebody's account. git treats a
//! bare repository on the filesystem exactly as a remote, so the push under
//! test is the push that ships.
//!
//! Opening a pull request is not exercised here: it needs a forge, an account
//! and a network, and the shape of the answer — opened, already open, nothing
//! pushed, no tool — is what every crate above this one asserts against a fake.

use adapter_traits::{
    Base, BaseOnTheRemote, BroughtUpToDate, Delivery, Pushed, Standing, Vcs, Worktree,
};
use adapter_traits::{NotDelivered, WorktreeSpec};

use crate::tests::repo::TempRepo;
use crate::worktree::GitVcs;

const JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2A3B4";

fn worktree_for(repo: &TempRepo) -> Worktree {
    let spec = WorktreeSpec::for_job(&repo.root_str(), JOB).expect("a legal spec");
    GitVcs::new().create_worktree(&spec).expect("a worktree")
}

/// Move `main` on under the Job, the way a person merging something else does.
fn main_moves_on(repo: &TempRepo, file: &str, contents: &str) {
    repo.commit_one(file, contents, "something else landed");
}

fn wrote(worktree: &Worktree, relative: &str, contents: &str) {
    std::fs::write(format!("{}/{relative}", worktree.path()), contents).expect("the file");
}

fn read(worktree: &Worktree, relative: &str) -> String {
    std::fs::read_to_string(format!("{}/{relative}", worktree.path())).expect("the file")
}

// ------------------------------------------------------------- the base

#[test]
fn nothing_declared_infers_the_conventional_branch() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    assert_eq!(
        GitVcs::new().base(&worktree, None).expect("a reading"),
        Some(Base::Inferred(String::from("main")))
    );
}

#[test]
fn the_declared_key_overrides_what_would_have_been_inferred() {
    let repo = TempRepo::with_a_commit();
    repo.git(&["branch", "release"]);
    let worktree = worktree_for(&repo);

    // `main` is here and would have been inferred. The key wins, and the answer
    // says it was declared rather than read.
    let base = GitVcs::new()
        .base(&worktree, Some("release"))
        .expect("a reading");
    assert_eq!(base, Some(Base::Declared(String::from("release"))));
    assert!(base.expect("a base").was_declared());
}

#[test]
fn a_declared_branch_that_is_not_there_is_refused_rather_than_guessed_past() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    let refused = GitVcs::new()
        .base(&worktree, Some("nowhere"))
        .expect_err("a base the repository has not got");
    let NotDelivered { said, .. } = &refused;
    assert!(said.contains("nowhere"), "{said}");
    assert!(
        said.contains("armada.yml"),
        "the refusal names the file to fix: {said}"
    );
}

// ---------------------------------------------------------- the standing

#[test]
fn a_branch_nothing_has_moved_under_is_up_to_date() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    let base = Base::Inferred(String::from("main"));
    assert_eq!(
        GitVcs::new().standing(&worktree, &base).expect("a reading"),
        Standing::UpToDate
    );
}

#[test]
fn the_standing_counts_what_the_base_has_and_the_branch_has_not() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    main_moves_on(&repo, "elsewhere.txt", "one");
    main_moves_on(&repo, "elsewhere.txt", "two");

    let base = Base::Inferred(String::from("main"));
    assert_eq!(
        GitVcs::new().standing(&worktree, &base).expect("a reading"),
        Standing::Behind { commits: 2 }
    );
}

// -------------------------------------------------------- catching up

#[test]
fn a_behind_branch_holding_uncommitted_work_keeps_it_across_the_rebase() {
    // The mid-Job shape: Fleet commits only at the last step, so the branch has
    // no commits of its own and the worktree is full of the Drone's changes.
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    wrote(&worktree, "the-drone-was-here.txt", "half done");
    main_moves_on(&repo, "elsewhere.txt", "one");

    let base = Base::Inferred(String::from("main"));
    let moved = GitVcs::new()
        .bring_up_to_date(&worktree, &base)
        .expect("a rebase");

    assert_eq!(
        moved,
        BroughtUpToDate::Clean {
            base: String::from("main"),
            commits: 1
        }
    );
    assert_eq!(
        read(&worktree, "the-drone-was-here.txt"),
        "half done",
        "the uncommitted work came across — destroying it is the failure this guards"
    );
    assert_eq!(
        read(&worktree, "elsewhere.txt"),
        "one",
        "and what moved on the base is now in the worktree"
    );
}

#[test]
fn work_that_will_not_come_back_across_is_left_as_a_conflict_to_resolve() {
    let repo = TempRepo::with_a_commit();
    repo.write("shared.txt", "the original\n");
    repo.commit_everything("a file both sides touch");

    let worktree = worktree_for(&repo);
    wrote(&worktree, "shared.txt", "what the Drone wrote\n");
    repo.commit_one(
        "shared.txt",
        "what somebody else merged\n",
        "somebody else got there first",
    );

    let base = Base::Inferred(String::from("main"));
    let moved = GitVcs::new()
        .bring_up_to_date(&worktree, &base)
        .expect("a rebase that conflicts is still an answer");

    let BroughtUpToDate::Conflicted { files, .. } = &moved else {
        panic!("two edits to one file conflict: {moved:?}");
    };
    assert_eq!(files, &["shared.txt"], "the conflict names the file");
    assert!(
        read(&worktree, "shared.txt").contains("what the Drone wrote"),
        "nothing was destroyed — the Drone's line is still in the file"
    );
}

#[test]
fn a_committed_branch_that_will_not_replay_is_put_back_exactly_as_it_was() {
    // The last-step shape: Fleet has committed, so the branch has a commit of
    // its own to replay, and replaying it is what conflicts.
    let repo = TempRepo::with_a_commit();
    repo.write("shared.txt", "the original\n");
    repo.commit_everything("a file both sides touch");

    let worktree = worktree_for(&repo);
    wrote(&worktree, "shared.txt", "what the Drone wrote\n");
    GitVcs::new()
        .commit_all(
            &worktree,
            "the Job's work",
            adapter_traits::CommitTime::seconds_since_epoch(1_787_734_800),
        )
        .expect("a commit");
    let tip_before = repo.git(&["rev-parse", &format!("armada/{JOB}")]);

    repo.commit_one(
        "shared.txt",
        "what somebody else merged\n",
        "somebody else got there first",
    );

    let base = Base::Inferred(String::from("main"));
    let moved = GitVcs::new()
        .bring_up_to_date(&worktree, &base)
        .expect("a rebase that conflicts is still an answer");

    let BroughtUpToDate::PutBack { files, .. } = &moved else {
        panic!("a replayed commit that conflicts puts the branch back: {moved:?}");
    };
    assert_eq!(files, &["shared.txt"]);
    assert_eq!(
        repo.git(&["rev-parse", &format!("armada/{JOB}")]),
        tip_before,
        "the branch is exactly where it was — nothing is half-rebased"
    );
    assert!(
        repo.git(&["status", "--porcelain"]).is_empty()
            || !std::path::Path::new(&format!("{}/.git", worktree.path())).is_dir(),
        "and no rebase is left in progress"
    );
}

// ------------------------------------------------------------- the push

// ------------------------------------- the base against the remote's

/// The defect this exists for: `main` on the machine holds commits `origin/main`
/// does not, so a pull request opened against `origin/main` carries every one of
/// them under a Job that never touched their files.
#[test]
fn a_base_ahead_of_its_remote_is_counted_and_the_tracking_branch_named() {
    let repo = TempRepo::with_a_commit();
    repo.with_a_bare_remote();
    repo.git(&["push", "--set-upstream", "origin", "main"]);
    // Two commits on `main` that nobody pushed. This is the whole shape.
    main_moves_on(&repo, "one.txt", "1");
    main_moves_on(&repo, "two.txt", "2");
    let worktree = worktree_for(&repo);

    assert_eq!(
        GitVcs::new()
            .base_on_the_remote(&worktree, &Base::Inferred(String::from("main")))
            .expect("a reading"),
        BaseOnTheRemote::Apart {
            remote: String::from("origin/main"),
            ahead: 2,
            behind: 0,
        },
        "the remote is named as a person would type it, not as a refspec"
    );
}

/// A base level with its remote has nothing to say, and this is every ordinary
/// Job — so an answer here that was not `Agreed` would put a caveat on all of
/// them.
#[test]
fn a_base_level_with_its_remote_says_nothing() {
    let repo = TempRepo::with_a_commit();
    repo.with_a_bare_remote();
    repo.git(&["push", "--set-upstream", "origin", "main"]);
    let worktree = worktree_for(&repo);

    assert_eq!(
        GitVcs::new()
            .base_on_the_remote(&worktree, &Base::Inferred(String::from("main")))
            .expect("a reading"),
        BaseOnTheRemote::Agreed
    );
}

/// **Not a refusal.** A repository with no remote, and a base branch nobody has
/// ever pushed, both mean there is no second reading to compare with — the same
/// reading `Pushed::NoRemote` is, one call earlier.
#[test]
fn a_base_that_tracks_nothing_is_agreed_rather_than_refused() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    assert_eq!(
        GitVcs::new()
            .base_on_the_remote(&worktree, &Base::Inferred(String::from("main")))
            .expect("an answer, not a refusal"),
        BaseOnTheRemote::Agreed
    );
}

#[test]
fn a_repository_with_no_remote_is_answered_rather_than_failed() {
    let repo = TempRepo::with_a_commit();
    let worktree = worktree_for(&repo);
    assert_eq!(
        GitVcs::new().push(&worktree).expect("an answer"),
        Pushed::NoRemote,
        "a local-only repository is ordinary — the branch is the work"
    );
}

#[test]
fn the_branch_reaches_the_remote_under_its_own_name() {
    let repo = TempRepo::with_a_commit();
    let bare = repo.with_a_bare_remote();
    let worktree = worktree_for(&repo);
    wrote(&worktree, "answer.txt", "42");
    GitVcs::new()
        .commit_all(
            &worktree,
            "the Job's work",
            adapter_traits::CommitTime::seconds_since_epoch(1_787_734_800),
        )
        .expect("a commit");

    let pushed = GitVcs::new().push(&worktree).expect("a push");
    assert_eq!(
        pushed,
        Pushed::ToTheRemote {
            remote: String::from("origin"),
            branch: format!("armada/{JOB}")
        }
    );

    let landed = std::process::Command::new("git")
        .args(["-C", &bare.to_string_lossy(), "branch", "--list"])
        .output()
        .expect("git on PATH");
    assert!(
        String::from_utf8_lossy(&landed.stdout).contains(&format!("armada/{JOB}")),
        "the bare repository holds the Job's branch"
    );
}
