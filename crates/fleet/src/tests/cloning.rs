//! Cloning a repository from a URL. Git runs for real, against a bare
//! repository in a temporary directory, and never a network.

use std::path::Path;
use std::sync::Arc;

use adapters::GitVcs;
use axum::http::StatusCode;
use ipc::{ManifestSummary, RepositoryList, RepositorySummary, RunId, WireError};
use testkit::{FakeHarness, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::repositories::{folder_named_by, Located, Locating};
use crate::tests::daemon::fitted_over;
use crate::tests::http::call;
use crate::tests::reclaim::{commit, git};
use crate::tests::repositories::Planted;
use crate::tests::tmp::TempDir;

type Cloner = Fleet<FakeHarness, GitVcs, FakeWorkProduct>;

/// A bare repository holding one commit, named `shop.git`.
fn a_bare_origin(home: &TempDir) -> String {
    let work = home.path().join("work");
    std::fs::create_dir_all(&work).expect("a working copy");
    std::fs::write(work.join("README"), "the shop\n").expect("a file");
    git(&work, &["-c", "init.defaultBranch=main", "init", "--quiet"]);
    git(&work, &["add", "README"]);
    commit(&work, "the first commit");
    let bare = home.path().join("origin").join("shop.git");
    let (from, to) = (work.to_string_lossy(), bare.to_string_lossy());
    git(home.path(), &["clone", "--quiet", "--bare", &from, &to]);
    to.to_string()
}

/// A Fleet over real git, reading `<parent>/shop` as a folder with no Manifest.
fn a_fleet_cloning_under(home: &TempDir, parent: &Path) -> (Arc<Cloner>, String) {
    std::fs::create_dir_all(parent).expect("a parent folder");
    let real = parent.canonicalize().expect("a real path");
    let root = real.join("shop").to_string_lossy().to_string();
    let located = Located {
        root: root.clone(),
        records_root: format!("{root}-records"),
        set_up: None,
    };
    let planted = Arc::new(Planted::nothing().with(&root, located));
    let (work, harness) = (FakeWorkProduct::untouched(), FakeHarness::that_listens());
    let mut fittings = fitted_over(home, work, harness, GitVcs::new());
    fittings.locating = planted as Arc<dyn Locating>;
    (Arc::new(Fleet::assembled(fittings)), root)
}

fn served(fleet: &Arc<Cloner>) -> axum::Router {
    api::router(api::Served::sharing(
        Arc::clone(fleet),
        RunId::carried("01RUN"),
        fleet.events(),
    ))
}

fn asking(url: &str, parent: &Path) -> String {
    format!(r#"{{"url":"{url}","parent":"{}"}}"#, parent.display())
}

fn refusal(body: &[u8]) -> WireError {
    ipc::decode("a refusal", body).expect("a WireError")
}

async fn listed(app: &axum::Router) -> Vec<String> {
    let (_, body) = call(app, "GET", "/repositories", "").await;
    let listed: RepositoryList = ipc::decode("repositories", &body).expect("repositories");
    listed
        .repositories
        .into_iter()
        .map(|one| one.root)
        .collect()
}

#[test]
fn a_url_names_the_folder_git_would_make() {
    for (url, name) in [
        ("ssh://host.invalid/owner/shop.git", Some("shop")),
        ("host.invalid:owner/shop.git", Some("shop")),
        ("host.invalid:shop", Some("shop")),
        ("/srv/repos/shop.git/", Some("shop")),
        ("/srv/repos/shop/.git", Some("shop")),
        ("file:///srv/repos/shop", Some("shop")),
        ("host.invalid:", None),
        ("  ", None),
    ] {
        assert_eq!(folder_named_by(url).as_deref(), name, "{url}");
    }
}

/// **Cloned into `<parent>/<name>`, served, and listed as not set up** — an
/// empty folder of that name is where git may still land.
#[tokio::test]
async fn a_repository_cloned_from_a_url_lands_under_the_parent_and_is_served_not_set_up() {
    let home = TempDir::new();
    let url = a_bare_origin(&home);
    let parent = home.path().join("projects");
    let (fleet, root) = a_fleet_cloning_under(&home, &parent);
    std::fs::create_dir_all(parent.join("shop")).expect("an empty folder");
    let app = served(&fleet);

    let (status, body) = call(&app, "POST", "/repositories/clone", &asking(&url, &parent)).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let cloned: RepositorySummary = ipc::decode("the repository", &body).expect("a repository");
    assert_eq!(cloned.root, root);
    assert!(cloned.manifest.is_none(), "no armada.yml, so not set up");
    let readme = std::fs::read_to_string(Path::new(&root).join("README")).expect("checked out");
    assert_eq!(readme, "the shop\n");

    assert_eq!(listed(&app).await.last(), Some(&root));
    let (_, body) = call(&app, "GET", "/manifests", "").await;
    let manifests: Vec<ManifestSummary> = ipc::decode("manifests", &body).expect("manifests");
    assert_eq!(manifests.len(), 1, "and lists as no Manifest");
}

/// **Refused, and said why:** a parent that is not there or not absolute, a
/// URL git refuses in git's own words, and a destination with something in it.
#[tokio::test]
async fn a_clone_into_a_missing_or_occupied_folder_or_from_a_bad_url_is_refused() {
    let home = TempDir::new();
    let url = a_bare_origin(&home);
    let parent = home.path().join("projects");
    let (fleet, root) = a_fleet_cloning_under(&home, &parent);
    let app = served(&fleet);

    for nowhere in [home.path().join("nowhere"), "projects".into()] {
        let (status, body) =
            call(&app, "POST", "/repositories/clone", &asking(&url, &nowhere)).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            nowhere.display()
        );
        assert_eq!(refusal(&body).code, "fleet.no_such_folder");
    }

    let missing = home.path().join("origin").join("missing.git");
    let (status, body) = call(
        &app,
        "POST",
        "/repositories/clone",
        &asking(&missing.to_string_lossy(), &parent),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let refused = refusal(&body);
    assert_eq!(refused.code, "fleet.clone_refused");
    assert!(
        refused.message.contains("does not exist"),
        "git's own words: {}",
        refused.message
    );
    assert!(!parent.join("missing").exists());

    std::fs::create_dir_all(&root).expect("a folder");
    std::fs::write(Path::new(&root).join("notes"), "mine\n").expect("a file");
    let (status, body) = call(&app, "POST", "/repositories/clone", &asking(&url, &parent)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(refusal(&body).code, "fleet.destination_occupied");
    assert!(Path::new(&root).join("notes").exists());

    assert_eq!(listed(&app).await.len(), 1, "nothing refused was served");
}
