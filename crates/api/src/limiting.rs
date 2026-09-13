//! Fleet's three changeable limits: the read and the save, on one page for
//! [`crate::editing`]'s reason — a reader of one needs the other.
//!
//! **No `Resolved` extractor and no path parameter.** The limits are Fleet's
//! own, beside `/capacity` and for its reason: nothing about them is a Job's.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::SaveLimits;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::served::Served;

/// The limits in force, and what shipped.
pub(crate) async fn get_limits<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_limits().await {
        Ok(limits) => answer(StatusCode::OK, &limits, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Save any of the three, and answer with what is now in force.
///
/// **A value out of range is the 400 an undecodable body gets**, and the daemon
/// is never asked: `ipc::SaveLimits` cannot hold one, so nothing is saved.
pub(crate) async fn save_limits<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let save: SaveLimits = match ipc::decode("limits to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_limits(save).await {
        Ok(limits) => answer(StatusCode::OK, &limits, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
