//! A Studio kept: whole after a reopen, listed by the repository it belongs
//! to, its edges only ever between its own nodes, and a row that does not read
//! back refused by name rather than dropped.

use core_model::{
    ManifestId, ScoutCheckout, ScoutEnded, ScoutLook, ScoutOutcome, Studio, StudioAuthor,
    StudioEdge, StudioEdgeId, StudioEdgeKind, StudioEdgeStanding, StudioFinding, StudioId,
    StudioName, StudioNode, StudioNodeContent, StudioNodeId, StudioNodeState, StudioPosition,
    StudioRelation, Timestamp, Ulid,
};

use crate::migrations::tables_pointing_at_a_job;
use crate::tests::{open, TempDir};
use crate::{Store, StudioError, Unreadable};

fn at(minute: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-09-17T09:{minute:02}:00.000Z"))
}

fn studio_id(id: &str) -> StudioId {
    StudioId::carried(Ulid::carried(id))
}

fn node_id(id: &str) -> StudioNodeId {
    StudioNodeId::carried(Ulid::carried(id))
}

fn edge_id(id: &str) -> StudioEdgeId {
    StudioEdgeId::carried(Ulid::carried(id))
}

fn a_studio(store: &mut Store, id: &str, manifest: &str, minute: u32) -> StudioId {
    let studio = Studio {
        id: studio_id(id),
        manifest_id: ManifestId::carried(Ulid::carried(manifest)),
        name: StudioName::named("Stale counts"),
        named_by: Some(StudioAuthor::Person),
        created_at: at(minute),
        touched_at: at(minute),
    };
    store.create_studio(&studio).expect("kept");
    studio.id
}

fn a_note(store: &mut Store, studio: &StudioId, id: &str, said: &str, x: i64) -> StudioNodeId {
    let node = StudioNode::added(
        node_id(id),
        StudioNodeContent::Note {
            said: said.to_string(),
            capture: None,
        },
        StudioPosition { x, y: 40 },
        at(1),
        StudioAuthor::Person,
    );
    store
        .add_studio_node(studio, &node, None, &at(1))
        .expect("added");
    node_id(id)
}

fn proposed(id: &str, from: &StudioNodeId, to: &StudioNodeId) -> StudioEdge {
    StudioEdge::proposed(
        edge_id(id),
        from.clone(),
        to.clone(),
        StudioRelation::SameAs,
        at(2),
        StudioAuthor::Person,
    )
    .expect("two different nodes")
}

/// `#1285`'s own claim: two Notes, a proposed Same as edge and a moved node,
/// read back identically once the file is reopened.
#[test]
fn a_studio_reads_back_whole_after_a_reopen_with_every_node_where_it_was_left() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", "armada", 0);
    let first = a_note(
        &mut store,
        &studio,
        "01NOTE1",
        "The chip keeps its count",
        0,
    );
    let second = a_note(&mut store, &studio, "01NOTE2", "Overview says three", 200);
    store
        .add_studio_edge(&studio, &proposed("01EDGE", &first, &second), &at(2))
        .expect("proposed");
    store
        .move_studio_node(&studio, &second, StudioPosition { x: 480, y: -120 }, &at(3))
        .expect("moved");
    let before = store.studio(&studio).expect("reads");
    drop(store);

    let store = open(&dir);
    let after = store.studio(&studio).expect("reads after a reopen");
    assert_eq!(after, before);
    assert_eq!(after.nodes.len(), 2);
    assert_eq!(
        after.nodes[1].position(),
        StudioPosition { x: 480, y: -120 }
    );
    assert_eq!(after.edges[0].standing(), StudioEdgeStanding::Proposed);
    assert_eq!(after.studio.touched_at, at(3), "the move touched it last");
}

#[test]
fn a_repository_lists_its_own_studios_the_last_touched_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let older = a_studio(&mut store, "01OLDER", "armada", 0);
    let newer = a_studio(&mut store, "01NEWER", "armada", 5);
    a_studio(&mut store, "01THEIRS", "elsewhere", 9);
    a_note(
        &mut store,
        &older,
        "01NOTE",
        "touched after the newer one",
        0,
    );
    store
        .rename_studio(
            &older,
            &StudioName::named("Renamed").expect("a name"),
            StudioAuthor::Person,
            &at(7),
        )
        .expect("renamed");

    let listed: Vec<_> = store
        .studios(&ManifestId::carried(Ulid::carried("armada")))
        .expect("reads")
        .into_iter()
        .map(|studio| studio.id)
        .collect();
    assert_eq!(listed, vec![older.clone(), newer]);
    let renamed = store.studio(&older).expect("reads").studio;
    assert_eq!(
        renamed.name.as_ref().map(StudioName::as_str),
        Some("Renamed")
    );
}

/// **Only a proposed edge is decided.** The Studio's own `Produced` edge is
/// accepted as drawn, and a rejected proposal is gone rather than kept.
#[test]
fn a_proposed_edge_is_accepted_or_rejected_and_nothing_else_is() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", "armada", 0);
    let note = a_note(&mut store, &studio, "01NOTE", "The chip keeps its count", 0);
    let finding = StudioNode::added(
        node_id("01FINDING"),
        StudioNodeContent::Finding(StudioFinding::asked("what reads the count")),
        StudioPosition { x: 0, y: 200 },
        at(1),
        StudioAuthor::Helm,
    );
    store
        .add_studio_node(&studio, &finding, Some((&note, edge_id("01MADE"))), &at(1))
        .expect("added, with the edge that says what made it");
    let other = a_note(&mut store, &studio, "01OTHER", "Overview says three", 200);
    store
        .add_studio_edge(&studio, &proposed("01KEEP", &note, &other), &at(2))
        .expect("proposed");
    store
        .add_studio_edge(&studio, &proposed("01DROP", &other, &note), &at(2))
        .expect("proposed");

    let made = store
        .decide_studio_edge(&studio, &edge_id("01MADE"), true, &at(3))
        .expect_err("drawn by the Studio, not proposed");
    assert!(matches!(made, StudioError::NotProposed { .. }), "{made}");
    store
        .decide_studio_edge(&studio, &edge_id("01KEEP"), true, &at(3))
        .expect("accepted");
    store
        .decide_studio_edge(&studio, &edge_id("01DROP"), false, &at(3))
        .expect("rejected");

    let edges = store.studio(&studio).expect("reads").edges;
    let kept: Vec<_> = edges
        .iter()
        .map(|edge| (edge.id().as_str(), edge.kind(), edge.standing()))
        .collect();
    assert_eq!(
        kept,
        vec![
            (
                "01MADE",
                StudioEdgeKind::Produced,
                StudioEdgeStanding::Accepted
            ),
            (
                "01KEEP",
                StudioEdgeKind::SameAs,
                StudioEdgeStanding::Accepted
            ),
        ]
    );
}

#[test]
fn an_edge_reaches_only_this_studios_nodes_and_is_kept_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let ours = a_studio(&mut store, "01OURS", "armada", 0);
    let theirs = a_studio(&mut store, "01THEIRS", "armada", 0);
    let here = a_note(&mut store, &ours, "01HERE", "ours", 0);
    let there = a_note(&mut store, &theirs, "01THERE", "theirs", 0);
    let away = store
        .add_studio_edge(&ours, &proposed("01AWAY", &here, &there), &at(2))
        .expect_err("a node on another Studio");
    assert!(
        matches!(&away, StudioError::NoSuchNode { node_id } if node_id == "01THERE"),
        "{away}"
    );

    let near = a_note(&mut store, &ours, "01NEAR", "ours too", 100);
    store
        .add_studio_edge(&ours, &proposed("01ONCE", &here, &near), &at(2))
        .expect("proposed");
    let twice = store
        .add_studio_edge(&ours, &proposed("01TWICE", &here, &near), &at(2))
        .expect_err("the same relation again");
    assert!(matches!(twice, StudioError::EdgeExists { .. }), "{twice}");
}

#[test]
fn removing_a_node_takes_its_edges_and_deleting_a_studio_takes_everything_on_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", "armada", 0);
    let kept = a_studio(&mut store, "01KEPT", "armada", 0);
    let first = a_note(&mut store, &studio, "01NOTE1", "one", 0);
    let second = a_note(&mut store, &studio, "01NOTE2", "two", 100);
    a_note(&mut store, &kept, "01NOTE3", "three", 0);
    store
        .add_studio_edge(&studio, &proposed("01EDGE", &first, &second), &at(2))
        .expect("proposed");

    store
        .remove_studio_nodes(&studio, &[second], &at(3))
        .expect("removed");
    let left = store.studio(&studio).expect("reads");
    assert_eq!(left.nodes.len(), 1);
    assert!(left.edges.is_empty(), "the edge went with its node");

    store.delete_studio(&studio).expect("deleted");
    let gone = store.studio(&studio).expect_err("deleted");
    assert!(matches!(gone, StudioError::NoSuchStudio { .. }), "{gone}");
    let nodes: i64 = store
        .conn
        .query_row("SELECT COUNT(*) FROM studio_nodes", [], |row| row.get(0))
        .expect("counts");
    assert_eq!(nodes, 1, "only the other Studio's node is left");
    let again = store.delete_studio(&studio).expect_err("nothing to delete");
    assert!(matches!(again, StudioError::NoSuchStudio { .. }), "{again}");
}

/// **Kept until a person deletes it.** Nothing here points at a Job, so
/// forgetting one reaches none of these tables.
#[test]
fn no_studio_table_is_one_forgetting_a_job_reaches() {
    let dir = TempDir::new();
    let store = open(&dir);
    let reached = tables_pointing_at_a_job(&store.conn).expect("reads the catalog");
    assert!(
        !reached.iter().any(|table| table.starts_with("studio")),
        "{reached:?}"
    );
}

#[test]
fn a_node_whose_content_does_not_read_back_is_refused_by_name() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", "armada", 0);
    a_note(&mut store, &studio, "01NOTE", "fine as written", 0);
    store
        .conn
        .execute(
            "UPDATE studio_nodes SET content = '{\"title\":\"no said\"}'",
            [],
        )
        .expect("damaged by hand");
    let refused = store.studio(&studio).expect_err("a Note with nothing said");
    assert!(
        matches!(
            &refused,
            StudioError::Unreadable { id, why: Unreadable::Content(_), .. } if id == "01NOTE"
        ),
        "{refused}"
    );
}

/// `#1292`: **a Finding is the one node rewritten, and only as its scout
/// moves it.** Gathering, it is listed as one a restart has to settle; Frozen,
/// it reads back after a reopen with every file, its checkout and its cost.
#[test]
fn a_scouts_finding_is_kept_as_it_gathers_and_reads_back_frozen_after_a_reopen() {
    let dir = TempDir::new();
    let studio = {
        let mut store = open(&dir);
        let studio = a_studio(&mut store, "01STUDIO", "armada", 0);
        let proposed = StudioNode::added(
            node_id("01FINDING"),
            StudioNodeContent::Finding(StudioFinding::asked("how is routing decided")),
            StudioPosition { x: 0, y: 0 },
            at(1),
            StudioAuthor::Person,
        );
        store
            .add_studio_node(&studio, &proposed, None, &at(1))
            .expect("added");
        let mut gathering = proposed
            .scouting(ScoutCheckout {
                commit: "4bdb169c".to_string(),
                uncommitted: true,
            })
            .expect("proposed");
        gathering.looked(ScoutLook::File("crates/fleet/src/routing.rs".to_string()));
        store
            .keep_scouted(&studio, &gathering, &at(2))
            .expect("kept");
        let listed = store.gathering_findings().expect("read");
        assert_eq!(listed, vec![(studio.clone(), gathering.clone())]);

        let frozen = gathering.frozen(
            Some("By weight.".to_string()),
            ScoutEnded {
                outcome: ScoutOutcome::Failed {
                    why: "the agent exited 1".to_string(),
                },
                cost_micros: Some(420),
            },
        );
        store.keep_scouted(&studio, &frozen, &at(3)).expect("kept");
        assert!(store.gathering_findings().expect("read").is_empty());
        studio
    };

    let store = open(&dir);
    let graph = store.studio(&studio).expect("reads back");
    let node = &graph.nodes[0];
    assert_eq!(node.state(), Some(StudioNodeState::Frozen));
    let StudioNodeContent::Finding(finding) = node.content() else {
        panic!("a Finding");
    };
    assert_eq!(finding.read(), ["crates/fleet/src/routing.rs".to_string()]);
    assert_eq!(finding.checkout().map(|at| at.uncommitted), Some(true));
    assert_eq!(finding.learned(), Some("By weight."));
    let ended = finding.ended().expect("ended");
    assert_eq!(ended.cost_micros, Some(420));
    assert!(matches!(&ended.outcome, ScoutOutcome::Failed { why } if why == "the agent exited 1"));
    assert_eq!(graph.studio.touched_at, at(3));
}

/// **A Note's capture reads back whole after a reopen.** `#1290`.
///
/// The failure this is against is a capture that survives the write and loses
/// a field on the way back — a selector, a style or the frame's name, each of
/// which is what turns the Note back into the element it was left on.
#[test]
fn a_captured_notes_styles_markup_and_frame_read_back_after_a_reopen() {
    let dir = TempDir::new();
    let capture = core_model::StudioCapture {
        component: Some("FilterChip".to_string()),
        owners: vec!["BoardFilters".to_string(), "Board".to_string()],
        selector: "button.armada-chip:nth-of-type(2)".to_string(),
        element: core_model::CaptureElement {
            tag: "button".to_string(),
            text: "Queued 3".to_string(),
            label: Some("Queued, 3 Jobs".to_string()),
        },
        screen: Some("Job Board".to_string()),
        layer: None,
        location: "/".to_string(),
        bounds: core_model::CaptureBounds {
            x: 312,
            y: 148,
            width: 96,
            height: 28,
        },
        window: core_model::CaptureWindow {
            width: 1440,
            height: 900,
        },
        styles: [("color".to_string(), "rgb(232, 232, 237)".to_string())]
            .into_iter()
            .collect(),
        markup: "<button class=\"armada-chip\">Queued 3</button>".to_string(),
        source: None,
        frame: Some(core_model::CaptureFrame {
            filename: "01POINTED.png".to_string(),
            byte_size: 214_880,
            width: 2880,
            height: 1800,
        }),
        // Captured in the capture window rather than on Bridge — `#1294`.
        served: Some(core_model::CaptureServed {
            run: "01SERVER".to_string(),
            name: "web_dev".to_string(),
            address: "http://127.0.0.1:41207".to_string(),
        }),
    };
    let studio = {
        let mut store = open(&dir);
        let studio = a_studio(&mut store, "01STUDIO", "armada", 0);
        let node = StudioNode::added(
            node_id("01POINTED"),
            StudioNodeContent::Note {
                said: "The chip keeps its count".to_string(),
                capture: Some(capture.clone()),
            },
            StudioPosition { x: 0, y: 0 },
            at(1),
            StudioAuthor::Person,
        );
        store
            .add_studio_node(&studio, &node, None, &at(1))
            .expect("added");
        studio
    };

    let store = open(&dir);
    let graph = store.studio(&studio).expect("read back");
    let StudioNodeContent::Note {
        said,
        capture: read,
    } = graph.nodes[0].content()
    else {
        panic!("a Note: {:?}", graph.nodes[0].content());
    };
    assert_eq!(said, "The chip keeps its count");
    assert_eq!(
        read.as_ref(),
        Some(&capture),
        "every field of the capture, the frame's name and size included"
    );
}

/// A Note kept before capture existed has no capture and still reads back:
/// the field is absent from the content object rather than written as null.
#[test]
fn a_note_with_no_capture_reads_back_as_one() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", "armada", 0);
    a_note(&mut store, &studio, "01PLAIN", "typed, not pointed", 0);

    let graph = store.studio(&studio).expect("read back");
    let StudioNodeContent::Note { capture, .. } = graph.nodes[0].content() else {
        panic!("a Note");
    };
    assert!(capture.is_none(), "nothing was pointed at");
}

/// `#1411`'s own claim, the half the store answers for: a person picks every
/// node and one write takes all of them, with the edges on them.
#[test]
fn removing_every_node_picked_is_one_write_that_takes_their_edges_with_them() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", "armada", 0);
    let kept = a_studio(&mut store, "01KEPT", "armada", 0);
    let picked: Vec<StudioNodeId> = (0..18)
        .map(|which| {
            a_note(
                &mut store,
                &studio,
                &format!("01NODE{which:012}"),
                "one",
                which,
            )
        })
        .collect();
    a_note(&mut store, &kept, "01ELSEWHERE0", "another Studio", 0);
    store
        .add_studio_edge(&studio, &proposed("01EDGE", &picked[0], &picked[1]), &at(2))
        .expect("proposed");

    store
        .remove_studio_nodes(&studio, &picked, &at(3))
        .expect("removed");

    let left = store.studio(&studio).expect("reads");
    assert!(left.nodes.is_empty(), "every picked node went");
    assert!(left.edges.is_empty(), "the edge went with its nodes");
    assert_eq!(left.studio.touched_at, at(3), "the Studio was touched once");
    let elsewhere = store.studio(&kept).expect("reads");
    assert_eq!(elsewhere.nodes.len(), 1, "another Studio is untouched");
}

/// **All of them or none.** A selection carrying one name this Studio does not
/// hold refuses, and every node it does hold is still there afterwards — the
/// half-delete the issue exists against.
#[test]
fn a_delete_that_names_a_node_this_studio_does_not_hold_leaves_every_node_in_place() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", "armada", 0);
    let elsewhere = a_studio(&mut store, "01KEPT", "armada", 0);
    let first = a_note(&mut store, &studio, "01NOTE1", "one", 0);
    let second = a_note(&mut store, &studio, "01NOTE2", "two", 100);
    let third = a_note(&mut store, &studio, "01NOTE3", "three", 200);
    // A node of another Studio, which is exactly as absent from this one as a
    // name nothing ever minted.
    let another = a_note(&mut store, &elsewhere, "01NOTE4", "four", 0);
    store
        .add_studio_edge(&studio, &proposed("01EDGE", &first, &second), &at(2))
        .expect("proposed");

    let refused = store
        .remove_studio_nodes(
            &studio,
            &[first.clone(), another.clone(), third.clone()],
            &at(3),
        )
        .expect_err("one name is not on this Studio");
    assert!(
        matches!(&refused, StudioError::NoSuchNode { node_id } if node_id == another.as_str()),
        "{refused}"
    );

    let left = store.studio(&studio).expect("reads");
    assert_eq!(left.nodes.len(), 3, "nothing named before it went");
    assert_eq!(left.edges.len(), 1, "and no edge went either");
    assert_eq!(
        left.studio.touched_at,
        at(2),
        "the Studio is where the edge left it: a refused write rolls its touch back too"
    );
    assert_eq!(
        store.studio(&elsewhere).expect("reads").nodes.len(),
        1,
        "the other Studio's node was never this call's to take"
    );
}

/// The same node named twice is one node, not a refusal on the second pass:
/// every name is checked before anything is deleted.
#[test]
fn a_node_named_twice_in_one_delete_goes_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let studio = a_studio(&mut store, "01STUDIO", "armada", 0);
    let only = a_note(&mut store, &studio, "01NOTE1", "one", 0);
    let other = a_note(&mut store, &studio, "01NOTE2", "two", 100);

    store
        .remove_studio_nodes(&studio, &[only.clone(), only.clone()], &at(3))
        .expect("removed");

    let left = store.studio(&studio).expect("reads");
    assert_eq!(left.nodes.len(), 1, "the other node stayed");
    assert_eq!(left.nodes[0].id(), &other);
}
