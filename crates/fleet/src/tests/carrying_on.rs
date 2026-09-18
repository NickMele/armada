//! A redispatch reaching the Studio that dispatched it. `#1440`.
//!
//! **The Job under test is redispatched for real**, through `Fleet::redispatch`
//! — the whole of what the complaint is about is that nothing on that path
//! touched a Studio. What is asserted is the record that came back: the node
//! that was there unchanged, a node of its own for the replacement, and the
//! `produced` edge between them.
//!
//! The Job node is written straight through the store, as `tests::promoting`'s
//! `seeded` is: a real dispatch from a draft needs a proposer, and what this
//! file is about is what happens to a Job node however it got there.

use api::Studios;
use core_model::{JobId, StudioNodeKind};
use ipc::{CreateStudio, Studio, StudioNode, StudioNodeContent, StudioNodeId, StudioPosition};
use testkit::FakeWorkProduct;

use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet, a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = crate::daemon::Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// Where the Job node is put, so what the replacement is laid out beside is a
/// stated value rather than the origin.
const LEFT_AT: StudioPosition = StudioPosition { x: 180, y: 60 };

fn fleet_in(home: &TempDir) -> Fixture {
    a_fleet(home, FakeWorkProduct::changed(&["src/log.rs"]))
}

async fn a_studio(fleet: &Fixture) -> Studio {
    fleet
        .create_studio(
            CreateStudio {
                name: Some("Stale counts".to_string()),
            },
            None,
        )
        .await
        .expect("a Studio in the repository Fleet starts in")
}

/// A Job at the gate, released to run and then stopped by hand — the shortest
/// redispatchable status that needs no evidence and no ruling.
async fn a_killed_job(fleet: &Fixture, home: &TempDir, said: &str) -> JobId {
    let job = fleet
        .propose(a_proposal(said))
        .await
        .expect("a Job at the gate");
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.expect("released to run");
    fleet.kill_job(job.id()).await.expect("ended by hand");
    job.id().clone()
}

/// The Job node a dispatch from a Studio leaves, written through the store.
async fn a_job_node(
    fleet: &Fixture,
    studio: &Studio,
    job_id: &JobId,
    position: StudioPosition,
) -> StudioNodeId {
    let at = fleet.now();
    let node = core_model::StudioNode::added(
        core_model::StudioNodeId::carried(fleet.mint().ulid()),
        core_model::StudioNodeContent::Job {
            job_id: job_id.clone(),
        },
        core_model::StudioPosition {
            x: position.x,
            y: position.y,
        },
        at.clone(),
        core_model::StudioAuthor::Person,
    );
    let mut store = fleet.store().lock().await;
    store
        .add_studio_node(&studio.id.to_domain(), &node, None, &at)
        .expect("added");
    StudioNodeId::from(node.id())
}

async fn read_back(fleet: &Fixture, studio: &Studio) -> Studio {
    fleet
        .get_studio(studio.id.clone(), None)
        .await
        .expect("the Studio reads back")
}

/// Every Job node on the Studio, as the Job each names. **Spelled**, because
/// the wire's id and the domain's are two types over one string.
fn jobs_on(studio: &Studio) -> Vec<&str> {
    studio
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            StudioNodeContent::Job { job_id } => Some(job_id.as_str()),
            _ => None,
        })
        .collect()
}

/// The node naming `job_id`, refused where there is not exactly one.
fn node_for<'a>(studio: &'a Studio, job_id: &JobId) -> &'a StudioNode {
    let found: Vec<_> = studio
        .nodes
        .iter()
        .filter(|node| match &node.content {
            StudioNodeContent::Job { job_id: on } => on.as_str() == job_id.as_str(),
            _ => false,
        })
        .collect();
    match found[..] {
        [one] => one,
        _ => panic!("one node names {}, and {} do", job_id.as_str(), found.len()),
    }
}

/// Every `produced` edge into `node`, as the ids that made it.
fn made_by<'a>(studio: &'a Studio, node: &StudioNodeId) -> Vec<&'a StudioNodeId> {
    studio
        .edges
        .iter()
        .filter(|edge| &edge.to == node && edge.kind.as_wire() == "produced")
        .map(|edge| &edge.from)
        .collect()
}

/// **The issue's own definition of done.** A Job dispatched from a Studio is
/// killed and redispatched, and the Studio holds both: the one that stopped,
/// still naming the Job it always named, and the replacement beside it under
/// an edge saying the work carried on there.
#[tokio::test]
async fn a_redispatched_job_arrives_beside_the_one_it_replaced_under_a_produced_edge() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    let studio = a_studio(&fleet).await;
    let killed = a_killed_job(&fleet, &home, "stopped on purpose").await;
    let was_there = a_job_node(&fleet, &studio, &killed, LEFT_AT).await;

    let both = fleet.redispatch(&killed).await.expect("a replacement");

    let after = read_back(&fleet, &studio).await;
    assert_eq!(
        jobs_on(&after),
        vec![killed.as_str(), both.dispatched.id().as_str()],
        "nothing is deleted and nothing is rewritten: both Jobs are on it"
    );
    let replacement = node_for(&after, both.dispatched.id());
    assert_eq!(
        made_by(&after, &replacement.id),
        [&was_there],
        "the edge runs from the Job that stopped to the one the work carried on as"
    );
    assert!(
        replacement.state.is_none(),
        "a Job node holds a reference and reads its status off the Board, as every other does"
    );
    assert!(
        replacement.position.x > LEFT_AT.x,
        "beside the one it replaced, rather than at the origin: {:?}",
        replacement.position
    );
}

/// **A Studio is a record.** The node that was there is untouched — same id,
/// same Job, same position — because that Job really did run and really was
/// killed, and where a person put it is part of how they read the work.
#[tokio::test]
async fn the_node_it_replaced_still_names_the_job_that_stopped_where_it_was_left() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    let studio = a_studio(&fleet).await;
    let killed = a_killed_job(&fleet, &home, "stopped on purpose").await;
    let was_there = a_job_node(&fleet, &studio, &killed, LEFT_AT).await;

    fleet.redispatch(&killed).await.expect("a replacement");

    let after = read_back(&fleet, &studio).await;
    let stopped = node_for(&after, &killed);
    assert_eq!(stopped.id, was_there, "the same node, not a new one");
    assert_eq!(stopped.position, LEFT_AT, "where the person left it");
}

/// **A chain.** A replacement that is itself redispatched adds the next node,
/// hung off the node it replaced rather than off the first — so the Studio
/// reads as the order the work went in.
#[tokio::test]
async fn a_replacement_that_is_itself_redispatched_adds_the_next_node_to_the_chain() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    let studio = a_studio(&fleet).await;
    let first = a_killed_job(&fleet, &home, "stopped on purpose").await;
    a_job_node(&fleet, &studio, &first, LEFT_AT).await;

    let second = fleet.redispatch(&first).await.expect("a replacement");
    let second = second.dispatched.id().clone();
    // The replacement stands at the approval gate, so it is released and
    // stopped the same way the first one was before it is replaced in turn.
    worktree_directory(&home, &fleet.load(&second).await.expect("the replacement"));
    dispatched(&fleet, &second).await.expect("released to run");
    fleet.kill_job(&second).await.expect("ended by hand");
    let third = fleet.redispatch(&second).await.expect("a second replacement");

    let after = read_back(&fleet, &studio).await;
    assert_eq!(
        jobs_on(&after),
        vec![first.as_str(), second.as_str(), third.dispatched.id().as_str()],
        "three Jobs, in the order the work went in"
    );
    assert_eq!(
        made_by(&after, &node_for(&after, third.dispatched.id()).id),
        [&node_for(&after, &second).id],
        "the third hangs off the second, which is the Job it replaced"
    );
}

/// **A Job redispatched twice leaves two replacements and no Job drawn
/// twice.** `killed` is itself redispatchable, so nothing refuses the second
/// press — `docs/concepts/job.md`. Each replacement is its own Job and gets
/// its own node, and the guard on the replacement is what keeps one Job from
/// appearing on the board more than once.
#[tokio::test]
async fn a_job_redispatched_twice_draws_each_replacement_once_and_no_job_twice() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    let studio = a_studio(&fleet).await;
    let killed = a_killed_job(&fleet, &home, "stopped on purpose").await;
    let was_there = a_job_node(&fleet, &studio, &killed, LEFT_AT).await;

    let first = fleet.redispatch(&killed).await.expect("a replacement");
    let second = fleet.redispatch(&killed).await.expect("a second replacement");

    let after = read_back(&fleet, &studio).await;
    assert_eq!(
        jobs_on(&after),
        vec![
            killed.as_str(),
            first.dispatched.id().as_str(),
            second.dispatched.id().as_str()
        ],
        "one node per Job, and both replacements are real Jobs that really ran"
    );
    let (one, two) = (
        node_for(&after, first.dispatched.id()),
        node_for(&after, second.dispatched.id()),
    );
    assert_eq!(made_by(&after, &one.id), [&was_there]);
    assert_eq!(made_by(&after, &two.id), [&was_there]);
    assert_ne!(
        one.position, two.position,
        "the second is laid out under the first rather than on top of it"
    );
}

/// **A Studio that never drew the Job gets nothing**, which is nearly every
/// redispatch: a Job proposed from a request belongs to no Studio, and a
/// Studio is not a place work appears because it happened somewhere.
#[tokio::test]
async fn a_studio_that_never_drew_the_job_is_left_alone() {
    let home = TempDir::new();
    let fleet = fleet_in(&home);
    let studio = a_studio(&fleet).await;
    let elsewhere = a_killed_job(&fleet, &home, "nothing to do with the Studio").await;
    let drawn = a_killed_job(&fleet, &home, "dispatched from the Studio").await;
    a_job_node(&fleet, &studio, &drawn, LEFT_AT).await;

    fleet.redispatch(&elsewhere).await.expect("a replacement");

    let after = read_back(&fleet, &studio).await;
    assert_eq!(
        jobs_on(&after),
        vec![drawn.as_str()],
        "only the Job this Studio drew is on it"
    );
    assert_eq!(
        after
            .nodes
            .iter()
            .filter(|node| node.content.to_domain().kind() == StudioNodeKind::Job)
            .count(),
        1
    );
}
