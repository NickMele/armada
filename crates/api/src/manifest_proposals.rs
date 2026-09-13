//! Setup's proposal routes, under `/repository` beside Scan: there is no `armada.yml` yet
//! for them to sit under `/manifest`.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::EditManifestProposal;

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
