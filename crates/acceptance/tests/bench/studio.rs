//! Studio's apparatus: the text an Issue draft would hold, the catalogue a
//! proposer reads it against, and the round trip a dispatch request makes. It
//! asserts nothing.
//!
//! **No Studio type appears here, because none exists.** The two Notes and the
//! draft written up from them are strings, not nodes: a Note, a Cluster and an
//! Issue draft are #1285's to mint, and a fixture that invented their shape
//! here would be a second vocabulary for #1285 to find and reconcile. When the
//! record lands, the draft below is read off an Issue draft node rather than
//! written out, and this file's constants go.

use std::collections::BTreeMap;

use config::ResolvedWorkflow;
use core_model::WorkflowId;
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
