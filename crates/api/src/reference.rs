//! The `:job_id` segment, resolved before a handler can reach it.
//!
//! # An extractor, so no handler holds the unresolved text
//!
//! Every route under `/jobs/:job_id` used to build an `ipc::JobId` out of the
//! path segment, which meant the segment had to *be* a ULID — and the id
//! Bridge shows a person is the handle. Resolution could have been a line at
//! the top of thirty handlers; it is an extractor instead, because a line at
//! the top of thirty handlers is thirty places to forget it and one for the
//! next route to be written without.
//!
//! What a handler is given is [`Resolved`], which has no constructor taking a
//! string and no way back to what was typed. A handler cannot act on an
//! unresolved reference because it never has one.
//!
//! # It carries the handle as well as the id
//!
//! Not for the sake of it: the handle is what a Job's worktree, its branch and
//! every path under `.armada/` are named by, and the socket serving a Job's own
//! log needs it. One resolve answers both, where a handler deriving it would
//! read the row a second time for a fact the first read already had.

use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use axum::response::Response;
use ipc::{JobId, WireError};
use serde::Deserialize;

use crate::answers::{problem, refused};
use crate::daemon::Queries;
use crate::served::Served;

/// The Job a request named, resolved.
///
/// **Minted by the daemon and by nothing else.** There is no constructor here
/// that takes a path segment, which is what makes "a route's `:job_id` is
/// resolved before it is used" a property of the type rather than of a line
/// somebody remembered to write in each handler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    job_id: JobId,
    handle: String,
}

impl Resolved {
    /// What the daemon found. Called by the implementation of
    /// [`Queries::resolve_job`] and by this crate's own fake.
    pub fn of(job_id: JobId, handle: impl Into<String>) -> Resolved {
        Resolved {
            job_id,
            handle: handle.into(),
        }
    }

    /// The record's key, which every query joins on.
    pub fn id(&self) -> JobId {
        self.job_id.clone()
    }

    /// What this Job is called where a person reads it, and what every path
    /// under `.armada/` for it is named by.
    pub fn handle(&self) -> &str {
        &self.handle
    }
}

/// A path with a `:job_id` in it.
///
/// **Named rather than positional**, so a route carrying a second segment —
/// `/jobs/:job_id/calls/:call_id` — extracts this and its own without either
/// having to know the other is there.
#[derive(Deserialize)]
struct Named {
    job_id: String,
}

/// The segment is read, handed to the daemon, and the request stops here if
/// nothing answers to it.
///
/// **The rejection is the daemon's own refusal**, so a handle that names no
/// Job is the 404 a ULID that names no Job has always been, with the text the
/// caller typed quoted back in it.
#[axum::async_trait]
impl<D: Queries> FromRequestParts<Served<D>> for Resolved {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        served: &Served<D>,
    ) -> Result<Resolved, Response> {
        let Path(named) = Path::<Named>::from_request_parts(parts, served)
            .await
            .map_err(|why| {
                // Unreachable from a routed request: this extractor is only
                // named by handlers whose path has the segment. Answered rather
                // than panicked, because a panic here drops the connection.
                problem(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &WireError::raised(NO_JOB_SEGMENT, &why.to_string(), served.run_id().clone()),
                )
            })?;
        served
            .daemon()
            .resolve_job(named.job_id)
            .await
            .map_err(refused)
    }
}

/// A handler asked for the Job on a route whose path names none.
pub(crate) const NO_JOB_SEGMENT: &str = "api.no_job_segment";
