//! A Studio over the router and through the door: its graph round-trips, a
//! person's acts reach no agent, and a Helm session is held to what it may
//! propose. `#1285`.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::door::Reachable;
use ipc::{Studio, StudioEdgeStanding};
use tower::ServiceExt;

use crate::tests::fake::{FakeDaemon, THE_STUDIO};
use crate::tests::shapes::run_id;
use crate::{door_within, router, Broadcaster, Redirector, Served, DOOR_PATH};

/// Every Studio act that is a person's alone.
const A_PERSONS: &[&str] = &[
    "create_studio",
    "delete_studio",
    "move_studio_node",
    "remove_studio_node",
    "decide_studio_edge",
];

const HELM: u16 = 52000;
const ANYONE: u16 = 52001;

fn acting(_: &Reachable) -> bool {
    true
}

fn shared(daemon: &Arc<FakeDaemon>) -> Router {
    router(Served::sharing(
        Arc::clone(daemon),
        run_id(),
        Broadcaster::new(),
    ))
}

fn helm_holding() -> Arc<FakeDaemon> {
    let daemon = FakeDaemon::new(Broadcaster::new());
    *daemon.helm_on.lock().expect("not poisoned") = Some((HELM, acting));
    Arc::new(daemon)
}

async fn sent(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes();
    (status, String::from_utf8_lossy(&body).to_string())
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    sent(app, request).await
}

async fn from(app: &Router, port: u16, uri: &str, body: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    sent(app, request).await.1
}

fn calling(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;

fn studio(body: &str) -> Studio {
    ipc::decode("a Studio", body.as_bytes()).expect("a Studio")
}

/// Created, two Notes added, one moved, a Same as proposed: what reads back
/// is what was sent.
#[tokio::test]
async fn a_studio_built_over_the_wire_reads_back_as_it_was_built() {
    let app = shared(&helm_holding());
    let (status, created) = call(&app, "POST", "/studios/create", r#"{"name":"Counts"}"#).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let id = studio(&created).id;
    let at = |act: &str| format!("/studios/{}/{act}", id.as_str());
    for said in ["The chip keeps its count", "Overview says three"] {
        let note = format!(r#"{{"kind":"note","said":"{said}","position":{{"x":0,"y":0}}}}"#);
        let (status, body) = call(&app, "POST", &at("add_node"), &note).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    let (_, body) = call(
        &app,
        "POST",
        &at("move_node"),
        r#"{"node_id":"01NODE1","position":{"x":320,"y":-40}}"#,
    )
    .await;
    assert_eq!(studio(&body).nodes[1].position.x, 320);
    let (status, body) = call(
        &app,
        "POST",
        &at("propose_edge"),
        r#"{"from":"01NODE0","to":"01NODE1","kind":"same_as"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, read) = call(&app, "GET", &format!("/studios/{}", id.as_str()), "").await;
    assert_eq!(status, StatusCode::OK);
    let read = studio(&read);
    assert_eq!(read, studio(&body), "a read agrees with the last answer");
    assert_eq!(read.name.as_deref(), Some("Counts"));
    assert_eq!(
        read.edges[0].standing,
        StudioEdgeStanding::from_wire("proposed").expect("a standing")
    );
}

/// **A person's acts are offered to no agent**, Helm included, and a call to
/// one is refused before any route is reached.
#[tokio::test]
async fn no_agent_is_offered_a_persons_studio_act_and_a_call_to_one_is_refused() {
    let daemon = helm_holding();
    let app = shared(&daemon);
    let helm = from(&app, HELM, DOOR_PATH, LISTING).await;
    let anyone = from(&app, ANYONE, DOOR_PATH, LISTING).await;
    for act in A_PERSONS {
        let named = format!("\"name\":\"{act}\"");
        assert!(!helm.contains(&named), "{act} offered to Helm");
        assert!(!anyone.contains(&named), "{act} offered to an agent");
    }
    assert!(anyone.contains("\"name\":\"get_studio\""), "{anyone}");
    assert!(helm.contains("\"name\":\"propose_studio_edge\""), "{helm}");
    assert!(
        !anyone.contains("\"name\":\"propose_studio_edge\""),
        "{anyone}"
    );

    let accept = calling(
        "decide_studio_edge",
        &format!(
            r#"{{"studio_id":"{THE_STUDIO}","body":{{"edge_id":"01EDGE0","accepted":true}}}}"#
        ),
    );
    for port in [HELM, ANYONE] {
        let body = from(&app, port, DOOR_PATH, &accept).await;
        assert!(body.contains("\"isError\":true"), "{body}");
        assert!(body.contains("is not a tool this Fleet offers"), "{body}");
    }
}

/// **Helm adds what starts proposed, and the transport says it was Helm.** A
/// Note is what a person pointed at, so Helm's is refused.
#[tokio::test]
async fn a_node_helm_adds_is_recorded_as_helms_and_held_to_a_proposed_kind() {
    let daemon = helm_holding();
    let app = shared(&daemon);
    let adding = |content: &str| {
        calling(
            "add_studio_node",
            &format!(
                r#"{{"studio_id":"{THE_STUDIO}","body":{{{content},"position":{{"x":0,"y":0}}}}}}"#
            ),
        )
    };
    let finding = from(
        &app,
        HELM,
        DOOR_PATH,
        &adding(r#""kind":"finding","asked":"what reads the count""#),
    )
    .await;
    assert!(finding.contains("\"isError\":false"), "{finding}");
    let note = from(
        &app,
        HELM,
        DOOR_PATH,
        &adding(r#""kind":"note","said":"Helm saw this""#),
    )
    .await;
    assert!(note.contains("\"isError\":true"), "{note}");
    assert_eq!(
        *daemon.added_by.lock().expect("not poisoned"),
        vec![Redirector::Helm, Redirector::Helm]
    );

    let (status, _) = call(
        &app,
        "POST",
        &format!("/studios/{THE_STUDIO}/add_node"),
        r#"{"kind":"note","said":"a person saw this","position":{"x":0,"y":0}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Bridge's own call is a person's");
    assert_eq!(
        daemon.added_by.lock().expect("not poisoned").last(),
        Some(&Redirector::Person)
    );
}

/// A door session is answered only about its own repository's Studios, by id
/// and by list.
#[tokio::test]
async fn a_door_session_reads_no_studio_of_another_repository() {
    let daemon = helm_holding();
    daemon.studios.lock().expect("not poisoned").push(Studio {
        id: ipc::StudioId::carried("01THEIRS"),
        manifest_id: ipc::ManifestId::carried("01OTHER"),
        ..crate::tests::fake::the_studio()
    });
    let app = shared(&daemon);
    let read = |id: &str| calling("get_studio", &format!(r#"{{"studio_id":"{id}"}}"#));
    let ours = from(&app, ANYONE, &door_within("01MF"), &read(THE_STUDIO)).await;
    assert!(ours.contains("\"isError\":false"), "{ours}");
    let theirs = from(&app, ANYONE, &door_within("01MF"), &read("01THEIRS")).await;
    assert!(theirs.contains("\"isError\":true"), "{theirs}");
    assert!(theirs.contains("no Studio is"), "{theirs}");
    let (status, _) = call(&app, "GET", "/studios/01THEIRS", "").await;
    assert_eq!(status, StatusCode::OK, "Bridge names a Studio by id alone");

    let listed = from(
        &app,
        ANYONE,
        &door_within("01MF"),
        &calling("list_studios", r#"{"manifest_id":"01OTHER"}"#),
    )
    .await;
    assert!(listed.contains("\"isError\":true"), "{listed}");
    assert!(listed.contains("the one it stands in"), "{listed}");
}

/// **Helm's other unasked acts say who took them too** (#1288): proposing an
/// edge and naming a Studio through the door are Helm's, and the same calls from
/// Bridge are a person's, so Fleet can publish the one as Helm's act.
#[tokio::test]
async fn an_edge_helm_proposes_and_a_name_it_gives_are_recorded_as_helms() {
    let daemon = helm_holding();
    let app = shared(&daemon);
    let proposed = from(
        &app,
        HELM,
        DOOR_PATH,
        &calling(
            "propose_studio_edge",
            &format!(
                r#"{{"studio_id":"{THE_STUDIO}","body":{{"from":"01A","to":"01B","kind":"same_as"}}}}"#
            ),
        ),
    )
    .await;
    assert!(proposed.contains("\"isError\":false"), "{proposed}");
    let named = from(
        &app,
        HELM,
        DOOR_PATH,
        &calling(
            "rename_studio",
            &format!(r#"{{"studio_id":"{THE_STUDIO}","body":{{"name":"Stale counts"}}}}"#),
        ),
    )
    .await;
    assert!(named.contains("\"isError\":false"), "{named}");
    let (status, _) = call(
        &app,
        "POST",
        &format!("/studios/{THE_STUDIO}/rename"),
        r#"{"name":"A person's name for it"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        *daemon.added_by.lock().expect("not poisoned"),
        vec![Redirector::Helm, Redirector::Helm, Redirector::Person]
    );
}

/// **Helm reads the checkout runs it can start, in its own repository** (#1288).
/// A checkout route answers about the first repository served when none is
/// named, so the door names the session's; a plain agent is offered none of
/// them.
#[tokio::test]
async fn helm_reads_checkout_runs_in_the_repository_it_stands_in() {
    let daemon = helm_holding();
    let app = shared(&daemon);
    let anyone = from(&app, ANYONE, DOOR_PATH, LISTING).await;
    let helm = from(&app, HELM, DOOR_PATH, LISTING).await;
    for read in [
        "list_checkout_runs",
        "get_checkout_run_sheet",
        "get_checkout_run_output",
    ] {
        let named = format!("\"name\":\"{read}\"");
        assert!(helm.contains(&named), "{read} is not offered to Helm");
        assert!(!anyone.contains(&named), "{read} is offered to an agent");
    }

    let listed = from(
        &app,
        HELM,
        &door_within("01MF"),
        &calling("list_checkout_runs", "{}"),
    )
    .await;
    assert!(listed.contains("\"isError\":false"), "{listed}");
    let (status, _) = call(&app, "GET", "/manifest/runs", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        *daemon.checkout_runs_named.lock().expect("not poisoned"),
        vec![Some(ipc::ManifestId::carried("01MF")), None],
        "the door names the session's repository, and Bridge names none"
    );
}
