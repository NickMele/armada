//! What reading an Epic in again takes back and what it leaves. `#1405`.
//!
//! **A graph built by hand, so each mark is held on its own.** Fleet's own
//! read-in test drives the whole act through a stand-in forge; what is held
//! here is the decision, which is where the rule about a person's work lives.

use adapters::{AnIssue, MilestoneRead};
use core_model::{
    EpicRead, EpicTake, ForgeState, ManifestId, StudioAuthor, StudioEdge, StudioEdgeId,
    StudioGraph, StudioId, StudioNode, StudioNodeContent, StudioNodeId, StudioNodeKind,
    StudioPosition, StudioRelation, Timestamp, Ulid,
};

use super::{what_it_took, WhatTheEpicTook};
use crate::reading_in::{laid_out, ACROSS};

fn node_id(id: &str) -> StudioNodeId {
    StudioNodeId::carried(Ulid::carried(id))
}

fn at() -> Timestamp {
    Timestamp::from_rfc3339("2026-09-17T09:00:00.000Z")
}

const CORNER: StudioPosition = StudioPosition { x: 400, y: 0 };

/// A milestone of three issues: two open, one closed.
fn milestone() -> MilestoneRead {
    MilestoneRead {
        title: "Studio".into(),
        issues: vec![
            an_issue("1", ForgeState::Open),
            an_issue("2", ForgeState::Closed),
            an_issue("3", ForgeState::Open),
        ],
        total: 3,
    }
}

fn an_issue(number: &str, state: ForgeState) -> AnIssue {
    AnIssue {
        address: address(number),
        number: number.into(),
        title: format!("Issue {number}"),
        state: Some(state),
    }
}

fn address(number: &str) -> String {
    format!("https://example.invalid/o/r/issues/{number}")
}

/// An Epic that has been read in once, and the Issue nodes that read made.
fn already_read(numbers: &[&str], read_in: Option<EpicRead>) -> StudioGraph {
    let epic = StudioNode::added(
        node_id("epic"),
        StudioNodeContent::on_the_forge(
            StudioNodeKind::Epic,
            "https://example.invalid/o/r/milestone/17".into(),
            "17".into(),
            None,
        )
        .expect("an Epic")
        .resolved(&core_model::ForgeFacts {
            title: Some("Studio".into()),
            state: None,
            read_in,
        })
        .expect("an Epic"),
        StudioPosition { x: 0, y: 0 },
        at(),
        StudioAuthor::Person,
    );
    let mut nodes = vec![epic];
    let mut edges = Vec::new();
    for (n, number) in numbers.iter().enumerate() {
        let id = node_id(number);
        nodes.push(StudioNode::added(
            id.clone(),
            StudioNodeContent::on_the_forge(
                StudioNodeKind::Issue,
                address(number),
                (*number).into(),
                None,
            )
            .expect("an Issue"),
            laid_out(CORNER, n as i64),
            at(),
            StudioAuthor::Person,
        ));
        edges.push(
            StudioEdge::produced(
                StudioEdgeId::carried(Ulid::carried(&format!("e{number}"))),
                node_id("epic"),
                id,
                at(),
                StudioAuthor::Person,
            )
            .expect("an edge"),
        );
    }
    StudioGraph {
        studio: core_model::Studio {
            id: StudioId::carried(Ulid::carried("studio")),
            manifest_id: ManifestId::carried(Ulid::carried("manifest")),
            name: None,
            named_by: None,
            created_at: at(),
            touched_at: at(),
        },
        nodes,
        edges,
    }
}

fn read(graph: &StudioGraph, take: EpicTake) -> WhatTheEpicTook {
    let epic = graph.nodes[0].clone();
    what_it_took(graph, &epic, take, &milestone(), CORNER)
}

/// An Epic nobody has read in.
const UNREAD: Option<EpicRead> = None;

/// An Epic that took every issue, into the block at [`CORNER`].
const EVERYTHING: Option<EpicRead> = Some(EpicRead {
    issues: 3,
    total: 3,
    took: Some(EpicTake::Everything),
    left_out: 0,
    kept: 0,
    laid_out_from: Some(CORNER),
});

/// **The first read is the answer alone**: nothing is on the board, so every
/// issue the answer takes is made and nothing is taken back.
#[test]
fn only_what_is_open_takes_the_open_issues_and_says_what_it_left_out() {
    let empty = already_read(&[], UNREAD);
    let what = read(&empty, EpicTake::Open);
    assert_eq!(what.made.len(), 2, "the two open issues");
    assert!(what.taken_back.is_empty());
    assert_eq!(what.read_in.issues, 2);
    assert_eq!(what.read_in.total, 3);
    assert_eq!(what.read_in.took, Some(EpicTake::Open));
    assert_eq!(what.read_in.left_out, 1, "the closed one");
    assert_eq!(what.read_in.kept, 0);
    assert_eq!(what.read_in.laid_out_from, Some(CORNER));
}

/// Narrowing an Epic that took everything takes back the closed one, and the
/// two open nodes stay exactly where they were.
#[test]
fn narrowing_takes_back_the_closed_issue_and_moves_nothing_else() {
    let graph = already_read(&["1", "2", "3"], EVERYTHING);
    let what = read(&graph, EpicTake::Open);
    assert_eq!(what.taken_back, vec![node_id("2")], "the closed issue");
    assert!(what.made.is_empty(), "both open ones are already here");
    assert_eq!(what.read_in.issues, 2);
    assert_eq!(what.read_in.kept, 0);
}

/// **The issue's own case**: a Note written against a closed issue keeps its
/// node, and the Epic counts it.
#[test]
fn a_node_something_hangs_off_stays_and_the_epic_says_it_kept_it() {
    let mut graph = already_read(&["1", "2", "3"], EVERYTHING);
    graph.nodes.push(StudioNode::added(
        node_id("note"),
        StudioNodeContent::Note {
            said: "this one shipped without the legend".into(),
            capture: None,
        },
        StudioPosition { x: 900, y: 0 },
        at(),
        StudioAuthor::Person,
    ));
    graph.edges.push(
        StudioEdge::produced(
            StudioEdgeId::carried(Ulid::carried("en")),
            node_id("2"),
            node_id("note"),
            at(),
            StudioAuthor::Person,
        )
        .expect("an edge"),
    );
    let what = read(&graph, EpicTake::Open);
    assert!(
        what.taken_back.is_empty(),
        "a person's own work is not the read-in's to remove"
    );
    assert_eq!(what.read_in.kept, 1);
    assert_eq!(what.read_in.issues, 3, "two taken and one kept");
}

/// A promotion the other way round — an edge drawn *onto* the node, which is
/// what accepting a relation makes — keeps it too.
#[test]
fn a_relation_drawn_onto_a_node_keeps_it() {
    let mut graph = already_read(&["1", "2", "3"], EVERYTHING);
    graph.edges.push(
        StudioEdge::proposed(
            StudioEdgeId::carried(Ulid::carried("er")),
            node_id("1"),
            node_id("2"),
            StudioRelation::Blocks,
            at(),
            StudioAuthor::Person,
        )
        .expect("an edge"),
    );
    let what = read(&graph, EpicTake::Open);
    assert!(what.taken_back.is_empty());
    assert_eq!(what.read_in.kept, 1);
}

/// A line of a person's own beside the address keeps it: nothing but an edit
/// writes one.
#[test]
fn a_line_a_person_wrote_keeps_the_node() {
    let mut graph = already_read(&["1", "2", "3"], EVERYTHING);
    let closed = graph.nodes[2].clone();
    graph.nodes[2] = StudioNode::recorded(
        closed.id().clone(),
        closed
            .content()
            .with_said(Some("the legend regressed here".into()))
            .expect("a kind that keeps an address"),
        None,
        closed.position(),
        at(),
        Some(StudioAuthor::Person),
    )
    .expect("an Issue");
    let what = read(&graph, EpicTake::Open);
    assert!(what.taken_back.is_empty());
    assert_eq!(what.read_in.kept, 1);
}

/// A node dragged out of the block it was laid out in keeps it — and one still
/// in the block does not.
#[test]
fn a_node_somebody_moved_stays_and_one_still_in_the_block_does_not() {
    let graph = already_read(&["1", "2", "3"], EVERYTHING);
    let mut moved = graph.clone();
    moved.nodes[2] = moved.nodes[2].moved(StudioPosition { x: 1_234, y: 77 });
    assert_eq!(read(&moved, EpicTake::Open).read_in.kept, 1);
    assert!(read(&moved, EpicTake::Open).taken_back.is_empty());
    assert_eq!(read(&graph, EpicTake::Open).read_in.kept, 0);
}

/// **An Epic read in before `#1405` kept no corner**, so where its issues sit
/// says nothing and narrowing it still works.
#[test]
fn an_epic_read_before_this_change_narrows_on_its_edges_alone() {
    let graph = already_read(&["1", "2", "3"], UNREAD);
    let mut moved = graph.clone();
    moved.nodes[2] = moved.nodes[2].moved(StudioPosition { x: 1_234, y: 77 });
    let what = what_it_took(
        &moved,
        &moved.nodes[0].clone(),
        EpicTake::Open,
        &milestone(),
        CORNER,
    );
    assert_eq!(what.taken_back, vec![node_id("2")]);
    assert_eq!(what.read_in.kept, 0);
    assert_eq!(
        what.read_in.laid_out_from,
        Some(CORNER),
        "the corner the block already had, now recorded"
    );
}

/// Widening fills the gap the narrowing left rather than laying a second grid
/// over the first.
#[test]
fn widening_puts_the_closed_issue_back_in_its_own_block() {
    let narrowed = already_read(
        &["1", "3"],
        Some(EpicRead {
            issues: 2,
            total: 3,
            took: Some(EpicTake::Open),
            left_out: 1,
            kept: 0,
            laid_out_from: Some(CORNER),
        }),
    );
    // The two open issues are in cells 0 and 1, so the closed one lands in 2.
    let what = what_it_took(
        &narrowed,
        &narrowed.nodes[0].clone(),
        EpicTake::Everything,
        &milestone(),
        StudioPosition {
            x: CORNER.x + ACROSS * 9,
            y: 900,
        },
    );
    assert_eq!(what.made.len(), 1);
    assert_eq!(what.made[0].1, laid_out(CORNER, 2), "the block's own gap");
    assert_eq!(what.read_in.issues, 3);
    assert_eq!(what.read_in.left_out, 0);
}
