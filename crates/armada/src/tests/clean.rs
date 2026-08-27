//! `armada clean`, against a real repository and a real store.
//!
//! **Real git, because the thing under test is git's own opinion** — whether a
//! branch is gone, whether a registration outlived a directory. A fake would be
//! asserting this crate's guess.
//!
//! The test this file exists for is
//! [`an_unrelated_armada_branch_is_left_alone`]: cleaning by hand with a glob
//! over the `armada/` namespace destroyed nine unmerged branches belonging to
//! no Job.

use std::path::Path;
use std::process::Command;

use adapter_traits::{CommitTime, Vcs, Worktree, WorktreeSpec};
use adapters::{BranchGone, GitVcs, UnmergedWork};
use core_model::{
    Facts, Job, JobId, ManifestId, ModelName, NewJob, StepId, StepSeed, Timestamp, Title,
    TopLevelOrigin, Ulid, Urgency,
};
use store::Store;

use crate::clean::{clean, CleanRefused, FileGone, Scope};
use crate::serve::STORE_FILE;
use crate::tests::TempDir;

const MANIFEST_ID: &str = "a-test-project";
const JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2A3B4";
const OTHER_JOB: &str = "01K3Q4R5S6T7V8W9X0Y1Z2C5D6";

/// A git repository with one commit and an `armada.yml` naming [`MANIFEST_ID`].
fn a_repository() -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        &format!("version: 1\nid: {MANIFEST_ID}\nchecks:\n  build:\n    run: /bin/sh -c true\n"),
    );
    // `main` by name, not by whoever's `init.defaultBranch` is set: what
    // counts as merged is now read from the repository.
    git(
        dir.path(),
        &["-c", "init.defaultBranch=main", "init", "--quiet"],
    );
    git(dir.path(), &["add", "."]);
    git(
        dir.path(),
        &[
            "-c",
            "user.name=armada",
            "-c",
            "user.email=armada@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "the first commit",
        ],
    );
    dir
}

fn git(at: &Path, args: &[&str]) -> String {
    let run = Command::new("git")
        .arg("-C")
        .arg(at)
        .args(args)
        .output()
        .expect("git on PATH — a test nothing can run is a test that does not exist");
    assert!(run.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&run.stdout).into_owned()
}

fn branches(at: &Path) -> Vec<String> {
    git(at, &["branch", "--format=%(refname:short)"])
        .lines()
        .map(str::to_string)
        .collect()
}

/// One Job, owned by `manifest`, with a worktree and a branch to match.
fn a_job_with_a_worktree(machine: &Path, repo: &Path, job: &str, manifest: &str) {
    let spec = WorktreeSpec::for_job(&repo.to_string_lossy(), job).expect("a legal spec");
    GitVcs::new().create_worktree(&spec).expect("a worktree");

    let mut store = Store::open(&machine.join(STORE_FILE)).expect("a store");
    store
        .insert_job(&a_job(job, manifest), &at())
        .expect("the job is stored");
}

fn at() -> Timestamp {
    Timestamp::from_rfc3339("2026-08-26T09:00:00.000Z")
}

fn a_job(id: &str, manifest: &str) -> Job {
    Job::create_top_level(
        NewJob {
            id: JobId::carried(Ulid::carried(id)),
            title: Title::new("a job that had a worktree").expect("a title"),
            workflow: testkit::frozen(&[testkit::Sketch {
                id: "fix",
                label: "Fix",
                evidence_type: Some("diff"),
                gates: &[],
            }]),
            owner_manifest_id: ManifestId::carried(Ulid::carried(manifest)),
            urgency: Urgency::Normal,
            atomic: true,
            model: ModelName::new("a-model-name").expect("a model name"),
            acceptance_criteria: Vec::new(),
            steps: vec![StepSeed {
                step_id: StepId::new("fix"),
                ordinal: 0,
            }],
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            facts: Facts::new("what was observed"),
            scope_revisions: Vec::new(),
        },
        TopLevelOrigin::HelmDrafted,
        at(),
    )
}

// -------------------------------------------------------------- what it does

#[test]
fn a_jobs_worktree_its_branch_and_its_record_all_go() {
    let repo = a_repository();
    let machine = TempDir::new();
    a_job_with_a_worktree(machine.path(), repo.path(), JOB, MANIFEST_ID);

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    assert_eq!(cleaned.jobs.len(), 1);
    assert!(cleaned.jobs[0].forgotten.existed);
    assert!(!repo.path().join(".armada/worktrees").join(JOB).exists());
    assert!(!branches(repo.path()).contains(&format!("armada/{JOB}")));

    let mut store = Store::open(&machine.path().join(STORE_FILE)).expect("the store");
    assert!(store.load_all_jobs().expect("a read").jobs.is_empty());
}

/// **The bug this verb was shaped by.** `clean` derives the branches it deletes
/// from the Jobs it is deleting, so a branch in the same namespace that belongs
/// to no Job is not its to touch.
#[test]
fn an_unrelated_armada_branch_is_left_alone() {
    let repo = a_repository();
    let machine = TempDir::new();
    a_job_with_a_worktree(machine.path(), repo.path(), JOB, MANIFEST_ID);
    git(
        repo.path(),
        &["branch", "armada/a-branch-somebody-is-using"],
    );

    clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    let left = branches(repo.path());
    assert!(
        left.contains(&"armada/a-branch-somebody-is-using".to_string()),
        "no Job derived it, so nothing here may delete it: {left:?}"
    );
    assert!(!left.contains(&format!("armada/{JOB}")));
}

/// A Manifest cleans its own Jobs. Another project's Job on the same machine
/// keeps its worktree, its branch and its record.
#[test]
fn another_manifests_jobs_are_not_this_manifests_to_clean() {
    let repo = a_repository();
    let machine = TempDir::new();
    a_job_with_a_worktree(machine.path(), repo.path(), JOB, MANIFEST_ID);
    a_job_with_a_worktree(machine.path(), repo.path(), OTHER_JOB, "another-project");

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    assert_eq!(cleaned.jobs.len(), 1);
    assert!(branches(repo.path()).contains(&format!("armada/{OTHER_JOB}")));
    assert!(repo
        .path()
        .join(".armada/worktrees")
        .join(OTHER_JOB)
        .is_dir());
}

/// **Evidence, not litter.** A checkout with nothing behind it is reported and
/// left where it is, because a directory no Job explains is worth looking at.
#[test]
fn a_worktree_with_no_job_behind_it_is_reported_and_left_alone() {
    let repo = a_repository();
    let machine = TempDir::new();
    let orphan = repo
        .path()
        .join(".armada/worktrees/01NOJOBEVERCLAIMEDTHIS0000");
    std::fs::create_dir_all(&orphan).expect("a directory nothing accounts for");

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    assert_eq!(cleaned.unclaimed, vec![orphan.clone()]);
    assert!(orphan.is_dir(), "it is still there");
}

// ------------------------------- rows the store holds and cannot rebuild

/// A stored Job whose `workflow` column is null, exactly as V7 left the four
/// real ones. The row still names its Job and its Manifest; nothing folds it.
fn a_row_that_will_not_rebuild(machine: &Path, repo: &Path, job: &str, manifest: &str) {
    a_job_with_a_worktree(machine, repo, job, manifest);
    let mut store = Store::open(&machine.join(STORE_FILE)).expect("a store");
    store
        .unfreeze_a_jobs_workflow(job)
        .expect("the column is now null, as the migration left it");
}

/// **The helplessness this closes.** Recovery from four such rows took `--all`,
/// which wipes every Manifest's Jobs on the machine, and then git by hand.
#[test]
fn a_row_that_will_not_rebuild_is_cleared_from_its_own_repository() {
    let repo = a_repository();
    let machine = TempDir::new();
    a_row_that_will_not_rebuild(machine.path(), repo.path(), JOB, MANIFEST_ID);

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    assert!(cleaned.jobs.is_empty(), "nothing folded into a Job");
    assert_eq!(cleaned.unreadable.len(), 1);
    assert_eq!(cleaned.unreadable[0].job_id, JOB);
    assert!(
        cleaned.unreadable[0].why.contains("workflow"),
        "it says why, while the row still exists to say it: {}",
        cleaned.unreadable[0].why
    );
    assert!(cleaned.unreadable[0].forgotten.existed);
    assert!(!repo.path().join(".armada/worktrees").join(JOB).exists());
    assert!(!branches(repo.path()).contains(&format!("armada/{JOB}")));
    assert!(
        cleaned.unclaimed.is_empty(),
        "the row accounted for its worktree: {:?}",
        cleaned.unclaimed
    );

    let mut store = Store::open(&machine.path().join(STORE_FILE)).expect("the store");
    assert!(
        store
            .load_all_jobs()
            .expect("nothing is left to refuse")
            .jobs
            .is_empty(),
        "and the row is gone, so the next boot has nothing to report"
    );
}

/// **Knowing less is a reason to be more careful, not less.** An unreadable row
/// is a row Armada cannot say anything about, and its branch may be the only
/// copy of the work.
#[test]
fn an_unreadable_rows_branch_is_kept_when_it_holds_unmerged_work() {
    let repo = a_repository();
    let machine = TempDir::new();
    a_job_that_finished(machine.path(), repo.path(), JOB);
    Store::open(&machine.path().join(STORE_FILE))
        .expect("a store")
        .unfreeze_a_jobs_workflow(JOB)
        .expect("the migration's damage");

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    assert_eq!(cleaned.unreadable.len(), 1);
    assert!(branches(repo.path()).contains(&format!("armada/{JOB}")));
    let left = cleaned.branches_left();
    assert_eq!(left.len(), 1, "and it is named at the end, like any other");
    let BranchGone::Kept { base, commits, .. } = left[0] else {
        panic!("it says how much is unmerged: {:?}", left[0]);
    };
    assert_eq!((base.as_str(), *commits), ("main", 1));
    assert!(!repo.path().join(".armada/worktrees").join(JOB).exists());
}

/// The Manifest on the row selects it, exactly as the owner on a folded Job
/// does. `--all` was the wrong recovery because it ignored this.
#[test]
fn another_manifests_unreadable_row_is_left_where_it_is() {
    let repo = a_repository();
    let machine = TempDir::new();
    a_row_that_will_not_rebuild(machine.path(), repo.path(), OTHER_JOB, "another-project");

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    assert!(cleaned.unreadable.is_empty());
    assert_eq!(
        cleaned.unreadable_elsewhere, 1,
        "counted, not silently dropped"
    );
    assert!(branches(repo.path()).contains(&format!("armada/{OTHER_JOB}")));
    assert!(repo
        .path()
        .join(".armada/worktrees")
        .join(OTHER_JOB)
        .is_dir());
}

// ------------------------------------------ what it refuses to throw away

const NINE: CommitTime = CommitTime::seconds_since_epoch(1_787_734_800);

/// A Job whose last step advanced, so Fleet committed its work to its branch.
fn a_job_that_finished(machine: &Path, repo: &Path, job: &str) {
    a_job_with_a_worktree(machine, repo, job, MANIFEST_ID);
    // Derived from the spec, never composed by hand — the same rule the verb
    // under test follows for the branch it deletes.
    let spec = WorktreeSpec::for_job(&repo.to_string_lossy(), job).expect("a legal spec");
    let worktree = Worktree::at(spec.worktree_path(), spec.branch());
    std::fs::write(format!("{}/answer.txt", worktree.path()), "42").expect("the work");
    GitVcs::new()
        .commit_all(&worktree, "the work", NINE)
        .expect("Fleet commits when the last step advances");
}

/// **The loss this closes.** A completed Job's branch is the only copy of its
/// work until somebody merges it, so a clean that deletes it destroys a commit.
#[test]
fn a_branch_holding_work_nothing_has_taken_survives_the_clean_and_is_named() {
    let repo = a_repository();
    let machine = TempDir::new();
    a_job_that_finished(machine.path(), repo.path(), JOB);

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    assert!(branches(repo.path()).contains(&format!("armada/{JOB}")));
    let left = cleaned.branches_left();
    assert_eq!(left.len(), 1);
    let BranchGone::Kept { base, commits, .. } = left[0] else {
        panic!("it says how much is unmerged: {:?}", left[0]);
    };
    assert_eq!((base.as_str(), *commits), ("main", 1));
    // The worktree is reproducible and the commit is not, so only one stays.
    assert!(!repo.path().join(".armada/worktrees").join(JOB).exists());
    assert!(cleaned.faults.is_empty(), "keeping a branch is not a fault");
}

/// Merged is deleted. Somebody took the work, so the branch is a label.
#[test]
fn a_branch_whose_work_is_on_main_is_deleted_as_before() {
    let repo = a_repository();
    let machine = TempDir::new();
    a_job_that_finished(machine.path(), repo.path(), JOB);
    git(
        repo.path(),
        &["merge", "--ff-only", &format!("armada/{JOB}")],
    );

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    assert!(!branches(repo.path()).contains(&format!("armada/{JOB}")));
    assert!(cleaned.branches_left().is_empty());
}

/// `--force` is the deliberate override. It is a different question from
/// `--all`, and a different flag.
#[test]
fn force_deletes_a_branch_nothing_has_taken() {
    let repo = a_repository();
    let machine = TempDir::new();
    a_job_that_finished(machine.path(), repo.path(), JOB);

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Delete,
    )
    .expect("a clean");

    assert!(!branches(repo.path()).contains(&format!("armada/{JOB}")));
    assert!(cleaned.branches_left().is_empty());
    // The tip is the only thing that makes a deleted branch recoverable.
    assert!(matches!(
        cleaned.jobs[0].reclaimed.branch,
        BranchGone::Deleted { .. }
    ));
}

// -------------------------------------------------------------- what it takes

#[test]
fn all_removes_the_store_and_the_files_beside_it() {
    let repo = a_repository();
    let machine = TempDir::new();
    a_job_with_a_worktree(machine.path(), repo.path(), JOB, MANIFEST_ID);
    machine.write("mcp.json", "{}");

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::AndTheMachine,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    assert!(!machine.path().join(STORE_FILE).exists());
    assert!(!machine.path().join("mcp.json").exists());
    // The Job went first, or the store would have been gone before anything
    // could derive its branch.
    assert_eq!(cleaned.jobs.len(), 1);
    assert!(!branches(repo.path()).contains(&format!("armada/{JOB}")));
}

/// Absence is an answer. A file that was not there reads differently from one
/// that was removed, and both are printed.
#[test]
fn a_machine_file_that_was_never_there_is_said_rather_than_skipped() {
    let repo = a_repository();
    let machine = TempDir::new();

    let cleaned = clean(
        repo.path(),
        machine.path(),
        Scope::AndTheMachine,
        UnmergedWork::Keep,
    )
    .expect("a clean");

    assert!(cleaned
        .machine
        .iter()
        .all(|file| matches!(file, FileGone::Absent(_))));
}

/// **The refusal `--all` exists for**, and bare `clean` takes it too: both
/// forget Jobs a live Fleet is holding in memory.
#[test]
fn a_clean_is_refused_while_a_fleet_holds_the_store() {
    let repo = a_repository();
    let machine = TempDir::new();
    pretend_a_fleet_is_running(machine.path());

    for scope in [Scope::Repository, Scope::AndTheMachine] {
        let refused = clean(repo.path(), machine.path(), scope, UnmergedWork::Keep)
            .expect_err("a live Fleet is holding these Jobs");
        let CleanRefused::FleetIsRunning { pid, .. } = refused else {
            panic!("the refusal names the Fleet: {refused}");
        };
        assert_eq!(pid, std::process::id(), "and names it by pid");
    }
}

/// A runtime file naming this very process, which is a pid that is certainly
/// held by the process that wrote it.
fn pretend_a_fleet_is_running(machine: &Path) {
    let pid = std::process::id();
    let fleet::Holder::Held(started_at) = fleet::holder_of(pid).expect("this process is visible")
    else {
        panic!("a running process holds its own pid");
    };
    let file = fleet::runtime::RuntimeFile {
        protocol_version: ipc::PROTOCOL_VERSION,
        pid,
        port: 47821,
        started_at,
    };
    std::fs::write(
        machine.join(fleet::runtime::FILE_NAME),
        ipc::encode(&file).expect("four scalars serialise"),
    )
    .expect("the runtime file");
}

#[test]
fn a_directory_that_is_not_a_repository_is_refused_by_naming_the_manifest() {
    let nowhere = TempDir::new();
    let machine = TempDir::new();

    let refused = clean(
        nowhere.path(),
        machine.path(),
        Scope::Repository,
        UnmergedWork::Keep,
    )
    .expect_err("there is no Manifest here")
    .to_string();

    assert!(refused.contains("armada.yml"), "{refused}");
}
