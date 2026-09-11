//! A person's run in a Job's worktree, driven the way the API drives it —
//! issue #621's definition of done, case by case.
//!
//! **A real repository in the Job's worktree**, because what Undo protects is
//! what git would call the Drone's uncommitted work. The commands are `sh`
//! one-liners, and what they print or write is the assertion.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::WorktreeSpec;
use api::{Next, Queries, Subscription};
use core_model::{Job, JobStatus};
use ipc::{ChangeKind, Event, RunRecord, StartRun};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::clock::rfc3339_utc;
use crate::daemon::Fleet;
use crate::gate::CheckBudget;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal_for, fittings, one};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// `test` narrows to the changed paths and sleeps, so it is still running when
/// it is stopped; whole, it exits 3. `format` requires `fmt`, which writes.
const MANIFEST: &str = r#"version: 1
id: 01FIXTUREMANIFEST
checks:
  test:
    run: "/bin/sh -c 'echo whole; exit 3'"
    narrow:
      run: "/bin/sh -c 'echo narrowed; sleep 30'"
      each: "{}"
  format:
    run: "/bin/sh -c 'grep -q formatted src/lib.rs'"
    requires: [fmt]
commands:
  bootstrap:
    run: /usr/bin/true
  fmt:
    run: "/bin/sh -c 'echo formatted > src/lib.rs; echo made > generated.rs'"
setup:
  requires: [bootstrap]
"#;

const WORKFLOW: &str = "version: 1\nworkflow_id: fixture-rehearsal\nname: fixture\nstructure: \
     linear\nsteps:\n  - id: implement\n    label: \"Implement\"\n    evidence_type: diff\n    \
     delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - type: \
     manifest_check\n        check: test\n      - type: manifest_check\n        check: format\n";

/// When the fixture's `armada.yml` was committed: the day before the fixture
/// clock's first reading, so before any Job froze it.
const EDITED: i64 = 1_787_659_200;
/// A later edit, after the Job froze the file.
const EDITED_AFTER: i64 = EDITED + 7 * 86_400;

const NARROWED: &str = "/bin/sh -c 'echo narrowed; sleep 30' src/log.rs";

fn a_fleet_rehearsing(home: &TempDir, events: &api::Broadcaster) -> Arc<Fixture> {
    let manifest = config::Manifest::parse(Path::new("armada.yml"), MANIFEST)
        .unwrap_or_else(|why| panic!("the fixture manifest did not parse: {why}"));
    let def = config::WorkflowDef::parse(
        Path::new("fixture-rehearsal.yml"),
        WORKFLOW,
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|why| panic!("the fixture workflow did not parse: {why}"));
    let workflow = config::ResolvedWorkflow::resolve(&def, &manifest)
        .unwrap_or_else(|why| panic!("the fixture workflow did not resolve: {why}"));
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = one(workflow);
    fittings.manifest = manifest;
    fittings.events = events.clone();
    // Past the narrowed `test`'s thirty seconds, so what ends it is Stop.
    fittings.budget = CheckBudget::of(Duration::from_secs(120));
    Arc::new(Fleet::assembled(fittings))
}

fn git(at: &Path, args: &[&str], dated: Option<i64>) {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(at)
        .args([
            "-c",
            "user.name=a person",
            "-c",
            "user.email=a@person.invalid",
        ])
        .args(args);
    if let Some(seconds) = dated {
        let date = format!("@{seconds} +0000");
        command
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date);
    }
    let run = command.output().expect("git on PATH");
    assert!(
        run.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
}

fn read(tree: &Path, path: &str) -> String {
    std::fs::read_to_string(tree.join(path)).unwrap_or_default()
}

/// A Job at the approval gate whose worktree is a repository holding one
/// commit, with a Drone's work on top of it: `src/lib.rs` edited and
/// `notes.md` new, neither committed — what every Job looks like until it
/// delivers.
async fn a_job_with_work_in_it(fleet: &Fixture, home: &TempDir) -> (Job, PathBuf) {
    let job = fleet
        .propose(a_proposal_for("tidy the log reader", "fixture-rehearsal"))
        .await
        .expect("a Job at the approval gate");
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), &job.handle()).expect("a legal spec");
    let tree = PathBuf::from(spec.worktree_path());
    std::fs::create_dir_all(tree.join("src")).expect("the worktree");
    git(
        &tree,
        &["-c", "init.defaultBranch=main", "init", "--quiet"],
        None,
    );
    std::fs::write(tree.join("src/lib.rs"), "committed\n").expect("a file");
    std::fs::write(tree.join("armada.yml"), MANIFEST).expect("the Manifest");
    git(&tree, &["add", "."], None);
    git(
        &tree,
        &["commit", "--quiet", "-m", "the first commit"],
        Some(EDITED),
    );
    std::fs::write(tree.join("src/lib.rs"), "the drone's\n").expect("the Drone's edit");
    std::fs::write(tree.join("notes.md"), "the drone's notes\n").expect("the Drone's file");
    (job, tree)
}

fn asked(name: &str, narrowed: bool) -> StartRun {
    StartRun {
        name: name.to_string(),
        narrowed,
        worktree_version: false,
    }
}

async fn next_event(watching: &mut Subscription, wanted: impl Fn(&Event) -> bool) -> Event {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) if wanted(&delivered.event) => return delivered.event,
                Some(_) => continue,
                None => panic!("the stream closed"),
            }
        }
    })
    .await
    .expect("the event arrived")
}

async fn finished(watching: &mut Subscription, id: &str) -> RunRecord {
    let Event::RunFinished(record) = next_event(
        watching,
        |event| matches!(event, Event::RunFinished(record) if record.id == id),
    )
    .await
    else {
        unreachable!("the filter admits only a finished run");
    };
    record
}

/// **The first half of the definition of done.** `test`, narrowed, prints
/// while it runs; Stop ends it; the history holds it with its log beside a
/// whole run with its exit code — and no Check row or Evidence was written.
#[tokio::test]
async fn a_narrowed_test_streams_is_stopped_and_is_in_the_history_with_no_verdict() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_rehearsing(&home, &events);
    let (job, _) = a_job_with_work_in_it(&fleet, &home).await;
    let mut watching = events.subscribe();

    let underway = Arc::clone(&fleet)
        .start_rehearsal(job.id(), asked("test", true))
        .await
        .expect("underway");
    assert!(underway.narrowed);
    assert_eq!(
        underway.command, NARROWED,
        "narrowed to what the Job changed"
    );

    let Event::RunOutput(output) = next_event(
        &mut watching,
        |event| matches!(event, Event::RunOutput(lines) if !lines.lines.is_empty()),
    )
    .await
    else {
        unreachable!("the filter admits only output");
    };
    assert_eq!(output.id, underway.id);
    assert_eq!(output.lines, vec!["narrowed".to_string()]);

    let stopped = fleet
        .stop_rehearsal(job.id(), underway.id.clone())
        .await
        .expect("stopped");
    assert!(stopped.stopped);
    assert_eq!(stopped.exit_code, None, "a stopped run has no code");
    assert_eq!(stopped.ended, "was stopped");

    let whole = Arc::clone(&fleet)
        .start_rehearsal(job.id(), asked("test", false))
        .await
        .expect("a second run, once the first is over");
    let ended = finished(&mut watching, &whole.id).await;
    assert_eq!(ended.exit_code, Some(3));
    assert_eq!(ended.expect_exit_code, 0);

    let history = fleet.rehearsal_history(job.id()).await.expect("a history");
    assert!(history.unreadable.is_empty(), "{:?}", history.unreadable);
    let ids: Vec<&str> = history.runs.iter().map(|run| run.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![whole.id.as_str(), underway.id.as_str()],
        "newest first"
    );
    assert_eq!(history.runs[0].exit_code, Some(3));
    assert!(history.runs[1].stopped);
    let log = fleet
        .rehearsal_output(job.id(), underway.id.clone())
        .await
        .expect("the stopped run's log");
    assert_eq!(
        log.lines,
        vec!["narrowed".to_string()],
        "the log keeps what printed"
    );
    assert!(log.whole);

    let checks = fleet
        .store()
        .lock()
        .await
        .step_checks_every_attempt(job.id())
        .expect("the record reads");
    assert!(
        checks.iter().all(|attempt| attempt.record.is_empty()),
        "a run from the sheet wrote a Check row"
    );
    let evidence = Queries::get_evidence(&*fleet, ipc::JobId::from(job.id()))
        .await
        .expect("the evidence reads");
    assert!(
        evidence.steps.is_empty(),
        "a run from the sheet wrote Evidence"
    );
    assert_eq!(
        fleet.load(job.id()).await.expect("the Job").status(),
        job.status(),
        "nothing moved the Job"
    );
}

/// **The second half.** `fmt` writes; the files it wrote are named; Undo puts
/// back the Drone's uncommitted work — not the commit the branch started from.
#[tokio::test]
async fn after_fmt_the_changed_files_are_named_and_undo_puts_the_drones_work_back() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_rehearsing(&home, &events);
    let (job, tree) = a_job_with_work_in_it(&fleet, &home).await;
    let mut watching = events.subscribe();

    let underway = Arc::clone(&fleet)
        .start_rehearsal(job.id(), asked("fmt", false))
        .await
        .expect("underway");
    let ended = finished(&mut watching, &underway.id).await;
    assert_eq!(ended.exit_code, Some(0));
    let changed: Vec<(&str, ChangeKind)> = ended
        .changed
        .iter()
        .map(|file| (file.path.as_str(), file.change))
        .collect();
    assert_eq!(
        changed,
        vec![
            ("generated.rs", ChangeKind::Added),
            ("src/lib.rs", ChangeKind::Modified)
        ]
    );
    assert_eq!(read(&tree, "src/lib.rs"), "formatted\n");

    let undone = fleet
        .undo_rehearsal(job.id(), underway.id.clone())
        .await
        .expect("undone");
    assert!(undone.undone_at.is_some());
    assert_eq!(read(&tree, "src/lib.rs"), "the drone's\n");
    assert_eq!(read(&tree, "notes.md"), "the drone's notes\n");
    assert!(!tree.join("generated.rs").exists());

    let twice = fleet
        .undo_rehearsal(job.id(), underway.id)
        .await
        .expect_err("a run is undone once");
    assert_eq!(twice.status(), 409);
}

/// A Check's `requires` run first, into the same log, and the record names
/// them — a person reading changed files is told what wrote them.
#[tokio::test]
async fn a_checks_prerequisites_run_first_into_the_same_log() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_rehearsing(&home, &events);
    let (job, _) = a_job_with_work_in_it(&fleet, &home).await;
    let mut watching = events.subscribe();

    let underway = Arc::clone(&fleet)
        .start_rehearsal(job.id(), asked("format", false))
        .await
        .expect("underway");
    let ended = finished(&mut watching, &underway.id).await;
    assert_eq!(ended.required, vec!["fmt".to_string()]);
    assert_eq!(ended.exit_code, Some(0), "`fmt` ran before the Check read");
    let log = fleet
        .rehearsal_output(job.id(), underway.id)
        .await
        .expect("its log");
    assert!(
        log.lines
            .first()
            .is_some_and(|line| line.contains("`fmt` first")),
        "{:?}",
        log.lines
    );
}

/// Undo is not offered while a Drone works: its edits are uncommitted, and
/// nothing can tell them from the run's.
#[tokio::test]
async fn undo_is_refused_while_a_drone_is_working() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_rehearsing(&home, &events);
    let (job, tree) = a_job_with_work_in_it(&fleet, &home).await;
    let mut watching = events.subscribe();
    let underway = Arc::clone(&fleet)
        .start_rehearsal(job.id(), asked("fmt", false))
        .await
        .expect("underway");
    finished(&mut watching, &underway.id).await;

    let running = dispatched(&fleet, job.id()).await.expect("released");
    assert_eq!(running.status(), JobStatus::Running);
    let refused = fleet
        .undo_rehearsal(job.id(), underway.id)
        .await
        .expect_err("a Drone is working");
    assert_eq!(refused.status(), 409);
    assert_eq!(
        read(&tree, "src/lib.rs"),
        "formatted\n",
        "nothing was put back"
    );
}

/// The sheet lists what the Job froze; the worktree's own file moving on is
/// a notice, and its version runs only when asked for.
#[tokio::test]
async fn the_sheet_lists_what_the_job_froze_and_the_worktrees_version_runs_only_when_asked() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_rehearsing(&home, &events);
    let (job, tree) = a_job_with_work_in_it(&fleet, &home).await;

    let sheet = fleet.run_sheet(job.id()).await.expect("a sheet");
    let names = |entries: &[ipc::RunEntry]| -> Vec<String> {
        entries.iter().map(|entry| entry.name.clone()).collect()
    };
    assert_eq!(names(&sheet.setup), vec!["bootstrap"]);
    assert_eq!(names(&sheet.checks), vec!["test", "format"]);
    assert_eq!(names(&sheet.commands), vec!["fmt"]);
    assert!(sheet.checks[0].frozen && sheet.checks[0].narrows);
    assert_eq!(sheet.checks[0].narrow_run.as_deref(), Some(NARROWED));
    assert!(sheet.commands[0].frozen, "a frozen Check requires `fmt`");
    assert!(
        !sheet.setup[0].frozen,
        "nothing the Job holds freezes setup"
    );
    assert!(sheet.worktree_on_disk && !sheet.drone_working && !sheet.worktree_differs);
    let edited = Some(rfc3339_utc(EDITED * 1_000));
    assert_eq!(
        sheet
            .manifest_edited_at
            .as_ref()
            .map(|at| at.as_str().to_string()),
        edited
    );

    std::fs::write(
        tree.join("armada.yml"),
        MANIFEST.replace("echo whole; exit 3", "echo theirs"),
    )
    .expect("the worktree's file moves on");
    git(
        &tree,
        &["commit", "--quiet", "-am", "edited after"],
        Some(EDITED_AFTER),
    );
    let sheet = fleet.run_sheet(job.id()).await.expect("a sheet");
    assert!(sheet.worktree_differs);
    assert_eq!(sheet.checks[0].run, "/bin/sh -c 'echo whole; exit 3'");
    assert_eq!(
        sheet
            .manifest_edited_at
            .as_ref()
            .map(|at| at.as_str().to_string()),
        edited,
        "the edit before the Job froze the file, not the latest"
    );

    let mut watching = events.subscribe();
    let theirs = Arc::clone(&fleet)
        .start_rehearsal(
            job.id(),
            StartRun {
                name: "test".to_string(),
                narrowed: false,
                worktree_version: true,
            },
        )
        .await
        .expect("the worktree's version, asked for");
    assert_eq!(theirs.command, "/bin/sh -c 'echo theirs'");
    let ended = finished(&mut watching, &theirs.id).await;
    assert!(ended.worktree_version && !ended.frozen);
    assert_eq!(ended.exit_code, Some(0));
}

/// What the sheet cannot run is refused before anything runs, and one run at
/// a time is the rule in one tree.
#[tokio::test]
async fn a_name_nothing_declares_a_second_run_and_a_narrowing_of_nothing_are_refused() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_rehearsing(&home, &events);
    let (job, _) = a_job_with_work_in_it(&fleet, &home).await;

    let unknown = Arc::clone(&fleet)
        .start_rehearsal(job.id(), asked("deploy", false))
        .await
        .expect_err("nothing declares it");
    assert_eq!(unknown.status(), 422);
    let no_narrow = Arc::clone(&fleet)
        .start_rehearsal(job.id(), asked("fmt", true))
        .await
        .expect_err("`fmt` declares no narrowing");
    assert_eq!(no_narrow.status(), 422);

    let first = Arc::clone(&fleet)
        .start_rehearsal(job.id(), asked("test", true))
        .await
        .expect("underway");
    let sheet = fleet.run_sheet(job.id()).await.expect("a sheet");
    assert_eq!(sheet.running.map(|out| out.id), Some(first.id.clone()));
    let second = Arc::clone(&fleet)
        .start_rehearsal(job.id(), asked("fmt", false))
        .await
        .expect_err("one run at a time in one tree");
    assert_eq!(second.status(), 409);
    fleet
        .stop_rehearsal(job.id(), first.id.clone())
        .await
        .expect("stopped");
    let again = fleet
        .stop_rehearsal(job.id(), first.id)
        .await
        .expect_err("already over");
    assert_eq!(again.status(), 409);
}
