//! The Manifest file's two routes: Journey 9, *Editing*.
//!
//! **One subject on one page**, the read and the write together, for
//! [`crate::rehearsing`]'s reason: a reader of one needs the other.
//!
//! **No `Resolved` extractor and no path parameter on either.** There is no Job
//! to resolve and no file to name — these sit under `/manifest` beside
//! `get_manifest_reading` and for its reason, that a Fleet serves one
//! repository and resolves its own `armada.yml`.

use axum::body::Bytes;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::SaveManifestFile;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::scoped::InManifest;
use crate::served::Served;

/// `armada.yml` as it is on disk, whole and unparsed.
pub(crate) async fn get_manifest_file<D: Queries>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
) -> Response {
    match served.daemon().get_manifest_file(scope.manifest()).await {
        Ok(file) => answer(StatusCode::OK, &file, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Write a corrected Manifest, and stop there.
///
/// **200 and not 202.** The bytes are on disk when this answers; what is still
/// to come is Fleet's own re-read, which is `manifest.reread` and not this
/// route's to promise.
pub(crate) async fn save_manifest_file<D: Commands>(
    State(served): State<Served<D>>,
    Query(scope): Query<InManifest>,
    body: Bytes,
) -> Response {
    let save: SaveManifestFile = match ipc::decode("a Manifest to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .save_manifest_file(save, scope.manifest())
        .await
    {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
