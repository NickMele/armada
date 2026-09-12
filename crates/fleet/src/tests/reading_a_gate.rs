//! A Job is readable while its own gate is running.
//!
//! **The half of `#628` nothing proved.** Fleet publishes `job.checking` as
//! each Check starts and finishes, and `crate::tests::underway` holds that it
//! does — but the event is only a wake-up, and what a surface draws comes from
//! the `get_job` it makes on hearing one. The turn held the Job's slot across
//! its whole gate and `get_job` reads that slot, so every such read waited for
//! the Checks and the Judge to finish and then answered with the gate already
//! down. A person watching saw nothing for minutes and then everything at once.
//!
//! A real command, for `crate::tests::checking`'s reason: the claim is about
//! one read landing while another thing is genuinely still running.

use std::time::{Duration, Instant};

use api::Queries;
use core_model::JobStatus;
use testkit::{FakeWorkProduct, Gate, Sketch};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, diff_evidence, fittings, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// How long the step's one Check runs for. Long enough that a read taken
/// while it runs is unambiguous, short enough not to pace the suite.
const WHILE_IT_RUNS: Duration = Duration::from_secs(2);

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
        let began = Instant::now();
        while began.elapsed() < WHILE_IT_RUNS {
            let detail = Queries::get_job(&fleet, ipc::JobId::from(job.id()))
                .await
                .expect("the Job is there to read");
            let took = began.elapsed();
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
        "the read answered after the Check had finished, in {took:?}"
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
