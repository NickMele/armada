//! A Drone whose run has ended, and a process that is still there.
//!
//! # What this measured before it was fixed
//!
//! `#314`: a Job read `Working` while its own log said the Drone run had
//! ended. Run against the code as it stood, the first case below did escalate
//! — `Poked { spent: 1 }` at the first silence window, `Poked { spent: 2 }` at
//! the second, `Escalated { pokes: 2 }` under `stalled` at the third. The
//! snapshot in the issue was taken 2m07s after the run ended, four minutes
//! short of that, and every "67 minutes" in it is the step's own age.
//!
//! What was wrong was the response — two model runs asking a process that had
//! stopped whether it had stopped — and then the record. The escalation
//! shipped first and left the step `running`, because an idle process is still
//! reachable and `restart_step` refuses while a Drone holds the slot. **A
//! `running` step beneath a Drone that has said it is finished is false,
//! whatever it buys.** Fleet reaps it, the step stops under `run_ended`, and
//! the slot comes back — which is what `restart_step` needs.
//!
//! # These start a real child and a real shell
//!
//! The cases differ by one line of shell: whether the child prints a line
//! decoding to a terminating event, whether it prints a tool call first, and
//! whether anything but the child holds the write end when Fleet signals it.
//! Each stays alive holding its pipe, which is what `#371` leaves behind.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use config::ResolvedWorkflow;
use core_model::{
    EscalationTrigger, JobStatus, StepId, StepLevelTrigger, StepState, StepVerdict, Timestamp,
    TransitionReason,
};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::clock::Clock;
use crate::daemon::Fleet;
use crate::silence::{Liveness, Vigil};
use crate::tests::daemon::{a_proposal, diff_evidence, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The production threshold, so what these exercise is the number that ships.
const QUIET_AFTER: Duration = Duration::from_secs(120);

/// How long a case waits for a real child to speak before it calls itself
/// broken. Generous for `crate::tests::silence`'s reason: every wait polls and
/// breaks the moment the pipe moves, so it is only ever spent on a run that is
/// actually wrong.
const A_CHILD_HAS_LONG_ENOUGH: Duration = Duration::from_secs(30);

/// A clock that ticks a second per reading, and jumps when a test says so.
struct Held {
    ticks: AtomicU64,
    pushed: AtomicU64,
}

impl Held {
    fn started() -> Held {
        Held {
            ticks: AtomicU64::new(0),
            pushed: AtomicU64::new(0),
        }
    }

    fn on(&self, seconds: u64) {
        self.pushed.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for Held {
    fn now(&self) -> Timestamp {
        let at = self.ticks.fetch_add(1, Ordering::SeqCst) + self.pushed.load(Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-09-02T{:02}:{:02}:{:02}.000Z",
            (at / 3_600) % 24,
            (at / 60) % 60,
            at % 60
        ))
    }
}

/// One step, gated on nothing, so nothing but the vigil can move it.
fn one_step() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// One tool call, as the transcript would carry it.
fn called() -> DroneEvent {
    DroneEvent::Called {
        tool: String::from("Read"),
        call: String::from("a-call"),
        detail: CallDetail::of("a file"),
    }
}

/// The run ending, with the incident's own numbers on it.
fn ended(refusals: usize) -> DroneEvent {
    DroneEvent::Ended {
        turns: 8,
        cost_micros: 9_473_688,
        refusals,
    }
}

/// The incident's Drone: it worked, its run ended, and its process stayed.
///
/// **`sleep 30` is the whole of what makes this the case it is.** A child that
/// exited would close its pipe, and `Fleet::reap` would fold the same events
/// into the same ending on the same turn — that road has worked all along. What
/// nothing reached was a run that ended over a process that did not.
fn a_drone_whose_run_ends() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo CALLED; echo ENDED; sleep 30"])
        .reading("CALLED", vec![called()])
        .reading("ENDED", vec![ended(0)])
}

/// The same, having reached for nothing at all.
fn a_drone_that_ends_having_called_nothing() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo ENDED; sleep 30"])
        .reading("ENDED", vec![ended(0)])
}

/// The same, having been refused every call it made.
fn a_drone_that_ends_refused() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo CALLED; echo ENDED; sleep 30"])
        .reading("CALLED", vec![called()])
        .reading("ENDED", vec![ended(3)])
}

/// The same again, with something of its own still holding the pipe.
///
/// **`&` is the whole of `#371` in one character.** The backgrounded `sleep`
/// inherits the same write end and is not the process Fleet signalled, so the
/// child dies and the pipe stays open — which is a drain that would wait
/// forever if it were not bounded, on the turn loop, for the whole fleet.
fn a_drone_whose_tools_outlive_it() -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &["-c", "echo CALLED; echo ENDED; sleep 30 & sleep 30"],
    )
    .reading("CALLED", vec![called()])
    .reading("ENDED", vec![ended(0)])
}

/// A Drone that talks and never comes to rest. **The other reading**, and the
/// one the poke ladder is still for.
fn a_drone_that_goes_quiet() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo CALLED; sleep 30"])
        .reading("CALLED", vec![called()])
}

/// A Fleet watching one step with that Drone on it, and a Judge that fails
/// every call — **nothing here may ask a model anything**, for the reason
/// `crate::tests::silence` gives: a Judge that answered would let a regression
/// into the free half pass unseen.
fn a_watched_fleet(home: &TempDir, harness: FakeHarness, clock: Arc<Held>) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.workflows = one(one_step());
    fittings.clock = clock;
    fittings.liveness = Liveness::of(QUIET_AFTER, 2);
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about an ending"));
    Fleet::assembled(fittings)
}

/// Approve the Job and hand back its id, with a worktree on disk.
async fn started(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .unwrap();
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.unwrap();
    job.id().clone()
}

/// How many events the Drone in the slot has produced. **A count and never the
/// content**: what a Drone said is a claim the gate exists to refuse.
async fn heard(fleet: &Fixture) -> usize {
    fleet
        .the_only_slot()
        .await
        .lock()
        .await
        .as_ref()
        .map(|at_work| at_work.heard().len())
        .unwrap_or_default()
}

/// Wait until the child has produced `how_many` events.
///
/// **A poll on the pipe rather than a sleep beside it**, and the precondition
/// every case here needs: the transcript is drained on a task of its own, so a
/// case that turned before the ending landed would be a case about a Drone that
/// was still talking.
async fn spoke(fleet: &Fixture, how_many: usize) -> bool {
    let deadline = tokio::time::Instant::now() + A_CHILD_HAS_LONG_ENOUGH;
    while tokio::time::Instant::now() < deadline {
        if heard(fleet).await >= how_many {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    false
}

/// The incident, and the whole claim: **one turn, no poke, and four rows that
/// agree with each other.**
///
/// The clock never moves. That is what says the escalation was not the silence
/// ladder arriving early — nothing here is past any threshold, and the reading
/// that fired is free.
#[tokio::test]
async fn a_run_that_ends_with_nothing_submitted_escalates_on_the_next_turn() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_watched_fleet(&home, a_drone_whose_run_ends(), Arc::clone(&clock));
    let job = started(&fleet, &home).await;
    assert!(spoke(&fleet, 2).await, "the run never ended");

    // Read while the Drone is still held, so what is asserted below is measured
    // against a process demonstrably alive rather than assumed to be.
    let pid = fleet
        .the_only_slot()
        .await
        .lock()
        .await
        .as_ref()
        .expect("a Drone is working")
        .session()
        .pid();

    let turned = fleet.turn().await.expect("a turn");
    let quiet = turned
        .quiet()
        .expect("the vigil said nothing about the ending");
    assert!(
        matches!(
            quiet.said,
            Vigil::AtRest {
                found: EscalationTrigger::Stalled
            }
        ),
        "it worked and submitted nothing, which is `stalled`: {:?}",
        quiet.said
    );

    let record = fleet.load(&job).await.unwrap();
    assert_eq!(record.status(), JobStatus::Escalated);
    assert_eq!(
        fleet.last_reason(&job).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Stalled)),
    );

    // **The step says what became of it, and the Job says what for.** Two
    // readings of one ending, which is `drone_killed`'s arrangement with the
    // actor changed: `stalled` is Job-level by its own registry row and cannot
    // attach here, so a step carrying only the Job's reason would carry
    // nothing — and `restart_step` reads a `stopped` row.
    let step = record.step(&StepId::new("implement")).expect("the step");
    assert_eq!(step.state(), StepState::Stopped);
    assert_eq!(
        step.last_verdict(),
        StepLevelTrigger::of(EscalationTrigger::RunEnded).map(StepVerdict::Failed),
    );

    // **The process, and then the slot.** Either one alone would let the other
    // rot: a killed Drone whose slot is still held is a slot the fleet has lost,
    // and a slot handed back over a live process is two Drones in one worktree.
    assert!(
        !alive(pid),
        "the Drone's run had ended and its process was left running"
    );
    assert!(
        fleet.the_only_slot().await.lock().await.is_none(),
        "the slot was not given back"
    );

    // **And this is what the reap was for.** `restart_step` asks three
    // questions and the escalation on its own answers none of them: it wants a
    // `stopped` step, no Drone standing, and a worktree still on disk. Left
    // `running` under a held slot it refused twice over, so the only act on
    // offer was redispatching the whole Job.
    let restarted = fleet
        .restart_step(&job)
        .await
        .expect("a reaped step is one a person can restart");
    assert_ne!(restarted.status(), JobStatus::Escalated);
}

/// **The reap is bounded, and it says how much it heard.**
///
/// `#371`: a tool the Drone spawned inherits the same write end, so the pipe
/// outlives the process Fleet signalled. The drain gives it two seconds and
/// then gives up — this case is the assertion that it *does* give up, because
/// an unbounded wait here is the turn loop stopped for the whole fleet on a
/// process nothing ever observed.
///
/// **And the slot does not come back quietly.** A transcript cut short is a fold
/// over a prefix, so the line that hands the slot back has to say the reading
/// was incomplete; a person reading `no terminating event ever arrived` beside
/// nothing else would take it for a Drone that vanished.
#[tokio::test]
async fn a_pipe_something_else_holds_open_does_not_hold_the_turn_loop() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_watched_fleet(&home, a_drone_whose_tools_outlive_it(), Arc::clone(&clock));
    let job = started(&fleet, &home).await;
    assert!(spoke(&fleet, 2).await, "the run never ended");

    // Generously above the drain's own two seconds and far below the thirty the
    // backgrounded tool holds the pipe for, so what this bound catches is a
    // wait on the pipe rather than a slow machine.
    let turned = tokio::time::timeout(Duration::from_secs(10), fleet.turn())
        .await
        .expect("the turn loop waited on a pipe nothing was going to close")
        .expect("a turn");
    assert!(
        turned.quiet().is_some(),
        "the vigil said nothing about the ending"
    );

    let record = fleet.load(&job).await.unwrap();
    assert_eq!(record.status(), JobStatus::Escalated);
    let step = record.step(&StepId::new("implement")).expect("the step");
    assert_eq!(step.state(), StepState::Stopped);
    assert!(
        fleet.the_only_slot().await.lock().await.is_none(),
        "the slot was not given back"
    );
    let log = logged(&home, &job);
    assert!(
        log.contains("cut short"),
        "the slot came back on a transcript that was cut short and the log did \
         not say so: {log}"
    );
}

/// **The three endings, told apart.** This road answered `stalled` for all of
/// them while the reaping road folded the same events into three different
/// remedies — a rephrase, a configuration change, and a redispatch — and which
/// one a Job got turned on whether its process happened to exit.
#[tokio::test]
async fn what_the_run_reported_on_its_way_out_is_what_the_job_is_held_for() {
    for (harness, expected) in [
        (
            a_drone_that_ends_having_called_nothing(),
            EscalationTrigger::Silent,
        ),
        (
            a_drone_that_ends_refused(),
            EscalationTrigger::BlockedByPolicy,
        ),
    ] {
        let home = TempDir::new();
        let clock = Arc::new(Held::started());
        let fleet = a_watched_fleet(&home, harness, Arc::clone(&clock));
        let job = started(&fleet, &home).await;
        assert!(spoke(&fleet, 1).await, "the run never ended");

        let mut found = None;
        for _ in 0..40 {
            if let Some(quiet) = fleet.turn().await.expect("a turn").quiet() {
                found = Some(format!("{:?}", quiet.said));
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(found.is_some(), "the vigil said nothing about the ending");
        assert_eq!(
            fleet.last_reason(&job).await.unwrap(),
            Some(TransitionReason::Escalation(expected.clone())),
            "{found:?}"
        );
    }
}

/// **The other half of the pair the issue asks for.** A run that ends after
/// submitting is a Drone waiting on the gate, and the gate is already queued to
/// move it — so nothing here may read the ending as a stall.
#[tokio::test]
async fn a_run_that_ends_after_submitting_is_left_to_the_gate() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_watched_fleet(&home, a_drone_whose_run_ends(), Arc::clone(&clock));
    let job = started(&fleet, &home).await;
    assert!(spoke(&fleet, 2).await, "the run never ended");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the submission");

    for _ in 0..20 {
        let turned = fleet.turn().await.expect("a turn");
        let quiet = turned.quiet();
        assert!(
            quiet.is_none(),
            "the vigil spoke about a Drone whose evidence is at the gate: {quiet:?}"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_ne!(
        fleet.last_reason(&job).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Stalled)),
        "the Drone submitted and its work was read as a stall"
    );
    // **And no step was stopped for it**, which is the half of this case the
    // reap added. The slot is a weaker claim than it looks: the gate ends a
    // Drone itself once the step clears its machine gates — `#140` — so an
    // empty slot here says nothing about who emptied it. A `run_ended` row does.
    assert!(
        fleet
            .load(&job)
            .await
            .unwrap()
            .steps()
            .iter()
            .filter_map(|step| step.last_verdict())
            .all(|verdict| verdict
                != StepVerdict::Failed(
                    StepLevelTrigger::of(EscalationTrigger::RunEnded)
                        .expect("a step-level trigger")
                )),
        "a Drone waiting on the gate was reaped for having come to rest"
    );
}

/// **The reading this one is not.** A Drone that has said nothing and has not
/// ended its run may be inside a long command, so it gets the ladder it always
/// had — and a change that reached for `transcript_ended` or for `boundaries >
/// 0` instead of the baseline would escalate it here without a poke.
#[tokio::test]
async fn a_drone_that_has_not_ended_its_run_still_gets_its_pokes() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_watched_fleet(&home, a_drone_that_goes_quiet(), Arc::clone(&clock));
    let job = started(&fleet, &home).await;
    assert!(spoke(&fleet, 1).await, "the Drone never said anything");

    // Inside the threshold: nothing at all, which is what says the ladder is
    // still what governs this Drone.
    for _ in 0..10 {
        let turned = fleet.turn().await.expect("a turn");
        let quiet = turned.quiet();
        assert!(quiet.is_none(), "a Drone inside the threshold: {quiet:?}");
    }
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Running);

    clock.on(QUIET_AFTER.as_secs() + 60);
    let mut said = None;
    for _ in 0..40 {
        if let Some(quiet) = fleet.turn().await.expect("a turn").quiet() {
            said = Some(format!("{:?}", quiet.said));
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(
        said.as_deref(),
        Some("Poked { spent: 1 }"),
        "a Drone that has not ended its run is poked, not escalated"
    );
}

/// The Job's own activity log, as text.
fn logged(home: &TempDir, job: &core_model::JobId) -> String {
    std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        job,
    ))
    .unwrap_or_default()
}

/// Whether a pid is still running. `kill -0` rather than `libc::kill`, because
/// this crate denies `unsafe` and the one exception is `detach.rs`.
fn alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
