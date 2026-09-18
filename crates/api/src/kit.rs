//! Kit's MCP servers, and each Manifest's word over one — `#1275`.
//!
//! **Every act answers with the whole list**, the way
//! [`crate::repository_allow`] does: a caller folding one answer has already
//! folded the other, and nothing has to ask again to draw what changed.
//!
//! The Manifest is a query parameter rather than a path segment, for that
//! module's reason: a Fleet serves one repository, and this scopes the second
//! tier rather than naming a resource.

use axum::body::Bytes;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{AddKitServer, ForgetKitServer, SetKitServerReach, SetManifestServerReach};

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::scoped::InManifest;
use crate::served::Served;

/// The setup a person already works with, read from their harness's own home —
/// `#1491`.
///
/// **Machine-wide**, so no Manifest scopes it: what somebody has is theirs and
/// not a repository's.
pub(crate) async fn get_kit_inventory<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_kit_inventory().await {
        Ok(inventory) => answer(StatusCode::OK, &inventory, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Every server in Kit, and what a Drone dispatched against this Manifest
/// resolves.
pub(crate) async fn get_kit_servers<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().get_kit_servers(scope.manifest()).await {
        Ok(servers) => answer(StatusCode::OK, &servers, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn add_kit_server<D: Commands>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let adding: AddKitServer = match ipc::decode("a server to add to Kit", &body) {
        Ok(adding) => adding,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .add_kit_server(adding, scope.manifest())
        .await
    {
        Ok(servers) => answer(StatusCode::OK, &servers, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn forget_kit_server<D: Commands>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let forgetting: ForgetKitServer = match ipc::decode("a server to take out of Kit", &body) {
        Ok(forgetting) => forgetting,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .forget_kit_server(forgetting, scope.manifest())
        .await
    {
        Ok(servers) => answer(StatusCode::OK, &servers, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn set_kit_server_reach<D: Commands>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let setting: SetKitServerReach = match ipc::decode("Kit's own reach for a server", &body) {
        Ok(setting) => setting,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .set_kit_server_reach(setting, scope.manifest())
        .await
    {
        Ok(servers) => answer(StatusCode::OK, &servers, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn set_manifest_server_reach<D: Commands>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let setting: SetManifestServerReach =
        match ipc::decode("this Manifest's word about a server", &body) {
            Ok(setting) => setting,
            Err(why) => return undecodable(&why.to_string(), served.run_id()),
        };
    match served
        .daemon()
        .set_manifest_server_reach(setting, scope.manifest())
        .await
    {
        Ok(servers) => answer(StatusCode::OK, &servers, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
