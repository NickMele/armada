//! Two repositories' main checkouts, kept apart: each Verifies at once, each
//! run sheet answers its own, and each server lists under its own repository.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::gate::CheckBudget;
use crate::ports::PortRange;
use crate::repositories::{Located, Served, SetUp};
use crate::tests::daemon::fittings;
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const FIRST: &str = "01FIXTUREMANIFEST";
const SECOND: &str = "01SECONDMANIFEST";

/// `slow` outlives the assertions around it; `web` serves until stopped.
fn manifest_text(id: &str, slow: &str) -> String {
    format!(
        "version: 1\nid: {id}\nchecks:\n  {slow}:\n    run: \"/bin/sh -c 'echo going; sleep 30'\"\n  \
         quick:\n    run: \"/bin/sh -c 'echo quick'\"\ncommands:\n  web:\n    \
         serve: \"/bin/sh -c 'echo up; sleep 30'\"\n"
    )
}

fn committed(at: &Path, text: &str) {
    let git = |args: &[&str]| {
        let run = Command::new("git")
            .arg("-C")
            .arg(at)
            .args([
                "-c",
                "user.name=a person",
                "-c",
                "user.email=a@person.invalid",
            ])
            .args(args)
            .output()
            .expect("git on PATH");
        assert!(run.status.success(), "git {args:?} failed");
    };
    std::fs::create_dir_all(at).expect("a checkout");
    git(&["-c", "init.defaultBranch=main", "init", "--quiet"]);
    std::fs::write(at.join("armada.yml"), text).expect("the Manifest");
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "the first commit"]);
}

/// A Fleet started in `home`, serving a second repository beside it.
fn two_repositories(home: &TempDir) -> (Arc<Fixture>, Served, Served) {
    let first_text = manifest_text(FIRST, "slow");
    committed(home.path(), &first_text);
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().manifest =
        config::Manifest::parse(Path::new("armada.yml"), &first_text).expect("the first loads");
    fittings.budget = CheckBudget::of(Duration::from_secs(120));
    let base = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|bound| bound.port())
        .expect("a port the kernel will hand out");
    fittings.port_range = PortRange::of(base, base.saturating_add(19), 1);
    let fleet = Arc::new(Fleet::assembled(fittings));

    let root = home.path().join("second");
    let second_text = manifest_text(SECOND, "deliver");
    committed(&root, &second_text);
    let manifest =
        config::Manifest::parse(&root.join("armada.yml"), &second_text).expect("the second loads");
    fleet
        .repositories()
        .add(Located {
            root: root.to_string_lossy().to_string(),
            records_root: home
                .path()
                .join("second-records")
                .to_string_lossy()
                .to_string(),
            set_up: Some(SetUp::of(manifest, Default::default())),
        })
        .expect("the second is served");
    let second = fleet.repositories().serving(SECOND).expect("served");
    let first = fleet.first();
    (fleet, first, second)
}

async fn sheet(fleet: &Fixture, served: &Served) -> ipc::CheckoutRunSheet {
    fleet
        .checkout_run_sheet(served.clone())
        .await
        .expect("a sheet")
}

fn names(verify: &ipc::CheckoutVerify) -> Vec<&str> {
    verify.steps.iter().map(|step| step.name.as_str()).collect()
}

fn running_step(verify: &ipc::CheckoutVerify) -> String {
    match &verify.steps[0].state {
        ipc::VerifyStepState::Running { run_id } => run_id.clone(),
        other => panic!("the first step is out when a Verify answers: {other:?}"),
    }
}

async fn until(fleet: &Fixture, served: &Served, done: impl Fn(&ipc::CheckoutRunSheet) -> bool) {
    tokio::time::timeout(Duration::from_secs(60), async {
        while !done(&sheet(fleet, served).await) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the sheet came round");
}

/// **Two repositories Verify at once.** Each run sheet answers its own Verify
/// only, and a Verify in one refuses runs in that checkout and not the other.
#[tokio::test]
async fn two_repositories_verify_at_once_and_each_sheet_answers_its_own() {
    let home = TempDir::new();
    let (fleet, first, second) = two_repositories(&home);

    let in_first = Arc::clone(&fleet)
        .begin_checkout_verify(first.clone(), None)
        .await
        .expect("the first's Verify is underway");
    assert_eq!(
        sheet(&fleet, &second).await.verify,
        None,
        "the second has none"
    );

    let quick = ipc::StartCheckoutRun {
        name: String::from("quick"),
        workspace: None,
    };
    Arc::clone(&fleet)
        .start_checkout_rehearsal(quick.clone(), second.clone())
        .await
        .expect("a Verify in the first refuses no run in the second");
    until(&fleet, &second, |sheet| sheet.running.is_none()).await;

    let in_second = Arc::clone(&fleet)
        .begin_checkout_verify(second.clone(), None)
        .await
        .expect("the second Verifies while the first still is");
    assert_ne!(in_first.id, in_second.id);

    let (first_sheet, second_sheet) = (sheet(&fleet, &first).await, sheet(&fleet, &second).await);
    let first_verify = first_sheet.verify.expect("the first's own");
    let second_verify = second_sheet.verify.expect("the second's own");
    assert_eq!(first_verify.id, in_first.id);
    assert_eq!(names(&first_verify), ["slow", "quick"]);
    assert_eq!(second_verify.id, in_second.id);
    assert_eq!(names(&second_verify), ["deliver", "quick"]);
    assert!(first_verify.ended_at.is_none() && second_verify.ended_at.is_none());

    let refused = Arc::clone(&fleet)
        .start_checkout_rehearsal(quick, first.clone())
        .await;
    assert!(
        matches!(refused, Err(api::Refusal::IllegalMove(_))),
        "its own checkout is still held"
    );

    for (served, verify) in [(&first, &in_first), (&second, &in_second)] {
        fleet
            .stop_checkout_rehearsal(running_step(verify), served.clone())
            .await
            .expect("the step stopped");
        until(&fleet, served, |sheet| {
            sheet
                .verify
                .as_ref()
                .is_some_and(|one| one.ended_at.is_some())
        })
        .await;
    }
}

/// **A server started in each repository's main checkout lists under its own
/// repository**, started through the route that names it.
#[tokio::test]
async fn a_server_started_in_each_repository_lists_under_its_own() {
    let home = TempDir::new();
    let (fleet, _, _) = two_repositories(&home);
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        ipc::RunId::carried("01RUN"),
        fleet.events(),
    ));

    let mut started = Vec::new();
    for query in ["", "?manifest_id=01SECONDMANIFEST"] {
        let uri = format!("/servers/start{query}");
        let (status, body) = call(&app, "POST", &uri, r#"{"name":"web"}"#).await;
        assert_eq!(
            status,
            StatusCode::ACCEPTED,
            "{}",
            String::from_utf8_lossy(&body)
        );
        let state: ipc::ServerState = ipc::decode("a server", &body).expect("a server");
        started.push(state.id);
    }
    assert_ne!(started[0], started[1], "one instance per repository");

    let (_, body) = call(&app, "GET", "/servers", "").await;
    let listed: ipc::ServerList = ipc::decode("servers", &body).expect("a list");
    let whose = |id: &str| {
        listed
            .servers
            .iter()
            .find(|one| one.id == id)
            .and_then(|one| one.manifest_id.as_ref())
            .map(|held| held.as_str().to_string())
    };
    assert_eq!(whose(&started[0]).as_deref(), Some(FIRST));
    assert_eq!(whose(&started[1]).as_deref(), Some(SECOND));

    fleet.stopped_every_server().await;
}
