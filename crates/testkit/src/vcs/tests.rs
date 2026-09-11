//! What the fake promises: the derivation the real one makes, every refusal a
//! caller has to handle, and nothing on disk.
//!
//! **Its own file for size**, which is the seam `docs/practices/rust.md`
//! section 6 names — `vcs.rs` is a fake for three traits and was at the 900
//! lines the gate refuses at.

use super::*;

const JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2A3B4";

fn spec(job: &str) -> WorktreeSpec {
    WorktreeSpec::for_job("/repos/armada", job).expect("a legal spec")
}

#[test]
fn it_derives_the_same_path_and_branch_the_real_one_would() {
    let made = FakeVcs::new().create_worktree(&spec(JOB)).unwrap();
    assert_eq!(made.path(), spec(JOB).worktree_path());
    assert_eq!(made.branch(), spec(JOB).branch());
}

#[test]
fn a_seeded_branch_is_refused_rather_than_reused() {
    let vcs = FakeVcs::new().with_existing_branch(format!("armada/{JOB}"));
    assert_eq!(
        vcs.create_worktree(&spec(JOB)),
        Err(FakeVcsError::BranchExists {
            branch: format!("armada/{JOB}")
        })
    );
    assert!(vcs.created().is_empty());
}

#[test]
fn the_same_job_twice_collides_with_itself() {
    let vcs = FakeVcs::new();
    vcs.create_worktree(&spec(JOB)).expect("the first");
    assert!(vcs.create_worktree(&spec(JOB)).is_err());
}

#[test]
fn a_scripted_refusal_applies_once() {
    let vcs = FakeVcs::new();
    vcs.refuse_next("a full disk");
    assert_eq!(
        vcs.create_worktree(&spec(JOB)),
        Err(FakeVcsError::Refused {
            standing_in_for: "a full disk"
        })
    );
    assert!(vcs.created().is_empty());
    vcs.create_worktree(&spec(JOB)).expect("the retry");
}

#[test]
fn nothing_it_recorded_ever_goes_away() {
    let vcs = FakeVcs::new();
    vcs.create_worktree(&spec("01AAA")).unwrap();
    vcs.create_worktree(&spec("01BBB")).unwrap();
    assert!(vcs.create_worktree(&spec("01AAA")).is_err());
    assert_eq!(vcs.created().len(), 2);
}

/// A test can tell a commit of some paths from a commit of everything, which
/// is the whole of what separates the two calls.
#[test]
fn a_commit_of_some_paths_records_which() {
    let vcs = FakeVcs::new();
    let made = vcs.create_worktree(&spec(JOB)).unwrap();
    let at = CommitTime::seconds_since_epoch(1);
    vcs.commit_all(&made, "everything", at).unwrap();
    vcs.commit_paths(&made, &["a.txt", "b/c.txt"], "some", at)
        .unwrap();

    let paths: Vec<Option<Vec<String>>> =
        vcs.committed().into_iter().map(|made| made.paths).collect();
    assert_eq!(
        paths,
        vec![None, Some(vec!["a.txt".to_string(), "b/c.txt".to_string()])]
    );
}

#[test]
fn it_creates_nothing_on_disk() {
    let vcs = FakeVcs::new();
    let made = vcs.create_worktree(&spec(JOB)).unwrap();
    assert!(!std::path::Path::new(made.path()).exists());
}

const COMMIT: &str = "a787ffc2000000000000000000000000000000ab";
const MOVED: &str = "b899aa11000000000000000000000000000000cd";

fn base(commit: &str) -> BaseSpec {
    BaseSpec::at("/repos/armada", commit).expect("a legal spec")
}

/// **The opposite of a Job's worktree, and deliberately so.** Two Jobs on one
/// base share the checkout; two Jobs never share a branch.
#[test]
fn one_commit_is_one_base_checkout_however_many_jobs_ask_for_it() {
    let vcs = FakeVcs::new();
    let first = vcs.base_checkout(&base(COMMIT)).expect("a base checkout");
    let second = vcs.base_checkout(&base(COMMIT)).expect("the same one");
    assert_eq!(first.path(), second.path());
    assert_eq!(vcs.bases().len(), 1);
}

/// The mark says `setup.requires` finished, so the second Job on a base is told
/// it need not pay for it again — which is the whole point of sharing one.
#[test]
fn a_prepared_base_says_so_to_every_job_after_the_first() {
    let vcs = FakeVcs::new();
    assert!(!vcs.base_checkout(&base(COMMIT)).unwrap().prepared());
    vcs.base_is_prepared(COMMIT);
    assert!(vcs.base_checkout(&base(COMMIT)).unwrap().prepared());
}

#[test]
fn a_base_that_moved_is_a_different_checkout_and_the_old_one_can_be_given_back() {
    let vcs = FakeVcs::new().with_ref_at("main", COMMIT);
    vcs.base_checkout(&base(COMMIT)).expect("a base checkout");
    vcs.move_ref_to("main", MOVED);

    assert_eq!(
        vcs.base_commit("/repos/armada", Some("main")).unwrap(),
        Some(MOVED.to_string())
    );
    vcs.drop_base_checkout(&base(COMMIT))
        .expect("it comes back");
    assert_eq!(vcs.dropped_bases(), vec![COMMIT.to_string()]);
    assert!(vcs.bases().is_empty());
}

/// **A repository that names no base is not a failure.** It has nothing a
/// change is measured against, and the branch frames are the whole answer — so
/// the silence has to be a value rather than an error a caller reports.
#[test]
fn a_repository_with_no_base_answers_nothing_and_a_missing_declared_one_is_refused() {
    let bare = FakeVcs::new();
    assert_eq!(bare.base_commit("/repos/armada", None).unwrap(), None);

    let named = FakeVcs::new().with_ref_at("main", COMMIT);
    assert!(named.base_commit("/repos/armada", Some("release")).is_err());
}
