//! What a Drone is handed, once Kit holds servers — `#1275`.
//!
//! The subject is the document a spawn writes, not the table: `crate::kit`
//! resolves and `crate::spawning` writes, and the one thing worth proving is
//! that the two agree with what `get_kit_servers` draws.

use testkit::{FakeHarness, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet, a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

fn adding(name: &str) -> ipc::AddKitServer {
    ipc::AddKitServer {
        name: name.to_string(),
        address: ipc::ServerAddress::Http {
            url: format!("https://{name}.test/mcp"),
        },
    }
}

fn reaches(spelling: &str) -> ipc::ReachesDrones {
    ipc::ReachesDrones::from_wire(spelling).expect("a spelling the registry has")
}

fn word(spelling: &str) -> ipc::ManifestReach {
    ipc::ManifestReach::from_wire(spelling).expect("a spelling the registry has")
}

/// The document the next Drone was spawned against, read off the spawn the
/// harness recorded rather than off a path this test composed.

async fn what_a_drone_was_handed(home: &TempDir, fleet: &Fixture) -> String {
    let job = fleet
        .propose(a_proposal("a Job that spawns a Drone"))
        .await
        .expect("proposed");
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.expect("dispatched");
    let configured = fleet.harness().configured();
    let last = configured.last().expect("a Drone was configured");
    std::fs::read_to_string(last.mcp().path()).expect("the document the spawn wrote")
}

/// The list as a person reads it, for the Manifest the fixture serves.
async fn listed(fleet: &Fixture) -> Vec<ipc::KitServerRow> {
    let served = fleet.served_named(None).expect("one repository");
    fleet.kit_servers_listed(&served).await.servers
}

/// What a spawn would write, without paying for one. **The same call
/// `crate::spawning` makes**, which is why the two cases that do dispatch are
/// enough to hold the writing end.
async fn resolved(fleet: &Fixture) -> Vec<String> {
    let served = fleet.served_named(None).expect("one repository");
    fleet
        .servers_for_a_drone(&served)
        .await
        .into_iter()
        .map(|server| server.name.as_str().to_string())
        .collect()
}

/// **A Kit nobody has allowed anything from widens nothing.** The v1 defect was
/// a Drone holding every server the operator had connected; a server in Kit and
/// nobody's word about it leaves the document exactly as it was.
#[tokio::test]
async fn a_server_in_kit_and_nothing_said_reaches_no_drone() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/parse.rs"]));

    fleet.add_kit_server(adding("nexus")).await.expect("added");

    let document = what_a_drone_was_handed(&home, &fleet).await;
    assert!(document.contains(r#""armada""#), "{document}");
    assert!(!document.contains("nexus"), "{document}");

    let rows = listed(&fleet).await;
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].resolves, "and the surface says so: {rows:?}");
    assert_eq!(rows[0].drones.as_wire(), "no");
    assert_eq!(rows[0].manifest, None);
}

/// The issue's own definition of done: a person adds a server, allows it for
/// one Manifest, and the Drone dispatched on that Manifest is handed it.
#[tokio::test]
async fn a_manifest_extending_a_server_puts_it_in_the_drones_document() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/parse.rs"]));
    let served = fleet.served_named(None).expect("one repository");

    fleet.add_kit_server(adding("nexus")).await.expect("added");
    fleet
        .set_manifest_server_reach(
            &served,
            ipc::SetManifestServerReach {
                name: "nexus".to_string(),
                reach: Some(word("extended")),
            },
        )
        .await
        .expect("extended");

    let document = what_a_drone_was_handed(&home, &fleet).await;
    assert!(document.contains(r#""nexus""#), "{document}");
    assert!(
        document.contains("https://nexus.test/mcp"),
        "at the address it was given: {document}"
    );
    assert!(
        document.contains(r#""armada""#),
        "and the Evidence server stays: {document}"
    );
    assert!(listed(&fleet).await[0].resolves);
}

/// The other direction of `kit.md`'s *extend or restrict*: a Manifest may
/// withhold a server Kit turned on for everything else, and taking the word
/// back leaves Kit's default answering again.
#[tokio::test]
async fn a_manifest_restricting_one_kit_turned_on_withholds_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/parse.rs"]));
    let served = fleet.served_named(None).expect("one repository");
    fleet.add_kit_server(adding("nexus")).await.expect("added");
    fleet
        .set_kit_server_reach(ipc::SetKitServerReach {
            name: "nexus".to_string(),
            drones: reaches("yes"),
        })
        .await
        .expect("Kit's default moved");

    assert_eq!(
        resolved(&fleet).await,
        vec!["nexus".to_string()],
        "Kit's default reaches a Manifest that has said nothing"
    );
    assert!(listed(&fleet).await[0].resolves);

    fleet
        .set_manifest_server_reach(
            &served,
            ipc::SetManifestServerReach {
                name: "nexus".to_string(),
                reach: Some(word("restricted")),
            },
        )
        .await
        .expect("restricted");

    assert!(resolved(&fleet).await.is_empty());
    assert!(!listed(&fleet).await[0].resolves);

    fleet
        .set_manifest_server_reach(
            &served,
            ipc::SetManifestServerReach {
                name: "nexus".to_string(),
                reach: None,
            },
        )
        .await
        .expect("taken back");
    assert_eq!(resolved(&fleet).await, vec!["nexus".to_string()]);
    assert_eq!(listed(&fleet).await[0].manifest, None);
}

/// Forgetting a server takes it out of what a Drone resolves, and out of every
/// Manifest's word about it.
#[tokio::test]
async fn forgetting_a_server_takes_it_off_the_next_drone() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/parse.rs"]));
    let served = fleet.served_named(None).expect("one repository");
    fleet.add_kit_server(adding("nexus")).await.expect("added");
    fleet
        .set_manifest_server_reach(
            &served,
            ipc::SetManifestServerReach {
                name: "nexus".to_string(),
                reach: Some(word("extended")),
            },
        )
        .await
        .expect("extended");
    assert_eq!(resolved(&fleet).await, vec!["nexus".to_string()]);

    fleet
        .forget_kit_server(ipc::ForgetKitServer {
            name: "nexus".to_string(),
        })
        .await
        .expect("forgotten");

    assert!(resolved(&fleet).await.is_empty());
    assert!(listed(&fleet).await.is_empty());

    // Re-added under the same name, it arrives reaching nobody: the Manifest's
    // old word went with the server rather than waiting for the name to return.
    fleet.add_kit_server(adding("nexus")).await.expect("added");
    assert!(resolved(&fleet).await.is_empty());
    assert!(!listed(&fleet).await[0].resolves);
}

/// What a person cannot add. Each is a 409 rather than a row nothing can spawn
/// against.
#[tokio::test]
async fn a_name_and_an_address_are_refused_where_a_drone_could_not_use_them() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/parse.rs"]));

    for refused in ["", "two words", "dotted.name"] {
        assert!(
            fleet.add_kit_server(adding(refused)).await.is_err(),
            "`{refused}` is not a name a JSON key and a tool name spell alike"
        );
    }
    assert!(
        fleet.add_kit_server(adding("armada")).await.is_err(),
        "a second `armada` would leave a Drone's brief naming two servers"
    );
    assert!(
        fleet
            .add_kit_server(ipc::AddKitServer {
                name: "nexus".to_string(),
                address: ipc::ServerAddress::Http {
                    url: "nexus.test".to_string()
                },
            })
            .await
            .is_err(),
        "a bare host is not an address"
    );
    assert!(
        fleet
            .forget_kit_server(ipc::ForgetKitServer {
                name: "nexus".to_string()
            })
            .await
            .is_err(),
        "Kit holds no `nexus`"
    );
    assert!(listed(&fleet).await.is_empty());
}
