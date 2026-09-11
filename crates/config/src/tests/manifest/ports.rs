//! `ports:`, the fourth registry — a name, an optional `container`, an
//! optional `env`.

use super::parse;
use crate::error::Fault;
use crate::tests::{fault_at, refusals};

#[test]
fn ports_is_a_fourth_registry_with_two_optional_fields() {
    let manifest = parse(
        "version: 1\nid: armada\nports:\n  storybook: {}\n  api:\n    container: 3000\n    env: API_PORT\n",
    )
    .expect("ports parses");
    assert_eq!(manifest.port_names(), ["api", "storybook"]);
    let storybook = manifest.port("storybook").expect("storybook");
    assert_eq!(storybook.container(), None);
    assert_eq!(storybook.env(), None);
    let api = manifest.port("api").expect("api");
    assert_eq!(api.container(), Some(3000));
    assert_eq!(api.env(), Some("API_PORT"));
}

#[test]
fn an_unknown_key_inside_a_port_hard_fails() {
    let refused = refusals(parse(
        "version: 1\nid: armada\nports:\n  storybook:\n    protocol: http\n",
    ));
    assert!(matches!(
        fault_at(&refused, "ports.storybook.protocol"),
        Fault::Unknown { .. }
    ));
}
