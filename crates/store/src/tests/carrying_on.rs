//! Which Studios drew a Job, asked by Job id. `#1440`.
//!
//! The claim worth breaking is that the answer comes off the node's own
//! content rather than off a second table: a Job node's reference is text with
//! no foreign key, so a Job nothing holds and a Job whose node was deleted
//! read the same way — and a Studio that drew some other Job is not an answer.

use core_model::{
    JobId, ManifestId, Studio, StudioAuthor, StudioEdgeId, StudioId, StudioName, StudioNode,
    StudioNodeContent, StudioNodeId, StudioPosition, Timestamp, Ulid,
};

use crate::tests::{open, TempDir};
use crate::Store;

fn at(minute: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-09-18T09:{minute:02}:00.000Z"))
}

fn job_id(id: &str) -> JobId {
    JobId::carried(Ulid::carried(id))
}

fn a_studio(store: &mut Store, id: &str) -> StudioId {
    let studio = Studio {
        id: StudioId::carried(Ulid::carried(id)),
        manifest_id: ManifestId::carried(Ulid::carried("01MANIFEST")),
        name: StudioName::named("Redispatch"),
        named_by: Some(StudioAuthor::Person),
        created_at: at(0),
        touched_at: at(0),
    };
    store.create_studio(&studio).expect("kept");
    studio.id
}

/// A Job node on `studio`, produced by `from` where something made it.
fn a_job_node(
    store: &mut Store,
    studio: &StudioId,
    id: &str,
    job: &str,
    at_x: i64,
    from: Option<&StudioNodeId>,
) -> StudioNodeId {
    let node = StudioNode::added(
        StudioNodeId::carried(Ulid::carried(id)),
        StudioNodeContent::Job {
            job_id: job_id(job),
        },
        StudioPosition { x: at_x, y: 80 },
        at(1),
        StudioAuthor::Person,
    );
    let edge = StudioEdgeId::carried(Ulid::carried(format!("01EDGE{id}")));
    store
        .add_studio_node(studio, &node, from.map(|from| (from, edge)), &at(1))
        .expect("added");
    node.id().clone()
}

#[test]
fn a_job_no_studio_drew_is_on_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIOA");
    a_job_node(&mut store, &studio, "01NODEA", "01JOBOTHER", 100, None);

    let found = store.job_on_studios(&job_id("01JOBALONE")).expect("reads");

    assert_eq!(found, vec![], "no node names this job");
}

#[test]
fn a_job_a_studio_drew_names_the_studio_the_node_and_where_it_sits() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIOA");
    let node = a_job_node(&mut store, &studio, "01NODEA", "01JOBFAILED", 140, None);

    let found = store.job_on_studios(&job_id("01JOBFAILED")).expect("reads");

    assert_eq!(found.len(), 1, "one node names it");
    assert_eq!(found[0].studio_id, studio);
    assert_eq!(found[0].node_id, node);
    assert_eq!(
        found[0].position,
        StudioPosition { x: 140, y: 80 },
        "where a person left it, which is what a replacement is laid out beside"
    );
    assert_eq!(found[0].produced, 0, "it has made nothing yet");
}

#[test]
fn one_job_on_two_studios_answers_with_both() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = a_studio(&mut store, "01STUDIOA");
    let second = a_studio(&mut store, "01STUDIOB");
    a_job_node(&mut store, &first, "01NODEA", "01JOBFAILED", 100, None);
    a_job_node(&mut store, &second, "01NODEB", "01JOBFAILED", 200, None);

    let found = store.job_on_studios(&job_id("01JOBFAILED")).expect("reads");

    let studios: Vec<_> = found.iter().map(|on| on.studio_id.clone()).collect();
    assert_eq!(studios, vec![first, second], "a Job can be on both");
}

/// A Job redispatched twice: the second replacement is laid out under the
/// first rather than on top of it, and the count is what says so.
#[test]
fn a_node_that_already_produced_one_says_how_many() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIOA");
    let failed = a_job_node(&mut store, &studio, "01NODEA", "01JOBFAILED", 100, None);
    a_job_node(
        &mut store,
        &studio,
        "01NODEB",
        "01JOBAGAIN",
        440,
        Some(&failed),
    );

    let found = store.job_on_studios(&job_id("01JOBFAILED")).expect("reads");

    assert_eq!(found[0].produced, 1, "one replacement already hangs off it");

    let replacement = store.job_on_studios(&job_id("01JOBAGAIN")).expect("reads");
    assert_eq!(
        replacement[0].produced, 0,
        "the replacement has produced nothing, so its own replacement takes the first row"
    );
}

/// The node stands after the Job is forgotten, which is `studio.rs`'s rule —
/// no foreign key reaches `jobs` — and the read still answers off the text.
#[test]
fn a_node_naming_a_job_no_row_holds_is_still_an_answer() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIOA");
    a_job_node(&mut store, &studio, "01NODEA", "01JOBGONE", 100, None);

    let found = store.job_on_studios(&job_id("01JOBGONE")).expect("reads");

    assert_eq!(found.len(), 1, "the node is the record, not the job row");
}
