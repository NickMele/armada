//! Being told is not the same as being spoken to.
//!
//! **The chain's fourth stage asks whether a Drone answered a directive, and
//! that question is only fair once the Drone has one.** Injection lands at a
//! turn boundary — spike 4 measured 1.59s between two fast calls and 33.14s
//! inside a slow one — so a session busy in a tool call holds nothing Fleet has
//! written until that call returns. Everything here is about the gap.
//!
//! Its sibling cases live in the parent module, which owns the fixtures: this
//! is the one subject inside the chain that is not about what a look found.

use core_model::JobStatus;
use ipc;
use testkit::FakeHarness;

use super::{a_chain_that_will_reach_the_trigger, called, next_stage, started, turns};
use crate::converging::Stage;
use crate::tests::tmp::TempDir;

/// A Drone told to report while inside a tool call, and so handed nothing.
///
/// **It never reads its stdin, which is the whole model.** Injection lands at a
/// turn boundary, so a session busy in a call takes the turn only once that
/// call returns — and a turn never taken is never echoed back.
fn a_drone_still_inside_a_call(calls: u32) -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]).reading("BUSY", called(calls))
}

/// **The grace runs from when the Drone was handed the directive, not from when
/// Fleet wrote it.**
///
/// Job `01M21BKVPW002DC0ATD1X9T0VF`: told at 02:37:42, handed the turn at
/// 02:39:14 — 92s, injection waiting on the call in flight — wrote its
/// deliverable at 02:39:40, stopped at `no_report` at 02:39:42. It obeyed in 26
/// seconds and had 28 of its 120 left by the time it could hear, and the record
/// read as a Drone ignoring a directive it had never been given.
///
/// **Two seconds of grace over forty turns**, which is twenty of them. A
/// deadline measured from the write fires on the first.
#[tokio::test]
async fn a_drone_that_has_not_been_handed_the_directive_is_not_stopped_for_ignoring_it() {
    let home = TempDir::new();
    let fleet = a_chain_that_will_reach_the_trigger(&home, a_drone_still_inside_a_call(90));
    let job_id = started(&fleet, &home).await;

    let asked = next_stage(&fleet, "told the Drone to report").await;
    assert!(matches!(asked.stage, Stage::AskedToReport { .. }));

    let after: Vec<_> = turns(&fleet, 40)
        .await
        .into_iter()
        .filter(|said| matches!(said.stage, Stage::Escalated { .. }))
        .collect();
    assert!(
        after.is_empty(),
        "the turn never reached this Drone, so it has not failed to answer one: {after:?}"
    );
    assert_eq!(
        fleet.load(&job_id).await.unwrap().status(),
        JobStatus::Running,
        "the Job is still the Drone's, and no person has been handed anything"
    );
}

/// **The fixture's model of delivery is pinned to what Fleet actually writes.**
///
/// `testkit::REPLAYED` reads an echoed turn back as Armada's by matching the
/// head of the line, because the gate keeps untyped JSON reads inside
/// `store` and `ipc`. That is a prefix against a serialisation, so it can drift in
/// silence — a field added above `type`, a `rename` changed — and the drift
/// would not fail anything. It would make every fake Drone one that was never
/// handed a turn, which is a green suite and a chain that cannot escalate.
#[test]
fn a_turn_fleet_writes_is_read_back_as_armadas() {
    let written = ipc::encode(&crate::session::Turn::first("do the work"))
        .expect("a turn Fleet would put down the pipe");
    assert!(
        written.starts_with(testkit::REPLAYED),
        "the fake reads an echo by this prefix and Fleet no longer writes it: {written}"
    );
    assert!(
        matches!(
            adapter_traits::AgentHarness::read(&testkit::FakeHarness::that_listens(), &written)
                .as_slice(),
            [adapter_traits::DroneEvent::Said {
                by: adapter_traits::Speaker::Armada,
                ..
            }]
        ),
        "an echoed turn is the harness saying it handed one over"
    );
}
