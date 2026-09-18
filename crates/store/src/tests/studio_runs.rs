//! A Run node and what it keeps when the run it references is swept. `#1289`.

use core_model::{
    ManifestId, Studio, StudioAuthor, StudioId, StudioName, StudioNode, StudioNodeContent,
    StudioNodeId, StudioPosition, StudioRun, StudioRunKept, Timestamp, Ulid,
};

use crate::tests::{open, TempDir};
use crate::Store;

fn at(minute: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-09-17T09:{minute:02}:00.000Z"))
}

fn a_studio(store: &mut Store, id: &str) -> StudioId {
    let studio = Studio {
        id: StudioId::carried(Ulid::carried(id)),
        manifest_id: ManifestId::carried(Ulid::carried("armada")),
        name: StudioName::named("Stale counts"),
        named_by: Some(StudioAuthor::Person),
        created_at: at(0),
        touched_at: at(0),
    };
    store.create_studio(&studio).expect("kept");
    studio.id
}

fn a_run_node(store: &mut Store, studio: &StudioId, id: &str, run_id: &str) -> StudioNodeId {
    let node = StudioNode::added(
        StudioNodeId::carried(Ulid::carried(id)),
        StudioNodeContent::Run {
            run: StudioRun::Checkout(run_id.to_string()),
            kept: None,
        },
        StudioPosition { x: 0, y: 0 },
        at(1),
        StudioAuthor::Person,
    );
    store
        .add_studio_node(studio, &node, None, &at(1))
        .expect("added");
    node.id().clone()
}

fn a_tail() -> StudioRunKept {
    StudioRunKept {
        name: String::from("test"),
        command: String::from("pnpm test"),
        exit_code: Some(1),
        expect_exit_code: 0,
        stopped: false,
        duration_ms: 12_400,
        lines: vec![String::from("4 failed"), String::from("done")],
        total_lines: 903,
        whole: false,
    }
}

fn content_of(store: &Store, studio: &StudioId, node: &StudioNodeId) -> StudioNodeContent {
    store
        .studio(studio)
        .expect("reads")
        .nodes
        .iter()
        .find(|held| held.id() == node)
        .expect("the node is there")
        .content()
        .clone()
}

/// **A Run node keeps the result and the tail of a run that was swept**, and
/// reads it back whole after a reopen.
///
/// The failure this is against is a node pointing at nothing: a Studio is kept
/// until a person deletes it and a run's log is not, so what the run said has
/// to survive on the node or not at all.
#[test]
fn a_swept_runs_result_and_tail_are_kept_on_the_node_that_held_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO");
    let node = a_run_node(&mut store, &studio, "01RUNNODE", "01RUN");
    let touched_before = store.studio(&studio).expect("reads").studio.touched_at;

    let changed = store.keep_studio_run("01RUN", &a_tail()).expect("kept");
    assert_eq!(changed, vec![studio.clone()], "the Studio that holds it");

    drop(store);
    let store = open(&dir);
    let StudioNodeContent::Run { run, kept } = content_of(&store, &studio, &node) else {
        panic!("a Run node");
    };
    assert_eq!(
        run,
        StudioRun::Checkout(String::from("01RUN")),
        "it still says which run it was, and whose"
    );
    assert_eq!(kept, Some(a_tail()), "field for field, through the column");
    assert_eq!(
        store.studio(&studio).expect("reads").studio.touched_at,
        touched_before,
        "retention passing is nothing a person did on the Studio"
    );
}

/// **Only the node that holds the run, and only once.** A second sweep of the
/// same run writes nothing, so a tail taken while the log was there is never
/// replaced by one taken after it was gone.
#[test]
fn a_run_no_node_holds_changes_nothing_and_a_kept_tail_is_not_written_over() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO");
    let node = a_run_node(&mut store, &studio, "01RUNNODE", "01RUN");
    let other = a_run_node(&mut store, &studio, "01OTHERNODE", "01OTHERRUN");

    assert!(
        store
            .keep_studio_run("01NOBODYHOLDS", &a_tail())
            .expect("reads")
            .is_empty(),
        "no Studio held that run"
    );
    store.keep_studio_run("01RUN", &a_tail()).expect("kept");

    let mut second = a_tail();
    second.lines = vec![String::from("read after the sweep")];
    assert!(
        store.keep_studio_run("01RUN", &second).expect("reads")[..].is_empty(),
        "a node that already kept its tail is not written again"
    );
    let StudioNodeContent::Run { kept, .. } = content_of(&store, &studio, &node) else {
        panic!("a Run node");
    };
    assert_eq!(kept, Some(a_tail()), "the tail taken before the sweep");
    assert_eq!(
        content_of(&store, &studio, &other),
        StudioNodeContent::Run {
            run: StudioRun::Checkout(String::from("01OTHERRUN")),
            kept: None,
        },
        "the other run is still there to read"
    );
}
