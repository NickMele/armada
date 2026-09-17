//! Studio's apparatus: a Studio with two Notes on it, the text an Issue draft
//! would hold, the catalogue a proposer reads it against, and the round trip a
//! request and a record make. It asserts nothing.
//!
//! **The record is `core_model`'s and its wire form is `ipc`'s.** What this
//! file adds is the assembly — which Notes, where a person left them, and the
//! Same as edge proposed between them — built through the constructors `store`
//! reads a row back with, so nothing here is a second vocabulary for a node.
//!
//! The draft is still written out rather than read off an Issue draft node:
//! writing one up from Notes is #1291's, and its constants go when that lands.

use std::collections::BTreeMap;

use config::ResolvedWorkflow;
use core_model::{
    ManifestId, ScoutCheckout, ScoutEnded, ScoutLook, ScoutOutcome, Studio, StudioAuthor,
    StudioEdge, StudioEdgeId, StudioFinding, StudioGraph, StudioId, StudioName, StudioNode,
    StudioNodeContent, StudioNodeId, StudioPosition, StudioRelation, Timestamp, Ulid, WorkflowId,
};
use ipc::JobRequest;

use super::bug_workflow_with_the_fix_judged;

/// What a person pointed at on Bridge and said, the first time.
pub const FIRST_NOTE: &str = "The Board's filter chip keeps its count after the filter is cleared";

/// The second Note, captured later on another screen, about the same fault.
pub const SECOND_NOTE: &str = "Overview still says three waiting after I answered one";

/// The Issue draft's title, as a person would write it up.
pub const DRAFT_TITLE: &str = "Counts go stale after the thing they count changes";

/// The Issue draft, title and body, as the text a dispatch sends.
///
/// **Both Notes' words are in the body**, because a write-up is made from the
/// Notes feeding it, and what the proposer and then the Drone are told has to
/// be what the person actually saw rather than a summary of it.
pub fn an_issue_draft() -> String {
    format!(
        "{DRAFT_TITLE}\n\n\
         Two places show a count that does not move when what it counts does.\n\n\
         - {FIRST_NOTE}\n\
         - {SECOND_NOTE}\n"
    )
}

/// The request as Fleet receives it: encoded, sent, and read back.
///
/// **No `client_ref` and no attachments.** A draft is dispatched from its text
/// alone, and nothing here names a filed issue — filing one is optional
/// and a person's own act.
pub fn received_request(draft: &str) -> JobRequest {
    let sent = JobRequest {
        request: draft.to_string(),
        client_ref: None,
        attachments: Vec::new(),
    };
    let body = ipc::encode(&sent).expect("a request that serialises");
    ipc::decode("a Job request", body.as_bytes()).expect("a request that reads back")
}

/// The workflows the proposer is offered: one, the bench's Bug.
pub fn held() -> BTreeMap<WorkflowId, ResolvedWorkflow> {
    let bug = bug_workflow_with_the_fix_judged();
    BTreeMap::from([(bug.id().clone(), bug)])
}

/// A proposer's answer naming one Job under `workflow`, titled as the draft is.
pub fn one_job_under(workflow: &WorkflowId) -> String {
    format!(
        "workflow: {}\ntitle: {DRAFT_TITLE}\nbecause: a fault in what two screens draw",
        workflow.as_str()
    )
}

/// The repository the bench's Studio belongs to.
pub const REPOSITORY: &str = "01FIXTUREMANIFEST";

/// Where a person left the second Note, having dragged it off where it landed.
pub const LEFT_AT: StudioPosition = StudioPosition { x: 480, y: -120 };

fn at(minute: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-09-17T09:{minute:02}:00.000Z"))
}

/// A Studio in the bench's repository holding both Notes, the second moved to
/// [`LEFT_AT`], and a Same as edge Helm proposed between them, not yet accepted.
pub fn a_studio_with_two_notes() -> StudioGraph {
    let note = |id: &str, said: &str, x: i64| {
        StudioNode::added(
            StudioNodeId::carried(Ulid::carried(id)),
            StudioNodeContent::Note {
                said: said.to_string(),
            },
            StudioPosition { x, y: 0 },
            at(1),
            StudioAuthor::Person,
        )
    };
    let first = note("01NOTEFIRST", FIRST_NOTE, 0);
    let second = note("01NOTESECOND", SECOND_NOTE, 240).moved(LEFT_AT);
    let same_as = StudioEdge::proposed(
        StudioEdgeId::carried(Ulid::carried("01EDGESAMEAS")),
        first.id().clone(),
        second.id().clone(),
        StudioRelation::SameAs,
        at(2),
        StudioAuthor::Helm,
    )
    .expect("two different Notes");
    StudioGraph {
        studio: Studio {
            id: StudioId::carried(Ulid::carried("01STUDIO")),
            manifest_id: ManifestId::carried(Ulid::carried(REPOSITORY)),
            name: StudioName::named(DRAFT_TITLE),
            named_by: Some(StudioAuthor::Person),
            created_at: at(0),
            touched_at: at(3),
        },
        nodes: vec![first, second],
        edges: vec![same_as],
    }
}

/// The Studio as a client reads it: served, encoded, sent, and read back.
pub fn received_studio(graph: &StudioGraph) -> ipc::Studio {
    let body = ipc::encode(&ipc::Studio::of(graph)).expect("a Studio that serialises");
    ipc::decode("a Studio", body.as_bytes()).expect("a Studio that reads back")
}

/// The repository Helm is briefed for, named and never quoted.
pub fn helms_manifest() -> config::Manifest {
    config::Manifest::parse(
        std::path::Path::new("/work/storefront/armada.yml"),
        &format!("version: 1\nid: {REPOSITORY}\n"),
    )
    .expect("a Manifest")
}

/// Helm's act on a Studio as a client reads it off the stream.
pub fn received_event(event: &ipc::Event) -> ipc::Event {
    let body = ipc::encode(event).expect("an event that serialises");
    ipc::decode("an event", body.as_bytes()).expect("an event that reads back")
}

/// What a person asked a scout, on the bench's Studio.
pub const ASKED: &str = "How is a Job's count on the Board decided?";

/// The files the bench's scout read, in the order it read them.
pub const READ: [&str; 2] = [
    "packages/screens/src/Board.tsx",
    "crates/fleet/src/listing.rs",
];

/// The commit the bench's scout read, with a change on top not yet committed.
pub const COMMIT: &str = "4bdb169c0e1d2f3a4b5c6d7e8f9a0b1c2d3e4f5a";

/// What the scout cost, in millionths of a dollar.
pub const COST: u64 = 18_020;

/// The bench's Studio with a Finding on it, made by the first Note, which its
/// scout started, read [`READ`] into and froze as answered — **through the
/// transitions a scout's Fleet makes**, and nothing that sets a state by hand.
pub fn a_studio_with_a_frozen_finding() -> StudioGraph {
    let mut graph = a_studio_with_two_notes();
    let proposed = StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01FINDING")),
        StudioNodeContent::Finding(StudioFinding::asked(ASKED)),
        StudioPosition { x: 0, y: 240 },
        at(4),
    );
    let mut gathering = proposed
        .scouting(ScoutCheckout {
            commit: COMMIT.to_string(),
            uncommitted: true,
        })
        .expect("a Finding just added is Proposed");
    for file in READ {
        gathering.looked(ScoutLook::File(file.to_string()));
    }
    gathering.looked(ScoutLook::Search("count in packages/screens".to_string()));
    let frozen = gathering.frozen(
        Some("The Board counts what `list_job_board` answers.".to_string()),
        ScoutEnded {
            outcome: ScoutOutcome::Answered,
            cost_micros: Some(COST),
        },
    );
    let produced = StudioEdge::produced(
        StudioEdgeId::carried(Ulid::carried("01EDGEASKED")),
        graph.nodes[0].id().clone(),
        frozen.node().id().clone(),
        at(4),
    )
    .expect("a Note and a Finding");
    graph.nodes.push(frozen.node().clone());
    graph.edges.push(produced);
    graph
}
