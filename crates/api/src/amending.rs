//! The form's route into `armada.yml`: edits, not text — Journey 9,
//! *Editing*, and `#721`.
//!
//! **Beside [`crate::editing`] and not inside it.** That page is the file as
//! bytes, read and saved whole; this one is a form saying which keys it
//! changed. No `Resolved` extractor and no path parameter, for that page's
//! reason: a Fleet resolves its own `armada.yml`.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::EditManifest;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Commands;
use crate::served::Served;

/// Apply a form's edits and write what they left.
///
/// **200 and not 202**, for `save_manifest_file`'s reason: the bytes are on
/// disk when this answers, and the text that comes back is what is there.
pub(crate) async fn edit_manifest<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let edit: EditManifest = match ipc::decode("a Manifest edit", &body) {
        Ok(edit) => edit,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().edit_manifest(edit).await {
        Ok(edited) => answer(StatusCode::OK, &edited, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
