//! What the collection reads, and what it refuses to read.
//!
//! The parser tests carry the real declarations from `refusing.rs` and
//! `failures.ts` verbatim, and the four file-name `const`s that share their
//! shape. Those four are the reason the shape test exists, so a fixture that
//! invented its own would test the rule against nothing.

use super::*;
use crate::Finding;

/// Every finding's text, so a test can assert on what a person reads.
fn lines(report: &Report) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

// ------------------------------------------------------------ the Rust half

#[test]
fn a_rust_code_is_read_off_the_const() {
    let text = r#"
/// A proposal that decoded and names something that cannot produce a Drone.
const UNACCEPTABLE: &str = "fleet.unacceptable_proposal";
"#;
    let found = rust_declarations("crates/fleet/src/refusing.rs", text);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, "fleet.unacceptable_proposal");
    assert_eq!(found[0].line, 3);
    assert_eq!(
        found[0].meaning.as_deref(),
        Some("A proposal that decoded and names something that cannot produce a Drone.")
    );
}

#[test]
fn every_visibility_reads_the_same() {
    let text = "pub(crate) const A: &str = \"api.one\";\n\
                pub const B: &str = \"api.two\";\n\
                const C: &'static str = \"api.three\";\n";
    let found = rust_declarations("crates/api/src/answers.rs", text);
    let codes: Vec<&str> = found.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, ["api.one", "api.two", "api.three"]);
}

#[test]
fn a_file_name_is_not_a_code() {
    // All four live in `crates/` today and all four are `const NAME: &str`.
    for literal in ["fleet.json", "armada.yml", "armada.db", "mcp.json"] {
        assert!(
            !looks_like_a_code(literal),
            "`{literal}` is a file name and was read as a code"
        );
    }
}

#[test]
fn a_version_and_a_shouted_name_are_not_codes() {
    for literal in ["0.1.0", "5.2", "Fleet.Job", "fleet", "fleet.", ".fleet"] {
        assert!(
            !looks_like_a_code(literal),
            "`{literal}` was read as a code"
        );
    }
}

#[test]
fn a_real_code_is_a_code() {
    for literal in [
        "fleet.no_such_job",
        "api.no_journal_reader",
        "bridge.fleet.unreachable",
    ] {
        assert!(looks_like_a_code(literal), "`{literal}` was not read");
    }
}

#[test]
fn a_commented_out_declaration_is_not_a_declaration() {
    let text = "// const NO_SUCH_JOB: &str = \"fleet.no_such_job\";\n";
    assert!(rust_declarations("crates/fleet/src/refusing.rs", text).is_empty());
}

#[test]
fn a_const_of_another_type_is_not_a_declaration() {
    let text = "const CAP: usize = 25;\n\
                const SOURCE_EXTS: &[&str] = &[\"rs\"];\n";
    assert!(rust_declarations("crates/fleet/src/refusing.rs", text).is_empty());
}

// ---------------------------------------------------------- the Bridge half

#[test]
fn a_bridge_code_is_read_off_the_type() {
    let text = "/** Fleet's process is alive and its socket has stopped answering. */\n\
                const FLEET_UNREACHABLE: BridgeCode = \"bridge.fleet.unreachable\";\n";
    let found = bridge_declarations("packages/shell/src/failures.ts", text);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, "bridge.fleet.unreachable");
    assert_eq!(found[0].site(), "packages/shell/src/failures.ts:2");
    assert_eq!(
        found[0].meaning.as_deref(),
        Some("Fleet's process is alive and its socket has stopped answering.")
    );
}

#[test]
fn a_bridge_code_carries_the_first_line_of_a_block_comment() {
    let text = "/**\n \
                * A region of the window threw while drawing.\n \
                *\n \
                * One code and not one per region.\n \
                */\n\
                const RENDER_BOUNDARY: BridgeCode = \"bridge.render.boundary\";\n";
    let found = bridge_declarations("packages/shell/src/failures.ts", text);
    assert_eq!(
        found[0].meaning.as_deref(),
        Some("A region of the window threw while drawing.")
    );
}

#[test]
fn a_bridge_type_that_is_not_an_assignment_is_not_a_declaration() {
    let text = "function facts(code: BridgeCode, fields: DebugField[]): FailureFacts {\n\
                import type { BridgeCode } from \"@armada/components\";\n";
    assert!(bridge_declarations("packages/shell/src/failures.ts", text).is_empty());
}

#[test]
fn a_computed_bridge_code_is_not_a_declaration() {
    let text = "const CODE: BridgeCode = `bridge.${region}`;\n";
    assert!(bridge_declarations("packages/shell/src/failures.ts", text).is_empty());
}

// ----------------------------------------------------------- the collisions

/// One declaration, at a site.
fn declared(code: &str, path: &str, line: usize) -> Declaration {
    Declaration {
        code: code.to_string(),
        path: path.to_string(),
        line,
        meaning: None,
    }
}

#[test]
fn one_code_declared_twice_names_both_sites() {
    let mut report = Report::new("test");
    collisions(
        &mut report,
        &[
            declared("fleet.fault", "crates/fleet/src/refusing.rs", 33),
            declared("fleet.fault", "crates/api/src/answers.rs", 20),
        ],
        "Rust",
    );
    let found = lines(&report);
    assert_eq!(found.len(), 1, "one finding for one collision: {found:?}");
    assert!(found[0].contains("crates/fleet/src/refusing.rs:33"));
    assert!(found[0].contains("crates/api/src/answers.rs:20"));
    assert!(found[0].contains("`fleet.fault`"));
}

#[test]
fn a_third_site_is_named_too() {
    let mut report = Report::new("test");
    collisions(
        &mut report,
        &[
            declared("bridge.a.b", "packages/shell/src/failures.ts", 1),
            declared("bridge.a.b", "packages/shell/src/other.ts", 2),
            declared("bridge.a.b", "apps/desktop/src/third.ts", 3),
        ],
        "Bridge",
    );
    let found = lines(&report);
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("packages/shell/src/other.ts:2"));
    assert!(found[0].contains("apps/desktop/src/third.ts:3"));
}

#[test]
fn distinct_codes_collide_with_nothing() {
    let mut report = Report::new("test");
    collisions(
        &mut report,
        &[
            declared("fleet.no_such_job", "crates/fleet/src/refusing.rs", 31),
            declared("fleet.illegal_move", "crates/fleet/src/refusing.rs", 32),
        ],
        "Rust",
    );
    assert!(!report.failed());
}

// ------------------------------------------------- the repository as it is

#[test]
fn the_repository_has_no_duplicate_today() {
    let report = one_code_names_one_failure(&crate::repo_root());
    assert!(!report.failed(), "{:?}", lines(&report));
}

/// Both halves find something, and each finds a code named rather than a
/// count. A floor on the count would go stale the first time a code is
/// withdrawn; a code that has to be there catches a scan that matched one
/// line by accident.
#[test]
fn both_halves_find_the_codes_the_repository_declares() {
    let (rust, bridge) = collect(&crate::repo_root());
    let named = |half: &[Declaration], code: &str| half.iter().any(|d| d.code == code);
    assert!(named(&rust, "fleet.no_such_job"), "Rust: {}", rust.len());
    assert!(
        named(&rust, "api.no_journal_reader"),
        "Rust: {}",
        rust.len()
    );
    assert!(
        named(&bridge, "bridge.fleet.unreachable"),
        "Bridge: {}",
        bridge.len()
    );
}

#[test]
fn every_bridge_code_carries_the_prefix_and_no_rust_code_does() {
    let (rust, bridge) = collect(&crate::repo_root());
    for one in &bridge {
        assert!(one.code.starts_with(BRIDGE_PREFIX), "{}", one.site());
    }
    for one in &rust {
        assert!(!one.code.starts_with(BRIDGE_PREFIX), "{}", one.site());
    }
}

#[test]
fn a_rust_code_in_bridges_namespace_fails() {
    let mut report = Report::new("test");
    borrowed_namespace(
        &mut report,
        &[
            declared("fleet.no_such_job", "crates/fleet/src/refusing.rs", 31),
            declared("bridge.fleet.unreachable", "crates/api/src/answers.rs", 9),
        ],
    );
    let found = lines(&report);
    assert_eq!(found.len(), 1, "only the borrowed one: {found:?}");
    assert!(found[0].contains("crates/api/src/answers.rs:9"));
    assert!(found[0].contains("bridge."));
}

/// A root with no source in it finds nothing, and **fails rather than
/// passing**. A rule keyed to a convention dies silently when the convention
/// moves, and this is the only line that would say so.
#[test]
fn an_empty_repository_fails_both_halves() {
    let report = one_code_names_one_failure(std::path::Path::new("/nonexistent"));
    let found = lines(&report);
    assert!(report.failed());
    assert!(found.iter().any(|f| f.starts_with("crates/")), "{found:?}");
    assert!(
        found.iter().any(|f| f.starts_with("packages/, apps/")),
        "{found:?}"
    );
}
