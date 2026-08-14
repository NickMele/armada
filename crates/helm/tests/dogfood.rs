//! **The dogfood test `ARCHITECTURE.md` §2.6 asks for, now that the verb
//! exists.**
//!
//! Through phase 6 the gate runs the raw tools and this asserts `armada manifest check`
//! agrees with them. That staging is deliberate and the reason is not caution:
//! if `armada manifest check` is the gate and `armada manifest check` breaks, every PR fails —
//! including the PR that fixes it. Deferring the flip is not giving up the
//! forcing function, because breaking `armada manifest check` still fails *this*, so it
//! still cannot be merged.
//!
//! **What is asserted is agreement, not success.** A test that only checked
//! `armada manifest check` exits 0 would pass on a repository that is genuinely broken and
//! would fail for a reason that has nothing to do with Armada. Running the raw
//! command and comparing the two verdicts is the assertion that has content:
//! whatever the truth is, Armada has to reach it.
//!
//! **One check, not the suite.** `armada:test` runs `cargo test --workspace`,
//! and running that from inside `cargo test` is a recursion, not a dogfood.
//! `armada:fmt` is the check that is fast, deterministic, and has no
//! dependency on the state of the tree beyond the tree itself. The caveat
//! `ARCHITECTURE.md` §2.6 already records applies with more force here: this
//! config is one component, pure Rust, no services — "it works on Armada" is
//! not evidence the abstraction generalises. The six fixtures and phase 8 are.

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository this suite is compiled inside.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// Whether the raw tool agrees the tree is formatted.
fn raw_fmt_passes(root: &Path) -> Option<bool> {
    Command::new("cargo")
        .args(["fmt", "--all", "--check"])
        .current_dir(root)
        .output()
        .ok()
        .map(|output| output.status.success())
}

/// **`armada manifest check armada:fmt` reaches the same verdict as
/// `cargo fmt --all --check`.**
#[test]
fn armada_check_agrees_with_the_raw_tool_on_armadas_own_config() {
    let root = repo_root();
    let Some(raw) = raw_fmt_passes(&root) else {
        // No cargo on PATH is a machine Armada cannot say anything about. Loud
        // rather than silent: a skip nobody sees is a test nobody has.
        eprintln!("skipping the dogfood check: `cargo fmt` could not be run");
        return;
    };

    // A scratch `$HOME`, so the run's lease and port block land in a throwaway
    // `manifest.db` rather than in the developer's — the same property that makes
    // every other suite here safe to run concurrently.
    let home = tempfile::tempdir().expect("a scratch home");
    let output = Command::new(support::armada_binary())
        .args(["manifest", "check", "armada:fmt", "--json"])
        .current_dir(&root)
        .env("HOME", home.path())
        .output()
        .expect("Armada runs");

    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "not JSON: {e}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });

    let row = payload["data"]["results"]
        .as_array()
        .expect("results[]")
        .iter()
        .find(|row| row["id"] == "armada:fmt")
        .unwrap_or_else(|| panic!("armada:fmt did not resolve: {payload}"));

    if raw {
        assert_eq!(
            row["status"], "PASS",
            "the raw tool passed and Armada did not: {payload}"
        );
        assert_eq!(output.status.code(), Some(0));
    } else {
        assert_eq!(
            row["status"], "FAILED",
            "the raw tool failed and Armada did not: {payload}"
        );
        assert_eq!(
            row["error"]["class"], "tool_failed",
            "a formatting failure is the tool's own result, not Armada's fault"
        );
        assert_eq!(output.status.code(), Some(1));
    }
}

/// **Every check id Armada declares resolves to something Armada can run.**
///
/// The half of §2.6's ask that does not need the tools: a selector that names
/// nothing would report `SKIPPED` and exit 0, which is the failure mode the
/// whole dogfood arrangement exists to catch before phase 6 makes this the
/// gate.
#[test]
fn every_check_armada_declares_is_reachable_by_its_id() {
    let root = repo_root();
    let home = tempfile::tempdir().expect("a scratch home");

    for id in [
        "armada:boundaries",
        "armada:docs",
        "armada:fmt",
        "armada:lint",
        "armada:test",
    ] {
        // `--dry-run` reaches the plan without running anything, which is what
        // makes it safe to ask this of `armada:test`.
        let output = Command::new(support::armada_binary())
            .args(["manifest", "check", id, "--dry-run", "--json"])
            .current_dir(&root)
            .env("HOME", home.path())
            .output()
            .expect("Armada runs");

        let payload: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("an envelope");
        let would: Vec<String> = payload["data"]["would_run"]
            .as_array()
            .map(|lines| {
                lines
                    .iter()
                    .map(|line| line.as_str().unwrap_or_default().to_string())
                    .collect()
            })
            .unwrap_or_default();

        assert!(
            would.iter().any(|line| line.starts_with(&format!("{id}:"))),
            "{id} resolved to nothing Armada would run: {payload}"
        );
        assert_eq!(output.status.code(), Some(0), "{id}");
    }
}
