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

/// A file, answered as itself.
///
/// **The one route on this seam that does not answer JSON**, and it is a frame:
/// an image cannot be windowed the way a Check's output can — a truncated PNG
/// is not a shorter PNG, it is a file nothing can draw — so there is no partial
/// reading to describe and nothing for a JSON envelope to carry but the bytes
/// under a base64 that inflates them by a third.
///
/// **The type is read off the file's own name and never carried on the record.**
/// A media type stored beside a path is a second place the same fact has to be
/// kept true; the extension is already in both. Anything unrecognised is
/// `application/octet-stream`, which is the honest answer and the one that
/// makes a browser refuse to render rather than guess.
///
/// **`nosniff`, because a caller may not have chosen what it is looking at.**
/// The bytes came from a file a repository's own harness wrote, and content
/// sniffing is how a file that is not an image comes to be treated as
/// something executable.
pub(crate) fn file(name: &str, bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, media_type(name)),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        bytes,
    )
        .into_response()
}

/// What a file's own name says it is.
///
/// **A short list and a wide default.** These are the formats `#605` named a
/// Job screen must draw — a screenshot, a harness's own text, a structured
/// result, a recording — and everything else is bytes, which is what the
/// caller is told. Growing this list is a decision about what Armada will
/// render, and it is made here rather than by a crate that knows every
/// extension there is.
fn media_type(name: &str) -> &'static str {
    match name.rsplit('.').next().map(str::to_ascii_lowercase) {
        Some(ext) if ext == "png" => "image/png",
        Some(ext) if ext == "jpg" || ext == "jpeg" => "image/jpeg",
        Some(ext) if ext == "webp" => "image/webp",
        Some(ext) if ext == "gif" => "image/gif",
        Some(ext) if ext == "txt" => "text/plain",
        Some(ext) if ext == "json" => "application/json",
        Some(ext) if ext == "webm" => "video/webm",
        Some(ext) if ext == "mp4" => "video/mp4",
        // **Served, and never rendered from a type this chose.** An SVG or an
        // HTML file is a document a browser executes, so each is answered as
        // bytes rather than as the image or the markup it is — a harness that
        // writes one has still written a frame, and what draws it (or that
        // nothing here will) is a decision for the surface rather than for a
        // header set here.
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::media_type;

    #[test]
    fn every_declared_kind_answers_its_own_media_type() {
        assert_eq!(media_type("home.png"), "image/png");
        assert_eq!(
            media_type("home.PNG"),
            "image/png",
            "the extension, cased either way"
        );
        assert_eq!(media_type("home.jpg"), "image/jpeg");
        assert_eq!(media_type("home.jpeg"), "image/jpeg");
        assert_eq!(media_type("home.webp"), "image/webp");
        assert_eq!(media_type("home.gif"), "image/gif");
        assert_eq!(media_type("output.txt"), "text/plain");
        assert_eq!(media_type("result.json"), "application/json");
        assert_eq!(media_type("session.webm"), "video/webm");
        assert_eq!(media_type("session.mp4"), "video/mp4");
    }

    /// **SVG and HTML are documents a browser executes, and stay bytes.** The
    /// module doc says why: sniffed and rendered from a type chosen here, either
    /// one would run as a document rather than draw as the frame it is.
    #[test]
    fn svg_and_html_are_still_served_as_bytes() {
        assert_eq!(media_type("home.svg"), "application/octet-stream");
        assert_eq!(media_type("home.html"), "application/octet-stream");
    }

    #[test]
    fn a_name_with_no_extension_or_an_unknown_one_is_bytes() {
        assert_eq!(media_type("README"), "application/octet-stream");
        assert_eq!(media_type("archive.tar.gz"), "application/octet-stream");
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
