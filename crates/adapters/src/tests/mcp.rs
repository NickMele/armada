//! The bytes Fleet writes for a Drone to dial into, asserted exactly — and the
//! one entry it adds to a repository's own file without disturbing the rest.
//!
//! Not just that the file parses — that `type` still spells `"http"`, so a
//! rename of the transport enum or its `serde` attribute that would change
//! the wire spelling fails here instead of failing silently at a Drone's
//! spawn. See `docs/spikes/010-can-a-drone-be-identified.md` for what an
//! unrecognised `type` does instead: nothing a Drone's log shows.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::{only_the_evidence_server, publish_the_agents_door, Published};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "armada-adapters-mcp-{}-{}.json",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn the_transport_still_spells_http() {
    let at = scratch_path();

    only_the_evidence_server(&at, "http://127.0.0.1:4180/evidence").expect("the file written");
    let written = std::fs::read_to_string(&at).expect("the file read back");

    assert_eq!(
        written,
        r#"{"mcpServers":{"armada":{"type":"http","url":"http://127.0.0.1:4180/evidence"}}}"#
    );

    let _ = std::fs::remove_file(&at);
}

/// The other document, and the property that makes it safe to write into a
/// file somebody else owns: the entry appears and nothing else moves.
#[test]
fn the_agents_door_is_added_and_everything_else_is_kept() {
    let at = scratch_path();
    std::fs::write(
        &at,
        r#"{"mcpServers":{"theirs":{"type":"stdio","command":"their-server"}},"other":[1,2]}"#,
    )
    .expect("a configuration somebody already wrote");

    let published = publish_the_agents_door(&at, "armada", &["mcp"]).expect("the entry written");
    let written = std::fs::read_to_string(&at).expect("the file read back");

    assert_eq!(published, Published::Written);
    assert!(written.contains(r#""armada-fleet""#), "{written}");
    assert!(written.contains(r#""command": "armada""#), "{written}");
    assert!(written.contains(r#""type": "stdio""#), "{written}");
    assert!(written.contains(r#""their-server""#), "{written}");
    assert!(written.contains(r#""other""#), "a key this crate never heard of survives");

    let _ = std::fs::remove_file(&at);
}

/// **A Fleet restart does not dirty a working tree.** The entry carries no
/// port, so the second boot has nothing to change and changes nothing.
#[test]
fn publishing_twice_writes_once() {
    let at = scratch_path();

    assert_eq!(
        publish_the_agents_door(&at, "armada", &["mcp"]).expect("the file created"),
        Published::Written
    );
    let first = std::fs::read_to_string(&at).expect("the file read back");
    assert_eq!(
        publish_the_agents_door(&at, "armada", &["mcp"]).expect("the file left alone"),
        Published::AlreadyThere
    );

    assert_eq!(
        first,
        std::fs::read_to_string(&at).expect("the file read back again")
    );
    let _ = std::fs::remove_file(&at);
}

/// A file that will not parse is a file somebody is editing, or one written by
/// something else. Either way Armada does not get to replace it.
#[test]
fn a_file_that_will_not_parse_is_left_alone() {
    let at = scratch_path();
    std::fs::write(&at, "{ not json at all").expect("a file");

    let refused = publish_the_agents_door(&at, "armada", &["mcp"]).expect_err("refused");

    assert!(refused.to_string().contains("was left alone"), "{refused}");
    assert_eq!(
        std::fs::read_to_string(&at).expect("the file read back"),
        "{ not json at all"
    );
    let _ = std::fs::remove_file(&at);
}

/// The two spellings of the servers key — one a `serde` attribute, one a
/// constant — reach the same place by different routes.
#[test]
fn both_documents_hold_their_servers_under_one_key() {
    let at = scratch_path();
    only_the_evidence_server(&at, "http://127.0.0.1:4180/mcp").expect("the file written");
    let drones = std::fs::read_to_string(&at).expect("the file read back");

    let agents = scratch_path();
    publish_the_agents_door(&agents, "armada", &["mcp"]).expect("the entry written");
    let theirs = std::fs::read_to_string(&agents).expect("the file read back");

    assert!(drones.contains(r#""mcpServers""#), "{drones}");
    assert!(theirs.contains(r#""mcpServers""#), "{theirs}");

    let _ = std::fs::remove_file(&at);
    let _ = std::fs::remove_file(&agents);
}
