//! Emit `PROTOCOL_VERSION` from `protocol-version.toml` at the repo root.
//!
//! One number governs both sides of the wire. Fleet and Bridge ship as a pair
//! and version together, so the Rust constant and the TypeScript one are read
//! from the same file rather than kept in step by hand — which is the drift
//! this arrangement exists to make impossible.
//!
//! No dependencies, and no TOML parser. The file has one key and adding a
//! parser to read it would put a build-time dependency underneath the whole
//! workspace to save four lines.

use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/ipc sits two levels below the repo root")
        .to_path_buf();
    let source = root.join("protocol-version.toml");

    println!("cargo:rerun-if-changed={}", source.display());

    let text = fs::read_to_string(&source).unwrap_or_else(|e| {
        panic!(
            "{} is the source of truth for the protocol version and could not be read: {e}",
            source.display()
        )
    });

    let version = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| line.strip_prefix("version"))
        .and_then(|rest| rest.trim().strip_prefix('='))
        .and_then(|rest| rest.trim().parse::<u32>().ok())
        .unwrap_or_else(|| panic!("{} has no `version = <integer>` line", source.display()));

    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"))
        .join("protocol_version.rs");
    fs::write(
        &out,
        format!("pub const PROTOCOL_VERSION: u32 = {version};\n"),
    )
    .expect("OUT_DIR is writable");
}
