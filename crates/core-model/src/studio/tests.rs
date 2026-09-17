//! What a Studio's vocabulary refuses: a state its kind does not hold, a
//! relation that is not one, an edge from a node to itself.

use alloc::string::String;

use crate::envelope::{Timestamp, Ulid};
use crate::studio::{
    EdgeRefused, StudioAuthor, StudioEdge, StudioEdgeId, StudioEdgeKind, StudioEdgeStanding,
    StudioName, StudioNode, StudioNodeContent, StudioNodeId, StudioNodeKind, StudioNodeState,
    StudioPosition, StudioRelation, ToItself,
};
use crate::studio::{
    NotScoutable, ScoutCheckout, ScoutEnded, ScoutLook, ScoutOutcome, StudioFinding,
};

fn node_id(id: &str) -> StudioNodeId {
    StudioNodeId::carried(Ulid::carried(id))
}

fn at() -> Timestamp {
    Timestamp::from_rfc3339("2026-09-17T09:00:00.000Z")
}

#[test]
fn every_set_reads_back_from_its_own_spelling() {
    assert_eq!(StudioNodeKind::ALL.len(), 11, "studio.md's eleven kinds");
    for kind in StudioNodeKind::ALL {
        assert_eq!(StudioNodeKind::from_wire(kind.as_wire()), Some(*kind));
    }
    for state in StudioNodeState::ALL {
        assert_eq!(StudioNodeState::from_wire(state.as_wire()), Some(*state));
    }
    assert_eq!(StudioEdgeKind::ALL.len(), 4, "studio.md's four edges");
    for kind in StudioEdgeKind::ALL {
        assert_eq!(StudioEdgeKind::from_wire(kind.as_wire()), Some(*kind));
    }
    assert_eq!(StudioNodeKind::IssueDraft.as_wire(), "issue_draft");
}

/// A relation is spelled as the edge kind it becomes, and none becomes
/// `Produced`.
#[test]
fn no_relation_is_the_studios_own_produced_edge() {
    for relation in StudioRelation::ALL {
        let kind = StudioEdgeKind::from(*relation);
        assert_ne!(kind, StudioEdgeKind::Produced);
        assert_eq!(kind.as_wire(), relation.as_wire());
    }
}

/// **A Run or a Job reads its state off what it references**, so neither
/// holds one here; a Finding never reads back without one.
#[test]
fn a_state_is_held_only_by_a_kind_that_has_it() {
    assert!(StudioNodeKind::Run.admits(None));
    assert!(!StudioNodeKind::Job.admits(Some(StudioNodeState::Frozen)));
    assert!(!StudioNodeKind::Finding.admits(None));
    assert!(StudioNodeKind::Contradiction.admits(Some(StudioNodeState::ResolvedHere)));
    assert!(!StudioNodeKind::Note.admits(Some(StudioNodeState::Draft)));
    let proposed: alloc::vec::Vec<_> = StudioNodeKind::ALL
        .iter()
        .filter(|kind| kind.starts_proposed())
        .collect();
    assert_eq!(proposed, [&StudioNodeKind::Finding]);

    let refused = StudioNode::recorded(
        node_id("01NOTE"),
        StudioNodeContent::Note {
            said: String::from("the count is stale"),
        },
        Some(StudioNodeState::Frozen),
        StudioPosition { x: 0, y: 0 },
        at(),
        Some(StudioAuthor::Person),
    );
    assert!(refused.is_err(), "a Note has no state to be frozen in");
}

#[test]
fn a_node_starts_in_its_kinds_first_state_and_moving_it_changes_nothing_else() {
    let finding = StudioNode::added(
        node_id("01FINDING"),
        StudioNodeContent::Finding(StudioFinding::asked("what reads the count")),
        StudioPosition { x: 10, y: -4 },
        at(),
        StudioAuthor::Helm,
    );
    assert_eq!(finding.state(), Some(StudioNodeState::Proposed));
    let moved = finding.moved(StudioPosition { x: 300, y: 12 });
    assert_eq!(moved.position(), StudioPosition { x: 300, y: 12 });
    assert_eq!(moved.content(), finding.content());
    assert_eq!(moved.state(), finding.state());
    assert_eq!(moved.id(), finding.id());
    assert_eq!(
        moved.added_by(),
        Some(StudioAuthor::Helm),
        "who added it moves with it"
    );
}

#[test]
fn an_edge_to_itself_and_an_unaccepted_produced_edge_are_refused() {
    let same = StudioEdge::proposed(
        StudioEdgeId::carried(Ulid::carried("01EDGE")),
        node_id("01A"),
        node_id("01A"),
        StudioRelation::SameAs,
        at(),
        StudioAuthor::Person,
    );
    assert!(matches!(same, Err(ToItself { .. })));
    let produced = StudioEdge::recorded(
        StudioEdgeId::carried(Ulid::carried("01EDGE")),
        node_id("01A"),
        node_id("01B"),
        StudioEdgeKind::Produced,
        StudioEdgeStanding::Proposed,
        at(),
        None,
    );
    assert_eq!(produced, Err(EdgeRefused::ProducedUnaccepted));
}

#[test]
fn a_blank_name_is_no_name_and_a_blank_field_is_named() {
    assert_eq!(StudioName::named("   "), None);
    assert_eq!(
        StudioName::named("  Stale counts ").map(|name| String::from(name.as_str())),
        Some(String::from("Stale counts"))
    );
    let draft = StudioNodeContent::IssueDraft {
        title: String::from("Counts go stale"),
        body: String::from(" "),
    };
    assert_eq!(draft.blank(), Some("body"));
}

fn a_proposed_finding() -> StudioNode {
    StudioNode::added(
        node_id("01FINDING"),
        StudioNodeContent::Finding(StudioFinding::asked("how is routing decided")),
        StudioPosition { x: 0, y: 0 },
        at(),
        StudioAuthor::Person,
    )
}

fn checked_out() -> ScoutCheckout {
    ScoutCheckout {
        commit: String::from("4bdb169c"),
        uncommitted: true,
    }
}

/// **A Finding moves Proposed, Gathering, Frozen, and only its scout moves
/// it.** Each step keeps what the one before recorded, and a file read twice
/// is listed once.
#[test]
fn a_finding_is_started_gathers_what_it_read_and_freezes_with_how_it_ended() {
    let proposed = a_proposed_finding();
    let mut gathering = proposed
        .scouting(checked_out())
        .expect("a proposed Finding");
    assert_eq!(gathering.node().state(), Some(StudioNodeState::Gathering));
    assert!(gathering.looked(ScoutLook::File(String::from("crates/fleet/src/routing.rs"))));
    assert!(!gathering.looked(ScoutLook::File(String::from("crates/fleet/src/routing.rs"))));
    assert!(gathering.looked(ScoutLook::Search(String::from("route in crates"))));

    let frozen = gathering.frozen(
        Some(String::from("By weight.")),
        ScoutEnded {
            outcome: ScoutOutcome::Answered,
            cost_micros: Some(18_020),
        },
    );
    let node = frozen.node();
    assert_eq!(node.state(), Some(StudioNodeState::Frozen));
    let StudioNodeContent::Finding(finding) = node.content() else {
        panic!("still a Finding");
    };
    assert_eq!(finding.ask(), "how is routing decided");
    assert_eq!(
        finding.read(),
        [String::from("crates/fleet/src/routing.rs")]
    );
    assert_eq!(finding.searched(), [String::from("route in crates")]);
    assert_eq!(finding.checkout(), Some(&checked_out()));
    assert_eq!(finding.learned(), Some("By weight."));
    assert_eq!(node.id(), proposed.id());
    assert_eq!(node.position(), proposed.position());
}

#[test]
fn only_a_proposed_finding_is_started() {
    let note = StudioNode::added(
        node_id("01NOTE"),
        StudioNodeContent::Note {
            said: String::from("the count is stale"),
        },
        StudioPosition { x: 0, y: 0 },
        at(),
        StudioAuthor::Person,
    );
    assert_eq!(note.scouting(checked_out()), Err(NotScoutable::NotAFinding));
    let started = a_proposed_finding()
        .scouting(checked_out())
        .expect("a proposed Finding");
    assert_eq!(
        started.node().scouting(checked_out()),
        Err(NotScoutable::NotProposed(StudioNodeState::Gathering))
    );
}

/// **A Finding's content says which state it is in**, so a row claiming a
/// Frozen Finding that never recorded its checkout does not read back.
#[test]
fn a_finding_whose_content_does_not_fit_its_state_is_refused() {
    let claimed = StudioFinding::recorded(
        String::from("how is routing decided"),
        None,
        alloc::vec![String::from("src/lib.rs")],
        alloc::vec::Vec::new(),
        None,
        None,
    );
    for state in [StudioNodeState::Proposed, StudioNodeState::Frozen] {
        let read = StudioNode::recorded(
            node_id("01FINDING"),
            StudioNodeContent::Finding(claimed.clone()),
            Some(state),
            StudioPosition { x: 0, y: 0 },
            at(),
            Some(StudioAuthor::Person),
        );
        assert!(read.is_err(), "{state:?}");
    }
}
