//! Kit's servers and each Manifest's word over one — `#1275`'s two tables.

use core_model::{
    a_drone_resolves, Actor, KitServer, ManifestId, ManifestReach, ReachesDrones, ServerAddress,
    ServerName, Timestamp,
};

use crate::tests::{at, open, ulid, TempDir};

fn manifest_id(value: &str) -> ManifestId {
    ManifestId::carried(ulid(value))
}

fn named(name: &str) -> ServerName {
    ServerName::named(name).expect("a plain name")
}

fn server(name: &str, address: ServerAddress) -> KitServer {
    KitServer::added(
        named(name),
        address,
        at("2026-09-17T09:00:00.000Z"),
        Actor::Human,
    )
}

fn program(command: &str, args: &[&str]) -> ServerAddress {
    let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
    ServerAddress::program(command, &args).expect("a program")
}

fn said(store: &crate::open::Store, manifest: &ManifestId, name: &str) -> Option<ManifestReach> {
    store
        .manifest_server_reaches(manifest)
        .expect("reads")
        .into_iter()
        .find(|(held, _)| held.as_str() == name)
        .map(|(_, reach)| reach)
}

#[test]
fn a_kit_nobody_added_to_reads_empty() {
    let dir = TempDir::new();
    let store = open(&dir);
    assert!(store.kit_servers().expect("reads").is_empty());
    assert!(store
        .manifest_server_reaches(&manifest_id("01NOBODY"))
        .expect("reads")
        .is_empty());
}

/// Both addresses survive the column split, arguments included — a `stdio` row
/// keeps no url and an `http` row keeps no command.
#[test]
fn both_addresses_read_back_as_they_were_written() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let piped = server(
        "gh",
        program("npx", &["-y", "@modelcontextprotocol/server-github"]),
    );
    let addressed = server(
        "nexus",
        ServerAddress::address("https://example.test/mcp").expect("an address"),
    );

    store.put_kit_server(&piped).expect("put");
    store.put_kit_server(&addressed).expect("put");

    assert_eq!(store.kit_servers().expect("reads"), vec![piped, addressed]);
}

/// The store cannot hand back a server that already reaches a Drone, because
/// `KitServer::added` cannot make one and this write never sets the column.
#[test]
fn a_server_is_stored_reaching_no_drone_and_a_rewrite_leaves_its_reach_alone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .put_kit_server(&server("gh", program("gh-mcp", &[])))
        .expect("put");
    assert_eq!(
        store.kit_servers().expect("reads")[0].drones,
        ReachesDrones::No
    );

    assert!(store
        .set_kit_server_reach(&named("gh"), ReachesDrones::Yes)
        .expect("set"));
    // The same name again, with a corrected command. A person fixing a typo is
    // not re-granting the server.
    store
        .put_kit_server(&server("gh", program("gh-mcp", &["--stdio"])))
        .expect("put again");

    let held = store.kit_servers().expect("reads");
    assert_eq!(held.len(), 1, "one name, one row");
    assert_eq!(held[0].drones, ReachesDrones::Yes);
    assert_eq!(held[0].address, program("gh-mcp", &["--stdio"]));
}

#[test]
fn a_manifests_word_is_its_own_and_reads_back_in_either_direction() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let (this, other) = (manifest_id("01THIS"), manifest_id("01OTHER"));
    store
        .put_kit_server(&server("gh", program("gh-mcp", &[])))
        .expect("put");

    store
        .set_manifest_server_reach(
            &this,
            &named("gh"),
            Some(ManifestReach::Extended),
            &at("2026-09-17T10:00:00.000Z"),
            Actor::Human,
        )
        .expect("set");

    assert_eq!(said(&store, &this, "gh"), Some(ManifestReach::Extended));
    assert_eq!(
        said(&store, &other, "gh"),
        None,
        "a neighbour's is not this one's"
    );

    store
        .set_manifest_server_reach(
            &this,
            &named("gh"),
            Some(ManifestReach::Restricted),
            &at("2026-09-17T11:00:00.000Z"),
            Actor::Human,
        )
        .expect("set again");
    assert_eq!(said(&store, &this, "gh"), Some(ManifestReach::Restricted));
}

/// Taking the word back leaves no row, which is what `a_drone_resolves` reads
/// as Kit's own default answering again.
#[test]
fn taking_a_manifests_word_back_leaves_kits_default_answering() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let this = manifest_id("01BACK");
    store
        .put_kit_server(&server("gh", program("gh-mcp", &[])))
        .expect("put");
    store
        .set_kit_server_reach(&named("gh"), ReachesDrones::Yes)
        .expect("set");
    store
        .set_manifest_server_reach(
            &this,
            &named("gh"),
            Some(ManifestReach::Restricted),
            &at("2026-09-17T10:00:00.000Z"),
            Actor::Human,
        )
        .expect("set");
    assert!(!a_drone_resolves(
        &store.kit_servers().expect("reads")[0],
        said(&store, &this, "gh")
    ));

    store
        .set_manifest_server_reach(
            &this,
            &named("gh"),
            None,
            &at("2026-09-17T11:00:00.000Z"),
            Actor::Human,
        )
        .expect("took it back");

    assert_eq!(said(&store, &this, "gh"), None);
    assert!(a_drone_resolves(
        &store.kit_servers().expect("reads")[0],
        said(&store, &this, "gh")
    ));
}

/// A re-added name must not arrive already extended, so forgetting a server
/// takes every Manifest's word about it with it.
#[test]
fn forgetting_a_server_takes_every_manifests_word_with_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let this = manifest_id("01GONE");
    store
        .put_kit_server(&server("gh", program("gh-mcp", &[])))
        .expect("put");
    store
        .set_manifest_server_reach(
            &this,
            &named("gh"),
            Some(ManifestReach::Extended),
            &at("2026-09-17T10:00:00.000Z"),
            Actor::Human,
        )
        .expect("set");

    assert!(store.forget_kit_server(&named("gh")).expect("forgotten"));

    assert!(store.kit_servers().expect("reads").is_empty());
    assert_eq!(said(&store, &this, "gh"), None);
    assert!(
        !store.forget_kit_server(&named("gh")).expect("reads"),
        "already gone"
    );
    assert!(
        !store
            .set_kit_server_reach(&named("gh"), ReachesDrones::Yes)
            .expect("reads"),
        "nothing to move"
    );
}

/// A timestamp is stored as it was handed over, so a read is not the clock.
#[test]
fn when_a_server_was_added_is_kept() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .put_kit_server(&server("gh", program("gh-mcp", &[])))
        .expect("put");
    assert_eq!(
        store.kit_servers().expect("reads")[0].added_at,
        Timestamp::from_rfc3339("2026-09-17T09:00:00.000Z".to_string())
    );
}
