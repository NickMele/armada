//! The run sheet's routes: Journey 9, *Running one inside a Job*.
//!
//! **One subject on one page**, the reads and the writes together, because a
//! reader of one needs the other; `crate::routes` still holds the table.

use axum::body::Bytes;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{NamedRun, StartRun};
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::reference::Resolved;
use crate::routes::Served;

/// The segment after `runs` on `get_run_output`.
#[derive(Deserialize)]
pub(crate) struct Ran {
    run_id: String,
}

pub(crate) async fn get_run_sheet<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().get_run_sheet(job.id()).await {
        Ok(sheet) => answer(StatusCode::OK, &sheet, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_runs<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().list_runs(job.id()).await {
        Ok(runs) => answer(StatusCode::OK, &runs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_run_output<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
    Path(Ran { run_id }): Path<Ran>,
) -> Response {
    match served.daemon().get_run_output(job.id(), run_id).await {
        Ok(output) => answer(StatusCode::OK, &output, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One run's output, as it prints. **A socket per run**, `crate::watching_run`;
/// the daemon is asked before the upgrade, so an id naming no run of this Job
/// is a refusal read at the moment it was asked.
pub(crate) async fn observe_run<D: Queries>(
    State(served): State<Served<D>>,
    job: Resolved,
    Path(Ran { run_id }): Path<Ran>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served.daemon().observe_run(job.id(), run_id).await {
        Ok(observed) => {
            upgrade.on_upgrade(move |socket| crate::watching_run::relay(socket, observed))
        }
        Err(refusal) => refused(refusal),
    }
}

/// 202: the run is underway and has not finished — its end is `run.finished`.
pub(crate) async fn start_run<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let asked: StartRun = match ipc::decode("a run to start", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_run(job.id(), asked).await {
        Ok(underway) => answer(StatusCode::ACCEPTED, &underway, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn stop_run<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let named: NamedRun = match ipc::decode("a run to stop", &body) {
        Ok(named) => named,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().stop_run(job.id(), named).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn undo_run<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let named: NamedRun = match ipc::decode("a run to undo", &body) {
        Ok(named) => named,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().undo_run(job.id(), named).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
