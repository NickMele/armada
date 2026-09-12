//! A Job is readable while Fleet is working it — through its gate, and
//! through the one look its turn can pay for.
//!
//! **The half of `#628` nothing proved.** Fleet publishes `job.checking` as
//! each Check starts and finishes, and `crate::tests::underway` holds that it
//! does — but the event is only a wake-up, and what a surface draws comes from
//! the `get_job` it makes on hearing one. The turn held the Job's slot across
//! its whole gate and `get_job` reads that slot, so every such read waited for
//! the Checks and the Judge to finish and then answered with the gate already
//! down. A person watching saw nothing for minutes and then everything at once.
//!
//! The second case is the same defect one watcher along: `watch_convergence`
//! held the slot across its Judge call, which stopped every surface — the whole
//! Board, through `list_jobs` — for the length of a call measured at 15s.
//!
//! A real command and a real child, for `crate::tests::checking`'s reason: the
//! claim is about one read landing while another thing is genuinely running.

use std::time::{Duration, Instant};

use api::Queries;
use core_model::JobStatus;
use testkit::{FakeWorkProduct, Gate, Sketch};

use crate::converging::StepNorms;
use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::converging::{a_drone_that_will_not_answer, a_watched_fleet, one_step};
use crate::tests::daemon::{a_proposal, diff_evidence, fittings, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// How long the Check, and the Judge call below it, take to answer. Long
/// enough that a read landing inside one is unambiguous, short enough not to
/// pace the suite.
const WHILE_IT_RUNS: Duration = Duration::from_secs(2);

/// How long a case will keep reading for. **Wider than the thing it is reading
/// during**, deliberately: the turn reaches its call some way in, so a window
/// the same width as the call is one the reader can fall out of before the
/// call has even started — which reads as the defect and is a slow machine.
const KEEP_READING_FOR: Duration = Duration::from_secs(10);

/// A step gated on one Check that takes [`WHILE_IT_RUNS`] to answer.
fn a_step_whose_check_takes_a_while() -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::Check {
            name: "slow",
            run: "/bin/sleep 2",
            expect_exit_code: 0,
            when: &[],
        }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// **The whole claim.** While the gate is running the step's Check, a
/// `get_job` answers — promptly, and with the Check in flight on it.
///
/// Both halves are the test. A read that answers late with an empty
/// `checking` is exactly what the gate used to serve, and it is
/// indistinguishable from a Job with no Checks at all.
#[tokio::test(flavor = "multi_thread")]
async fn a_job_answers_while_its_own_gate_is_checking() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = one(a_step_whose_check_takes_a_while());
    let fleet = Fleet::assembled(fittings);

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job to work");
    worktree_directory(&home, &job);
    let running = dispatched(&fleet, job.id()).await.expect("it dispatches");
    assert_eq!(running.status(), JobStatus::Running);
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the Drone reports its diff");

    // The turn and the read are driven together, because "it answered while
    // the gate ran" is the only form the claim has. Driving one and then the
    // other would pass against the Fleet this test exists to fail.
    let read = async {
        let mut seen = None;
        let giving_up = Instant::now();
        while giving_up.elapsed() < KEEP_READING_FOR {
            // Timed per read, never from the start of the loop: what the claim
            // is about is one call being served, and a clock started before
            // the gate was even reached measures the turn's own pace instead.
            let asked = Instant::now();
            let detail = Queries::get_job(&fleet, ipc::JobId::from(job.id()))
                .await
                .expect("the Job is there to read");
            let took = asked.elapsed();
            if let Some(step) = detail.steps.iter().find(|step| step.checking.is_some()) {
                seen = Some((step.checking.clone().expect("just matched"), took));
                break;
            }
            tokio::task::yield_now().await;
        }
        seen
    };
    let (turned, seen) = tokio::join!(fleet.turn(), read);
    turned.expect("the gate runs");

    let (checking, took) = seen.expect(
        "no read taken while the gate was running carried its Checks — the turn is holding \
         the Job's slot across the gate again, and every surface is waiting on it",
    );
    assert!(
        took < WHILE_IT_RUNS,
        "the read that carried the live Checks took {took:?} to answer, which is \
         the length of the Check it was waiting behind"
    );
    let names: Vec<&str> = checking
        .checks
        .iter()
        .map(|check| check.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["slow"],
        "the read carried a live gate that is not this step's"
    );
}

/// The Judge's answer to the convergence look. What it says does not matter
/// here; how long it takes to say it does.
const CONVERGING: &str = "state: converging";

/// **The same claim, one watcher along.** While the convergence look's Judge
/// call is out, a `get_job` answers — promptly, and saying that is what the
/// Job is doing.
///
/// The look is the only model call a turn makes outside the gate, and it is
/// made on a Drone that is still working, so a read blocked behind it is a
/// Board that stops while nothing is wrong.
#[tokio::test(flavor = "multi_thread")]
async fn a_job_answers_while_its_convergence_look_is_out() {
    let home = TempDir::new();
    let fleet = a_watched_fleet(
        &home,
        a_drone_that_will_not_answer(90),
        std::sync::Arc::new(testkit::FakeJudge::saying(CONVERGING).taking(WHILE_IT_RUNS)),
        // Five calls against the ninety the Drone reports: the tripwire fires
        // on the first turn, which is what buys the look.
        StepNorms::of(5, Duration::from_secs(86_400), Duration::from_secs(86_400)),
        one_step(None),
    );

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job to work");
    worktree_directory(&home, &job);
    let running = dispatched(&fleet, job.id()).await.expect("it dispatches");
    assert_eq!(running.status(), JobStatus::Running);

    let read = async {
        let mut seen = None;
        let giving_up = Instant::now();
        while giving_up.elapsed() < KEEP_READING_FOR {
            // Timed per read, for the gate case's reason.
            let asked = Instant::now();
            let detail = Queries::get_job(&fleet, ipc::JobId::from(job.id()))
                .await
                .expect("the Job is there to read");
            let took = asked.elapsed();
            if let Some(step) = detail.steps.iter().find(|step| step.judging.is_some()) {
                seen = Some((step.judging.clone().expect("just matched"), took));
                break;
            }
            tokio::task::yield_now().await;
        }
        seen
    };
    // **Turned until the chain speaks, on `converging::next_stage`'s rule.**
    // The tripwire reads a count off a line the Drone has to have printed and
    // Fleet has to have read, so the turn that takes the look is not reliably
    // the first one — a single turn here passed alone and failed under a
    // loaded workspace run, which is the same test asserting the machine's
    // pace.
    let turning = async {
        for _ in 0..400 {
            let turned = fleet.turn().await.expect("a turn");
            if turned.each.iter().any(|worked| worked.wandering.is_some()) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("the chain never looked at the step");
    };
    let (_, seen) = tokio::join!(turning, read);

    let (judging, took) = seen.expect(
        "no read taken while the look was out carried it — the turn is holding the Job's slot \
         across its Judge call again, and every surface is waiting on it",
    );
    assert!(
        took < WHILE_IT_RUNS,
        "the read that carried the live look took {took:?} to answer, which is \
         the length of the call it was waiting behind"
    );
    assert_eq!(
        judging.look, "convergence",
        "the read carried a look that is not the one this turn made"
    );
}
