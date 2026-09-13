//! One Fleet serving several repositories: adding one by folder, every
//! Manifest listed, a Job created in each against its own Checks, and a restart
//! that reconciles both.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use axum::http::StatusCode;
use core_model::{Actor, JobId, JobStatus, Target};
use ipc::{AddRepository, ManifestSummary, RepositoryList, RepositoryScan, RunId, WireError};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::repositories::{Located, Locating, NotLocated, SetUp};
use crate::tests::daemon::{a_proposal, fitted_with, manifest, one};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The id the second repository's `armada.yml` declares.
const SECOND: &str = "01SECONDMANIFEST";
/// The id the fixture's own `armada.yml` declares.
const FIRST: &str = "01FIXTUREMANIFEST";

/// Folders a case planted, read the way the composition root would read them.
#[derive(Default)]
pub struct Planted {
    folders: Mutex<BTreeMap<PathBuf, Located>>,
    served: Mutex<Vec<String>>,
}

impl Planted {
    pub fn nothing() -> Planted {
        Planted::default()
    }

    pub fn with(self, folder: impl Into<PathBuf>, located: Located) -> Planted {
        self.plant(folder, located);
        self
    }

    /// What a folder reads as from now on — an `armada.yml` Write just put down.
    pub fn plant(&self, folder: impl Into<PathBuf>, located: Located) {
        self.folders
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(folder.into(), located);
    }

    /// Every root Fleet said it now serves, in order.
    pub fn served(&self) -> Vec<String> {
        self.served
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Locating for Planted {
    fn located(&self, folder: &Path) -> Result<Located, NotLocated> {
        let folders = self.folders.lock().unwrap_or_else(PoisonError::into_inner);
        folders
            .get(folder)
            .cloned()
            .ok_or_else(|| NotLocated::NotARepository {
                folder: folder.display().to_string(),
                why: String::from("nothing is planted there"),
            })
    }

    fn serving(&self, root: &str) {
        self.served
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(root.to_string());
    }
}

/// A second checkout under `home`, whose Manifest declares a Check the fixture's
/// does not, and whose own `fixture-workflow` gates on every Check it declares.
fn second_repository(home: &TempDir) -> Located {
    let root = home.path().join("second");
    std::fs::create_dir_all(&root).expect("a second checkout");
    let text = format!("version: 1\nid: {SECOND}\nchecks:\n  deliver:\n    run: go test ./...\n");
    let manifest = config::Manifest::parse(&root.join("armada.yml"), &text)
        .expect("the second Manifest loads");
    let def = config::WorkflowDef::parse(
        Path::new("second.yml"),
        "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nstructure: linear\nsteps:\n  \
         - id: implement\n    label: \"Implement\"\n    evidence: {submitted: {type: diff}}\n    \
         mechanical_checks:\n      - type: every_manifest_check\n    delivers: false\n    \
         advance_gate: auto\n",
        &config::Roster::offering_nothing(),
    )
    .expect("the second workflow parses");
    let workflow = config::ResolvedWorkflow::resolve(&def, &manifest).expect("and resolves there");
    folder_at(home, "second", Some(SetUp::of(manifest, one(workflow))))
}

fn folder_at(home: &TempDir, name: &str, set_up: Option<SetUp>) -> Located {
    Located {
        root: home.path().join(name).to_string_lossy().to_string(),
        records_root: home
            .path()
            .join(format!("{name}-records"))
            .to_string_lossy()
            .to_string(),
        set_up,
    }
}

fn a_fleet_reading(home: &TempDir, planted: &Arc<Planted>) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::untouched(),
        FakeHarness::that_listens(),
    );
    fittings.locating = Arc::clone(planted) as Arc<dyn Locating>;
    Fleet::assembled(fittings)
}

fn served(fleet: &Arc<Fixture>) -> axum::Router {
    api::router(api::Served::sharing(
        Arc::clone(fleet),
        RunId::carried("01RUN"),
        fleet.events(),
    ))
}

fn body_of(path: &str) -> String {
    format!(r#"{{"path":"{path}"}}"#)
}

fn code(body: &[u8]) -> String {
    let refused: WireError = ipc::decode("a refusal", body).expect("a WireError");
    refused.code
}

fn in_the_second(title: &str) -> ipc::ProposeJob {
    let mut asked = a_proposal(title);
    asked.owner_manifest_id = ipc::ManifestId::carried(SECOND);
    asked
}

/// A Job the store says is `running`, with no Drone anywhere.
async fn running(fleet: &Fixture, asked: ipc::ProposeJob) -> JobId {
    let job = fleet.propose(asked).await.expect("proposed");
    let job = fleet.approve(job.id()).await.expect("approved");
    fleet
        .move_job(&job, Target::Running, Actor::Fleet)
        .await
        .expect("moved to running");
    job.id().clone()
}

/// **A Fleet started in one repository adds a second by folder**, and both
/// Manifests list — the first one first.
#[tokio::test]
async fn a_fleet_adds_a_second_repository_by_folder_and_lists_both_manifests() {
    let home = TempDir::new();
    let second = second_repository(&home);
    let folder = second.root.clone();
    let planted = Arc::new(Planted::nothing().with(&folder, second));
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);

    let (status, body) = call(&app, "POST", "/repositories/add", &body_of(&folder)).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let added: ipc::RepositorySummary = ipc::decode("the repository", &body).expect("a repository");
    assert_eq!(
        added.manifest.map(|held| held.id.as_str().to_string()),
        Some(SECOND.to_string())
    );
    assert_eq!(
        planted.served(),
        [folder.clone()],
        "and the composition root is told it is served"
    );

    let (_, body) = call(&app, "GET", "/manifests", "").await;
    let listed: Vec<ManifestSummary> = ipc::decode("manifests", &body).expect("manifests");
    let ids: Vec<&str> = listed.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, [FIRST, SECOND]);

    let (_, body) = call(&app, "GET", "/repositories", "").await;
    let listed: RepositoryList = ipc::decode("repositories", &body).expect("repositories");
    let roots: Vec<&str> = listed
        .repositories
        .iter()
        .map(|one| one.root.as_str())
        .collect();
    assert_eq!(
        roots,
        [home.path().to_string_lossy().as_ref(), folder.as_str()]
    );
}

/// **Refused, and said why:** a folder that is not a repository, one already
/// served, and one whose Manifest id another served repository declares.
#[tokio::test]
async fn a_folder_that_is_not_a_repository_or_is_already_served_is_refused() {
    let home = TempDir::new();
    let second = second_repository(&home);
    let folder = second.root.clone();
    let copy = folder_at(&home, "copy", Some(SetUp::of(manifest(), BTreeMap::new())));
    let first = home.path().to_string_lossy().to_string();
    let planted = Arc::new(
        Planted::nothing()
            .with(&folder, second)
            .with(&copy.root.clone(), copy.clone())
            .with(&first, folder_at(&home, "", None)),
    );
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);

    for nowhere in ["/nowhere/at/all", "second"] {
        let (status, body) = call(&app, "POST", "/repositories/add", &body_of(nowhere)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{nowhere}");
        assert_eq!(code(&body), "fleet.not_a_repository");
    }
    let (status, _) = call(&app, "POST", "/repositories/add", &body_of(&folder)).await;
    assert_eq!(status, StatusCode::CREATED);
    for served_already in [folder.as_str(), copy.root.as_str()] {
        let (status, body) =
            call(&app, "POST", "/repositories/add", &body_of(served_already)).await;
        assert_eq!(status, StatusCode::CONFLICT, "{served_already}");
        assert_eq!(code(&body), "fleet.repository_served");
    }
    assert_eq!(planted.served(), [folder], "and nothing refused was served");
}

/// **A Job in each repository is created against that repository's own
/// Checks**, and a number both repositories hold names no one Job.
#[tokio::test]
async fn a_job_in_each_repository_is_held_to_its_own_checks() {
    let home = TempDir::new();
    let second = second_repository(&home);
    let folder = second.root.clone();
    let planted = Arc::new(Planted::nothing().with(&folder, second));
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    fleet
        .added_repository(AddRepository { path: folder })
        .await
        .expect("the second repository is served");

    let first = fleet
        .propose(a_proposal("in the first"))
        .await
        .expect("a Job in the first");
    let second = fleet
        .propose(in_the_second("in the second"))
        .await
        .expect("a Job in the second");
    let named = |job: &core_model::Job| -> Vec<String> {
        let steps = job.workflow().steps().iter();
        steps
            .flat_map(|step| {
                step.checks()
                    .iter()
                    .filter_map(|check| check.name().map(str::to_string))
            })
            .collect()
    };
    assert_eq!(first.owner_manifest_id().as_str(), FIRST);
    assert!(
        named(&first).is_empty(),
        "the fixture's Manifest declares no Check"
    );
    assert_eq!(second.owner_manifest_id().as_str(), SECOND);
    assert_eq!(
        named(&second),
        ["deliver"],
        "the second's Job is held to the second's Check"
    );

    let app = served(&fleet);
    let (status, body) = call(&app, "GET", "/jobs/1", "").await;
    assert_ne!(status, StatusCode::OK, "both repositories hold a Job 1");
    assert!(String::from_utf8_lossy(&body).contains("no one Job"));
}

/// **A restart serves again what was added, and reconciles both.** A
/// remembered folder that is gone is said, and stays remembered.
#[tokio::test]
async fn a_restart_serves_again_what_was_added_and_reconciles_both() {
    let home = TempDir::new();
    let folder = second_repository(&home).root;
    let (first, second) = {
        let planted = Arc::new(Planted::nothing().with(&folder, second_repository(&home)));
        let fleet = a_fleet_reading(&home, &planted);
        let asked = AddRepository {
            path: folder.clone(),
        };
        fleet.added_repository(asked).await.expect("served");
        (
            running(&fleet, a_proposal("mid-flight in the first")).await,
            running(&fleet, in_the_second("mid-flight in the second")).await,
        )
    };

    let planted = Arc::new(Planted::nothing().with(&folder, second_repository(&home)));
    let restarted = a_fleet_reading(&home, &planted);
    assert!(restarted.served_again().await.is_empty());
    assert_eq!(planted.served(), [folder.clone()], "and watched again");
    let reconciled = restarted.reconcile().await.expect("reconciled");
    assert_eq!(reconciled.interrupted, vec![first.clone(), second.clone()]);
    for job in [&first, &second] {
        let status = restarted.load(job).await.expect("there").status();
        assert_eq!(status, JobStatus::Escalated);
    }

    let gone = a_fleet_reading(&home, &Arc::new(Planted::nothing()));
    assert_eq!(
        gone.served_again().await.len(),
        1,
        "a folder that is gone is said"
    );
    let again = a_fleet_reading(&home, &planted);
    assert!(
        again.served_again().await.is_empty(),
        "and still remembered"
    );
}

/// **Each repository's main checkout claims its own span**, so two servers
/// with no Job never share a port.
#[tokio::test]
async fn two_repositories_main_checkouts_claim_separate_spans() {
    let home = TempDir::new();
    let ports = "version: 1\nid: {id}\nports:\n  web: {}\n";
    let declaring = |root: &str, id: &str| {
        let text = ports.replace("{id}", id);
        let at = Path::new(root).join("armada.yml");
        SetUp::of(
            config::Manifest::parse(&at, &text).expect("loads"),
            BTreeMap::new(),
        )
    };
    let other = folder_at(&home, "other", None);
    let set_up = declaring(&other.root, SECOND);
    let planted = Arc::new(Planted::nothing());
    let mut fittings = fitted_with(
        &home,
        FakeWorkProduct::untouched(),
        FakeHarness::that_listens(),
    );
    fittings.manifest = declaring(&home.path().to_string_lossy(), FIRST)
        .manifest()
        .clone();
    fittings.locating = Arc::clone(&planted) as Arc<dyn Locating>;
    let fleet = Fleet::assembled(fittings);
    fleet
        .repositories()
        .add(Located {
            set_up: Some(set_up),
            ..other
        })
        .expect("added");

    let first = fleet.main_checkout_ports(&fleet.first()).await;
    let second = fleet.repositories().serving(SECOND).expect("served");
    let second = fleet.main_checkout_ports(&second).await;
    assert!(first.get("web").is_some() && second.get("web").is_some());
    assert_ne!(first.get("web"), second.get("web"));
    fleet.released_main_checkout_ports().await;
    assert!(fleet
        .store()
        .lock()
        .await
        .every_port_claim()
        .expect("read")
        .is_empty());
}

/// **A folder with no `armada.yml` is served for Scan**, lists as a repository
/// and not a Manifest, and becomes one when an `armada.yml` loads at its root.
#[tokio::test]
async fn a_folder_with_no_manifest_is_scanned_and_set_up_once_one_loads() {
    let home = TempDir::new();
    let unset = folder_at(&home, "unset-up", None);
    std::fs::create_dir_all(&unset.root).expect("a checkout");
    let package = r#"{"scripts":{"test":"vitest run"}}"#;
    std::fs::write(Path::new(&unset.root).join("package.json"), package).expect("a package");
    let root = unset.root.clone();
    let planted = Arc::new(Planted::nothing().with(&root, unset));
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);

    let (status, body) = call(&app, "POST", "/repositories/add", &body_of(&root)).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert!(
        planted.served().is_empty(),
        "nothing to watch until a Manifest loads"
    );
    let (_, body) = call(&app, "GET", "/manifests", "").await;
    assert_eq!(
        ipc::decode::<Vec<ManifestSummary>>("manifests", &body)
            .expect("listed")
            .len(),
        1
    );

    let (status, body) = call(
        &app,
        "GET",
        &format!("/repository/scan?repository={root}"),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let scan: RepositoryScan = ipc::decode("a scan", &body).expect("a scan");
    assert!(scan.workspaces[0]
        .runnables
        .iter()
        .any(|one| one.name == "test"));
    let (status, body) = call(&app, "GET", "/repository/scan?repository=/not/served", "").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(code(&body), "fleet.no_such_repository");

    let text = "version: 1\nid: 01SETUPMANIFEST\n";
    let written =
        config::Manifest::parse(&Path::new(&root).join("armada.yml"), text).expect("loads");
    planted.plant(
        &root,
        folder_at(&home, "unset-up", Some(SetUp::of(written, BTreeMap::new()))),
    );
    let repository = fleet.repositories().at(&root).expect("served");
    fleet.set_up_after_write(&repository);
    let (_, body) = call(&app, "GET", "/manifests", "").await;
    assert_eq!(
        ipc::decode::<Vec<ManifestSummary>>("manifests", &body)
            .expect("listed")
            .len(),
        2
    );
    assert_eq!(
        planted.served(),
        [root],
        "and it is watched once it has one"
    );
}
