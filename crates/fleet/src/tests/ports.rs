//! The pure half of `crate::ports`: sizing, picking a span, and resolving
//! `${port.NAME}` — none of it needs a live Fleet, which is what keeps it
//! tested here rather than only through a dispatch fixture.

use std::collections::BTreeMap;

use config::Manifest;
use core_model::{JobId, Timestamp, Ulid};
use store::{PortClaim, PortClaimant};

use crate::ports::{
    armada_port_env_name, env_names, env_vars, pick_span, port_map, resolve_ports, rounded_width,
    PortProbe, PortRange, PortsRefused,
};

fn manifest(text: &str) -> Manifest {
    Manifest::parse(std::path::Path::new("armada.yml"), text).expect("a manifest that parses")
}

/// A probe that answers from a fixed table, so a test controls exactly which
/// ports are "free" without opening a socket.
struct Scripted<'a>(&'a [u16]);

impl PortProbe for Scripted<'_> {
    fn free(&self, port: u16) -> bool {
        self.0.contains(&port)
    }
}

/// Every port in `range` reads as free — the ordinary case, where nothing
/// else on the machine holds anything nearby.
struct AlwaysFree;

impl PortProbe for AlwaysFree {
    fn free(&self, _port: u16) -> bool {
        true
    }
}

#[test]
fn a_manifest_with_no_ports_rounds_to_zero() {
    assert_eq!(rounded_width(0, 8), 0);
}

#[test]
fn a_width_rounds_up_to_the_granule() {
    assert_eq!(rounded_width(1, 8), 8);
    assert_eq!(rounded_width(8, 8), 8);
    assert_eq!(rounded_width(9, 8), 16);
}

/// **Two Jobs get different spans.** The first candidate is occupied, so the
/// second Job's claim starts a whole granule later.
#[test]
fn two_jobs_get_different_spans() {
    let range = PortRange::of(41_000, 41_100, 8);
    let first = pick_span(range, 8, &[], &AlwaysFree).expect("a span for the first Job");
    let second =
        pick_span(range, 8, &[(first, 8)], &AlwaysFree).expect("a span for the second Job");
    assert_ne!(
        first, second,
        "the second Job does not collide with the first"
    );
    assert_eq!(
        second,
        first + 8,
        "it steps by the granule, not by one port"
    );
}

/// The probe gates every hand-out: a span with a free base but one occupied
/// port inside it is refused, and the search moves past it.
#[test]
fn a_span_with_one_bound_port_inside_it_is_refused() {
    let range = PortRange::of(41_000, 41_100, 8);
    // 41_002 answers, so the first candidate [41000, 41007] fails the probe
    // and the granule after it is picked instead.
    let probe = Scripted(&[
        41_008, 41_009, 41_010, 41_011, 41_012, 41_013, 41_014, 41_015,
    ]);
    let picked = pick_span(range, 8, &[], &probe).expect("the next granule is free");
    assert_eq!(picked, 41_008);
}

/// **Never guess past the ceiling.** A range with no room left refuses rather
/// than handing out a span past it.
#[test]
fn a_range_with_no_room_left_refuses() {
    let range = PortRange::of(41_000, 41_005, 8);
    assert_eq!(pick_span(range, 8, &[], &AlwaysFree), None);
}

#[test]
fn armada_port_env_name_uppercases_and_replaces_punctuation() {
    assert_eq!(armada_port_env_name("storybook"), "ARMADA_PORT_STORYBOOK");
    assert_eq!(armada_port_env_name("api-v2"), "ARMADA_PORT_API_V2");
}

/// **Each Command sees its own `${port.storybook}`.** Two different port maps
/// resolve the same command text to two different literal numbers.
#[test]
fn port_name_resolves_to_the_claimed_number() {
    let text = "pnpm exec storybook dev -p ${port.storybook} --no-open";
    let mut first = BTreeMap::new();
    first.insert("storybook".to_string(), 41_000u16);
    let mut second = BTreeMap::new();
    second.insert("storybook".to_string(), 41_008u16);

    assert_eq!(
        resolve_ports(text, &first),
        "pnpm exec storybook dev -p 41000 --no-open"
    );
    assert_eq!(
        resolve_ports(text, &second),
        "pnpm exec storybook dev -p 41008 --no-open"
    );
}

/// A reference to a name the map does not hold is left exactly as written —
/// there is nothing else this function could put there.
#[test]
fn an_unresolved_port_reference_is_left_alone() {
    let ports = BTreeMap::new();
    assert_eq!(
        resolve_ports("curl ${port.unknown}", &ports),
        "curl ${port.unknown}"
    );
}

#[test]
fn a_port_with_no_env_still_gets_the_guaranteed_variable() {
    let m = manifest("version: 1\nid: armada\nports:\n  storybook: {}\n");
    let names = env_names(&m).expect("no collision");
    assert_eq!(
        names.get("storybook").map(Vec::as_slice),
        Some(["ARMADA_PORT_STORYBOOK".to_string()].as_slice())
    );
}

#[test]
fn a_declared_env_is_set_beside_the_guaranteed_variable() {
    let m = manifest("version: 1\nid: armada\nports:\n  api:\n    env: API_PORT\n");
    let names = env_names(&m).expect("no collision");
    let mut ports = BTreeMap::new();
    ports.insert("api".to_string(), 41_010u16);
    let vars = env_vars(&names, &ports);
    assert_eq!(
        vars,
        vec![
            ("ARMADA_PORT_API".to_string(), "41010".to_string()),
            ("API_PORT".to_string(), "41010".to_string()),
        ]
    );
}

/// **Colliding `env` names across a Job's Manifest set are refused at claim
/// time.**
#[test]
fn two_ports_declaring_the_same_env_are_refused() {
    let m = manifest(
        "version: 1\nid: armada\nports:\n  api:\n    env: SHARED\n  admin:\n    env: SHARED\n",
    );
    let refused = env_names(&m).expect_err("the collision is refused");
    assert!(matches!(refused, PortsRefused::CollidingEnv { name } if name == "SHARED"));
}

/// The claim's base and width, read back against the Manifest's own
/// declaration order, is what turns a bare span into a name-to-port map.
#[test]
fn a_claims_span_maps_back_to_the_manifests_declared_names() {
    let m = manifest("version: 1\nid: armada\nports:\n  admin: {}\n  api: {}\n");
    let claim = PortClaim {
        claimant: PortClaimant::Job(JobId::carried(Ulid::carried("01PORTMAPTEST"))),
        base: 41_000,
        width: 2,
        claimed_at: Timestamp::from_rfc3339("2026-09-11T09:00:00.000Z"),
    };
    let map = port_map(&m, &claim);
    // `admin` sorts before `api`, and `Manifest::port_names` is sorted — so
    // the lower port in the span is whichever name sorts first.
    assert_eq!(map.get("admin"), Some(&41_000));
    assert_eq!(map.get("api"), Some(&41_001));
}
