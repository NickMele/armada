//! Two Jobs worked at once, and each Drone's calls landing on its own step.
//!
//! # A test that passes because it was slow proves nothing
//!
//! Two Jobs admitted in one turn that happen not to interleave is not the
//! claim. The claim is that **both are in the roster at the same moment** and
//! that **each Drone's own tool call reached the step that made it** — so every
//! case here holds both Jobs open across the assertion rather than driving one
//! and then the other, and the two Drones say different things so that a call
//! landing on the wrong step is a failure rather than a coincidence.
//!
//! # And the attribution is doing the work, not the ordering
//!
//! The last case swaps which port each Drone calls from and asserts the
//! declarations swap with it. Without that, every assertion here would also
//! pass against a Fleet that attributed by "whichever Job was admitted first",
//! which is what the single slot did and is the thing `#50` had to stop doing.

use std::sync::Arc;

use adapter_traits::WorktreeSpec;
use core_model::JobId;
use ipc::mcp::DeclareScope;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Gate, Scoped, Sketch};

use crate::daemon::{Fittings, Fleet};
use crate::slots::Concurrency;
use crate::tests::daemon::{a_proposal, fittings, one, worktree_directory};
use crate::tests::peer::Placing;
use crate::tests::planning::A_PLAN;
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;

pub(crate) type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The port Fleet is listening on in these fixtures — the far half of every
/// pair below, and the same number `daemon::fittings` puts on the `Host`.
pub(crate) const SERVED_ON: u16 = 47821;

/// One step that declares a scope, so a Drone has something to say that lands
/// on its own step and nowhere else.
pub(crate) fn one_scoped_step() -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::DiffNonempty],
        judged_on: &[],
        scope: Some(Scoped {
            diff_check: false,
            at_step_start: true,
            exclude: &[],
            references: &[],
        }),
        gaming: None,
    }])
}

/// A Fleet bounded at two, whose callers are placed by a plant the test holds.
pub(crate) fn two_at_once(home: &TempDir) -> (Fixture, Arc<Placing>) {
    let peers = Placing::nothing();
    let mut fittings: Fittings<FakeHarness, FakeVcs, FakeWorkProduct> =
        fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = one(one_scoped_step());
    fittings.concurrency = Concurrency::of(2);
    fittings.peers = Arc::clone(&peers) as Arc<dyn crate::peer::PeerOf>;
    (Fleet::assembled(fittings), peers)
}

/// Propose and approve a Job, with the worktree its dispatch will want.
pub(crate) async fn approved(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = fleet
        .propose(a_proposal(title))
        .await
        .expect("a proposal is drafted");
    worktree_directory(home, job.id());
    fleet
        .approve(job.id())
        .await
        .expect("a person approves it, one by one");
    job.id().clone()
}

/// What this Job's Drone declared, read off its own slot.
async fn declared_on(fleet: &Fixture, job: &JobId) -> Option<Vec<String>> {
    let slot = fleet.slot_of(job).await?;
    let held = slot.lock().await;
    held.as_ref()?.declared().map(|paths| {
        paths
            .paths()
            .iter()
            .map(|path| path.as_str().to_string())
            .collect()
    })
}

/// Say which Drone is calling from which port, by asking Fleet for the pids it
/// is holding. **The plant is the seam and the pids are real** — the mapping a
/// live Fleet reads out of the kernel is the mapping this writes down.
pub(crate) fn calling_from(peers: &Placing, drones: &[(JobId, u32)], job: &JobId, port: u16) {
    let pid = drones
        .iter()
        .find(|(held, _)| held == job)
        .map(|(_, pid)| *pid)
        .expect("Fleet is holding a Drone for this Job");
    peers.holding(pid, port, SERVED_ON);
}

/// **The definition of done, as one case.** Two approved Jobs are worked at the
/// same time, each in its own worktree.
#[tokio::test]
async fn two_approved_jobs_are_worked_at_once_each_in_its_own_worktree() {
    let home = TempDir::new();
    let (fleet, _peers) = two_at_once(&home);

    let first = approved(&fleet, &home, "fix the reader").await;
    let second = approved(&fleet, &home, "fix the writer").await;

    // **Both, at the same moment.** One reading of the roster, not one after
    // the other — a Fleet that worked them in sequence would pass an assertion
    // taken twice and fails this one.
    let working = fleet.working_on().await;
    assert_eq!(working.len(), 2, "both Jobs are being worked: {working:?}");
    assert!(working.contains(&first) && working.contains(&second));

    let root = home.path().to_string_lossy().to_string();
    let one = WorktreeSpec::for_job(&root, first.as_str()).expect("a legal spec");
    let two = WorktreeSpec::for_job(&root, second.as_str()).expect("a legal spec");
    assert_ne!(
        one.worktree_path(),
        two.worktree_path(),
        "a worktree is per Job, so two Jobs is two directories"
    );

    // And a Drone each, which is the other half of "at once": one slot holding
    // two Jobs in turn would answer the roster the same way and hold one pid.
    let drones = fleet.drones_at_work();
    assert_eq!(drones.len(), 2, "a process each: {drones:?}");
    assert_ne!(drones[0].1, drones[1].1, "and they are different processes");
}

/// **The other half of the definition of done.** Each Drone's tool call reaches
/// the step that made it, and neither reaches the other's.
#[tokio::test]
async fn each_drones_call_lands_on_its_own_step_and_not_on_the_others() {
    let home = TempDir::new();
    let (fleet, peers) = two_at_once(&home);

    let first = approved(&fleet, &home, "fix the reader").await;
    let second = approved(&fleet, &home, "fix the writer").await;
    let drones = fleet.drones_at_work();
    calling_from(&peers, &drones, &first, 51001);
    calling_from(&peers, &drones, &second, 51002);

    // Neither has declared anything yet, so a declaration appearing on a slot
    // is one that arrived rather than one that was there.
    assert_eq!(declared_on(&fleet, &first).await, None);
    assert_eq!(declared_on(&fleet, &second).await, None);

    // **Both Drones are alive across both calls.** Nothing is stood down
    // between them, which is what makes this concurrency rather than a
    // sequence.
    let caller = api::Caller::at("127.0.0.1:51001".parse().expect("an address"));
    let job = fleet.caller_of(&caller).expect("the first Drone is placed");
    fleet
        .declare_scope(
            &job,
            &DeclareScope {
                context_paths: vec!["src/reader.rs".to_string()],
            },
        )
        .await
        .expect("the first Drone declares");

    let caller = api::Caller::at("127.0.0.1:51002".parse().expect("an address"));
    let job = fleet
        .caller_of(&caller)
        .expect("the second Drone is placed");
    fleet
        .declare_scope(
            &job,
            &DeclareScope {
                context_paths: vec!["src/writer.rs".to_string()],
            },
        )
        .await
        .expect("the second Drone declares");

    assert_eq!(
        declared_on(&fleet, &first).await,
        Some(vec!["src/reader.rs".to_string()]),
        "the first Job's step holds what the first Drone said"
    );
    assert_eq!(
        declared_on(&fleet, &second).await,
        Some(vec!["src/writer.rs".to_string()]),
        "and the second Job's step holds what the second Drone said"
    );
}

/// **The assertion that the attribution is doing the work.** The same two
/// Drones, with the ports the other way round, and the declarations follow the
/// ports rather than the admission order.
///
/// Without this, every case above would also pass against a Fleet that read
/// "whichever Job was admitted first", which is exactly what one working slot
/// did.
#[tokio::test]
async fn the_declaration_follows_the_connection_and_not_the_admission_order() {
    let home = TempDir::new();
    let (fleet, peers) = two_at_once(&home);

    let first = approved(&fleet, &home, "fix the reader").await;
    let second = approved(&fleet, &home, "fix the writer").await;
    let drones = fleet.drones_at_work();
    // Swapped: the Job admitted **first** is now the one calling from the
    // **second** port.
    calling_from(&peers, &drones, &first, 51002);
    calling_from(&peers, &drones, &second, 51001);

    let caller = api::Caller::at("127.0.0.1:51001".parse().expect("an address"));
    let job = fleet.caller_of(&caller).expect("a Drone holds that port");
    assert_eq!(job, second, "the port names the Drone, not the queue");
    fleet
        .declare_scope(
            &job,
            &DeclareScope {
                context_paths: vec!["src/writer.rs".to_string()],
            },
        )
        .await
        .expect("it declares");

    assert_eq!(
        declared_on(&fleet, &second).await,
        Some(vec!["src/writer.rs".to_string()]),
        "the declaration went where the connection said"
    );
    assert_eq!(
        declared_on(&fleet, &first).await,
        None,
        "and the Job that was admitted first was not touched by it"
    );
}

/// **A caller nothing holds is refused rather than credited to whoever is
/// nearest.** The shape spike 10 measured, with two Drones running: a call
/// arriving from a port no Drone opened.
#[tokio::test]
async fn a_call_from_a_port_no_drone_holds_reaches_neither_job() {
    let home = TempDir::new();
    let (fleet, peers) = two_at_once(&home);

    let first = approved(&fleet, &home, "fix the reader").await;
    let second = approved(&fleet, &home, "fix the writer").await;
    let drones = fleet.drones_at_work();
    calling_from(&peers, &drones, &first, 51001);
    calling_from(&peers, &drones, &second, 51002);

    let stranger = api::Caller::at("127.0.0.1:51999".parse().expect("an address"));
    let refused = fleet
        .caller_of(&stranger)
        .expect_err("nothing this Fleet started holds that connection");
    assert!(
        refused.to_string().contains("no Job to record it against"),
        "{refused}"
    );
    assert_eq!(declared_on(&fleet, &first).await, None);
    assert_eq!(declared_on(&fleet, &second).await, None);
}

/// **The bound is the bound.** A third approved Job stays `queued` while two are
/// worked, and the Board says why in the same words admission decided in.
#[tokio::test]
async fn a_third_approved_job_waits_on_the_bound_and_the_board_says_so() {
    let home = TempDir::new();
    let (fleet, _peers) = two_at_once(&home);

    let first = approved(&fleet, &home, "fix the reader").await;
    let second = approved(&fleet, &home, "fix the writer").await;
    let third = approved(&fleet, &home, "fix the parser").await;

    let working = fleet.working_on().await;
    assert_eq!(working.len(), 2, "two, and not three: {working:?}");
    assert!(!working.contains(&third), "the third is not one of them");
    assert_eq!(
        fleet.load(&third).await.expect("the Job is there").status(),
        core_model::JobStatus::Queued,
        "it is approved and waiting, which is what `queued` is"
    );
    assert!(
        working.contains(&first) && working.contains(&second),
        "and the two that were approved first are the two running"
    );
}

/// **A stranded dependent does not take the room the bound has left.**
///
/// `#48` escalates a `queued` Job whose upstream ended badly, and it does so on
/// the turn before admission. With one working slot that ordering was enough on
/// its own, because there was no room to compete for; with two there is, and
/// this is the case that says the ordering still holds — one Job working, one
/// place free, and the Job that would have taken it moved off `queued` instead
/// of into it.
///
/// A stranded dependent cannot **hold** a slot either, and that needs no case:
/// a slot is opened by admission, which moves the Job to `running` as it goes.
/// Nothing else opens one — the acts a person takes on a Job that is
/// `escalated` or at a gate all re-queue instead, which is what
/// `crate::tests::bounding` is about.
#[tokio::test]
async fn a_dependent_whose_upstream_failed_does_not_take_the_free_place() {
    let home = TempDir::new();
    let mut fittings: Fittings<FakeHarness, FakeVcs, FakeWorkProduct> =
        fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.workflows = a_catalogue()
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fittings.judge = Arc::new(FakeJudge::saying(A_PLAN));
    fittings.concurrency = Concurrency::of(2);
    let fleet = Fleet::assembled(fittings);

    let made = fleet
        .propose_from("two coupled changes")
        .await
        .expect("a plan");
    // The upstream runs, which is what leaves exactly one place free.
    worktree_directory(&home, made[0].id());
    fleet.approve(made[0].id()).await.expect("the upstream");
    worktree_directory(&home, made[1].id());
    fleet.approve(made[1].id()).await.expect("the dependent");
    assert_eq!(
        fleet.working_on().await,
        vec![made[0].id().clone()],
        "the dependent is queued behind its upstream and not behind the bound"
    );

    fleet
        .kill_job(made[0].id())
        .await
        .expect("the upstream ends");
    let turned = fleet.turn().await.expect("the loop turns");

    assert_eq!(
        turned.stranded,
        vec![made[1].id().clone()],
        "the dependent moved off queued"
    );
    assert!(
        !turned.admitted.contains(made[1].id()),
        "and admission, which ran after it and had room, did not take it: {:?}",
        turned.admitted
    );
    assert!(
        !fleet.working_on().await.contains(made[1].id()),
        "so no Drone is on a base that never landed"
    );
    assert_eq!(
        fleet
            .load(made[1].id())
            .await
            .expect("the Job reads")
            .status(),
        core_model::JobStatus::Escalated,
    );
}
