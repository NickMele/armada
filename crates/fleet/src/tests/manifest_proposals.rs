//! Setup's three operations over the router that ships, answered by a real
//! Fleet. What a proposal holds and what Write does to a file are
//! `manifest_proposal`'s own tests; this is the line between them and the
//! checkout a Fleet serves — `serving`'s `search_files` reason.

use std::sync::Arc;

use axum::http::StatusCode;
use ipc::{ManifestProposal, ManifestProposals, Provenance, RunId, WireError};
use testkit::FakeWorkProduct;

use crate::tests::daemon::a_fleet;
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

fn refusal(body: &[u8]) -> WireError {
    ipc::decode("a refusal", body).expect("a WireError")
}

/// **The three operations, over the router that ships, from a real Fleet** —
/// against `Host::repo_root`, the checkout it serves, holding the edit between
/// requests.
#[tokio::test]
async fn a_fleet_proposes_holds_an_edit_and_writes_the_checkout_it_serves() {
    let home = TempDir::new();
    let package = r#"{"scripts":{"test":"vitest run"}}"#;
    std::fs::write(home.path().join("package.json"), package).expect("a package");
    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::changed(&[])));
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));

    let edit =
        r#"{"dir":".","edit":{"edit":"command","name":"fmt","run":"pnpm prettier --write ."}}"#;
    let (status, body) = call(&app, "POST", "/repository/edit_proposal", edit).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

    let (status, body) = call(&app, "GET", "/repository/proposals", "").await;
    assert_eq!(status, StatusCode::OK);
    let proposals: ManifestProposals = ipc::decode("proposals", &body).expect("proposals");
    let root = &proposals.proposals[0];
    assert_eq!(root.checks[0].name, "test");
    let fmt = root
        .commands
        .iter()
        .find(|one| one.name == "fmt")
        .expect("the edit is held between requests");
    assert_eq!(fmt.provenance, Provenance::AddedDuringSetup);

    let write = r#"{"dir":"."}"#;
    let (status, body) = call(&app, "POST", "/repository/write_proposal", write).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let written: ManifestProposal = ipc::decode("the proposal", &body).expect("a proposal");
    assert!(written.written.is_some());
    let file = home.path().join("armada.yml");
    assert_eq!(
        std::fs::read_to_string(&file).expect("on disk"),
        written.text
    );
    config::Manifest::load(&file).unwrap_or_else(|why| panic!("loads: {why}"));

    let (status, body) = call(&app, "POST", "/repository/write_proposal", write).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(refusal(&body).code, "fleet.proposal_written");

    let nowhere = r#"{"dir":"nowhere"}"#;
    let (status, body) = call(&app, "POST", "/repository/write_proposal", nowhere).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(refusal(&body).code, "fleet.no_such_workspace");
}
