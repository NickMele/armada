//! What a handler answers with, whichever handler it is.
//!
//! Four functions and two error codes, in one place because every route uses
//! them and none of them owns them. They were spelled out at each handler
//! until the eighth copy of the same four lines had to agree with the other
//! seven.
//!
//! **The codes are declared beside the thing that raises them**, which is what
//! makes the set closed by collection rather than by a registry somebody has to
//! keep in step.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use ipc::{RunId, WireError};
use serde::Serialize;

use crate::daemon::Refusal;

/// A request body that would not parse.
pub(crate) const UNDECODABLE_REQUEST: &str = "api.undecodable_request";

/// A response that would not serialise. Unreachable for plain data, and
/// answered rather than panicked: a panic here drops a socket mid-Job.
pub(crate) const UNENCODABLE_RESPONSE: &str = "api.unencodable_response";

pub(crate) fn answer<T: Serialize>(status: StatusCode, value: &T, run_id: &RunId) -> Response {
    match ipc::encode(value) {
        Ok(body) => (status, [(header::CONTENT_TYPE, "application/json")], body).into_response(),
        Err(why) => problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            &WireError::raised(UNENCODABLE_RESPONSE, why.to_string(), run_id.clone())
                .caused_by(vec![why.to_string()]),
        ),
    }
}

pub(crate) fn refused(refusal: Refusal) -> Response {
    let status =
        StatusCode::from_u16(refusal.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    problem(status, refusal.error())
}

pub(crate) fn problem(status: StatusCode, error: &WireError) -> Response {
    match ipc::encode(error) {
        Ok(body) => (status, [(header::CONTENT_TYPE, "application/json")], body).into_response(),
        // The error would not serialise, which leaves nothing true to say with
        // a body. The status is still the answer.
        Err(_) => status.into_response(),
    }
}

/// A body that would not parse, as the 400 it is. **Written once**: this arm
/// was spelled out at seven commands, each four lines of `WireError` that had to
/// agree with the other six, and the eighth is what made it a function.
pub(crate) fn undecodable(why: &str, run_id: &RunId) -> Response {
    problem(
        StatusCode::BAD_REQUEST,
        &WireError::raised(UNDECODABLE_REQUEST, why.to_string(), run_id.clone())
            .caused_by(vec![why.to_string()]),
    )
}
