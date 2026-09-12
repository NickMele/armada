//! The four reads that narrow the Board rather than drawing it.
//!
//! **Each is `list_jobs` with a question on it.** A caller could filter the
//! whole list itself; the point of four routes is that the rule which decides
//! *what is waiting on you* lives in one place instead of in each surface.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused};
use crate::daemon::Queries;
use crate::served::Served;

/// The open queue: Jobs no Drone has been started on.
pub(crate) async fn list_job_board<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_job_board().await {
        Ok(jobs) => answer(StatusCode::OK, &jobs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Jobs resting at a gate somebody has to answer.
pub(crate) async fn list_reviews<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_reviews().await {
        Ok(jobs) => answer(StatusCode::OK, &jobs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Jobs that are over, newest first.
pub(crate) async fn get_activity_feed<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_activity_feed().await {
        Ok(jobs) => answer(StatusCode::OK, &jobs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What is waiting on a person, split by what the waiting costs.
pub(crate) async fn list_alerts<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_alerts().await {
        Ok(alerts) => answer(StatusCode::OK, &alerts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
