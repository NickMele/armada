//! What Kit's vocabulary refuses, and the one rule a Drone's reach turns on.

use alloc::string::String;
use alloc::vec;

use crate::envelope::{Actor, Timestamp};
use crate::kit::{
    a_drone_resolves, KitServer, ManifestReach, ReachesDrones, ServerAddress, ServerName,
};

fn at() -> Timestamp {
    Timestamp::from_rfc3339("2026-09-17T09:00:00.000Z")
}

fn server(name: &str) -> KitServer {
    KitServer::added(
        ServerName::named(name).expect("a plain name"),
        ServerAddress::address("https://example.test/mcp").expect("an https address"),
        at(),
        Actor::Human,
    )
}

#[test]
fn every_set_reads_back_from_its_own_spelling() {
    for reach in ReachesDrones::ALL {
        assert_eq!(ReachesDrones::from_wire(reach.as_wire()), Some(*reach));
    }
    for reach in ManifestReach::ALL {
        assert_eq!(ManifestReach::from_wire(reach.as_wire()), Some(*reach));
    }
    assert_eq!(ReachesDrones::from_wire("on"), None);
    assert_eq!(ManifestReach::from_wire("allowed"), None);
}

/// A name is a JSON key and half a tool name, so the set it may draw from is
/// the set both readers spell the same way.
#[test]
fn a_name_is_letters_digits_dash_and_underscore() {
    assert!(ServerName::named("  tracker-mcp_2  ").is_some());
    assert_eq!(
        ServerName::named("  tracker  ").map(|n| String::from(n.as_str())),
        Some(String::from("tracker")),
        "trimmed rather than refused"
    );
    for refused in ["", "   ", "two words", "dotted.name", "slash/ed", "★"] {
        assert!(ServerName::named(refused).is_none(), "{refused}");
    }
    let long = "a".repeat(65);
    assert!(ServerName::named(&long).is_none(), "65 characters");
    assert!(ServerName::named(&long[..64]).is_some(), "64 characters");
}

#[test]
fn an_address_is_a_program_or_an_http_url() {
    assert!(ServerAddress::program("npx", &[String::from("-y")]).is_some());
    assert!(ServerAddress::program("   ", &[]).is_none());
    assert!(ServerAddress::address("http://127.0.0.1:9000/mcp").is_some());
    assert!(ServerAddress::address("https://example.test/mcp").is_some());
    for refused in ["example.test", "ws://example.test", "file:///tmp/x"] {
        assert!(ServerAddress::address(refused).is_none(), "{refused}");
    }
    assert_eq!(
        ServerAddress::program("npx", &[String::from("-y"), String::from("pkg")])
            .map(|a| a.transport()),
        Some("stdio")
    );
}

/// `docs/scope.md`'s one confinement, as the type holds it: adding a server
/// widens nothing, because the constructor cannot make one that reaches.
#[test]
fn a_server_a_person_adds_reaches_no_drone() {
    let added = server("tracker");
    assert_eq!(added.drones, ReachesDrones::No);
    assert!(!a_drone_resolves(&added, None));
}

#[test]
fn the_manifests_word_wins_in_either_direction() {
    let off = server("tracker");
    let mut on = server("tracker");
    on.drones = ReachesDrones::Yes;

    assert!(
        a_drone_resolves(&off, Some(ManifestReach::Extended)),
        "a Manifest extends what Kit leaves off"
    );
    assert!(
        !a_drone_resolves(&on, Some(ManifestReach::Restricted)),
        "a Manifest restricts what Kit leaves on"
    );
    assert!(a_drone_resolves(&on, None), "Kit's default, unspoken to");
    assert!(!a_drone_resolves(&off, None));
}

#[test]
fn arguments_are_kept_as_written_and_never_split() {
    let address = ServerAddress::program("npx", &vec![String::from("-y"), String::from("a b")])
        .expect("a program");
    let ServerAddress::Stdio { args, .. } = address else {
        panic!("a program is stdio");
    };
    assert_eq!(args, vec![String::from("-y"), String::from("a b")]);
}
