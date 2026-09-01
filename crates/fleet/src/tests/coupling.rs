//! What an upstream's terminal status does to the Job waiting behind it.
//!
//! The ordering half — a dependent skipped until its upstream lands — is
//! `planning`'s and `queued`'s claim and is not restated here. **This file is
//! the three-way release rule and the refusal**, which is everything that used
//! to leave a Job at `queued` for ever.
//!
//! Two of the three outcomes are driven end to end through a real Fleet. The
//! third is not, and deliberately: nothing in the workspace can put a Job at
//! `superseded` — the only edge into it is `piloted -> superseded` and Pilot is
//! unbuilt — so the board is planted for that one case and the predicate is
//! called directly. Driving it would mean minting a transition the registry
//! does not have.

use std::collections::BTreeMap;

use core_model::{JobId, JobStatus};
use testkit::{FakeJudge, FakeWorkProduct};

use crate::coupling::{coupling, Coupling};
use crate::tests::daemon::{a_fleet, a_fleet_proposing_through, a_proposal, worktree_directory};
use crate::tests::planning::A_PLAN;
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;

/// Three Jobs in a line: the second waits on the first, the third on the
/// second. What a chain has to be to prove a failure does not run down it.
const A_CHAIN: &str = "\
job: 1
workflow: feature
title: Add the endpoint
because: nothing can be written against something that is not there
writes: crates/api/src/routes.rs

job: 2
workflow: feature
title: Update the consumer
because: it reads the endpoint the first Job adds
writes: apps/desktop/src/main/connection.ts
after: 1

job: 3
workflow: feature
title: Draw the new field
because: it renders what the consumer now has
writes: packages/screens/src/Jobs.tsx
after: 2
";

/// An upstream that ends badly moves its dependent off `queued`.
///
/// **The whole of the defect this file exists for.** The trigger, the edge and
/// the registry row all existed and nothing raised it, so a dependent sat at
/// `queued` behind `blocked_by_dependency` — a label with no action attached,
/// on a wait that never self-clears.
#[tokio::test]
async fn a_killed_upstream_escalates_its_dependent_as_dependency_failed() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(A_PLAN),
    );
    let made = fleet
        .propose_from("two coupled changes")
        .await
        .expect("a plan");
    worktree_directory(&home, made[1].id());
    fleet.approve(made[1].id()).await.expect("the dependent");
    fleet.kill_job(made[0].id()).await.expect("the upstream");

    let turned = fleet.turn().await.expect("the loop turns");

    assert_eq!(
        turned.stranded,
        vec![made[1].id().clone()],
        "the turn after the upstream ended is the turn the dependent moves"
    );
    let stranded = fleet.load(made[1].id()).await.expect("the Job reads");
    assert_eq!(stranded.status(), JobStatus::Escalated);
    assert_eq!(
        fleet
            .last_reason(made[1].id())
            .await
            .expect("a reason reads")
            .and_then(|why| why.as_wire()),
        Some("dependency_failed"),
        "escalated for the upstream and not for anything of its own"
    );
    assert!(
        fleet.working_on().await.is_empty(),
        "and nothing was dispatched onto a base that never landed"
    );
}

/// Escalating stops at the first dependent.
///
/// `fleet.md`: a failed upstream escalates the dependent *"so a person decides
/// rather than one failure terminating a chain unattended"*. The third Job
/// waits on the second, and the second is now `escalated` rather than terminal
/// — so there is nothing to release it and nothing to strand it either.
#[tokio::test]
async fn the_chain_below_the_first_dependent_is_left_where_it_was() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(A_CHAIN),
    );
    let made = fleet.propose_from("three in a line").await.expect("a plan");
    assert_eq!(made.len(), 3);
    worktree_directory(&home, made[1].id());
    worktree_directory(&home, made[2].id());
    fleet.approve(made[1].id()).await.expect("the second");
    fleet.approve(made[2].id()).await.expect("the third");
    fleet.kill_job(made[0].id()).await.expect("the first");

    let turned = fleet.turn().await.expect("the loop turns");

    assert_eq!(
        turned.stranded,
        vec![made[1].id().clone()],
        "one Job moved, and it is the one that named the Job that failed"
    );
    let last = fleet.load(made[2].id()).await.expect("the Job reads");
    assert_eq!(
        last.status(),
        JobStatus::Queued,
        "an escalated upstream is not a terminal one, so the third is still merely waiting"
    );
}

/// A dependent already escalated is not escalated again.
///
/// The walk runs every turn against the whole board, so the guard that keeps it
/// from re-raising is that it reads `queued` and nothing else. Without it the
/// second turn asks the machine for `escalated -> escalated`, which is a bug in
/// Fleet rather than a warning.
#[tokio::test]
async fn a_second_turn_strands_nothing_more() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(A_PLAN),
    );
    let made = fleet
        .propose_from("two coupled changes")
        .await
        .expect("a plan");
    worktree_directory(&home, made[1].id());
    fleet.approve(made[1].id()).await.expect("the dependent");
    fleet.kill_job(made[0].id()).await.expect("the upstream");
    fleet.turn().await.expect("the first turn");

    let again = fleet.turn().await.expect("the second turn");

    assert!(again.stranded.is_empty());
}

/// The third outcome: released, and released is not satisfied.
///
/// Planted rather than driven, for the reason this module's comment gives. What
/// it asserts is the pair — the edge no longer holds the Job, *and* the peer is
/// named as unsatisfied rather than quietly counted as landed.
#[tokio::test]
async fn a_superseded_upstream_releases_and_is_carried_as_unsatisfied() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let upstream = fleet.propose(a_proposal("the upstream")).await.unwrap();
    let mut proposal = a_proposal("the dependent");
    proposal.dependencies = vec![ipc::DependencyEdge {
        direction: ipc::DependencyDirection::from_wire("depends_on").expect("a direction"),
        peer: ipc::JobId::from(upstream.id()),
    }];
    let dependent = fleet.propose(proposal).await.expect("a coupled Job");

    let superseded: BTreeMap<JobId, JobStatus> =
        BTreeMap::from([(upstream.id().clone(), JobStatus::Superseded)]);
    let rejected: BTreeMap<JobId, JobStatus> =
        BTreeMap::from([(upstream.id().clone(), JobStatus::Rejected)]);

    assert_eq!(
        coupling(&dependent, &superseded),
        Coupling::Clear {
            unsatisfied: vec![upstream.id().clone()]
        },
        "the work landed outside the Job, so the base is there and the record is not"
    );
    assert_eq!(
        coupling(&dependent, &rejected),
        Coupling::Failed {
            peer: upstream.id().clone()
        },
        "and the terminal beside it is still the failing one"
    );
}

/// A proposal naming a peer nothing holds is refused where it enters.
///
/// `ProposeJob.dependencies` has always said a peer must already exist. Nothing
/// enforced it, so an edge could point at an id that was never minted and the
/// Job it made was unadmittable for ever behind an ordinary-looking label.
#[tokio::test]
async fn a_proposal_naming_a_peer_nothing_holds_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let mut proposal = a_proposal("waiting on nobody");
    proposal.dependencies = vec![ipc::DependencyEdge {
        direction: ipc::DependencyDirection::from_wire("depends_on").expect("a direction"),
        peer: ipc::JobId::carried("01NEVERMINTEDBYTHISFLEET"),
    }];

    let refused = fleet.propose(proposal).await.expect_err("no such peer");

    assert!(
        refused.to_string().contains("01NEVERMINTEDBYTHISFLEET"),
        "the message names the id, since a caller cannot fix an edge it cannot find: {refused}"
    );
    let (loaded, _) = fleet.every_job().await.expect("the board reads");
    assert!(
        loaded.jobs.is_empty(),
        "refused at creation, so there is no row to clean up"
    );
}

/// **A cycle is not detected, it is unstatable.**
///
/// Two Jobs each depending on the other needs one of the two edges to point at
/// a Job that does not exist yet, and that is the edge refused above. There is
/// no topological sort anywhere in the workspace and this is why one is not
/// owed: `dependencies` is written once, at insert, so every edge points at a
/// strictly older Job and the graph cannot come to have a cycle later.
#[tokio::test]
async fn two_jobs_cannot_be_made_to_wait_on_each_other() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let first = fleet.propose(a_proposal("the first")).await.unwrap();
    let mut second = a_proposal("the second");
    second.dependencies = vec![ipc::DependencyEdge {
        direction: ipc::DependencyDirection::from_wire("depends_on").expect("a direction"),
        peer: ipc::JobId::from(first.id()),
    }];
    let second = fleet.propose(second).await.expect("pointing backwards");

    // The other half of the cycle would have to be added to `first`, and there
    // is no operation that writes an edge onto a Job that exists — the column
    // is written by the insert and by nothing else. So the closest a caller can
    // get is a third Job naming the second, which is a chain and not a cycle.
    let mut third = a_proposal("the third");
    third.dependencies = vec![ipc::DependencyEdge {
        direction: ipc::DependencyDirection::from_wire("depends_on").expect("a direction"),
        peer: ipc::JobId::from(second.id()),
    }];
    let third = fleet.propose(third).await.expect("still backwards");

    let standing: BTreeMap<JobId, JobStatus> = BTreeMap::from([
        (first.id().clone(), JobStatus::CompletedSuccess),
        (second.id().clone(), JobStatus::Queued),
    ]);
    assert_eq!(
        coupling(&third, &standing),
        Coupling::Waiting,
        "waiting on a Job that will move, which is the only kind of wait left"
    );
}
