//! A Drone that has stopped speaking, and the three stages between that and an
//! escalation.
//!
//! # The two cases are the whole claim
//!
//! `docs/spikes/009-how-long-does-a-step-take.md` says silence separates a
//! stuck step from an honest one where elapsed time does not. So the pair that
//! matters is a Drone quiet past the threshold, which is poked and then
//! escalates, and a Drone talking steadily, which is left alone however long
//! its step runs — and the second is the one a tripwire that fires routinely
//! would fail.
//!
//! # These start a real child and a real shell
//!
//! One Drone prints a line and then says nothing; one prints a line every few
//! milliseconds for as long as it is allowed to; one answers whatever is
//! injected into it. That is the whole difference between the three cases, and
//! it is a property of a process rather than of a value.
//!
//! # The clock is pushed rather than waited on
//!
//! [`Held`] ticks a second per reading like `Ticking` does, and takes a shove
//! the test decides. A threshold measured in minutes is not one a test can sit
//! through, and sleeping for it would be a test that is slow *and* timing-
//! dependent — this way the number under test is the real one.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use config::ResolvedWorkflow;
use core_model::{EscalationTrigger, JobStatus, StepState, Timestamp, TransitionReason};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::clock::Clock;
use crate::daemon::Fleet;
use crate::silence::{Liveness, Poke, Quiet, Vigil};
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The threshold every case below is measured against. The production one, so
/// what the tests exercise is the number that ships.
const QUIET_AFTER: Duration = Duration::from_secs(120);

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

    /// Move the clock on. **Never backwards**, which `converging::elapsed`
    /// reads as zero and which no machine should produce.
    fn on(&self, seconds: u64) {
        self.pushed.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for Held {
    fn now(&self) -> Timestamp {
        let at = self.ticks.fetch_add(1, Ordering::SeqCst) + self.pushed.load(Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-08-26T{:02}:{:02}:{:02}.000Z",
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
fn called() -> Vec<DroneEvent> {
    vec![DroneEvent::Called {
        tool: String::from("Read"),
        call: String::from("a-call"),
        detail: CallDetail::of("a file"),
    }]
}

/// A Drone that says one thing and is then quiet for longer than any test runs.
fn a_drone_that_goes_quiet() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]).reading("BUSY", called())
}

/// A Drone that keeps talking. Every line is one tool call, which is the
/// cheapest thing a working Drone emits — a real one also emits a progress
/// heartbeat every thirty seconds inside a long command, which counts the same.
fn a_drone_that_keeps_talking() -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &["-c", "while :; do echo BUSY; sleep 0.01; done"],
    )
    .reading("BUSY", called())
}

/// A Drone that says nothing until it is spoken to, and answers when it is.
fn a_drone_that_answers() -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            "echo BUSY; while IFS= read -r line; do echo ANSWERED; done",
        ],
    )
    .reading("BUSY", called())
    .reading("ANSWERED", called())
}

/// A Fleet watching one step with that Drone on it, and a Judge that fails
/// every call — **nothing here may ask a model anything**, and a Judge that
/// answered would let a regression into the cheap half pass unseen.
fn a_watched_fleet(
    home: &TempDir,
    harness: FakeHarness,
    clock: Arc<Held>,
    liveness: Liveness,
) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.workflows = one(one_step());
    fittings.clock = clock;
    fittings.liveness = liveness;
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about silence"));
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

/// Push the clock past the threshold and turn until the vigil says something.
async fn after_the_threshold(fleet: &Fixture, clock: &Held, waiting_for: &str) -> Quiet {
    clock.on(QUIET_AFTER.as_secs() + 60);
    for _ in 0..400 {
        let turned = fleet.turn().await.expect("a turn");
        if let Some(quiet) = turned.each.into_iter().find_map(|worked| worked.quiet) {
            return quiet;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the vigil never {waiting_for}");
}

/// **The case a tripwire that fires routinely would fail.** The step runs for
/// as long as the quiet one does — the clock is pushed by the same amount, over
/// and over — and the only difference is that the Drone is saying things.
#[tokio::test]
async fn a_drone_that_keeps_talking_is_never_poked() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_watched_fleet(
        &home,
        a_drone_that_keeps_talking(),
        Arc::clone(&clock),
        Liveness::of(QUIET_AFTER, 2),
    );
    started(&fleet, &home).await;

    let mut said = Vec::new();
    for _ in 0..20 {
        // Half the threshold per turn, so twenty turns run the step ten times
        // past a wall clock that would have fired on the honest case.
        tokio::time::sleep(Duration::from_millis(40)).await;
        clock.on(QUIET_AFTER.as_secs() / 2);
        let turned = fleet.turn().await.expect("a turn");
        if let Some(quiet) = turned.each.into_iter().find_map(|worked| worked.quiet) {
            said.push(quiet);
        }
    }
    assert!(
        said.is_empty(),
        "a Drone that is talking was poked: {said:?}"
    );
}

/// The whole point, in one case: quiet past the threshold, poked twice, and
/// escalated as `stalled` when the pokes are spent.
#[tokio::test]
async fn a_drone_silent_past_the_threshold_is_poked_and_then_escalates() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_watched_fleet(
        &home,
        a_drone_that_goes_quiet(),
        Arc::clone(&clock),
        Liveness::of(QUIET_AFTER, 2),
    );
    let job = started(&fleet, &home).await;

    let first = after_the_threshold(&fleet, &clock, "poked the Drone").await;
    assert!(
        matches!(first.said, Vigil::Poked { spent: 1 }),
        "{:?}",
        first.said
    );
    assert!(
        first.after >= QUIET_AFTER,
        "the poke names how long it had been quiet: {:?}",
        first.after
    );
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "one poke escalates nothing"
    );

    let second = after_the_threshold(&fleet, &clock, "poked the Drone again").await;
    assert!(
        matches!(second.said, Vigil::Poked { spent: 2 }),
        "{:?}",
        second.said
    );
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Running);

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

    let last = after_the_threshold(&fleet, &clock, "escalated the Job").await;
    assert!(
        matches!(last.said, Vigil::Escalated { pokes: 2 }),
        "{:?}",
        last.said
    );
    let record = fleet.load(&job).await.unwrap();
    assert_eq!(record.status(), JobStatus::Escalated);
    assert_eq!(
        fleet.last_reason(&job).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Stalled)),
        "the registry's own trigger, and its liveness timer is what fired"
    );
    // **Job-level, so no step carries a verdict.** `stalled` is typed Job-level
    // in `escalation-triggers.toml` — a Drone that stopped producing anything
    // is a fact about the Drone rather than about the step it happened to be
    // on — and only a step-level trigger reaches a step's `last_verdict`.
    let step = record
        .step(&core_model::StepId::new("implement"))
        .expect("the step being worked");
    assert_eq!(step.state(), StepState::Running);
    assert_eq!(step.last_verdict(), None);

    // **Held, not killed** — the difference from `thrashing`. A silent Drone is
    // spending nothing, and the worktree it is sitting in is what a person
    // redispatches onto.
    assert!(
        alive(pid),
        "a Drone that went quiet was ended; escalating holds one"
    );
}

/// A Drone that answers the poke is a Drone that is there. **The poke is spent,
/// the answer is heard, and nothing escalates** — the ordinary outcome, and the
/// reason a nudge comes before a trigger at all.
///
/// What it does *not* claim is that answering buys the step a fresh budget. The
/// pokes are the step's, so a Drone that answers two of them and does nothing
/// else still reaches `stalled` on the third silence — which is the whole of
/// what a Drone answering "still here" every two minutes and producing nothing
/// would otherwise be able to do.
#[tokio::test]
async fn a_drone_that_answers_the_poke_is_heard_and_not_escalated() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_watched_fleet(
        &home,
        a_drone_that_answers(),
        Arc::clone(&clock),
        Liveness::of(QUIET_AFTER, 2),
    );
    let job = started(&fleet, &home).await;
    let before = heard(&fleet).await;

    let poked = after_the_threshold(&fleet, &clock, "poked the Drone").await;
    assert!(
        matches!(poked.said, Vigil::Poked { spent: 1 }),
        "{:?}",
        poked.said
    );

    // **The poke reached the Drone and something came back**, which is what
    // restarts the clock. Read as a count of events and never as their content,
    // for [`Working::came_to_rest`]'s reason.
    let mut answered = false;
    for _ in 0..200 {
        // Inside the threshold on purpose: what is under test is the answer
        // arriving, and a second silence would be a second episode.
        clock.on(QUIET_AFTER.as_secs() / 8);
        let turned = fleet.turn().await.expect("a turn");
        let quiet = turned.quiet();
        assert!(quiet.is_none(), "the vigil spoke again: {quiet:?}");
        if heard(&fleet).await > before {
            answered = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(answered, "the poke was sent and nothing came back");
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "the Drone answered and was escalated anyway"
    );
}

/// How much the Drone in the slot has said so far. **A count and never the
/// content**: whether the Drone answered is a fact about the pipe, and what it
/// answered is a claim the gate exists to refuse.
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

/// The prompt contract is explicit that this turn names elapsed time and never
/// the count: *"this turn must never become 'second of two pokes', which would
/// tell a Drone precisely how long it has left to look busy."*
#[test]
fn the_poke_names_the_minutes_and_not_the_budget() {
    let text = Poke::after(Duration::from_secs(180)).text().to_string();

    assert!(text.contains("3 minutes"), "{text}");
    assert!(!text.contains("poke"), "{text}");
    assert!(
        text.contains("keep going") && text.contains("submit"),
        "the three branches, because the poke cannot tell which one it is in: {text}"
    );
}

/// Whether a pid is still running. `kill -0` rather than `libc::kill`, because
/// this crate denies `unsafe` and the one exception is the `setsid` in
/// `detach.rs` — a test is not a second reason to open that door.
fn alive(pid: u32) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
