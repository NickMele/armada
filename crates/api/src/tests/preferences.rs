//! `get_preferences` and `save_preferences` over the router: a save
//! round-trips, and a name outside the closed set is refused by name rather
//! than saved.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{Preferences, WireError};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::{self, run_id};
use crate::{router, Broadcaster, Served};

fn wired() -> Router {
    let events = Broadcaster::new();
    router(Served::by(
        FakeDaemon::new(events.clone()),
        run_id(),
        events,
    ))
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

async fn read(app: &Router) -> Preferences {
    let (status, body) = call(app, "GET", "/preferences", "").await;
    assert_eq!(status, StatusCode::OK);
    ipc::decode("preferences", &body).expect("the preferences decode")
}

#[tokio::test]
async fn a_save_round_trips_and_a_read_agrees_with_it() {
    let app = wired();
    assert_eq!(read(&app).await, shapes::preferences());

    let (status, body) = call(
        &app,
        "POST",
        "/preferences/save",
        r#"{"name":"where_things_are_open","value":true}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let saved: Preferences = ipc::decode("saved preferences", &body).expect("decodes");
    assert_eq!(
        saved,
        Preferences {
            where_things_are_open: true
        }
    );
    assert_eq!(read(&app).await, saved, "a read agrees with it");
}

#[tokio::test]
async fn a_name_outside_the_closed_set_is_refused_by_name_and_nothing_is_saved() {
    let app = wired();
    let (status, answer) = call(
        &app,
        "POST",
        "/preferences/save",
        r#"{"name":"where_things_are_purple","value":true}"#,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let error: WireError = ipc::decode("an error", &answer).expect("the usual error shape");
    assert!(
        error.message.contains("where_things_are_purple"),
        "{}",
        error.message
    );
    assert_eq!(read(&app).await, shapes::preferences(), "nothing was saved");
}
