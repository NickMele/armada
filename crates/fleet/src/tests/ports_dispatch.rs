//! End to end: two Jobs whose Manifest declares `ports:` get different spans,
//! a `setup.requires` command sees both `${port.NAME}` and
//! `ARMADA_PORT_<NAME>` for the number it was actually claimed, and the span
//! is free again once the Job ends.
//!
//! **A real shell, not a fake.** `crate::tests::drone` gives the reason one
//! module over: whether a value reached a spawned process's environment is not
//! a property a mock can stand in for.

use std::path::Path;

use adapter_traits::WorktreeSpec;
use config::Manifest;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::slots::Concurrency;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;

/// A Manifest declaring one port, and a `setup.requires` command that writes
/// both channels `${port.NAME}` resolves and `ARMADA_PORT_<NAME>` sets to a
/// file in the worktree — `/bin/sh -c` so the second is a real shell
/// expansion of the environment Fleet built, not a literal Armada never wrote.
///
/// **`concurrency` is a parameter and not the fixture's own one slot.** The
/// fake Drone `fittings` starts holds its slot open — `FakeHarness::that_listens`
/// — so a case asking for a second Job to be dispatched *while the first is
/// still running* needs a second slot to dispatch it into; a case that only
/// ever has one Job in flight takes the fixture's own default.
fn a_fleet_declaring_storybook(
    home: &TempDir,
    concurrency: usize,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.manifest = Manifest::parse(
        Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n\
         ports:\n  storybook: {}\n\
         commands:\n  bootstrap:\n    run: >-\n      /bin/sh -c \"echo ${port.storybook} $ARMADA_PORT_STORYBOOK > port.txt\"\n\
         setup:\n  requires: [bootstrap]\n",
    )
    .expect("a manifest declaring one port");
    fittings.concurrency = Concurrency::of(concurrency);
    Fleet::assembled(fittings)
}

/// What `bootstrap` wrote, as the two numbers it read.
fn port_txt(home: &TempDir, handle: &str) -> (String, String) {
    let spec = WorktreeSpec::for_job(&home.path().to_string_lossy(), handle).expect("a legal spec");
    let written = std::fs::read_to_string(Path::new(&spec.worktree_path()).join("port.txt"))
        .expect("bootstrap wrote port.txt");
    let mut words = written.split_whitespace();
    (
        words
            .next()
            .expect("the resolved ${port.storybook}")
            .to_string(),
        words
            .next()
            .expect("the shell's own $ARMADA_PORT_STORYBOOK")
            .to_string(),
    )
}

/// **Each Command sees its own `${port.storybook}` and
/// `ARMADA_PORT_STORYBOOK`.** Both channels agree with each other and with
/// what the store recorded — nothing here reads a hardcoded number.
#[tokio::test]
async fn a_setup_command_sees_the_resolved_port_and_the_env_var_together() {
    let home = TempDir::new();
    let fleet = a_fleet_declaring_storybook(&home, 1);
    let job = fleet
        .propose(a_proposal("a Job whose worktree needs a port"))
        .await
        .expect("proposed");
    worktree_directory(&home, &job);

    let dispatched = dispatched(&fleet, job.id()).await.expect("dispatch runs");

    let (resolved, from_env) = port_txt(&home, &dispatched.handle());
    assert_eq!(
        resolved, from_env,
        "${{port.storybook}} and $ARMADA_PORT_STORYBOOK name the same claim"
    );
    let claimed = fleet
        .store()
        .lock()
        .await
        .port_span_for_job(dispatched.id())
        .expect("the read succeeds")
        .expect("a claim was made");
    assert_eq!(
        resolved,
        claimed.base.to_string(),
        "the command saw exactly the number the store recorded"
    );
}

/// **Two Jobs declaring `storybook` get different ports.**
#[tokio::test]
async fn two_jobs_declaring_the_same_port_get_different_numbers() {
    let home = TempDir::new();
    // Two slots: both Jobs' Drones must be alive at once for their spans to
    // be provably distinct rather than one reused after the other released.
    let fleet = a_fleet_declaring_storybook(&home, 2);

    let first = fleet
        .propose(a_proposal("the first Job to claim storybook"))
        .await
        .expect("proposed");
    worktree_directory(&home, &first);
    let first = dispatched(&fleet, first.id()).await.expect("dispatch runs");

    let second = fleet
        .propose(a_proposal("the second Job to claim storybook"))
        .await
        .expect("proposed");
    worktree_directory(&home, &second);
    let second = dispatched(&fleet, second.id())
        .await
        .expect("dispatch runs");

    let (first_port, _) = port_txt(&home, &first.handle());
    let (second_port, _) = port_txt(&home, &second.handle());
    assert_ne!(
        first_port, second_port,
        "the two Jobs do not hold the same span"
    );
}

/// **The span is free again once the Job ends.** Release, not a timer.
#[tokio::test]
async fn the_span_is_freed_once_the_job_is_killed() {
    let home = TempDir::new();
    let fleet = a_fleet_declaring_storybook(&home, 1);
    let job = fleet
        .propose(a_proposal("a Job whose span outlives nothing"))
        .await
        .expect("proposed");
    worktree_directory(&home, &job);
    let dispatched = dispatched(&fleet, job.id()).await.expect("dispatch runs");

    assert!(
        fleet
            .store()
            .lock()
            .await
            .port_span_for_job(dispatched.id())
            .expect("the read succeeds")
            .is_some(),
        "the span is claimed while the Job runs"
    );

    fleet
        .kill_job(dispatched.id())
        .await
        .expect("a running Job may be killed");

    assert_eq!(
        fleet
            .store()
            .lock()
            .await
            .port_span_for_job(dispatched.id())
            .expect("the read succeeds"),
        None,
        "the span is released once the Job reaches a terminal status"
    );
}

/// **A Job and the main checkout never overlap.** Both are claimed against
/// the same range, through the same `try_claim`, and read each other's spans
/// back through the same `every_port_claim` before either probes a
/// candidate.
#[tokio::test]
async fn a_job_and_the_main_checkout_never_overlap() {
    let home = TempDir::new();
    let fleet = a_fleet_declaring_storybook(&home, 1);
    let job = fleet
        .propose(a_proposal("a Job beside the main checkout"))
        .await
        .expect("proposed");
    worktree_directory(&home, &job);
    let dispatched = dispatched(&fleet, job.id()).await.expect("dispatch runs");

    let main_checkout = fleet.main_checkout_ports().await;
    let job_ports = fleet.port_map(&dispatched).await;
    assert_ne!(
        main_checkout.get("storybook"),
        job_ports.get("storybook"),
        "the Job and the main checkout do not hold the same port"
    );
}

/// **Claimed on first need, and reused after that.** Two calls with no Job or
/// server in between answer with the same span — there is nothing here that
/// would claim a second one.
#[tokio::test]
async fn the_main_checkouts_span_is_reused_rather_than_reclaimed() {
    let home = TempDir::new();
    let fleet = a_fleet_declaring_storybook(&home, 1);

    let first = fleet.main_checkout_ports().await;
    let second = fleet.main_checkout_ports().await;
    assert_eq!(first, second, "the same claim, read back twice");
    assert_eq!(
        fleet
            .store()
            .lock()
            .await
            .every_port_claim()
            .expect("read")
            .len(),
        1,
        "one claim, not one per call"
    );
}

/// **The span is released on Fleet shutdown.** No Job and no server is
/// involved — the release the composition root calls once, after teardown.
#[tokio::test]
async fn the_main_checkouts_span_is_released_on_shutdown() {
    let home = TempDir::new();
    let fleet = a_fleet_declaring_storybook(&home, 1);
    let claimed = fleet.main_checkout_ports().await;
    assert!(!claimed.is_empty(), "a span was claimed");

    fleet.released_main_checkout_ports().await;

    assert!(
        fleet
            .store()
            .lock()
            .await
            .every_port_claim()
            .expect("read")
            .is_empty(),
        "the main checkout's span is free again"
    );
}
