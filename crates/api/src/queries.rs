//! Reads. Every one is request-response, and none of them moves a Job.
//!
//! **Split from the route table rather than from a line count.** `routes` is
//! the inventory and the router — what exists and where it is served — and this
//! is what each read answers with. The seam is a real one: a query never
//! decodes a body, never returns 201, and never has a refusal that means the
//! machine would not admit the move.
//!
//! The expensive reads are here on purpose and each says why in its own doc.
//! `get_diff` spends the patch, `get_call` spends one argument, and both are
//! separate routes so that the reads made on every refresh do not pay for them.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::JobId;

use crate::answers::{answer, refused};
use crate::daemon::Daemon;
use crate::routes::Served;

pub(crate) async fn list_jobs<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_jobs().await {
        Ok(jobs) => answer(StatusCode::OK, &jobs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The bound, what is occupying it, and what holds the next Drone back.
///
/// **Its own route rather than a field on `/jobs`.** That read is a list of
/// Jobs and is made on every Board refresh; this is three values about Fleet,
/// asked for by the surface that draws Fleet's state.
pub(crate) async fn get_capacity<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_capacity().await {
        Ok(capacity) => answer(StatusCode::OK, &capacity, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One Job in full. **The Board row plus what the list redacts** — the steps
/// and where each got to, the criteria, the branch, the brief.
pub(crate) async fn get_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon().get_job(JobId::carried(job_id)).await {
        Ok(detail) => answer(StatusCode::OK, &detail, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Every move one Job made, oldest first. **The path taken**, which `get_job`
/// answers nothing about — it says where a Job is, and this says how it got
/// there.
///
/// Its own route because a history has no bound and a detail view is fetched to
/// draw a summary. Nothing here folds: the rows are read and rendered, and
/// `crates/store/src/fold.rs` stays the only thing that replays them.
pub(crate) async fn get_job_events<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon().get_job_events(JobId::carried(job_id)).await {
        Ok(history) => answer(StatusCode::OK, &history, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Every claim a Job's Drones have submitted. **What the work says about
/// itself**, which the gate ruled on and a person reads before deciding.
///
/// Its own route beside the diff rather than folded into it: this is a few
/// sentences per step and that is however large the work is.
pub(crate) async fn get_evidence<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon().get_evidence(JobId::carried(job_id)).await {
        Ok(evidence) => answer(StatusCode::OK, &evidence, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One Job's whole patch. **The expensive read, on the one route that asks for
/// it** — `get_job` is fetched on every open to draw a summary, and the bytes
/// are what a person reading a diff needs and nothing else does.
pub(crate) async fn get_diff<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon().get_diff(JobId::carried(job_id)).await {
        Ok(diff) => answer(StatusCode::OK, &diff, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One tool call's arguments, whole. **The rest of a row that was cut**, on a
/// route rather than in the row itself.
///
/// A `called` row carries a line and how many characters the argument had, so a
/// person opening it reads *showing 200 of 14,320 characters* — and this is
/// where the other fourteen thousand come from. The split is `get_diff`'s: the
/// cheap fact streams to everyone watching, and the bytes are fetched once, by
/// the one person who asked.
///
/// **It is not the socket's job.** `observe_job` is bounded and lossy under
/// backpressure by design, and a row large enough to evict its neighbours would
/// take the short form down with it.
///
/// 404 where the Job is unknown. 422 where the Job is known and nothing in its
/// transcripts carries that call id — the request is well-formed and the value
/// in it names nothing, which is a different thing from the Job not existing.
pub(crate) async fn get_call<D: Daemon>(
    State(served): State<Served<D>>,
    Path((job_id, call_id)): Path<(String, String)>,
) -> Response {
    match served
        .daemon()
        .get_call(JobId::carried(job_id), call_id)
        .await
    {
        Ok(call) => answer(StatusCode::OK, &call, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The workflows a proposal may name. **The set Fleet will accept**, which is
/// why it is served rather than left to a caller to know.
pub(crate) async fn list_workflows<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_workflows().await {
        Ok(workflows) => answer(StatusCode::OK, &workflows, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_manifests<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_manifests().await {
        Ok(manifests) => answer(StatusCode::OK, &manifests, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_models<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_models().await {
        Ok(models) => answer(StatusCode::OK, &models, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Every report filed, newest first, with the counts they are read beside.
///
/// **Not under `/jobs`**, and that is the shape of the claim: a report survives
/// `armada clean` taking its Job away, so it is a record of its own rather than
/// a row beneath one.
pub(crate) async fn list_reports<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_reports().await {
        Ok(reports) => answer(StatusCode::OK, &reports, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Every worktree Fleet is holding disk for, and the test each one did not
/// pass. **Read across every Job at once**, which is the reading a field on
/// `get_job` could not give: what is being decided is which of these to give
/// back, and that is a question about the set.
///
/// A piloted Job's worktree is not in the answer. Fleet drops it — `#367` — so
/// there is nothing here to filter and nothing a client could show by mistake.
pub(crate) async fn list_worktrees<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_worktrees().await {
        Ok(held) => answer(StatusCode::OK, &held, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
