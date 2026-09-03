//! A Drone whose run has ended, and a process that is still there.
//!
//! # What this measured before it was fixed
//!
//! `#314`: a Job read `Working` while its own activity log said the Drone run
//! had ended. Run against the code as it stood, the first case below did
//! escalate — nothing for ten turns inside `quiet_after`, `Poked { spent: 1 }`
//! at the first window, `Poked { spent: 2 }` at the second, and
//! `Escalated { pokes: 2 }` under `stalled` at the third.
//!
//! **So "nothing would ever have" is not what the code said.** The snapshot in
//! the issue was taken 2m07s after the run ended — seven seconds past the first
//! poke's threshold, four minutes short of the escalation — and every "67
//! minutes" in that report is the step's own age. What was wrong was the
//! response: two model runs asking a process that had stopped whether it had
//! stopped, and one badge for three endings the reaping road tells apart.
//!
//! # These start a real child and a real shell
//!
//! The difference between the cases is one line of shell: whether the child
//! prints a line that decodes to a terminating event, and whether it prints one
//! that decodes to a tool call first. Each then holds its pipe open and stays
//! alive, which is the state `#371` leaves behind and the whole reason the
//! reaping road never reached these Jobs.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use config::ResolvedWorkflow;
use core_model::{EscalationTrigger, JobStatus, StepId, StepState, Timestamp, TransitionReason};
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

/// The incident, and the whole claim: **one turn, no poke, and the Drone still
/// there.**
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

    // **Job-level, so no step carries a verdict**, which is `stalled`'s own row
    // in `escalation-triggers.toml` and unchanged by this. The step stays the
    // Drone's, and so does the recourse: the session is alive, so a redirect
    // still reaches it. `kill_drone` is the road to a `stopped` row, and `#313`
    // is what made that road work.
    let step = record.step(&StepId::new("implement")).expect("the step");
    assert_eq!(step.state(), StepState::Running);
    assert_eq!(step.last_verdict(), None);
    assert!(
        alive(pid),
        "a Drone whose run ended was killed; escalating holds one"
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
