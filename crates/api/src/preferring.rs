//! A person's Bridge preferences: the read and the save, `limiting`'s shape
//! one table over.
//!
//! **No `Resolved` extractor and no path parameter.** The preferences are
//! Fleet's own, beside `/limits` and for its reason: nothing about them is a
//! Job's.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::SavePreference;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::served::Served;

/// Every preference, and what is in force for each.
pub(crate) async fn get_preferences<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_preferences().await {
        Ok(preferences) => answer(StatusCode::OK, &preferences, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Save one preference by name, and answer with every preference now in
/// force.
///
/// **A name outside the closed set is [`crate::daemon::Refusal::Unacceptable`]**,
/// not undecodable — `SavePreference.name` is a plain string, so the body
/// always decodes and the daemon is what refuses it, by name.
pub(crate) async fn save_preferences<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let save: SavePreference = match ipc::decode("a preference to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_preferences(save).await {
        Ok(preferences) => answer(StatusCode::OK, &preferences, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
