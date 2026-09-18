//! The way back from a Job to its Studio, read off the `produced` edge. `#1362`.

use core_model::{
    JobId, ManifestId, Studio, StudioAuthor, StudioEdgeId, StudioId, StudioName, StudioNode,
    StudioNodeContent, StudioNodeId, StudioNodeState, StudioPosition, Timestamp, Ulid,
};

use crate::tests::{open, TempDir};
use crate::Store;

fn at(minute: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-09-18T11:{minute:02}:00.000Z"))
}

fn job_id(id: &str) -> JobId {
    JobId::carried(Ulid::carried(id))
}

fn node_id(id: &str) -> StudioNodeId {
    StudioNodeId::carried(Ulid::carried(id))
}

fn a_studio(store: &mut Store, id: &str, name: Option<&str>) -> StudioId {
    let studio = Studio {
        id: StudioId::carried(Ulid::carried(id)),
        manifest_id: ManifestId::carried(Ulid::carried("armada")),
        name: name.and_then(StudioName::named),
        named_by: name.map(|_| StudioAuthor::Person),
        created_at: at(0),
        touched_at: at(0),
    };
    store.create_studio(&studio).expect("kept");
    studio.id
}

/// An Issue draft, and the Job node dispatch produced from it.
fn a_dispatch(store: &mut Store, studio: &StudioId, draft: &str, node: &str, job: &str) {
    let written = StudioNode::recorded(
        node_id(draft),
        StudioNodeContent::IssueDraft {
            title: "Cache the Manifest read".to_string(),
            body: "It is read on every row.".to_string(),
        },
        Some(StudioNodeState::Draft),
        StudioPosition { x: 0, y: 0 },
        at(1),
        Some(StudioAuthor::Person),
    )
    .expect("a draft is drafted");
    store
        .add_studio_node(studio, &written, None, &at(1))
        .expect("the draft is added");
    let dispatched = StudioNode::added(
        node_id(node),
        StudioNodeContent::Job {
            job_id: job_id(job),
        },
        StudioPosition { x: 240, y: 0 },
        at(2),
        StudioAuthor::Person,
    );
    store
        .add_studio_node(
            studio,
            &dispatched,
            Some((
                &node_id(draft),
                StudioEdgeId::carried(Ulid::carried("01EDGE")),
            )),
            &at(2),
        )
        .expect("the Job node is added");
}

#[test]
fn a_dispatched_job_names_the_studio_that_produced_it_and_the_node_to_land_on() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", Some("Stale counts"));
    a_dispatch(&mut store, &studio, "01DRAFT", "01JOBNODE", "01JOB");

    let found = store
        .dispatched_from(&job_id("01JOB"))
        .expect("the read runs")
        .expect("the Studio that produced it");
    assert_eq!(found.studio_id, studio);
    assert_eq!(
        found.name.as_ref().map(StudioName::as_str),
        Some("Stale counts")
    );
    assert_eq!(
        found.node_id,
        node_id("01JOBNODE"),
        "landing selects the node the Job arrived as, not the Studio's origin"
    );
}

#[test]
fn an_untitled_studio_is_named_by_nothing_here_and_bridge_says_the_word() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", None);
    a_dispatch(&mut store, &studio, "01DRAFT", "01JOBNODE", "01JOB");

    let found = store
        .dispatched_from(&job_id("01JOB"))
        .expect("the read runs")
        .expect("the Studio that produced it");
    assert_eq!(found.name, None);
}

#[test]
fn a_job_nothing_dispatched_from_a_studio_answers_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", Some("Stale counts"));
    a_dispatch(&mut store, &studio, "01DRAFT", "01JOBNODE", "01JOB");

    assert_eq!(
        store
            .dispatched_from(&job_id("01OTHER"))
            .expect("the read runs"),
        None,
        "most Jobs never came off a Studio"
    );
}

/// **A deleted Studio is the absence, and that is the point.** Nothing writes
/// a column pointing at a Studio, so deleting one takes the way back with it
/// and leaves `jobs.origin` to say the Job came off one at all.
#[test]
fn a_deleted_studio_leaves_the_job_with_no_way_back_rather_than_a_dead_one() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", Some("Stale counts"));
    a_dispatch(&mut store, &studio, "01DRAFT", "01JOBNODE", "01JOB");
    store.delete_studio(&studio).expect("a person deletes it");

    assert_eq!(
        store
            .dispatched_from(&job_id("01JOB"))
            .expect("the read runs"),
        None
    );
}

/// A Job node standing with nothing pointing into it says the Job is *on* a
/// Studio, which is a different claim from the Studio having produced it.
#[test]
fn a_job_node_with_no_produced_edge_into_it_is_not_a_studio_that_produced_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", Some("Stale counts"));
    let alone = StudioNode::added(
        node_id("01JOBNODE"),
        StudioNodeContent::Job {
            job_id: job_id("01JOB"),
        },
        StudioPosition { x: 0, y: 0 },
        at(1),
        StudioAuthor::Person,
    );
    store
        .add_studio_node(&studio, &alone, None, &at(1))
        .expect("added");

    assert_eq!(
        store
            .dispatched_from(&job_id("01JOB"))
            .expect("the read runs"),
        None
    );
}
