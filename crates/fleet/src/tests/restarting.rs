//! What a restarted step starts from.
//!
//! # The gap these exist for
//!
//! `restart_step` put a fresh Drone on a worktree whose branch had not moved
//! since it was cut. A step restarted an hour after it stopped began from
//! wherever the base was then, and nothing anywhere said so — `#180`. The
//! catch-up is not called by `restart_step`; it is inside `put_a_drone_on`,
//! which every spawn goes through, so what is asserted here is that the funnel
//! is reached rather than that a call was added.
//!
//! # The Job is stopped by hand and the Drone leaves on its own
//!
//! `crate::tests::overruling` drives a real refusal, and does so because an
//! override is *about* the verdict. A restart is not: it takes any stopped
//! step whatever stopped it, which `crate::tests::stuck` already asserts case
//! by case. So the record is moved the way that file moves it, and what is
//! driven for real is the part under test — the branch, the spawn and what the
//! new Drone is handed.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{BroughtUpToDate, CallDetail, DroneEvent, Standing};
use config::ResolvedWorkflow;
use core_model::{
    Actor, DroneId, EscalationTrigger, JobId, JobStatus, StepId, StepLevelTrigger, StepTarget,
    Target,
};
use testkit::{
    Delivered, Delivering, FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Gate, Sketch,
};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::daemon::{a_proposal, diff_evidence, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::transcript::transcript_of;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const IMPLEMENT: &str = "implement";

/// One step, gated on nothing, so nothing but the acts under test moves the
/// Job.
fn one_step() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: IMPLEMENT,
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn called() -> Vec<DroneEvent> {
    vec![DroneEvent::Called {
        tool: String::from("Read"),
        call: String::from("a-call"),
        detail: CallDetail::of("a file"),
    }]
}

/// A Drone that speaks once and leaves, emptying the slot. **A restart needs
/// one**: `DroneStillThere` refuses the act while a process is there.
fn a_drone_that_leaves() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY"]).reading("BUSY", called())
}

/// A repository three commits ahead of every branch in it, whose rebases
/// replay cleanly.
fn three_commits_behind() -> Delivering {
    Delivering {
        standing: Standing::Behind { commits: 3 },
        rebase: Some(BroughtUpToDate::Clean {
            base: String::from("main"),
            commits: 3,
        }),
        ..Delivering::default()
    }
}

/// The same, where the replay leaves markers in the file both sides touched.
/// **The case a restart is most likely to hit**: it re-runs the same step on a
/// worktree that already holds an attempt at it.
fn conflicting_on(file: &str) -> Delivering {
    Delivering {
        standing: Standing::Behind { commits: 1 },
        rebase: Some(BroughtUpToDate::Conflicted {
            base: String::from("main"),
            files: vec![String::from(file)],
        }),
        ..Delivering::default()
    }
}

fn a_fleet_with(home: &TempDir, harness: FakeHarness, vcs: FakeVcs) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.workflows = one(one_step());
    fittings.vcs = vcs;
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a restart"));
    Fleet::assembled(fittings)
}

/// A Job dispatched, then stopped at its step with the Job escalated over it —
/// the state all four acts on an escalated Job start from.
async fn stopped(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.expect("released to run");

    let record = fleet.load(job.id()).await.expect("the Job reads");
    let record = fleet
        .move_step(
            &record,
            &StepId::new(IMPLEMENT),
            StepTarget::Stopped(
                StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("a step-level trigger"),
            ),
        )
        .await
        .expect("the step stops");
    fleet
        .move_job(
            &record,
            Target::Escalated(EscalationTrigger::GateFailure),
            Actor::Fleet,
        )
        .await
        .expect("the Job escalates over it");
    job.id().clone()
}

/// Until the slot is empty, driving the loop rather than sleeping — the same
/// path a Drone dying in the field takes.
async fn until_reaped(fleet: &Fixture) {
    for _ in 0..400 {
        fleet.turn().await.expect("a turn");
        if fleet.the_only_slot().await.lock().await.is_none() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the Drone never left");
}

/// What the Drone said, once it has said anything. The transcript is written
/// off a queue, so this waits for the row rather than assuming it landed.
///
/// **`pub(super)` so `overruling` can read a brief too.** An override onto a
/// Job whose Drone has gone spawns exactly as a restart does, and a second
/// copy of this poll is a second thing that can be wrong about the queue.
pub(super) async fn until_spoken(home: &TempDir, drone: &DroneId) -> String {
    let path = transcript_of(&home.path().to_string_lossy(), drone);
    for _ in 0..400 {
        if let Ok(said) = std::fs::read_to_string(&path) {
            if !said.trim().is_empty() {
                return said;
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the restarted Drone's transcript stayed empty");
}

/// The Drone the record says is on the Job now.
pub(super) async fn on_it(fleet: &Fixture, job: &JobId) -> DroneId {
    fleet
        .load(job)
        .await
        .expect("the Job reads")
        .assigned_drone()
        .cloned()
        .expect("a Drone arrived")
}

/// **The whole of #180.** A step that stopped is restarted, and the branch it
/// is restarted onto is the one the base is at now rather than the one it was
/// cut from.
///
/// The delta is what is asserted, not the whole list: the dispatch that put the
/// first Drone on the worktree went through the same funnel, and a fake
/// scripted `Behind` is behind on that call too.
#[tokio::test]
async fn a_restart_catches_the_branch_up_before_the_new_drone_starts() {
    let home = TempDir::new();
    let fleet = a_fleet_with(
        &home,
        a_drone_that_leaves(),
        FakeVcs::new().delivering(three_commits_behind()),
    );
    let job = stopped(&fleet, &home).await;
    until_reaped(&fleet).await;
    let before = fleet.vcs().delivered().len();

    fleet
        .restart_step(&job)
        .await
        .expect("a stopped step with a worktree and no Drone restarts");

    assert_eq!(
        fleet.vcs().delivered().split_off(before),
        vec![Delivered::BroughtUpToDate {
            branch: format!("armada/{}", job.as_str()),
            base: String::from("main"),
        }],
        "the restarted step starts from the base as it stands, not as it was cut"
    );
    assert_eq!(
        fleet.load(&job).await.expect("the Job reads").status(),
        JobStatus::Running,
        "catching up is not a verdict — the restart carries on"
    );
}

/// **A rebase that conflicts on a restart has a reader, and the reader is the
/// Drone the restart spawned.** There is no session at the moment the rebase
/// runs, so the conflict rides the opening brief instead of an injected turn.
///
/// It is read off the transcript rather than off the config, because what is in
/// doubt is not that Fleet assembled the block — that is asserted in
/// `crate::tests::briefing` — but that the block reached the far end of the
/// pipe on the turn a restarted Drone actually gets.
#[tokio::test]
async fn a_conflicted_rebase_on_a_restart_is_the_new_drones_opening_work() {
    let home = TempDir::new();
    let fleet = a_fleet_with(
        &home,
        // Echoes its first turn back, which is how the brief becomes something
        // a test can read.
        FakeHarness::that_echoes_its_first_turn(),
        FakeVcs::new().delivering(conflicting_on("src/parse.rs")),
    );
    let job = stopped(&fleet, &home).await;
    until_reaped(&fleet).await;

    fleet.restart_step(&job).await.expect("a restart");

    let said = until_spoken(&home, &on_it(&fleet, &job).await).await;
    assert!(
        said.contains("conflict markers in them"),
        "the Drone was not told there was anything to resolve: {said}"
    );
    assert!(
        said.contains("src/parse.rs"),
        "and it was not told which file: {said}"
    );
    assert!(
        said.contains("WHY THIS PART IS BEING DONE AGAIN"),
        "the branch block did not displace the reason the step is being re-run: {said}"
    );
}

/// **#131's ordering, on the path #180 added.** The restart's baseline is read
/// after the restart's rebase, so a Drone that resolves none of the markers it
/// was handed fails `diff_nonempty` rather than passing on git's output.
///
/// The discrimination is in the fake's revision counter: a rebase writing into
/// a path that is already there changes what the worktree holds without
/// changing which paths are in it. Read before the rebase, the baseline is one
/// revision behind and a Drone that touched nothing looks like a Drone that
/// worked.
#[tokio::test]
async fn a_restart_that_resolves_none_of_a_conflicted_rebase_fails_its_diff_check() {
    let home = TempDir::new();
    let work = FakeWorkProduct::untouched();
    let mut fittings = fitted_with(&home, FakeWorkProduct::untouched(), a_drone_that_leaves());
    fittings.workflows = one(testkit::resolved(&[Sketch {
        id: IMPLEMENT,
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::DiffNonempty],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]));
    fittings.vcs = FakeVcs::new()
        .delivering(conflicting_on("src/parse.rs"))
        .writing_into(work.holding(), &["src/parse.rs"]);
    fittings.work = work;
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a restart"));
    let fleet = Fleet::assembled(fittings);

    let job = stopped(&fleet, &home).await;
    until_reaped(&fleet).await;
    fleet.restart_step(&job).await.expect("a restart");

    // The restarted Drone resolves nothing and submits anyway.
    fleet
        .submitted_by_the_one(diff_evidence())
        .await
        .expect("the tool took it");
    let turned = fleet.turn().await.expect("the gate ruled");
    let Some(Ruling::Failed { failures, .. }) = &turned.ruled() else {
        panic!(
            "the restart advanced on markers it did not resolve: {:?}",
            turned.ruled()
        );
    };
    assert_eq!(failures, &[verification::CheckFailed::DiffEmpty]);
}

/// A branch that is already current is not rebased on a restart either, and the
/// Drone is told nothing about its branch.
///
/// **The no-op is the ordinary case and it has to stay silent.** A first turn
/// that opens by describing a rebase that did not happen spends the Drone's
/// attention on nothing, which is the same argument `caught_up` makes about a
/// turn at a boundary.
#[tokio::test]
async fn a_restart_onto_a_current_branch_rebases_nothing_and_says_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_with(
        &home,
        FakeHarness::that_echoes_its_first_turn(),
        FakeVcs::new(),
    );
    let job = stopped(&fleet, &home).await;
    until_reaped(&fleet).await;

    fleet.restart_step(&job).await.expect("a restart");

    assert!(
        fleet.vcs().delivered().is_empty(),
        "nothing was behind anything, on either spawn"
    );
    let said = until_spoken(&home, &on_it(&fleet, &job).await).await;
    assert!(
        !said.contains("THE BRANCH YOU ARE ON"),
        "a branch that did not move is not announced: {said}"
    );
}
