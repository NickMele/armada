//! Servers' routes: a Command with `serve`, held by Fleet —
//! `docs/concepts/fleet.md`, *Servers*.
//!
//! **Not under `/jobs/:job_id`**, because a server started with no Job is one
//! of these and belongs to none. The Job, where there is one, is in the start
//! request's body and on every instance, for `stop_proposal`'s reason: an
//! instance has an id of its own and the Job adds nothing to finding it.

use axum::body::Bytes;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{NamedServer, StartServer};
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::routes::Served;

/// The segment before `observe` on `observe_server`.
#[derive(Deserialize)]
pub(crate) struct Instance {
    server_id: String,
}

pub(crate) async fn list_servers<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_servers().await {
        Ok(servers) => answer(StatusCode::OK, &servers, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One server's output, as it prints. Asked before the upgrade, so an id naming
/// no instance is a refusal read at the moment it was asked.
pub(crate) async fn observe_server<D: Queries>(
    State(served): State<Served<D>>,
    Path(Instance { server_id }): Path<Instance>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served.daemon().observe_server(server_id).await {
        Ok(observed) => {
            upgrade.on_upgrade(move |socket| crate::watching_run::relay_server(socket, observed))
        }
        Err(refusal) => refused(refusal),
    }
}

/// 202: starting, or already up — a second request gets the running one.
pub(crate) async fn start_server<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: StartServer = match ipc::decode("a server to start", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_server(asked).await {
        Ok(state) => answer(StatusCode::ACCEPTED, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn stop_server<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let named: NamedServer = match ipc::decode("a server to stop", &body) {
        Ok(named) => named,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().stop_server(named).await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
