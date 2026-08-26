//! The rule's own negative tests.
//!
//! A gate rule that has never been shown to fail is exactly the thing this rule
//! was written to replace — `the_registrys_counts_are_what_the_enums_hold`
//! asserted a hardcoded thirteen and passed while the registry went to
//! fourteen. So each direction of the comparison is proved against a mismatch
//! built here, not against the repository, which can only ever be in one state
//! at a time.

use super::*;
use crate::Finding;

/// A registry with the three keys named, in the shape the domain files use.
fn registry(keys: &[&str]) -> String {
    keys.iter()
        .map(|k| format!("[states.{k}]\nmeaning = \"x\"\n\n"))
        .collect()
}

/// A source file with the variants named, in the shape `status.rs` uses.
fn source(variants: &[(&str, &str)]) -> String {
    let all: String = variants
        .iter()
        .map(|(v, _)| format!("        StepState::{v},\n"))
        .collect();
    let arms: String = variants
        .iter()
        .map(|(v, w)| format!("            StepState::{v} => \"{w}\",\n"))
        .collect();
    format!(
        "impl StepState {{\n    pub const ALL: &'static [StepState] = &[\n{all}    ];\n\n    \
         pub fn as_wire(&self) -> &'static str {{\n        match self {{\n{arms}        }}\n    \
         }}\n}}\n"
    )
}

const PAIRING: Pairing = Pairing {
    registry: "step-states.toml",
    prefix: "states.",
    enum_name: "StepState",
};

const SOURCE: EnumSource = EnumSource {
    name: "StepState",
    path: "step.rs",
};

/// Every failing line the rule produces for one registry against one source.
fn run(registry: &str, source: &str) -> Vec<String> {
    let mut report = Report::new("test");
    let variants = read_enum(source, &SOURCE, &mut report);
    let keys = keys_under(registry, PAIRING.prefix, "step-states.toml", &mut report);
    compare("step-states.toml", &PAIRING, &keys, &variants, &mut report);
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

#[test]
fn a_registry_and_an_enum_that_agree_report_nothing() {
    let found = run(
        &registry(&["advanced", "not_started"]),
        &source(&[("Advanced", "advanced"), ("NotStarted", "not_started")]),
    );
    assert_eq!(found, Vec::<String>::new());
}

/// The failure that got past the old test: `hatch_unbidden` in the file, no
/// variant behind it.
#[test]
fn a_key_no_variant_spells_is_named_with_its_key_and_its_enum() {
    let found = run(
        &registry(&["advanced", "stopped"]),
        &source(&[("Advanced", "advanced")]),
    );
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("`[states.stopped]` is a key no `StepState` variant spells"));
    assert!(found[0].contains("step-states.toml:4"));
}

#[test]
fn a_variant_no_key_spells_is_named_from_the_other_side() {
    let found = run(
        &registry(&["advanced"]),
        &source(&[("Advanced", "advanced"), ("Stopped", "stopped")]),
    );
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("`StepState::Stopped` spells `stopped`"));
    assert!(found[0].contains("no `[states.stopped]` table"));
}

/// A misspelling on one side only is two failures, not one. Naming only the
/// side that has the extra spelling would read as an addition rather than a
/// divergence, and the fix is on whichever side is wrong.
#[test]
fn a_misspelling_in_one_place_fails_both_ways() {
    let found = run(
        &registry(&["awaitng_human"]),
        &source(&[("AwaitingHuman", "awaiting_human")]),
    );
    assert_eq!(found.len(), 2);
    assert!(found[0].contains("`[states.awaitng_human]` is a key no `StepState` variant spells"));
    assert!(found[1].contains("`StepState::AwaitingHuman` spells `awaiting_human`"));
}

/// Rust identifiers agreeing is not the comparison. `AwaitingHuman` reads as
/// the right variant for `awaiting_human` and is not, once the wire spelling
/// slips.
#[test]
fn the_comparison_is_on_wire_spellings_and_not_on_identifiers() {
    let found = run(
        &registry(&["awaiting_human"]),
        &source(&[("AwaitingHuman", "awaitingHuman")]),
    );
    assert_eq!(found.len(), 2);
}

#[test]
fn a_variant_in_all_with_no_wire_arm_is_the_enum_disagreeing_with_itself() {
    let source = "impl StepState {\n    pub const ALL: &'static [StepState] = &[\n        \
                  StepState::Advanced,\n        StepState::Stopped,\n    ];\n\n    pub fn \
                  as_wire(&self) -> &'static str {\n        match self {\n            \
                  StepState::Advanced => \"advanced\",\n        }\n    }\n}\n";
    let mut report = Report::new("test");
    read_enum(source, &SOURCE, &mut report);
    assert!(report.failed());
}

#[test]
fn a_missing_all_is_reported_rather_than_read_as_an_empty_enum() {
    let mut report = Report::new("test");
    let variants = read_enum("impl StepState {}\n", &SOURCE, &mut report);
    assert!(variants.is_empty());
    assert!(report.failed());
}

/// `level()` and `seen_under()` arms are qualified exactly like `as_wire`'s.
/// Reading them would invent variants and spellings that do not exist.
#[test]
fn only_as_wire_arms_are_read() {
    let source = "impl StepState {\n    pub fn as_wire(&self) -> &'static str {\n        match \
                  self {\n            StepState::Advanced => \"advanced\",\n        }\n    }\n\n    \
                  pub fn label(&self) -> &'static str {\n        match self {\n            \
                  StepState::Advanced => \"went through\",\n        }\n    }\n}\n";
    assert_eq!(
        wire_arms(source, "StepState"),
        vec![("Advanced".to_string(), "advanced".to_string())]
    );
}

/// The domain files carry `notes = """…"""` blocks whose prose contains
/// brackets. A bracketed line inside one is not a table.
#[test]
fn a_bracket_inside_a_multi_line_string_is_not_a_table() {
    let text = "[states.stopped]\nnotes = \"\"\"\n[states.invented]\n\"\"\"\n[states.running]\n";
    assert_eq!(
        tables(text),
        vec![
            ("states.stopped".to_string(), 1),
            ("states.running".to_string(), 5)
        ]
    );
}

#[test]
fn a_key_declared_twice_is_a_row_read_by_nobody() {
    let mut report = Report::new("test");
    let keys = keys_under(
        &registry(&["advanced", "advanced"]),
        "states.",
        "f.toml",
        &mut report,
    );
    assert_eq!(keys.len(), 1);
    assert!(report.failed());
}
