//! The rule's own negative tests, each against a file written here.

use super::*;
use crate::Finding;

fn run(source: &str) -> Vec<String> {
    let mut report = Report::new("test");
    check("crates/ipc/src/case.rs", source, &mut report);
    report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect()
}

#[test]
fn a_bare_option_on_a_serialized_struct_is_refused_by_name_and_line() {
    let found = run(
        "#[derive(Clone, Serialize, Deserialize)]\npub struct CitedAt {\n    pub file: String,\n    \
         /// Absent where the flag is about the whole file.\n    pub line: Option<u32>,\n}\n",
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("crates/ipc/src/case.rs:5 — `CitedAt::line`"),
        "{found:?}"
    );
}

#[test]
fn a_field_that_says_it_is_skipped_passes_whatever_order_its_doc_and_attribute_take() {
    let found = run(
        "#[derive(Serialize)]\npub struct A {\n    /// Why.\n    #[serde(default, skip_serializing_if = \
         \"Option::is_none\")]\n    pub one: Option<u32>,\n    #[serde(\n        default,\n        \
         skip_serializing_if = \"Option::is_none\"\n    )]\n    /// Why, after.\n    pub two: Option<String>,\n}\n",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_stated_null_is_a_value_and_passes() {
    let found = run(
        "#[derive(Serialize, Deserialize)]\npub struct SetModel {\n    #[serde(deserialize_with = \
         \"stated\")]\n    pub model: Option<String>,\n}\n",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn what_is_not_a_serialized_struct_field_is_not_read() {
    let found = run(
        "#[derive(Deserialize)]\npub struct Read {\n    pub only: Option<u32>,\n}\n\n#[derive(Serialize)]\n\
         pub struct Row {\n    pub id: String,\n}\n\nimpl Row {\n    pub fn of(\n        reason: Option<&str>,\n    \
         ) -> Row {\n        Row { id: String::new() }\n    }\n}\n",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_field_nested_in_a_serialized_structs_braces_is_not_mistaken_for_one_of_its_own() {
    let found = run(
        "#[derive(Serialize)]\npub struct Outer {\n    pub id: String,\n}\n\nfn build() {\n    let inner = \
         Outer {\n        id: Option<u32>::None,\n    };\n}\n",
    );
    assert!(found.is_empty(), "{found:?}");
}
