//! The reads that belong to no Job: the roster, the machine, the spend, the
//! configuration, and what crossed the stream.
//!
//! **Not under `/jobs`**, for `get_capacity`'s reason — a fact about Fleet
//! hung off a Job would be a fact about that Job.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{Cursor, DroneId, ManifestId};
use serde::Deserialize;

use crate::answers::{answer, refused};
use crate::daemon::Queries;
use crate::served::Served;

/// The `:drone_id` segment.
#[derive(Deserialize)]
pub(crate) struct Named {
    drone_id: String,
}

/// The `:manifest_id` segment.
#[derive(Deserialize)]
pub(crate) struct Which {
    manifest_id: String,
}

/// `?since=` on `get_events_since`.
///
/// **A query and not a path segment**, for `search_files`' reason: it is a
/// position a caller carries between turns rather than a resource under a
/// collection. Absent reads as nought, which is the whole stream.
#[derive(Deserialize)]
pub(crate) struct From {
    #[serde(default)]
    since: u64,
}

/// Every Drone Fleet holds a slot for.
pub(crate) async fn list_drones<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_drones().await {
        Ok(drones) => answer(StatusCode::OK, &drones, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One Drone: what it declared, what drifted outside it, and a window of its
/// own rows.
pub(crate) async fn get_drone<D: Queries>(
    State(served): State<Served<D>>,
    Path(Named { drone_id }): Path<Named>,
) -> Response {
    match served.daemon().get_drone(DroneId::carried(drone_id)).await {
        Ok(drone) => answer(StatusCode::OK, &drone, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The probes Fleet can run on itself, and the Doctor modules it cannot.
pub(crate) async fn get_health<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_health().await {
        Ok(health) => answer(StatusCode::OK, &health, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What the fleet has spent, against the ceilings that refuse the next Drone.
pub(crate) async fn get_usage<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_usage().await {
        Ok(usage) => answer(StatusCode::OK, &usage, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// One Manifest as Fleet resolved it.
pub(crate) async fn get_manifest<D: Queries>(
    State(served): State<Served<D>>,
    Path(Which { manifest_id }): Path<Which>,
) -> Response {
    match served
        .daemon()
        .get_manifest(ManifestId::carried(manifest_id))
        .await
    {
        Ok(manifest) => answer(StatusCode::OK, &manifest, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What crossed the stream since a cursor, counted rather than carried.
///
/// **Answered from the broadcaster this listener already holds**, not from the
/// daemon: the positions are this channel's own. It cannot refuse — a window
/// that has lost rows says how many rather than failing.
pub(crate) async fn get_events_since<D: Queries>(
    State(served): State<Served<D>>,
    Query(From { since }): Query<From>,
) -> Response {
    let counted = served.events().since(Cursor::at(since));
    answer(StatusCode::OK, &counted, served.run_id())
}
