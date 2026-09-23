//! Who this listener answers, decided before a handler sees the request.
//!
//! **Fleet answers no page in a browser.** The port is on loopback, which is a
//! boundary around the machine and not around the machine's browsers: a page
//! in any tab can open a loopback port, and a WebSocket upgrade is not subject
//! to the same-origin policy at all — the browser sends `Origin` and leaves
//! the server to decide. Nothing decided, so the event stream was readable by
//! any page that found the port and the routes that change state were
//! reachable by one (`#1460`).
//!
//! **Presence is the refusal, and an allowlist would be the wrong shape.** A
//! browser sets `Origin` and a page cannot forge it; anything that is not a
//! browser sends whatever it likes, so naming trusted values buys nothing.
//! Absence stays allowed, and absence is what every caller Fleet has sends.

use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use ipc::{RunId, WireError, WireValue};

use crate::answers::problem;

/// A request or an upgrade that carried an `Origin` header, which is a page in
/// a browser saying it is one.
///
/// **Not a caller that got its credentials wrong.** There are no credentials
/// on this seam; what this names is a kind of caller Fleet serves nothing to,
/// so the fix is never to retry with something added. `origin` on the error
/// carries the page's own address, which is the one fact that says which tab.
pub(crate) const FROM_A_PAGE: &str = "api.from_a_page";

/// The one doorman, over the whole listener.
///
/// **A layer and not a check in each handler**, which is the difference
/// between a property of the seam and a convention every new route has to
/// remember. The HTTP half and the upgrade half are the same decision here:
/// an upgrade is an ordinary request until the handler says otherwise, so a
/// page is refused before [`axum::extract::ws::WebSocketUpgrade`] is reached
/// and no socket opens.
pub(crate) async fn refuse_a_page(
    State(run_id): State<RunId>,
    request: Request,
    next: Next,
) -> Response {
    let Some(origin) = request.headers().get(header::ORIGIN) else {
        return next.run(request).await;
    };
    let said = origin.to_str().unwrap_or("bytes that are not text");
    problem(
        StatusCode::FORBIDDEN,
        &WireError::raised(
            FROM_A_PAGE,
            "this request carried an Origin header, so it came from a page in a browser, and \
             Fleet answers no page. Bridge's main process, the armada CLI, an agent's relay and \
             a Drone all reach Fleet without one",
            run_id,
        )
        .with_field("origin", WireValue::Str(said.to_string())),
    )
}
