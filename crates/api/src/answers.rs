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

use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use ipc::{RunId, WireError};
use serde::Serialize;

use crate::daemon::{FrameSpan, Refusal};

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
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(media_type(name)),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    // **A whole answer still says a span may be asked for.** A player reads
    // this before it decides whether it can seek at all, so a recording that
    // happened to be answered whole would otherwise be one nobody can scrub.
    if streams(name) {
        headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    }
    (StatusCode::OK, headers, bytes).into_response()
}

/// Whether a frame is answered a span at a time.
///
/// **Video, and nothing else.** A recording is the one kind a person watches
/// rather than reads, and the only one whose point is starting before the end
/// of it has arrived. Everything else here is a file small enough to answer
/// whole — and an SVG or an HTML file stays bytes for the reason
/// [`media_type`] gives, which streaming does not change.
pub(crate) fn streams(name: &str) -> bool {
    media_type(name).starts_with("video/")
}

/// One span of a file, as the 206 it is.
///
/// **What came back, not what was asked for.** A read windows a long span
/// rather than honouring it, so the header is composed from the bytes in hand
/// — the one arithmetic a player checks, and the one a caller must not restate
/// from its own request.
pub(crate) fn file_span(name: &str, first: u64, bytes: Vec<u8>, total: u64) -> Response {
    let last = first + (bytes.len() as u64).max(1) - 1;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(media_type(name)),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(header::CONTENT_RANGE, said(first, last, total));
    (StatusCode::PARTIAL_CONTENT, headers, bytes).into_response()
}

/// A span beginning at or past the end of the file, as the 416 it is.
///
/// **The length, and no body.** What reads this is a player asking again with
/// a span it can satisfy, never a person — so there is nothing a `WireError`
/// could tell anybody, and the one fact that helps is on the header the status
/// is defined by.
pub(crate) fn file_beyond(total: u64) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(
        header::CONTENT_RANGE,
        HeaderValue::try_from(format!("bytes */{total}")).expect("digits and a slash"),
    );
    (StatusCode::RANGE_NOT_SATISFIABLE, headers).into_response()
}

fn said(first: u64, last: u64, total: u64) -> HeaderValue {
    HeaderValue::try_from(format!("bytes {first}-{last}/{total}")).expect("digits and a slash")
}

/// What a `Range` header asked for, where it asked for something this serves.
///
/// **One span or nothing.** `bytes=a-b`, `bytes=a-` and `bytes=-n` are what a
/// media element sends. A header naming several spans, a unit that is not
/// `bytes`, or anything that will not parse answers `None` — which a caller
/// reads as *answer it whole*, exactly as a request carrying no header at all.
/// Ignoring a range is always legal; guessing at one is not.
pub(crate) fn asked_for(header: Option<&str>) -> Option<FrameSpan> {
    let spans = header?.trim().strip_prefix("bytes=")?;
    if spans.contains(',') {
        return None;
    }
    let (first, last) = spans.split_once('-')?;
    let (first, last) = (first.trim(), last.trim());
    if first.is_empty() {
        // `bytes=-0` asks for the last nothing, which is not a span.
        return last
            .parse()
            .ok()
            .filter(|back| *back > 0)
            .map(FrameSpan::Last);
    }
    let first: u64 = first.parse().ok()?;
    if last.is_empty() {
        return Some(FrameSpan::From { first, last: None });
    }
    let last: u64 = last.parse().ok()?;
    (last >= first).then_some(FrameSpan::From {
        first,
        last: Some(last),
    })
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
        Some(ext) if ext == "txt" || ext == "log" => "text/plain",
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
    use crate::daemon::FrameSpan;

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
        assert_eq!(media_type("run.log"), "text/plain");
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

    /// **Only a recording is asked for a span at a time.** Everything else is
    /// answered whole, which is what keeps a file a browser executes on the
    /// one path that has always been bytes.
    #[test]
    fn video_is_the_only_kind_a_span_is_served_of() {
        assert!(super::streams("walkthrough.webm"));
        assert!(super::streams("walkthrough.mp4"));
        assert!(!super::streams("home.png"));
        assert!(!super::streams("home.svg"));
        assert!(!super::streams("run.log"));
    }

    /// The three spellings a media element sends, and what each resolves to.
    #[test]
    fn the_spans_a_player_asks_for_are_read() {
        let read = |said: &str| match super::asked_for(Some(said)) {
            Some(FrameSpan::From { first, last }) => format!("{first}-{last:?}"),
            Some(FrameSpan::Last(back)) => format!("last {back}"),
            None => "whole".to_string(),
        };
        assert_eq!(read("bytes=0-"), "0-None", "opening a recording");
        assert_eq!(read("bytes=1024-2047"), "1024-Some(2047)", "a seek");
        assert_eq!(read("bytes=-512"), "last 512", "a container's index");
    }

    /// **Anything this cannot read answers whole**, which is what a request
    /// with no header at all answers. Ignoring a range is legal; guessing at
    /// one would serve bytes nobody asked for under a status saying they did.
    #[test]
    fn a_range_this_does_not_serve_is_answered_whole() {
        assert!(super::asked_for(None).is_none(), "no header");
        assert!(
            super::asked_for(Some("bytes=0-9, 20-29")).is_none(),
            "two spans"
        );
        assert!(super::asked_for(Some("items=0-9")).is_none(), "not bytes");
        assert!(super::asked_for(Some("bytes=9-4")).is_none(), "backwards");
        assert!(
            super::asked_for(Some("bytes=-0")).is_none(),
            "the last nothing"
        );
        assert!(
            super::asked_for(Some("bytes=abc-")).is_none(),
            "not a number"
        );
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
