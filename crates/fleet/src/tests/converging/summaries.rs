//! Every `JobSummary` a client hears about after this Job escalates, and
//! whether each one carries the reason its transition stored.
//!
//! Split out of the parent module on the line budget rather than the
//! subject, `handed`'s own reason for existing separately: this shares the
//! parent's fixtures and its one chain, and asks a question about what
//! reaches the stream rather than about the chain itself.

use std::time::Duration;

use core_model::{EscalationTrigger, TransitionReason};

use super::{
    a_chain_that_will_reach_the_trigger, a_drone_that_will_not_answer, next_stage, started,
};
use crate::converging::Stage;
use crate::tests::tmp::TempDir;

/// **`drone.exited` used to build its embedded row through the blanket
/// `From<&core_model::Job>`, which hardcodes `reason: None`.** So the event
/// published when the cap's kill is reaped silently erased the `no_report`
/// reason `job.state_changed` had just carried, on the last event a client
/// hears about this row until a person acts on it. `Fleet::published` is the
/// fix, and `drone.exited` here is what would have failed without it.
///
/// **`job.step_advanced` is pinned too, and it is not the same claim.** In
/// this chain it fires stopping the step, one line before the Job moves to
/// `escalated` — `converging`'s own ordering — so the row it carries here is
/// still `running` and has no escalation reason to be dropped. What that
/// leaves to assert is the thing `published` actually changed for this call
/// site: the row is read fresh rather than defaulted, so it is `running`'s
/// own reason (none, for an ordinary dispatch) rather than a stale one a
/// blanket `From` would have produced identically — which is why this alone
/// cannot tell the fix from its absence, and `drone.exited` above is what
/// does.
///
/// **The subscription opens before the Job starts**, for the reason every
/// stream test in this workspace takes it that way: nothing published in
/// between is lost.
#[tokio::test]
async fn every_summary_published_after_the_escalation_carries_its_reason() {
    let home = TempDir::new();
    let fleet = a_chain_that_will_reach_the_trigger(&home, a_drone_that_will_not_answer(90));
    let events = fleet.events();
    let mut watching = events.subscribe();
    started(&fleet, &home).await;

    next_stage(&fleet, "told the Drone to report").await;
    let escalated = next_stage(&fleet, "escalated").await;
    assert!(matches!(escalated.stage, Stage::Escalated { .. }));

    // The reap is asynchronous — the cap signals the process group and a
    // later turn notices the pipe closed, which is when `drone.exited`
    // publishes. `the_escalation_gives_the_place_back_and_fires_once`'s own
    // wait.
    for _ in 0..100 {
        if fleet.working_on().await.is_empty() {
            break;
        }
        fleet.turn().await.expect("a turn");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(
        fleet.working_on().await.is_empty(),
        "the Drone the cap ended was never reaped"
    );

    let expected = ipc::Reason::of(&TransitionReason::Escalation(EscalationTrigger::NoReport));
    assert!(expected.is_some(), "no_report carries a wire spelling");

    let (mut saw_state_changed, mut saw_step_advanced, mut saw_drone_exited) =
        (false, false, false);
    // Drained rather than counted: how many events the chain above produced
    // is not this test's claim, and a fixed count would break the moment
    // another one is added upstream of the escalation.
    while let Ok(Some(api::Next::Send(delivered))) =
        tokio::time::timeout(Duration::from_millis(200), watching.next()).await
    {
        match delivered.event {
            ipc::Event::JobStateChanged(changed) if changed.to.as_wire() == "escalated" => {
                assert_eq!(
                    changed.reason, expected,
                    "job.state_changed dropped the reason — the escalation itself"
                );
                saw_state_changed = true;
            }
            ipc::Event::JobStepAdvanced(advanced) => {
                assert_eq!(
                    advanced.job.status.as_wire(),
                    "running",
                    "this chain stops the step one line before the Job escalates"
                );
                assert_eq!(
                    advanced.job.reason, None,
                    "the row this call site reads fresh is running's own reason, not a stale one"
                );
                saw_step_advanced = true;
            }
            ipc::Event::DroneExited(exited) => {
                assert_eq!(
                    exited.job.status.as_wire(),
                    "escalated",
                    "the Drone that just ended was the one the cap stopped"
                );
                assert_eq!(
                    exited.job.reason, expected,
                    "drone.exited dropped the reason — the defect this test is for"
                );
                saw_drone_exited = true;
            }
            _ => {}
        }
    }
    assert!(
        saw_state_changed,
        "job.state_changed for the escalation never arrived"
    );
    assert!(
        saw_step_advanced,
        "job.step_advanced for the stopped step never arrived"
    );
    assert!(
        saw_drone_exited,
        "drone.exited for the reaped Drone never arrived"
    );
}
