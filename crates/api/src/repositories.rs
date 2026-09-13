//! The repositories a Fleet serves, and adding one by folder.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::AddRepository;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::scoped::InManifest;
use crate::served::Served;

pub(crate) async fn list_repositories<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_repositories().await {
        Ok(listed) => answer(StatusCode::OK, &listed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 201: the repository is served when this answers.
pub(crate) async fn add_repository<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: AddRepository = match ipc::decode("a folder to serve", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().add_repository(asked).await {
        Ok(added) => answer(StatusCode::CREATED, &added, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What one repository's catalogue left out of its workflows, each with why — #425.
pub(crate) async fn list_left_out_workflows<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served
        .daemon()
        .list_left_out_workflows(scope.manifest())
        .await
    {
        Ok(left_out) => answer(StatusCode::OK, &left_out, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
