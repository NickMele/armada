//! Setup's proposal routes: the proposals, an edit to one, and its Write.
//!
//! **Under `/repository` beside Scan, not under `/manifest`**, for Scan's
//! reason: they are for a workspace that has no `armada.yml` yet. **No path
//! parameter**, for `editing`'s: a Fleet serves one checkout, and a proposal is
//! named by the workspace Scan found, in the body.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{EditManifestProposal, WriteManifestProposal};

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::served::Served;

/// One proposal per workspace, each line citing its source.
pub(crate) async fn get_manifest_proposals<D: Queries>(
    State(served): State<Served<D>>,
) -> Response {
    match served.daemon().get_manifest_proposals().await {
        Ok(proposals) => answer(StatusCode::OK, &proposals, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One edit, answered with the proposal after it.
pub(crate) async fn edit_manifest_proposal<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let edit: EditManifestProposal = match ipc::decode("an edit to a proposal", &body) {
        Ok(edit) => edit,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().edit_manifest_proposal(edit).await {
        Ok(proposal) => answer(StatusCode::OK, &proposal, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The file, from the proposal as it stands. **200 and not 201**: the answer
/// is the proposal, now carrying `written`, not a new resource to fetch.
pub(crate) async fn write_manifest_proposal<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let write: WriteManifestProposal = match ipc::decode("a proposal to write", &body) {
        Ok(write) => write,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().write_manifest_proposal(write).await {
        Ok(proposal) => answer(StatusCode::OK, &proposal, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
