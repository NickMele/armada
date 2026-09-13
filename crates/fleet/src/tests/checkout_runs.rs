//! A person's run in the main checkout — Journey 9, *Running one* — driven
//! the way the API drives it.
//!
//! **No Job anywhere in these fixtures but the last one.** That is the issue's
//! own claim: the Manifest surface answers on a repository that has never had
//! a Job in it, because what it reads is the file Fleet is already holding.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::WorktreeSpec;
use axum::http::StatusCode;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::gate::CheckBudget;
use crate::tests::daemon::{a_proposal, fittings};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// `lint` exits zero and writes nothing; `generate` writes a file, so a run of
/// it has something to undo; `sleep` outlives the assertions around it.
const MANIFEST: &str = r#"version: 1
id: 01FIXTUREMANIFEST
checks:
  lint:
    run: "/bin/sh -c 'echo linted'"
commands:
  bootstrap:
    run: /usr/bin/true
  generate:
    run: "/bin/sh -c 'echo made > generated.rs'"
  sleep:
    run: "/bin/sh -c 'echo going; sleep 30'"
setup:
  requires: [bootstrap]
"#;

fn a_fleet_over(home: &TempDir) -> Arc<Fixture> {
    a_fleet_watched(home, &api::Broadcaster::new())
}

fn a_fleet_watched(home: &TempDir, events: &api::Broadcaster) -> Arc<Fixture> {
    let manifest = config::Manifest::parse(Path::new("armada.yml"), MANIFEST)
        .unwrap_or_else(|why| panic!("the fixture manifest did not parse: {why}"));
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().manifest = manifest;
    // Past `sleep`'s thirty seconds, so what ends that run is Stop.
    fittings.budget = CheckBudget::of(Duration::from_secs(120));
    fittings.events = events.clone();
    Arc::new(Fleet::assembled(fittings))
}

/// The checkout as a repository with one commit, which is what a snapshot is
/// taken against.
fn a_repository_at(at: &Path) {
    let git = |args: &[&str]| {
        let run = Command::new("git")
            .arg("-C")
            .arg(at)
            .args([
                "-c",
                "user.name=a person",
                "-c",
                "user.email=a@person.invalid",
            ])
            .args(args)
            .output()
            .expect("git on PATH");
        assert!(
            run.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&run.stderr)
        );
    };
    git(&["-c", "init.defaultBranch=main", "init", "--quiet"]);
    std::fs::write(at.join("armada.yml"), MANIFEST).expect("the Manifest");
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "the first commit"]);
}

/// The run `id`'s record, once it has written one.
async fn finished(fleet: &Fixture, id: &str) -> ipc::CheckoutRunRecord {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let history = fleet
                .checkout_rehearsal_history(fleet.first())
                .await
                .expect("a history");
            if let Some(record) = history.runs.into_iter().find(|run| run.id == id) {
                return record;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the run wrote its record")
}

/// #717's definition of done, first half: with nothing in the store, the sheet
/// lists this repository's Setup, Checks and Commands.
#[tokio::test]
async fn the_sheet_lists_the_manifest_with_no_job_in_the_store() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home);

    let sheet = fleet
        .checkout_run_sheet(fleet.first())
        .await
        .expect("a sheet");

    assert_eq!(
        sheet
            .setup
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        vec!["bootstrap"],
        "the Commands `setup.requires` names"
    );
    assert_eq!(
        sheet
            .checks
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        vec!["lint"]
    );
    assert_eq!(
        sheet
            .commands
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        vec!["generate", "sleep"],
        "every Command `setup.requires` does not name"
    );
    assert!(
        sheet.checks.iter().all(|entry| !entry.frozen),
        "the checkout froze nothing: there is one file and it is the one that runs"
    );
    assert!(sheet.running.is_none(), "nothing is out");
}

/// The second half: a run started from the checkout leaves its record and its
/// log under `.armada/runs/`, readable afterwards.
#[tokio::test]
async fn a_run_leaves_its_record_and_its_log_under_armada_runs() {
    let home = TempDir::new();
    a_repository_at(home.path());
    let fleet = a_fleet_over(&home);

    let underway = Arc::clone(&fleet)
        .start_checkout_rehearsal(
            ipc::StartCheckoutRun {
                name: String::from("lint"),
                workspace: None,
            },
            fleet.first(),
        )
        .await
        .expect("underway");
    let record = finished(&fleet, &underway.id).await;

    assert_eq!(record.exit_code, Some(0));
    assert_eq!(record.command, "/bin/sh -c 'echo linted'");
    let dir = home.path().join(".armada").join("runs").join("main");
    assert!(
        dir.join(&underway.id).join("run.json").is_file(),
        "the record is under .armada/runs/main/<run>/"
    );
    let output = fleet
        .checkout_rehearsal_output(underway.id.clone(), fleet.first())
        .await
        .expect("the log");
    assert_eq!(output.lines, vec!["linted".to_string()]);
    assert_eq!(
        output.path,
        format!(".armada/runs/main/{}/output.log", underway.id)
    );
}

/// A run that wrote is undoable, and Undo puts the file back.
#[tokio::test]
async fn a_run_that_wrote_is_undone_from_its_own_snapshot() {
    let home = TempDir::new();
    a_repository_at(home.path());
    let fleet = a_fleet_over(&home);

    let underway = Arc::clone(&fleet)
        .start_checkout_rehearsal(
            ipc::StartCheckoutRun {
                name: String::from("generate"),
                workspace: None,
            },
            fleet.first(),
        )
        .await
        .expect("underway");
    let record = finished(&fleet, &underway.id).await;

    assert!(record.undoable, "a snapshot was kept, so Undo is offered");
    assert!(home.path().join("generated.rs").is_file());
    let undone = fleet
        .undo_checkout_rehearsal(underway.id.clone(), fleet.first())
        .await
        .expect("the tree put back");
    assert!(undone.undone_at.is_some());
    assert!(
        !undone.undoable,
        "a run is undone once, and the sheet must not offer it twice"
    );
    assert!(
        !home.path().join("generated.rs").exists(),
        "the file the run wrote is gone"
    );
}

/// #782's definition of done: a run that changed a file is read back over the
/// wire as a patch against **its own snapshot** — the person's uncommitted
/// edit from before the run and one made after it are both absent — and once
/// that snapshot is gone the answer says so rather than diffing against `HEAD`.
#[tokio::test]
async fn a_runs_diff_is_against_its_own_snapshot_and_says_when_that_is_gone() {
    let home = TempDir::new();
    a_repository_at(home.path());
    std::fs::write(home.path().join("mine.txt"), "a person's own work\n").expect("an edit");
    let fleet = a_fleet_over(&home);
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        ipc::RunId::carried("01RUN"),
        fleet.events(),
    ));

    let underway = Arc::clone(&fleet)
        .start_checkout_rehearsal(
            ipc::StartCheckoutRun {
                name: String::from("generate"),
                workspace: None,
            },
            fleet.first(),
        )
        .await
        .expect("underway");
    finished(&fleet, &underway.id).await;
    std::fs::write(home.path().join("mine.txt"), "edited after the run\n").expect("an edit");
    let uri = format!("/manifest/runs/{}/diff", underway.id);

    let (status, body) = call(&app, "GET", &uri, "").await;
    assert_eq!(status, StatusCode::OK);
    let diff: ipc::CheckoutRunDiff = ipc::decode("a run's diff", &body).expect("a diff");
    assert_eq!(diff.id, underway.id);
    assert_eq!(diff.against, ipc::DiffAgainst::RunSnapshot);
    let ipc::RunDiffReading::Read { files, patch } = diff.reading else {
        panic!(
            "the snapshot is kept, so the patch is read: {:?}",
            diff.reading
        );
    };
    let paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, vec!["generated.rs"]);
    let patch = patch.expect("the run wrote something");
    assert!(patch.contains("+made"), "{patch}");
    assert!(
        !patch.contains("mine.txt"),
        "work either side of the run is not the run's: {patch}"
    );

    let reference = format!("refs/armada/rehearsals/{}", underway.id);
    let deleted = Command::new("git")
        .arg("-C")
        .arg(home.path())
        .args(["update-ref", "-d", &reference])
        .status()
        .expect("git on PATH");
    assert!(deleted.success());

    let (status, body) = call(&app, "GET", &uri, "").await;
    assert_eq!(status, StatusCode::OK);
    let diff: ipc::CheckoutRunDiff = ipc::decode("a run's diff", &body).expect("a diff");
    assert!(
        matches!(diff.reading, ipc::RunDiffReading::Gone { .. }),
        "a gone snapshot is said, never a patch against HEAD: {:?}",
        diff.reading
    );
    assert!(
        home.path().join("generated.rs").is_file(),
        "a read undid nothing"
    );

    let (status, _) = call(&app, "GET", "/manifest/runs/01NOSUCHRUN/diff", "").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// One run at a time is **per owner**: two in the checkout fight over one
/// build directory, and a Job's run is in another tree entirely.
#[tokio::test]
async fn a_checkout_run_and_a_jobs_run_do_not_lock_each_other_out() {
    let home = TempDir::new();
    a_repository_at(home.path());
    let fleet = a_fleet_over(&home);
    let job = fleet
        .propose(a_proposal("tidy the log reader"))
        .await
        .expect("a Job at the approval gate");
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), &job.handle()).expect("a legal spec");
    std::fs::create_dir_all(spec.worktree_path()).expect("the worktree");

    let theirs = Arc::clone(&fleet)
        .start_checkout_rehearsal(
            ipc::StartCheckoutRun {
                name: String::from("sleep"),
                workspace: None,
            },
            fleet.first(),
        )
        .await
        .expect("the checkout's run");
    let second = Arc::clone(&fleet)
        .start_checkout_rehearsal(
            ipc::StartCheckoutRun {
                name: String::from("lint"),
                workspace: None,
            },
            fleet.first(),
        )
        .await;
    assert!(
        second.is_err(),
        "two runs in the checkout share one tree and one build directory"
    );
    let jobs = Arc::clone(&fleet)
        .start_rehearsal(
            job.id(),
            ipc::StartRun {
                name: String::from("sleep"),
                narrowed: false,
                worktree_version: false,
            },
        )
        .await
        .expect("the Job's run, in the Job's own tree, while the checkout's is out");

    assert_ne!(theirs.id, jobs.id);
    fleet
        .stop_checkout_rehearsal(theirs.id, fleet.first())
        .await
        .expect("the checkout's run stopped");
    fleet
        .stop_rehearsal(job.id(), jobs.id)
        .await
        .expect("the Job's run stopped");
}

/// **One kind per owner.** A Job's run ends with `run.finished` carrying a
/// `RunRecord`; this one ends with `checkout_run.finished` carrying a record
/// that names no Job — so a reader folding the first by its `job_id` is never
/// handed one that has none.
#[tokio::test]
async fn a_checkout_run_ends_with_its_own_kind_on_the_stream() {
    let home = TempDir::new();
    a_repository_at(home.path());
    let events = api::Broadcaster::new();
    let fleet = a_fleet_watched(&home, &events);
    let mut watching = events.subscribe();

    let underway = Arc::clone(&fleet)
        .start_checkout_rehearsal(
            ipc::StartCheckoutRun {
                name: String::from("lint"),
                workspace: None,
            },
            fleet.first(),
        )
        .await
        .expect("underway");

    let seen = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match watching.next().await {
                Some(api::Next::Send(delivered)) => match delivered.event {
                    ipc::Event::CheckoutRunFinished(record) if record.id == underway.id => {
                        return record
                    }
                    ipc::Event::RunFinished(record) => {
                        panic!("a checkout run published a Job's kind: {}", record.id)
                    }
                    _ => continue,
                },
                Some(_) => continue,
                None => panic!("the stream closed"),
            }
        }
    })
    .await
    .expect("the event arrived");

    assert_eq!(seen.name, "lint");
    assert_eq!(seen.exit_code, Some(0));
    assert_eq!(seen.ended, "exited 0");
}
