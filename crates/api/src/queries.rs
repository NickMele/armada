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

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, file, refused};
use crate::daemon::Queries;
use crate::reference::Resolved;
use crate::served::Served;

/// The second segment of `/jobs/:job_id/calls/:call_id`.
///
/// **Its own struct rather than a tuple**, because the Job is extracted beside
/// it and a tuple would make each of the two handlers state a segment it does
/// not use.
#[derive(Deserialize)]
pub(crate) struct Called {
    call_id: String,
}

/// The second segment of `/jobs/:job_id/checks/:kept/output`. See [`Called`].
#[derive(Deserialize)]
pub(crate) struct Kept {
    kept: String,
}

/// The two segments after the Job on `get_frame`, named rather than
/// positional. **A tuple `Path` would take the Job's segment too**, and the
/// Job's segment is `Resolved`'s — which is the whole reason this struct
/// exists where `Kept`'s one field did not need it.
#[derive(Deserialize)]
pub(crate) struct Framed {
    run: String,
    name: String,
}

pub(crate) async fn list_jobs<D: Queries>(State(served): State<Served<D>>) -> Response {
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
pub(crate) async fn get_capacity<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_capacity().await {
        Ok(capacity) => answer(StatusCode::OK, &capacity, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What Fleet's last re-read of `armada.yml` came to.
///
/// **200 with `null` where Fleet has not re-read it**, rather than a 404. A
/// Fleet still running on the Manifest it booted with is not a missing
/// resource; it is a Fleet with nothing to report, and a client draws nothing
/// either way.
pub(crate) async fn get_manifest_reading<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_manifest_reading().await {
        Ok(reading) => answer(StatusCode::OK, &reading, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The `?q=` a person has typed after `@`. A query string rather than a path
/// segment — `q` is empty the instant somebody types `@` and before anything
/// follows it, which a path segment cannot carry. Absent decodes the same as
/// empty: a caller that sends no parameter at all gets everything, up to the
/// cap.
#[derive(Deserialize)]
pub(crate) struct Search {
    #[serde(default)]
    q: String,
}

/// Paths under the checkout narrowed against typed text, for Bridge's `@`
/// mention popup. It never refuses — see `Queries::search_files`.
pub(crate) async fn search_files<D: Queries>(
    State(served): State<Served<D>>,
    Query(Search { q }): Query<Search>,
) -> Response {
    match served.daemon().search_files(q).await {
        Ok(found) => answer(StatusCode::OK, &found, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One Job in full. **The Board row plus what the list redacts** — the steps
/// and where each got to, the criteria, the branch, the brief.
pub(crate) async fn get_job<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().get_job(job.id()).await {
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
pub(crate) async fn get_job_events<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().get_job_events(job.id()).await {
        Ok(history) => answer(StatusCode::OK, &history, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Every claim a Job's Drones have submitted. **What the work says about
/// itself**, which the gate ruled on and a person reads before deciding.
///
/// Its own route beside the diff rather than folded into it: this is a few
/// sentences per step and that is however large the work is.
pub(crate) async fn get_evidence<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().get_evidence(job.id()).await {
        Ok(evidence) => answer(StatusCode::OK, &evidence, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What anybody has written on one Job's open pull request.
///
/// **The one query on this router that talks to a forge**, which is why it is a
/// route somebody asks for rather than a field on `get_job`. A forge that would
/// not answer is a 500 and never an empty list.
pub(crate) async fn get_remarks<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().get_remarks(job.id()).await {
        Ok(remarks) => answer(StatusCode::OK, &remarks, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One Job's whole patch. **The expensive read, on the one route that asks for
/// it** — `get_job` is fetched on every open to draw a summary, and the bytes
/// are what a person reading a diff needs and nothing else does.
pub(crate) async fn get_diff<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().get_diff(job.id()).await {
        Ok(diff) => answer(StatusCode::OK, &diff, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What this Job holds on this machine — its processes, what each is burning,
/// the disk its worktree has taken, and when its own log was last written.
///
/// **Its own route rather than a field on `get_job`.** That read is made on
/// every open of a Job and again on every event naming it; this one walks a
/// process table and a directory, and paying for that continuously to answer a
/// question asked rarely is the cost the issue this serves rules out.
///
/// It answers rather than refuses on a reading it could not take. The figures
/// carry the instant they were read at, because a process can exit between the
/// sample and the render.
pub(crate) async fn get_job_resources<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().get_job_resources(job.id()).await {
        Ok(resources) => answer(StatusCode::OK, &resources, served.run_id()),
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
pub(crate) async fn get_call<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
    Path(Called { call_id }): Path<Called>,
) -> Response {
    match served.daemon().get_call(job.id(), call_id).await {
        Ok(call) => answer(StatusCode::OK, &call, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One Check's own output, read back into the app. **The other end of a path
/// that opened nothing in Armada.**
///
/// `CheckRun::output_path` has said where a Check wrote its stdout and stderr
/// since the record kept them, and the only thing Bridge could do with it was
/// hand it to the operating system. This is what a person who pressed the row
/// asks for, once, so the reading happens on the screen the Check is on.
///
/// **It is not the socket's job**, for the reason `get_call` is not: `/events`
/// is one drop-oldest channel carrying every Job, and a test runner's output on
/// it would evict the state changes the Board is drawn from.
///
/// 404 where the Job is unknown. 422 where the Job is known and no row of it
/// kept an output under that name — a reclaimed `.armada`, or an id that was
/// never one. The request is well-formed and the value in it names nothing,
/// which is a different thing from the Job not existing.
pub(crate) async fn get_check_output<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
    Path(Kept { kept }): Path<Kept>,
) -> Response {
    match served.daemon().get_check_output(job.id(), kept).await {
        Ok(output) => answer(StatusCode::OK, &output, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One frame a step's harness produced, answered as the file itself.
///
/// **The one route on this seam that does not answer JSON.** An image has no
/// window — a truncated PNG is not a shorter PNG, it is a file nothing can draw
/// — so there is no partial reading for an envelope to describe, and base64
/// would inflate the bytes by a third to carry nothing extra. The split is
/// `get_check_output`'s: the row rides `StepDetail` and the image is fetched
/// once, by whoever opens one.
///
/// The media type is read off the frame's own name rather than stored beside
/// its path, and `nosniff` goes with it — `answers::file` holds both, and why.
///
/// 404 where the Job is unknown. 422 where the Job is known and no row of it
/// kept a frame under that name — a reclaimed `.armada/frames`, or a name that
/// was never one.
/// **Two path segments and one id.** A frame's name is the harness's own, so
/// two steps of one Job may both have written `home.png` and the file name
/// alone identifies no row — the run's directory in front of it is what does.
/// They are rejoined here into the `kept` the record composes, which keeps the
/// one spelling of that identity in `showing::tail` rather than a second one
/// here.
pub(crate) async fn get_frame<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
    Path(Framed { run, name }): Path<Framed>,
) -> Response {
    match served
        .daemon()
        .get_frame(job.id(), format!("{run}/{name}"))
        .await
    {
        Ok((held, bytes)) => file(&held.name, bytes),
        Err(refusal) => refused(refusal),
    }
}

/// The workflows a proposal may name. **The set Fleet will accept**, which is
/// why it is served rather than left to a caller to know.
pub(crate) async fn list_workflows<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_workflows().await {
        Ok(workflows) => answer(StatusCode::OK, &workflows, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_manifests<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_manifests().await {
        Ok(manifests) => answer(StatusCode::OK, &manifests, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_models<D: Queries>(State(served): State<Served<D>>) -> Response {
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
pub(crate) async fn list_reports<D: Queries>(State(served): State<Served<D>>) -> Response {
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
pub(crate) async fn list_worktrees<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_worktrees().await {
        Ok(held) => answer(StatusCode::OK, &held, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
