//! `get_limits` and `save_limits` over the router: a save round-trips, an
//! omitted field keeps its value, and a value out of range is a 400 that
//! saves nothing — not even the in-range field sent beside it.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{FleetLimits, WireError};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::{self, run_id};
use crate::{router, Broadcaster, Served};

fn wired() -> Router {
    let events = Broadcaster::new();
    router(Served::by(FakeDaemon::new(events.clone()), run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes();
    (status, bytes.to_vec())
}

async fn read(app: &Router) -> FleetLimits {
    let (status, body) = call(app, "GET", "/limits", "").await;
    assert_eq!(status, StatusCode::OK);
    ipc::decode("limits", &body).expect("the limits decode")
}

#[tokio::test]
async fn a_save_round_trips_and_an_omitted_field_keeps_its_value() {
    let app = wired();
    assert_eq!(read(&app).await, shapes::limits());

    let (status, body) = call(&app, "POST", "/limits/save", r#"{"concurrency":4}"#).await;
    assert_eq!(status, StatusCode::OK);
    let saved: FleetLimits = ipc::decode("saved limits", &body).expect("decodes");

    let mut expected = shapes::limits();
    expected.values.concurrency = 4;
    assert_eq!(saved, expected, "the answer is what is now in force");
    assert_eq!(read(&app).await, expected, "and a read agrees with it");
}

#[tokio::test]
async fn a_value_out_of_range_is_a_400_and_nothing_is_saved() {
    let app = wired();
    for body in [
        r#"{"concurrency":9}"#,
        r#"{"memory_spare_percent":51}"#,
        r#"{"concurrency":3,"disk_floor_gib":101}"#,
    ] {
        let (status, answer) = call(&app, "POST", "/limits/save", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        let error: WireError = ipc::decode("an error", &answer).expect("the usual error shape");
        assert!(!error.message.is_empty(), "{body}");
    }
    assert_eq!(
        read(&app).await,
        shapes::limits(),
        "the concurrency of 3 beside a bad floor was not taken either"
    );
}
