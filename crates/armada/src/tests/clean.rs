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

use adapter_traits::{Vcs, WorktreeSpec};
use adapters::GitVcs;
use core_model::{
    Facts, Job, JobId, ManifestId, ModelName, NewJob, StepId, StepSeed, Timestamp, Title,
    TopLevelOrigin, Ulid, Urgency,
};
use store::Store;

use crate::clean::{clean, CleanRefused, FileGone};
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
    git(dir.path(), &["init", "--quiet"]);
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

    let cleaned = clean(repo.path(), machine.path(), false).expect("a clean");

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

    clean(repo.path(), machine.path(), false).expect("a clean");

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

    let cleaned = clean(repo.path(), machine.path(), false).expect("a clean");

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

    let cleaned = clean(repo.path(), machine.path(), false).expect("a clean");

    assert_eq!(cleaned.unclaimed, vec![orphan.clone()]);
    assert!(orphan.is_dir(), "it is still there");
}

// -------------------------------------------------------------- what it takes

#[test]
fn all_removes_the_store_and_the_files_beside_it() {
    let repo = a_repository();
    let machine = TempDir::new();
    a_job_with_a_worktree(machine.path(), repo.path(), JOB, MANIFEST_ID);
    machine.write("mcp.json", "{}");

    let cleaned = clean(repo.path(), machine.path(), true).expect("a clean");

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

    let cleaned = clean(repo.path(), machine.path(), true).expect("a clean");

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

    for everything in [false, true] {
        let refused = clean(repo.path(), machine.path(), everything)
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

    let refused = clean(nowhere.path(), machine.path(), false)
        .expect_err("there is no Manifest here")
        .to_string();

    assert!(refused.contains("armada.yml"), "{refused}");
}
