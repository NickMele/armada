//! A new worktree seeded from the base checkout's warm build. #1064.
//!
//! **A copy standing in for the clone, and a one-file `target/`.** What is
//! asserted is which directory reached which tree and what the Job was told,
//! never a real build. The warm-up's commands are real processes, for
//! `crate::tests::preparing`'s reason.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use adapter_traits::{BaseSpec, WorktreeSpec};
use config::Manifest;
use core_model::JobStatus;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::seeding::{recorded, CopyOnWrite, NotCloned, Recorded};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;

const COMMIT: &str = "a787ffc2000000000000000000000000000000ab";
const PREVIOUS: &str = "b899ffc2000000000000000000000000000000ab";

/// Copies where the volume would clone, and keeps what it was asked.
#[derive(Default)]
pub(crate) struct Copying {
    asked: Mutex<Vec<(PathBuf, PathBuf)>>,
}

impl Copying {
    fn asked(&self) -> usize {
        self.asked.lock().expect("not poisoned").len()
    }
}

impl CopyOnWrite for Copying {
    fn clone_tree(&self, from: &Path, to: &Path) -> Result<(), NotCloned> {
        self.asked
            .lock()
            .expect("not poisoned")
            .push((from.to_path_buf(), to.to_path_buf()));
        copied(from, to).map_err(|why| NotCloned::Failed {
            why: why.to_string(),
        })
    }
}

fn copied(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        match entry.file_type()?.is_dir() {
            true => copied(&entry.path(), &target)?,
            false => std::fs::copy(entry.path(), &target).map(|_| ())?,
        }
    }
    Ok(())
}

/// A volume with no copy-on-write clone.
struct NoClone;

impl CopyOnWrite for NoClone {
    fn clone_tree(&self, _: &Path, _: &Path) -> Result<(), NotCloned> {
        Err(NotCloned::Unsupported {
            why: String::from("Operation not supported (os error 45)"),
        })
    }
}

/// A Fleet whose `armada.yml` seeds `target`, warmed by `warm`, with `requires`
/// run in the worktree after the seed.
fn a_seeding_fleet(
    home: &TempDir,
    warm: &str,
    requires: &str,
    copying: Arc<dyn CopyOnWrite>,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.vcs = FakeVcs::new().with_ref_at("main", COMMIT);
    fittings.copy_on_write = copying;
    fittings.starting().manifest = Manifest::parse(
        Path::new("armada.yml"),
        &format!(
            "version: 1\nid: 01FIXTUREMANIFEST\nbase: main\ncommands:\n\
             \x20 warm:\n    run: {warm}\n\
             \x20 check:\n    run: {requires}\n\
             setup:\n  requires: [check]\n  seed:\n    paths: [target]\n    warm: [warm]\n"
        ),
    )
    .expect("a manifest that declares a seed");
    Fleet::assembled(fittings)
}

fn base(home: &TempDir, commit: &str) -> BaseSpec {
    BaseSpec::at(&home.path().to_string_lossy(), commit).expect("a legal spec")
}

/// A base checkout holding a one-file `target/`, marked warm or not.
fn a_seed(home: &TempDir, commit: &str, warm: bool) -> BaseSpec {
    let spec = base(home, commit);
    let debug = Path::new(&spec.path()).join("target/debug");
    std::fs::create_dir_all(&debug).expect("a base checkout");
    std::fs::write(debug.join("libwarm.rlib"), format!("built at {commit}")).expect("written");
    if warm {
        std::fs::write(spec.seed_marker(), commit).expect("marked");
    }
    spec
}

fn worktree(home: &TempDir, job: &core_model::Job) -> PathBuf {
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), &job.handle()).expect("a legal spec");
    PathBuf::from(spec.worktree_path())
}

async fn a_dispatched_job(
    home: &TempDir,
    fleet: &Fleet<FakeHarness, FakeVcs, FakeWorkProduct>,
) -> core_model::Job {
    let job = fleet
        .propose(a_proposal("a Job in a repository that declares a seed"))
        .await
        .expect("proposed");
    worktree_directory(home, &job);
    let running = dispatched(fleet, job.id()).await.expect("dispatch runs");
    assert_eq!(running.status(), JobStatus::Running);
    job
}

fn log(fleet: &Fleet<FakeHarness, FakeVcs, FakeWorkProduct>, job: &core_model::Job) -> String {
    std::fs::read_to_string(crate::transcript::log_of(
        fleet.first().records_root(),
        &job.handle(),
    ))
    .expect("the Job has a log")
}

fn cold_because(
    fleet: &Fleet<FakeHarness, FakeVcs, FakeWorkProduct>,
    job: &core_model::Job,
) -> String {
    match recorded(fleet.first().records_root(), &job.handle()) {
        Some(Recorded::Cold { why }) => why,
        other => panic!("expected a cold worktree, got {other:?}"),
    }
}

/// **Definition of done, first line**, as far as a test without cargo can take
/// it: the worktree holds the base checkout's build directory before anything
/// in `setup.requires` runs, so what a build compiles is what that directory
/// does not already hold. `check` exits non-zero if the seed is not there.
#[tokio::test]
async fn a_new_worktree_starts_from_the_warm_seed_before_setup_requires_runs() {
    let home = TempDir::new();
    a_seed(&home, COMMIT, true);
    let copying = Arc::new(Copying::default());
    let fleet = a_seeding_fleet(
        &home,
        "/usr/bin/false",
        "/bin/test -f target/debug/libwarm.rlib",
        copying.clone(),
    );

    let job = a_dispatched_job(&home, &fleet).await;

    assert_eq!(
        std::fs::read_to_string(worktree(&home, &job).join("target/debug/libwarm.rlib"))
            .expect("cloned"),
        format!("built at {COMMIT}")
    );
    assert_eq!(copying.asked(), 1, "one clone, of the one declared directory");
    assert_eq!(
        recorded(fleet.first().records_root(), &job.handle()),
        Some(Recorded::Seeded {
            commit: COMMIT.to_string(),
            paths: vec![String::from("target")],
        })
    );
    assert!(log(&fleet, &job).contains("the worktree was seeded from the base checkout's warm build"));
    assert_eq!(
        fleet.run_sheet(job.id()).await.expect("a run sheet").seeding,
        Some(ipc::WorktreeSeeding::Seeded {
            commit: COMMIT.to_string(),
            paths: vec![String::from("target")],
        }),
        "the Job's Setup group reads what was written down"
    );
    assert_eq!(
        fleet.declared_seed(&fleet.first()).expect("declared").warmth,
        ipc::SeedWarmth::Warm {
            commit: COMMIT.to_string()
        }
    );
}

/// **Definition of done, second line.** No `setup.seed`: no clone, no record,
/// no line, and the base checkout is never asked for.
#[tokio::test]
async fn a_repository_that_declares_no_seed_is_cut_exactly_as_before() {
    let home = TempDir::new();
    let copying = Arc::new(Copying::default());
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.copy_on_write = copying.clone();
    let fleet = Fleet::assembled(fittings);

    let job = a_dispatched_job(&home, &fleet).await;

    assert_eq!(copying.asked(), 0);
    assert_eq!(recorded(fleet.first().records_root(), &job.handle()), None);
    let log = log(&fleet, &job);
    assert!(
        !log.contains("the worktree was seeded") && !log.contains("the worktree starts cold"),
        "{log}"
    );
    assert!(fleet.vcs().bases().is_empty(), "no base checkout was made for it");
    assert_eq!(fleet.run_sheet(job.id()).await.expect("a run sheet").seeding, None);
    assert_eq!(fleet.declared_seed(&fleet.first()), None);
    assert!(fleet.warm_seeds().is_none(), "and nothing warms");
}

/// **Definition of done, third line**, and *never clone a seed mid-warm-up*:
/// the base checkout already holds a `target/`, and it is not taken while the
/// warm-up that is filling it runs.
#[tokio::test]
async fn a_job_cut_while_the_seed_is_warming_starts_cold_and_says_so() {
    let home = TempDir::new();
    let spec = a_seed(&home, COMMIT, false);
    let copying = Arc::new(Copying::default());
    let fleet = a_seeding_fleet(&home, "/bin/sleep 2", "/usr/bin/true", copying.clone());

    let warming = fleet.warm_seeds().expect("a warm-up starts");
    let job = a_dispatched_job(&home, &fleet).await;

    assert!(!worktree(&home, &job).join("target").exists());
    assert_eq!(copying.asked(), 0);
    let why = cold_because(&fleet, &job);
    assert!(why.contains("still warming"), "{why}");
    assert!(log(&fleet, &job).contains("the worktree starts cold, with no seed"));
    assert_eq!(
        fleet.declared_seed(&fleet.first()).expect("declared").warmth,
        ipc::SeedWarmth::Warming {
            commit: COMMIT.to_string()
        },
        "the Manifest's Setup says a warm-up is running"
    );

    warming.await.expect("the warm-up ends");
    assert!(Path::new(&spec.seed_marker()).exists(), "marked once it succeeded");
}

/// *A volume with no copy-on-write clone gets no seed rather than a full copy,
/// and the Job's setup says so.*
#[tokio::test]
async fn a_volume_with_no_copy_on_write_gets_no_seed_rather_than_a_copy() {
    let home = TempDir::new();
    a_seed(&home, COMMIT, true);
    let fleet = a_seeding_fleet(&home, "/usr/bin/false", "/usr/bin/true", Arc::new(NoClone));

    let job = a_dispatched_job(&home, &fleet).await;

    assert!(!worktree(&home, &job).join("target").exists());
    let why = cold_because(&fleet, &job);
    assert!(why.contains("cannot clone copy-on-write"), "{why}");
}

/// The warm-up runs its commands in the base checkout, and the seed is marked
/// only after they succeeded.
#[tokio::test]
async fn the_warm_up_runs_in_the_base_checkout_and_marks_the_seed_once_it_succeeded() {
    let home = TempDir::new();
    let spec = base(&home, COMMIT);
    std::fs::create_dir_all(spec.path()).expect("a base checkout");
    let fleet = a_seeding_fleet(
        &home,
        "/bin/mkdir target",
        "/usr/bin/true",
        Arc::new(Copying::default()),
    );

    fleet
        .warm_seeds()
        .expect("a warm-up starts")
        .await
        .expect("the warm-up ends");

    assert!(Path::new(&spec.path()).join("target").is_dir());
    assert!(Path::new(&spec.seed_marker()).exists());
    assert!(fleet.warm_seeds().is_none(), "a warm seed is not warmed again");
}

/// A warm-up that fails leaves no mark, is not retried at the same commit, and
/// the next Job says what happened to it.
#[tokio::test]
async fn a_warm_up_that_fails_leaves_no_mark_and_the_next_job_says_why() {
    let home = TempDir::new();
    let spec = base(&home, COMMIT);
    std::fs::create_dir_all(spec.path()).expect("a base checkout");
    let fleet = a_seeding_fleet(
        &home,
        "/usr/bin/false",
        "/usr/bin/true",
        Arc::new(Copying::default()),
    );

    fleet
        .warm_seeds()
        .expect("a warm-up starts")
        .await
        .expect("the warm-up ends");
    assert!(!Path::new(&spec.seed_marker()).exists());
    assert!(fleet.warm_seeds().is_none(), "not retried until the base moves");

    let job = a_dispatched_job(&home, &fleet).await;
    let why = cold_because(&fleet, &job);
    assert!(why.contains("did not finish") && why.contains("warm"), "{why}");
}

/// A base that moved warms from the last seed rather than from nothing: the
/// warm-up here passes only if the previous commit's `target/` is already there.
#[tokio::test]
async fn a_warm_up_starts_from_the_previous_warm_seed() {
    let home = TempDir::new();
    a_seed(&home, PREVIOUS, true);
    let spec = base(&home, COMMIT);
    std::fs::create_dir_all(spec.path()).expect("a base checkout");
    let fleet = a_seeding_fleet(
        &home,
        "/bin/test -f target/debug/libwarm.rlib",
        "/usr/bin/true",
        Arc::new(Copying::default()),
    );

    fleet
        .warm_seeds()
        .expect("a warm-up starts")
        .await
        .expect("the warm-up ends");

    assert!(
        Path::new(&spec.seed_marker()).exists(),
        "the previous seed was carried into the new base before its warm-up ran"
    );
}
