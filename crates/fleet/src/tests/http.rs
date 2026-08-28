//! One request through the router that ships, for every test that makes one.
//!
//! Here rather than beside the first caller: `serving` and `proposing` both
//! drive the real router, and a second copy of this would be a second answer to
//! what a request looks like.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

pub(crate) async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    body: &str,
) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers every request");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body that reads")
        .to_bytes()
        .to_vec();
    (status, body)
}
