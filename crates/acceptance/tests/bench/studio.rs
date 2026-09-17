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
    ManifestId, Studio, StudioEdge, StudioEdgeId, StudioGraph, StudioId, StudioName, StudioNode,
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
/// [`LEFT_AT`], and a Same as edge proposed between them and not yet accepted.
pub fn a_studio_with_two_notes() -> StudioGraph {
    let note = |id: &str, said: &str, x: i64| {
        StudioNode::added(
            StudioNodeId::carried(Ulid::carried(id)),
            StudioNodeContent::Note {
                said: said.to_string(),
            },
            StudioPosition { x, y: 0 },
            at(1),
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
    )
    .expect("two different Notes");
    StudioGraph {
        studio: Studio {
            id: StudioId::carried(Ulid::carried("01STUDIO")),
            manifest_id: ManifestId::carried(Ulid::carried(REPOSITORY)),
            name: StudioName::named(DRAFT_TITLE),
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
