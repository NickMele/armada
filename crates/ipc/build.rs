//! Emit the protocol version's two numbers from `protocol-version.toml`, and
//! the operations an agent may reach from `operations.toml`.
//!
//! Fleet and Bridge ship as a pair and version together, so the Rust constant
//! and the TypeScript one are read from the same file rather than kept in step
//! by hand — which is the drift this arrangement exists to make impossible.
//!
//! The agent's tool set is emitted here for the same reason. A list of tool
//! names written in `api` would be a second copy of the `agent_access` column,
//! and a second copy is how a tool gets added to the inventory and stays
//! unreachable.
//!
//! No dependencies, and no TOML parser. Both files have one shape and adding a
//! parser to read them would put a build-time dependency underneath the whole
//! workspace.

use std::fs;
use std::path::{Path, PathBuf};

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

    let major = number(&text, "major", &source);
    let minor = number(&text, "minor", &source);

    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    fs::write(
        out.join("protocol_version.rs"),
        format!("const PROTOCOL_MAJOR: u32 = {major};\nconst PROTOCOL_MINOR: u32 = {minor};\n"),
    )
    .expect("OUT_DIR is writable");

    let inventory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("operations.toml");
    println!("cargo:rerun-if-changed={}", inventory.display());
    let named = fs::read_to_string(&inventory).unwrap_or_else(|e| {
        panic!(
            "{} is the authority on which operations an agent may reach: {e}",
            inventory.display()
        )
    });
    fs::write(out.join("reachable.rs"), reachable(&named, &inventory)).expect("OUT_DIR is writable");
}

fn number(text: &str, key: &str, source: &Path) -> u32 {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| line.strip_prefix(key))
        .and_then(|rest| rest.trim().strip_prefix('='))
        .and_then(|rest| rest.trim().parse::<u32>().ok())
        .unwrap_or_else(|| panic!("{} has no `{key} = <integer>` line", source.display()))
}

/// Every row whose `agent_access` is `Yes`, as a table.
///
/// **`Yes` and nothing else.** `Bridge only` and `Drafts only` are decisions
/// about a surface that is not this door, and a row reading either is not
/// half-reachable.
fn reachable(inventory: &str, source: &Path) -> String {
    let mut rows = String::new();
    let mut found = 0usize;
    for block in inventory.split("\n[operations.") {
        let Some((header, body)) = block.split_once(']') else {
            continue;
        };
        let name = header.trim().trim_matches('"');
        // The text before the first table is the file's own header, and it
        // reaches here as a "name" with a paragraph in it.
        if name.is_empty() || name.contains(char::is_whitespace) {
            continue;
        }
        if !field(body, "agent_access").is_some_and(|access| access == "Yes") {
            continue;
        }
        let kind = field(body, "kind").unwrap_or_default();
        let description = field(body, "description").unwrap_or_default();
        rows.push_str(&format!(
            "    Reachable {{ operation: {}, kind: {}, description: {} }},\n",
            quoted(name),
            quoted(&kind),
            quoted(&description),
        ));
        found += 1;
    }
    // A parser that matched nothing would emit an empty tool set, and an empty
    // tool set is a door that opens onto nothing while every gate stays green.
    assert!(
        found > 0,
        "{} named no operation with `agent_access = \"Yes\"`, which has never been true — \
         either the file's shape changed or this parser no longer matches it",
        source.display()
    );
    format!("pub const REACHABLE: &[Reachable] = &[\n{rows}];\n")
}

/// One `key = "value"` out of a block, ignoring the `\"\"\"` prose fields.
fn field(block: &str, key: &str) -> Option<String> {
    block
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(key))
        .and_then(|rest| rest.trim().strip_prefix('='))
        .map(|rest| rest.trim().trim_matches('"').to_string())
}

/// A Rust string literal. The inventory holds backticks and apostrophes and no
/// escapes, but a literal built by concatenation has to survive one arriving.
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
