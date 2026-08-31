//! A worktree made workable before the first Drone is put on it.
//!
//! **Real processes, and they have to be.** The subject is whether a command
//! ran in a directory, so a fake runner would assert that Fleet called
//! something rather than that anything happened — which is exactly the class of
//! green that let #227 ship. `/bin/mkdir` and `/usr/bin/false` are on every
//! machine Armada runs on, and neither needs a shell, which `checks_runner` does
//! not give one.
//!
//! **`mkdir` rather than `touch`, and that is the once-per-worktree test.** A
//! second `mkdir` over the same path exits non-zero, so a Job that reaches its
//! second step still running is a Job whose preparation ran exactly once. There
//! is no counter and nothing to read: the command counts itself.

use std::path::Path;

use adapter_traits::WorktreeSpec;
use config::Manifest;
use core_model::{EscalationTrigger, JobStatus, StepState, TransitionReason};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// A Fleet whose `armada.yml` declares one Command and requires it.
///
/// **Through `Manifest::parse` rather than by building the value**, because the
/// refusal that makes `setup.requires` safe is at load: a fixture assembled
/// past the parser would be testing a Fleet no `armada.yml` could produce.
fn a_fleet_requiring(home: &TempDir, run: &str) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.manifest = Manifest::parse(
        Path::new("armada.yml"),
        &format!(
            "version: 1\nid: 01FIXTUREMANIFEST\ncommands:\n  bootstrap:\n    run: {run}\n\
             setup:\n  requires: [bootstrap]\n"
        ),
    )
    .expect("a manifest that requires one command");
    Fleet::assembled(fittings)
}

/// Where the Job's worktree is, derived the way Fleet derives it.
fn worktree(home: &TempDir, job: &core_model::JobId) -> std::path::PathBuf {
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), job.as_str()).expect("a legal spec");
    std::path::PathBuf::from(spec.worktree_path())
}

/// **(a)** What the Manifest requires has run, in the worktree, by the time the
/// first step is running.
///
/// The relative path is the assertion that matters as much as the file: a
/// command run from Fleet's own working directory would leave the marker
/// somewhere else entirely, which is how a repository ends up prepared and its
/// Job's worktree does not.
#[tokio::test]
async fn what_the_manifest_requires_has_run_in_the_worktree_before_any_drone() {
    let home = TempDir::new();
    let fleet = a_fleet_requiring(&home, "/bin/mkdir prepared");

    let job = fleet
        .propose(a_proposal("a Job in a worktree that needs preparing"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());

    let approved = fleet.approve(job.id()).await.expect("dispatch runs");
    assert_eq!(approved.status(), JobStatus::Running);
    assert_eq!(
        approved.current_step_id().map(|id| id.as_str()),
        Some("implement")
    );
    assert!(
        worktree(&home, job.id()).join("prepared").is_dir(),
        "the required command ran, and it ran in the Job's own worktree"
    );
}

/// **(b)** A repository requiring nothing is untouched by any of this.
#[tokio::test]
async fn a_repository_that_requires_nothing_dispatches_exactly_as_before() {
    let home = TempDir::new();
    // The fixture Manifest declares no `setup` at all.
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let job = fleet
        .propose(a_proposal("a Job needing nothing"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());

    let approved = fleet.approve(job.id()).await.expect("dispatch runs");
    assert_eq!(approved.status(), JobStatus::Running);
}

/// **(c)** A required command that fails escalates the Job, enters no step, and
/// the reason names the command.
///
/// **All three, because any two of them would have shipped #227 again.** A Job
/// that escalates without naming the command is the mystery this issue is
/// about; a Job whose step is marked `running` over a worktree nothing prepared
/// hands the next reader a failure that looks like the Drone's work.
#[tokio::test]
async fn a_required_command_that_fails_escalates_the_job_and_enters_no_step() {
    let home = TempDir::new();
    let fleet = a_fleet_requiring(&home, "/usr/bin/false");

    let job = fleet
        .propose(a_proposal("a Job whose install will not run"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());

    let refused = fleet
        .approve(job.id())
        .await
        .expect_err("a worktree that could not be prepared is not dispatched");
    let Adrift::NotPrepared { cause, .. } = &refused else {
        panic!("expected NotPrepared, got {refused:?}");
    };
    assert_eq!(cause.command, "bootstrap");
    assert_eq!(cause.run, "/usr/bin/false");
    let said = refused.to_string();
    assert!(
        said.contains("bootstrap") && said.contains("/usr/bin/false"),
        "the sentence a person reads names the command and the line: {said}"
    );

    let stopped = fleet
        .load(job.id())
        .await
        .expect("the Job is still readable");
    assert_eq!(stopped.status(), JobStatus::Escalated);
    assert_eq!(
        fleet.last_reason(job.id()).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Interrupted)),
        "the same trigger every other pre-flight failure in `dispatch` takes"
    );
    assert!(
        stopped
            .steps()
            .iter()
            .all(|step| step.state() == StepState::NotStarted),
        "no step was entered, so nothing can read this as failing work"
    );
    assert!(fleet.working_on().await.is_empty(), "the slot came free");
}

/// **(d)** One install for a Job, however many Drones it spawns.
///
/// A Drone belongs to a step, so this two-step Job puts two Drones on one
/// worktree. `mkdir` over an existing directory exits non-zero, so a second
/// preparation would escalate the Job at the step boundary — and the Job
/// reaching `summarise` still running is the whole assertion.
#[tokio::test]
async fn preparation_runs_once_for_the_worktree_and_not_once_per_drone() {
    let home = TempDir::new();
    let fleet = a_fleet_requiring(&home, "/bin/mkdir prepared");

    let job = fleet
        .propose(a_proposal("a Job with two steps"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.expect("dispatch runs");

    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("filed");
    fleet.turn().await.expect("the first step advances");

    let midway = fleet.load(job.id()).await.expect("readable");
    assert_eq!(
        midway.status(),
        JobStatus::Running,
        "a second `mkdir prepared` would have failed, so this Job's preparation ran once"
    );
    assert_eq!(
        midway.current_step_id().map(|id| id.as_str()),
        Some("summarise"),
        "the second Drone is on the same worktree, and nothing prepared it again"
    );

    submitted_by_the_one(&fleet, note_evidence())
        .await
        .expect("filed");
    fleet.turn().await.expect("the Job finishes");
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::CompletedSuccess
    );
}
