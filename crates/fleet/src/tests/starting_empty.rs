//! A Fleet that starts serving no repository, as a fresh install does: an
//! empty list, plain refusals, and the first add published and remembered.

use std::sync::Arc;

use axum::http::StatusCode;
use testkit::{FakeHarness, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::repositories::{Locating, NotLocated};
use crate::tests::daemon::fitted_with;
use crate::tests::http::call;
use crate::tests::repositories::{body_of, code, second_repository, served, Fixture, Planted};
use crate::tests::tmp::TempDir;

fn serving_nothing(home: &TempDir, planted: &Arc<Planted>) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::untouched(),
        FakeHarness::that_listens(),
    );
    fittings.starting_in = None;
    fittings.locating = Arc::clone(planted) as Arc<dyn Locating>;
    Fleet::assembled(fittings)
}

fn roots(body: &[u8]) -> Vec<String> {
    let listed: ipc::RepositoryList = ipc::decode("repositories", body).expect("a list");
    listed
        .repositories
        .into_iter()
        .map(|one| one.root)
        .collect()
}

/// Everything the subscription has waiting, stopping at the first pause.
async fn published(subscription: &mut api::Subscription) -> Vec<ipc::Event> {
    let mut seen = Vec::new();
    while let Ok(Some(api::Next::Send(delivered))) =
        tokio::time::timeout(std::time::Duration::from_millis(200), subscription.next()).await
    {
        seen.push(delivered.event);
    }
    seen
}

/// **Started with nothing, Fleet serves an empty list**, reconciles nothing,
/// and a route that needs a repository refuses in words rather than panicking.
#[tokio::test]
async fn a_fleet_started_with_nothing_serves_an_empty_list_and_refuses_plainly() {
    let home = TempDir::new();
    let fleet = Arc::new(serving_nothing(&home, &Arc::new(Planted::nothing())));
    let app = served(&fleet);

    let (status, body) = call(&app, "GET", "/repositories", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(roots(&body).is_empty());
    let (status, body) = call(&app, "GET", "/manifests", "").await;
    assert_eq!(
        (status, body.as_slice()),
        (StatusCode::OK, b"[]".as_slice())
    );

    let refusing = [
        "/workflows/left_out",
        "/repository/scan",
        "/manifest/run_sheet",
        "/manifest/reading",
    ];
    for path in refusing {
        let (status, body) = call(&app, "GET", path, "").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{path}");
        assert_eq!(code(&body), "fleet.no_repository", "{path}");
        let refused: ipc::WireError = ipc::decode("a refusal", &body).expect("a refusal");
        assert!(
            refused.message.contains("add one by folder"),
            "{path}: {}",
            refused.message
        );
    }
    for path in ["/manifest/runs", "/manifest/drift", "/servers", "/jobs"] {
        let (status, body) = call(&app, "GET", path, "").await;
        let plain = status == StatusCode::OK
            || (status == StatusCode::UNPROCESSABLE_ENTITY && code(&body) == "fleet.no_repository");
        assert!(plain, "{path}: {status} {}", String::from_utf8_lossy(&body));
    }

    let reconciled = fleet.reconcile().await.expect("nothing to reconcile");
    assert!(reconciled.interrupted.is_empty() && reconciled.admitted.is_empty());
}

/// **The first repository added is published whole**, answers the routes that
/// refused a moment ago, and is served again after a restart.
#[tokio::test]
async fn the_first_repository_added_is_published_and_served_again_after_a_restart() {
    let home = TempDir::new();
    let folder = second_repository(&home).root;
    {
        let planted = Arc::new(Planted::nothing().with(&folder, second_repository(&home)));
        let fleet = Arc::new(serving_nothing(&home, &planted));
        let app = served(&fleet);
        let mut events = fleet.events().subscribe();

        let (status, body) = call(&app, "POST", "/repositories/add", &body_of(&folder)).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "{}",
            String::from_utf8_lossy(&body)
        );
        let changed: Vec<Vec<String>> = published(&mut events)
            .await
            .into_iter()
            .filter_map(|event| match event {
                ipc::Event::RepositoriesChanged(list) => {
                    Some(list.repositories.into_iter().map(|one| one.root).collect())
                }
                _ => None,
            })
            .collect();
        assert_eq!(changed, [vec![folder.clone()]], "published once, whole");
        let (status, _) = call(&app, "GET", "/workflows/left_out", "").await;
        assert_eq!(status, StatusCode::OK, "the first added now answers");
    }

    let planted = Arc::new(Planted::nothing().with(&folder, second_repository(&home)));
    let restarted = Arc::new(serving_nothing(&home, &planted));
    assert!(restarted.served_again().await.is_empty());
    let (_, body) = call(&served(&restarted), "GET", "/repositories", "").await;
    assert_eq!(roots(&body), [folder]);
}

/// **A folder that is not a repository reads as one sentence** naming it
/// once, however much the reader said underneath.
#[tokio::test]
async fn a_folder_that_is_not_a_repository_is_refused_naming_it_once() {
    let home = TempDir::new();
    let fleet = Arc::new(serving_nothing(&home, &Arc::new(Planted::nothing())));
    let (status, body) = call(
        &served(&fleet),
        "POST",
        "/repositories/add",
        &body_of("/plain/folder"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let refused: ipc::WireError = ipc::decode("a refusal", &body).expect("a refusal");
    assert_eq!(
        refused.message.matches("/plain/folder").count(),
        1,
        "{}",
        refused.message
    );
    assert!(
        !refused.message.contains("planted"),
        "the cause stays out: {}",
        refused.message
    );
    let underneath = NotLocated::NotARepository {
        folder: String::from("/plain/folder"),
        why: String::from(
            "/plain/folder could not be opened: class=Repository (6); code=NotFound (-3)",
        ),
    };
    assert!(!underneath.to_string().contains("class="));
}
