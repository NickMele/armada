//! The rule's own negative tests.
//!
//! A rule that has never been shown to fail asserts nothing. Each case is
//! proved against a pair written here rather than against the repository, which
//! can only be in one state at a time — and is in the agreeing one.

use super::*;
use crate::Finding;

/// Every finding a report holds, as text.
fn findings(report: &Report) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

/// Whether the report would fail the gate, and every finding it holds.
fn run(pin: &str, manifest: &str) -> (bool, Vec<String>) {
    let mut report = Report::new("test");
    check(pin, manifest, &mut report);
    (report.failed(), findings(&report))
}

/// The manifest, as the repository writes it: the floor under `engines`, with
/// other keys around it so the reader has to find the right one.
fn manifest(floor: &str) -> String {
    format!(
        "{{\n  \"name\": \"armada\",\n  \"packageManager\": \"pnpm@11.6.0\",\n  \"engines\": \
         {{\n    \"node\": \"{floor}\"\n  }},\n  \"scripts\": {{\n    \"gate\": \"cargo xtask \
         verify-foundations\"\n  }}\n}}\n"
    )
}

/// The one finding a pair produces, or a panic naming what it produced instead.
fn only(found: Vec<String>) -> String {
    assert_eq!(found.len(), 1, "{found:?}");
    found.into_iter().next().unwrap_or_default()
}

#[test]
fn the_pair_the_repository_holds_reports_nothing() {
    assert_eq!(run("24.20.0\n", &manifest(">=24")), (false, Vec::new()));
}

#[test]
fn a_pin_below_the_floor_is_refused_and_names_both_values() {
    let (failed, found) = run("22.11.0\n", &manifest(">=24"));
    assert!(failed);
    let found = only(found);
    assert!(found.contains("22.11.0"), "{found}");
    assert!(found.contains(">=24"), "{found}");
    assert!(found.contains(PIN) && found.contains(MANIFEST), "{found}");
}

#[test]
fn a_pin_above_the_floor_is_refused_and_not_merely_warned_about() {
    let (failed, found) = run("26.0.1\n", &manifest(">=24"));
    assert!(failed, "{found:?}");
    let found = only(found);
    assert!(
        found.contains("26.0.1") && found.contains(">=24"),
        "{found}"
    );
}

#[test]
fn a_floor_in_a_form_the_rule_cannot_read_is_reported_not_skipped() {
    for floor in ["^24.20.0", "24.x", ">=24 <25", "*", ">= 24 || >=26"] {
        let (failed, found) = run("24.20.0\n", &manifest(floor));
        assert!(failed, "{floor} was not refused: {found:?}");
        let found = only(found);
        assert!(found.contains(floor), "{found}");
        assert!(found.contains(FORM), "{found}");
    }
}

#[test]
fn a_floor_with_space_after_the_operator_is_still_read() {
    assert_eq!(run("24.20.0\n", &manifest(">= 24")), (false, Vec::new()));
}

#[test]
fn a_manifest_with_no_engines_object_is_refused() {
    let (failed, found) = run("24.20.0\n", "{\n  \"name\": \"armada\"\n}\n");
    assert!(failed);
    assert!(only(found).contains("declares no `engines` object"));
}

#[test]
fn an_engines_object_with_no_node_floor_is_refused() {
    let text = "{\n  \"engines\": {\n    \"pnpm\": \">=11\"\n  }\n}\n";
    let (failed, found) = run("24.20.0\n", text);
    assert!(failed);
    assert!(only(found).contains("sets no `node` floor"));
}

#[test]
fn a_node_key_outside_engines_is_not_read_as_the_floor() {
    let text = "{\n  \"devDependencies\": {\n    \"node\": \">=18\"\n  }\n}\n";
    let (_, found) = run("24.20.0\n", text);
    assert!(
        found.iter().any(|f| f.contains("declares no `engines`")),
        "{found:?}"
    );
    assert!(!found.iter().any(|f| f.contains(">=18")), "{found:?}");
}

#[test]
fn a_dependency_whose_name_begins_with_node_is_not_the_floor() {
    let text = "{\n  \"engines\": {\n    \"node\": \">=24\"\n  },\n  \"devDependencies\": \
                {\n    \"nodemon\": \"^3\"\n  }\n}\n";
    assert_eq!(run("24.20.0\n", text), (false, Vec::new()));
}

#[test]
fn a_floor_written_on_the_engines_line_itself_is_read() {
    let text = "{\n  \"engines\": { \"node\": \">=24\" }\n}\n";
    assert_eq!(run("24.20.0\n", text), (false, Vec::new()));
}

#[test]
fn a_floor_set_twice_is_refused_because_which_one_holds_is_the_reader() {
    let text = "{\n  \"engines\": {\n    \"node\": \">=24\",\n    \"node\": \">=22\"\n  }\n}\n";
    let (failed, found) = run("24.20.0\n", text);
    assert!(failed);
    assert!(only(found).contains("already set at line 3"));
}

#[test]
fn a_floor_that_is_not_a_string_is_refused() {
    let text = "{\n  \"engines\": {\n    \"node\": 24\n  }\n}\n";
    let (failed, found) = run("24.20.0\n", text);
    assert!(failed);
    assert!(only(found).contains("other than a quoted range"));
}

#[test]
fn a_pin_that_is_an_alias_rather_than_a_version_is_refused() {
    for pin in ["lts/*", "node", "24", "24.20", ">=24"] {
        let (failed, found) = run(&format!("{pin}\n"), &manifest(">=24"));
        assert!(failed, "{pin} was not refused: {found:?}");
        let found = only(found);
        assert!(found.contains(pin), "{found}");
        assert!(found.contains("not an exact"), "{found}");
    }
}

#[test]
fn a_pin_printed_the_way_nvm_prints_one_is_read() {
    assert_eq!(run("v24.20.0\n", &manifest(">=24")), (false, Vec::new()));
}

#[test]
fn an_empty_pin_is_refused() {
    let (failed, found) = run("\n\n", &manifest(">=24"));
    assert!(failed);
    assert!(only(found).contains("names no version"));
}

#[test]
fn a_second_line_in_the_pin_is_refused_because_nothing_reads_it() {
    let (failed, found) = run("24.20.0\n22.11.0\n", &manifest(">=24"));
    assert!(failed);
    let found = only(found);
    assert!(found.contains("already set at line 1"), "{found}");
}

#[test]
fn two_empty_files_name_both_of_them() {
    let (failed, found) = run("", "");
    assert!(failed);
    assert!(found.iter().any(|f| f.starts_with(PIN)), "{found:?}");
    assert!(found.iter().any(|f| f.starts_with(MANIFEST)), "{found:?}");
}
