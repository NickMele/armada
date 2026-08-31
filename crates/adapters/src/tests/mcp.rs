//! The bytes Fleet writes for a Drone to dial into, asserted exactly.
//!
//! Not just that the file parses — that `type` still spells `"http"`, so a
//! rename of the transport enum or its `serde` attribute that would change
//! the wire spelling fails here instead of failing silently at a Drone's
//! spawn. See `docs/spikes/010-can-a-drone-be-identified.md` for what an
//! unrecognised `type` does instead: nothing a Drone's log shows.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::only_the_evidence_server;

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
