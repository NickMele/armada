//! A Drone that has stopped speaking, and the three stages between that and an
//! escalation.
//!
//! Why there are two modules: `poking` is what the vigil does once it has found
//! a silence, and `patience` is which declared threshold decides that it has.
//! The Drones and the Fleet they share are here, because a fixture that drifted
//! apart between them would leave the two arguing about different Drones.
//!
//! # These start a real child and a real shell
//!
//! One Drone prints a line and then says nothing; one prints a line every few
//! milliseconds for as long as it is allowed to; one answers whatever is
//! injected into it. That is the whole difference between the cases, and it is
//! a property of a process rather than of a value.
//!
//! # The clock is pushed rather than waited on
//!
//! [`Held`] ticks a second per reading like `Ticking` does, and takes a shove
//! the test decides. A threshold measured in minutes is not one a test can sit
//! through, and sleeping for it would be a test that is slow *and* timing-
//! dependent — this way the number under test is the real one.

mod patience;
mod poking;

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use config::ResolvedWorkflow;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::daemon::Fleet;
use crate::silence::{Liveness, Quiet};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::planted::Held;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The threshold every case below is measured against. The production one, so
/// what the tests exercise is the number that ships.
const QUIET_AFTER: Duration = Duration::from_secs(120);

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

/// A Fleet watching whichever workflow the case is about, with that Drone on
/// it.
fn watching(
    home: &TempDir,
    workflow: ResolvedWorkflow,
    harness: FakeHarness,
    clock: Arc<Held>,
    liveness: Liveness,
) -> Fixture {
    assembled(home, workflow, harness, clock, liveness, None)
}

/// The same, over a repository that states a patience of its own.
///
/// `says` is the body of an `armada.yml`'s `drone:` section, written the way a
/// repository writes it and put through the real parser — so a case here fails
/// if the key stops being read, rather than passing on a value planted past it.
fn watching_a_repository_that_says(
    home: &TempDir,
    workflow: ResolvedWorkflow,
    harness: FakeHarness,
    clock: Arc<Held>,
    liveness: Liveness,
    says: &str,
) -> Fixture {
    assembled(home, workflow, harness, clock, liveness, Some(says))
}

/// The Judge fails every call — **nothing here may ask a model anything**, and
/// a Judge that answered would let a regression into the cheap half pass unseen.
fn assembled(
    home: &TempDir,
    workflow: ResolvedWorkflow,
    harness: FakeHarness,
    clock: Arc<Held>,
    liveness: Liveness,
    says: Option<&str>,
) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.workflows = one(workflow);
    fittings.clock = clock;
    fittings.liveness = liveness;
    if let Some(says) = says {
        fittings.manifest = config::Manifest::parse(
            std::path::Path::new("armada.yml"),
            &format!("version: 1\nid: 01FIXTUREMANIFEST\ndrone:\n{says}"),
        )
        .expect("a repository that states its own patience");
    }
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about silence"));
    Fleet::assembled(fittings)
}

/// Approve the Job and hand back its id, with a worktree on disk.
async fn started(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .unwrap();
    worktree_directory(home, &job);
    dispatched(&fleet, job.id()).await.unwrap();
    job.id().clone()
}

/// Push the clock past the threshold and turn until the vigil says something.
async fn after_the_threshold(fleet: &Fixture, clock: &Held, waiting_for: &str) -> Quiet {
    clock.on(QUIET_AFTER.as_secs() + 60);
    turning_until_quiet(fleet, waiting_for).await
}

/// Turn until the vigil says something, with the clock left where it is.
///
/// **Separate from [`after_the_threshold`] because which threshold matters.**
/// A case about a step's own patience decides for itself how far to push, and
/// a helper that pushed past the Fleet-wide value would make every case look
/// alike whatever the step declared.
async fn turning_until_quiet(fleet: &Fixture, waiting_for: &str) -> Quiet {
    for _ in 0..400 {
        let turned = fleet.turn().await.expect("a turn");
        if let Some(quiet) = turned.each.into_iter().find_map(|worked| worked.quiet) {
            return quiet;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the vigil never {waiting_for}");
}
