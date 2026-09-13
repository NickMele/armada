//! The run sheet's routes: Journey 9, *Running one* and *Running one inside a
//! Job*.
//!
//! **One subject on one page**, the reads and the writes together, because a
//! reader of one needs the other; `crate::routes` still holds the table.

use axum::body::Bytes;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{NamedRun, StartCheckoutRun, StartRun};
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::reference::Resolved;
use crate::served::Served;

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

// ---------------------------------------------------------------------------
// The checkout's half — Journey 9, *Running one*
// ---------------------------------------------------------------------------
//
// **No `Resolved` extractor on any of these.** There is no Job to resolve:
// these routes are under `/manifest`, beside `get_manifest_reading`, for the
// reason that one is — a Fleet serves one repository, and this reads it before
// any Job exists to read one from instead.

pub(crate) async fn get_checkout_run_sheet<D: Queries>(
    State(served): State<Served<D>>,
) -> Response {
    match served.daemon().get_checkout_run_sheet().await {
        Ok(sheet) => answer(StatusCode::OK, &sheet, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_checkout_runs<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_checkout_runs().await {
        Ok(runs) => answer(StatusCode::OK, &runs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_checkout_run_output<D: Queries>(
    State(served): State<Served<D>>,
    Path(Ran { run_id }): Path<Ran>,
) -> Response {
    match served.daemon().get_checkout_run_output(run_id).await {
        Ok(output) => answer(StatusCode::OK, &output, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What one checkout run changed, against its own snapshot. A read: nothing in
/// the checkout is written, staged or committed.
pub(crate) async fn get_checkout_run_diff<D: Queries>(
    State(served): State<Served<D>>,
    Path(Ran { run_id }): Path<Ran>,
) -> Response {
    match served.daemon().get_checkout_run_diff(run_id).await {
        Ok(diff) => answer(StatusCode::OK, &diff, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One checkout run's output, as it prints — `observe_run`'s socket, one
/// owner over.
pub(crate) async fn observe_checkout_run<D: Queries>(
    State(served): State<Served<D>>,
    Path(Ran { run_id }): Path<Ran>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served.daemon().observe_checkout_run(run_id).await {
        Ok(observed) => {
            upgrade.on_upgrade(move |socket| crate::watching_run::relay_checkout(socket, observed))
        }
        Err(refusal) => refused(refusal),
    }
}

/// 202: the run is underway and has not finished.
pub(crate) async fn start_checkout_run<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: StartCheckoutRun = match ipc::decode("a run to start in the checkout", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_checkout_run(asked).await {
        Ok(underway) => answer(StatusCode::ACCEPTED, &underway, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 202: the Verify is underway, its first step out. **No body**: Verify runs
/// setup and every Check, and there is nothing to choose.
pub(crate) async fn start_checkout_verify<D: Commands>(
    State(served): State<Served<D>>,
) -> Response {
    match served.shared().start_checkout_verify().await {
        Ok(verify) => answer(StatusCode::ACCEPTED, &verify, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn stop_checkout_run<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let named: NamedRun = match ipc::decode("a run to stop", &body) {
        Ok(named) => named,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().stop_checkout_run(named).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn undo_checkout_run<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let named: NamedRun = match ipc::decode("a run to undo", &body) {
        Ok(named) => named,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().undo_checkout_run(named).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
