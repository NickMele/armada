//! A Studio's routes. `#1285`.
//!
//! **The collection reads `?manifest_id=`; a member reads its id off the path.**
//! A door call reaching a member carries the session's scope as an extension
//! that bytes on the wire cannot set, and the daemon refuses a Studio of
//! another repository against it.

use axum::body::Bytes;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{ManifestId, StudioId};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Redirector, Refusal, Studios};
use crate::door::{HelmCalled, Scoped};
use crate::scoped::InManifest;
use crate::served::Served;

type Scope = Option<Extension<Scoped>>;

fn within(scope: Scope) -> Option<ManifestId> {
    scope.map(|Extension(scoped)| scoped.manifest_id())
}

fn answered<T: Serialize, D: Studios>(
    served: &Served<D>,
    status: StatusCode,
    result: Result<T, Refusal>,
) -> Response {
    match result {
        Ok(value) => answer(status, &value, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The body, or the 400 that says why it is not one.
fn body<T: DeserializeOwned, D: Studios>(
    served: &Served<D>,
    what: &'static str,
    bytes: &Bytes,
) -> Result<T, Response> {
    ipc::decode(what, bytes).map_err(|why| undecodable(&why.to_string(), served.run_id()))
}

pub(crate) async fn list_studios<D: Studios>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    let listed = served.daemon().list_studios(scope.manifest()).await;
    answered(&served, StatusCode::OK, listed)
}

pub(crate) async fn get_studio<D: Studios>(
    State(served): State<Served<D>>,
    Path(studio_id): Path<String>,
    scope: Scope,
) -> Response {
    let studio = served
        .daemon()
        .get_studio(StudioId::carried(studio_id), within(scope))
        .await;
    answered(&served, StatusCode::OK, studio)
}

/// 201: the Studio exists now.
pub(crate) async fn create_studio<D: Studios>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    bytes: Bytes,
) -> Response {
    let create = match body(&served, "a Studio to create", &bytes) {
        Ok(create) => create,
        Err(response) => return response,
    };
    let created = served
        .daemon()
        .create_studio(create, scope.manifest())
        .await;
    answered(&served, StatusCode::CREATED, created)
}

pub(crate) async fn rename_studio<D: Studios>(
    State(served): State<Served<D>>,
    Path(studio_id): Path<String>,
    scope: Scope,
    bytes: Bytes,
) -> Response {
    let rename = match body(&served, "a Studio's name", &bytes) {
        Ok(rename) => rename,
        Err(response) => return response,
    };
    let renamed = served
        .daemon()
        .rename_studio(StudioId::carried(studio_id), rename, within(scope))
        .await;
    answered(&served, StatusCode::OK, renamed)
}

/// **Reads no body**, as `forget_job` reads none: the id is the whole request.
pub(crate) async fn delete_studio<D: Studios>(
    State(served): State<Served<D>>,
    Path(studio_id): Path<String>,
    scope: Scope,
) -> Response {
    let deleted = served
        .daemon()
        .delete_studio(StudioId::carried(studio_id), within(scope))
        .await;
    answered(&served, StatusCode::OK, deleted)
}

/// Who added it is the transport's word: a Helm session the door placed, or
/// a person.
pub(crate) async fn add_studio_node<D: Studios>(
    State(served): State<Served<D>>,
    Path(studio_id): Path<String>,
    scope: Scope,
    helm: Option<Extension<HelmCalled>>,
    bytes: Bytes,
) -> Response {
    let add = match body(&served, "a node to add", &bytes) {
        Ok(add) => add,
        Err(response) => return response,
    };
    let by = match helm {
        Some(_) => Redirector::Helm,
        None => Redirector::Person,
    };
    let added = served
        .daemon()
        .add_studio_node(StudioId::carried(studio_id), add, by, within(scope))
        .await;
    answered(&served, StatusCode::OK, added)
}

pub(crate) async fn move_studio_node<D: Studios>(
    State(served): State<Served<D>>,
    Path(studio_id): Path<String>,
    scope: Scope,
    bytes: Bytes,
) -> Response {
    let moving = match body(&served, "a node to move", &bytes) {
        Ok(moving) => moving,
        Err(response) => return response,
    };
    let moved = served
        .daemon()
        .move_studio_node(StudioId::carried(studio_id), moving, within(scope))
        .await;
    answered(&served, StatusCode::OK, moved)
}

pub(crate) async fn remove_studio_node<D: Studios>(
    State(served): State<Served<D>>,
    Path(studio_id): Path<String>,
    scope: Scope,
    bytes: Bytes,
) -> Response {
    let removing = match body(&served, "a node to remove", &bytes) {
        Ok(removing) => removing,
        Err(response) => return response,
    };
    let removed = served
        .daemon()
        .remove_studio_node(StudioId::carried(studio_id), removing, within(scope))
        .await;
    answered(&served, StatusCode::OK, removed)
}

pub(crate) async fn propose_studio_edge<D: Studios>(
    State(served): State<Served<D>>,
    Path(studio_id): Path<String>,
    scope: Scope,
    bytes: Bytes,
) -> Response {
    let proposal = match body(&served, "an edge to propose", &bytes) {
        Ok(proposal) => proposal,
        Err(response) => return response,
    };
    let proposed = served
        .daemon()
        .propose_studio_edge(StudioId::carried(studio_id), proposal, within(scope))
        .await;
    answered(&served, StatusCode::OK, proposed)
}

pub(crate) async fn decide_studio_edge<D: Studios>(
    State(served): State<Served<D>>,
    Path(studio_id): Path<String>,
    scope: Scope,
    bytes: Bytes,
) -> Response {
    let decision = match body(&served, "a decision on an edge", &bytes) {
        Ok(decision) => decision,
        Err(response) => return response,
    };
    let decided = served
        .daemon()
        .decide_studio_edge(StudioId::carried(studio_id), decision, within(scope))
        .await;
    answered(&served, StatusCode::OK, decided)
}
