//! Rules a person always-allowed for a Manifest's repository, kept by Fleet
//! itself rather than in `armada.yml` — `#836`.
//!
//! **No `Resolved` extractor and no path parameter**, for
//! [`crate::limiting`]'s reason: a Fleet serves one repository, and these
//! rules are its own rather than any one Job's.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::RemoveRepositoryAllowedCommand;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::served::Served;

/// Every rule a person always-allowed for this Manifest's repository.
pub(crate) async fn get_repository_allowed_commands<D: Queries>(
    State(served): State<Served<D>>,
) -> Response {
    match served.daemon().get_repository_allowed_commands().await {
        Ok(allowed) => answer(StatusCode::OK, &allowed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Take back a rule a person always-allowed for this Manifest's repository,
/// and answer with what remains.
pub(crate) async fn remove_repository_allowed_command<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let removing: RemoveRepositoryAllowedCommand = match ipc::decode("a rule to take back", &body) {
        Ok(removing) => removing,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .remove_repository_allowed_command(removing)
        .await
    {
        Ok(allowed) => answer(StatusCode::OK, &allowed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
